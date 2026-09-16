<script setup>
import { ref, computed, onMounted, onBeforeUnmount, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import { listen } from '@tauri-apps/api/event';
import { useVersionsStore } from '../../stores/versions';
import { useInstancesStore } from '../../stores/instances';
import { listContentVersions, curseforgeConfigured } from '../../api/content';
import { searchModpacks, listModpackCategories, installModpackVersion, installModpackFile, pickModpackFile } from '../../api/modpacks';
import { firstGrapheme } from '../../lib/text';
import { BIOME_KEYS } from '../../biomes';
import { panoramaForInstance } from '../../lib/panoramas';
import { LOADER_KEYS, useLoaderPicker } from '../../composables/useLoaderPicker';
import { useCatalogSearch } from '../../composables/useCatalogSearch';
import VersionSelect from '../version/VersionSelect.vue';
import GlassSelect from '../common/GlassSelect.vue';

const emit = defineEmits(['close']);
const { t } = useI18n();
const versions = useVersionsStore();
const instances = useInstancesStore();

// 'import' is the real modpack-import flow (not the existing multi-launcher
// migration page at /import, which pulls in whole other launchers' instances).
const mode = ref('new');

const name = ref('');
const groupName = ref('');
// No picker for this anymore, since a real panorama image always covers
// the tile/hero backgrounds; kept as a silent random pick since Worlds/Servers still use it to tint their own icon-less fallback avatars.
const selectedBiome = ref(BIOME_KEYS[Math.floor(Math.random() * BIOME_KEYS.length)]);
const selectedVersion = ref('');
const error = ref(null);
const creating = ref(false);
const {
  loader,
  loaderVersion,
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
} = useLoaderPicker({ t, isBusy: () => creating.value, error });

onMounted(async () => {
  if (!versions.manifest) await versions.fetch();
  selectedVersion.value = versions.manifest?.latest?.release ?? versions.releases[0]?.id ?? '';
  refreshAvailability(selectedVersion.value);

  curseforgeConfigured()
    .then((v) => (curseforgeAvailable.value = v))
    .catch(() => {});
  unlistenInstallProgress = await listen('download://progress', (event) => {
    const p = event.payload;
    if (p.stage !== 'modpack') return;
    installProgress.value = p.completed >= p.total ? null : p;
  });
});

onBeforeUnmount(() => {
  unlistenInstallProgress?.();
});

async function selectMcVersion(v) {
  if (creating.value || v === selectedVersion.value) return;
  selectedVersion.value = v;
  error.value = null;
  await refreshAvailability(v);
  if (loader.value !== 'vanilla') {
    await refreshLoaderVersions(loader.value, v);
    loaderVersion.value = loaderVersions.value[0]?.version ?? null;
  }
}

function selectLoader(key) {
  return selectLoaderPicker(key, selectedVersion.value);
}

function handleLoaderRowClick(key) {
  if (loaderDisabled(key) || creating.value) return;
  selectLoader(key);
}

function loaderStatusLabel(key) {
  return loaderStatusLabelPicker(key, selectedVersion.value);
}

// The exact same panoramaForInstance() Library tiles use; since it only reads
// .id/.mcVersion off whatever's passed in, a not-yet-real instance previews identically to how it'll actually look once created.
const previewName = computed(() => name.value.trim() || t('instances.previewPlaceholderName'));
const previewPanoramaUrl = computed(() =>
  panoramaForInstance({ id: name.value || 'preview', mcVersion: selectedVersion.value }, versions.manifest?.versions),
);

const canCreate = computed(() => {
  if (mode.value !== 'new') return false;
  if (!name.value.trim() || !selectedVersion.value) return false;
  if (loader.value !== 'vanilla') {
    if (!loaderVersion.value) return false;
    if (availability.value[loader.value]?.available === false) return false;
  }
  return true;
});

const importSubMode = ref('browse'); // 'browse' | 'file'
const curseforgeAvailable = ref(false);
const selectedPack = ref(null);
const packVersions = ref([]);
const loadingPackVersions = ref(false);
const selectedPackVersion = ref(null);
const pickedFilePath = ref(null);
const installProgress = ref(null);
let unlistenInstallProgress = null;

const {
  source: packSource,
  query: packQuery,
  results: packResults,
  searching: packSearching,
  loadingMore: packLoadingMore,
  searchError: packSearchError,
  hasMore: packHasMore,
  categories: packCategories,
  selectedCategories: selectedPackCategories,
  loadingCategories: loadingPackCategories,
  sortKey: packSortKey,
  sortOptions: packSortOptions,
  groupedCategories: groupedPackCategories,
  hitKey: packHitKey,
  runSearch: runPackSearch,
  loadMore: loadMorePacks,
  loadCategories: loadPackCategories,
  selectSource: selectPackSourceCatalog,
} = useCatalogSearch({
  t,
  fetchPage: (offset) => searchModpacks(packSource.value, packQuery.value, selectedVersion.value || null, selectedPackCategories.value, packSortKey.value, offset),
  fetchCategories: (src) => listModpackCategories(src),
});

function selectPackSource(key) {
  return selectPackSourceCatalog(key, curseforgeAvailable.value);
}

function selectPackVersionFilter(v) {
  selectedVersion.value = v;
  runPackSearch();
}

function enterImportMode() {
  mode.value = 'import';
  if (packResults.value.length === 0 && !packSearching.value) {
    loadPackCategories();
    runPackSearch();
  }
}

async function selectPack(hit) {
  selectedPack.value = hit;
  selectedPackVersion.value = null;
  packVersions.value = [];
  if (!name.value.trim()) name.value = hit.title;
  loadingPackVersions.value = true;
  error.value = null;
  try {
    packVersions.value = await listContentVersions(hit.source, hit.projectId);
    selectedPackVersion.value = packVersions.value[0] ?? null;
  } catch (e) {
    error.value = String(e);
  } finally {
    loadingPackVersions.value = false;
  }
}

async function pickFile() {
  const path = await pickModpackFile();
  if (!path) return;
  pickedFilePath.value = path;
  if (!name.value.trim()) {
    name.value = path.split(/[\\/]/).pop().replace(/\.(mrpack|zip)$/i, '');
  }
}

const canInstallPack = computed(() => {
  if (mode.value !== 'import') return false;
  if (!name.value.trim()) return false;
  return importSubMode.value === 'browse' ? !!selectedPackVersion.value : !!pickedFilePath.value;
});

async function installPack() {
  if (!canInstallPack.value || creating.value) return;
  creating.value = true;
  error.value = null;
  installProgress.value = null;
  try {
    if (importSubMode.value === 'browse') {
      await installModpackVersion(selectedPackVersion.value, name.value.trim(), groupName.value.trim() || null);
    } else {
      await installModpackFile(pickedFilePath.value, name.value.trim(), groupName.value.trim() || null);
    }
    await instances.refresh();
    emit('close');
  } catch (e) {
    error.value = String(e);
  } finally {
    creating.value = false;
    installProgress.value = null;
  }
}

async function submit() {
  if (!canCreate.value || creating.value) return;
  creating.value = true;
  error.value = null;
  try {
    await instances.create(
      name.value.trim(),
      selectedVersion.value,
      selectedBiome.value,
      loader.value === 'vanilla' ? null : loader.value,
      loader.value === 'vanilla' ? null : loaderVersion.value,
      groupName.value.trim() || null,
    );
    emit('close');
  } catch (e) {
    error.value = String(e);
  } finally {
    creating.value = false;
  }
}
</script>

<template>
  <Teleport to="body">
    <div class="modal-backdrop" @click.self="emit('close')">
      <div class="modal create-instance-modal">
        <div class="modal-head">
          <h3>{{ t('instances.createTitle') }}</h3>
          <button class="modal-close" type="button" @click="emit('close')">✕</button>
        </div>

        <div class="modal-body create-instance-body" :class="{ 'import-unified': mode === 'import' }">
          <div class="create-sidebar">
            <div class="segmented-toggle">
              <button type="button" class="segmented-opt" :class="{ selected: mode === 'new' }" @click="mode = 'new'">
                {{ t('instances.modeNew') }}
              </button>
              <button type="button" class="segmented-opt" :class="{ selected: mode === 'import' }" @click="enterImportMode">
                {{ t('instances.modeImport') }}
              </button>
            </div>

            <template v-if="mode === 'new'">
              <label class="field-label" for="instance-name">{{ t('instances.nameLabel') }}</label>
              <input
                id="instance-name"
                v-model="name"
                class="field"
                :placeholder="t('instances.namePlaceholder')"
                maxlength="48"
                @keyup.enter="submit"
              />

              <label class="field-label" style="margin-top: 16px" for="instance-version">{{ t('version.mcVersionLabel') }}</label>
              <VersionSelect
                id="instance-version"
                :model-value="selectedVersion"
                :groups="versions.groupedSelectableVersions"
                :loading="versions.loading"
                :disabled="creating"
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

              <label class="field-label" style="margin-top: 16px" for="instance-group">{{ t('instances.groupLabel') }}</label>
              <input
                id="instance-group"
                v-model="groupName"
                class="field"
                :placeholder="t('instances.groupPlaceholder')"
                maxlength="48"
                @keyup.enter="submit"
              />
            </template>

            <template v-else>
              <div class="import-submode-row" style="margin-top: 4px">
                <button
                  type="button"
                  class="btn btn-sm"
                  :class="importSubMode === 'browse' ? 'btn-mineral' : 'btn-ghost'"
                  @click="importSubMode = 'browse'"
                >
                  {{ t('instances.importBrowse') }}
                </button>
                <button
                  type="button"
                  class="btn btn-sm"
                  :class="importSubMode === 'file' ? 'btn-mineral' : 'btn-ghost'"
                  @click="importSubMode = 'file'"
                >
                  {{ t('instances.importFromFile') }}
                </button>
              </div>

              <label class="field-label" style="margin-top: 14px" for="instance-name-import">{{ t('instances.nameLabel') }}</label>
              <input
                id="instance-name-import"
                v-model="name"
                class="field"
                :placeholder="t('instances.namePlaceholder')"
                maxlength="48"
              />

              <label class="field-label" style="margin-top: 12px" for="instance-group-import">{{ t('instances.groupLabel') }}</label>
              <input
                id="instance-group-import"
                v-model="groupName"
                class="field"
                :placeholder="t('instances.groupPlaceholder')"
                maxlength="48"
              />

              <template v-if="importSubMode === 'browse'">
                <div class="source-picker" style="margin-top: 12px">
                  <button
                    type="button"
                    class="source-opt"
                    :class="{ selected: packSource === 'modrinth' }"
                    @click="selectPackSource('modrinth')"
                  >
                    Modrinth
                  </button>
                  <button
                    type="button"
                    class="source-opt source-curseforge"
                    :class="{ selected: packSource === 'curseforge' }"
                    :disabled="!curseforgeAvailable"
                    v-tooltip="!curseforgeAvailable ? t('content.curseforgeUnavailableHint') : ''"
                    @click="selectPackSource('curseforge')"
                  >
                    CurseForge
                  </button>
                </div>

                <div class="search-box" style="margin-top: 10px">
                  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                    <circle cx="11" cy="11" r="7" /><path d="M21 21l-4.3-4.3" />
                  </svg>
                  <input v-model="packQuery" :placeholder="t('instances.importSearchPlaceholder')" />
                </div>
                <p v-if="packSearchError" class="hint hint-warn" style="margin-top: 8px">{{ packSearchError }}</p>

                <div class="field-block" style="margin-top: 12px">
                  <div class="field-label">{{ t('content.sortByLabel') }}</div>
                  <GlassSelect v-model="packSortKey" :options="packSortOptions" style="width: 100%" />
                </div>

                <div class="field-block" style="margin-top: 12px">
                  <div class="field-label">{{ t('version.mcVersionLabel') }}</div>
                  <VersionSelect
                    :model-value="selectedVersion"
                    :groups="versions.groupedSelectableVersions"
                    :loading="versions.loading"
                    compact
                    style="width: 100%"
                    @update:model-value="selectPackVersionFilter"
                  />
                </div>

                <div class="field-block categories-block" style="margin-top: 12px">
                  <div class="field-label">{{ t('content.categoriesLabel') }}</div>
                  <div v-if="loadingPackCategories" class="hint">{{ t('content.searching') }}</div>
                  <div v-else-if="packCategories.length === 0" class="hint">{{ t('content.noCategories') }}</div>
                  <div v-else class="category-groups">
                    <div v-for="group in groupedPackCategories" :key="group.label || '__flat'" class="category-group">
                      <h5 v-if="group.label && groupedPackCategories.length > 1">{{ group.label }}</h5>
                      <label v-for="cat in group.items" :key="cat.id" class="checkbox-row">
                        <input type="checkbox" :value="cat.id" v-model="selectedPackCategories" />
                        {{ cat.name }}
                      </label>
                    </div>
                  </div>
                </div>
              </template>

              <button v-else class="btn btn-ghost btn-block" type="button" style="margin-top: 12px" @click="pickFile">
                {{ t('instances.importChooseFile') }}
              </button>
            </template>
          </div>

          <div v-if="mode === 'new'" class="create-main">
            <div class="create-preview">
              <img :src="previewPanoramaUrl" :key="previewPanoramaUrl" alt="" decoding="async" />
              <div class="create-preview-fade"></div>
              <div class="create-preview-info">
                <div class="create-preview-tags">
                  <span class="pill">{{ t(`loaders.${loader}`) }}</span>
                  <span class="pill mono">{{ selectedVersion || '…' }}</span>
                </div>
                <h4>{{ previewName }}</h4>
              </div>
            </div>

            <div v-if="error" class="error-box">{{ error }}</div>

            <div class="panel version-panel">
              <h4>{{ t('version.loaderHeading') }}</h4>
              <p class="hint">{{ t('instances.loaderHint') }}</p>
              <div class="loader-list">
                <div
                  v-for="key in LOADER_KEYS"
                  :key="key"
                  class="loader-row"
                  :class="{ selected: loader === key, disabled: loaderDisabled(key) || creating }"
                  :tabindex="loaderDisabled(key) || creating ? -1 : 0"
                  role="button"
                  @click="handleLoaderRowClick(key)"
                  @keydown.enter="handleLoaderRowClick(key)"
                  @keydown.space.prevent="handleLoaderRowClick(key)"
                >
                  <span class="loader-avatar" :class="{ 'loader-avatar-full': loader === key }">
                    <img v-if="loader === key" :src="loaderFullIconUrl(key)" alt="" class="loader-avatar-img" />
                    <span
                      v-else
                      class="loader-avatar-icon"
                      :style="{ maskImage: `url(/loaders/${key}.svg)`, webkitMaskImage: `url(/loaders/${key}.svg)` }"
                    ></span>
                  </span>
                  <span class="loader-name">{{ t(`loaders.${key}`) }}</span>
                  <svg v-if="loader === key" class="loader-check" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3">
                    <path d="M20 6L9 17l-5-5" />
                  </svg>
                  <GlassSelect
                    v-if="loader === key && key !== 'vanilla'"
                    :model-value="loaderVersion"
                    :options="loaderVersionOptions"
                    :disabled="creating || loadingLoaderVersions"
                    class="loader-row-select"
                    @click.stop
                    @update:model-value="(v) => (loaderVersion = v)"
                  />
                  <span v-else class="loader-status">{{ loaderStatusLabel(key) }}</span>
                </div>
              </div>
            </div>

            <button
              class="btn btn-mineral btn-block"
              type="button"
              style="margin-top: 18px"
              :disabled="creating || !canCreate"
              @click="submit"
            >
              <span v-if="creating" class="spinner"></span>
              {{ creating ? t('instances.creating') : t('instances.create') }}
            </button>
          </div>

          <div v-else class="create-main">
            <div v-if="error" class="error-box">{{ error }}</div>

            <template v-if="importSubMode === 'browse'">
              <template v-if="!selectedPack">
                <div v-if="packSearching" class="empty-state">{{ t('content.searching') }}</div>
                <div v-else-if="packResults.length === 0" class="empty-state">{{ t('content.noResults') }}</div>
                <template v-else>
                  <div
                    v-for="hit in packResults"
                    :key="packHitKey(hit)"
                    class="row-card select-mode"
                    @click="selectPack(hit)"
                  >
                    <img v-if="hit.iconUrl" :src="hit.iconUrl" class="row-icon" alt="" loading="lazy" decoding="async" />
                    <div v-else class="row-icon">{{ firstGrapheme(hit.title).toUpperCase() }}</div>
                    <div class="row-info">
                      <div class="rtitle">
                        <h4>{{ hit.title }}</h4>
                        <span class="src-badge" :class="`src-${hit.source}`">{{ hit.source === 'curseforge' ? 'CurseForge' : 'Modrinth' }}</span>
                      </div>
                      <p v-if="hit.author" class="row-author">{{ t('content.byAuthor', { author: hit.author }) }}</p>
                      <p class="row-desc">{{ hit.description }}</p>
                    </div>
                  </div>
                  <div v-if="packHasMore" class="load-more-row">
                    <button class="btn btn-ghost btn-sm" type="button" :disabled="packLoadingMore" @click="loadMorePacks">
                      {{ packLoadingMore ? t('content.loadingMore') : t('content.loadMore') }}
                    </button>
                  </div>
                </template>
              </template>

              <template v-else>
                <div class="row-card selected">
                  <img v-if="selectedPack.iconUrl" :src="selectedPack.iconUrl" class="row-icon" alt="" />
                  <div v-else class="row-icon">{{ firstGrapheme(selectedPack.title).toUpperCase() }}</div>
                  <div class="row-info">
                    <h4>{{ selectedPack.title }}</h4>
                    <p v-if="selectedPack.author" class="row-author">{{ t('content.byAuthor', { author: selectedPack.author }) }}</p>
                  </div>
                  <div class="row-actions">
                    <button class="btn btn-ghost btn-sm" type="button" @click="selectedPack = null">
                      {{ t('instances.importChangePack') }}
                    </button>
                  </div>
                </div>

                <label class="field-label" style="margin-top: 16px">{{ t('instances.importVersionLabel') }}</label>
                <div v-if="loadingPackVersions" class="hint">{{ t('version.checking') }}</div>
                <GlassSelect
                  v-else
                  :model-value="selectedPackVersion?.versionId"
                  :options="packVersions.map((v) => ({ value: v.versionId, label: v.versionNumber }))"
                  style="width: 100%"
                  @update:model-value="(id) => (selectedPackVersion = packVersions.find((v) => v.versionId === id))"
                />
              </template>
            </template>

            <template v-else>
              <div v-if="pickedFilePath" class="row-card">
                <div class="row-icon">{{ firstGrapheme(name || 'M').toUpperCase() }}</div>
                <div class="row-info">
                  <h4>{{ name || pickedFilePath }}</h4>
                  <p class="row-author">{{ pickedFilePath }}</p>
                </div>
              </div>
              <div v-else class="empty-state">{{ t('instances.importChooseFileHint') }}</div>
            </template>

            <p v-if="installProgress" class="hint" style="margin-top: 12px">
              {{ t('instances.installing') }} ({{ installProgress.completed }}/{{ installProgress.total }})
            </p>

            <button
              class="btn btn-mineral btn-block"
              type="button"
              style="margin-top: 18px"
              :disabled="creating || !canInstallPack"
              @click="installPack"
            >
              <span v-if="creating" class="spinner"></span>
              {{ creating ? t('instances.creating') : t('instances.installPack') }}
            </button>
          </div>
        </div>
      </div>
    </div>
  </Teleport>
</template>
