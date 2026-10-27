import { describe, expect, it } from 'vitest';
import { en, zh, type Content } from './content';
import { site } from './site';

/** The shape of a value: arrays by length, objects by keys, strings as "s". */
function shape(v: unknown): unknown {
	if (Array.isArray(v)) return v.map(shape);
	if (v && typeof v === 'object')
		return Object.fromEntries(Object.entries(v).map(([k, x]) => [k, shape(x)]));
	return typeof v;
}

describe('the landing page', () => {
	it('says the same things in English and Chinese', () => {
		expect(shape(zh)).toEqual(shape(en));
	});

	it('gives one price, everywhere it is named', () => {
		for (const c of [en, zh] as Content[]) {
			expect(c.pricing.store.price).toBe(site.price);
			expect(c.cta.text).toContain(site.price);
		}
		expect(site.price).toBe('US$29.99');
	});

	it('links to repository files that exist', async () => {
		const { existsSync } = await import('node:fs');
		const paths = [en, zh].flatMap((c) => [
			c.pricing.build.guide,
			...c.open.items.flatMap((i) => (i.link ? [i.link.path] : []))
		]);
		for (const p of paths) expect(existsSync(`../../${p.split('#')[0]}`), p).toBe(true);
	});
});

describe('the terms and the privacy policy', () => {
	it('say the same things in English and Chinese', async () => {
		const legal = await import('./legal');
		expect(shape(legal.zh)).toEqual(shape(legal.en));
	});
});
