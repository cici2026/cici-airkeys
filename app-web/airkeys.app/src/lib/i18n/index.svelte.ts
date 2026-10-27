// The web app's two languages, English and Simplified Chinese. `t` is the current language's
// dictionary, read afresh on every access: text in a component follows `lang.current` at once,
// and an error is worded in the language of the moment it is made. Numbers are not localised.
import { en, type Dict } from './en';
import { zh } from './zh';

export type Lang = 'en' | 'zh';
export const LANGS: Lang[] = ['en', 'zh'];
const DICTS: Record<Lang, Dict> = { en, zh };

/** The language a browser asks for: the first of its preferred languages that is Chinese or
 * English decides ("en-GB, zh-CN" is English, "zh-TW, en" is Chinese); neither is English. */
export function preferred(languages: readonly string[]): Lang {
	for (const l of languages) {
		const tag = l.toLowerCase();
		if (tag === 'zh' || tag.startsWith('zh-')) return 'zh';
		if (tag === 'en' || tag.startsWith('en-')) return 'en';
	}
	return 'en';
}

/** The browser's language on a page; English elsewhere (tests, the build). The stored choice,
 * if any, is applied by the settings when they load (state.svelte.ts). */
function initial(): Lang {
	if (typeof document === 'undefined' || typeof navigator === 'undefined') return 'en';
	return preferred(navigator.languages?.length ? navigator.languages : [navigator.language ?? '']);
}

export const lang = $state<{ current: Lang }>({ current: initial() });

export const isLang = (v: unknown): v is Lang => LANGS.includes(v as Lang);

/** The `lang` attribute of a page in this language. */
export const htmlLang = (l: Lang) => (l === 'zh' ? 'zh-Hans' : 'en');

/** The current language's text: `t.home.total`, `t.home.readingLeft(3)`. */
export const t: Dict = new Proxy({} as Dict, {
	get: (_, key) => DICTS[lang.current][key as keyof Dict]
});

export { en, zh, type Dict };
