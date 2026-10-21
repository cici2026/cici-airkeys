// Amounts are integers in base units (satoshi, wei) — bigint here, decimal
// strings on the wire (PROTOCOL §2.3).

/** "1.5" with 18 decimals → 1500000000000000000n; null when it is not a plain amount. */
export function parseUnits(text: string, decimals: number): bigint | null {
	// a comma only as a thousands separator ("1,234.5"): "0,5" is refused, never read as 5
	const t = text.trim();
	if (t.includes(',') && !/^\d{1,3}(,\d{3})+(\.\d*)?$/.test(t)) return null;
	const s = t.replace(/,/g, '');
	const m = /^(\d*)(?:\.(\d*))?$/.exec(s);
	if (!m || (m[1] === '' && (m[2] ?? '') === '')) return null;
	const frac = m[2] ?? '';
	if (frac.length > decimals) return null;
	return BigInt((m[1] || '0') + frac.padEnd(decimals, '0'));
}

/** 1234500n with 6 decimals → "1.2345": exact, no trailing zeros. */
export function formatUnits(value: bigint, decimals: number, group = true): string {
	const neg = value < 0n;
	const digits = (neg ? -value : value).toString().padStart(decimals + 1, '0');
	let int = digits.slice(0, digits.length - decimals);
	const frac = digits.slice(digits.length - decimals).replace(/0+$/, '');
	if (group) int = int.replace(/\B(?=(\d{3})+(?!\d))/g, ',');
	return (neg ? '-' : '') + int + (frac ? '.' + frac : '');
}

/** A short form for lists: four decimals above 1, `max` significant digits below; rounded down. */
export function formatShort(value: bigint, decimals: number, max = 6): string {
	if (value === 0n) return '0';
	const full = formatUnits(value, decimals);
	const [int, frac = ''] = full.split('.');
	if (int !== '0')
		return frac ? `${int}.${frac.slice(0, Math.min(max, 4))}`.replace(/\.?0+$/, '') : int;
	const lead = frac.length - frac.replace(/^0+/, '').length;
	return `0.${frac.slice(0, lead + max).replace(/0+$/, '')}`;
}
