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
      description: 'A terminal workspace for running coding agents, with stacked panes.',
      logo: { src: './public/assets/logo.svg' },
      favicon: '/assets/logo.svg',
      customCss: ['./src/styles/custom.css'],
      social: [{ icon: 'github', label: 'GitHub', href: 'https://github.com/remziduzagac/herdsman' }],
      editLink: {
        baseUrl: 'https://github.com/remziduzagac/herdsman/edit/dev/docs/next/website/',
      },
      routeMiddleware: preview ? ['./src/preview-banner.ts'] : [],
      sidebar: [
        { label: 'Start', items: ['install', 'quick-start', 'concepts', 'keyboard', 'how-to-work'] },
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
