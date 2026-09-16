<script setup>
import { ref, watch, onMounted, onBeforeUnmount } from 'vue';
import { useI18n } from 'vue-i18n';
import { useRoute } from 'vue-router';
import { listen } from '@tauri-apps/api/event';
import { convertFileSrc } from '@tauri-apps/api/core';
import { useAccessibility, resolveReducedMotion } from '../../composables/useAccessibility';
import { getMediaStatus } from '../../api/media';
import { DEFAULT_PANORAMAS } from '../../lib/panoramas';

const { t } = useI18n();
const route = useRoute();
const { reducedMotion: reducedMotionSetting, systemPrefersReducedMotion } = useAccessibility();

const VIDEOS = [
  'village',
  'aquarium',
  'beach-escape',
  'cherry-grove',
  'dreamy-deserts',
  'dungeons',
  'fireplace',
  'glowing-caves',
  'mining-blocks',
  'rainy-swamp',
  'serene-snow',
  'tricky-trials',
];
const CROSSFADE_MS = 900;

// Downloaded once in the background (see commands::media); a static
// panorama fills in until real videos land.
const mediaReady = ref(false);
const mediaDir = ref('');
const fallbackImage = DEFAULT_PANORAMAS[Math.floor(Math.random() * DEFAULT_PANORAMAS.length)];
const downloadPercent = ref(0);
let unlistenProgress = null;

function srcFor(name) {
  const dir = mediaDir.value.replace(/\\/g, '/').replace(/\/$/, '');
  return convertFileSrc(`${dir}/${name}.mp4`);
}

const videoA = ref(null);
const videoB = ref(null);
const activeIsA = ref(true);
const rerolling = ref(false);
const reducedMotion = ref(resolveReducedMotion());
let currentName = VIDEOS[Math.floor(Math.random() * VIDEOS.length)];

function pickNext(excluding) {
  if (VIDEOS.length <= 1) return VIDEOS[0];
  let next = excluding;
  while (next === excluding) {
    next = VIDEOS[Math.floor(Math.random() * VIDEOS.length)];
  }
  return next;
}

function waitReady(el) {
  return new Promise((resolve) => {
    let done = false;
    const finish = () => {
      if (done) return;
      done = true;
      resolve();
    };
    el.addEventListener('canplay', finish, { once: true });
    // A slow-loading pick should never leave the reroll button stuck.
    setTimeout(finish, 1800);
  });
}

// Live-updates if the user flips the manual override or the OS setting
// changes while Strata is open, not just at mount time.
watch([reducedMotionSetting, systemPrefersReducedMotion], () => {
  const next = resolveReducedMotion();
  if (next === reducedMotion.value) return;
  reducedMotion.value = next;
  const active = activeIsA.value ? videoA.value : videoB.value;
  if (!active) return;
  if (next) active.pause();
  else active.play().catch(() => {});
});

function playActive() {
  const active = videoA.value;
  if (!active) return;
  active.src = srcFor(currentName);
  active.load();
  if (!reducedMotion.value) active.play().catch(() => {});
}

onMounted(async () => {
  // dir is set before the listener below can ever fire, so a completion
  // event arriving mid-setup still has a real path to build from.
  const first = await getMediaStatus().catch(() => ({ ready: false, dir: '' }));
  mediaDir.value = first.dir;

  unlistenProgress = await listen('download://progress', (event) => {
    const p = event.payload;
    if (p.stage !== 'media') return;
    downloadPercent.value = p.total ? Math.round((p.completed / p.total) * 100) : 0;
    if (p.total && p.completed >= p.total) {
      mediaReady.value = true;
      playActive();
    }
  });

  // Covers the download finishing before this component ever mounted, or
  // in the brief window before the listener above was registered.
  const recheck = await getMediaStatus().catch(() => ({ ready: false }));
  if (recheck.ready) {
    mediaReady.value = true;
    playActive();
  }
});

onBeforeUnmount(() => {
  unlistenProgress?.();
});

async function reroll() {
  if (rerolling.value || !mediaReady.value) return;
  const incoming = activeIsA.value ? videoB.value : videoA.value;
  const outgoing = activeIsA.value ? videoA.value : videoB.value;
  if (!incoming) return;

  rerolling.value = true;
  const nextName = pickNext(currentName);
  incoming.src = srcFor(nextName);
  incoming.load();
  await waitReady(incoming);
  if (!reducedMotion.value) {
    try {
      await incoming.play();
    } catch {
      // autoplay quirk, but the crossfade still proceeds and the video catches up
    }
  }

  currentName = nextName;
  activeIsA.value = !activeIsA.value;

  window.setTimeout(() => {
    outgoing?.pause();
    rerolling.value = false;
  }, CROSSFADE_MS);
}
</script>

<template>
  <div class="video-bg" aria-hidden="true">
    <img v-if="!mediaReady" class="video-bg-el front" :src="`/panoramas/${fallbackImage}`" alt="" />
    <video ref="videoA" class="video-bg-el" :class="{ front: mediaReady && activeIsA }" loop muted playsinline disablepictureinpicture></video>
    <video ref="videoB" class="video-bg-el" :class="{ front: mediaReady && !activeIsA }" loop muted playsinline disablepictureinpicture></video>
    <div class="video-bg-overlay"></div>
    <div class="video-bg-vignette"></div>
    <div class="video-bg-grain"></div>
  </div>

  <div v-if="!mediaReady && downloadPercent > 0" class="media-download-note">
    {{ t('background.downloadingMedia', { percent: downloadPercent }) }}
  </div>

  <button
    v-show="route.name === 'library' && mediaReady"
    class="video-reroll"
    type="button"
    :class="{ spinning: rerolling }"
    :disabled="rerolling"
    v-tooltip="t('background.shuffle')"
    :aria-label="t('background.shuffle')"
    @click="reroll"
  >
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
      <path d="M4 6h3.5c1.4 0 2.7.7 3.5 1.9l6 8.2c.8 1.2 2.1 1.9 3.5 1.9H24" />
      <path d="M17.5 4.5L20.5 6l-3 1.5" />
      <path d="M4 18h3.5c1.4 0 2.7-.7 3.5-1.9l.6-.85" />
      <path d="M14.5 9.3l.6-.85c.8-1.2 2.1-1.9 3.5-1.9H21" />
      <path d="M17.5 21.5L20.5 20l-3-1.5" />
    </svg>
  </button>
</template>

<style scoped>
.video-bg {
  position: fixed;
  inset: 0;
  z-index: 0;
  overflow: hidden;
  background: var(--bg);
}
.video-bg-el {
  position: absolute;
  inset: 0;
  width: 100%;
  height: 100%;
  object-fit: cover;
  filter: saturate(1.08) brightness(0.58) contrast(1.05);
  transform: scale(1.02);
  opacity: 0;
  transition: opacity calc(0.9s) ease;
  /* Unlike the app's static glass panels, this element is genuinely always
     animating under a CSS filter, a real, justified use of will-change rather than a blanket habit. */
  will-change: filter;
}
.video-bg-el.front {
  opacity: 1;
}
.video-bg-overlay {
  position: absolute;
  inset: 0;
  background: linear-gradient(180deg, rgba(5, 6, 9, 0.62) 0%, rgba(5, 6, 9, 0.38) 38%, rgba(5, 6, 9, 0.64) 100%);
}
.video-bg-vignette {
  position: absolute;
  inset: 0;
  background: radial-gradient(ellipse at 50% 40%, transparent 30%, rgba(3, 4, 6, 0.6) 100%);
}
.video-bg-grain {
  position: absolute;
  inset: 0;
  opacity: 0.035;
  mix-blend-mode: overlay;
  background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='120' height='120'%3E%3Cfilter id='n'%3E%3CfeTurbulence type='fractalNoise' baseFrequency='0.9' numOctaves='2' stitchTiles='stitch'/%3E%3C/filter%3E%3Crect width='100%25' height='100%25' filter='url(%23n)'/%3E%3C/svg%3E");
}

.media-download-note {
  position: fixed;
  left: 16px;
  bottom: 16px;
  z-index: 40;
  font-size: 11px;
  color: var(--text-muted);
  background: var(--surface-raised);
  backdrop-filter: var(--blur);
  -webkit-backdrop-filter: var(--blur);
  border: 1px solid var(--border);
  border-radius: 8px;
  padding: 5px 10px;
}

.video-reroll {
  position: fixed;
  right: 16px;
  bottom: 16px;
  z-index: 40;
  width: 36px;
  height: 36px;
  border-radius: 50%;
  display: flex;
  align-items: center;
  justify-content: center;
  background: var(--surface-raised);
  backdrop-filter: var(--blur);
  -webkit-backdrop-filter: var(--blur);
  border: 1px solid var(--border);
  color: var(--text-muted);
  cursor: pointer;
  opacity: 0.5;
  padding: 0;
  transition: opacity var(--dur-fast) ease, transform var(--dur-fast) var(--ease-spring), color var(--dur-fast) ease,
    background var(--dur-fast) ease;
}
.video-reroll:hover:not(:disabled) {
  opacity: 1;
  color: var(--text);
  background: var(--surface-strong);
  transform: scale(1.08);
}
.video-reroll:active:not(:disabled) {
  transform: scale(0.92);
}
.video-reroll:disabled {
  cursor: default;
}
.video-reroll svg {
  width: 16px;
  height: 16px;
}
.video-reroll.spinning svg {
  animation: reroll-spin 0.9s var(--ease-out);
}
@keyframes reroll-spin {
  from {
    transform: rotate(0deg);
  }
  to {
    transform: rotate(360deg);
  }
}
</style>
