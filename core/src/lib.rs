pub mod error;
pub mod metadata;

use std::{collections::BTreeMap, path::PathBuf, str::FromStr};

use anyhow::Error;
use data_encoding::HEXLOWER;
use futures_buffered::try_join_all;
use futures_lite::StreamExt;
use iroh::{protocol::Router, Endpoint};
use iroh_blobs::{
    format::collection::Collection,
    get::db::DownloadProgress,
    hashseq::HashSeq,
    net_protocol::Blobs,
    rpc::client::blobs::{AddOutcome, WrapOption},
    store::fs::Store,
    ticket::BlobTicket,
    util::{local_pool::LocalPool, SetTagOption},
    BlobFormat, Hash, Tag,
};

use metadata::{CollectionMetadata, FileTransfer};

use rand::Rng;

pub struct IrohInstance {
    router: Router,

    blobs: Blobs<Store>,
}

impl IrohInstance {
    pub async fn new() -> Result<Self, Error> {
        let endpoint = Endpoint::builder().discovery_n0().bind().await.unwrap();
        let local_pool = LocalPool::default();

        let suffix = rand::thread_rng().gen::<[u8; 16]>();
        let cwd = std::env::current_dir()?;
        let blobs_data_dir = cwd.join(format!(".drop-send-{}", HEXLOWER.encode(&suffix)));
        let blobs = Blobs::persistent(blobs_data_dir)
            .await
            .unwrap()
            .build(&local_pool, &endpoint);

        let router = Router::builder(endpoint)
            .accept(iroh_blobs::ALPN, blobs.clone())
            .spawn()
            .await
            .unwrap();

        Ok(Self { router, blobs })
    }

    pub async fn send_files(&self, files: Vec<String>) -> Result<BlobTicket, Error> {
        let paths: Vec<PathBuf> = files
            .into_iter()
            .map(|path| Ok(PathBuf::from_str(&path)?))
            .filter_map(|path: Result<PathBuf, Error>| path.ok())
            .collect();

        let (hash, _tag) = self
            .import_collection(paths)
            .await
            .expect("Failed to Import collection");

        Ok(BlobTicket::new(
            self.router.endpoint().node_id().into(),
            hash,
            iroh_blobs::BlobFormat::HashSeq,
        )
        .expect("Failed to create ticket"))
    }

    pub async fn receive_files(&self, ticket: String) -> Result<Collection, Error> {
        let ticket = BlobTicket::from_str(&ticket).expect("Failed to parse ticket");

        if ticket.format() != BlobFormat::HashSeq {
            panic!("Invalid ticket format.");
        }

        let mut download = self
            .blobs
            .client()
            .download_hash_seq(ticket.hash(), ticket.node_addr().clone())
            .await
            .expect("Failed to download hash seq.");

        let mut curr_metadata: Option<CollectionMetadata> = None;
        let mut curr_hashseq: Option<HashSeq> = None;
        let mut files: Vec<FileTransfer> = Vec::new();
        let mut map: BTreeMap<u64, String> = BTreeMap::new();

        while let Some(event) = download.next().await {
            if let Ok(event) = event {
                match event {
                    DownloadProgress::FoundHashSeq { hash, .. } => {
                        let hashseq = self
                            .blobs
                            .client()
                            .read_to_bytes(hash)
                            .await
                            .expect("Failed to read hashseq");

                        let hashseq = HashSeq::try_from(hashseq)
                            .expect("Failed to convert hashseq to HashSeq.");

                        let metadata_hash =
                            hashseq.iter().next().expect("Failed to get metadata hash.");
                        let metadata_bytes = self
                            .blobs
                            .client()
                            .read_to_bytes(metadata_hash)
                            .await
                            .expect("Failed to read hashseq");

                        let metadata: CollectionMetadata = postcard::from_bytes(&metadata_bytes)
                            .expect("Failed to convert hashseq to HashSeq.");

                        // The hash sequence should have one more element than the metadata
                        // because the first element is the metadata itself
                        if metadata.names.len() + 1 != hashseq.len() {
                            panic!("Invalid metadata.");
                        }
                        curr_hashseq = Some(hashseq);
                        curr_metadata = Some(metadata);
                    }

                    DownloadProgress::AllDone(_) => {
                        let collection = self
                            .blobs
                            .client()
                            .get_collection(ticket.hash())
                            .await
                            .expect("Failed to get collection.");
                        files = vec![];
                        for (name, hash) in collection.iter() {
                            let content = self
                                .blobs
                                .client()
                                .read_to_bytes(*hash)
                                .await
                                .expect("Failed to read hashseq");
                            files.push({
                                FileTransfer {
                                    name: name.clone(),
                                    transferred: content.len() as u64,
                                    total: content.len() as u64,
                                }
                            })
                        }
                        // handle_chunk
                        //     .0
                        //     .send(files.clone())
                        //     .map_err(|_| IrohError::SendError)?;

                        return Ok(collection.into());
                    }

                    DownloadProgress::Done { id } => {
                        if let Some(name) = map.get(&id) {
                            if let Some(file) = files.iter_mut().find(|file| file.name == *name) {
                                file.transferred = file.total;
                            }
                        }
                        // handle_chunk
                        //     .0
                        //     .send(files.clone())
                        //     .map_err(|_| IrohError::SendError)?;
                    }

                    DownloadProgress::Found { id, hash, size, .. } => {
                        if let (Some(hashseq), Some(metadata)) = (&curr_hashseq, &curr_metadata) {
                            if let Some(idx) = hashseq.iter().position(|h| h == hash) {
                                if idx >= 1 && idx <= metadata.names.len() {
                                    if let Some(name) = metadata.names.get(idx - 1) {
                                        files.push(FileTransfer {
                                            name: name.clone(),
                                            transferred: 0,
                                            total: size,
                                        });
                                        // handle_chunk
                                        //     .0
                                        //     .send(files.clone())
                                        //     .map_err(|_| IrohError::SendError)?;
                                        map.insert(id, name.clone());
                                    }
                                }
                            } else {
                                unreachable!();
                            }
                        }
                    }

                    DownloadProgress::Progress { id, offset } => {
                        if let Some(name) = map.get(&id) {
                            if let Some(file) = files.iter_mut().find(|file| file.name == **name) {
                                file.transferred = offset;
                            }
                        }
                        // handle_chunk
                        //     .0
                        //     .send(files.clone())
                        //     .map_err(|_| IrohError::SendError)?;
                    }

                    DownloadProgress::FoundLocal { hash, size, .. } => {
                        if let (Some(hashseq), Some(metadata)) = (&curr_hashseq, &curr_metadata) {
                            if let Some(idx) = hashseq.iter().position(|h| h == hash) {
                                if idx >= 1 && idx <= metadata.names.len() {
                                    if let Some(name) = metadata.names.get(idx - 1) {
                                        if let Some(file) =
                                            files.iter_mut().find(|file| file.name == *name)
                                        {
                                            file.transferred = size.value();
                                            file.total = size.value();
                                            // handle_chunk
                                            //     .0
                                            //     .send(files.clone())
                                            //     .map_err(|_| IrohError::SendError)?;
                                        }
                                    }
                                }
                            }
                        }
                    }

                    _ => {}
                }
            }
        }

        let collection = self
            .blobs
            .client()
            .get_collection(ticket.hash())
            .await
            .expect("Failed to get collection.");

        Ok(collection.into())
    }

    pub async fn import_collection(&self, paths: Vec<PathBuf>) -> Result<(Hash, Tag), Error> {
        let outcomes = try_join_all(paths.into_iter().map(|path| async move {
            let add_progress = self
                .blobs
                .client()
                .add_from_path(path.clone(), true, SetTagOption::Auto, WrapOption::NoWrap)
                .await;

            println!("Importing: {:?}", path);

            match add_progress {
                Ok(add_progress) => {
                    let outcome = add_progress.finish().await;
                    if let Ok(progress) = outcome {
                        Ok::<(PathBuf, AddOutcome), Error>((path.clone(), progress))
                    } else {
                        panic!("Failed to add blob: {:?}", outcome.err().unwrap())
                    }
                }
                Err(e) => {
                    panic!("Failed to add blob: {:?}", e)
                }
            }
        }))
        .await
        .expect("Failed to import blobs.");

        let collection = outcomes
            .into_iter()
            .map(|(path, outcome)| {
                let name = path
                    .file_name()
                    .expect("The file name is not valid.")
                    .to_string_lossy()
                    .to_string();

                let hash = outcome.hash;
                (name, hash)
            })
            .collect();

        Ok(self
            .blobs
            .client()
            .create_collection(collection, SetTagOption::Auto, Default::default())
            .await
            .expect("Failed to create collection."))
    }
}
