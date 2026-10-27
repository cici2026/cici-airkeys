<script lang="ts">
	// A page that is not there (the static host's 404.html lands here), or one that failed.
	import { page } from '$app/state';
	import { resolve } from '$app/paths';
	import Mark from '$lib/ui/Mark.svelte';
	import { lang, t } from '$lib/i18n/index.svelte';
	// the language chosen in the app, over the browser's
	import '$lib/state.svelte';
</script>

<svelte:head><title>Airkeys · {page.status}</title></svelte:head>

<main class="stack">
	<Mark size={56} />
	<h1>{page.status === 404 ? t.error.notFound : t.error.failed}</h1>
	{#if page.status !== 404 && page.error?.message}<p class="dim">{page.error.message}</p>{/if}
	<div class="row">
		<a class="button primary" href={lang.current === 'zh' ? resolve('/zh/') : resolve('/')}
			>Airkeys</a
		>
		<a class="button" href={resolve('/app/')}>{t.error.openApp}</a>
	</div>
</main>

<style>
	main {
		max-width: 620px;
		margin: 0 auto;
		padding: 90px 32px;
		align-items: flex-start;
	}
	h1 {
		font-size: clamp(30px, 5vw, 44px);
		line-height: 1.1;
	}
</style>
