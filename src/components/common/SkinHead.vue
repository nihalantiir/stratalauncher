<script setup>
import { ref, watch, onMounted } from 'vue';
import { getDefaultSkin } from '../../api/skins';
import { renderSkinHeadIcon } from '../../lib/skinHeadRender';

const props = defineProps({
  // Real account skin URL (Mojang texture); null for offline accounts, which
  // fall back to the real extracted Steve texture (see SkinCapeView's use of the same command).
  skinUrl: { type: String, default: null },
  size: { type: Number, default: 28 },
  // Vary the angle a little so a wall of avatars doesn't look identical.
  frontFace: { type: String, default: 'right' },
});

const iconUrl = ref(null);

let sharedDefaultSkinPromise = null;
function getSharedDefaultSkinUrl() {
  if (!sharedDefaultSkinPromise) {
    sharedDefaultSkinPromise = getDefaultSkin()
      .then((bytes) => URL.createObjectURL(new Blob([new Uint8Array(bytes)], { type: 'image/png' })))
      .catch(() => null);
  }
  return sharedDefaultSkinPromise;
}

async function load() {
  const src = props.skinUrl || (await getSharedDefaultSkinUrl());
  if (!src) return;
  const dpr = window.devicePixelRatio || 1;
  try {
    iconUrl.value = await renderSkinHeadIcon(src, Math.round(props.size * dpr), props.frontFace);
  } catch {
    // Leave it blank on a real render failure, never a wrong texture.
  }
}

onMounted(load);
watch(() => props.skinUrl, load);
</script>

<template>
  <img v-if="iconUrl" :src="iconUrl" class="skin-head" :style="{ width: size + 'px', height: size + 'px' }" alt="" />
  <span v-else class="skin-head" :style="{ width: size + 'px', height: size + 'px' }"></span>
</template>

<style scoped>
.skin-head {
  image-rendering: pixelated;
  display: block;
}
</style>
