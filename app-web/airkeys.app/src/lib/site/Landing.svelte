<script lang="ts">
	// The landing page of airkeys.app, in either language: the words are content.ts's.
	import type { Content } from './content';
	import { site, repoFile, securityReport, listings, launched, extension } from './site';
	import { registry } from '#lib/registry.ts';
	import Mark from '#lib/ui/Mark.svelte';
	import Badge from '#lib/ui/Badge.svelte';

	let { c }: { c: Content } = $props();
	const chains = registry.evm.filter((x) => !x.testnet).map((x) => x.name);
	// the web app, relative so the page works from any folder it is hosted in
	const app = $derived(c.path === '/' ? 'app/' : '../app/');
	const home = $derived(c.path === '/' ? './' : '../');
	const other = $derived(c.path === '/' ? 'zh/' : '../');
	const screens = $derived(c.path === '/' ? 'screens/' : '../screens/');

	// 24-pixel line icons for the six protections, in the order of c.features.items
	const icons = [
		// an airplane
		'M10.5 21l1.5-1v-5.5l8 2.5v-2l-8-5V4.5a1.5 1.5 0 0 0-3 0V10l-8 5v2l8-2.5V20l1.5 1',
		// a storage cylinder, struck through
		'M5 5.5a7 2.5 0 1 0 14 0a7 2.5 0 1 0-14 0M5 5.5v13c0 1.4 3.1 2.5 7 2.5s7-1.1 7-2.5v-13M5 12c0 1.4 3.1 2.5 7 2.5s7-1.1 7-2.5M3 3l18 18',
		// an eye
		'M2 12s3.5-7 10-7 10 7 10 7-3.5 7-10 7S2 12 2 12zM12 15a3 3 0 1 0 0-6 3 3 0 0 0 0 6z',
		// braces
		'M8 3H7a2 2 0 0 0-2 2v4a2 2 0 0 1-2 2 2 2 0 0 1 2 2v4a2 2 0 0 0 2 2h1M16 3h1a2 2 0 0 1 2 2v4a2 2 0 0 0 2 2 2 2 0 0 0-2 2v4a2 2 0 0 1-2 2h-1',
		// a screen with a lock
		'M4 4h16a1 1 0 0 1 1 1v11a1 1 0 0 1-1 1H4a1 1 0 0 1-1-1V5a1 1 0 0 1 1-1zM8 21h8M12 17v4M10 11h4v3h-4zM10.5 11V9.5a1.5 1.5 0 0 1 3 0V11',
		// two phones, one core
		'M5 4h5a1 1 0 0 1 1 1v14a1 1 0 0 1-1 1H5a1 1 0 0 1-1-1V5a1 1 0 0 1 1-1zM14 4h5a1 1 0 0 1 1 1v14a1 1 0 0 1-1 1h-5a1 1 0 0 1-1-1V5a1 1 0 0 1 1-1zM7 17h1M16 17h1'
	];
</script>

<a class="skip" href="#main">{c.lang === 'zh' ? '跳到正文' : 'Skip to content'}</a>

<header class="bar">
	<nav class="wrap nav" aria-label="Airkeys">
		<a class="brand" href={home}><Mark size={30} /><span>Airkeys</span></a>
		<div class="links">
			<a href="#how">{c.nav.how}</a>
			<a href="#security">{c.nav.security}</a>
			<a href="#pricing">{c.nav.pricing}</a>
			<a href="#faq">{c.nav.faq}</a>
			<a href={site.repository} rel="external">{c.nav.source}</a>
			<a class="pill" href={other} hreflang={c.other.hreflang} lang={c.other.hreflang}
				>{c.other.label}</a
			>
			<a class="button tonal small-button" href={app}>{c.nav.app}</a>
		</div>
	</nav>
</header>

<main id="main">
	<section class="hero wrap">
		<div class="hero-text">
			<p class="eyebrow">{c.hero.eyebrow}</p>
			<h1>{c.hero.title}</h1>
			<p class="lead">{c.hero.lead}</p>
			<div class="actions">
				<a class="button primary" href="#pricing">{c.hero.primary}</a>
				<a class="button tonal" href="#how">{c.hero.secondary}</a>
			</div>
			<p class="note">{c.hero.note}</p>
		</div>
		<div class="hero-phones">
			{#each c.shots as shot, i (shot.src)}
				<img
					class="phone phone-{i}"
					src={screens + shot.src}
					alt={shot.alt}
					width="600"
					height="1304"
					loading={i ? 'lazy' : 'eager'}
				/>
			{/each}
		</div>
	</section>

	<ul class="facts wrap" aria-label="Airkeys">
		{#each c.facts as fact (fact)}<li>{fact}</li>{/each}
	</ul>

	<section id="why" class="section wrap">
		<h2>{c.why.title}</h2>
		<p class="section-lead">{c.why.lead}</p>
		<div class="card table-card">
			<table>
				<thead>
					<tr><th scope="col">{c.why.head[0]}</th><th scope="col">{c.why.head[1]}</th></tr>
				</thead>
				<tbody>
					{#each c.why.rows as row, i (row[0])}
						<tr class:ours={i === c.why.rows.length - 1}>
							<th scope="row">{row[0]}</th>
							<td>{row[1]}</td>
						</tr>
					{/each}
				</tbody>
			</table>
			<p class="table-note">{c.why.note}</p>
		</div>
	</section>

	<section id="how" class="section wrap">
		<h2>{c.how.title}</h2>
		<p class="section-lead">{c.how.lead}</p>
		<div
			class="flow"
			role="img"
			aria-label="{c.how.flow.phone} ⇄ {c.how.flow.code} ⇄ {c.how.flow.web}"
		>
			<div class="chip">
				<strong>{c.how.flow.phone}</strong><small>{c.how.flow.phoneNote}</small>
			</div>
			<div class="arrows" aria-hidden="true"><span></span><span></span></div>
			<div class="chip lamp">
				<strong>{c.how.flow.code}</strong><small>{c.how.flow.codeNote}</small>
			</div>
			<div class="arrows" aria-hidden="true"><span></span><span></span></div>
			<div class="chip">
				<strong>{c.how.flow.web}</strong><small>{c.how.flow.webNote}</small>
			</div>
		</div>
		<ol class="steps">
			{#each c.how.steps as step, i (step.title)}
				<li>
					<span class="step-number">{i + 1}</span>
					<h3>{step.title}</h3>
					<p>{step.text}</p>
				</li>
			{/each}
		</ol>
	</section>

	<section class="section wrap bunker">
		<div class="bunker-text">
			<h2>{c.bunker.title}</h2>
			<p class="section-lead">{c.bunker.lead}</p>
			<ul class="open">
				{#each c.bunker.points as point (point.title)}
					<li>
						<h3>{point.title}</h3>
						<p>{point.text}</p>
					</li>
				{/each}
			</ul>
		</div>
		<img
			class="screen panel"
			src={screens + 'bunker.jpg'}
			alt={c.bunker.alt}
			width="720"
			height="303"
			loading="lazy"
		/>
	</section>

	<section id="security" class="section wrap">
		<h2>{c.features.title}</h2>
		<p class="section-lead">{c.features.lead}</p>
		<ul class="features">
			{#each c.features.items as item, i (item.title)}
				<li class="card">
					<svg class="icon" viewBox="0 0 24 24" aria-hidden="true"><path d={icons[i]} /></svg>
					<h3>{item.title}</h3>
					<p>{item.text}</p>
				</li>
			{/each}
		</ul>
	</section>

	<section class="section wrap">
		<h2>{c.model.title}</h2>
		<div class="card table-card">
			<table class="model">
				<thead>
					<tr><th scope="col">{c.model.head[0]}</th><th scope="col">{c.model.head[1]}</th></tr>
				</thead>
				<tbody>
					{#each c.model.rows as row (row[0])}
						<tr><th scope="row">{row[0]}</th><td>{row[1]}</td></tr>
					{/each}
				</tbody>
			</table>
		</div>
		<ul class="points">
			{#each c.model.points as point (point)}<li>{point}</li>{/each}
		</ul>
	</section>

	<section class="section wrap">
		<h2>{c.networks.title}</h2>
		<p class="section-lead">{c.networks.lead}</p>
		<div class="networks">
			<div class="card net">
				<Badge network="bitcoin" size={44} />
				<div>
					<h3>Bitcoin</h3>
					<p>{c.networks.bitcoin}</p>
				</div>
			</div>
			<div class="card net">
				<Badge network="evm" size={44} />
				<div>
					<h3>Ethereum & EVM</h3>
					<p>{c.networks.evm} {chains.join(', ')}, {c.networks.more}</p>
				</div>
			</div>
		</div>
	</section>

	<section class="section wrap">
		<h2>{c.web.title}</h2>
		<p class="section-lead">{c.web.lead}</p>
		<img
			class="screen"
			src={screens + 'web.jpg'}
			alt={c.web.alt}
			width="1472"
			height="786"
			loading="lazy"
		/>
		<ul class="ticks">
			{#each c.web.points as point (point)}<li>{point}</li>{/each}
		</ul>
		<div class="actions">
			<a class="button tonal" href={app}>{c.web.open}</a>
			<a class="button tonal" href={extension.download} rel="external">{c.web.extension}</a>
		</div>
		<p class="caption">
			{c.web.install}<a href={extension.releases} rel="external">{c.web.releases}</a>{c.web.after}
		</p>
	</section>

	<section class="section wrap platforms">
		<div>
			<h2>{c.platforms.title}</h2>
			<p class="section-lead">{c.platforms.lead}</p>
			<p class="caption">{c.platforms.caption}</p>
		</div>
		<div class="pair">
			<figure>
				<img
					class="phone"
					src={screens + 'android.png'}
					alt="Android"
					width="600"
					height="1333"
					loading="lazy"
				/>
				<figcaption>{c.platforms.android}</figcaption>
			</figure>
			<figure>
				<img
					class="phone"
					src={screens + 'review.png'}
					alt="iPhone"
					width="600"
					height="1304"
					loading="lazy"
				/>
				<figcaption>{c.platforms.ios}</figcaption>
			</figure>
		</div>
	</section>

	<section class="section wrap">
		<h2>{c.open.title}</h2>
		<p class="section-lead">{c.open.lead}</p>
		<ul class="open two">
			{#each c.open.items as item (item.title)}
				<li>
					<h3>{item.title}</h3>
					<p>{item.text}</p>
					{#if item.link}
						<a class="more" href={repoFile(item.link.path)} rel="external">{item.link.label} →</a>
					{/if}
				</li>
			{/each}
		</ul>
		<div class="actions">
			<a class="button tonal" href={repoFile('docs/PROTOCOL.md')} rel="external"
				>{c.open.protocol}</a
			>
			<a class="button tonal" href={repoFile('SECURITY.md')} rel="external">{c.open.security}</a>
			<a class="button tonal" href={site.repository} rel="external">{c.open.source}</a>
		</div>
	</section>

	<section id="pricing" class="section wrap">
		<h2>{c.pricing.title}</h2>
		<p class="section-lead">{c.pricing.lead}</p>
		<div class="plans">
			<div class="card plan">
				<h3>{c.pricing.build.name}</h3>
				<p class="price">{c.pricing.build.price} <small>{c.pricing.build.per}</small></p>
				<ul>
					{#each c.pricing.build.points as point (point)}<li>{point}</li>{/each}
				</ul>
				<a class="button tonal" href={repoFile(c.pricing.build.guide)} rel="external"
					>{c.pricing.build.action}</a
				>
			</div>
			<div class="card plan featured">
				<h3>{c.pricing.store.name}</h3>
				<p class="price">{c.pricing.store.price} <small>{c.pricing.store.per}</small></p>
				<ul>
					{#each c.pricing.store.points as point (point)}<li>{point}</li>{/each}
				</ul>
				{#if launched}
					<div class="plan-stores">
						{#each listings.filter((store) => store.url) as store (store.name)}
							<a class="button primary" href={store.url} rel="external">{store.name}</a>
						{/each}
					</div>
				{:else}
					<span class="store" aria-disabled="true">{c.cta.soon}</span>
				{/if}
			</div>
		</div>
		<p class="caption">{c.pricing.note}</p>
	</section>

	<section id="faq" class="section wrap faq">
		<h2>{c.faq.title}</h2>
		<div class="questions">
			{#each c.faq.items as item (item.q)}
				<details>
					<summary>{item.q}</summary>
					<p>{item.a}</p>
				</details>
			{/each}
		</div>
	</section>

	<section class="cta wrap">
		<Mark size={64} />
		<h2>{c.cta.title}</h2>
		<p>{c.cta.text}</p>
		<div class="stores">
			{#each listings as store (store.name)}
				{#if store.url}
					<a class="button primary" href={store.url} rel="external">{store.name}</a>
				{:else}
					<span class="store" aria-disabled="true">{store.name} · {c.cta.soon}</span>
				{/if}
			{/each}
		</div>
		<a class="button tonal" href={app}>{c.cta.app}</a>
	</section>
</main>

<footer class="footer">
	<div class="wrap footer-inner">
		<div class="brand"><Mark size={24} /><span>Airkeys</span></div>
		<p>{c.footer.license} {c.footer.warning}</p>
		<p>
			<a href={repoFile('docs/PROTOCOL.md')} rel="external">{c.footer.protocol}</a>
			· <a href={securityReport} rel="external">{c.footer.security}</a>
			· <a href="terms/">{c.footer.terms}</a>
			· <a href="privacy/">{c.footer.privacy}</a>
			· <a href={site.repository} rel="external">GitHub</a>
			· <a href={other} hreflang={c.other.hreflang}>{c.other.label}</a>
		</p>
		<p class="quiet">
			© 2026 {site.company} · <a href="mailto:{site.email}" rel="external">{site.email}</a>
		</p>
	</div>
</footer>

<style>
	.wrap {
		width: min(1120px, 100% - 2 * var(--gutter));
		margin-inline: auto;
	}
	:global(body) {
		--gutter: 20px;
		overflow-x: hidden;
	}
	:global(html) {
		scroll-behavior: smooth;
		scroll-padding-top: 80px;
	}
	@media (prefers-reduced-motion: reduce) {
		:global(html) {
			scroll-behavior: auto;
		}
	}
	.skip {
		position: absolute;
		left: -999px;
		top: 8px;
		background: var(--surface);
		padding: 8px 12px;
		border-radius: 8px;
		z-index: 10;
	}
	.skip:focus {
		left: 8px;
	}

	/* Header */
	.bar {
		position: sticky;
		top: 0;
		z-index: 5;
		background: color-mix(in srgb, var(--bg) 82%, transparent);
		backdrop-filter: saturate(1.4) blur(12px);
		-webkit-backdrop-filter: saturate(1.4) blur(12px);
		border-bottom: 1px solid color-mix(in srgb, var(--stroke) 70%, transparent);
	}
	.nav {
		display: flex;
		align-items: center;
		justify-content: space-between;
		height: 64px;
		gap: 16px;
	}
	.brand {
		display: inline-flex;
		align-items: center;
		gap: 10px;
		color: var(--text);
		font-weight: 650;
		font-size: 19px;
	}
	.brand:hover {
		text-decoration: none;
	}
	.links {
		display: flex;
		align-items: center;
		gap: 22px;
		font-size: 15px;
	}
	.links a:not(.button) {
		color: var(--dim);
	}
	.links a:not(.button):hover {
		color: var(--text);
		text-decoration: none;
	}
	.links a.pill {
		color: var(--text);
		border: 1px solid var(--stroke);
		background: var(--surface);
		border-radius: 999px;
		padding: 4px 14px;
	}
	@media (max-width: 900px) {
		.links a:not(.pill):not(.button) {
			display: none;
		}
	}
	@media (max-width: 420px) {
		.small-button {
			display: none;
		}
	}

	/* Buttons */
	.button {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		min-height: 50px;
		padding: 0 24px;
		border-radius: 14px;
		font-weight: 600;
		font-size: 16px;
		border: none;
		transition:
			transform 0.15s ease,
			filter 0.15s ease;
	}
	.button:hover {
		transform: translateY(-1px);
		filter: brightness(1.05);
		text-decoration: none;
	}
	.primary {
		background: var(--lamp);
		color: var(--on-lamp);
		box-shadow: 0 8px 28px rgb(242 181 68 / 22%);
	}
	.small-button {
		min-height: 38px;
		padding: 0 16px;
		font-size: 14px;
		border-radius: 11px;
		box-shadow: none;
	}
	.tonal {
		background: color-mix(in srgb, var(--lamp) 12%, var(--surface));
		color: var(--lamp);
	}

	/* Hero */
	.hero {
		display: grid;
		grid-template-columns: 1.05fr 1fr;
		gap: 48px;
		align-items: center;
		padding-block: 72px 40px;
	}
	.eyebrow {
		color: var(--lamp);
		font-weight: 600;
		font-size: 14px;
		letter-spacing: 0.02em;
		margin-bottom: 16px;
	}
	h1 {
		text-wrap: balance;
		font-size: clamp(38px, 5.6vw, 62px);
		font-weight: 700;
		line-height: 1.08;
		letter-spacing: -0.03em;
	}
	.lead {
		font-size: clamp(17px, 1.6vw, 20px);
		color: var(--dim);
		margin-top: 20px;
		max-width: 36em;
		line-height: 1.6;
	}
	.actions {
		display: flex;
		flex-wrap: wrap;
		gap: 12px;
		margin-top: 32px;
	}
	.note {
		margin-top: 16px;
		font-size: 14px;
		color: var(--faint);
	}
	.hero-phones {
		position: relative;
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 20px;
		align-items: start;
	}
	.hero-phones::before {
		content: '';
		position: absolute;
		inset: 10% -10% 0;
		background: radial-gradient(closest-side, rgb(242 181 68 / 18%), transparent);
		z-index: -1;
	}
	.phone {
		width: 100%;
		height: auto;
		border-radius: 30px;
		border: 1px solid var(--stroke);
		box-shadow:
			0 1px 2px rgb(0 0 0 / 30%),
			0 18px 48px rgb(0 0 0 / 45%);
		background: var(--surface);
	}
	.phone-1 {
		margin-top: 56px;
	}
	@media (max-width: 860px) {
		.hero {
			grid-template-columns: 1fr;
			padding-block: 40px 24px;
			gap: 36px;
		}
		.hero-phones {
			max-width: 460px;
			margin-inline: auto;
		}
		.hero-phones::before {
			inset: 10% 0 0;
		}
	}

	/* Facts */
	.facts {
		list-style: none;
		padding: 0;
		margin-block: 24px 0;
		display: flex;
		flex-wrap: wrap;
		justify-content: center;
		gap: 10px;
	}
	.facts li {
		background: var(--surface);
		border: 1px solid var(--stroke);
		border-radius: 999px;
		padding: 6px 16px;
		font-size: 14px;
		font-weight: 500;
		color: var(--dim);
	}

	/* Sections */
	.section {
		padding-block: 96px 0;
	}
	h2 {
		text-wrap: balance;
		font-size: clamp(28px, 3.6vw, 40px);
		font-weight: 700;
		line-height: 1.15;
	}
	.section-lead {
		margin-top: 14px;
		color: var(--dim);
		max-width: 44em;
		font-size: 18px;
		line-height: 1.6;
	}
	h3 {
		font-size: 18px;
		font-weight: 600;
	}
	.card {
		background: var(--surface);
		border: 1px solid var(--stroke);
		border-radius: var(--radius);
		padding: 0;
	}
	p {
		line-height: 1.6;
	}

	/* Tables */
	.table-card {
		margin-top: 32px;
		overflow-x: auto;
	}
	table {
		width: 100%;
		border-collapse: collapse;
		font-size: 16px;
	}
	th,
	td {
		text-align: left;
		padding: 16px 20px;
		border-bottom: 1px solid var(--stroke);
		vertical-align: top;
	}
	thead th {
		font-size: 13px;
		font-weight: 600;
		color: var(--faint);
		background: var(--raised);
	}
	tbody th {
		font-weight: 600;
	}
	td {
		color: var(--dim);
	}
	tbody tr:last-child th,
	tbody tr:last-child td {
		border-bottom: none;
	}
	tr.ours th {
		color: var(--lamp);
	}
	tr.ours td {
		color: var(--safe);
		font-weight: 600;
	}
	.table-note {
		padding: 14px 20px;
		border-top: 1px solid var(--stroke);
		color: var(--dim);
		font-size: 15px;
	}
	@media (max-width: 560px) {
		th,
		td {
			padding: 12px 12px;
			font-size: 14px;
		}
	}

	/* Flow */
	.flow {
		display: grid;
		grid-template-columns: 1fr auto 1fr auto 1fr;
		align-items: center;
		gap: 16px;
		margin-top: 40px;
	}
	.chip {
		display: grid;
		gap: 4px;
		background: var(--surface);
		border: 1px solid var(--stroke);
		border-radius: 14px;
		padding: 16px 18px;
		text-align: center;
	}
	.chip small {
		color: var(--faint);
		font-size: 13px;
	}
	.chip.lamp {
		background: var(--lamp);
		border: none;
		color: var(--on-lamp);
	}
	.chip.lamp small {
		color: color-mix(in srgb, var(--on-lamp) 70%, transparent);
	}
	/* two-way: an arrow each way */
	.arrows {
		display: grid;
		gap: 8px;
	}
	.arrows span {
		display: block;
		width: 32px;
		height: 2px;
		background: var(--stroke);
		position: relative;
	}
	.arrows span::after {
		content: '';
		position: absolute;
		top: -4px;
		border: 5px solid transparent;
	}
	.arrows span:first-child::after {
		right: -1px;
		border-left: 7px solid var(--stroke);
		border-right: 0;
	}
	.arrows span:last-child::after {
		left: -1px;
		border-right: 7px solid var(--stroke);
		border-left: 0;
	}
	@media (max-width: 860px) {
		.flow {
			grid-template-columns: 1fr;
		}
		.arrows {
			grid-auto-flow: column;
			justify-self: center;
		}
		.arrows span {
			width: 2px;
			height: 24px;
		}
		.arrows span:first-child::after {
			top: auto;
			bottom: -1px;
			right: -4px;
			border: 5px solid transparent;
			border-top: 7px solid var(--stroke);
			border-bottom: 0;
		}
		.arrows span:last-child::after {
			top: -1px;
			left: -4px;
			border: 5px solid transparent;
			border-bottom: 7px solid var(--stroke);
			border-top: 0;
		}
	}
	.steps {
		list-style: none;
		padding: 0;
		margin: 40px 0 0;
		display: grid;
		grid-template-columns: repeat(4, 1fr);
		gap: 24px;
	}
	.steps li {
		display: grid;
		align-content: start;
		gap: 10px;
	}
	.steps p {
		color: var(--dim);
	}
	.step-number {
		display: inline-grid;
		place-items: center;
		width: 32px;
		height: 32px;
		border-radius: 50%;
		background: color-mix(in srgb, var(--lamp) 14%, var(--surface));
		color: var(--lamp);
		font-weight: 700;
		font-size: 15px;
	}
	@media (max-width: 960px) {
		.steps {
			grid-template-columns: repeat(2, 1fr);
		}
	}
	@media (max-width: 560px) {
		.steps {
			grid-template-columns: 1fr;
		}
	}

	/* Bunker */
	.bunker {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 56px;
		align-items: center;
	}
	.bunker .panel {
		margin-top: 0;
	}
	@media (max-width: 860px) {
		.bunker {
			grid-template-columns: 1fr;
		}
	}

	/* Features */
	.features {
		list-style: none;
		padding: 0;
		margin: 36px 0 0;
		display: grid;
		grid-template-columns: repeat(3, 1fr);
		gap: 20px;
	}
	.features li {
		padding: 24px;
		display: grid;
		align-content: start;
		gap: 10px;
	}
	.features p {
		color: var(--dim);
	}
	.icon {
		width: 28px;
		height: 28px;
		fill: none;
		stroke: var(--lamp);
		stroke-width: 1.7;
		stroke-linecap: round;
		stroke-linejoin: round;
	}
	@media (max-width: 960px) {
		.features {
			grid-template-columns: repeat(2, 1fr);
		}
	}
	@media (max-width: 600px) {
		.features {
			grid-template-columns: 1fr;
		}
	}
	.points {
		margin: 24px 0 0;
		padding-left: 1.2em;
		color: var(--dim);
		display: grid;
		gap: 8px;
	}

	/* Networks */
	.networks {
		display: grid;
		grid-template-columns: 1fr 1.4fr;
		gap: 20px;
		margin-top: 32px;
	}
	.net {
		display: flex;
		gap: 16px;
		align-items: flex-start;
		padding: 24px;
	}
	.net p {
		color: var(--dim);
		margin-top: 6px;
	}
	@media (max-width: 760px) {
		.networks {
			grid-template-columns: 1fr;
		}
	}

	/* Web app */
	.screen {
		width: 100%;
		height: auto;
		margin-top: 32px;
		border-radius: var(--radius);
		border: 1px solid var(--stroke);
		box-shadow: 0 18px 48px rgb(0 0 0 / 45%);
	}
	.ticks {
		list-style: none;
		padding: 0;
		margin: 28px 0 0;
		display: grid;
		grid-template-columns: repeat(2, 1fr);
		gap: 12px 32px;
		color: var(--dim);
	}
	.ticks li {
		padding-left: 28px;
		position: relative;
	}
	.ticks li::before {
		content: '✓';
		position: absolute;
		left: 0;
		color: var(--safe);
		font-weight: 700;
	}
	@media (max-width: 700px) {
		.ticks {
			grid-template-columns: 1fr;
		}
	}

	/* Platforms */
	.platforms {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 48px;
		align-items: center;
	}
	.caption {
		margin-top: 16px;
		color: var(--faint);
		font-size: 15px;
	}
	.pair {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 20px;
	}
	figure {
		margin: 0;
		display: grid;
		gap: 12px;
		justify-items: center;
	}
	figcaption {
		font-size: 14px;
		color: var(--dim);
		font-weight: 500;
	}
	@media (max-width: 860px) {
		.platforms {
			grid-template-columns: 1fr;
		}
		.pair {
			max-width: 460px;
		}
	}

	/* Open */
	.open {
		list-style: none;
		padding: 0;
		margin: 32px 0 0;
		display: grid;
		gap: 24px;
	}
	.open.two {
		grid-template-columns: repeat(2, 1fr);
		gap: 28px 40px;
		margin-top: 36px;
	}
	.open li {
		display: grid;
		gap: 8px;
		padding-left: 18px;
		border-left: 3px solid var(--lamp);
		align-content: start;
	}
	.open p {
		color: var(--dim);
	}
	.more {
		font-weight: 600;
		font-size: 15px;
	}
	@media (max-width: 760px) {
		.open.two {
			grid-template-columns: 1fr;
		}
	}

	/* Pricing */
	.plans {
		display: grid;
		grid-template-columns: 1fr 1.15fr;
		gap: 20px;
		margin-top: 36px;
		max-width: 920px;
		align-items: stretch;
	}
	.plan {
		padding: 28px;
		display: flex;
		flex-direction: column;
		gap: 16px;
	}
	.plan ul {
		margin: 0;
		padding-left: 1.2em;
		color: var(--dim);
		display: grid;
		align-content: start;
		gap: 8px;
		flex: 1;
	}
	.plan .button,
	.plan .store,
	.plan-stores {
		align-self: flex-start;
	}
	.plan-stores {
		display: flex;
		flex-wrap: wrap;
		gap: 10px;
	}
	.price {
		font-size: 40px;
		font-weight: 700;
		letter-spacing: -0.02em;
		line-height: 1.1;
	}
	.price small {
		font-size: 16px;
		font-weight: 500;
		color: var(--faint);
		letter-spacing: 0;
	}
	.featured {
		border: 1px solid color-mix(in srgb, var(--lamp) 55%, var(--stroke));
		box-shadow: 0 18px 48px rgb(242 181 68 / 10%);
	}
	.featured .price {
		color: var(--lamp);
	}
	.store {
		display: inline-flex;
		align-items: center;
		min-height: 44px;
		padding: 0 18px;
		border-radius: 12px;
		border: 1px dashed var(--stroke);
		color: var(--faint);
		font-weight: 600;
		font-size: 15px;
	}
	@media (max-width: 960px) {
		.plans {
			grid-template-columns: 1fr;
			max-width: 560px;
		}
	}

	/* FAQ */
	.questions {
		margin-top: 32px;
		display: grid;
		gap: 12px;
		max-width: 860px;
	}
	details {
		background: var(--surface);
		border: 1px solid var(--stroke);
		border-radius: 14px;
		padding: 0 20px;
	}
	summary {
		cursor: pointer;
		font-weight: 600;
		padding: 18px 0;
		list-style: none;
		display: flex;
		justify-content: space-between;
		gap: 16px;
	}
	summary::-webkit-details-marker {
		display: none;
	}
	summary::after {
		content: '+';
		color: var(--lamp);
		font-weight: 400;
		font-size: 22px;
		line-height: 1;
	}
	details[open] summary::after {
		content: '−';
	}
	details p {
		color: var(--dim);
		padding-bottom: 18px;
	}

	/* Closing */
	.cta {
		margin-block: 112px 0;
		padding: 56px 24px;
		text-align: center;
		display: grid;
		justify-items: center;
		gap: 16px;
		border-radius: 24px;
		background:
			radial-gradient(circle at 50% 0%, rgb(242 181 68 / 14%), transparent 60%), var(--surface);
		border: 1px solid var(--stroke);
	}
	.cta p {
		color: var(--dim);
		max-width: 40em;
	}
	.stores {
		display: flex;
		flex-wrap: wrap;
		justify-content: center;
		gap: 12px;
		margin-top: 8px;
	}

	/* Footer */
	.footer {
		margin-top: 96px;
		border-top: 1px solid var(--stroke);
		padding-block: 40px;
	}
	.footer-inner {
		display: grid;
		gap: 10px;
		color: var(--dim);
		font-size: 14px;
	}
	.quiet {
		color: var(--faint);
	}
	@media (max-width: 640px) {
		:global(body) {
			--gutter: 16px;
		}
		.section {
			padding-block: 72px 0;
		}
	}
</style>
