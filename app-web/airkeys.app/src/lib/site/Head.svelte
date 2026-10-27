<script lang="ts">
	// The page's title, description, languages and preview, for search engines and link cards.
	import type { Content } from './content';
	import { site } from './site';
	import Analytics from './Analytics.svelte';

	let { c }: { c: Content } = $props();
	const url = $derived(site.url + c.path);
</script>

<svelte:head>
	<title>{c.meta.title}</title>
	<meta name="description" content={c.meta.description} />
	<link rel="canonical" href={url} />
	<link rel="alternate" hreflang="en" href="{site.url}/" />
	<link rel="alternate" hreflang="zh-Hans" href="{site.url}/zh/" />
	<link rel="alternate" hreflang="x-default" href="{site.url}/" />
	<meta property="og:type" content="website" />
	<meta property="og:site_name" content="Airkeys" />
	<meta property="og:title" content={c.meta.title} />
	<meta property="og:description" content={c.meta.description} />
	<meta property="og:url" content={url} />
	<meta property="og:image" content="{site.url}/screens/card.png" />
	<meta name="twitter:card" content="summary_large_image" />
</svelte:head>

<Analytics />
