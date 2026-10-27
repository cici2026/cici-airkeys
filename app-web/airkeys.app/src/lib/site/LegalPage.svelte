<script lang="ts">
	// A legal document of airkeys.app (terms, privacy), in either language.
	import type { LegalDoc, Legal } from './legal';
	import { site } from './site';
	import Mark from '$lib/ui/Mark.svelte';
	import Analytics from './Analytics.svelte';

	let {
		doc,
		legal,
		lang,
		path
	}: { doc: LegalDoc; legal: Legal; lang: 'en' | 'zh'; path: 'terms' | 'privacy' } = $props();
	// relative links: the pages work from any folder they are hosted in
	const root = $derived(lang === 'en' ? '../' : '../../');
	const home = $derived(lang === 'en' ? '../' : '../');
	const other = $derived(lang === 'en' ? `../zh/${path}/` : `../../${path}/`);
	const url = $derived(`${site.url}/${lang === 'en' ? '' : 'zh/'}${path}/`);
</script>

<svelte:head>
	<title>{doc.title} · Airkeys</title>
	<meta name="description" content={doc.description} />
	<link rel="canonical" href={url} />
	<link rel="alternate" hreflang="en" href="{site.url}/{path}/" />
	<link rel="alternate" hreflang="zh-Hans" href="{site.url}/zh/{path}/" />
	<link rel="alternate" hreflang="x-default" href="{site.url}/{path}/" />
</svelte:head>

<Analytics />

<header class="bar">
	<nav class="wrap nav" aria-label="Airkeys">
		<a class="brand" href={home}><Mark size={28} /><span>Airkeys</span></a>
		<a class="pill" href={other} hreflang={lang === 'en' ? 'zh-Hans' : 'en'}
			>{lang === 'en' ? '中文' : 'English'}</a
		>
	</nav>
</header>

<main class="wrap doc">
	<h1>{doc.title}</h1>
	<p class="updated">{doc.updated}</p>
	<p class="intro">{doc.intro}</p>
	{#each doc.sections as section (section.title)}
		<section>
			<h2>{section.title}</h2>
			{#each section.paragraphs as paragraph (paragraph)}<p>{paragraph}</p>{/each}
		</section>
	{/each}
	<section>
		<h2>{legal.ui.contact}</h2>
		<p>
			{site.company} · {legal.ui.contactText}
			<a href="mailto:{site.email}" rel="external">{site.email}</a>
		</p>
	</section>
	<p class="back"><a href={home}>{legal.ui.back}</a></p>
</main>

<footer class="footer">
	<div class="wrap links">
		<a
			href="{root}{lang === 'en' ? '' : 'zh/'}terms/"
			aria-current={path === 'terms' ? 'page' : undefined}>{legal.ui.terms}</a
		>
		·
		<a
			href="{root}{lang === 'en' ? '' : 'zh/'}privacy/"
			aria-current={path === 'privacy' ? 'page' : undefined}>{legal.ui.privacy}</a
		>
		· <a href={site.repository} rel="external">GitHub</a>
		<p class="company">© 2026 {site.company}</p>
	</div>
</footer>

<style>
	.wrap {
		width: min(760px, 100% - 32px);
		margin-inline: auto;
	}
	.bar {
		border-bottom: 1px solid var(--stroke);
	}
	.nav {
		display: flex;
		align-items: center;
		justify-content: space-between;
		height: 64px;
	}
	.brand {
		display: inline-flex;
		align-items: center;
		gap: 10px;
		color: var(--text);
		font-weight: 650;
		font-size: 18px;
	}
	.brand:hover {
		text-decoration: none;
	}
	.pill {
		color: var(--text);
		border: 1px solid var(--stroke);
		background: var(--surface);
		border-radius: 999px;
		padding: 4px 14px;
		font-size: 14px;
	}
	.doc {
		padding-block: 56px 24px;
	}
	h1 {
		font-size: clamp(32px, 5vw, 44px);
		font-weight: 700;
		letter-spacing: -0.02em;
	}
	.updated {
		margin-top: 10px;
		color: var(--faint);
		font-size: 14px;
	}
	.intro {
		margin-top: 28px;
		font-size: 18px;
		color: var(--text);
	}
	section {
		margin-top: 36px;
	}
	h2 {
		font-size: 21px;
		font-weight: 650;
		margin-bottom: 10px;
	}
	p {
		line-height: 1.7;
		color: var(--dim);
	}
	section p + p {
		margin-top: 12px;
	}
	.back {
		margin-top: 48px;
	}
	.footer {
		margin-top: 48px;
		border-top: 1px solid var(--stroke);
		padding-block: 28px;
		font-size: 14px;
		color: var(--faint);
	}
	.company {
		margin-top: 8px;
		color: var(--faint);
	}
	.links a[aria-current] {
		color: var(--text);
	}
</style>
