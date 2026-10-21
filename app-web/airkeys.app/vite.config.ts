import { defineConfig } from 'vitest/config';
import adapter from '@sveltejs/adapter-static';
import { sveltekit } from '@sveltejs/kit/vite';

// AIRKEYS_EXTENSION=1 builds the browser extension (scripts/extension.mjs): the app alone,
// hash-routed into one page, because an extension serves files, not routes
const extension = process.env.AIRKEYS_EXTENSION === '1';

export default defineConfig({
	plugins: [
		sveltekit({
			compilerOptions: { runes: true },
			// a static site: host the build/ folder anywhere
			adapter: extension ? adapter({ pages: 'extension', assets: 'extension', fallback: 'index.html' }) : adapter({ fallback: '404.html' }),
			// browser extensions refuse folder names that start with "_"
			appDir: 'app-kit',
			...(extension ? { files: { routes: 'src/extension' }, router: { type: 'hash' as const } } : {})
		})
	],
	// registry/networks.json is shared with the phone's core, at the repository root
	server: { fs: { allow: ['../..'] } },
	test: { include: ['src/**/*.test.ts'], environment: 'node' }
});
