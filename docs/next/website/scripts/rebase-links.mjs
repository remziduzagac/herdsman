// The pages link to each other as /docs/<page>/, the path the base project's
// own site used. The herdsman site lives at /herdsman/docs/, so after the build this
// prefixes every such link in the generated HTML.
import { readdirSync, readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

const prefix = '/herdsman';
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
    return `${attribute}="${prefix}/docs/`;
  });
  if (rebased !== html) {
    writeFileSync(path, rebased);
    files += 1;
  }
}

walk('dist');
console.log(`rebase-links: prefixed ${links} links in ${files} pages`);
