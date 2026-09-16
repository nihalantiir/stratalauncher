<script setup>
import { computed, onMounted, onUnmounted, ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import { convertFileSrc } from '@tauri-apps/api/core';
import { formatSize } from '../../lib/text';

const props = defineProps({
  screenshots: { type: Array, required: true },
  index: { type: Number, required: true },
  deletingFile: { type: String, default: null },
});
const emit = defineEmits(['close', 'update:index', 'delete-requested']);
const { t } = useI18n();

const confirmingDelete = ref(false);
watch(
  () => props.index,
  () => {
    confirmingDelete.value = false;
  },
);

const current = computed(() => props.screenshots[props.index]);
const currentSrc = computed(() => (current.value ? convertFileSrc(current.value.path) : ''));
const hasPrev = computed(() => props.index > 0);
const hasNext = computed(() => props.index < props.screenshots.length - 1);
const isDeleting = computed(() => !!current.value && props.deletingFile === current.value.fileName);

function prev() {
  if (hasPrev.value) emit('update:index', props.index - 1);
}
function next() {
  if (hasNext.value) emit('update:index', props.index + 1);
}
function dateLabel(shot) {
  return shot.takenAt ? t('screenshots.takenAt', { time: new Date(shot.takenAt).toLocaleString() }) : t('screenshots.unknownDate');
}
function onKeydown(e) {
  if (e.key === 'Escape') emit('close');
  else if (e.key === 'ArrowLeft') prev();
  else if (e.key === 'ArrowRight') next();
}
onMounted(() => window.addEventListener('keydown', onKeydown));
onUnmounted(() => window.removeEventListener('keydown', onKeydown));
</script>

<template>
  <Teleport to="body">
    <div class="modal-backdrop" @click.self="emit('close')">
      <div class="lightbox" v-if="current">
        <div class="modal-head">
          <h3>{{ current.fileName }}</h3>
          <button class="modal-close" type="button" @click="emit('close')">✕</button>
        </div>

        <div class="lightbox-stage">
          <button
            v-if="hasPrev"
            class="lightbox-nav lightbox-prev"
            type="button"
            :aria-label="t('screenshots.previous')"
            @click="prev"
          >
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2"><path d="M15 6l-6 6 6 6" /></svg>
          </button>

          <img :src="currentSrc" :alt="current.fileName" />

          <button
            v-if="hasNext"
            class="lightbox-nav lightbox-next"
            type="button"
            :aria-label="t('screenshots.next')"
            @click="next"
          >
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2"><path d="M9 6l6 6-6 6" /></svg>
          </button>
        </div>

        <div class="lightbox-foot" v-if="!confirmingDelete">
          <span class="lightbox-meta">{{ dateLabel(current) }} · {{ formatSize(current.sizeBytes) }}</span>
          <button class="btn btn-danger-ghost btn-sm" type="button" @click="confirmingDelete = true">
            {{ t('screenshots.delete') }}
          </button>
        </div>
        <div class="lightbox-foot" v-else>
          <span class="lightbox-meta">{{ t('screenshots.confirmDelete') }}</span>
          <div class="world-actions">
            <button class="btn btn-ghost btn-sm" type="button" @click="confirmingDelete = false">
              {{ t('instances.cancel') }}
            </button>
            <button
              class="btn btn-danger-ghost btn-sm"
              type="button"
              :disabled="isDeleting"
              @click="emit('delete-requested', current.fileName)"
            >
              {{ isDeleting ? t('screenshots.deleting') : t('screenshots.delete') }}
            </button>
          </div>
        </div>
      </div>
    </div>
  </Teleport>
</template>
