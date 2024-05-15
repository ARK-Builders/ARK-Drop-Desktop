export type FileUploaded = {
	fileName: string;
	fileSize: number; // bytes
	recipient: string;
	sentAt: Date;
};
