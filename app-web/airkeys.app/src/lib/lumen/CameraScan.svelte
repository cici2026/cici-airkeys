<script lang="ts">
	// The camera, reading the code on the phone's screen. Frames go to a worker; the page stays
	// smooth. `onmessage` gets the complete message once.
	import { onDestroy, onMount } from 'svelte';

	let {
		onmessage,
		hint = 'Hold the phone in front of the camera, its code facing it.'
	}: { onmessage: (bytes: Uint8Array) => void; hint?: string } = $props();
	let video: HTMLVideoElement;
	let error = $state('');
	let seen = $state(false);
	let progress = $state<{ received: number; needed: number } | null>(null);
	let stream: MediaStream | undefined;
	let worker: Worker | undefined;
	let busy = false;
	let stopped = false;
	let canvas: OffscreenCanvas | HTMLCanvasElement;

	onMount(async () => {
		try {
			stream = await navigator.mediaDevices.getUserMedia({
				video: { facingMode: 'environment', width: { ideal: 1920 }, height: { ideal: 1080 } },
				audio: false
			});
		} catch (e) {
			error =
				(e as Error).name === 'NotAllowedError'
					? 'The browser was not allowed to use the camera. Allow it for this page, then try again.'
					: 'No camera is available.';
			return;
		}
		video.srcObject = stream;
		await video.play();
		worker = new Worker(new URL('./scan.worker.ts', import.meta.url), { type: 'module' });
		worker.onmessage = (e) => {
			busy = false;
			if (e.data.done) {
				stop();
				onmessage(new Uint8Array(e.data.bytes));
				return;
			}
			seen = e.data.seen;
			if (e.data.progress) progress = e.data.progress;
		};
		tick();
	});

	function tick() {
		if (stopped) return;
		requestAnimationFrame(tick);
		if (busy || !video.videoWidth) return;
		const w = video.videoWidth;
		const h = video.videoHeight;
		canvas ??=
			typeof OffscreenCanvas !== 'undefined'
				? new OffscreenCanvas(w, h)
				: Object.assign(document.createElement('canvas'), { width: w, height: h });
		const g = canvas.getContext('2d', { willReadFrequently: true }) as CanvasRenderingContext2D;
		g.drawImage(video, 0, 0, w, h);
		const img = g.getImageData(0, 0, w, h);
		busy = true;
		worker!.postMessage({ width: w, height: h, data: img.data.buffer }, [img.data.buffer]);
	}

	function stop() {
		stopped = true;
		stream?.getTracks().forEach((t) => t.stop());
		worker?.terminate();
	}
	onDestroy(stop);
</script>

<div class="scan">
	<!-- svelte-ignore a11y_media_has_caption -->
	<video bind:this={video} playsinline muted></video>
	<div class="frame" class:seen></div>
</div>
{#if error}
	<p class="problem">{error}</p>
{:else if progress && progress.needed > 0}
	<div class="bar">
		<div style="width: {Math.min(100, (100 * progress.received) / progress.needed)}%"></div>
	</div>
	<p class="dim">Receiving…</p>
{:else}
	<p class="dim">{seen ? 'Code found — hold still.' : hint}</p>
{/if}

<style>
	.scan {
		position: relative;
		border-radius: 20px;
		overflow: hidden;
		background: #05060a;
		aspect-ratio: 4 / 3;
	}
	video {
		width: 100%;
		height: 100%;
		object-fit: cover;
		display: block;
	}
	.frame {
		position: absolute;
		inset: 10%;
		border: 3px solid color-mix(in srgb, var(--lamp) 70%, transparent);
		border-radius: 18px;
		transition: border-color 0.2s;
	}
	.frame.seen {
		border-color: var(--safe);
	}
	.bar {
		height: 6px;
		border-radius: 3px;
		background: var(--raised);
		overflow: hidden;
		margin-top: 14px;
	}
	.bar div {
		height: 100%;
		background: var(--lamp);
		transition: width 0.2s;
	}
	.problem {
		color: var(--danger);
	}
</style>
