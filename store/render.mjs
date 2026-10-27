// Renders the store graphics from the raw screenshots and the HTML sources in this folder:
// the framed screenshots (*/framed/), the Play feature graphics and the store icons.
//
//   node store/render.mjs        (needs Google Chrome; CHROME=<path> to use another)
import { execFileSync } from 'node:child_process';
import { existsSync, mkdirSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

const STORE = path.dirname(fileURLToPath(import.meta.url));
const CHROME = process.env.CHROME ?? '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome';

const captions = {
	en: [
		['01-welcome', 'A cold wallet for a phone that stays offline', 'Bring your recovery phrase each time. Nothing is stored.'],
		['02-home', 'Bitcoin and Ethereum, 100 accounts each', 'The web companion gets addresses, never public keys.'],
		['03-review', 'Every recipient, amount and fee before you sign', 'Hold to sign. Airkeys signs transfers only.'],
		['04-code', 'Animated codes, no cable and no network', 'The web companion reads them with its camera.']
	],
	zh: [
		['01-welcome', '装在离线手机上的冷钱包', '每次输入助记词，什么都不存储。'],
		['02-home', 'Bitcoin 和 Ethereum，各 100 个账户', '网页助手只得到地址，从不得到公钥。'],
		['03-review', '签名前逐项核对', '每个收款方、金额和手续费都显示出来，按住才签名。'],
		['04-code', '用动态码传递，不连线、不联网', '网页助手用摄像头读取。']
	]
};
// App Store 6.9-inch; Google Play (sides at most 2:1)
const targets = [
	{ dir: 'app-store', width: 1320, height: 2868 },
	{ dir: 'google-play', width: 1080, height: 2160 }
];

function shoot(url, out, width, height) {
	execFileSync(CHROME, [
		'--headless=new',
		'--hide-scrollbars',
		'--force-device-scale-factor=1',
		'--allow-file-access-from-files',
		`--window-size=${width},${height}`,
		`--screenshot=${out}`,
		url
	], { stdio: 'ignore' });
	console.log(path.relative(STORE, out));
}

const page = (file) => pathToFileURL(path.join(STORE, file)).href;

for (const { dir, width, height } of targets) {
	for (const [lang, list] of Object.entries(captions)) {
		for (const [name, title, sub] of list) {
			const raw = path.join(STORE, dir, 'screenshots', lang, `${name}.png`);
			if (!existsSync(raw)) {
				console.log('missing', path.relative(STORE, raw));
				continue;
			}
			const out = path.join(STORE, dir, 'framed', lang);
			mkdirSync(out, { recursive: true });
			const query = new URLSearchParams({ img: pathToFileURL(raw).href, title, sub, lang });
			shoot(`${page('screenshot-frame.html')}?${query}`, path.join(out, `${name}.png`), width, height);
		}
	}
}
for (const lang of ['en', 'zh']) {
	shoot(`${page('feature-graphic.html')}#${lang}`, path.join(STORE, 'google-play', `feature-graphic-${lang}.png`), 1024, 500);
}
// the store icons, from the iOS app icon (1024, no transparency)
const icon = path.join(STORE, '../app-ios/Airkeys/Resources/Assets.xcassets/AppIcon.appiconset/icon-1024.png');
execFileSync('sips', ['-s', 'format', 'png', icon, '--out', path.join(STORE, 'app-store/icon-1024.png')], { stdio: 'ignore' });
execFileSync('sips', ['-z', '512', '512', icon, '--out', path.join(STORE, 'google-play/icon-512.png')], { stdio: 'ignore' });
console.log('app-store/icon-1024.png\ngoogle-play/icon-512.png');
