// Real per-provider sort fields, verified against each API's current docs
// (not a made-up unified list). Shared by DownloadContentModal (mods/
// resource packs/shaders) and the modpack browser in NewInstanceModal, so
// every option shown actually does something on whichever provider is
// currently selected.
export const MODRINTH_SORTS = ['relevance', 'downloads', 'follows', 'newest', 'updated'];
export const CURSEFORGE_SORTS = ['featured', 'popularity', 'lastUpdated', 'name', 'author', 'totalDownloads', 'rating'];

export const SORT_LABEL_KEYS = {
  relevance: 'content.sortRelevance',
  downloads: 'content.sortDownloads',
  follows: 'content.sortFollows',
  newest: 'content.sortNewest',
  updated: 'content.sortUpdated',
  featured: 'content.sortFeatured',
  popularity: 'content.sortPopularity',
  lastUpdated: 'content.sortUpdated',
  name: 'content.sortName',
  author: 'content.sortAuthor',
  totalDownloads: 'content.sortDownloads',
  rating: 'content.sortRating',
};

export function sortsForSource(source) {
  return source === 'curseforge' ? CURSEFORGE_SORTS : MODRINTH_SORTS;
}

export function defaultSortForSource(source) {
  return sortsForSource(source)[0];
}
