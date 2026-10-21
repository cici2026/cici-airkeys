/** @type {import("prettier").Config} */
const config = {
	useTabs: true,
	// a tab counts two columns toward printWidth, whatever an .editorconfig says
	tabWidth: 2,
	singleQuote: true,
	trailingComma: 'none',
	printWidth: 100,
	plugins: ['prettier-plugin-svelte'],
	overrides: [{ files: '*.svelte', options: { parser: 'svelte' } }]
};

export default config;
