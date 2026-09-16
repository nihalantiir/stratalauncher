<script setup>
import { ref, computed, watch, onMounted, onActivated } from 'vue';
import { useI18n } from 'vue-i18n';
import { useRouter } from 'vue-router';
import { open } from '@tauri-apps/plugin-dialog';
import { useInstancesStore } from '../stores/instances';
import { useVersionsStore } from '../stores/versions';
import { getRequiredJava, probeJavaAt, listJavaInstallations } from '../api/java';
import { listWorlds } from '../api/worlds';
import { getAppSettings, getRecommendedMemoryMb } from '../api/appSettings';
import { parseVersionKey, compareVersionKeys, resolveTargetVersion } from '../lib/versionMath';
import GlassSelect from '../components/common/GlassSelect.vue';

const { t } = useI18n();
const router = useRouter();
const instances = useInstancesStore();
const versions = useVersionsStore();

const inst = computed(() => instances.current);

const active = ref('general');

const name = ref('');
const groupName = ref('');

// Java
const javaPath = ref('');
const javaMode = ref('auto');
const skipJavaCheck = ref(false);
const detectedJava = ref([]);
const detectingJava = ref(false);

// Memory: `memoryOverride` decides whether this instance's own numbers are sent, or `null` (inherit the global
// default) instead. Without this flag, saving any unrelated field would silently hardcode whatever the sliders last displayed.
const memoryOverride = ref(false);
const memoryMb = ref(2048);
const minMemoryMb = ref(1024);

// JVM arguments
const jvmArgs = ref('');

// Window
const windowOverride = ref(false);
const windowWidth = ref(925);
const windowHeight = ref(530);
const windowMaximized = ref(false);

// On launch
const consoleMode = ref('never');
const quickPlayMode = ref('off');
const quickPlayTarget = ref('');

// Commands & environment
const envVars = ref('');
const preLaunchCmd = ref('');
const wrapperCmd = ref('');
const postExitCmd = ref('');

const error = ref(null);
const saving = ref(false);
const confirmingDelete = ref(false);

const requiredJava = ref(null);
const requiredJavaError = ref(null);
const javaProbe = ref(null);
let javaProbeHandle = null;

const worlds = ref([]);

// What "inherit" actually resolves to right now, for the read-only summary shown when an override is
// off, kept live so it's never a stale number.
const globalDefaults = ref({ defaultMemoryMb: null, defaultMinMemoryMb: null, windowWidth: null, windowHeight: null, windowMaximized: false });
const recommendedMemoryMb = ref(2048);
const resolvedMemoryMb = computed(() => globalDefaults.value.defaultMemoryMb ?? recommendedMemoryMb.value);
const resolvedMinMemoryMb = computed(() => globalDefaults.value.defaultMinMemoryMb ?? resolvedMemoryMb.value);
const resolvedWindowLabel = computed(() => {
  if (globalDefaults.value.windowMaximized) return t('instances.windowMaximized');
  return `${globalDefaults.value.windowWidth ?? 925} × ${globalDefaults.value.windowHeight ?? 530}`;
});

const memoryLabel = computed(() => `${(memoryMb.value / 1024).toFixed(1)} GB`);
const minMemoryLabel = computed(() => `${(minMemoryMb.value / 1024).toFixed(1)} GB`);
const memoryTooLow = computed(() => memoryOverride.value && minMemoryMb.value > memoryMb.value);

// Only shows the Save bar (and only actually writes) once something has really changed, same as the Skin
// & Cape page: don't ask for a deliberate Save click for nothing, but don't silently no-op either.
const hasPendingChanges = computed(() => {
  const i = inst.value;
  if (!i) return false;
  const hadMemoryOverride = i.memoryMb != null || i.minMemoryMb != null;
  const hadWindowOverride = i.windowWidth != null || i.windowHeight != null;
  return (
    name.value.trim() !== i.name ||
    groupName.value.trim() !== (i.groupName ?? '') ||
    javaMode.value !== (i.javaPath ? 'custom' : 'auto') ||
    (javaMode.value === 'custom' && javaPath.value.trim() !== (i.javaPath ?? '')) ||
    skipJavaCheck.value !== !!i.skipJavaCheck ||
    memoryOverride.value !== hadMemoryOverride ||
    (memoryOverride.value && (memoryMb.value !== (i.memoryMb ?? resolvedMemoryMb.value) || minMemoryMb.value !== (i.minMemoryMb ?? resolvedMinMemoryMb.value))) ||
    jvmArgs.value.trim() !== (i.jvmArgs ?? '') ||
    windowOverride.value !== hadWindowOverride ||
    (windowOverride.value &&
      (windowWidth.value !== (i.windowWidth ?? 925) || windowHeight.value !== (i.windowHeight ?? 530) || windowMaximized.value !== !!i.windowMaximized)) ||
    consoleMode.value !== (i.consoleMode ?? 'never') ||
    quickPlayMode.value !== (i.quickPlayMode ?? 'off') ||
    quickPlayTarget.value.trim() !== (i.quickPlayTarget ?? '') ||
    envVars.value.trim() !== (i.envVars ?? '') ||
    preLaunchCmd.value.trim() !== (i.preLaunchCmd ?? '') ||
    wrapperCmd.value.trim() !== (i.wrapperCmd ?? '') ||
    postExitCmd.value.trim() !== (i.postExitCmd ?? '')
  );
});

const supportsQuickPlay = computed(() => {
  if (!inst.value) return false;
  const target = resolveTargetVersion(inst.value.mcVersion, versions.manifest?.versions);
  const key = parseVersionKey(target);
  return !!key && compareVersionKeys(key, [1, 20, 0]) >= 0;
});

const consoleModeOptions = computed(() => [
  { value: 'always', label: t('instances.consoleAlways') },
  { value: 'on_crash', label: t('instances.consoleOnCrash') },
  { value: 'never', label: t('instances.consoleNever') },
]);
const quickPlayModeOptions = computed(() => [
  { value: 'off', label: t('instances.quickPlayOff') },
  { value: 'world', label: t('instances.quickPlayWorld') },
  { value: 'server', label: t('instances.quickPlayServer') },
]);
const worldOptions = computed(() => worlds.value.map((w) => ({ value: w.folder, label: w.name })));

function loadFromInstance() {
  if (!inst.value) return;
  name.value = inst.value.name;
  groupName.value = inst.value.groupName ?? '';

  javaPath.value = inst.value.javaPath ?? '';
  javaMode.value = inst.value.javaPath ? 'custom' : 'auto';
  skipJavaCheck.value = !!inst.value.skipJavaCheck;

  memoryOverride.value = inst.value.memoryMb != null || inst.value.minMemoryMb != null;
  memoryMb.value = inst.value.memoryMb ?? resolvedMemoryMb.value;
  minMemoryMb.value = inst.value.minMemoryMb ?? resolvedMinMemoryMb.value;

  jvmArgs.value = inst.value.jvmArgs ?? '';

  windowOverride.value = inst.value.windowWidth != null || inst.value.windowHeight != null;
  windowWidth.value = inst.value.windowWidth ?? 925;
  windowHeight.value = inst.value.windowHeight ?? 530;
  windowMaximized.value = !!inst.value.windowMaximized;

  consoleMode.value = inst.value.consoleMode ?? 'never';
  quickPlayMode.value = inst.value.quickPlayMode ?? 'off';
  quickPlayTarget.value = inst.value.quickPlayTarget ?? '';

  envVars.value = inst.value.envVars ?? '';
  preLaunchCmd.value = inst.value.preLaunchCmd ?? '';
  wrapperCmd.value = inst.value.wrapperCmd ?? '';
  postExitCmd.value = inst.value.postExitCmd ?? '';

  confirmingDelete.value = false;
  error.value = null;
}

async function loadGlobalDefaults() {
  try {
    const [s, recommended] = await Promise.all([getAppSettings(), getRecommendedMemoryMb()]);
    globalDefaults.value = {
      defaultMemoryMb: s.defaultMemoryMb,
      defaultMinMemoryMb: s.defaultMinMemoryMb,
      windowWidth: s.windowWidth,
      windowHeight: s.windowHeight,
      windowMaximized: s.windowMaximized,
    };
    recommendedMemoryMb.value = recommended;
  } catch {
    // Falls back to the same built-in defaults the backend itself uses.
  }
}

async function loadRequiredJava() {
  if (!inst.value) return;
  requiredJavaError.value = null;
  try {
    requiredJava.value = await getRequiredJava(inst.value.mcVersion);
  } catch (e) {
    requiredJava.value = null;
    requiredJavaError.value = String(e);
  }
}

async function loadDetectedJava() {
  detectingJava.value = true;
  try {
    detectedJava.value = await listJavaInstallations();
  } catch {
    detectedJava.value = [];
  } finally {
    detectingJava.value = false;
  }
}

async function browseForJava() {
  const path = await open({ multiple: false, filters: [{ name: 'Java', extensions: ['exe'] }] });
  if (typeof path === 'string') javaPath.value = path;
}

async function loadWorlds() {
  if (!inst.value) return;
  try {
    worlds.value = await listWorlds(inst.value.id);
  } catch {
    worlds.value = [];
  }
}

onMounted(async () => {
  await loadGlobalDefaults();
  loadFromInstance();
  loadRequiredJava();
  loadWorlds();
  loadDetectedJava();
  if (!versions.manifest) versions.fetch();
});
// Kept alive across navigation (see App.vue); re-sync whenever this page is shown again, since the
// current instance (or its saved settings) may have changed while away.
onActivated(async () => {
  await loadGlobalDefaults();
  loadFromInstance();
  loadRequiredJava();
  loadWorlds();
});
watch(() => inst.value?.id, loadFromInstance);
watch(() => inst.value?.mcVersion, loadRequiredJava);

watch(javaPath, (path) => {
  clearTimeout(javaProbeHandle);
  javaProbe.value = null;
  const trimmed = path.trim();
  if (!trimmed) return;
  javaProbeHandle = setTimeout(async () => {
    try {
      const majorVersion = await probeJavaAt(trimmed);
      javaProbe.value = { majorVersion };
    } catch (e) {
      javaProbe.value = { error: String(e) };
    }
  }, 500);
});

const javaMismatch = computed(() => {
  if (javaMode.value !== 'custom' || !javaProbe.value?.majorVersion || !requiredJava.value?.majorVersion) return false;
  return javaProbe.value.majorVersion !== requiredJava.value.majorVersion;
});

function discardChanges() {
  loadFromInstance();
}

async function save() {
  if (!inst.value || !hasPendingChanges.value || !name.value.trim() || memoryTooLow.value) return;
  saving.value = true;
  error.value = null;
  try {
    await instances.update({
      id: inst.value.id,
      name: name.value.trim(),
      groupName: groupName.value.trim() || null,
      mcVersion: inst.value.mcVersion,
      memoryMb: memoryOverride.value ? memoryMb.value : null,
      minMemoryMb: memoryOverride.value ? minMemoryMb.value : null,
      jvmArgs: jvmArgs.value.trim() || null,
      javaPath: javaMode.value === 'custom' ? javaPath.value.trim() || null : null,
      iconBiome: inst.value.iconBiome,
      skipJavaCheck: skipJavaCheck.value,
      windowWidth: windowOverride.value ? windowWidth.value : null,
      windowHeight: windowOverride.value ? windowHeight.value : null,
      windowMaximized: windowOverride.value ? windowMaximized.value : false,
      envVars: envVars.value.trim() || null,
      preLaunchCmd: preLaunchCmd.value.trim() || null,
      wrapperCmd: wrapperCmd.value.trim() || null,
      postExitCmd: postExitCmd.value.trim() || null,
      consoleMode: consoleMode.value,
      quickPlayMode: supportsQuickPlay.value ? quickPlayMode.value : 'off',
      quickPlayTarget: quickPlayTarget.value.trim() || null,
    });
  } catch (e) {
    error.value = String(e);
  } finally {
    saving.value = false;
  }
}

async function confirmDelete() {
  if (!inst.value) return;
  await instances.remove(inst.value.id);
  router.push('/');
}
</script>

<template>
  <section class="view" v-if="!inst">
    <div class="empty-state">
      <p style="margin: 0 0 14px">{{ t('version.noInstance') }}</p>
      <router-link to="/" class="btn btn-mineral">{{ t('sidebar.library') }}</router-link>
    </div>
  </section>

  <section class="view" v-else>
    <p class="view-intro">{{ t('instances.settingsIntro') }}</p>
    <div v-if="error" class="error-box" style="margin-top: 14px">{{ error }}</div>

    <div v-if="confirmingDelete" class="field-group" style="margin-top: 16px; max-width: 480px">
      <h4 style="margin: 0 0 8px">{{ t('instances.deleteConfirmTitle', { name: inst.name }) }}</h4>
      <p class="view-intro">{{ t('instances.deleteConfirmBody') }}</p>
      <div style="display: flex; gap: 10px; margin-top: 16px">
        <button class="btn btn-ghost" style="flex: 1" type="button" @click="confirmingDelete = false">
          {{ t('instances.cancel') }}
        </button>
        <button class="btn btn-danger-ghost" style="flex: 1" type="button" @click="confirmDelete">
          {{ t('instances.confirmDelete') }}
        </button>
      </div>
    </div>

    <div v-else class="settings-layout" style="margin-top: 16px">
      <nav class="settings-nav">
        <button type="button" class="nav-item" :class="{ active: active === 'general' }" @click="active = 'general'">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8">
            <rect x="4" y="4" width="16" height="16" rx="3" />
            <path d="M8 9h8M8 13h5" />
          </svg>
          <span>{{ t('instances.sectionGeneral') }}</span>
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

        <div class="nav-group-label">{{ t('instances.advancedGroupLabel') }}</div>
        <button type="button" class="nav-item" :class="{ active: active === 'jvm' }" @click="active = 'jvm'">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8">
            <polyline points="8 9 4 12 8 15" />
            <polyline points="16 9 20 12 16 15" />
            <line x1="13" y1="6" x2="11" y2="18" />
          </svg>
          <span>{{ t('instances.sectionJvm') }}</span>
        </button>
        <button type="button" class="nav-item" :class="{ active: active === 'window' }" @click="active = 'window'">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8">
            <rect x="3" y="4" width="18" height="16" rx="2" />
            <path d="M3 8h18" />
          </svg>
          <span>{{ t('instances.sectionWindow') }}</span>
        </button>
        <button type="button" class="nav-item" :class="{ active: active === 'behavior' }" @click="active = 'behavior'">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
            <path d="M5 3v18" />
            <path d="M5 4h12l-2.5 3L17 10H5" />
          </svg>
          <span>{{ t('instances.sectionOnLaunch') }}</span>
        </button>
        <button type="button" class="nav-item" :class="{ active: active === 'commands' }" @click="active = 'commands'">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8">
            <polyline points="4 17 10 11 4 5" />
            <line x1="12" y1="19" x2="20" y2="19" />
          </svg>
          <span>{{ t('instances.sectionCommands') }}</span>
        </button>
      </nav>

      <div class="settings-panel">
        <div v-if="active === 'general'" class="field-group">
          <h4>{{ t('instances.sectionGeneral') }}</h4>
          <label class="field-label settings-field-label" for="inst-settings-name">{{ t('instances.nameLabel') }}</label>
          <input id="inst-settings-name" v-model="name" class="field" maxlength="48" />

          <label class="field-label settings-field-label" for="inst-settings-group">{{ t('instances.groupLabel') }}</label>
          <input
            id="inst-settings-group"
            v-model="groupName"
            class="field"
            maxlength="48"
            :placeholder="t('instances.groupPlaceholder')"
          />

          <button
            class="btn btn-danger-ghost btn-block"
            type="button"
            style="margin-top: 20px"
            @click="confirmingDelete = true"
          >
            {{ t('instances.deleteInstance') }}
          </button>
        </div>

        <div v-else-if="active === 'memory'" class="field-group">
          <h4>{{ t('instances.sectionMemory') }}</h4>

          <label class="checkbox-row">
            <input type="checkbox" v-model="memoryOverride" />
            {{ t('instances.memoryOverride') }}
          </label>

          <template v-if="memoryOverride">
            <label class="field-label settings-field-label" for="inst-memory-max">{{ t('instances.maxMemoryLabel') }}</label>
            <div class="slider-row">
              <input type="range" min="512" max="16384" step="256" v-model.number="memoryMb" />
              <div class="slider-num-wrap">
                <input id="inst-memory-max" type="number" class="slider-num" min="512" max="65536" step="1" v-model.number="memoryMb" />
                <span class="slider-num-suffix">MB</span>
              </div>
            </div>
            <p class="hint">{{ t('instances.memoryGbEquivalent', { gb: memoryLabel }) }}</p>

            <label class="field-label settings-field-label" for="inst-memory-min">{{ t('instances.minMemoryLabel') }}</label>
            <div class="slider-row">
              <input type="range" min="512" max="16384" step="256" v-model.number="minMemoryMb" />
              <div class="slider-num-wrap">
                <input id="inst-memory-min" type="number" class="slider-num" min="512" max="65536" step="1" v-model.number="minMemoryMb" />
                <span class="slider-num-suffix">MB</span>
              </div>
            </div>
            <p class="hint">{{ t('instances.minMemoryHint') }}</p>
            <p v-if="memoryTooLow" class="java-warning">{{ t('instances.memoryTooLow') }}</p>
          </template>
          <p v-else class="hint inherit-note">
            {{ t('instances.memoryInherited', { max: (resolvedMemoryMb / 1024).toFixed(1), min: (resolvedMinMemoryMb / 1024).toFixed(1) }) }}
          </p>
        </div>

        <div v-else-if="active === 'java'" class="field-group">
          <h4>{{ t('instances.sectionJava') }}</h4>
          <div class="loader-picker">
            <button type="button" class="loader-opt" :class="{ selected: javaMode === 'auto' }" @click="javaMode = 'auto'">
              {{ t('instances.javaAuto') }}
              <span class="lo-ver">
                <template v-if="requiredJava?.majorVersion">{{ t('instances.javaAutoHint', { version: requiredJava.majorVersion }) }}</template>
                <template v-else-if="requiredJavaError">{{ t('instances.javaAutoUnknown') }}</template>
              </span>
            </button>
            <button type="button" class="loader-opt" :class="{ selected: javaMode === 'custom' }" @click="javaMode = 'custom'">
              {{ t('instances.javaCustom') }}
              <span class="lo-ver">{{ t('instances.javaCustomHint') }}</span>
            </button>
          </div>

          <template v-if="javaMode === 'custom'">
            <div style="display: flex; gap: 8px; margin-top: 10px">
              <input id="inst-settings-java" v-model="javaPath" class="field mono" style="flex: 1" :placeholder="t('instances.javaPathPlaceholder')" />
              <button class="btn btn-ghost btn-sm" type="button" @click="browseForJava">{{ t('instances.javaBrowse') }}</button>
            </div>
            <p v-if="javaProbe?.error" class="java-warning">{{ t('instances.javaProbeFailed') }}</p>
            <p v-else-if="javaMismatch" class="java-warning">
              {{ t('instances.javaMismatch', { actual: javaProbe.majorVersion, required: requiredJava.majorVersion }) }}
            </p>
            <p v-else-if="javaProbe?.majorVersion" class="java-ok">
              {{ t('instances.javaProbeOk', { version: javaProbe.majorVersion }) }}
            </p>

            <div class="detected-java-head">
              <label class="field-label settings-field-label">{{ t('instances.javaDetectedLabel') }}</label>
              <button class="btn btn-ghost btn-sm" type="button" :disabled="detectingJava" @click="loadDetectedJava">
                <span v-if="detectingJava" class="spinner"></span>
                <svg v-else viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M3 12a9 9 0 1 0 3-6.7M3 4v5h5" /></svg>
                {{ t('instances.javaRescan') }}
              </button>
            </div>
            <p v-if="!detectingJava && detectedJava.length === 0" class="hint">{{ t('instances.javaNoneDetected') }}</p>
            <div v-else class="component-list">
              <button
                v-for="j in detectedJava"
                :key="j.path"
                type="button"
                class="component-row detected-java-item"
                :class="{ selected: j.path === javaPath }"
                :title="j.path"
                @click="javaPath = j.path"
              >
                <img src="/loaders/full/openjdk.png" alt="" class="component-icon" />
                <span class="component-name">{{ j.source }} · <span class="mono">{{ j.path }}</span></span>
                <span class="component-version mono">{{ t('instances.javaDetectedVersion', { version: j.majorVersion }) }}</span>
              </button>
            </div>
          </template>

          <label class="checkbox-row" style="margin-top: 18px">
            <input type="checkbox" v-model="skipJavaCheck" />
            {{ t('instances.skipJavaCheck') }}
          </label>
          <p class="java-warning">{{ t('instances.skipJavaCheckHint') }}</p>
        </div>

        <div v-else-if="active === 'jvm'" class="field-group">
          <h4>{{ t('instances.sectionJvm') }}</h4>
          <label class="field-label" for="inst-settings-jvm">{{ t('instances.jvmArgsLabel') }}</label>
          <textarea
            id="inst-settings-jvm"
            v-model="jvmArgs"
            class="textarea-field"
            :placeholder="t('instances.jvmArgsPlaceholder')"
          ></textarea>
          <p class="hint">{{ t('instances.jvmArgsHint') }}</p>
        </div>

        <div v-else-if="active === 'window'" class="field-group">
          <h4>{{ t('instances.sectionWindow') }}</h4>

          <label class="checkbox-row">
            <input type="checkbox" v-model="windowOverride" />
            {{ t('instances.windowOverride') }}
          </label>

          <template v-if="windowOverride">
            <label class="checkbox-row" style="margin-top: 14px">
              <input type="checkbox" v-model="windowMaximized" />
              {{ t('instances.windowMaximized') }}
            </label>

            <template v-if="!windowMaximized">
              <div class="dim-row" style="margin-top: 14px">
                <div>
                  <label class="field-label" for="inst-settings-width">{{ t('instances.windowWidthLabel') }}</label>
                  <input id="inst-settings-width" type="number" min="200" v-model.number="windowWidth" class="field mono" />
                </div>
                <div>
                  <label class="field-label" for="inst-settings-height">{{ t('instances.windowHeightLabel') }}</label>
                  <input id="inst-settings-height" type="number" min="200" v-model.number="windowHeight" class="field mono" />
                </div>
              </div>
            </template>
          </template>
          <p v-else class="hint inherit-note">{{ t('instances.windowInherited', { size: resolvedWindowLabel }) }}</p>
        </div>

        <div v-else-if="active === 'behavior'" class="field-group">
          <h4>{{ t('instances.sectionOnLaunch') }}</h4>

          <label class="field-label" for="inst-console-mode">{{ t('instances.consoleLabel') }}</label>
          <GlassSelect id="inst-console-mode" v-model="consoleMode" :options="consoleModeOptions" style="width: 220px" />
          <p class="hint">{{ t('instances.consoleHint') }}</p>

          <template v-if="supportsQuickPlay">
            <label class="field-label settings-field-label" for="inst-quickplay-mode">{{ t('instances.quickPlayLabel') }}</label>
            <GlassSelect id="inst-quickplay-mode" v-model="quickPlayMode" :options="quickPlayModeOptions" style="width: 220px" />

            <template v-if="quickPlayMode === 'world'">
              <GlassSelect
                v-if="worldOptions.length"
                v-model="quickPlayTarget"
                :options="worldOptions"
                style="width: 260px; margin-top: 10px"
              />
              <p v-else class="hint">{{ t('instances.quickPlayNoWorlds') }}</p>
            </template>
            <input
              v-else-if="quickPlayMode === 'server'"
              v-model="quickPlayTarget"
              class="field mono"
              style="margin-top: 10px; max-width: 260px"
              :placeholder="t('instances.quickPlayServerPlaceholder')"
            />
          </template>
          <p v-else class="hint">{{ t('instances.quickPlayUnsupported') }}</p>
        </div>

        <div v-else-if="active === 'commands'" class="field-group">
          <h4>{{ t('instances.sectionCommands') }}</h4>
          <p class="hint">{{ t('instances.commandsHint') }}</p>

          <label class="field-label settings-field-label" for="inst-settings-env">{{ t('instances.envVarsLabel') }}</label>
          <textarea
            id="inst-settings-env"
            v-model="envVars"
            class="textarea-field mono"
            :placeholder="t('instances.envVarsPlaceholder')"
          ></textarea>

          <label class="field-label settings-field-label" for="inst-pre-launch">{{ t('instances.preLaunchLabel') }}</label>
          <input id="inst-pre-launch" v-model="preLaunchCmd" class="field mono" :placeholder="t('instances.preLaunchPlaceholder')" />

          <label class="field-label settings-field-label" for="inst-wrapper">{{ t('instances.wrapperLabel') }}</label>
          <input id="inst-wrapper" v-model="wrapperCmd" class="field mono" :placeholder="t('instances.wrapperPlaceholder')" />
          <p class="hint">{{ t('instances.wrapperHint') }}</p>

          <label class="field-label settings-field-label" for="inst-post-exit">{{ t('instances.postExitLabel') }}</label>
          <input id="inst-post-exit" v-model="postExitCmd" class="field mono" :placeholder="t('instances.preLaunchPlaceholder')" />
        </div>

        <Transition name="fade-slide">
          <div v-if="hasPendingChanges" class="save-bar">
            <span class="save-msg">{{ t('instances.unsavedChanges') }}</span>
            <div class="save-bar-actions">
              <button class="btn btn-ghost" type="button" :disabled="saving" @click="discardChanges">
                {{ t('instances.discardChanges') }}
              </button>
              <button
                class="btn btn-mineral"
                type="button"
                :disabled="saving || !name.trim() || memoryTooLow"
                @click="save"
              >
                <span v-if="saving" class="spinner"></span>
                {{ saving ? t('instances.saving') : t('instances.saveChanges') }}
              </button>
            </div>
          </div>
        </Transition>
      </div>
    </div>
  </section>
</template>
