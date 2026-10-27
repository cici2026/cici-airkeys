<script lang="ts">
	// The camera, reading the code on the phone's screen. Frames go to a worker; the page stays
	// smooth. `onmessage` gets the complete message once.
	import { onDestroy, onMount } from 'svelte';
	import { t } from '$lib/i18n/index.svelte';

	let { onmessage }: { onmessage: (bytes: Uint8Array) => void } = $props();
	let video: HTMLVideoElement;
	let error = $state('');
	let seen = $state(false);
	let progress = $state<{ received: number; needed: number } | null>(null);
	let stream: MediaStream | undefined;
	let worker: Worker | undefined;
	let busy = false;
	let stopped = false;
	let canvas: OffscreenCanvas | HTMLCanvasElement;

	/** Why the camera did not start, in words a person can act on. */
	function why(e: unknown): string {
		switch ((e as Error)?.name) {
			case 'NotAllowedError':
			case 'SecurityError':
				return t.scan.notAllowed;
			case 'NotFoundError':
			case 'OverconstrainedError':
				return t.scan.noCamera;
			case 'NotReadableError':
			case 'AbortError':
				return t.scan.inUse;
			default:
				return t.scan.notStarted(String((e as Error)?.message ?? e));
		}
	}

	onMount(async () => {
		// browsers give the camera to secure pages only: HTTPS, or this computer
		if (!isSecureContext || !navigator.mediaDevices) {
			error = t.scan.insecure;
			return;
		}
		let s: MediaStream;
		try {
			s = await navigator.mediaDevices.getUserMedia({
				video: { facingMode: 'environment', width: { ideal: 1920 }, height: { ideal: 1080 } },
				audio: false
			});
		} catch (e) {
			if (!stopped) error = why(e);
			return;
		}
		// the view was left while the browser asked: let the camera go at once
		if (stopped) {
			s.getTracks().forEach((track) => track.stop());
			return;
		}
		stream = s;
		video.srcObject = stream;
		try {
			await video.play();
		} catch (e) {
			if (!stopped) error = t.scan.noPicture((e as Error).message);
			stop();
			return;
		}
		if (stopped) return;
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
		worker.onerror = (e) => {
			error = t.scan.readerStopped(e.message || t.scan.didNotLoad);
			stop();
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
		stream?.getTracks().forEach((track) => track.stop());
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
	<p class="problem" role="alert">{error}</p>
{:else if progress && progress.needed > 0}
	{@const percent = Math.min(100, Math.round((100 * progress.received) / progress.needed))}
	<div
		class="bar"
		role="progressbar"
		aria-label={t.scan.receivingLabel}
		aria-valuemin={0}
		aria-valuemax={100}
		aria-valuenow={percent}
	>
		<div style="width: {percent}%"></div>
	</div>
	<p class="dim" role="status">{t.scan.receiving}</p>
{:else}
	<p class="dim" role="status">
		{seen ? t.scan.found : t.scan.hold}
	</p>
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
