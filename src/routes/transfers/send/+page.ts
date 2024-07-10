import type { PageLoad } from './$types';

export const load: PageLoad = ({ url }) => {
	const files = url.searchParams.getAll('file');

	return {
		files
	};
};
