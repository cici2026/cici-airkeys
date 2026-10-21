<script lang="ts">
	import Mark from '$lib/ui/Mark.svelte';
	import Badge from '$lib/ui/Badge.svelte';
	import { registry } from '$lib/registry';
	const repo = 'https://github.com/mondaylabsltd/airkeys';
	const evmNames = registry.evm.filter((c) => !c.testnet).map((c) => c.name);
</script>

<svelte:head>
	<title>Airkeys — a cold wallet that never shows a public key</title>
	<meta
		name="description"
		content="Airkeys turns an offline phone into a cold wallet that shares addresses only. Bitcoin, Ethereum and EVM chains. Open source."
	/>
</svelte:head>

<header>
	<div class="brand"><Mark size={30} /><b>Airkeys</b></div>
	<nav>
		<a href="#how">How it works</a>
		<a href="#get">Get it</a>
		<a href={repo}>Source</a>
		<a class="button primary" href="app/">Open the web app</a>
	</nav>
</header>

<main>
	<section class="hero">
		<Mark size={88} />
		<h1>A cold wallet that never shows a public key.</h1>
		<p class="lede">
			Airkeys turns an offline phone into a cold wallet. It shows the world addresses — hashes of
			keys — and nothing more, so the funds behind them stay out of reach, even of a quantum
			computer.
		</p>
		<div class="cta">
			<a class="button primary" href="app/">Open the web app</a>
			<a class="button" href="#get">Get the phone app</a>
		</div>
	</section>

	<section class="problem">
		<div>
			<div class="label">Why</div>
			<h2>Every way of connecting a hardware wallet gives away public keys.</h2>
		</div>
		<div class="stack">
			<p class="dim">
				A quantum computer that breaks elliptic curves recovers a private key from its public key.
				Bitcoin and Ethereum addresses are hashes of public keys, so an address that has never sent
				keeps its key unknown — and its funds out of reach.
			</p>
			<ul>
				<li><b>Ledger</b> returns the public key with every address it gives the computer.</li>
				<li>
					<b>Trezor</b> can give addresses alone, but wallets connect it through an extended public key.
				</li>
				<li>
					<b>QR air-gapped wallets</b> sync extended public keys: every address, with its key.
				</li>
			</ul>
			<p>
				Airkeys shares addresses only, signs only after showing every detail, and keeps the rest of
				your funds hidden when you spend.
			</p>
		</div>
	</section>

	<section id="how" class="steps">
		<div class="label">How it works</div>
		<div class="grid3">
			<div class="card stack">
				<span class="n">1</span>
				<h3>An offline phone</h3>
				<p class="dim">
					Install Airkeys on a phone with Airplane mode on, Wi-Fi and Bluetooth off — it refuses to
					work otherwise. Enter your recovery phrase, or scan your SeedQR. Nothing is stored: close
					the app and it is gone.
				</p>
			</div>
			<div class="card stack">
				<span class="n">2</span>
				<h3>Addresses only</h3>
				<p class="dim">
					The phone shows an animated code with your addresses. This web app films it with the
					camera and watches balances. No public key, no extended key, ever crosses.
				</p>
			</div>
			<div class="card stack">
				<span class="n">3</span>
				<h3>Clear signing</h3>
				<p class="dim">
					The web app prepares a transfer and shows it as a code. The phone reads it, shows every
					recipient, amount and fee, and signs only when you hold the button. The web app
					broadcasts.
				</p>
			</div>
		</div>
	</section>

	<section class="bunker">
		<div class="card stack">
			<div class="label">Bunker mode</div>
			<h2>Spend from one account. The rest moves somewhere hidden.</h2>
			<p class="dim">
				Sending reveals the key of the account that sends. So Airkeys spends every Bitcoin coin of
				that account and sends the change to a fresh account, and on EVM chains moves everything
				else to a fresh account in the same signing. Every key is derived on a fully hardened path:
				one revealed key says nothing about any other.
			</p>
		</div>
	</section>

	<section class="networks">
		<div class="label">Networks</div>
		<div class="grid2">
			<div class="net card">
				<Badge network="bitcoin" />
				<div>
					<b>Bitcoin</b>
					<p class="small">BTC, to any address type. Native SegWit accounts.</p>
				</div>
			</div>
			<div class="net card">
				<Badge network="evm" />
				<div>
					<b>Ethereum & EVM</b>
					<p class="small">ETH and ERC-20 on {evmNames.join(', ')} — and any chain you add.</p>
				</div>
			</div>
		</div>
	</section>

	<section class="facts">
		<div class="label">What you can check</div>
		<ul class="grid2">
			<li>
				<b>No internet permission.</b> The Android app cannot open a connection; both apps stop while
				any radio is on.
			</li>
			<li>
				<b>Plain JSON.</b> Every code is readable: scan one with any
				<a href="https://lumencode.awesometools.dev/recv.html">Lumen Code reader</a> and see that it holds
				no key.
			</li>
			<li>
				<b>Transfers only.</b> The phone builds every byte it signs from a transfer's fields. It cannot
				be asked to sign anything else.
			</li>
			<li>
				<b>One core.</b> iOS and Android run the same Rust core, tested against Bitcoin Core and a fork
				of Ethereum mainnet.
			</li>
		</ul>
	</section>

	<section id="get" class="get">
		<div class="label">Get it</div>
		<div class="grid3">
			<div class="card stack">
				<h3>Android</h3>
				<p class="dim">Android 8 or later. Build the APK from source, or install a release.</p>
				<a href={repo}>Build instructions ↗</a>
			</div>
			<div class="card stack">
				<h3>iPhone</h3>
				<p class="dim">iOS 17 or later. Build with Xcode from the source.</p>
				<a href={repo}>Build instructions ↗</a>
			</div>
			<div class="card stack">
				<h3>This web app</h3>
				<p class="dim">
					A static site: serve the files from any static host over HTTPS (the camera needs a secure
					page; localhost works), or load them as a browser extension.
				</p>
				<a href="app/">Open it →</a>
			</div>
		</div>
	</section>
</main>

<footer>
	<span class="small"
		>Airkeys · open source · <a href="{repo}/blob/main/docs/PROTOCOL.md">protocol</a></span
	>
</footer>

<style>
	header {
		display: flex;
		align-items: center;
		gap: 16px;
		padding: 18px 32px;
		max-width: 1160px;
		margin: 0 auto;
	}
	.brand {
		display: flex;
		align-items: center;
		gap: 10px;
		font-size: 18px;
	}
	nav {
		margin-left: auto;
		display: flex;
		align-items: center;
		gap: 22px;
	}
	nav a:not(.button) {
		color: var(--dim);
	}
	main {
		max-width: 1160px;
		margin: 0 auto;
		padding: 0 32px;
		display: flex;
		flex-direction: column;
		gap: 110px;
	}
	.hero {
		padding: 90px 0 10px;
		display: flex;
		flex-direction: column;
		gap: 26px;
		max-width: 820px;
	}
	h1 {
		font-size: clamp(38px, 6vw, 64px);
		line-height: 1.04;
		letter-spacing: -0.035em;
	}
	h2 {
		font-size: clamp(26px, 3.4vw, 36px);
		line-height: 1.12;
	}
	.lede {
		font-size: 20px;
		color: var(--dim);
		max-width: 680px;
	}
	.cta {
		display: flex;
		gap: 12px;
		flex-wrap: wrap;
	}
	.cta .button {
		padding: 14px 22px;
		font-size: 16px;
	}
	.problem {
		display: grid;
		grid-template-columns: 1fr 1.2fr;
		gap: 48px;
	}
	.problem .label,
	.steps .label,
	.networks .label,
	.facts .label,
	.get .label {
		margin-bottom: 12px;
	}
	ul {
		margin: 0;
		padding-left: 20px;
		display: flex;
		flex-direction: column;
		gap: 8px;
		color: var(--dim);
	}
	ul b {
		color: var(--text);
	}
	.grid3 {
		display: grid;
		grid-template-columns: repeat(3, 1fr);
		gap: 18px;
	}
	.grid2 {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 18px;
	}
	.facts ul {
		list-style: none;
		padding: 0;
	}
	.facts li {
		background: var(--surface);
		border: 1px solid var(--stroke);
		border-radius: var(--radius);
		padding: 18px;
	}
	.n {
		width: 32px;
		height: 32px;
		border-radius: 10px;
		display: grid;
		place-items: center;
		background: color-mix(in srgb, var(--lamp) 14%, transparent);
		color: var(--lamp);
		font-weight: 700;
	}
	.bunker .card {
		padding: 36px;
		background: linear-gradient(
			135deg,
			color-mix(in srgb, var(--lamp) 9%, var(--surface)),
			var(--surface)
		);
		max-width: 900px;
	}
	.net {
		display: flex;
		gap: 16px;
		align-items: flex-start;
	}
	footer {
		max-width: 1160px;
		margin: 120px auto 0;
		padding: 28px 32px 48px;
		border-top: 1px solid var(--stroke);
	}
	@media (max-width: 820px) {
		header {
			padding: 14px 18px;
		}
		nav a:not(.button) {
			display: none;
		}
		main {
			padding: 0 18px;
			gap: 72px;
		}
		.hero {
			padding-top: 40px;
		}
		.problem,
		.grid2,
		.grid3 {
			grid-template-columns: 1fr;
			gap: 14px;
		}
	}
</style>
