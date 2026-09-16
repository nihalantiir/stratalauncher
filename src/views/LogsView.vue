<script setup>
import { ref, computed, nextTick, onMounted, onActivated, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import { openUrl } from '@tauri-apps/plugin-opener';
import { openPath } from '../api/opener';
import { useInstancesStore } from '../stores/instances';
import { useLogUploadTarget, UPLOAD_TARGET_LABELS } from '../composables/useLogUploadTarget';
import * as api from '../api/logs';
import GlassSelect from '../components/common/GlassSelect.vue';

const { t } = useI18n();
const instances = useInstancesStore();
const { uploadTarget } = useLogUploadTarget();

const tab = ref('minecraft');
const logFiles = ref([]);
const logFileOptions = computed(() => logFiles.value.map((f) => ({ value: f.filename, label: f.label })));
const selectedFile = ref(null);
const lines = ref([]);
const loading = ref(false);
const error = ref(null);
const search = ref('');
const levels = ref({ info: true, warn: true, error: true, debug: false });
const copyState = ref('idle');
const linkCopyState = ref('idle');
const uploading = ref(false);
const uploadedUrl = ref(null);
const insights = ref(null);
const panelEl = ref(null);

const instanceId = computed(() => instances.current?.id);

const consoleTitle = computed(() => {
  if (tab.value === 'launcher') return t('logs.tabLauncher');
  return logFiles.value.find((f) => f.filename === selectedFile.value)?.label ?? t('logs.tabMinecraft');
});

function bucket(level) {
  const l = (level || '').toUpperCase();
  if (l === 'WARN' || l === 'WARNING') return 'warn';
  if (l === 'ERROR' || l === 'FATAL') return 'error';
  if (l === 'DEBUG' || l === 'TRACE') return 'debug';
  return 'info';
}

function escapeHtml(text) {
  return text.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
}

function highlightedMessage(message) {
  const q = search.value.trim();
  if (!q) return escapeHtml(message);
  const idx = message.toLowerCase().indexOf(q.toLowerCase());
  if (idx === -1) return escapeHtml(message);
  const before = escapeHtml(message.slice(0, idx));
  const match = escapeHtml(message.slice(idx, idx + q.length));
  const after = escapeHtml(message.slice(idx + q.length));
  return `${before}<mark class="log-highlight">${match}</mark>${after}`;
}

const filteredLines = computed(() => {
  const q = search.value.trim().toLowerCase();
  return lines.value.filter((line) => {
    if (!levels.value[bucket(line.level)]) return false;
    if (!q) return true;
    return (
      line.message.toLowerCase().includes(q) ||
      (line.logger ?? '').toLowerCase().includes(q) ||
      (line.thread ?? '').toLowerCase().includes(q)
    );
  });
});

async function refreshFileList() {
  if (!instanceId.value) {
    logFiles.value = [];
    selectedFile.value = null;
    return;
  }
  logFiles.value = await api.listLogFiles(instanceId.value);
  if (!logFiles.value.some((f) => f.filename === selectedFile.value)) {
    selectedFile.value = logFiles.value[0]?.filename ?? null;
  }
}

async function refreshLines() {
  loading.value = true;
  error.value = null;
  uploadedUrl.value = null;
  insights.value = null;
  try {
    lines.value = tab.value === 'minecraft'
      ? instanceId.value && selectedFile.value
        ? await api.readMinecraftLog(instanceId.value, selectedFile.value)
        : []
      : await api.readLauncherLog();
  } catch (e) {
    error.value = String(e);
  } finally {
    loading.value = false;
  }
}

async function switchTab(next) {
  tab.value = next;
  if (next === 'minecraft') await refreshFileList();
  await refreshLines();
}

async function refreshAll() {
  await refreshFileList();
  await refreshLines();
}

onMounted(refreshAll);
watch(instanceId, refreshAll);
watch(selectedFile, refreshLines);
// A freshly loaded log's most relevant content (a crash, the latest
// activity) is almost always at the end, not the start.
watch(lines, async () => {
  await nextTick();
  if (panelEl.value) panelEl.value.scrollTop = panelEl.value.scrollHeight;
});
// Kept alive across navigation (see App.vue); a play session between visits
// changes the log file's content without changing instanceId or selectedFile, so re-read on every return visit instead of relying on the watchers above.
onActivated(refreshAll);

async function currentRawText() {
  if (tab.value === 'minecraft') {
    if (!instanceId.value || !selectedFile.value) return '';
    return api.readMinecraftLogRaw(instanceId.value, selectedFile.value);
  }
  return api.readLauncherLogRaw();
}

async function copyLog() {
  try {
    const text = await currentRawText();
    await navigator.clipboard.writeText(text);
    copyState.value = 'copied';
    setTimeout(() => (copyState.value = 'idle'), 1500);
  } catch (e) {
    error.value = String(e);
  }
}

async function copyUploadedLink() {
  if (!uploadedUrl.value) return;
  await navigator.clipboard.writeText(uploadedUrl.value);
  linkCopyState.value = 'copied';
  setTimeout(() => (linkCopyState.value = 'idle'), 1500);
}

async function uploadLog() {
  uploading.value = true;
  uploadedUrl.value = null;
  insights.value = null;
  error.value = null;
  try {
    const text = await currentRawText();
    uploadedUrl.value = await api.uploadLogText(text, uploadTarget.value);
    // mclo.gs's real advantage over a plain text host: it parses the log and
    // reports back detected problems; best-effort only, since a failed/empty analysis shouldn't make the upload itself look like it failed.
    if (uploadTarget.value === 'mclogs') {
      const id = uploadedUrl.value.split('/').pop();
      try {
        const result = await api.getMclogsInsights(id);
        if (result.problems.length > 0) insights.value = result;
      } catch {
        // ignore, since insights are a bonus, not the point of the upload
      }
    }
  } catch (e) {
    error.value = String(e);
  } finally {
    uploading.value = false;
  }
}

async function openFolder() {
  const dir = tab.value === 'minecraft' ? await api.getLogsDir(instanceId.value) : await api.getLauncherLogsDir();
  await openPath(dir);
}

function openUploaded() {
  if (uploadedUrl.value) openUrl(uploadedUrl.value);
}
</script>

<template>
  <section class="view">
    <div class="tab-row" style="max-width: 320px">
      <button class="tab-btn" :class="{ on: tab === 'minecraft' }" type="button" @click="switchTab('minecraft')">
        {{ t('logs.tabMinecraft') }}
      </button>
      <button class="tab-btn" :class="{ on: tab === 'launcher' }" type="button" @click="switchTab('launcher')">
        {{ t('logs.tabLauncher') }}
      </button>
    </div>

    <div class="log-toolbar">
      <GlassSelect
        v-if="tab === 'minecraft'"
        v-model="selectedFile"
        :options="logFileOptions"
        style="width: 220px"
      />
      <div v-else style="flex: 1"></div>

      <div style="display: flex; gap: 8px; align-items: center">
        <button class="btn btn-ghost btn-sm" type="button" @click="copyLog">
          {{ copyState === 'copied' ? t('logs.copied') : t('logs.copy') }}
        </button>
        <button
          class="btn btn-ghost btn-sm"
          type="button"
          :disabled="uploading"
          v-tooltip="t('logs.uploadTo', { target: UPLOAD_TARGET_LABELS[uploadTarget] ?? uploadTarget })"
          @click="uploadLog"
        >
          {{ uploading ? t('logs.uploading') : t('logs.upload') }}
        </button>
        <button class="btn btn-ghost btn-sm" type="button" @click="openFolder">{{ t('logs.openFolder') }}</button>
      </div>
    </div>

    <div
      v-if="uploadedUrl"
      class="error-box"
      style="background: rgba(79, 169, 122, 0.12); border-color: rgba(79, 169, 122, 0.35); color: var(--mineral); display: flex; align-items: center; justify-content: space-between; gap: 12px"
    >
      <span>
        {{ t('logs.uploadedLabel') }}
        <a href="#" style="color: var(--mineral); text-decoration: underline" @click.prevent="openUploaded">{{ uploadedUrl }}</a>
      </span>
      <button class="btn btn-ghost btn-sm" type="button" @click="copyUploadedLink">
        {{ linkCopyState === 'copied' ? t('logs.copied') : t('logs.copyLink') }}
      </button>
    </div>
    <div v-if="insights" class="log-insights">
      <h4>{{ t('logs.insightsHeading') }}</h4>
      <div v-for="(problem, i) in insights.problems" :key="i" class="log-insight-problem">
        <p>{{ problem.message }}</p>
        <ul v-if="problem.solutions.length">
          <li v-for="(solution, j) in problem.solutions" :key="j">{{ solution.message }}</li>
        </ul>
      </div>
    </div>
    <div v-if="error" class="error-box">{{ error }}</div>

    <div class="log-filters">
      <button
        v-for="key in ['info', 'warn', 'error', 'debug']"
        :key="key"
        class="log-level-chip"
        :class="[`level-${key}`, { on: levels[key] }]"
        type="button"
        @click="levels[key] = !levels[key]"
      >
        {{ t(`logs.level${key[0].toUpperCase()}${key.slice(1)}`) }}
      </button>
      <input v-model="search" class="field log-search" :placeholder="t('logs.searchPlaceholder')" />
    </div>

    <div class="log-console">
      <div class="log-console-head">
        <span class="log-console-title">{{ consoleTitle }}</span>
        <span class="log-console-count">{{ t('logs.lineCount', { shown: filteredLines.length, total: lines.length }) }}</span>
      </div>
      <div class="log-panel" ref="panelEl">
        <div v-if="!loading && filteredLines.length === 0" class="empty-state" style="border: none">{{ t('logs.empty') }}</div>
        <div v-for="(line, i) in filteredLines" :key="i" class="log-line">
          <span v-if="line.time" class="log-time">[{{ line.time }}]</span>
          <span class="log-level-tag" :class="`level-${bucket(line.level)}`">{{ line.level }}</span>
          <span v-if="line.thread || line.logger" class="log-source">{{ [line.thread, line.logger].filter(Boolean).join(' · ') }}</span>
          <span class="log-msg" v-html="highlightedMessage(line.message)"></span>
        </div>
      </div>
    </div>
  </section>
</template>
