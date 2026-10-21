<script lang="ts">
	let { size = 40 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="27 27 54 54" aria-hidden="true">
	<circle cx="54" cy="54" r="24.75" fill="none" stroke="var(--lamp)" stroke-width="4.5" />
	<path
		fill="var(--lamp)"
		d="M54 40.5a8.5 8.5 0 0 1 4.9 15.45l3.1 13.55a1.6 1.6 0 0 1-1.56 1.95h-12.88a1.6 1.6 0 0 1-1.56-1.95l3.1-13.55a8.5 8.5 0 0 1 4.9-15.45z"
	/>
</svg>
