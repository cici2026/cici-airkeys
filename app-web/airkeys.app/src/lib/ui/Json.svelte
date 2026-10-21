<script lang="ts">
	// A message's JSON, indented and coloured as the phone shows it: names, strings, numbers,
	// true/false/null.
	let { value }: { value: unknown } = $props();
	const TOKEN =
		/("(?:\\.|[^"\\])*")(?=\s*:)|("(?:\\.|[^"\\])*")|(-?\d+(?:\.\d+)?(?:[eE][+-]?\d+)?)|\b(true|false|null)\b/g;
	const pieces = $derived.by(() => {
		const text = JSON.stringify(value, null, 2);
		const out: { text: string; kind: string }[] = [];
		let at = 0;
		for (const m of text.matchAll(TOKEN)) {
			if (m.index > at) out.push({ text: text.slice(at, m.index), kind: 'punctuation' });
			out.push({ text: m[0], kind: m[1] ? 'key' : m[2] ? 'text' : m[3] ? 'number' : 'literal' });
			at = m.index + m[0].length;
		}
		out.push({ text: text.slice(at), kind: 'punctuation' });
		return out;
	});
</script>

<pre class="json">{#each pieces as p, i (i)}<span class={p.kind}>{p.text}</span>{/each}</pre>

<style>
	.json {
		margin: 0;
		padding: 14px 16px;
		border-radius: 12px;
		background: var(--surface);
		font:
			12.5px/1.55 ui-monospace,
			'SF Mono',
			Menlo,
			monospace;
		white-space: pre-wrap;
		word-break: break-all;
	}
	.key {
		color: #8fb3ff;
	}
	.text {
		color: #7fd6a4;
	}
	.number {
		color: #f2b544;
	}
	.literal {
		color: #c9a0ff;
	}
	.punctuation {
		color: var(--faint, #7f879a);
	}
</style>
