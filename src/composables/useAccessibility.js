import { ref, watch } from 'vue';

// CSS here is almost entirely fixed px, not rem, so a root font-size trick
// wouldn't work; `zoom` (Chromium, and WebView2 is Chromium) scales the whole rendered page uniformly instead.
function applyUiScale(value) {
  document.documentElement.style.zoom = String(value / 100);
}

// "system" | "on" | "off". No broad @media(prefers-reduced-motion) sweep
// exists to hook into; the one real consumer is the background video's autoplay (see VideoBackground.vue).
function detectReducedMotion() {
  try {
    const stored = localStorage.getItem('strata-reduced-motion');
    if (stored === 'on' || stored === 'off' || stored === 'system') return stored;
  } catch {
    // ignore storage errors
  }
  return 'system';
}

function detectUiScale() {
  try {
    const stored = Number(localStorage.getItem('strata-ui-scale'));
    if (stored >= 80 && stored <= 150) return stored;
  } catch {
    // ignore storage errors
  }
  return 100;
}

const reducedMotion = ref(detectReducedMotion());
const uiScale = ref(detectUiScale());
applyUiScale(uiScale.value);

watch(uiScale, (value) => {
  applyUiScale(value);
  try {
    localStorage.setItem('strata-ui-scale', String(value));
  } catch {
    // ignore storage errors (private mode, etc.)
  }
});
watch(reducedMotion, (value) => {
  try {
    localStorage.setItem('strata-reduced-motion', value);
  } catch {
    // ignore storage errors
  }
});

// Real system preference, re-read live if the OS setting changes while
// Strata is open.
const systemPrefersReducedMotion = ref(window.matchMedia('(prefers-reduced-motion: reduce)').matches);
window.matchMedia('(prefers-reduced-motion: reduce)').addEventListener('change', (e) => {
  systemPrefersReducedMotion.value = e.matches;
});

export function useAccessibility() {
  return { reducedMotion, uiScale, systemPrefersReducedMotion };
}

/** "system" resolves against the real OS preference; "on"/"off" are explicit. */
export function resolveReducedMotion() {
  if (reducedMotion.value === 'system') return systemPrefersReducedMotion.value;
  return reducedMotion.value === 'on';
}
