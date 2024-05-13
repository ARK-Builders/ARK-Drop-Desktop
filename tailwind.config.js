/** @type {import('tailwindcss').Config} */
export default {
	content: ['./src/**/*.{html,js,svelte,ts}'],
	theme: {
		extend: {
			fontFamily: {
				sans: ['Inter', 'sans-serif']
			},
			colors: {
				'button-secondary-border': '#D0D5DD',
				'button-secondary-fg': '#344054',
				'nav-item-icon-fg': '#667085',
				'primary-fg': '#fff',
				'text-secondary': {
					700: '#344054'
				},
				'gray-modern': {
					100: '#EEF2F6',
					200: '#E3E8EF',
					300: '#CDD5DF',
					500: '#697586',
					600: '#4B5565',
					900: '#121926'
				},
				'blue-dark': {
					400: '#528BFF',
					500: '#2970FF',
					600: '#155EEF'
				},
				'gray-light': {
					400: '#98A2B3'
				},
				error: {
					500: '#F04438'
				},
				success: {
					500: '#17B26A'
				}
			},
			borderWidth: {
				1: '1px'
			}
		}
	},
	plugins: []
};
