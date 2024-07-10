import { redirect } from '@sveltejs/kit';
import type { PageLoad } from './$types';

export const load: PageLoad = ({ url }) => {
	const ticket = url.searchParams.get('ticket');

	if (ticket === null) {
		redirect(302, '/');
	}

	return {
		ticket
	};
};
