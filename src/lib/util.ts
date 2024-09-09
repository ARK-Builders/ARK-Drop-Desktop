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

export function getDateInterval(date: Date) {
	const now = new Date();

	const nowDate = new Date(now.getFullYear(), now.getMonth(), now.getDate());
	const inputDateNormalized = new Date(date.getFullYear(), date.getMonth(), date.getDate());

	const diffTime = nowDate.getUTCDate() - inputDateNormalized.getUTCDate();

	if (diffTime === 0) {
		return 'Today';
	} else if (diffTime === 1) {
		return 'Yesterday';
	} else {
		const startOfWeek = new Date(nowDate);
		startOfWeek.setDate(nowDate.getDate() - nowDate.getDay());

		const startOfMonth = new Date(now.getFullYear(), now.getMonth(), 1);
		const startOfYear = new Date(now.getFullYear(), 0, 1);

		if (inputDateNormalized >= startOfWeek) {
			return 'This Week';
		} else if (inputDateNormalized >= startOfMonth) {
			return 'This Month';
		} else if (inputDateNormalized >= startOfYear) {
			return 'This Year';
		} else {
			return 'Older';
		}
	}
}

export function getConfirmationCode(hash: string) {
	return (
		hash
			.split('')
			.map((char) => char.charCodeAt(0))
			.reduce((acc, curr) => acc + curr, 0) % 100
	);
}
