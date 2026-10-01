// The herdsman documentation site, served from GitHub Pages at /herdsman/docs/.
// The pages in src/content/docs link to each other as /docs/<page>/; after the
// build, scripts/rebase-links.mjs adds the /herdsman prefix the Pages site needs.
import { defineConfig } from 'astro/config';
import starlight from '@astrojs/starlight';

export default defineConfig({
  site: 'https://remziduzagac.github.io',
  base: '/herdsman/docs',
  trailingSlash: 'always',
  integrations: [
    starlight({
      title: 'herdsman',
      description: 'A terminal workspace for running coding agents, with stacked panes.',
      logo: { src: './public/assets/logo.svg' },
      favicon: '/assets/logo.svg',
      social: [{ icon: 'github', label: 'GitHub', href: 'https://github.com/remziduzagac/herdsman' }],
      editLink: {
        baseUrl: 'https://github.com/remziduzagac/herdsman/edit/dev/docs/next/website/',
      },
      sidebar: [
        { label: 'Start', items: ['install', 'quick-start', 'concepts', 'keyboard', 'how-to-work'] },
        {
          label: 'Agents',
          items: ['agents', 'stacks', 'agent-automation', 'agent-skill', 'integrations', 'add-herdsman-support'],
        },
        { label: 'Sessions and machines', items: ['session-state', 'persistence-remote', 'connecting-machines'] },
        { label: 'Configure and extend', items: ['configuration', 'config-reference', 'plugins'] },
        { label: 'Reference', items: ['cli-reference', 'socket-api', 'troubleshooting', 'windows-beta'] },
        'about',
      ],
    }),
  ],
});
