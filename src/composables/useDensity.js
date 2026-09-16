import { ref, watch } from 'vue';

// "comfortable" | "compact". Applied as a class on <html> so any stylesheet rule can key off it
// (`html.density-compact .foo {...}`) without every component reading the setting, same shape as useAccessibility's singletons.
function detectDensity() {
  try {
    const stored = localStorage.getItem('strata-density');
    if (stored === 'compact' || stored === 'comfortable') return stored;
  } catch {
    // ignore storage errors
  }
  return 'comfortable';
}

function applyDensity(value) {
  document.documentElement.classList.toggle('density-compact', value === 'compact');
}

const density = ref(detectDensity());
applyDensity(density.value);

watch(density, (value) => {
  applyDensity(value);
  try {
    localStorage.setItem('strata-density', value);
  } catch {
    // ignore storage errors (private mode, etc.)
  }
});

export function useDensity() {
  return { density };
}
