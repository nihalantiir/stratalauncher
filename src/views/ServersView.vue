<script setup>
import { ref, computed, onMounted, onActivated, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import { useInstancesStore } from '../stores/instances';
import * as api from '../api/servers';
import { firstGrapheme } from '../lib/text';
import { useContextMenu } from '../composables/useContextMenu';

const { t } = useI18n();
const instances = useInstancesStore();
const { openContextMenu } = useContextMenu();

const servers = ref([]);
const statuses = ref({});
const loading = ref(false);
const error = ref(null);

const instanceId = computed(() => instances.current?.id);
const biome = computed(() => instances.current?.iconBiome ?? 'ore');

async function pingOne(index, address) {
  statuses.value = { ...statuses.value, [index]: { loading: true } };
  try {
    const status = await api.pingServer(address);
    statuses.value = { ...statuses.value, [index]: { loading: false, ...status } };
  } catch (e) {
    statuses.value = { ...statuses.value, [index]: { loading: false, online: false, error: String(e) } };
  }
}

async function refresh() {
  if (!instanceId.value) return;
  loading.value = true;
  error.value = null;
  try {
    servers.value = await api.listServers(instanceId.value);
    statuses.value = {};
    servers.value.forEach((server, i) => pingOne(i, server.address));
  } catch (e) {
    error.value = String(e);
  } finally {
    loading.value = false;
  }
}

onMounted(refresh);
watch(instanceId, refresh);
onActivated(refresh);

function rePing(index) {
  const server = servers.value[index];
  if (server) pingOne(index, server.address);
}

function serverIcon(server, index) {
  return server.iconDataUrl || statuses.value[index]?.faviconDataUrl || null;
}

function onContextMenu(event, server, index) {
  openContextMenu(event, [
    { label: t('servers.refresh'), icon: 'rotate', action: () => rePing(index) },
    { label: t('servers.copyAddress'), icon: 'copy', action: () => navigator.clipboard.writeText(server.address) },
  ]);
}

function statusLabel(index) {
  const s = statuses.value[index];
  if (!s || s.loading) return t('servers.pinging');
  return s.online ? t('servers.online') : t('servers.offline');
}

function statusClass(index) {
  const s = statuses.value[index];
  if (!s || s.loading) return 'server-pinging';
  return s.online ? 'server-online' : 'server-offline';
}

function playersLabel(index) {
  const s = statuses.value[index];
  if (!s?.online) return null;
  return t('servers.playersOnline', { online: s.playersOnline ?? '?', max: s.playersMax ?? '?' });
}

function metaLine(server, index) {
  const parts = [server.address];
  const s = statuses.value[index];
  if (s?.online) {
    if (s.pingMs != null) parts.push(t('servers.pingMs', { ms: s.pingMs }));
    const players = playersLabel(index);
    if (players) parts.push(players);
  }
  return parts.join(' · ');
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
      <p class="view-intro" style="margin: 0">{{ t('servers.intro') }}</p>
      <button class="btn btn-ghost btn-sm" type="button" @click="refresh">{{ t('servers.refreshAll') }}</button>
    </div>

    <div v-if="error" class="error-box">{{ error }}</div>
    <div v-if="!loading && servers.length === 0" class="empty-state">{{ t('servers.empty') }}</div>

    <div class="world-list">
      <div v-for="(server, i) in servers" :key="`${server.address}-${i}`" class="world-row" @contextmenu="onContextMenu($event, server, i)">
        <div class="world-row-icon" :class="{ [`biome-${biome}-bg`]: !serverIcon(server, i) }">
          <img v-if="serverIcon(server, i)" :src="serverIcon(server, i)" alt="" />
          <span v-else>{{ firstGrapheme(server.name || server.address).toUpperCase() }}</span>
        </div>

        <div class="world-row-info">
          <div class="world-row-title">
            <h4>{{ server.name || server.address }}</h4>
            <span class="server-badge" :class="statusClass(i)" v-tooltip="statuses[i]?.error">
              {{ statusLabel(i) }}
            </span>
            <span
              v-if="server.resourcePackAccepted !== null && server.resourcePackAccepted !== undefined"
              class="server-badge"
              :class="server.resourcePackAccepted ? 'rp-enabled' : 'rp-declined'"
            >
              {{ server.resourcePackAccepted ? t('servers.resourcePackEnabled') : t('servers.resourcePackDeclined') }}
            </span>
          </div>
          <p class="world-row-meta">{{ metaLine(server, i) }}</p>
        </div>

        <div class="world-row-actions">
          <button
            class="icon-btn"
            type="button"
            :disabled="statuses[i]?.loading"
            v-tooltip="t('servers.refresh')"
            @click="rePing(i)"
          >
            <span v-if="statuses[i]?.loading" class="spinner"></span>
            <svg v-else viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <path d="M3 12a9 9 0 1 0 3-6.7M3 4v5h5" />
            </svg>
          </button>
        </div>
      </div>
    </div>
  </section>
</template>
