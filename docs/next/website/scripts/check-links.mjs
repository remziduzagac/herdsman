// Fails the build when a page links to another page of the site that does not
// exist, to a heading that page does not have, or to a path on the domain
// outside the herdsman site. External links are not checked.
import { existsSync, readdirSync, readFileSync } from 'node:fs';
import { join } from 'node:path';

const base = '/herdsman/docs/';
const pages = new Map();

function walk(dir) {
  for (const entry of readdirSync(dir, { withFileTypes: true })) {
    const path = join(dir, entry.name);
    if (entry.isDirectory()) walk(path);
    else if (entry.name.endsWith('.html')) pages.set(path, readFileSync(path, 'utf8'));
  }
}

function target(url) {
  const [path, hash] = url.split('#');
  const relative = path.slice(base.length);
  const file = relative === '' || relative.endsWith('/') ? join('dist', relative, 'index.html') : join('dist', relative);
  return { file, hash };
}

walk('dist');
const broken = [];
for (const [page, html] of pages) {
  for (const [, url] of html.matchAll(/href="([^"]+)"/g)) {
    if (url.startsWith('/') && !url.startsWith('//') && !url.startsWith('/herdsman/')) {
      broken.push(`${page}: ${url} (outside the herdsman site)`);
      continue;
    }
    if (!url.startsWith(base)) continue;
    const { file, hash } = target(url);
    if (!existsSync(file)) {
      broken.push(`${page}: ${url} (no page)`);
    } else if (hash && file.endsWith('.html') && !(pages.get(file) ?? '').includes(`id="${hash}"`)) {
      broken.push(`${page}: ${url} (no heading #${hash})`);
    }
  }
}

if (broken.length > 0) {
  console.error(`check-links: ${broken.length} broken links\n${broken.join('\n')}`);
  process.exit(1);
}
console.log(`check-links: every internal link in ${pages.size} pages resolves`);
