<script lang="ts">
	import { en } from '#lib/site/legal.ts';
	import LegalPage from '#lib/site/LegalPage.svelte';
</script>

<LegalPage doc={en.terms} legal={en} lang="en" path="terms" />
