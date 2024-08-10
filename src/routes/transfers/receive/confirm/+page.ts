import { redirect } from '@sveltejs/kit';
import type { PageLoad } from './$types';
import { getConfirmationCode } from '$lib/util';

export const load: PageLoad = ({ url }) => {
	const hash = url.searchParams.get('hash');

	if (hash === null) {
		redirect(302, '/');
	}

	const confirmationCode = getConfirmationCode(hash);

	return {
		hash,
		confirmationCode
	};
};
