// The herdsman documentation site, served from GitHub Pages at /herdsman/docs/,
// and the newest preview's at /herdsman/docs/preview/ (see scripts/site.mjs).
// The pages in src/content/docs link to each other as /docs/<page>/; after the
// build, scripts/rebase-links.mjs turns /docs/ into the site's base.
import { defineConfig } from 'astro/config';
import starlight from '@astrojs/starlight';
import { base, preview } from './scripts/site.mjs';

export default defineConfig({
  site: 'https://remziduzagac.github.io',
  base,
  trailingSlash: 'always',
  integrations: [
    starlight({
      title: 'herdsman',
      description: 'A terminal workspace for running coding agents such as Claude Code and Codex, with stacked panes. Built on herdr.',
      logo: {
        light: '../../../assets/herdsman-logomark-dark.svg',
        dark: '../../../assets/herdsman-logomark-light.svg',
      },
      favicon: '/assets/herdsman-logomark-small-dark.svg',
      customCss: ['./src/styles/custom.css'],
      // The card shown when a page is shared; the screenshot from the docs home.
      head: [
        {
          tag: 'meta',
          attrs: { property: 'og:image', content: 'https://remziduzagac.github.io/herdsman/docs/assets/social.png' },
        },
        { tag: 'meta', attrs: { property: 'og:image:width', content: '1200' } },
        { tag: 'meta', attrs: { property: 'og:image:height', content: '630' } },
        { tag: 'meta', attrs: { name: 'twitter:card', content: 'summary_large_image' } },
      ],
      social: [{ icon: 'github', label: 'GitHub', href: 'https://github.com/remziduzagac/herdsman' }],
      editLink: {
        baseUrl: 'https://github.com/remziduzagac/herdsman/edit/dev/docs/next/website/',
      },
      routeMiddleware: preview ? ['./src/preview-pages.ts'] : [],
      sidebar: [
        { label: 'Start', items: ['install', 'quick-start', 'concepts', 'what-herdsman-adds', 'keyboard', 'how-to-work'] },
        {
          label: 'Agents',
          items: ['agents', 'stacks', 'presets', 'agent-automation', 'agent-skill', 'integrations', 'add-herdsman-support'],
        },
        { label: 'Sessions and machines', items: ['session-state', 'persistence-remote', 'connecting-machines'] },
        { label: 'Configure and extend', items: ['configuration', 'config-reference', 'plugins'] },
        { label: 'Reference', items: ['cli-reference', 'socket-api', 'troubleshooting', 'windows-beta'] },
        'about',
      ],
    }),
  ],
});
