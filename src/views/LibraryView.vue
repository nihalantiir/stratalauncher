<script setup>
import { onMounted, onActivated, onBeforeUnmount, ref, computed } from 'vue';
import { useI18n } from 'vue-i18n';
import { listen } from '@tauri-apps/api/event';
import { useVersionsStore } from '../stores/versions';
import { useAccountsStore } from '../stores/accounts';
import { useInstancesStore } from '../stores/instances';
import { launchInstance } from '../api/launch';
import InstanceCard from '../components/library/InstanceCard.vue';
import NewInstanceModal from '../components/library/NewInstanceModal.vue';
import GlassSelect from '../components/common/GlassSelect.vue';
import { panoramaForInstance } from '../lib/panoramas';
import { useContextMenu } from '../composables/useContextMenu';
import { useInstanceContextMenu } from '../composables/useInstanceContextMenu';

const { t, locale } = useI18n();
const versions = useVersionsStore();
const accounts = useAccountsStore();
const instances = useInstancesStore();
const { openContextMenu } = useContextMenu();
const { buildInstanceMenuItems } = useInstanceContextMenu();

const launching = ref(false);
const launchError = ref(null);
const progress = ref(null);
const showCreate = ref(false);
const search = ref('');
const sortKey = ref('recent');

let unlistenProgress = null;

function refreshData() {
  if (!versions.manifest) versions.fetch();
  accounts.refresh();
  instances.refresh();
}

onMounted(async () => {
  refreshData();
  unlistenProgress = await listen('download://progress', (event) => {
    const p = event.payload;
    // Hide immediately once a stage finishes, instead of leaving a "Downloading X (32/32)" row stuck on
    // screen while a later, silent stage (e.g. resolving Java) is what's actually happening.
    progress.value = p.completed >= p.total ? null : p;
  });
});

// The page is kept alive across navigation now (see App.vue), so onMounted only fires once per app session;
// this re-syncs the account/instance/version data (which could have changed while away) each time it's shown again.
onActivated(refreshData);

onBeforeUnmount(() => {
  // Only fires on a real destroy now (kept-alive pages are deactivated, not unmounted, on navigation).
  unlistenProgress?.();
});

const loaderLabel = computed(() => t(`loaders.${instances.current?.loader}`));
// Every instance shows here, including whichever one is current; clicking any tile just calls setCurrent,
// which drives the hero card above, and the clicked tile picks up the checkmark once it's the current one.
const allInstances = computed(() => instances.list);
const currentRunning = computed(() => instances.current && instances.isRunning(instances.current.id));
const canPlay = computed(() => !!accounts.active && !!instances.current && !launching.value && !currentRunning.value);
const lastPlayedText = computed(() => {
  const raw = instances.current?.lastPlayedAt;
  if (!raw) return t('library.neverPlayed');
  return t('library.lastPlayed', { time: new Date(raw).toLocaleString() });
});
const heroPanoramaUrl = computed(() => panoramaForInstance(instances.current, versions.manifest?.versions));

const sortOptions = computed(() => [
  { value: 'recent', label: t('library.sortRecent') },
  { value: 'name', label: t('library.sortName') },
]);

const filteredInstances = computed(() => {
  const q = search.value.trim().toLowerCase();
  const list = allInstances.value.filter((i) => !q || i.name.toLowerCase().includes(q));
  if (sortKey.value === 'name') return [...list].sort((a, b) => a.name.localeCompare(b.name, locale.value));
  return [...list].sort((a, b) => {
    const at = new Date(a.lastPlayedAt || a.createdAt).getTime();
    const bt = new Date(b.lastPlayedAt || b.createdAt).getTime();
    return bt - at;
  });
});

// Grouped display only kicks in once at least one instance actually has a group set, so nobody who's
// never touched the field sees any change here.
const hasAnyGroups = computed(() => allInstances.value.some((i) => i.groupName));
const groupedInstances = computed(() => {
  const groups = new Map();
  for (const inst of filteredInstances.value) {
    const key = inst.groupName || null;
    if (!groups.has(key)) groups.set(key, []);
    groups.get(key).push(inst);
  }
  const entries = [...groups.entries()].map(([key, list]) => ({ key, label: key ?? t('library.ungrouped'), list }));
  entries.sort((a, b) => {
    if (a.key === null) return -1;
    if (b.key === null) return 1;
    return a.label.localeCompare(b.label, locale.value);
  });
  return entries;
});

// Group header management: rename a group (bulk-updates every instance in it) or drag a tile onto a different
// section's header to move it there. Ungrouped never gets rename controls; dragging into it still works via groupKey === null.

// `undefined` means "nothing being edited"; this must NOT be `null`, since `null` is Ungrouped's real key. An
// earlier version used `null` as the sentinel, which collided with it and permanently rendered Ungrouped's rename input instead of its label.
const editingGroupKey = ref(undefined);
const renameValue = ref('');
// Same reasoning as editingGroupKey above: `undefined` for "nothing being dragged over", distinct from
// `null`, the real key of the Ungrouped section and a legitimate drop target.
const dragOverKey = ref(undefined);

function startRename(group) {
  editingGroupKey.value = group.key;
  renameValue.value = group.key;
}
function cancelRename() {
  editingGroupKey.value = undefined;
  renameValue.value = '';
}
async function confirmRename(oldKey) {
  if (editingGroupKey.value !== oldKey) return; // already resolved (e.g. Enter's confirm beat blur's)
  const newName = renameValue.value.trim();
  editingGroupKey.value = undefined;
  if (newName === oldKey) return;
  const members = allInstances.value.filter((i) => (i.groupName || null) === oldKey);
  await Promise.all(members.map((inst) => instances.update({ ...inst, groupName: newName || null })));
}

function onDragEnter(groupKey) {
  dragOverKey.value = groupKey;
}
function onDragLeave(groupKey) {
  if (dragOverKey.value === groupKey) dragOverKey.value = undefined;
}
async function onDrop(event, groupKey) {
  dragOverKey.value = undefined;
  const instanceId = event.dataTransfer.getData('text/plain');
  const inst = allInstances.value.find((i) => i.id === instanceId);
  if (!inst || (inst.groupName || null) === groupKey) return;
  await instances.update({ ...inst, groupName: groupKey });
}

async function play(instanceId) {
  if (!accounts.active) return;
  launching.value = true;
  launchError.value = null;
  progress.value = null;
  try {
    await launchInstance(instanceId);
    instances.refresh();
  } catch (e) {
    launchError.value = String(e);
  } finally {
    launching.value = false;
    progress.value = null;
  }
}

function stopCurrent() {
  if (instances.current) instances.stopInstance(instances.current.id);
}

function selectInstance(id) {
  instances.setCurrent(id);
}

function onHeroContextMenu(event) {
  if (!instances.current) return;
  openContextMenu(event, buildInstanceMenuItems(instances.current));
}
</script>

<template>
  <section class="view">
    <div v-if="!instances.loading && instances.list.length === 0" class="empty-state" style="margin-top: 40px">
      <p style="margin: 0 0 14px">{{ t('library.empty') }}</p>
      <button class="btn btn-mineral" type="button" @click="showCreate = true">{{ t('library.emptyAction') }}</button>
    </div>

    <template v-else>
      <div class="library-top" v-if="instances.current">
        <div class="hero-card hero-full" @contextmenu="onHeroContextMenu">
          <div class="hero-art">
            <img
              class="hero-panorama"
              :src="heroPanoramaUrl"
              :key="heroPanoramaUrl"
              alt=""
              decoding="async"
              loading="eager"
              fetchpriority="high"
            />
          </div>
          <div class="hero-panel">
            <div class="hero-tags" style="display: flex; align-items: center; gap: 8px; margin-bottom: 10px">
              <span class="pill">{{ loaderLabel }}</span>
              <span class="pill mono">{{ instances.current.mcVersion }}</span>
              <span v-if="instances.current.lastCrashed" class="crash-badge" v-tooltip="t('library.crashed')">
                <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                  <path d="M10.29 3.86L1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0z" />
                  <path d="M12 9v4M12 17h.01" />
                </svg>
              </span>
            </div>
            <h2 :title="instances.current.name">{{ instances.current.name }}</h2>
            <p style="font-size: 12.5px; color: rgba(244, 239, 228, 0.75); margin-bottom: 4px">{{ lastPlayedText }}</p>

            <div class="hero-actions" style="display: flex; gap: 10px">
              <button v-if="currentRunning" class="btn btn-lg btn-danger-ghost" type="button" @click="stopCurrent">
                <svg viewBox="0 0 24 24" fill="currentColor"><rect x="6" y="6" width="12" height="12" rx="1.5" /></svg>
                {{ t('library.stop') }}
              </button>
              <button v-else class="btn btn-lg btn-mineral" :disabled="!canPlay" @click="play(instances.current.id)">
                <span v-if="launching" class="spinner"></span>
                <svg v-else viewBox="0 0 24 24" fill="currentColor"><path d="M7 5l12 7-12 7z" /></svg>
                {{ launching ? t('library.launching') : t('library.play') }}
              </button>
            </div>

            <div class="status-row" v-if="!accounts.active">
              <span class="status-dot warn"></span>
              <span>{{ t('library.needAccount') }}</span>
            </div>

            <div class="status-row" v-if="progress">
              <span class="spinner"></span>
              <span>{{
                t('library.downloadProgress', {
                  stage: t(`library.downloadStage.${progress.stage}`),
                  completed: progress.completed,
                  total: progress.total,
                })
              }}</span>
            </div>

            <div v-if="launchError" class="error-box" style="margin-top: 14px; max-width: 480px">{{ launchError }}</div>
          </div>
        </div>
      </div>

      <div class="library-bottom">
        <div class="grid-head">
          <h3>{{ t('library.yourInstances') }}</h3>
          <div class="grid-head-actions">
            <button class="btn btn-ghost btn-sm" type="button" @click="showCreate = true">
              {{ t('library.newInstance') }}
            </button>
          </div>
        </div>

        <div v-if="versions.error" class="error-box" style="margin-bottom: 14px">
          {{ t('versions.error') }}
          <button class="btn btn-ghost btn-sm" style="margin-left: 8px" @click="versions.fetch">{{ t('versions.retry') }}</button>
        </div>

        <div class="search-row">
          <div class="search-box">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <circle cx="11" cy="11" r="7" /><path d="M21 21l-4.3-4.3" />
            </svg>
            <input v-model="search" :placeholder="t('library.filterPlaceholder')" />
          </div>
          <GlassSelect v-model="sortKey" :options="sortOptions" style="width: 170px" />
        </div>

        <div v-if="filteredInstances.length === 0" class="empty-state">{{ t('library.noMatches') }}</div>
        <template v-else-if="hasAnyGroups">
          <div
            v-for="group in groupedInstances"
            :key="group.key ?? '__ungrouped'"
            class="library-group"
            :class="{ 'drag-over': dragOverKey === group.key }"
            @dragover.prevent
            @dragenter.prevent="onDragEnter(group.key)"
            @dragleave="onDragLeave(group.key)"
            @drop="onDrop($event, group.key)"
          >
            <div class="library-group-label-row">
              <input
                v-if="editingGroupKey === group.key"
                v-model="renameValue"
                class="field library-group-rename-input"
                maxlength="48"
                @keyup.enter="confirmRename(group.key)"
                @keyup.esc="cancelRename"
                @blur="confirmRename(group.key)"
                @click.stop
              />
              <template v-else>
                <h4 class="library-group-label">{{ group.label }}</h4>
                <button
                  v-if="group.key !== null"
                  class="library-group-edit"
                  type="button"
                  v-tooltip="t('library.renameGroup')"
                  @click="startRename(group)"
                >
                  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                    <path d="M12 20h9" />
                    <path d="M16.5 3.5a2.121 2.121 0 0 1 3 3L7 19l-4 1 1-4Z" />
                  </svg>
                </button>
              </template>
            </div>
            <div class="instances">
              <InstanceCard
                v-for="inst in group.list"
                :key="inst.id"
                :instance="inst"
                :selected="inst.id === instances.current?.id"
                @select="selectInstance"
              />
            </div>
          </div>
        </template>
        <div v-else class="instances">
          <InstanceCard
            v-for="inst in filteredInstances"
            :key="inst.id"
            :instance="inst"
            :selected="inst.id === instances.current?.id"
            @select="selectInstance"
          />
        </div>
      </div>
    </template>

    <Transition name="modal">
      <NewInstanceModal v-if="showCreate" @close="showCreate = false" />
    </Transition>
  </section>
</template>
