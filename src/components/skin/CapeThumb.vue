<script setup>
// Crops the back-panel region (the 10x16 patch at UV (1,1)-(11,17) on the
// 64x32 texture, the convention every cape viewer uses) out of the account's own real cape texture URL.
import { ref, watch, onMounted } from 'vue';

const props = defineProps({ url: { type: String, default: null } });
const canvasRef = ref(null);

function loadImage(src) {
  return new Promise((resolve, reject) => {
    const img = new Image();
    img.onload = () => resolve(img);
    img.onerror = reject;
    img.src = src;
  });
}

async function render() {
  const canvas = canvasRef.value;
  if (!canvas) return;
  const ctx = canvas.getContext('2d');
  ctx.imageSmoothingEnabled = false;
  ctx.clearRect(0, 0, canvas.width, canvas.height);
  if (!props.url) return;
  try {
    const img = await loadImage(props.url);
    ctx.drawImage(img, 1, 1, 10, 16, 0, 0, canvas.width, canvas.height);
  } catch {
    // Leave the canvas blank on a load failure instead of a misleading placeholder.
  }
}

onMounted(render);
watch(() => props.url, render);
</script>

<template>
  <canvas ref="canvasRef" width="44" height="68" class="cape-thumb-canvas"></canvas>
</template>
