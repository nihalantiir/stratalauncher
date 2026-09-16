import { ref } from 'vue';

// Module-level, same reasoning as useContextMenu: only one tooltip can
// ever be showing at a time app-wide.
const state = ref({ visible: false, text: '', x: 0, y: 0 });

function showAt(x, y, text) {
  if (!text) return;
  state.value = { visible: true, text, x, y };
}

function moveTo(x, y) {
  if (!state.value.visible) return;
  state.value = { ...state.value, x, y };
}

function hide() {
  if (!state.value.visible) return;
  state.value = { ...state.value, visible: false };
}

export function useTooltip() {
  return { tooltipState: state, showAt, moveTo, hide };
}
