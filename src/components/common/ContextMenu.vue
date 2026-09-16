<script setup>
import { ref, watch, nextTick, onMounted, onBeforeUnmount } from 'vue';
import { useContextMenu } from '../../composables/useContextMenu';
import { MENU_ICONS } from './menuIcons';

const { contextMenuState, closeContextMenu } = useContextMenu();
const menuEl = ref(null);
const style = ref({});

// Renders once (hidden) at the raw click point, measures the real box, then corrects so it never
// runs off the window edge; simpler than useFloatingDropdown since a menu anchors to a point, not a trigger element.
function computePosition() {
  if (!menuEl.value) return;
  const margin = 8;
  const rect = menuEl.value.getBoundingClientRect();
  let x = contextMenuState.value.x;
  let y = contextMenuState.value.y;
  if (x + rect.width + margin > window.innerWidth) x = Math.max(margin, window.innerWidth - rect.width - margin);
  if (y + rect.height + margin > window.innerHeight) y = Math.max(margin, window.innerHeight - rect.height - margin);
  style.value = { position: 'fixed', left: `${x}px`, top: `${y}px`, zIndex: 150 };
}

watch(
  () => contextMenuState.value.open,
  (open) => {
    if (!open) return;
    style.value = {
      position: 'fixed',
      left: `${contextMenuState.value.x}px`,
      top: `${contextMenuState.value.y}px`,
      zIndex: 150,
      visibility: 'hidden',
    };
    nextTick(computePosition);
  },
);

function runAction(item) {
  if (item.disabled) return;
  closeContextMenu();
  item.action?.();
}

function onDocMousedown(event) {
  if (!contextMenuState.value.open) return;
  if (menuEl.value?.contains(event.target)) return;
  closeContextMenu();
}
function onKeydown(event) {
  if (event.key === 'Escape') closeContextMenu();
}
function onScrollOrResize() {
  closeContextMenu();
}

onMounted(() => {
  document.addEventListener('mousedown', onDocMousedown, true);
  document.addEventListener('keydown', onKeydown);
  window.addEventListener('resize', onScrollOrResize);
  window.addEventListener('scroll', onScrollOrResize, true);
});
onBeforeUnmount(() => {
  document.removeEventListener('mousedown', onDocMousedown, true);
  document.removeEventListener('keydown', onKeydown);
  window.removeEventListener('resize', onScrollOrResize);
  window.removeEventListener('scroll', onScrollOrResize, true);
});
</script>

<template>
  <Teleport to="body">
    <div v-if="contextMenuState.open" ref="menuEl" class="dropdown context-menu" :style="style" @contextmenu.prevent>
      <template v-for="(item, i) in contextMenuState.items" :key="i">
        <div v-if="item === 'separator'" class="context-menu-separator"></div>
        <button
          v-else
          type="button"
          class="context-menu-item"
          :class="{ danger: item.danger, disabled: item.disabled }"
          :disabled="item.disabled"
          @click="runAction(item)"
        >
          <svg
            v-if="item.icon"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
            stroke-linecap="round"
            stroke-linejoin="round"
            v-html="MENU_ICONS[item.icon]"
          ></svg>
          <span v-else class="context-menu-icon-spacer"></span>
          {{ item.label }}
        </button>
      </template>
    </div>
  </Teleport>
</template>
