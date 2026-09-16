<script setup>
import { ref, computed, onMounted, onBeforeUnmount } from 'vue';
import { useI18n } from 'vue-i18n';
import { open } from '@tauri-apps/plugin-dialog';
import { listen } from '@tauri-apps/api/event';
import { useLocale } from '../composables/useLocale';
import { useAccessibility } from '../composables/useAccessibility';
import { useDensity } from '../composables/useDensity';
import { useSidebarMode } from '../composables/useSidebarMode';
import { useLogUploadTarget, UPLOAD_TARGET_LABELS } from '../composables/useLogUploadTarget';
import { listUploadTargets } from '../api/logs';
import { useAccountsStore } from '../stores/accounts';
import GlassSelect from '../components/common/GlassSelect.vue';
import LoginModal from '../components/auth/LoginModal.vue';
import SkinHead from '../components/common/SkinHead.vue';
import {
  getAppSettings,
  updateAppSettings,
  getDataDirInfo,
  setPendingDataDir,
  resetDataDir,
  restartApp,
  getRecommendedMemoryMb,
} from '../api/appSettings';
import { checkForUpdate, installUpdate } from '../api/updater';
import { version as appVersion } from '../../package.json';

const { t } = useI18n();
const { locale, locales } = useLocale();
const { reducedMotion, uiScale } = useAccessibility();
const { density } = useDensity();
const { persistent: sidebarPersistent } = useSidebarMode();
const { uploadTarget: logUploadTarget } = useLogUploadTarget();
const availableUploadTargets = ref(['mclogs']);
const logUploadTargetOptions = computed(() =>
  availableUploadTargets.value.map((target) => ({ value: target, label: UPLOAD_TARGET_LABELS[target] ?? target })),
);
const accounts = useAccountsStore();

const LANGUAGE_NAMES = { en: 'English', es: 'Español', fr: 'Français', pl: 'Polski' };
const languageOptions = computed(() => locales.map((code) => ({ value: code, label: LANGUAGE_NAMES[code] ?? code })));
const reducedMotionOptions = computed(() => [
  { value: 'system', label: t('settings.reducedMotionSystem') },
  { value: 'on', label: t('settings.reducedMotionOn') },
  { value: 'off', label: t('settings.reducedMotionOff') },
]);
const densityOptions = computed(() => [
  { value: 'comfortable', label: t('settings.densityComfortable') },
  { value: 'compact', label: t('settings.densityCompact') },
]);

const active = ref('language');

// ---- Global app settings (window, memory, Java default, JVM args, commands, behavior, updates) ----
const windowWidth = ref(925);
const windowHeight = ref(530);
const windowMaximized = ref(false);
const recommendedMemoryMb = ref(2048);
const defaultMemoryMb = ref(2048);
const defaultMinMemoryMb = ref(1024);
const jvmArgs = ref('');
const skipJavaCheckDefault = ref(false);
const envVars = ref('');
const preLaunchCmd = ref('');
const wrapperCmd = ref('');
const postExitCmd = ref('');
const launcherBehavior = ref('keep_open');
const autoCheckUpdates = ref(true);
const updateCheckIntervalHours = ref(24);

const original = ref(null);
const saving = ref(false);
const error = ref(null);

const launcherBehaviorOptions = computed(() => [
  { value: 'keep_open', label: t('settings.launcherKeepOpen') },
  { value: 'close_on_launch', label: t('settings.launcherCloseOnLaunch') },
  { value: 'hide_to_tray', label: t('settings.launcherHideToTray') },
  { value: 'quit_on_exit', label: t('settings.launcherQuitOnExit') },
]);
const updateIntervalOptions = computed(() => [
  { value: 1, label: t('settings.updateIntervalHourly') },
  { value: 6, label: t('settings.updateIntervalEvery', { hours: 6 }) },
  { value: 24, label: t('settings.updateIntervalDaily') },
  { value: 72, label: t('settings.updateIntervalEvery', { hours: 72 }) },
  { value: 168, label: t('settings.updateIntervalWeekly') },
]);

const defaultMemoryLabel = computed(() => `${(defaultMemoryMb.value / 1024).toFixed(1)} GB`);
const defaultMinMemoryLabel = computed(() => `${(defaultMinMemoryMb.value / 1024).toFixed(1)} GB`);
const memoryTooLow = computed(() => defaultMinMemoryMb.value > defaultMemoryMb.value);

function applyFromSettings(s) {
  windowWidth.value = s.windowWidth ?? 925;
  windowHeight.value = s.windowHeight ?? 530;
  windowMaximized.value = !!s.windowMaximized;
  defaultMemoryMb.value = s.defaultMemoryMb ?? recommendedMemoryMb.value;
  defaultMinMemoryMb.value = s.defaultMinMemoryMb ?? recommendedMemoryMb.value;
  jvmArgs.value = s.jvmArgs ?? '';
  skipJavaCheckDefault.value = !!s.skipJavaCheckDefault;
  envVars.value = s.envVars ?? '';
  preLaunchCmd.value = s.preLaunchCmd ?? '';
  wrapperCmd.value = s.wrapperCmd ?? '';
  postExitCmd.value = s.postExitCmd ?? '';
  launcherBehavior.value = s.launcherBehavior ?? 'keep_open';
  autoCheckUpdates.value = !!s.autoCheckUpdates;
  updateCheckIntervalHours.value = s.updateCheckIntervalHours ?? 24;
}

async function loadSettings() {
  const s = await getAppSettings();
  original.value = s;
  applyFromSettings(s);
}

// Same dirty-tracking + floating save-bar pattern as Instance Settings and
// Skin & Cape; nothing here writes until Save is pressed.
const hasPendingChanges = computed(() => {
  const o = original.value;
  if (!o) return false;
  return (
    windowWidth.value !== (o.windowWidth ?? 925) ||
    windowHeight.value !== (o.windowHeight ?? 530) ||
    windowMaximized.value !== !!o.windowMaximized ||
    defaultMemoryMb.value !== (o.defaultMemoryMb ?? recommendedMemoryMb.value) ||
    defaultMinMemoryMb.value !== (o.defaultMinMemoryMb ?? recommendedMemoryMb.value) ||
    jvmArgs.value.trim() !== (o.jvmArgs ?? '') ||
    skipJavaCheckDefault.value !== !!o.skipJavaCheckDefault ||
    envVars.value.trim() !== (o.envVars ?? '') ||
    preLaunchCmd.value.trim() !== (o.preLaunchCmd ?? '') ||
    wrapperCmd.value.trim() !== (o.wrapperCmd ?? '') ||
    postExitCmd.value.trim() !== (o.postExitCmd ?? '') ||
    launcherBehavior.value !== (o.launcherBehavior ?? 'keep_open') ||
    autoCheckUpdates.value !== !!o.autoCheckUpdates ||
    updateCheckIntervalHours.value !== (o.updateCheckIntervalHours ?? 24)
  );
});

function discardChanges() {
  if (original.value) applyFromSettings(original.value);
}

async function save() {
  if (!hasPendingChanges.value || memoryTooLow.value) return;
  saving.value = true;
  error.value = null;
  try {
    const updated = await updateAppSettings({
      windowWidth: windowWidth.value,
      windowHeight: windowHeight.value,
      windowMaximized: windowMaximized.value,
      defaultMemoryMb: defaultMemoryMb.value,
      defaultMinMemoryMb: defaultMinMemoryMb.value,
      jvmArgs: jvmArgs.value.trim() || null,
      skipJavaCheckDefault: skipJavaCheckDefault.value,
      envVars: envVars.value.trim() || null,
      preLaunchCmd: preLaunchCmd.value.trim() || null,
      wrapperCmd: wrapperCmd.value.trim() || null,
      postExitCmd: postExitCmd.value.trim() || null,
      launcherBehavior: launcherBehavior.value,
      autoCheckUpdates: autoCheckUpdates.value,
      updateCheckIntervalHours: updateCheckIntervalHours.value,
    });
    original.value = updated;
    applyFromSettings(updated);
  } catch (e) {
    error.value = String(e);
  } finally {
    saving.value = false;
  }
}

// ---- Storage ----
const dataDirInfo = ref(null);
const choosingDir = ref(false);
const dirActionError = ref(null);
const restartingForDataDir = ref(false);

async function loadDataDirInfo() {
  dataDirInfo.value = await getDataDirInfo();
}

// Data lives at the resolved path for the rest of this run regardless of the
// change just queued, so a live restart is the only way to actually move to it.
async function restartForDataDirChange() {
  restartingForDataDir.value = true;
  await new Promise((resolve) => setTimeout(resolve, 1500));
  await restartApp();
}

async function chooseDataDir() {
  dirActionError.value = null;
  const folder = await open({ directory: true });
  if (!folder || typeof folder !== 'string') return;
  choosingDir.value = true;
  try {
    await setPendingDataDir(folder);
    await restartForDataDirChange();
  } catch (e) {
    dirActionError.value = String(e);
    choosingDir.value = false;
  }
}

async function resetDataDirToDefault() {
  dirActionError.value = null;
  choosingDir.value = true;
  try {
    await resetDataDir();
    await restartForDataDirChange();
  } catch (e) {
    dirActionError.value = String(e);
    choosingDir.value = false;
  }
}

// ---- Accounts ----
const loginOpen = ref(false);
function removeAccount(id, event) {
  event.stopPropagation();
  accounts.remove(id);
}

// ---- Self-update (About panel) ----
const updateState = ref('idle'); // idle | checking | upToDate | available | error
const updateInfo = ref(null);
const installing = ref(false);
const installError = ref(null);
const installProgress = ref(null);
let unlistenUpdateProgress = null;

const installProgressPercent = computed(() => {
  const p = installProgress.value;
  if (!p || !p.total) return 0;
  return Math.round((p.completed / p.total) * 100);
});

async function checkUpdate() {
  updateState.value = 'checking';
  try {
    const info = await checkForUpdate();
    updateInfo.value = info;
    updateState.value = info ? 'available' : 'upToDate';
  } catch {
    updateState.value = 'error';
  }
}

async function doInstallUpdate() {
  if (!updateInfo.value) return;
  installing.value = true;
  installError.value = null;
  installProgress.value = null;
  try {
    await installUpdate(updateInfo.value);
  } catch (e) {
    installError.value = String(e);
  } finally {
    installing.value = false;
  }
}

onMounted(async () => {
  recommendedMemoryMb.value = await getRecommendedMemoryMb();
  loadSettings();
  loadDataDirInfo();
  accounts.refresh();
  checkUpdate();
  availableUploadTargets.value = await listUploadTargets();
  unlistenUpdateProgress = await listen('download://progress', (event) => {
    const p = event.payload;
    if (p.stage !== 'update') return;
    installProgress.value = p;
  });
});

onBeforeUnmount(() => {
  unlistenUpdateProgress?.();
});
</script>

<template>
  <section class="view">
    <div class="settings-layout">
      <nav class="settings-nav">
        <button type="button" class="nav-item" :class="{ active: active === 'language' }" @click="active = 'language'">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8">
            <circle cx="12" cy="12" r="9" />
            <path d="M3 12h18M12 3a14 14 0 0 1 0 18 14 14 0 0 1 0-18z" />
          </svg>
          <span>{{ t('settings.languageAccessibilityHeading') }}</span>
        </button>
        <button type="button" class="nav-item" :class="{ active: active === 'storage' }" @click="active = 'storage'">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8">
            <path d="M4 7a2 2 0 0 1 2-2h4l2 2h6a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2z" />
          </svg>
          <span>{{ t('settings.storageHeading') }}</span>
        </button>
        <button type="button" class="nav-item" :class="{ active: active === 'updates' }" @click="active = 'updates'">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8">
            <path d="M21 12a9 9 0 1 1-3-6.7" />
            <polyline points="21 3 21 9 15 9" />
          </svg>
          <span>{{ t('settings.updatesHeading') }}</span>
        </button>

        <div class="nav-group-label">{{ t('settings.minecraftGroupLabel') }}</div>
        <button type="button" class="nav-item" :class="{ active: active === 'window' }" @click="active = 'window'">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8">
            <rect x="3" y="4" width="18" height="16" rx="2" />
            <path d="M3 8h18" />
          </svg>
          <span>{{ t('instances.sectionWindow') }}</span>
        </button>
        <button type="button" class="nav-item" :class="{ active: active === 'memory' }" @click="active = 'memory'">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8">
            <rect x="3" y="7" width="18" height="10" rx="1.5" />
            <path d="M7 7v4M11 7v4M15 7v4M19 7v4" />
          </svg>
          <span>{{ t('instances.sectionMemory') }}</span>
        </button>
        <button type="button" class="nav-item" :class="{ active: active === 'java' }" @click="active = 'java'">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8">
            <path d="M9 3v3a3 3 0 0 0 6 0V3" />
            <path d="M6 10h12l-1 8a2 2 0 0 1-2 2H9a2 2 0 0 1-2-2z" />
          </svg>
          <span>{{ t('instances.sectionJava') }}</span>
        </button>
        <button type="button" class="nav-item" :class="{ active: active === 'jvm' }" @click="active = 'jvm'">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8">
            <polyline points="8 9 4 12 8 15" />
            <polyline points="16 9 20 12 16 15" />
            <line x1="13" y1="6" x2="11" y2="18" />
          </svg>
          <span>{{ t('instances.sectionJvm') }}</span>
        </button>
        <button type="button" class="nav-item" :class="{ active: active === 'commands' }" @click="active = 'commands'">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8">
            <polyline points="4 17 10 11 4 5" />
            <line x1="12" y1="19" x2="20" y2="19" />
          </svg>
          <span>{{ t('settings.commandsHeading') }}</span>
        </button>
        <button type="button" class="nav-item" :class="{ active: active === 'behavior' }" @click="active = 'behavior'">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8">
            <polygon points="6 3 20 12 6 21 6 3" />
          </svg>
          <span>{{ t('settings.behaviorHeading') }}</span>
        </button>

        <div class="nav-group-label">{{ t('settings.miscGroupLabel') }}</div>
        <button type="button" class="nav-item" :class="{ active: active === 'accounts' }" @click="active = 'accounts'">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8">
            <circle cx="12" cy="8" r="3.4" />
            <path d="M5 20c1.2-4 4.2-6 7-6s5.8 2 7 6" />
          </svg>
          <span>{{ t('settings.accountsHeading') }}</span>
        </button>
        <button type="button" class="nav-item" :class="{ active: active === 'about' }" @click="active = 'about'">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8">
            <circle cx="12" cy="12" r="9" />
            <line x1="12" y1="11" x2="12" y2="16" />
            <circle cx="12" cy="7.5" r="0.9" fill="currentColor" stroke="none" />
          </svg>
          <span>{{ t('settings.aboutHeading') }}</span>
        </button>
      </nav>

      <div class="settings-panel">
        <div v-if="active === 'language'" class="field-group">
          <h4>{{ t('settings.languageAccessibilityHeading') }}</h4>

          <label class="field-label settings-field-label" for="settings-language">{{ t('settings.languageLabel') }}</label>
          <GlassSelect id="settings-language" v-model="locale" :options="languageOptions" style="width: 220px" />

          <label class="field-label settings-field-label" for="settings-reduced-motion">{{ t('settings.reducedMotionLabel') }}</label>
          <GlassSelect id="settings-reduced-motion" v-model="reducedMotion" :options="reducedMotionOptions" style="width: 220px" />
          <p class="hint">{{ t('settings.reducedMotionHint') }}</p>

          <label class="field-label settings-field-label" for="settings-ui-scale">{{ t('settings.uiScaleLabel') }}</label>
          <div class="slider-row">
            <input type="range" min="80" max="150" step="5" v-model.number="uiScale" />
            <div class="slider-num-wrap">
              <input id="settings-ui-scale" type="number" class="slider-num" min="50" max="200" step="1" v-model.number="uiScale" />
              <span class="slider-num-suffix">%</span>
            </div>
          </div>

          <label class="field-label settings-field-label" for="settings-density">{{ t('settings.densityLabel') }}</label>
          <GlassSelect id="settings-density" v-model="density" :options="densityOptions" style="width: 220px" />
          <p class="hint">{{ t('settings.densityHint') }}</p>

          <label class="checkbox-row" style="margin-top: 20px">
            <input type="checkbox" v-model="sidebarPersistent" />
            {{ t('settings.sidebarPersistentLabel') }}
          </label>
          <p class="hint">{{ t('settings.sidebarPersistentHint') }}</p>
        </div>

        <div v-else-if="active === 'storage'" class="field-group">
          <h4>{{ t('settings.storageHeading') }}</h4>
          <p class="hint">{{ t('settings.storageHint') }}</p>

          <label class="field-label settings-field-label">{{ t('settings.storageCurrentLabel') }}</label>
          <div class="storage-path-row">
            <span class="mono storage-path">{{ dataDirInfo?.path }}</span>
            <span class="tag" :class="{ 'tag-accent': dataDirInfo?.isCustom }">
              {{ dataDirInfo?.isCustom ? t('settings.storageCustomTag') : t('settings.storageDefaultTag') }}
            </span>
          </div>

          <p v-if="dataDirInfo?.pendingPath" class="hint inherit-note">
            {{ t('settings.storagePendingLabel', { path: dataDirInfo.pendingPath }) }}
          </p>
          <div v-if="dirActionError" class="error-box" style="margin-top: 10px">{{ dirActionError }}</div>

          <p v-if="restartingForDataDir" class="hint inherit-note">
            <span class="spinner"></span>
            {{ t('settings.storageRestartingNotice') }}
          </p>
          <div v-else style="display: flex; gap: 10px; margin-top: 16px">
            <button class="btn btn-mineral" type="button" :disabled="choosingDir" @click="chooseDataDir">
              {{ t('settings.storageChange') }}
            </button>
            <button
              v-if="dataDirInfo?.isCustom || dataDirInfo?.pendingPath"
              class="btn btn-ghost"
              type="button"
              :disabled="choosingDir"
              @click="resetDataDirToDefault"
            >
              {{ t('settings.storageReset') }}
            </button>
          </div>
          <p v-if="!restartingForDataDir" class="hint" style="margin-top: 14px">{{ t('settings.storageRestartHint') }}</p>
        </div>

        <div v-else-if="active === 'updates'" class="field-group">
          <h4>{{ t('settings.updatesHeading') }}</h4>
          <p class="hint">{{ t('settings.updatesHint') }}</p>

          <label class="checkbox-row">
            <input type="checkbox" v-model="autoCheckUpdates" />
            {{ t('settings.autoCheckUpdates') }}
          </label>

          <template v-if="autoCheckUpdates">
            <label class="field-label settings-field-label" for="settings-update-interval">{{ t('settings.updateIntervalLabel') }}</label>
            <GlassSelect
              id="settings-update-interval"
              v-model="updateCheckIntervalHours"
              :options="updateIntervalOptions"
              style="width: 220px"
            />
          </template>

          <div class="update-block" style="margin-top: 20px">
            <button v-if="updateState === 'idle'" class="btn btn-ghost" type="button" @click="checkUpdate">
              {{ t('settings.updateCheck') }}
            </button>

            <div v-else-if="updateState === 'checking'" class="hint update-check-row">
              <span class="spinner"></span>
              {{ t('settings.updateChecking') }}
            </div>

            <p v-else-if="updateState === 'upToDate'" class="hint">{{ t('settings.updateUpToDate') }}</p>

            <p v-else-if="updateState === 'error'" class="hint">{{ t('settings.updateError') }}</p>

            <div v-else-if="updateState === 'available'">
              <p class="hint">{{ t('settings.updateAvailable', { version: updateInfo?.version }) }}</p>
              <button class="btn btn-mineral" type="button" :disabled="installing" @click="doInstallUpdate">
                <span v-if="installing" class="spinner"></span>
                {{ installing ? t('settings.updateInstalling') : t('settings.updateInstallButton') }}
              </button>
              <p v-if="installing && installProgress" class="hint">{{ installProgressPercent }}%</p>
              <div v-if="installError" class="error-box" style="margin-top: 10px">{{ installError }}</div>
            </div>
          </div>
        </div>

        <div v-else-if="active === 'window'" class="field-group">
          <h4>{{ t('instances.sectionWindow') }}</h4>
          <p class="hint">{{ t('settings.globalWindowHint') }}</p>

          <label class="checkbox-row">
            <input type="checkbox" v-model="windowMaximized" />
            {{ t('instances.windowMaximized') }}
          </label>

          <template v-if="!windowMaximized">
            <div class="dim-row" style="margin-top: 14px">
              <div>
                <label class="field-label" for="settings-window-width">{{ t('instances.windowWidthLabel') }}</label>
                <input id="settings-window-width" type="number" min="200" v-model.number="windowWidth" class="field mono" />
              </div>
              <div>
                <label class="field-label" for="settings-window-height">{{ t('instances.windowHeightLabel') }}</label>
                <input id="settings-window-height" type="number" min="200" v-model.number="windowHeight" class="field mono" />
              </div>
            </div>
          </template>
        </div>

        <div v-else-if="active === 'memory'" class="field-group">
          <h4>{{ t('instances.sectionMemory') }}</h4>
          <p class="hint">{{ t('settings.globalMemoryHint') }}</p>

          <label class="field-label settings-field-label" for="settings-memory-max">{{ t('instances.maxMemoryLabel') }}</label>
          <div class="slider-row">
            <input type="range" min="512" max="16384" step="256" v-model.number="defaultMemoryMb" />
            <div class="slider-num-wrap">
              <input id="settings-memory-max" type="number" class="slider-num" min="512" max="65536" step="1" v-model.number="defaultMemoryMb" />
              <span class="slider-num-suffix">MB</span>
            </div>
          </div>
          <p class="hint">{{ t('instances.memoryGbEquivalent', { gb: defaultMemoryLabel }) }}</p>

          <label class="field-label settings-field-label" for="settings-memory-min">{{ t('instances.minMemoryLabel') }}</label>
          <div class="slider-row">
            <input type="range" min="512" max="16384" step="256" v-model.number="defaultMinMemoryMb" />
            <div class="slider-num-wrap">
              <input id="settings-memory-min" type="number" class="slider-num" min="512" max="65536" step="1" v-model.number="defaultMinMemoryMb" />
              <span class="slider-num-suffix">MB</span>
            </div>
          </div>
          <p class="hint">{{ t('instances.minMemoryHint') }}</p>
          <p v-if="memoryTooLow" class="java-warning">{{ t('instances.memoryTooLow') }}</p>
        </div>

        <div v-else-if="active === 'java'" class="field-group">
          <h4>{{ t('instances.sectionJava') }}</h4>

          <label class="checkbox-row">
            <input type="checkbox" v-model="skipJavaCheckDefault" />
            {{ t('settings.skipJavaCheckDefault') }}
          </label>
          <p class="java-warning">{{ t('settings.skipJavaCheckDefaultHint') }}</p>
        </div>

        <div v-else-if="active === 'jvm'" class="field-group">
          <h4>{{ t('instances.sectionJvm') }}</h4>
          <label class="field-label" for="settings-jvm-args">{{ t('instances.jvmArgsLabel') }}</label>
          <textarea
            id="settings-jvm-args"
            v-model="jvmArgs"
            class="textarea-field"
            :placeholder="t('instances.jvmArgsPlaceholder')"
          ></textarea>
          <p class="hint">{{ t('settings.globalJvmArgsHint') }}</p>
        </div>

        <div v-else-if="active === 'commands'" class="field-group">
          <h4>{{ t('settings.commandsHeading') }}</h4>
          <p class="hint">{{ t('settings.globalCommandsHint') }}</p>

          <label class="field-label settings-field-label" for="settings-env">{{ t('instances.envVarsLabel') }}</label>
          <textarea
            id="settings-env"
            v-model="envVars"
            class="textarea-field mono"
            :placeholder="t('instances.envVarsPlaceholder')"
          ></textarea>

          <label class="field-label settings-field-label" for="settings-pre-launch">{{ t('instances.preLaunchLabel') }}</label>
          <input id="settings-pre-launch" v-model="preLaunchCmd" class="field mono" :placeholder="t('instances.preLaunchPlaceholder')" />

          <label class="field-label settings-field-label" for="settings-wrapper">{{ t('instances.wrapperLabel') }}</label>
          <input id="settings-wrapper" v-model="wrapperCmd" class="field mono" :placeholder="t('instances.wrapperPlaceholder')" />
          <p class="hint">{{ t('instances.wrapperHint') }}</p>

          <label class="field-label settings-field-label" for="settings-post-exit">{{ t('instances.postExitLabel') }}</label>
          <input id="settings-post-exit" v-model="postExitCmd" class="field mono" :placeholder="t('instances.preLaunchPlaceholder')" />
        </div>

        <div v-else-if="active === 'behavior'" class="field-group">
          <h4>{{ t('settings.behaviorHeading') }}</h4>

          <label class="field-label" for="settings-launcher-behavior">{{ t('settings.launcherBehaviorLabel') }}</label>
          <GlassSelect id="settings-launcher-behavior" v-model="launcherBehavior" :options="launcherBehaviorOptions" style="width: 280px" />
          <p v-if="launcherBehavior === 'close_on_launch'" class="hint">{{ t('settings.launcherCloseOnLaunchHint') }}</p>
          <p v-else-if="launcherBehavior === 'hide_to_tray'" class="hint">{{ t('settings.launcherHideToTrayHint') }}</p>

          <label class="field-label settings-field-label" for="settings-log-upload-target">{{ t('settings.logUploadTargetLabel') }}</label>
          <GlassSelect
            id="settings-log-upload-target"
            v-model="logUploadTarget"
            :options="logUploadTargetOptions"
            style="width: 220px"
          />
          <p class="hint">{{ t('settings.logUploadTargetHint') }}</p>
        </div>

        <div v-else-if="active === 'accounts'" class="field-group">
          <h4>{{ t('settings.accountsHeading') }}</h4>

          <div v-if="accounts.accounts.length === 0" class="empty-state" style="margin-top: 4px">
            {{ t('topbar.signIn') }}
          </div>

          <div
            v-for="acc in accounts.accounts"
            :key="acc.id"
            class="dropdown-item"
            role="button"
            tabindex="0"
            style="align-items: center; cursor: pointer; margin-top: 8px"
            @click="accounts.setActive(acc.id)"
            @keyup.enter="accounts.setActive(acc.id)"
          >
            <div class="avatar" :class="{ offline: acc.kind === 'offline' }" style="width: 36px; height: 36px; flex: 0 0 36px">
              <SkinHead :skin-url="acc.skinUrl" :size="36" />
            </div>
            <div style="flex: 1; min-width: 0">
              <p>
                {{ acc.username }}
                <span v-if="acc.isActive" class="active-tag">{{ t('topbar.active') }}</span>
              </p>
              <small>{{ acc.kind === 'microsoft' ? t('topbar.microsoftAccount') : t('topbar.offlineProfile') }}</small>
            </div>
            <button class="btn btn-danger-ghost btn-sm" type="button" @click="removeAccount(acc.id, $event)">
              {{ t('topbar.remove') }}
            </button>
          </div>

          <button class="btn btn-ghost" type="button" style="margin-top: 16px" @click="loginOpen = true">
            {{ t('topbar.addAccount') }}
          </button>
        </div>

        <div v-else-if="active === 'about'" class="field-group about-panel">
          <h4 class="about-title">{{ t('app.name') }}</h4>
          <img src="/branding/about-artwork.webp" alt="" class="about-artwork" />
          <div class="about-meta">
            <span>{{ t('settings.versionLabel', { version: appVersion }) }}</span>
            <span class="about-sep" aria-hidden="true">•</span>
            <span>{{ t('settings.authorLabel', { author: 'Nihalantiir' }) }}</span>
          </div>

          <div class="about-disclosure">
            <p>{{ t('settings.disclosureLine1') }}</p>
            <p>{{ t('settings.disclosureLine2') }}</p>
          </div>
        </div>

        <Transition name="fade-slide">
          <div v-if="hasPendingChanges" class="save-bar">
            <span class="save-msg">{{ t('settings.unsavedChanges') }}</span>
            <div class="save-bar-actions">
              <button class="btn btn-ghost" type="button" :disabled="saving" @click="discardChanges">
                {{ t('settings.discardChanges') }}
              </button>
              <button class="btn btn-mineral" type="button" :disabled="saving || memoryTooLow" @click="save">
                <span v-if="saving" class="spinner"></span>
                {{ saving ? t('settings.saving') : t('settings.saveChanges') }}
              </button>
            </div>
          </div>
        </Transition>
        <div v-if="error" class="error-box" style="margin-top: 14px">{{ error }}</div>
      </div>
    </div>

    <Transition name="modal">
      <LoginModal v-if="loginOpen" @close="loginOpen = false" />
    </Transition>
  </section>
</template>

<style scoped>
.storage-path-row {
  display: flex;
  align-items: center;
  gap: 10px;
  flex-wrap: wrap;
}
.storage-path {
  font-size: 13px;
  color: var(--text);
  word-break: break-all;
}
.active-tag {
  color: var(--mineral);
  font-weight: 500;
  font-size: 12px;
  margin-left: 6px;
}
.update-block {
  margin-bottom: 24px;
}
.update-check-row {
  display: flex;
  align-items: center;
  gap: 8px;
}
</style>
