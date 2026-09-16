<script setup>
import { ref, computed, onMounted, onActivated, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import { openPath } from '../api/opener';
import { useInstancesStore } from '../stores/instances';
import * as api from '../api/worlds';
import BackupHistoryModal from '../components/worlds/BackupHistoryModal.vue';
import { firstGrapheme, formatSize } from '../lib/text';
import { useContextMenu } from '../composables/useContextMenu';

const { t } = useI18n();
const instances = useInstancesStore();
const { openContextMenu } = useContextMenu();

const worlds = ref([]);
const loading = ref(false);
const error = ref(null);
const backingUp = ref({});
const confirmingRestore = ref({});
const restoring = ref({});
const confirmingDeleteWorld = ref({});
const deletingWorld = ref({});
const historyWorld = ref(null);

const instanceId = computed(() => instances.current?.id);
const biome = computed(() => instances.current?.iconBiome ?? 'ore');

async function refresh() {
  if (!instanceId.value) return;
  loading.value = true;
  error.value = null;
  try {
    worlds.value = await api.listWorlds(instanceId.value);
  } catch (e) {
    error.value = String(e);
  } finally {
    loading.value = false;
  }
}

onMounted(refresh);
watch(instanceId, refresh);
// Kept alive across navigation (see App.vue). The instanceId watcher above covers switching
// instances, but a play session in between doesn't change instanceId, so this catches new worlds too.
onActivated(refresh);

async function backupNow(world) {
  backingUp.value[world.folder] = true;
  try {
    await api.backupWorld(instanceId.value, world.folder);
    await refresh();
  } catch (e) {
    error.value = String(e);
  } finally {
    backingUp.value[world.folder] = false;
  }
}

async function restoreBackup(world) {
  restoring.value[world.folder] = true;
  try {
    await api.restoreLatestBackup(instanceId.value, world.folder);
    confirmingRestore.value[world.folder] = false;
    await refresh();
  } catch (e) {
    error.value = String(e);
  } finally {
    restoring.value[world.folder] = false;
  }
}

async function removeWorld(world) {
  deletingWorld.value[world.folder] = true;
  try {
    await api.deleteWorld(instanceId.value, world.folder);
    confirmingDeleteWorld.value[world.folder] = false;
    await refresh();
  } catch (e) {
    error.value = String(e);
  } finally {
    deletingWorld.value[world.folder] = false;
  }
}

async function openSavesFolder() {
  const dir = await api.getSavesDir(instanceId.value);
  await openPath(dir);
}

async function openWorldFolder(world) {
  const dir = await api.getWorldDir(instanceId.value, world.folder);
  await openPath(dir);
}

function openHistory(world) {
  historyWorld.value = world;
}

function onContextMenu(event, world) {
  openContextMenu(event, [
    {
      label: backingUp.value[world.folder] ? t('worlds.backingUp') : t('worlds.backupNow'),
      icon: 'upload',
      disabled: backingUp.value[world.folder],
      action: () => backupNow(world),
    },
    ...(world.lastBackup
      ? [
          {
            label: t('worlds.restore'),
            icon: 'rotate',
            action: () => (confirmingRestore.value[world.folder] = true),
          },
          { label: t('worlds.history'), icon: 'history', action: () => openHistory(world) },
        ]
      : []),
    'separator',
    { label: t('worlds.openWorldFolder'), icon: 'folder', action: () => openWorldFolder(world) },
    'separator',
    {
      label: t('worlds.deleteWorld'),
      icon: 'trash',
      danger: true,
      action: () => (confirmingDeleteWorld.value[world.folder] = true),
    },
  ]);
}

function lastPlayedLabel(world) {
  return world.lastPlayed ? t('library.lastPlayed', { time: new Date(world.lastPlayed).toLocaleString() }) : t('library.neverPlayed');
}

function backupLabel(world) {
  return world.lastBackup ? t('worlds.backedUpAt', { time: new Date(world.lastBackup).toLocaleString() }) : t('worlds.neverBackedUp');
}

function gameModeLabel(world) {
  if (!world.gameMode) return null;
  const key = world.hardcore && world.gameMode === 'survival' ? 'hardcore' : world.gameMode;
  return t(`worlds.gameMode.${key}`);
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
      <p class="view-intro" style="margin: 0">{{ t('worlds.intro') }}</p>
      <button class="btn btn-ghost btn-sm" type="button" @click="openSavesFolder">{{ t('worlds.openFolder') }}</button>
    </div>

    <div v-if="error" class="error-box">{{ error }}</div>
    <div v-if="!loading && worlds.length === 0" class="empty-state">{{ t('worlds.empty') }}</div>

    <div class="world-list">
      <div v-for="world in worlds" :key="world.folder" class="world-row" @contextmenu="onContextMenu($event, world)">
        <div class="world-row-icon" :class="{ [`biome-${biome}-bg`]: !world.iconDataUrl }">
          <img v-if="world.iconDataUrl" :src="world.iconDataUrl" alt="" />
          <span v-else>{{ firstGrapheme(world.name).toUpperCase() }}</span>
        </div>

        <div class="world-row-info">
          <div class="world-row-title">
            <h4>{{ world.name }}</h4>
            <span
              v-if="gameModeLabel(world)"
              class="gamemode-badge"
              :class="`gm-${world.hardcore && world.gameMode === 'survival' ? 'hardcore' : world.gameMode}`"
            >
              {{ gameModeLabel(world) }}
            </span>
            <span
              class="backup-badge"
              :class="world.lastBackup ? 'backup-yes' : 'backup-no'"
              v-tooltip="backupLabel(world)"
            >
              {{ world.lastBackup ? t('worlds.backedUpFlag') : t('worlds.noBackupFlag') }}
            </span>
          </div>
          <p class="world-row-meta">{{ formatSize(world.sizeBytes) }} · {{ lastPlayedLabel(world) }}</p>
        </div>

        <div class="world-row-actions" v-if="!confirmingDeleteWorld[world.folder] && !confirmingRestore[world.folder]">
          <button
            class="icon-btn"
            type="button"
            :disabled="backingUp[world.folder]"
            v-tooltip="backingUp[world.folder] ? t('worlds.backingUp') : t('worlds.backupNow')"
            @click="backupNow(world)"
          >
            <span v-if="backingUp[world.folder]" class="spinner"></span>
            <svg v-else viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <path d="M12 3v12m0-12 4 4m-4-4-4 4M4 17v2a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2v-2" />
            </svg>
          </button>
          <button
            v-if="world.lastBackup"
            class="icon-btn"
            type="button"
            v-tooltip="t('worlds.restore')"
            @click="confirmingRestore[world.folder] = true"
          >
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M3 12a9 9 0 1 0 3-6.7M3 4v5h5" /></svg>
          </button>
          <button
            v-if="world.lastBackup"
            class="icon-btn"
            type="button"
            v-tooltip="t('worlds.history')"
            @click="openHistory(world)"
          >
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><circle cx="12" cy="12" r="9" /><path d="M12 7v5l3 3" /></svg>
          </button>
          <button class="icon-btn" type="button" v-tooltip="t('worlds.openWorldFolder')" @click="openWorldFolder(world)">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <path d="M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v7a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V7Z" />
            </svg>
          </button>
          <button
            class="icon-btn icon-btn-danger"
            type="button"
            v-tooltip="t('worlds.deleteWorld')"
            @click="confirmingDeleteWorld[world.folder] = true"
          >
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <path d="M3 6h18M8 6V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2m3 0-1 14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2L4 6h16Z" />
            </svg>
          </button>
        </div>

        <div class="world-row-actions" v-else-if="confirmingRestore[world.folder]">
          <button class="btn btn-ghost btn-sm" type="button" @click="confirmingRestore[world.folder] = false">
            {{ t('instances.cancel') }}
          </button>
          <button
            class="btn btn-danger-ghost btn-sm"
            type="button"
            :disabled="restoring[world.folder]"
            @click="restoreBackup(world)"
          >
            {{ restoring[world.folder] ? t('worlds.restoring') : t('worlds.confirmRestore') }}
          </button>
        </div>

        <div class="world-row-actions" v-else>
          <button
            class="btn btn-ghost btn-sm"
            type="button"
            @click="confirmingDeleteWorld[world.folder] = false"
          >
            {{ t('instances.cancel') }}
          </button>
          <button
            class="btn btn-danger-ghost btn-sm"
            type="button"
            :disabled="deletingWorld[world.folder]"
            @click="removeWorld(world)"
          >
            {{ deletingWorld[world.folder] ? t('worlds.deleting') : t('worlds.confirmDeleteWorld') }}
          </button>
        </div>
      </div>
    </div>

    <Transition name="modal">
      <BackupHistoryModal
        v-if="historyWorld"
        :instance-id="instanceId"
        :folder="historyWorld.folder"
        :world-name="historyWorld.name"
        @close="historyWorld = null"
        @changed="refresh"
      />
    </Transition>
  </section>
</template>
