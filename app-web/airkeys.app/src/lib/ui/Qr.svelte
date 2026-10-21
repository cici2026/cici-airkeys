<script lang="ts">
	// A standard QR code of an address, for wallets and exchanges that pay to it.
	import { encode } from 'uqr';
	let { value, size = 200 }: { value: string; size?: number } = $props();
	const qr = $derived(encode(value, { ecc: 'M', border: 2 }));
	const path = $derived(
		qr.data.flatMap((row, y) => row.map((on, x) => (on ? `M${x},${y}h1v1h-1z` : ''))).join('')
	);
</script>

<svg
	viewBox="0 0 {qr.size} {qr.size}"
	width={size}
	height={size}
	shape-rendering="crispEdges"
	role="img"
	aria-label="QR code of the address"
>
	<rect width={qr.size} height={qr.size} fill="#f2f4f8" rx="1" />
	<path d={path} fill="#0b0d11" />
</svg>
