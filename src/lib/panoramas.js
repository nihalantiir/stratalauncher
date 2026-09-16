// Maps a Minecraft version to the closest official panorama Strata has on hand, using the
// latest entry at or before it (list is ordered ascending); falls back to a generic default for older versions.
import { resolveTargetVersion, latestBoundaryAtOrBefore } from './versionMath';

const VERSION_PANORAMAS = [
  { version: '1.12', file: '1.12.webp' }, // World of Color
  { version: '1.13', file: '1.13.webp' }, // Update Aquatic
  { version: '1.14', file: '1.14.webp' }, // Village & Pillage
  { version: '1.15', file: '1.15.webp' }, // Buzzy Bees
  { version: '1.16', file: '1.16.webp' }, // Nether Update
  { version: '1.17', file: '1.17.webp' }, // Caves & Cliffs I
  { version: '1.18', file: '1.18.webp' }, // Caves & Cliffs II
  { version: '1.19', file: '1.19.webp' }, // Wild Update
  { version: '1.20', file: '1.20.webp' }, // Trails & Tales
  { version: '1.21', file: '1.21.webp' }, // Tricky Trials (covers 1.21 - 1.21.3)
  { version: '1.21.4', file: '1.21.4.webp' }, // The Garden Awakens
  { version: '1.21.5', file: '1.21.5.webp' }, // Spring to Life
  { version: '1.21.6', file: '1.21.6.webp' }, // Chase the Skies
  { version: '1.21.9', file: '1.21.9.webp' }, // The Copper Age (covers 1.21.9 - 1.21.10)
  { version: '1.21.11', file: '1.21.11.webp' }, // Mounts of Mayhem
  { version: '26.1', file: '26.1.webp' }, // Tiny Takeover
  { version: '26.2', file: '26.2.webp' }, // Chaos Cubed
  { version: '26.3', file: '26.3.webp' }, // Wilderness Bound
];
export const DEFAULT_PANORAMAS = ['default-1.webp', 'default-2.webp', 'default-3.webp'];

function hashString(s) {
  let h = 0;
  for (let i = 0; i < s.length; i++) h = (h * 31 + s.charCodeAt(i)) | 0;
  return Math.abs(h);
}

function defaultPanoramaFor(seed) {
  return DEFAULT_PANORAMAS[hashString(String(seed)) % DEFAULT_PANORAMAS.length];
}

/**
 * Resolves the panorama URL for an instance. `manifestVersions` (the real
 * Mojang manifest array, with type/releaseTime) lets a snapshot resolve to
 * the panorama of whichever release it's actually heading toward, exactly
 * like the game's own launcher would show as of that snapshot.
 */
export function panoramaForInstance(instance, manifestVersions) {
  const seed = instance?.id ?? instance?.mcVersion ?? 'default';
  if (!instance?.mcVersion) return `/panoramas/${defaultPanoramaFor(seed)}`;
  const target = resolveTargetVersion(instance.mcVersion, manifestVersions);
  const match = latestBoundaryAtOrBefore(VERSION_PANORAMAS, target);
  return `/panoramas/${match ? match.file : defaultPanoramaFor(seed)}`;
}
