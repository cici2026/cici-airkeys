// Builds the browser extension (Manifest V3): the app alone, hash-routed into one page that the
// toolbar button opens in a tab. Extension pages may not run inline scripts, so SvelteKit's
// inline bootstrap moves into a file beside the page — a classic script in the same place, so
// relative imports and `document.currentScript` behave as before.
//
//   node scripts/extension.mjs      → extension/ (load it unpacked, or zip it for a store)
import { execFileSync } from 'node:child_process';
import { readdirSync, readFileSync, rmSync, statSync, writeFileSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const out = path.join(root, 'extension');
const pkg = JSON.parse(readFileSync(path.join(root, 'package.json'), 'utf8'));
rmSync(out, { recursive: true, force: true });
execFileSync(process.execPath, [path.join(root, 'node_modules/vite/bin/vite.js'), 'build'], {
	cwd: root,
	env: { ...process.env, AIRKEYS_EXTENSION: '1' },
	stdio: 'ignore'
});

let moved = 0;
const walk = (dir) => {
	for (const name of readdirSync(dir)) {
		const p = path.join(dir, name);
		if (statSync(p).isDirectory()) walk(p);
		else if (name.endsWith('.html')) {
			let n = 0;
			const html = readFileSync(p, 'utf8').replace(/<script>([\s\S]*?)<\/script>/g, (_, code) => {
				const file = `${path.basename(name, '.html')}-boot-${n++}.js`;
				writeFileSync(path.join(dir, file), code);
				moved++;
				return `<script src="./${file}"></script>`;
			});
			writeFileSync(p, html);
		}
	}
};
walk(out);

writeFileSync(
	path.join(out, 'manifest.json'),
	JSON.stringify(
		{
			manifest_version: 3,
			name: 'Airkeys',
			version: pkg.version,
			description:
				'The online side of the Airkeys cold wallet: balances, transfer requests, broadcasting. It never holds a key.',
			icons: { 16: 'icons/icon-16.png', 48: 'icons/icon-48.png', 128: 'icons/icon-128.png' },
			action: {
				default_title: 'Open Airkeys',
				default_icon: { 16: 'icons/icon-16.png', 48: 'icons/icon-48.png' }
			},
			background: { service_worker: 'background.js' },
			content_security_policy: { extension_pages: "script-src 'self'; object-src 'self'" }
		},
		null,
		2
	)
);
writeFileSync(
	path.join(out, 'background.js'),
	"chrome.action.onClicked.addListener(() => chrome.tabs.create({ url: 'index.html' }));\n"
);
console.log(`extension/ written (${moved} inline scripts moved to files)`);
