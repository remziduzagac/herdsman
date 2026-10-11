// Where the built site is served. The Pages workflow builds the docs twice: from
// main at /herdsman/docs/, and from the newest preview build's commit, with
// HERDSMAN_DOCS_CHANNEL=preview, at /herdsman/docs/preview/.
export const preview = process.env.HERDSMAN_DOCS_CHANNEL === 'preview';
export const base = preview ? '/herdsman/docs/preview' : '/herdsman/docs';
