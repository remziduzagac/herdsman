// Marks every page of the preview build's docs (see scripts/site.mjs): a banner
// pointing readers at the stable docs, and noindex so search engines list only
// the stable pages, which say the same for the current release.
import { defineRouteMiddleware } from '@astrojs/starlight/route-data';

export const onRequest = defineRouteMiddleware((context) => {
  context.locals.starlightRoute.entry.data.banner = {
    content:
      'These pages describe the newest <a href="/herdsman/docs/preview/install/#preview-builds">preview build</a>. ' +
      'For the current release, read the <a href="/herdsman/docs/">stable documentation</a>.',
  };
  context.locals.starlightRoute.head.push({ tag: 'meta', attrs: { name: 'robots', content: 'noindex' } });
});
