// Shared helpers for the end-to-end checks: run the phone's signing path (the `sign`
// example of airkeys-wallet) and talk JSON-RPC.
import { execFileSync, spawn } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import path from 'node:path';

export const CORE = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
export const PHRASE = 'test test test test test test test test test test test junk';
const SIGNER = path.join(CORE, 'target/release/examples/sign');

export function buildSigner() {
	execFileSync('cargo', ['build', '--release', '-q', '-p', 'airkeys-wallet', '--example', 'sign'], { cwd: CORE, stdio: 'inherit' });
}

/** The phone: an accounts message, or the signed reply to a request (throws if refused). */
export function phone(args, input, phrase = PHRASE) {
	const out = execFileSync(SIGNER, args, { input: input ? JSON.stringify(input) : '', env: { ...process.env, AIRKEYS_PHRASE: phrase }, stdio: ['pipe', 'pipe', 'pipe'] });
	return JSON.parse(out.toString());
}

export function phoneRefuses(input, phrase = PHRASE) {
	try {
		phone([], input, phrase);
	} catch (e) {
		return e.stderr.toString();
	}
	throw new Error('the phone signed a request it should refuse');
}

export function address(network, index) {
	return phone(['accounts', network, String(index)]).accounts[0].address;
}

export async function rpc(url, method, params = []) {
	const r = await fetch(url, { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify({ jsonrpc: '2.0', id: 1, method, params }) });
	const j = await r.json();
	if (j.error) throw new Error(`${method}: ${JSON.stringify(j.error)}`);
	return j.result;
}

/** A receipt, once the node has one (anvil mines a moment after accepting). */
export async function receipt(url, hash) {
	for (let i = 0; i < 100; i++) {
		const r = await rpc(url, 'eth_getTransactionReceipt', [hash]);
		if (r) return r;
		await new Promise((ok) => setTimeout(ok, 100));
	}
	return null;
}

export function start(cmd, args, readyText) {
	return new Promise((resolve, reject) => {
		const p = spawn(cmd, args, { stdio: ['ignore', 'pipe', 'pipe'] });
		let log = '';
		const onData = (d) => {
			log += d;
			if (log.includes(readyText)) resolve(p);
		};
		p.stdout.on('data', onData);
		p.stderr.on('data', onData);
		p.on('exit', (code) => reject(new Error(`${cmd} exited ${code}: ${log}`)));
	});
}

let failures = 0;
export function check(ok, what) {
	console.log(`${ok ? '  ok ' : 'FAIL '} ${what}`);
	if (!ok) failures++;
}
export function done() {
	console.log(failures ? `\n${failures} check(s) failed` : '\nall checks passed');
	process.exit(failures ? 1 : 0);
}

// ------------------------------------------------------------ address encodings (tests only)
import { createHash } from 'node:crypto';

const CHARSET = 'qpzry9x8gf2tvdw0s3jn54khce6mua7l';
function polymod(values) {
	const GEN = [0x3b6a57b2, 0x26508e6d, 0x1ea119fa, 0x3d4233dd, 0x2a1462b3];
	let chk = 1;
	for (const v of values) {
		const top = chk >> 25;
		chk = ((chk & 0x1ffffff) << 5) ^ v;
		for (let i = 0; i < 5; i++) if ((top >> i) & 1) chk ^= GEN[i];
	}
	return chk;
}
const expand = (hrp) => [...[...hrp].map((c) => c.charCodeAt(0) >> 5), 0, ...[...hrp].map((c) => c.charCodeAt(0) & 31)];
function convert(data, from, to, pad) {
	let acc = 0, bits = 0;
	const out = [];
	for (const v of data) {
		acc = (acc << from) | v;
		bits += from;
		while (bits >= to) out.push((acc >> (bits -= to)) & ((1 << to) - 1));
	}
	if (pad && bits) out.push((acc << (to - bits)) & ((1 << to) - 1));
	return out;
}
/** A segwit address for a witness version and program (bech32 for v0, bech32m above). */
export function segwitAddress(hrp, version, program) {
	const data = [version, ...convert(program, 8, 5, true)];
	const constant = version === 0 ? 1 : 0x2bc830a3;
	const mod = polymod([...expand(hrp), ...data, 0, 0, 0, 0, 0, 0]) ^ constant;
	const checksum = [0, 1, 2, 3, 4, 5].map((i) => (mod >> (5 * (5 - i))) & 31);
	return hrp + '1' + [...data, ...checksum].map((d) => CHARSET[d]).join('');
}
/** The witness version and program of a segwit address (checksum not verified: test input). */
export function segwitProgram(addr) {
	const data = [...addr.slice(addr.lastIndexOf('1') + 1, -6)].map((c) => CHARSET.indexOf(c));
	return { version: data[0], program: Buffer.from(convert(data.slice(1), 5, 8, false)) };
}
const B58 = '123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz';
const sha256 = (b) => createHash('sha256').update(b).digest();
export function base58check(payload) {
	const b = Buffer.concat([payload, sha256(sha256(payload)).subarray(0, 4)]);
	let n = BigInt('0x' + b.toString('hex')), s = '';
	while (n > 0n) { s = B58[Number(n % 58n)] + s; n /= 58n; }
	for (const byte of b) { if (byte) break; s = '1' + s; }
	return s;
}
export function unbase58check(s) {
	let n = 0n;
	for (const c of s) n = n * 58n + BigInt(B58.indexOf(c));
	let hex = n.toString(16);
	if (hex.length % 2) hex = '0' + hex;
	const lead = s.match(/^1*/)[0].length;
	const b = Buffer.concat([Buffer.alloc(lead), Buffer.from(hex, 'hex')]);
	return b.subarray(0, -4);
}
