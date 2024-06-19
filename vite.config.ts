import localIpUrl from 'local-ip-url';
import { sveltekit } from '@sveltejs/kit/vite';
import * as vite from 'vite';
import { defineConfig } from 'vite';

const platform = process.env.TAURI_ENV_PLATFORM!;

let config: vite.UserConfig = {
	// Prevent Vite from obscuring Tauri Rust errors.
	clearScreen: false,
	plugins: [sveltekit()]
};

if (platform === 'ios' || platform === 'android') {
	config = vite.mergeConfig(config, {
		server: {
			host: '0.0.0.0',
			hmr: {
				host: localIpUrl()
			}
		}
	});
}

export default defineConfig(config);
