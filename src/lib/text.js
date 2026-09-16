// `str.slice(0, 1)` would split a surrogate pair (emoji, some rarer scripts) into an invalid half;
// Array.from iterates by code point instead, so this fallback "icon" always gets one whole character.
export function firstGrapheme(str) {
  return Array.from(str || '')[0] ?? '';
}

export function formatSize(bytes) {
  if (bytes > 1024 ** 3) return `${(bytes / 1024 ** 3).toFixed(1)} GB`;
  if (bytes > 1024 ** 2) return `${(bytes / 1024 ** 2).toFixed(1)} MB`;
  return `${Math.max(1, Math.round(bytes / 1024))} KB`;
}
