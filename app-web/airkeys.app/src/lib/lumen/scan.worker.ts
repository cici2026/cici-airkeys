// Reads Lumen Code from camera frames off the main thread: scan, collect packets, rebuild the
// message. The page posts frames; the worker answers with progress and, once, the message.
import { Scanner } from 'lumen-code/src/core/scan.js';
import { Receiver } from 'lumen-code/src/core/transfer.js';

const scanner = new Scanner();
let receiver = new Receiver({ maxSize: 1 << 20 });
let last = '';

self.onmessage = async (
	e: MessageEvent<{ reset?: boolean; width: number; height: number; data: ArrayBuffer }>
) => {
	if (e.data.reset) {
		receiver = new Receiver({ maxSize: 1 << 20 });
		last = '';
		return;
	}
	const { width, height, data } = e.data;
	const res = scanner.scan({ width, height, data: new Uint8ClampedArray(data) });
	if (res.ok) for (const cw of res.codewords) if (cw.ok) await receiver.push(cw.bytes);
	const out = await receiver.tryFinish();
	if (out && out.bytes) {
		self.postMessage({ done: true, bytes: out.bytes });
		return;
	}
	const p = receiver.progress();
	const now = p ? `${p.received}/${p.needed ?? 0}` : res.ok ? 'seen' : '';
	self.postMessage({
		seen: res.ok,
		progress: p ? { received: p.received, needed: p.needed ?? 0 } : null,
		changed: now !== last
	});
	last = now;
};
