// Types for the parts of lumen-code (plain ES modules) that the web app uses.

declare module 'lumen-code/src/core/transfer.js' {
	/** A message ready to send: its payload (compressed when that helps) and its description. */
	export interface PreparedPayload {
		payload: Uint8Array;
		meta: { name: string; fileSize: number; crc: number; flags: number; payloadLength: number };
	}
	export function preparePayload(bytes: Uint8Array, name: string): Promise<PreparedPayload>;

	export class Sender {
		constructor(
			prepared: PreparedPayload,
			options: { mode?: number; ecc?: number; X: number; Y: number; tid?: number }
		);
		/** The packets of the next frame, without drawing it. */
		nextPackets(): Uint8Array[];
		/** The next frame's cells, and the grid they fill. */
		nextFrame(): { cells: Uint8Array; layout: unknown; seq: number };
	}

	export class Receiver {
		constructor(limits?: { maxSize?: number; maxKept?: number; maxTransfers?: number });
		push(bytes: Uint8Array): Promise<string>;
		progress(): { received: number; needed: number | null } | null;
		/** The message once every block is decoded; `null` until then. */
		tryFinish(): Promise<{ name: string; bytes?: Uint8Array; error?: string } | null>;
	}
}

declare module 'lumen-code/src/core/scan.js' {
	export class Scanner {
		scan(image: {
			width: number;
			height: number;
			data: Uint8ClampedArray;
		}):
			{ ok: true; codewords: { ok: boolean; bytes: Uint8Array }[] } | { ok: false; reason: string };
	}
}

declare module 'lumen-code/src/core/render.js' {
	export class Renderer {
		constructor(layout: unknown, mode: number, scale: number, quiet?: number);
		/** In device pixels, quiet zone included. */
		width: number;
		height: number;
		/** Draws a frame's cells into RGBA pixels. */
		draw(cells: Uint8Array, out: Uint32Array): void;
	}
}
