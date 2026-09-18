<script setup>
import { ref, reactive, computed, onMounted, onActivated, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import { openPath } from '../api/opener';
import { convertFileSrc } from '@tauri-apps/api/core';
import { useInstancesStore } from '../stores/instances';
import * as api from '../api/screenshots';
import ScreenshotLightbox from '../components/screenshots/ScreenshotLightbox.vue';
import { useContextMenu } from '../composables/useContextMenu';

const { t } = useI18n();
const instances = useInstancesStore();
const { openContextMenu } = useContextMenu();

const shots = ref([]);
const loading = ref(false);
const error = ref(null);
const confirmingDelete = ref({});
const deletingFile = ref(null);
const lightboxIndex = ref(null);

const selectMode = ref(false);
const selected = reactive(new Set());
const confirmingBulkDelete = ref(false);
const bulkDeleting = ref(false);

const instanceId = computed(() => instances.current?.id);

async function refresh() {
  if (!instanceId.value) return;
  loading.value = true;
  error.value = null;
  try {
    const list = await api.listScreenshots(instanceId.value);
    // Generates any thumbnails missing since the last refresh so the grid
    // never points at a thumbnail file that doesn't exist yet.
    await api.ensureScreenshotThumbnails(instanceId.value);
    shots.value = list;
  } catch (e) {
    error.value = String(e);
  } finally {
    loading.value = false;
  }
}

onMounted(refresh);
watch(instanceId, () => {
  exitSelectMode();
  refresh();
});
// A play session between visits can add new screenshots without changing
// instanceId, so this needs its own refresh too, same reasoning as Worlds.
onActivated(refresh);

async function removeScreenshot(fileName) {
  deletingFile.value = fileName;
  try {
    await api.deleteScreenshot(instanceId.value, fileName);
    const idx = shots.value.findIndex((s) => s.fileName === fileName);
    if (idx !== -1) shots.value.splice(idx, 1);
    confirmingDelete.value = { ...confirmingDelete.value, [fileName]: false };
    if (lightboxIndex.value !== null) {
      if (shots.value.length === 0) lightboxIndex.value = null;
      else if (lightboxIndex.value >= shots.value.length) lightboxIndex.value = shots.value.length - 1;
    }
  } catch (e) {
    error.value = String(e);
  } finally {
    deletingFile.value = null;
  }
}

function enterSelectMode() {
  selectMode.value = true;
}

function exitSelectMode() {
  selectMode.value = false;
  selected.clear();
  confirmingBulkDelete.value = false;
}

function toggleSelected(fileName) {
  if (selected.has(fileName)) selected.delete(fileName);
  else selected.add(fileName);
}

function toggleSelectAll() {
  if (selected.size === shots.value.length) {
    selected.clear();
  } else {
    selected.clear();
    shots.value.forEach((s) => selected.add(s.fileName));
  }
}

async function deleteSelected() {
  bulkDeleting.value = true;
  try {
    const names = [...selected];
    await Promise.all(names.map((name) => api.deleteScreenshot(instanceId.value, name)));
    shots.value = shots.value.filter((s) => !selected.has(s.fileName));
    exitSelectMode();
  } catch (e) {
    error.value = String(e);
  } finally {
    bulkDeleting.value = false;
  }
}

async function openScreenshotsFolder() {
  const dir = await api.getScreenshotsDir(instanceId.value);
  await openPath(dir);
}

function fileSrc(shot) {
  return convertFileSrc(shot.thumbPath);
}

function dateLabel(shot) {
  return shot.takenAt ? t('screenshots.takenAt', { time: new Date(shot.takenAt).toLocaleString() }) : t('screenshots.unknownDate');
}

function onCardActivate(shot, i) {
  if (selectMode.value) toggleSelected(shot.fileName);
  else lightboxIndex.value = i;
}

function onContextMenu(event, shot, i) {
  openContextMenu(event, [
    { label: t('screenshots.view'), icon: 'eye', action: () => (lightboxIndex.value = i) },
    'separator',
    {
      label: t('screenshots.delete'),
      icon: 'trash',
      danger: true,
      action: () => (confirmingDelete.value = { ...confirmingDelete.value, [shot.fileName]: true }),
    },
  ]);
}
</script>

<template>
  <section class="view" v-if="!instanceId">
    <div class="empty-state">
      <p style="margin: 0 0 14px">{{ t('version.noInstance') }}</p>
      <router-link to="/" class="btn btn-mineral">{{ t('sidebar.library') }}</router-link>
    </div>
  </section>

  <section class="view" v-else>
    <div class="view-toolbar">
      <p class="view-intro" style="margin: 0">{{ t('screenshots.intro') }}</p>
      <div class="view-toolbar-actions">
        <span v-if="shots.length && !selectMode" class="view-count">{{ t('screenshots.count', { count: shots.length }) }}</span>
        <button v-if="shots.length && !selectMode" class="btn btn-ghost btn-sm" type="button" @click="enterSelectMode">
          {{ t('screenshots.select') }}
        </button>
        <button class="btn btn-ghost btn-sm" type="button" @click="openScreenshotsFolder">
          {{ t('screenshots.openFolder') }}
        </button>
      </div>
    </div>

    <div v-if="error" class="error-box">{{ error }}</div>

    <div v-if="selectMode" class="select-bar">
      <template v-if="!confirmingBulkDelete">
        <span class="view-count">{{ t('screenshots.selectedCount', { count: selected.size }) }}</span>
        <div class="view-toolbar-actions">
          <button class="btn btn-ghost btn-sm" type="button" @click="toggleSelectAll">
            {{ selected.size === shots.length ? t('screenshots.clearSelection') : t('screenshots.selectAll') }}
          </button>
          <button
            class="btn btn-danger-ghost btn-sm"
            type="button"
            :disabled="selected.size === 0"
            @click="confirmingBulkDelete = true"
          >
            {{ t('screenshots.deleteSelected') }}
          </button>
          <button class="btn btn-ghost btn-sm" type="button" @click="exitSelectMode">{{ t('screenshots.cancelSelect') }}</button>
        </div>
      </template>
      <template v-else>
        <span class="view-count">{{ t('screenshots.confirmDeleteMany', { count: selected.size }) }}</span>
        <div class="view-toolbar-actions">
          <button class="btn btn-ghost btn-sm" type="button" :disabled="bulkDeleting" @click="confirmingBulkDelete = false">
            {{ t('instances.cancel') }}
          </button>
          <button class="btn btn-danger-ghost btn-sm" type="button" :disabled="bulkDeleting" @click="deleteSelected">
            {{ bulkDeleting ? t('screenshots.deleting') : t('screenshots.deleteSelected') }}
          </button>
        </div>
      </template>
    </div>

    <div v-if="!loading && shots.length === 0" class="empty-state">{{ t('screenshots.empty') }}</div>

    <div class="screenshots-grid">
      <div
        v-for="(shot, i) in shots"
        :key="shot.fileName"
        class="screenshot-card"
        :class="{ selected: selected.has(shot.fileName) }"
        tabindex="0"
        role="button"
        @click="onCardActivate(shot, i)"
        @keydown.enter="onCardActivate(shot, i)"
        @contextmenu="!selectMode && onContextMenu($event, shot, i)"
      >
        <img :src="fileSrc(shot)" :alt="shot.fileName" loading="lazy" />

        <div v-if="selectMode" class="screenshot-check" :class="{ checked: selected.has(shot.fileName) }">
          <svg v-if="selected.has(shot.fileName)" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.6">
            <path d="M5 12l4.5 4.5L19 7" />
          </svg>
        </div>

        <template v-else>
          <div class="screenshot-overlay" v-if="!confirmingDelete[shot.fileName]">
            <span class="screenshot-date">{{ dateLabel(shot) }}</span>
            <button
              class="screenshot-delete"
              type="button"
              :aria-label="t('screenshots.delete')"
              @click.stop="confirmingDelete = { ...confirmingDelete, [shot.fileName]: true }"
            >
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <path d="M4 7h16M9 7V5a1 1 0 0 1 1-1h4a1 1 0 0 1 1 1v2m-9 0 1 13a1 1 0 0 0 1 1h8a1 1 0 0 0 1-1l1-13" />
              </svg>
            </button>
          </div>
          <div class="screenshot-confirm" v-else @click.stop>
            <p>{{ t('screenshots.confirmDelete') }}</p>
            <div class="screenshot-confirm-actions">
              <button
                class="btn btn-ghost btn-sm"
                type="button"
                @click="confirmingDelete = { ...confirmingDelete, [shot.fileName]: false }"
              >
                {{ t('instances.cancel') }}
              </button>
              <button
                class="btn btn-danger-ghost btn-sm"
                type="button"
                :disabled="deletingFile === shot.fileName"
                @click="removeScreenshot(shot.fileName)"
              >
                {{ deletingFile === shot.fileName ? t('screenshots.deleting') : t('screenshots.delete') }}
              </button>
            </div>
          </div>
        </template>
      </div>
    </div>

    <Transition name="modal">
      <ScreenshotLightbox
        v-if="lightboxIndex !== null"
        :screenshots="shots"
        :index="lightboxIndex"
        :deleting-file="deletingFile"
        @update:index="lightboxIndex = $event"
        @close="lightboxIndex = null"
        @delete-requested="removeScreenshot"
      />
    </Transition>
  </section>
</template>
