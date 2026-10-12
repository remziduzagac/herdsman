// The pages link to each other as /docs/<page>/, the path the base project's
// own site used. The herdsman site lives at /herdsman/docs/ (the preview's at
// /herdsman/docs/preview/), so after the build this rewrites every such link in
// the generated HTML to start with the site's base.
import { readdirSync, readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { base } from './site.mjs';

let files = 0;
let links = 0;

function walk(dir) {
  for (const entry of readdirSync(dir, { withFileTypes: true })) {
    const path = join(dir, entry.name);
    if (entry.isDirectory()) walk(path);
    else if (entry.name.endsWith('.html')) rebase(path);
  }
}

function rebase(path) {
  const html = readFileSync(path, 'utf8');
  const rebased = html.replace(/(href|src)="\/docs\//g, (_, attribute) => {
    links += 1;
    return `${attribute}="${base}/`;
  });
  if (rebased !== html) {
    writeFileSync(path, rebased);
    files += 1;
  }
}

walk('dist');
console.log(`rebase-links: rebased ${links} links in ${files} pages onto ${base}/`);
