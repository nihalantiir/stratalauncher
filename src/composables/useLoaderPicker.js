import { ref, computed } from 'vue';
import { listLoaderVersions, checkLoaderAvailability } from '../api/loaders';

export const LOADER_KEYS = ['vanilla', 'fabric', 'forge', 'neoforge', 'quilt'];

// Shared loader/loader-version selection state and logic behind the
// Version page's Apply/Discard flow and the New Instance modal's picker.
export function useLoaderPicker({ t, isBusy, error }) {
  const loader = ref('vanilla');
  const loaderVersion = ref(null);
  const loaderVersions = ref([]);
  const availability = ref({});
  const loadingLoaderVersions = ref(false);

  const loaderVersionOptions = computed(() =>
    loaderVersions.value.map((lv) => ({ value: lv.version, label: `${lv.version}${lv.stable ? '' : ' (beta)'}` })),
  );

  async function refreshAvailability(mcVersion) {
    if (!mcVersion) return;
    try {
      const list = await checkLoaderAvailability(mcVersion);
      const map = {};
      for (const entry of list) map[entry.loader] = entry;
      availability.value = map;
    } catch {
      // Non-fatal: loaders just render without a build-count/disabled state.
    }
  }

  async function refreshLoaderVersions(key, mcVersion) {
    if (key === 'vanilla' || !mcVersion) {
      loaderVersions.value = [];
      return;
    }
    loadingLoaderVersions.value = true;
    try {
      loaderVersions.value = await listLoaderVersions(key, mcVersion);
    } catch (e) {
      if (error) error.value = String(e);
      loaderVersions.value = [];
    } finally {
      loadingLoaderVersions.value = false;
    }
  }

  async function selectLoader(key, mcVersion) {
    if (isBusy() || key === loader.value) return;
    if (error) error.value = null;
    loader.value = key;
    if (key === 'vanilla') {
      loaderVersion.value = null;
      loaderVersions.value = [];
      return;
    }
    await refreshLoaderVersions(key, mcVersion);
    loaderVersion.value = loaderVersions.value[0]?.version ?? null;
  }

  function loaderDisabled(key) {
    if (key === 'vanilla' || key === loader.value) return false;
    const entry = availability.value[key];
    return !!entry && !entry.available;
  }

  function loaderStatusLabel(key, mcVersion) {
    if (key === loader.value) return t('version.selected');
    if (key === 'vanilla') return '';
    const entry = availability.value[key];
    if (!entry) return t('version.checking');
    if (!entry.available) return t('version.notAvailable', { mcVersion });
    return `${entry.buildCount} ${t('version.buildsLabel')}`;
  }

  // Same real full-color/outline icon system every loader list uses (see
  // public/loaders/ + its NOTICE.md).
  function loaderFullIconUrl(key) {
    return key === 'vanilla' ? '/grass-block.png' : `/loaders/full/${key}.png`;
  }

  return {
    loader,
    loaderVersion,
    loaderVersions,
    loaderVersionOptions,
    availability,
    loadingLoaderVersions,
    refreshAvailability,
    refreshLoaderVersions,
    selectLoader,
    loaderDisabled,
    loaderStatusLabel,
    loaderFullIconUrl,
  };
}
