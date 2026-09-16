<script setup>
import { computed } from 'vue';
import { useI18n } from 'vue-i18n';
import { useInstancesStore } from '../../stores/instances';
import { useVersionsStore } from '../../stores/versions';
import { panoramaForInstance } from '../../lib/panoramas';
import { useContextMenu } from '../../composables/useContextMenu';
import { useInstanceContextMenu } from '../../composables/useInstanceContextMenu';

const props = defineProps({
  instance: { type: Object, required: true },
  selected: { type: Boolean, default: false },
});
const emit = defineEmits(['select']);

const { t } = useI18n();
const instances = useInstancesStore();
const versions = useVersionsStore();
const { openContextMenu } = useContextMenu();
const { buildInstanceMenuItems } = useInstanceContextMenu();

const loaderLabel = computed(() => t(`loaders.${props.instance.loader}`));
const running = computed(() => instances.isRunning(props.instance.id));
const panoramaUrl = computed(() => panoramaForInstance(props.instance, versions.manifest?.versions));

// Lets a tile be dropped onto a group section header in LibraryView.vue to reassign its group;
// the id is all the drop handler needs, it looks the real instance back up from the store.
function onDragStart(event) {
  event.dataTransfer.setData('text/plain', props.instance.id);
  event.dataTransfer.effectAllowed = 'move';
}

function onContextMenu(event) {
  openContextMenu(event, buildInstanceMenuItems(props.instance));
}
</script>

<template>
  <div
    class="card"
    :class="{ selected }"
    draggable="true"
    @click="emit('select', instance.id)"
    @dragstart="onDragStart"
    @contextmenu="onContextMenu"
  >
    <div class="thumb">
      <img class="thumb-panorama" :src="panoramaUrl" alt="" decoding="async" loading="lazy" />
      <div class="thumb-fade"></div>
      <div v-if="running" class="running-tag">{{ t('library.running') }}</div>
    </div>
    <div class="card-body">
      <div class="card-text">
        <p class="cname">{{ instance.name }}</p>
        <div class="card-tags">
          <span class="tag">{{ loaderLabel }}</span>
          <span class="tag mono">{{ instance.mcVersion }}</span>
          <span v-if="instance.lastCrashed" class="crash-badge crash-badge-sm" v-tooltip="t('library.crashed')">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <path d="M10.29 3.86L1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0z" />
              <path d="M12 9v4M12 17h.01" />
            </svg>
          </span>
        </div>
      </div>
      <div class="current-mark">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3"><path d="M20 6L9 17l-5-5" /></svg>
      </div>
    </div>
  </div>
</template>
