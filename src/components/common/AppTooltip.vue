<script setup>
import { ref, watch, nextTick } from 'vue';
import { useTooltip } from '../../composables/useTooltip';

const { tooltipState } = useTooltip();
const tipEl = ref(null);
const style = ref({});

// Same measure-then-clamp approach as ContextMenu: a tooltip follows the cursor, so its raw x/y
// is a point, not a trigger box, and it just needs to never run off the window edge.
function computePosition() {
  if (!tipEl.value) return;
  const margin = 10;
  const gap = 14; // clear of the cursor itself, not just its point
  const rect = tipEl.value.getBoundingClientRect();
  let x = tooltipState.value.x - rect.width / 2;
  let y = tooltipState.value.y + gap;
  x = Math.min(Math.max(margin, x), window.innerWidth - rect.width - margin);
  if (y + rect.height + margin > window.innerHeight) y = tooltipState.value.y - gap - rect.height;
  style.value = { position: 'fixed', left: `${x}px`, top: `${y}px` };
}

watch(
  () => tooltipState.value.visible,
  (visible) => {
    if (!visible) return;
    style.value = { position: 'fixed', left: `${tooltipState.value.x}px`, top: `${tooltipState.value.y}px`, visibility: 'hidden' };
    nextTick(computePosition);
  },
);
watch(
  () => [tooltipState.value.x, tooltipState.value.y],
  () => {
    if (tooltipState.value.visible) computePosition();
  },
);
</script>

<template>
  <Teleport to="body">
    <Transition name="app-tooltip-fade">
      <div v-if="tooltipState.visible" ref="tipEl" class="app-tooltip" :style="style">{{ tooltipState.text }}</div>
    </Transition>
  </Teleport>
</template>
