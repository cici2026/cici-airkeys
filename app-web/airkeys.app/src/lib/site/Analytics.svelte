<script lang="ts">
	// Counts visits to the site's pages (Rybbit). The web companion and the extension load no
	// analytics: this component is used only by the landing and legal pages.
	import { site } from './site';
</script>

<svelte:head>
	<script src={site.analytics.src} data-site-id={site.analytics.siteId} defer></script>
</svelte:head>
