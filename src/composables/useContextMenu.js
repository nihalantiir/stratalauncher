import { ref } from 'vue';

// Module-level (not per-component): only one context menu can be open at a time app-wide.
// <ContextMenu>, mounted once in App.vue, is the only thing that reads this state.
const state = ref({ open: false, x: 0, y: 0, items: [] });

/**
 * `items` is `{ label, action, danger, disabled }` entries or the string 'separator';
 * `action` runs on click and the menu closes itself first.
 */
function openContextMenu(event, items) {
  event.preventDefault();
  event.stopPropagation();
  state.value = { open: true, x: event.clientX, y: event.clientY, items };
}

function closeContextMenu() {
  state.value = { ...state.value, open: false };
}

export function useContextMenu() {
  return { contextMenuState: state, openContextMenu, closeContextMenu };
}
