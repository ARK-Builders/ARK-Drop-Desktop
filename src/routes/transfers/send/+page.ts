import type { PageLoad } from './$types';

export const load: PageLoad = ({ url }) => {
	const file = url.searchParams.get('file');
	return {
		file
	};
};
