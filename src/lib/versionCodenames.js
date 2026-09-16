// Real Minecraft Java Edition update codenames, ordered ascending; each
// entry covers every patch up to (but not including) the next boundary version.
import { parseVersionKey, resolveTargetVersion, latestBoundaryAtOrBefore } from './versionMath';

const CODENAMES = [
  { version: '1.0', name: 'Adventure Update' },
  { version: '1.1', name: 'Superflat Update' },
  { version: '1.2', name: 'The Jungle Update' },
  { version: '1.3', name: 'Trading Update' },
  { version: '1.4', name: 'The Pretty Scary Update' },
  { version: '1.5', name: 'Redstone Update' },
  { version: '1.6', name: 'The Horse Update' },
  { version: '1.7', name: 'The Update that Changed the World' },
  { version: '1.8', name: 'Bountiful Update' },
  { version: '1.9', name: 'Combat Update' },
  { version: '1.10', name: 'Frostburn Update' },
  { version: '1.11', name: 'Exploration Update' },
  { version: '1.12', name: 'World of Color' },
  { version: '1.13', name: 'Update Aquatic' },
  { version: '1.14', name: 'Village & Pillage' },
  { version: '1.15', name: 'Buzzy Bees' },
  { version: '1.16', name: 'Nether Update' },
  { version: '1.17', name: 'Caves & Cliffs I' },
  { version: '1.18', name: 'Caves & Cliffs II' },
  { version: '1.19', name: 'Wild Update' },
  { version: '1.20', name: 'Trails & Tales' },
  { version: '1.21', name: 'Tricky Trials' },
  { version: '1.21.4', name: 'The Garden Awakens' },
  { version: '1.21.5', name: 'Spring to Life' },
  { version: '1.21.6', name: 'Chase the Skies' },
  { version: '1.21.9', name: 'The Copper Age' },
  { version: '1.21.11', name: 'Mounts of Mayhem' },
  { version: '26.1', name: 'Tiny Takeover' },
  { version: '26.2', name: 'Chaos Cubed' },
  { version: '26.3', name: 'Wilderness Bound' },
];

/**
 * The version-selector subcategory label for one manifest entry. Snapshots
 * resolve to the codename of whichever release they're heading toward; pre-1.0 old_alpha/old_beta fall back to their type label instead of a made-up name.
 */
export function groupLabelFor(entry, manifestVersions) {
  if (entry.type === 'old_alpha') return { key: 'old_alpha', name: null };
  if (entry.type === 'old_beta') return { key: 'old_beta', name: null };

  const target = resolveTargetVersion(entry.id, manifestVersions);
  const match = latestBoundaryAtOrBefore(CODENAMES, target);
  if (match) return { key: match.version, name: match.name };

  const key = parseVersionKey(target);
  const fallback = key ? `${key[0]}.${key[1]}` : target;
  return { key: fallback, name: null, fallbackVersion: fallback };
}

/**
 * Groups a flat, manifest-ordered version list into
 * `[{ key, name, fallbackVersion, versions: [...] }]`, preserving manifest order; a group's position is wherever its first (newest) member appears.
 */
export function groupVersions(versionList, manifestVersions) {
  const groups = new Map();
  for (const entry of versionList) {
    const label = groupLabelFor(entry, manifestVersions);
    if (!groups.has(label.key)) groups.set(label.key, { ...label, versions: [] });
    groups.get(label.key).versions.push(entry);
  }
  return Array.from(groups.values());
}
