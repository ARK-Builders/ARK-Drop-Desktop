export type FileUploaded = FileTransfer & {
	recipient: string;
	sentAt: Date;
};

export type FileTransfer = {
	name: string;
	transfered: number;
	total: number;
};
