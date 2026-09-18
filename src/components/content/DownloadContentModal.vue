<script setup>
import { ref, computed, watch, onMounted } from 'vue';
import { useI18n } from 'vue-i18n';
import * as api from '../../api/content';
import { firstGrapheme } from '../../lib/text';
import { useInstancesStore } from '../../stores/instances';
import { useVersionsStore } from '../../stores/versions';
import GlassSelect from '../common/GlassSelect.vue';
import VersionSelect from '../version/VersionSelect.vue';
import { useCatalogSearch } from '../../composables/useCatalogSearch';

const props = defineProps({
  instanceId: { type: String, required: true },
  kind: { type: String, required: true },
  installedProjectIds: { type: Array, default: () => [] },
});
const emit = defineEmits(['close', 'installed']);
const { t } = useI18n();
const instances = useInstancesStore();
const versions = useVersionsStore();

const currentInstance = computed(() => instances.list.find((i) => i.id === props.instanceId));

const installingKey = ref(null);
const installedKeys = ref(new Set(props.installedProjectIds.map((id) => `modrinth:${id}`)));
// { title, extra: InstalledItem[] } for the "required libraries installed
// too" popup, set right after an install that pulled in dependencies.
const dependencyResult = ref(null);
const curseforgeAvailable = ref(false);
// A display-only filter over whatever's already been fetched, not a search
// facet, so no re-fetch is needed when it's toggled.
const hideInstalled = ref(false);

// Defaults to this instance's own Minecraft version; picking a different one is
// now what "search/install for a version other than this instance's own" means, instead of a plain ignore-compatibility toggle.
const selectedVersion = ref(currentInstance.value?.mcVersion ?? '');

const {
  source,
  query,
  results,
  searching: loading,
  loadingMore,
  searchError: error,
  hasMore,
  categories,
  selectedCategories,
  loadingCategories,
  sortKey,
  sortOptions,
  groupedCategories,
  hitKey,
  runSearch,
  loadMore,
  loadCategories,
  selectSource: selectSourceCatalog,
} = useCatalogSearch({
  t,
  fetchPage: (offset) =>
    api.searchContent(props.instanceId, props.kind, source.value, query.value, sortKey.value, selectedCategories.value, selectedVersion.value, offset),
  fetchCategories: (src) => api.listContentCategories(src, props.kind),
});

const visibleResults = computed(() =>
  hideInstalled.value ? results.value.filter((hit) => !installedKeys.value.has(hitKey(hit))) : results.value,
);

function selectSource(key) {
  return selectSourceCatalog(key, curseforgeAvailable.value);
}

watch(selectedVersion, runSearch);

onMounted(async () => {
  if (!versions.manifest) await versions.fetch();
  if (!selectedVersion.value) selectedVersion.value = currentInstance.value?.mcVersion ?? '';
  loadCategories();
  runSearch();
  try {
    curseforgeAvailable.value = await api.curseforgeConfigured();
  } catch {
    curseforgeAvailable.value = false;
  }
});

async function install(hit) {
  installingKey.value = hitKey(hit);
  error.value = null;
  try {
    const result = await api.installContent(
      props.instanceId,
      props.kind,
      hit.source,
      hit.projectId,
      hit.title,
      hit.iconUrl,
      selectedVersion.value,
    );
    installedKeys.value.add(hitKey(hit));
    emit('installed', result.item);
    if (result.extraInstalled?.length) {
      dependencyResult.value = { title: hit.title, extra: result.extraInstalled };
    }
  } catch (e) {
    error.value = String(e);
  } finally {
    installingKey.value = null;
  }
}

function formatDownloads(n) {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
  if (n >= 1_000) return `${Math.round(n / 1000)}K`;
  return String(n);
}

// Modrinth-only (`hit.clientSide`/`hit.serverSide`); CurseForge's API has no
// equivalent field, so a CurseForge hit always resolves to null and shows no badge instead of a fake "unknown" one.
function sideLabel(hit) {
  const needsClient = hit.clientSide === 'required' || hit.clientSide === 'optional';
  const needsServer = hit.serverSide === 'required' || hit.serverSide === 'optional';
  if (needsClient && needsServer) return t('content.sideBoth');
  if (needsClient) return t('content.sideClient');
  if (needsServer) return t('content.sideServer');
  return null;
}
</script>

<template>
  <Teleport to="body">
  <div class="modal-backdrop" @click.self="emit('close')">
    <div class="modal downloader-modal">
      <div class="modal-head">
        <h3>{{ t(`content.downloadTitle.${kind}`) }}</h3>
        <button class="modal-close" type="button" @click="emit('close')">✕</button>
      </div>
      <div class="modal-body downloader-body">
        <aside class="downloader-sidebar">
          <div class="field-block">
            <div class="field-label">{{ t('content.sourceLabel') }}</div>
            <div class="source-picker">
              <button
                type="button"
                class="source-opt"
                :class="{ selected: source === 'modrinth' }"
                @click="selectSource('modrinth')"
              >
                Modrinth
              </button>
              <button
                type="button"
                class="source-opt source-curseforge"
                :class="{ selected: source === 'curseforge' }"
                :disabled="!curseforgeAvailable"
                v-tooltip="!curseforgeAvailable ? t('content.curseforgeUnavailableHint') : ''"
                @click="selectSource('curseforge')"
              >
                CurseForge
              </button>
            </div>
          </div>

          <div class="field-block">
            <div class="field-label">{{ t('content.sortByLabel') }}</div>
            <GlassSelect v-model="sortKey" :options="sortOptions" style="width: 100%" />
          </div>

          <div class="field-block">
            <div class="field-label">{{ t('content.versionLabel') }}</div>
            <VersionSelect
              :model-value="selectedVersion"
              :groups="versions.groupedSelectableVersions"
              :loading="versions.loading"
              compact
              style="width: 100%"
              @update:model-value="(v) => (selectedVersion = v)"
            />
            <p v-if="selectedVersion !== currentInstance?.mcVersion" class="hint hint-warn">
              {{ t('content.differentVersionHint') }}
            </p>
          </div>

          <div class="field-block categories-block">
            <div class="field-label">{{ t('content.categoriesLabel') }}</div>
            <div v-if="loadingCategories" class="hint">{{ t('content.searching') }}</div>
            <div v-else-if="categories.length === 0" class="hint">{{ t('content.noCategories') }}</div>
            <div v-else class="category-groups">
              <div v-for="group in groupedCategories" :key="group.label || '__flat'" class="category-group">
                <h5 v-if="group.label && groupedCategories.length > 1">{{ group.label }}</h5>
                <label v-for="cat in group.items" :key="cat.id" class="checkbox-row">
                  <input type="checkbox" :value="cat.id" v-model="selectedCategories" />
                  {{ cat.name }}
                </label>
              </div>
            </div>
          </div>
        </aside>

        <div class="downloader-results">
          <div class="search-row">
            <div class="search-box">
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <circle cx="11" cy="11" r="7" /><path d="M21 21l-4.3-4.3" />
              </svg>
              <input v-model="query" :placeholder="t('content.searchPlaceholder', { source: source === 'curseforge' ? 'CurseForge' : 'Modrinth' })" />
            </div>
            <label class="checkbox-row hide-installed-check">
              <input type="checkbox" v-model="hideInstalled" />
              {{ t('content.hideInstalledLabel') }}
            </label>
          </div>

          <div v-if="error" class="error-box">{{ error }}</div>

          <div v-if="loading" class="empty-state">{{ t('content.searching') }}</div>
          <div v-else-if="visibleResults.length === 0" class="empty-state">{{ t('content.noResults') }}</div>

          <div v-for="hit in visibleResults" :key="hitKey(hit)" class="row-card">
            <img v-if="hit.iconUrl" :src="hit.iconUrl" class="row-icon" alt="" loading="lazy" decoding="async" />
            <div v-else class="row-icon">{{ firstGrapheme(hit.title).toUpperCase() }}</div>
            <div class="row-info">
              <div class="rtitle">
                <h4>{{ hit.title }}</h4>
                <span class="src-badge" :class="`src-${hit.source}`">{{ hit.source === 'curseforge' ? 'CurseForge' : 'Modrinth' }}</span>
                <span v-if="sideLabel(hit)" class="side-badge">{{ sideLabel(hit) }}</span>
              </div>
              <p v-if="hit.author" class="row-author">{{ t('content.byAuthor', { author: hit.author }) }}</p>
              <p class="row-desc">{{ hit.description }}</p>
            </div>
            <div class="row-meta">{{ formatDownloads(hit.downloads) }} {{ t('content.downloadsLabel') }}</div>
            <div class="row-actions">
              <button
                v-if="installedKeys.has(hitKey(hit))"
                class="btn btn-installed btn-sm"
                type="button"
                disabled
              >
                {{ t('content.installed') }}
              </button>
              <button
                v-else
                class="btn btn-mineral btn-sm"
                type="button"
                :disabled="installingKey === hitKey(hit)"
                @click="install(hit)"
              >
                <span v-if="installingKey === hitKey(hit)" class="spinner"></span>
                {{ installingKey === hitKey(hit) ? t('content.installing') : t('content.install') }}
              </button>
            </div>
          </div>

          <div v-if="hasMore && !loading" class="load-more-row">
            <button class="btn btn-ghost btn-sm" type="button" :disabled="loadingMore" @click="loadMore">
              {{ loadingMore ? t('content.loadingMore') : t('content.loadMore') }}
            </button>
          </div>
        </div>
      </div>
    </div>
  </div>
  </Teleport>

  <Teleport to="body">
    <div v-if="dependencyResult" class="modal-backdrop" @click.self="dependencyResult = null">
      <div class="modal">
        <div class="modal-head">
          <h3>{{ t('content.dependenciesInstalledTitle') }}</h3>
          <button class="modal-close" type="button" @click="dependencyResult = null">✕</button>
        </div>
        <div class="modal-body">
          <p style="margin: 0 0 10px">
            {{ t('content.dependenciesInstalledBody', { title: dependencyResult.title, count: dependencyResult.extra.length }) }}
          </p>
          <ul class="dependency-list">
            <li v-for="dep in dependencyResult.extra" :key="dep.filename">{{ dep.title }}</li>
          </ul>
          <button class="btn btn-mineral btn-block" type="button" style="margin-top: 16px" @click="dependencyResult = null">
            {{ t('content.close') }}
          </button>
        </div>
      </div>
    </div>
  </Teleport>
</template>
