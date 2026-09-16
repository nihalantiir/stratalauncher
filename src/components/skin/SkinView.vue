<script setup>
import { ref, onMounted, onBeforeUnmount, watch } from 'vue';
import { SkinViewer } from 'skinview3d';

const props = defineProps({
  skinUrl: { type: String, default: null },
  capeUrl: { type: String, default: null },
  model: { type: String, default: 'default' }, // 'default' (classic/Steve) | 'slim' (Alex)
  backEquipment: { type: String, default: 'cape' }, // 'cape' | 'elytra', a real skinview3d option, purely visual
});

const canvasEl = ref(null);
let viewer = null;

function applySkin() {
  if (!viewer) return;
  if (props.skinUrl) {
    viewer.loadSkin(props.skinUrl, { model: props.model }).catch(() => {});
  } else {
    viewer.loadSkin(null);
  }
}

function applyCape() {
  if (!viewer) return;
  if (props.capeUrl) {
    viewer.loadCape(props.capeUrl, { backEquipment: props.backEquipment }).catch(() => {});
  } else {
    viewer.loadCape(null);
  }
}

onMounted(() => {
  viewer = new SkinViewer({ canvas: canvasEl.value, width: 300, height: 400, zoom: 0.85 });
  viewer.autoRotate = true;
  viewer.autoRotateSpeed = 0.8;
  viewer.controls.enableZoom = true;
  applySkin();
  applyCape();
});

onBeforeUnmount(() => {
  viewer?.dispose();
  viewer = null;
});

watch(() => [props.skinUrl, props.model], applySkin);
watch(() => props.capeUrl, applyCape);
// A same-cape equipment-type flip doesn't need a full reload; the model
// exposes this as a plain settable property, same texture, no flicker.
watch(
  () => props.backEquipment,
  (value) => {
    if (viewer && props.capeUrl) viewer.playerObject.backEquipment = value;
  },
);
</script>

<template>
  <canvas ref="canvasEl" class="skin-canvas"></canvas>
</template>
