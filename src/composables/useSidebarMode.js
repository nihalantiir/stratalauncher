import { ref, watch } from 'vue';

// Whether the sidebar stays fully expanded all the time, opting out of the
// default hover-to-expand/auto-collapse behavior (see Sidebar.vue). Same
// singleton-ref/localStorage shape as useAccessibility/useDensity.
function detectPersistent() {
  try {
    return localStorage.getItem('strata-sidebar-persistent') === 'true';
  } catch {
    return false;
  }
}

const persistent = ref(detectPersistent());

watch(persistent, (value) => {
  try {
    localStorage.setItem('strata-sidebar-persistent', String(value));
  } catch {
    // ignore storage errors
  }
});

export function useSidebarMode() {
  return { persistent };
}
