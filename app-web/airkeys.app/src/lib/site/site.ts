/** Site-wide settings of airkeys.app. */
export const site = {
	url: 'https://airkeys.app',
	repository: 'https://github.com/mondaylabsltd/airkeys',
	/** One price worldwide, paid once, on Google Play and the App Store; each store converts it. */
	price: 'US$29.99',
	/** The store listings, once each store has approved Airkeys; until then the site says
	 *  "Coming soon" (store/README.md, Releasing a version). */
	stores: { googlePlay: '', appStore: '' },
	/** Who makes Airkeys: named in the terms, the privacy policy and the footers. */
	company: 'MONDAY LABS LTD',
	/** The company address: security reports (SECURITY.md) and questions about the policies. */
	email: 'hello@mondaylabs.ltd',
	/** Visit counting (Rybbit) on the site's pages only, never in the web companion. */
	analytics: { src: 'https://tj.appsdata.org/api/script.js', siteId: '609294662431' }
};

/** The stores Airkeys can be bought from, by name, with their listings. */
export const listings = [
	{ name: 'Google Play', url: site.stores.googlePlay },
	{ name: 'App Store', url: site.stores.appStore }
];

/** The stores that sell Airkeys so far, by name; none before the first release. */
export const onSale = listings.filter((store) => store.url).map((store) => store.name);
export const launched = onSale.length > 0;

/** A file in the repository, with an optional #anchor. */
export const repoFile = (path: string) => `${site.repository}/blob/main/${path}`;

/** The browser extension, attached to every GitHub release (.github/workflows/release.yml). */
export const extension = {
	download: `${site.repository}/releases/latest/download/airkeys-extension.zip`,
	releases: `${site.repository}/releases`
};

/** Where vulnerabilities are reported, privately (SECURITY.md). */
export const securityReport = `${site.repository}/security/advisories/new`;
