import type { Handle } from '@sveltejs/kit';

/** The page's language on <html>: Chinese under /zh/, English elsewhere (the pages are prerendered). */
export const handle: Handle = ({ event, resolve }) => {
	if (!event.url.pathname.startsWith('/zh/')) return resolve(event);
	return resolve(event, {
		transformPageChunk: ({ html }) => html.replace('<html lang="en">', '<html lang="zh-Hans">')
	});
};
