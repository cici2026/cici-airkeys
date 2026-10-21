<script lang="ts">
	import type { Asset } from '$lib/chains';
	import type { Account } from '$lib/protocol';
	import Wallet from '$lib/app/Wallet.svelte';
	import AccountPage from '$lib/app/AccountPage.svelte';
	import Send from '$lib/app/Send.svelte';
	import Connect from '$lib/app/Connect.svelte';
	import Settings from '$lib/app/Settings.svelte';
	import Mark from '$lib/ui/Mark.svelte';
	import { save, store } from '$lib/state.svelte';

	/** Mainnet or testnet: never both on screen. Switching starts again from the accounts. */
	function mode(testnet: boolean) {
		if (!!store.settings.testnet === testnet) return;
		store.settings.testnet = testnet;
		save();
		go({ name: 'wallet' });
	}

	type View =
		| { name: 'wallet' }
		| { name: 'connect' }
		| { name: 'settings' }
		| { name: 'account'; account: Account }
		| { name: 'send'; account: Account; asset: Asset };
	let view = $state<View>({ name: 'wallet' });
	let toast = $state('');
	const go = (v: View) => ((view = v), scrollTo(0, 0));
	/** Where the logo leads: the site, or nowhere inside the extension. */
	let { home = '../' }: { home?: string } = $props();
</script>

<svelte:head><title>Airkeys</title></svelte:head>

<header>
	<!-- the name is hidden on a phone: the link keeps it -->
	<a class="brand" href={home} aria-label="Airkeys"><Mark size={30} /><b>Airkeys</b></a>
	<div class="mode" role="group" aria-label="Networks shown">
		<button
			class:on={!store.settings.testnet}
			aria-pressed={!store.settings.testnet}
			onclick={() => mode(false)}>Mainnet</button
		>
		<button
			class:on={!!store.settings.testnet}
			aria-pressed={!!store.settings.testnet}
			class="test"
			onclick={() => mode(true)}>Testnet</button
		>
	</div>
	<nav>
		<button
			class:active={view.name === 'wallet'}
			class="ghost"
			onclick={() => go({ name: 'wallet' })}>Accounts</button
		>
		<button
			class:active={view.name === 'settings'}
			class="ghost"
			onclick={() => go({ name: 'settings' })}>Settings</button
		>
		<button class="primary" onclick={() => go({ name: 'connect' })}>Scan phone</button>
	</nav>
</header>

{#if store.settings.testnet}
	<div class="testnet">
		Testnet — coins on test networks have no value. Real balances are under Mainnet.
	</div>
{/if}

<main>
	<!-- a live region from the start, so a reader announces what appears in it -->
	<div role="status">
		{#if toast}<div class="notice info toast">{toast}</div>{/if}
	</div>
	{#if view.name === 'wallet'}
		<Wallet
			open={(account) => go({ name: 'account', account })}
			connect={() => go({ name: 'connect' })}
		/>
	{:else if view.name === 'connect'}
		<Connect
			done={(m) => ((toast = m), go({ name: 'wallet' }), setTimeout(() => (toast = ''), 4000))}
		/>
	{:else if view.name === 'settings'}
		<Settings />
	{:else if view.name === 'account'}
		{@const account = view.account}
		<AccountPage
			{account}
			back={() => go({ name: 'wallet' })}
			send={(asset) => go({ name: 'send', account, asset })}
		/>
	{:else if view.name === 'send'}
		{@const account = view.account}
		<Send {account} asset={view.asset} back={() => go({ name: 'account', account })} />
	{/if}
</main>

<style>
	header {
		position: sticky;
		top: 0;
		z-index: 2;
		display: flex;
		align-items: center;
		gap: 16px;
		padding: 14px 24px;
		background: color-mix(in srgb, var(--bg) 88%, transparent);
		backdrop-filter: blur(10px);
		border-bottom: 1px solid var(--stroke);
	}
	.brand {
		display: flex;
		align-items: center;
		gap: 10px;
		color: var(--text);
		font-size: 18px;
		text-decoration: none;
	}
	.mode {
		display: flex;
		padding: 3px;
		border-radius: 10px;
		background: var(--surface);
		border: 1px solid var(--stroke);
	}
	.mode button {
		padding: 5px 12px;
		border: 0;
		border-radius: 7px;
		background: none;
		color: var(--dim);
		font-size: 13px;
		font-weight: 600;
	}
	.mode button.on {
		background: var(--raised);
		color: var(--text);
	}
	.mode button.test.on {
		background: color-mix(in srgb, var(--warn) 22%, var(--raised));
		color: var(--warn);
	}
	.testnet {
		position: sticky;
		top: 61px;
		z-index: 1;
		padding: 8px 24px;
		text-align: center;
		font-size: 13px;
		font-weight: 600;
		color: var(--warn);
		background: color-mix(in srgb, var(--warn) 12%, var(--bg));
		border-bottom: 1px solid color-mix(in srgb, var(--warn) 30%, var(--stroke));
	}
	nav {
		margin-left: auto;
		display: flex;
		gap: 6px;
	}
	nav .active {
		color: var(--text);
		background: var(--raised);
	}
	main {
		max-width: 920px;
		margin: 0 auto;
		padding: 28px 24px 80px;
	}
	.toast {
		margin-bottom: 16px;
	}
	@media (max-width: 560px) {
		/* two rows: the brand and the mode; then the three places, side by side */
		header {
			flex-wrap: wrap;
			gap: 10px;
			padding: 10px 14px;
		}
		.mode {
			margin-left: auto;
		}
		nav {
			order: 3;
			width: 100%;
			margin-left: 0;
		}
		nav button {
			flex: 1;
			white-space: nowrap;
			padding-inline: 8px;
		}
		.testnet {
			top: 108px;
		}
		.brand b {
			display: none;
		}
		main {
			padding: 18px 14px 60px;
		}
	}
</style>
