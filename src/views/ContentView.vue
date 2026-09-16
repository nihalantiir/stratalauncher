<script setup>
import { ref, reactive, computed, onMounted, onActivated, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import { openPath } from '@tauri-apps/plugin-opener';
import { useInstancesStore } from '../stores/instances';
import * as api from '../api/content';
import DownloadContentModal from '../components/content/DownloadContentModal.vue';
import ChangeVersionModal from '../components/content/ChangeVersionModal.vue';
import GlassSelect from '../components/common/GlassSelect.vue';
import { firstGrapheme, formatSize } from '../lib/text';
import { useContextMenu } from '../composables/useContextMenu';

const props = defineProps({
  kind: { type: String, required: true },
});

const { t, locale } = useI18n();
const instances = useInstancesStore();
const { openContextMenu } = useContextMenu();

const items = ref([]);
const loading = ref(false);
const error = ref(null);
const updates = ref({});
const showDownload = ref(false);
const sortKey = ref('name');
const search = ref('');
const confirmingRemove = ref({});
const removingFile = ref(null);
const updatingAll = ref(false);
const selectMode = ref(false);
const selected = reactive(new Set());
const confirmingBulkRemove = ref(false);
const changeVersionItem = ref(null);
const bulkRemoving = ref(false);
const sortOptions = computed(() => [
  { value: 'name', label: t('content.sortName') },
  { value: 'size', label: t('content.sortSize') },
  { value: 'enabled', label: t('content.sortEnabled') },
]);

const instanceId = computed(() => instances.current?.id);
const needsLoader = computed(() => props.kind === 'mod' && instances.current?.loader === 'vanilla');
// Selection-scoped: with specific items checked (see the select-mode bar),
// "Check for updates" acts only on those; with nothing checked, it scans everything installed.
const updateScope = computed(() =>
  selectMode.value && selected.size > 0 ? items.value.filter((item) => selected.has(item.filename)) : items.value,
);
const updateCount = computed(
  () => updateScope.value.filter((item) => updates.value[item.filename]?.updateAvailable).length,
);

const sortedItems = computed(() => {
  const q = search.value.trim().toLowerCase();
  const list = items.value.filter((item) => !q || item.title.toLowerCase().includes(q));
  if (sortKey.value === 'size') list.sort((a, b) => b.sizeBytes - a.sizeBytes);
  else if (sortKey.value === 'enabled') list.sort((a, b) => Number(b.enabled) - Number(a.enabled));
  // Collate using the app's chosen language, not the OS default; matters for
  // languages with their own sort order (e.g. Polish "ł" sorts distinctly from plain "l" under pl collation).
  else list.sort((a, b) => a.title.localeCompare(b.title, locale.value));
  return list;
});

// Split from checkUpdates() below: toggling/removing an item never changes
// any version, so those call relist() alone instead of paying for a full
// update re-check on every single click.
async function relist() {
  if (!instanceId.value || needsLoader.value) return;
  loading.value = true;
  error.value = null;
  try {
    items.value = await api.listInstalledContent(instanceId.value, props.kind);
  } catch (e) {
    error.value = String(e);
  } finally {
    loading.value = false;
  }
}

async function refresh() {
  await relist();
  checkUpdates();
}

async function checkUpdates() {
  try {
    const results = await api.checkContentUpdates(instanceId.value, props.kind);
    const map = {};
    for (const r of results) map[r.filename] = r;
    updates.value = map;
  } catch {
    // Update-check failures shouldn't block viewing the installed list.
  }
}

onMounted(refresh);
watch(
  () => [instanceId.value, props.kind],
  () => {
    exitSelectMode();
    refresh();
  },
);
// Kept alive across navigation (see App.vue): also re-checks for updates and
// re-lists content installed via some other path (e.g. a manual file drop) while the page was last shown.
onActivated(refresh);

async function toggle(item) {
  await api.toggleContent(instanceId.value, props.kind, item.filename);
  relist();
}

async function remove(item) {
  removingFile.value = item.filename;
  try {
    await api.removeContent(instanceId.value, props.kind, item.filename);
    confirmingRemove.value = { ...confirmingRemove.value, [item.filename]: false };
    await relist();
  } finally {
    removingFile.value = null;
  }
}

async function performUpdate(item) {
  const newItem = await api.installContent(
    instanceId.value,
    props.kind,
    item.source ?? 'modrinth',
    item.projectId,
    item.title,
    item.iconUrl,
    instances.current.mcVersion,
  );
  if (newItem.filename !== item.filename) {
    await api.removeContent(instanceId.value, props.kind, item.filename);
  }
}

async function update(item) {
  await performUpdate(item);
  refresh();
}

async function checkForUpdatesAction() {
  updatingAll.value = true;
  error.value = null;
  try {
    await checkUpdates(); // re-check, not just apply whatever was last cached
    const toUpdate = updateScope.value.filter((item) => updates.value[item.filename]?.updateAvailable);
    const results = await Promise.allSettled(toUpdate.map(performUpdate));
    const failed = results.filter((r) => r.status === 'rejected');
    if (failed.length > 0) {
      error.value = t('content.updateAllPartialFailure', { count: failed.length });
    }
  } finally {
    updatingAll.value = false;
    await refresh();
  }
}

function openChangeVersion(item) {
  changeVersionItem.value = item;
}

function enterSelectMode() {
  selectMode.value = true;
}

function exitSelectMode() {
  selectMode.value = false;
  selected.clear();
  confirmingBulkRemove.value = false;
}

function toggleSelected(filename) {
  if (selected.has(filename)) selected.delete(filename);
  else selected.add(filename);
}

function toggleSelectAll() {
  if (selected.size === sortedItems.value.length) {
    selected.clear();
  } else {
    selected.clear();
    sortedItems.value.forEach((item) => selected.add(item.filename));
  }
}

async function removeSelected() {
  bulkRemoving.value = true;
  try {
    const filenames = [...selected];
    await Promise.all(filenames.map((filename) => api.removeContent(instanceId.value, props.kind, filename)));
    exitSelectMode();
    await relist();
  } finally {
    bulkRemoving.value = false;
  }
}

async function openContentFolder() {
  if (!instanceId.value) return;
  const dir = await api.getContentDir(instanceId.value, props.kind);
  await openPath(dir);
}

function onContextMenu(event, item) {
  openContextMenu(event, [
    ...(updates.value[item.filename]?.updateAvailable ? [{ label: t('content.update'), icon: 'upload', action: () => update(item) }] : []),
    ...(item.projectId ? [{ label: t('content.changeVersion'), icon: 'history', action: () => openChangeVersion(item) }] : []),
    { label: item.enabled ? t('content.disable') : t('content.enable'), icon: 'power', action: () => toggle(item) },
    'separator',
    { label: t('content.openFolder'), icon: 'folder', action: openContentFolder },
    'separator',
    {
      label: t('content.remove'),
      icon: 'trash',
      danger: true,
      action: () => (confirmingRemove.value = { ...confirmingRemove.value, [item.filename]: true }),
    },
  ]);
}

</script>

<template>
  <section class="view">
    <div v-if="!instances.current" class="empty-state">
      <p style="margin: 0 0 14px">{{ t('version.noInstance') }}</p>
      <router-link to="/" class="btn btn-mineral">{{ t('sidebar.library') }}</router-link>
    </div>
    <div v-else-if="needsLoader" class="empty-state">{{ t('content.needsLoader') }}</div>

    <template v-else>
      <div class="view-toolbar">
        <p class="view-intro" style="margin: 0">{{ t(`content.intro.${kind}`) }}</p>
        <div class="view-toolbar-actions">
          <span v-if="items.length && !selectMode" class="view-count">{{ t('content.count', { count: items.length }) }}</span>
          <button v-if="items.length && !selectMode" class="btn btn-ghost btn-sm" type="button" @click="enterSelectMode">
            {{ t('content.select') }}
          </button>
          <button class="icon-btn" type="button" v-tooltip="t('content.openFolder')" @click="openContentFolder">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <path d="M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v7a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V7Z" />
            </svg>
          </button>
          <GlassSelect v-model="sortKey" :options="sortOptions" style="width: 160px" />
          <button class="btn btn-mineral btn-sm" type="button" @click="showDownload = true">
            {{ t('content.download') }}
          </button>
        </div>
      </div>

      <div v-if="items.length" class="search-row">
        <div class="search-box">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <circle cx="11" cy="11" r="7" /><path d="M21 21l-4.3-4.3" />
          </svg>
          <input v-model="search" :placeholder="t('content.filterPlaceholder')" />
        </div>
        <button
          v-if="updateCount > 0"
          class="btn btn-ghost btn-sm"
          type="button"
          :disabled="updatingAll"
          @click="checkForUpdatesAction"
        >
          {{ updatingAll ? t('content.updatingAll') : t('content.updateAll') }}
          <span v-if="!updatingAll" class="tag mono" style="margin-left: 6px">{{ updateCount }}</span>
        </button>
      </div>

      <div v-if="error" class="error-box">{{ error }}</div>

      <div v-if="selectMode" class="select-bar">
        <template v-if="!confirmingBulkRemove">
          <span class="view-count">{{ t('content.selectedCount', { count: selected.size }) }}</span>
          <div class="view-toolbar-actions">
            <button class="btn btn-ghost btn-sm" type="button" @click="toggleSelectAll">
              {{ selected.size === sortedItems.length ? t('content.clearSelection') : t('content.selectAll') }}
            </button>
            <button
              class="btn btn-danger-ghost btn-sm"
              type="button"
              :disabled="selected.size === 0"
              @click="confirmingBulkRemove = true"
            >
              {{ t('content.removeSelected') }}
            </button>
            <button class="btn btn-ghost btn-sm" type="button" @click="exitSelectMode">{{ t('content.cancelSelect') }}</button>
          </div>
        </template>
        <template v-else>
          <span class="view-count">{{ t('content.confirmRemoveMany', { count: selected.size }) }}</span>
          <div class="view-toolbar-actions">
            <button class="btn btn-ghost btn-sm" type="button" :disabled="bulkRemoving" @click="confirmingBulkRemove = false">
              {{ t('instances.cancel') }}
            </button>
            <button class="btn btn-danger-ghost btn-sm" type="button" :disabled="bulkRemoving" @click="removeSelected">
              {{ bulkRemoving ? t('content.removing') : t('content.removeSelected') }}
            </button>
          </div>
        </template>
      </div>

      <div v-if="!loading && sortedItems.length === 0" class="empty-state">{{ t(`content.empty.${kind}`) }}</div>

      <div
        v-for="item in sortedItems"
        :key="item.filename"
        class="row-card"
        :class="{ 'select-mode': selectMode, selected: selected.has(item.filename) }"
        @click="selectMode && toggleSelected(item.filename)"
        @contextmenu="!selectMode && onContextMenu($event, item)"
      >
        <div v-if="selectMode" class="row-check" :class="{ checked: selected.has(item.filename) }">
          <svg v-if="selected.has(item.filename)" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.6">
            <path d="M5 12l4.5 4.5L19 7" />
          </svg>
        </div>
        <img v-if="item.iconUrl" :src="item.iconUrl" class="row-icon" alt="" />
        <div v-else class="row-icon">{{ firstGrapheme(item.title).toUpperCase() }}</div>
        <div class="row-info">
          <div class="rtitle">
            <h4>{{ item.title }}</h4>
            <span v-if="item.versionNumber" class="tag mono">{{ item.versionNumber }}</span>
            <span v-if="item.source" class="src-badge" :class="`src-${item.source}`">
              {{ item.source === 'curseforge' ? 'CurseForge' : 'Modrinth' }}
            </span>
            <span v-if="updates[item.filename]?.updateAvailable" class="src-badge update-badge">
              {{ t('content.updateAvailable') }}
            </span>
          </div>
          <p>{{ formatSize(item.sizeBytes) }}</p>
        </div>

        <template v-if="!selectMode">
          <div class="row-actions" v-if="!confirmingRemove[item.filename]">
            <button
              v-if="updates[item.filename]?.updateAvailable"
              class="icon-btn icon-btn-accent"
              type="button"
              v-tooltip="t('content.update')"
              @click="update(item)"
            >
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <path d="M12 3v12m0-12 4 4m-4-4-4 4M4 17v2a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2v-2" />
              </svg>
            </button>
            <div
              class="switch"
              :class="{ off: !item.enabled }"
              role="switch"
              :aria-checked="item.enabled"
              tabindex="0"
              v-tooltip="item.enabled ? t('content.disable') : t('content.enable')"
              @click="toggle(item)"
              @keyup.enter="toggle(item)"
            ></div>
            <span class="row-actions-divider"></span>
            <button
              class="icon-btn icon-btn-danger"
              type="button"
              v-tooltip="t('content.remove')"
              @click="confirmingRemove = { ...confirmingRemove, [item.filename]: true }"
            >
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <path d="M3 6h18M8 6V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2m3 0-1 14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2L4 6h16Z" />
              </svg>
            </button>
          </div>
          <div class="row-actions" v-else>
            <button
              class="btn btn-ghost btn-sm"
              type="button"
              @click="confirmingRemove = { ...confirmingRemove, [item.filename]: false }"
            >
              {{ t('instances.cancel') }}
            </button>
            <button
              class="btn btn-danger-ghost btn-sm"
              type="button"
              :disabled="removingFile === item.filename"
              @click="remove(item)"
            >
              {{ removingFile === item.filename ? t('content.removing') : t('content.remove') }}
            </button>
          </div>
        </template>
      </div>
    </template>

    <Transition name="modal">
      <DownloadContentModal
        v-if="showDownload"
        :instance-id="instanceId"
        :kind="kind"
        :installed-project-ids="items.map((i) => i.projectId).filter(Boolean)"
        @close="showDownload = false"
        @installed="refresh"
      />
    </Transition>

    <Transition name="modal">
      <ChangeVersionModal
        v-if="changeVersionItem"
        :instance-id="instanceId"
        :kind="kind"
        :item="changeVersionItem"
        @close="changeVersionItem = null"
        @changed="refresh"
      />
    </Transition>
  </section>
</template>
