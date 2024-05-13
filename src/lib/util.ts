export function formatBytes(bytes: number, decimals = 2) {
	if (bytes === 0) return '0 Bytes';

	const k = 1024;
	const dm = decimals < 0 ? 0 : decimals;
	const sizes = ['Bytes', 'KB', 'MB', 'GB', 'TB', 'PB', 'EB', 'ZB', 'YB'];

	const i = Math.floor(Math.log(bytes) / Math.log(k));

	return parseFloat((bytes / Math.pow(k, i)).toFixed(dm)) + ' ' + sizes[i];
}

export function formatTime(seconds: number): string {
	const timeInHrs = seconds / 3600;

	if (timeInHrs > 4) {
		return `${Math.ceil(timeInHrs)} hrs`;
	}

	const timeInMins = seconds / 60;

	if (timeInMins > 4) {
		return `${Math.ceil(timeInMins)} mins`;
	}

	return `${Math.ceil(seconds)} secs`;
}
