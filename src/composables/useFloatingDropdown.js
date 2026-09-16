import { ref, nextTick, onMounted, onBeforeUnmount } from 'vue';

/**
 * Shared open/position/outside-click/scroll behavior for every custom
 * dropdown. Expects the dropdown teleported to <body>.
 */
export function useFloatingDropdown({ minWidth } = {}) {
  const open = ref(false);
  const rootEl = ref(null);
  const triggerEl = ref(null);
  const ddEl = ref(null);
  const ddStyle = ref({});

  function updatePosition() {
    if (!triggerEl.value) return;
    const rect = triggerEl.value.getBoundingClientRect();
    const gap = 6;
    const margin = 12; // keep this much breathing room from the window edge
    const minUsable = 160; // below this, flip up instead of squeezing to near-nothing
    const spaceBelow = window.innerHeight - rect.bottom - gap - margin;
    const spaceAbove = rect.top - gap - margin;

    // Clamped to whatever room is actually there (with margin), flipping
    // to open upward when there's more room above than below.
    const openUpward = spaceBelow < minUsable && spaceAbove > spaceBelow;
    const maxHeight = Math.max(80, openUpward ? spaceAbove : spaceBelow);

    // minWidth lets a caller opt into "at least this wide" instead of
    // matching an icon-only collapsed-sidebar trigger's unreadable width.
    const width = minWidth ? Math.max(rect.width, minWidth) : rect.width;

    const style = {
      position: 'fixed',
      left: `${rect.left}px`,
      width: `${width}px`,
      maxHeight: `${maxHeight}px`,
      // Inline so it reliably beats .dropdown's own z-index; equal-
      // specificity class rules otherwise resolve by fragile source order.
      zIndex: 150,
    };
    if (openUpward) {
      // `bottom` alone collapses height for a teleported element (CSS
      // derives it from static position); read scrollHeight and set `top`.
      const naturalHeight = Math.min(ddEl.value?.scrollHeight ?? maxHeight, maxHeight);
      style.top = `${rect.top - gap - naturalHeight}px`;
    } else {
      style.top = `${rect.bottom + gap}px`;
    }
    ddStyle.value = style;
  }

  function toggle(disabled) {
    if (disabled) return;
    open.value = !open.value;
    if (open.value) nextTick(updatePosition);
  }
  function close() {
    open.value = false;
  }

  function onDocClick(event) {
    if (!open.value) return;
    if (rootEl.value?.contains(event.target)) return;
    open.value = false;
  }
  function onScrollOrResize(event) {
    if (!open.value) return;
    // A scroll inside the dropdown's own list must not close it, only
    // scrolling of something else that would invalidate the position.
    if (ddEl.value?.contains(event.target)) return;
    open.value = false;
  }
  onMounted(() => {
    document.addEventListener('click', onDocClick);
    window.addEventListener('resize', onScrollOrResize);
    window.addEventListener('scroll', onScrollOrResize, true);
  });
  onBeforeUnmount(() => {
    document.removeEventListener('click', onDocClick);
    window.removeEventListener('resize', onScrollOrResize);
    window.removeEventListener('scroll', onScrollOrResize, true);
  });

  return { open, rootEl, triggerEl, ddEl, ddStyle, toggle, close, updatePosition };
}
