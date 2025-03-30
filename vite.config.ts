import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';

const host = process.env.TAURI_DEV_HOST;
const port = process.env.TAURI_DEV_PORT;

export default defineConfig({
	plugins: [sveltekit()],
	clearScreen: false,
	server: {
	  host: host || false,
	  port: parseInt(port || "5173"),
	  strictPort: false,
	  hmr: host
		? {
			protocol: 'ws',
			host: host,
			port: 1430,
		  }
		: undefined,
	},
});