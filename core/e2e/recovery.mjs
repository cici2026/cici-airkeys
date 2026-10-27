// Recovery end to end: the recipes of docs/RECOVERY.md, run with the tools they name, must find
// exactly the accounts the phone shows — with and without a passphrase. The root-key script is
// taken from the document itself; on regtest, a Bitcoin Core wallet made from it sweeps coins.
//
//   BITCOIND=/path/to/bitcoind node e2e/recovery.mjs        (needs `cast` and `python3`)
import { execFileSync } from 'node:child_process';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { CORE, PHRASE, base58check, buildSigner, check, done, phone, segwitAddress, segwitProgram, start, unbase58check } from './lib.mjs';

const BITCOIND = process.env.BITCOIND ?? 'bitcoind';
const CLI = BITCOIND.includes('/') ? path.join(path.dirname(BITCOIND), 'bitcoin-cli') : 'bitcoin-cli';
const PASSPHRASES = ['', 'bunker 7'];
const INDEXES = [0, 1, 7];
const run = (cmd, args, input) => execFileSync(cmd, args, { input, stdio: ['pipe', 'pipe', 'pipe'] }).toString().trim();
const json = (s) => { try { return JSON.parse(s); } catch { return s; } };

buildSigner();
const accounts = (network, pass) => {
	process.env.AIRKEYS_PASSPHRASE = pass;
	return phone(['accounts', network, ...INDEXES.map(String)]).accounts.map((a) => a.address);
};
const label = (i, pass) => `#${i}${pass ? ' with a passphrase' : ''}`;

// the document's root-key script, its two prompts answered from stdin
const doc = fs.readFileSync(path.join(CORE, '../docs/RECOVERY.md'), 'utf8');
const script = doc.match(/```python\n([\s\S]*?)```/)[1];
const rootKey = (pass) =>
	run('python3', ['-c', `import getpass, sys\ngetpass.getpass = lambda prompt="": sys.stdin.readline().rstrip("\\n")\n${script}`], `${PHRASE}\n${pass}\n`);

// ---------------------------------------------------------------- EVM: cast
for (const pass of PASSPHRASES) {
	const evm = accounts('evm', pass);
	INDEXES.forEach((i, k) => {
		const derive = (what) =>
			run('cast', ['wallet', what, '--mnemonic', PHRASE, '--mnemonic-derivation-path', `m/65738275'/44'/60'/0'/0'/${i}'`, ...(pass ? ['--mnemonic-passphrase', pass] : [])]);
		const address = derive('address');
		const key = derive('private-key');
		check(address === evm[k] && run('cast', ['wallet', 'address', '--private-key', key]) === evm[k], `cast: EVM ${label(i, pass)} ${address}, and its private key`);
	});
}

// ---------------------------------------------------------------- Bitcoin: Bitcoin Core
async function bitcoinCore(chain, port) {
	const dir = fs.mkdtempSync(path.join(os.tmpdir(), `airkeys-recovery-${chain}-`));
	const proc = await start(BITCOIND, [`-chain=${chain}`, `-datadir=${dir}`, `-rpcport=${port}`, '-rpcuser=a', '-rpcpassword=b', '-connect=0', '-fallbackfee=0.0002', '-printtoconsole'], 'init message: Done loading');
	const cli = (...args) => json(run(CLI, [`-chain=${chain}`, `-rpcport=${port}`, '-rpcuser=a', '-rpcpassword=b', ...args]));
	const stop = () => new Promise((ok) => { proc.once('exit', ok); proc.kill(); }).then(() => fs.rmSync(dir, { recursive: true, force: true }));
	return { cli, stop };
}
const descriptor = (cli, key) => {
	const d = `wpkh(${key}/65738275h/84h/0h/0h/0h/*h)`;
	return `${d}#${cli('getdescriptorinfo', d).checksum}`;
};

// mainnet, no peers: the descriptor of the document derives the phone's addresses
{
	const { cli, stop } = await bitcoinCore('main', 18601);
	try {
		for (const pass of PASSPHRASES) {
			const want = accounts('bitcoin', pass);
			const got = cli('deriveaddresses', descriptor(cli, rootKey(pass)), '[0,7]');
			INDEXES.forEach((i, k) => check(got[i] === want[k], `Bitcoin Core: ${label(i, pass)} ${got[i]}`));
		}
	} finally {
		await stop();
	}
}

// regtest: coins on accounts #0 and #7 are found and swept by a wallet holding only that descriptor
{
	const { cli, stop } = await bitcoinCore('regtest', 18602);
	try {
		// the same root key, spelled for test networks (tprv)
		const tprv = base58check(Buffer.concat([Buffer.from('04358394', 'hex'), unbase58check(rootKey('')).subarray(4)]));
		cli('createwallet', 'miner');
		const mine = (n) => cli('generatetoaddress', String(n), cli('-rpcwallet=miner', 'getnewaddress'));
		mine(101);
		const [a0, , a7] = accounts('bitcoin', '').map((a) => segwitAddress('bcrt', 0, segwitProgram(a).program));
		cli('-rpcwallet=miner', 'sendtoaddress', a0, '1.5');
		cli('-rpcwallet=miner', 'sendtoaddress', a7, '0.25');
		mine(1);

		cli('-named', 'createwallet', 'wallet_name=airkeys', 'blank=true');
		const imported = cli('-rpcwallet=airkeys', 'importdescriptors', JSON.stringify([{ desc: descriptor(cli, tprv), timestamp: 0, range: [0, 99] }]));
		check(imported[0]?.success === true, 'Bitcoin Core imports the hardened descriptor');
		check(cli('-rpcwallet=airkeys', 'getbalance') === 1.75, 'the recovered wallet finds the 1.75 BTC on accounts #0 and #7');
		const to = cli('-rpcwallet=miner', 'getnewaddress');
		const sent = cli('-rpcwallet=airkeys', 'sendall', JSON.stringify([to]));
		mine(1);
		const got = cli('-rpcwallet=miner', 'getreceivedbyaddress', to);
		check(sent.complete && cli('-rpcwallet=airkeys', 'getbalance') === 0 && got > 1.7499 && got < 1.75, `sendall swept them: ${got} BTC arrived, in a mined transaction`);
	} finally {
		await stop();
	}
}

done();
