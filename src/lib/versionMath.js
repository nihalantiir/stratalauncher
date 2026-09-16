// Shared helpers for comparing Minecraft version strings and resolving a
// snapshot/pre-release/rc to the stable release it actually targets.

// Used by both the panorama picker and the version-selector subcategory
// grouping so the two stay consistent with each other.

export function parseVersionKey(v) {
  const m = /^(\d+)(?:\.(\d+))?(?:\.(\d+))?/.exec(v ?? '');
  if (!m) return null;
  return [Number(m[1]), m[2] ? Number(m[2]) : 0, m[3] ? Number(m[3]) : 0];
}

export function compareVersionKeys(a, b) {
  for (let i = 0; i < 3; i++) {
    if (a[i] !== b[i]) return a[i] - b[i];
  }
  return 0;
}

// Caches the id->entry map and release-only list per manifest array
// reference, so repeated calls (once per version, per grouping/panorama
// lookup) don't each re-scan the ~900-entry manifest from scratch.
const indexCache = new WeakMap();

function versionIndex(manifestVersions) {
  let index = indexCache.get(manifestVersions);
  if (!index) {
    const byId = new Map();
    const releases = [];
    for (const v of manifestVersions) {
      byId.set(v.id, v);
      if (v.type === 'release') releases.push(v);
    }
    index = { byId, releases };
    indexCache.set(manifestVersions, index);
  }
  return index;
}

/**
 * Finds the release a snapshot/pre-release/rc targets: the next release by
 * releaseTime (robust regardless of manifest order). Returns the id unchanged for releases, old_beta/old_alpha, or when no manifest is available.
 */
export function resolveTargetVersion(mcVersion, manifestVersions) {
  if (!manifestVersions?.length) return mcVersion;
  const { byId, releases } = versionIndex(manifestVersions);
  const entry = byId.get(mcVersion);
  if (!entry || entry.type === 'release') return mcVersion;
  if (entry.type === 'old_beta' || entry.type === 'old_alpha') return mcVersion;

  let next = null;
  for (const r of releases) {
    if (r.releaseTime >= entry.releaseTime && (!next || r.releaseTime < next.releaseTime)) next = r;
  }
  return next ? next.id : mcVersion;
}

/** Scans an ascending `[{ prefix|version, ... }]` table for the latest entry at or before `targetVersion`. */
export function latestBoundaryAtOrBefore(table, targetVersion, keyField = 'version') {
  const key = parseVersionKey(targetVersion);
  if (!key) return null;
  let best = null;
  for (const entry of table) {
    if (compareVersionKeys(parseVersionKey(entry[keyField]), key) <= 0) best = entry;
    else break; // table is ascending, so once we've passed the target we're done
  }
  return best;
}
