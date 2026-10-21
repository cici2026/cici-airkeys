// Shared by the chain checks (bun scripts/chains/<network>.ts): the phone — the core's own
// signing path — and processes, waits and checks.
import { execFileSync, spawn, type ChildProcess } from 'node:child_process';
import path from 'node:path';
import { parseMessage, type Account, type SignRequest, type Signed } from '../../src/lib/protocol';

export type { Account };
export const ROOT = path.resolve(import.meta.dir, '../../../..');
const CORE = path.join(ROOT, 'core');
const SIGNER = path.join(CORE, 'target/release/examples/sign');
export const PHRASE = 'test test test test test test test test test test test junk';

execFileSync('cargo', ['build', '--release', '-q', '-p', 'airkeys-wallet', '--example', 'sign'], {
	cwd: CORE,
	stdio: 'inherit'
});

function phone(args: string[], input = ''): Buffer {
	return execFileSync(SIGNER, args, {
		input,
		env: { ...process.env, AIRKEYS_PHRASE: PHRASE },
		stdio: ['pipe', 'pipe', 'pipe']
	});
}

/** The accounts the phone shows, read by the web app's parser. */
export function accounts(network: string, ...indexes: number[]): Account[] {
	const m = parseMessage(phone(['accounts', network, ...indexes.map(String)]));
	if (m.type !== 'accounts') throw new Error('not an accounts message');
	return m.accounts;
}

/** The phone signs (or throws with its reason); the web app reads the reply as from its camera. */
export function sign(request: SignRequest): Signed {
	let out: Buffer;
	try {
		out = phone([], JSON.stringify(request));
	} catch (e) {
		throw new Error(`the phone refused: ${(e as { stderr: Buffer }).stderr}`);
	}
	const m = parseMessage(out);
	if (m.type !== 'signed') throw new Error('not a signed message');
	return m.signed;
}

const children: ChildProcess[] = [];
process.on('exit', () => children.forEach((c) => c.kill('SIGKILL')));
for (const s of ['SIGINT', 'SIGTERM'] as const) process.on(s, () => process.exit(130));

/** Starts a process and waits for a line of its output (`null`: does not wait). */
export function start(
	cmd: string,
	args: string[],
	ready: string | RegExp | null,
	cwd?: string
): Promise<ChildProcess> {
	return new Promise((resolve, reject) => {
		const p = spawn(cmd, args, { stdio: ['ignore', 'pipe', 'pipe'], cwd });
		children.push(p);
		let log = '';
		if (ready === null) resolve(p);
		const seen = (d: Buffer) => {
			log = (log + d).slice(-20_000);
			if (ready !== null && (typeof ready === 'string' ? log.includes(ready) : ready.test(log)))
				resolve(p);
		};
		p.stdout!.on('data', seen);
		p.stderr!.on('data', seen);
		p.on('exit', (code) => reject(new Error(`${cmd} exited ${code}:\n${log.slice(-3000)}`)));
		p.on('error', (e) => reject(new Error(`${cmd} did not start: ${e.message}`)));
	});
}

export async function until<T>(
	what: string,
	f: () => Promise<T | null | undefined | false>,
	ms = 60_000
): Promise<T> {
	const end = Date.now() + ms;
	for (;;) {
		try {
			const v = await f();
			if (v) return v;
		} catch {
			// not yet
		}
		if (Date.now() > end) throw new Error(`timed out waiting for ${what}`);
		await new Promise((r) => setTimeout(r, 250));
	}
}

let failures = 0;
export function check(ok: boolean, what: string) {
	console.log(`${ok ? '  ok ' : 'FAIL '} ${what}`);
	if (!ok) failures++;
}

export async function refuses(what: string, f: () => unknown) {
	try {
		await f();
		check(false, `${what}: accepted`);
	} catch (e) {
		check(true, `${what}: “${(e as Error).message.split('\n')[0]}”`);
	}
}

export function done(): never {
	console.log(failures ? `\n${failures} check(s) failed` : '\nall checks passed');
	process.exit(failures ? 1 : 0);
}
