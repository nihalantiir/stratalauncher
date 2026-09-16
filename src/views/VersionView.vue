<script setup>
import { ref, computed, watch, onMounted, onActivated } from 'vue';
import { useI18n } from 'vue-i18n';
import { open } from '@tauri-apps/plugin-dialog';
import { useInstancesStore } from '../stores/instances';
import { useVersionsStore } from '../stores/versions';
import { getVersionComponents } from '../api/loaders';
import { getCoremodIcon } from '../api/instances';
import { LOADER_KEYS, useLoaderPicker } from '../composables/useLoaderPicker';
import VersionSelect from '../components/version/VersionSelect.vue';
import GlassSelect from '../components/common/GlassSelect.vue';

const { t } = useI18n();
const instances = useInstancesStore();
const versions = useVersionsStore();

const inst = computed(() => instances.current);

const applying = ref(false);
const error = ref(null);

// Nothing mutates the real instance until Apply is clicked; every control
// edits these pending refs instead, same shape InstanceSettingsView uses.
const pendingMcVersion = ref('');
const {
  loader: pendingLoader,
  loaderVersion: pendingLoaderVersion,
  loaderVersions,
  loaderVersionOptions,
  availability,
  loadingLoaderVersions,
  refreshAvailability,
  refreshLoaderVersions,
  selectLoader: selectLoaderPicker,
  loaderDisabled,
  loaderStatusLabel: loaderStatusLabelPicker,
  loaderFullIconUrl,
} = useLoaderPicker({ t, isBusy: () => applying.value, error });

// Reflects the real saved instance, not the pending picker above; showing
// components for an unapplied configuration would be misleading.
const components = ref([]);
const loadingComponents = ref(false);
const componentsError = ref(null);
// Bumped on every refreshComponents() call so a stale in-flight request
// can't land after and clobber a newer one's result.
let componentsRequestId = 0;
const COMPONENT_ICONS = {
  minecraft: '/grass-block.png',
  intermediary: '/loaders/full/fabric.png',
  'fabric-loader': '/loaders/full/fabric.png',
  'quilt-loader': '/loaders/full/quilt.png',
  forge: '/loaders/full/forge.png',
  neoforge: '/loaders/full/neoforge.png',
  lwjgl: '/loaders/full/lwjgl.png',
};
function componentIcon(id) {
  return COMPONENT_ICONS[id] ?? '/grass-block.png';
}
function componentLabel(id) {
  return t(`version.component.${id.replace(/-([a-z])/g, (_, c) => c.toUpperCase())}`);
}
async function refreshComponents() {
  const requestId = ++componentsRequestId;
  if (!inst.value) {
    components.value = [];
    componentsError.value = null;
    return;
  }
  loadingComponents.value = true;
  componentsError.value = null;
  try {
    const result = await getVersionComponents(inst.value.mcVersion, inst.value.loader, inst.value.loaderVersion);
    if (requestId !== componentsRequestId) return; // superseded by a newer refresh
    components.value = result;
  } catch (e) {
    if (requestId !== componentsRequestId) return;
    components.value = [];
    componentsError.value = String(e);
  } finally {
    if (requestId === componentsRequestId) loadingComponents.value = false;
  }
}

const hasPendingChanges = computed(() => {
  if (!inst.value) return false;
  return (
    pendingLoader.value !== inst.value.loader ||
    pendingMcVersion.value !== inst.value.mcVersion ||
    (pendingLoader.value !== 'vanilla' && pendingLoaderVersion.value !== inst.value.loaderVersion)
  );
});
// Defaults to "available" while checkLoaderAvailability is still in flight,
// so a loader doesn't flash disabled on every version change.
const pendingLoaderAvailable = computed(
  () => pendingLoader.value === 'vanilla' || (availability.value[pendingLoader.value]?.available ?? true),
);
const canApply = computed(
  () =>
    hasPendingChanges.value &&
    pendingLoaderAvailable.value &&
    (pendingLoader.value === 'vanilla' || !!pendingLoaderVersion.value),
);

function loadFromInstance() {
  if (!inst.value) return;
  pendingLoader.value = inst.value.loader;
  pendingMcVersion.value = inst.value.mcVersion;
  pendingLoaderVersion.value = inst.value.loaderVersion;
}

async function refreshAll() {
  if (!versions.manifest) await versions.fetch();
  loadFromInstance();
  refreshComponents();
  if (!inst.value) return;
  await Promise.all([refreshAvailability(pendingMcVersion.value), refreshLoaderVersions(pendingLoader.value, pendingMcVersion.value)]);
}

onMounted(refreshAll);
// Kept alive across navigation (see App.vue); re-sync if the current
// instance changed while away.
onActivated(refreshAll);
watch(() => inst.value?.id, refreshAll);

function selectLoader(key) {
  return selectLoaderPicker(key, pendingMcVersion.value);
}

async function selectMcVersion(newVersion) {
  if (applying.value || newVersion === pendingMcVersion.value) return;
  error.value = null;
  pendingMcVersion.value = newVersion;
  await refreshAvailability(newVersion);
  if (pendingLoader.value !== 'vanilla') {
    await refreshLoaderVersions(pendingLoader.value, newVersion);
    pendingLoaderVersion.value = loaderVersions.value[0]?.version ?? null;
  }
}

function selectLoaderVersion(version) {
  pendingLoaderVersion.value = version;
}

function discardChanges() {
  error.value = null;
  loadFromInstance();
  refreshLoaderVersions(pendingLoader.value, pendingMcVersion.value);
}

async function applyChanges() {
  if (!inst.value || !canApply.value) return;
  applying.value = true;
  error.value = null;
  try {
    await instances.setLoader(
      inst.value.id,
      pendingLoader.value,
      pendingLoader.value === 'vanilla' ? null : pendingLoaderVersion.value,
      pendingMcVersion.value,
    );
    loadFromInstance();
    refreshComponents();
  } catch (e) {
    error.value = String(e);
  } finally {
    applying.value = false;
  }
}

function loaderStatusLabel(key) {
  return loaderStatusLabelPicker(key, pendingMcVersion.value);
}

// A plain div, not <button>, so GlassSelect can nest inside without
// interactive-in-interactive HTML; the disabled guard is done manually.
function handleLoaderRowClick(key) {
  if (loaderDisabled(key) || applying.value) return;
  selectLoader(key);
}

const coremodsBusy = ref(false);
const coremodsError = ref(null);
// A coremod jar's manifest has no reliable name field, so the filename is
// shown instead. Icon bytes become an object URL, revoked before replacement.
const coremodName = computed(() => {
  const path = inst.value?.customClientJar;
  if (!path) return '';
  return path.split(/[/\\]/).pop();
});
const coremodIconUrl = ref(null);

async function refreshCoremodIcon() {
  if (coremodIconUrl.value) {
    URL.revokeObjectURL(coremodIconUrl.value);
    coremodIconUrl.value = null;
  }
  const path = inst.value?.customClientJar;
  if (!path) return;
  try {
    const bytes = await getCoremodIcon(path);
    if (!bytes) return;
    const blob = new Blob([new Uint8Array(bytes)], { type: 'image/png' });
    coremodIconUrl.value = URL.createObjectURL(blob);
  } catch {
    // No icon, or the jar couldn't be read; the generic fallback mark covers both.
  }
}
watch(() => inst.value?.customClientJar, refreshCoremodIcon, { immediate: true });

async function pickCustomJar() {
  if (!inst.value || coremodsBusy.value) return;
  const path = await open({ multiple: false, filters: [{ name: 'Jar files', extensions: ['jar'] }] });
  if (!path) return;
  coremodsBusy.value = true;
  coremodsError.value = null;
  try {
    await instances.setCustomClientJar(inst.value.id, path);
  } catch (e) {
    coremodsError.value = String(e);
  } finally {
    coremodsBusy.value = false;
  }
}

async function clearCustomJar() {
  if (!inst.value || coremodsBusy.value) return;
  coremodsBusy.value = true;
  coremodsError.value = null;
  try {
    await instances.setCustomClientJar(inst.value.id, null);
  } catch (e) {
    coremodsError.value = String(e);
  } finally {
    coremodsBusy.value = false;
  }
}
</script>

<template>
  <section class="view" v-if="inst">
    <p class="view-intro">{{ t('version.intro') }}</p>

    <div class="panel version-panel">
      <div class="version-grid">
        <div class="version-col">
          <h4>{{ t('version.loaderHeading') }}</h4>
          <p class="hint">{{ t('version.loaderHint') }}</p>
          <div class="loader-list">
            <div
              v-for="key in LOADER_KEYS"
              :key="key"
              class="loader-row"
              :class="{ selected: pendingLoader === key, disabled: loaderDisabled(key) || applying }"
              :tabindex="loaderDisabled(key) || applying ? -1 : 0"
              role="button"
              @click="handleLoaderRowClick(key)"
              @keydown.enter="handleLoaderRowClick(key)"
              @keydown.space.prevent="handleLoaderRowClick(key)"
            >
              <span class="loader-avatar" :class="{ 'loader-avatar-full': pendingLoader === key }">
                <img v-if="pendingLoader === key" :src="loaderFullIconUrl(key)" alt="" class="loader-avatar-img" />
                <span v-else class="loader-avatar-icon" :style="{ maskImage: `url(/loaders/${key}.svg)`, webkitMaskImage: `url(/loaders/${key}.svg)` }"></span>
              </span>
              <span class="loader-name">{{ t(`loaders.${key}`) }}</span>
              <svg v-if="pendingLoader === key" class="loader-check" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3">
                <path d="M20 6L9 17l-5-5" />
              </svg>
              <GlassSelect
                v-if="pendingLoader === key && key !== 'vanilla'"
                :model-value="pendingLoaderVersion"
                :options="loaderVersionOptions"
                :disabled="applying || loadingLoaderVersions"
                class="loader-row-select"
                @click.stop
                @update:model-value="selectLoaderVersion"
              />
              <span v-else class="loader-status">{{ loaderStatusLabel(key) }}</span>
            </div>
          </div>
        </div>

        <div class="version-col version-col-mc">
          <h4>{{ t('version.mcVersionLabel') }}</h4>
          <VersionSelect
            :model-value="pendingMcVersion"
            :groups="versions.groupedSelectableVersions"
            :loading="versions.loading"
            :disabled="applying"
            style="width: 100%"
            @update:model-value="selectMcVersion"
          />
          <label class="checkbox-row" style="margin-top: 10px">
            <input
              type="checkbox"
              :checked="versions.showExperimental"
              @change="versions.setShowExperimental($event.target.checked)"
            />
            {{ t('versions.showExperimental') }}
          </label>
        </div>
      </div>
    </div>

    <div class="panel version-panel components-panel">
      <div class="version-grid">
        <div class="version-col">
          <h4>{{ t('version.componentsHeading') }}</h4>
          <p class="hint">{{ t('version.componentsHint') }}</p>
          <div v-if="loadingComponents" class="hint">{{ t('version.checking') }}</div>
          <div v-else-if="componentsError" class="hint hint-warn">
            {{ t('version.componentsError') }}
            <button class="btn btn-ghost btn-sm" type="button" style="margin-left: 8px" @click="refreshComponents">
              {{ t('versions.retry') }}
            </button>
          </div>
          <div v-else class="component-list">
            <div v-for="c in components" :key="c.id" class="component-row">
              <img :src="componentIcon(c.id)" alt="" class="component-icon" />
              <span class="component-name">{{ componentLabel(c.id) }}</span>
              <span class="component-version mono">{{ c.version }}</span>
            </div>
          </div>
        </div>

        <div class="version-col version-col-mc">
          <h4>{{ t('version.coremodsHeading') }}</h4>
          <p class="hint">{{ t('version.coremodsHint') }}</p>
          <div v-if="inst.customClientJar" class="coremods-current coremods-custom">
            <img v-if="coremodIconUrl" :src="coremodIconUrl" alt="" class="component-icon" />
            <span v-else class="component-icon coremods-icon-fallback"></span>
            <span class="mono">{{ coremodName }}</span>
          </div>
          <div class="coremods-actions">
            <button class="btn btn-ghost btn-sm" type="button" :disabled="coremodsBusy" @click="pickCustomJar">
              <span v-if="coremodsBusy" class="spinner"></span>
              {{ t('skin.chooseFile') }}
            </button>
            <button
              v-if="inst.customClientJar"
              class="btn btn-ghost btn-sm"
              type="button"
              :disabled="coremodsBusy"
              @click="clearCustomJar"
            >
              {{ t('version.coremodsReset') }}
            </button>
          </div>
          <p v-if="coremodsError" class="hint hint-warn">{{ coremodsError }}</p>
        </div>
      </div>
    </div>

    <div v-if="error" class="error-box" style="margin-top: 16px">{{ error }}</div>

    <Transition name="fade-slide">
      <div v-if="hasPendingChanges" class="save-bar">
        <span class="save-msg">
          {{ t('instances.unsavedChanges') }}
          <template v-if="canApply"> {{ t('version.backupNotice') }}</template>
        </span>
        <div class="save-bar-actions">
          <button class="btn btn-ghost" type="button" :disabled="applying" @click="discardChanges">
            {{ t('instances.discardChanges') }}
          </button>
          <button class="btn btn-mineral" type="button" :disabled="applying || !canApply" @click="applyChanges">
            <span v-if="applying" class="spinner"></span>
            {{ applying ? t('instances.saving') : t('instances.saveChanges') }}
          </button>
        </div>
      </div>
    </Transition>
  </section>

  <section class="view" v-else>
    <div class="empty-state">
      <p style="margin: 0 0 14px">{{ t('version.noInstance') }}</p>
      <router-link to="/" class="btn btn-mineral">{{ t('sidebar.library') }}</router-link>
    </div>
  </section>
</template>
