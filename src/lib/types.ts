export type FileUploaded = FileTransfer & {
	recipient: string;
	sentAt: Date;
};

export type FileTransfer = {
	name: string;
	transferred: number;
	total: number;
};
