import { redirect } from '@sveltejs/kit';
import type { PageLoad } from './$types';

export const load: PageLoad = ({ url }) => {
	const hash = url.searchParams.get('hash');

	if (hash === null) {
		redirect(302, '/');
	}

	return {
		hash
	};
};
