import { describe, expect, it } from 'vitest';
import { en, lang, preferred, t, zh } from './index.svelte';

type Tree = { [key: string]: unknown };

/** Every leaf of a dictionary, by path: its text, or a function's text for sample arguments. */
function leaves(d: Tree, at = ''): Map<string, unknown> {
	const out = new Map<string, unknown>();
	for (const [k, v] of Object.entries(d)) {
		const path = at ? `${at}.${k}` : k;
		if (v && typeof v === 'object') for (const [p, x] of leaves(v as Tree, path)) out.set(p, x);
		else out.set(path, v);
	}
	return out;
}

describe('the dictionaries', () => {
	const a = leaves(en);
	const b = leaves(zh);

	it('have the same keys', () => {
		expect([...b.keys()].sort()).toEqual([...a.keys()].sort());
	});

	it('have no empty text, and functions where the other has them', () => {
		for (const [path, v] of a) {
			const w = b.get(path);
			expect(typeof w, path).toBe(typeof v);
			for (const x of [v, w]) {
				if (typeof x === 'function') {
					expect(x.length, path).toBe((v as () => string).length);
					// every argument is a plain value: a number serves for each
					for (const arg of [1, 2])
						expect(String(x(...Array(x.length).fill(arg))).trim(), path).not.toBe('');
				} else {
					expect(typeof x, path).toBe('string');
					expect((x as string).trim(), path).not.toBe('');
				}
			}
		}
	});

	it('count in English, not in Chinese', () => {
		expect(en.home.accountsCount(1)).toBe('1 account');
		expect(en.home.accountsCount(2)).toBe('2 accounts');
		expect(en.wallet.counts(1, 2)).toBe('1 account · 2 addresses');
		expect(zh.home.accountsCount(2)).toBe('2 个账户');
	});
});

describe('the language', () => {
	it('follows the first Chinese or English preference', () => {
		expect(preferred(['zh-CN', 'en'])).toBe('zh');
		expect(preferred(['zh-Hant-TW'])).toBe('zh');
		expect(preferred(['ja', 'zh'])).toBe('zh');
		expect(preferred(['en-CN', 'zh-Hans-CN'])).toBe('en');
		expect(preferred(['fr', 'de'])).toBe('en');
		expect(preferred([])).toBe('en');
	});

	it('is English outside a page, and switches at once', () => {
		expect(lang.current).toBe('en');
		expect(t.common.refresh).toBe('Refresh');
		lang.current = 'zh';
		expect(t.common.refresh).toBe('刷新');
		lang.current = 'en';
	});
});
