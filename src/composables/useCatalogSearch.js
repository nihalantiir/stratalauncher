import { ref, computed, watch } from 'vue';
import { SORT_LABEL_KEYS, sortsForSource, defaultSortForSource } from '../lib/contentSort';

const DEBOUNCE_MS = 350;

// Shared search/filter/pagination state behind the mod/resourcepack/shader
// downloader and the New Instance modal's modpack browser. `fetchPage` and
// `fetchCategories` are supplied by the caller since the two callers hit
// different backend commands with different extra params (instance id/kind
// vs. nothing).
export function useCatalogSearch({ t, pageSize = 30, fetchPage, fetchCategories, initialSource = 'modrinth' }) {
  const source = ref(initialSource);
  const query = ref('');
  const results = ref([]);
  const searching = ref(false);
  const loadingMore = ref(false);
  const searchError = ref(null);
  const offset = ref(0);
  const hasMore = ref(false);
  const categories = ref([]);
  const selectedCategories = ref([]);
  const loadingCategories = ref(false);
  const sortKey = ref('relevance');

  const sortOptions = computed(() => sortsForSource(source.value).map((key) => ({ value: key, label: t(SORT_LABEL_KEYS[key]) })));

  // Grouped for Modrinth (its own "categories"/"features"/"resolutions"/
  // "performance impact" sections); CurseForge's list is flat, one unlabeled group.
  const groupedCategories = computed(() => {
    const groups = new Map();
    for (const cat of categories.value) {
      const key = cat.group ?? '';
      if (!groups.has(key)) groups.set(key, []);
      groups.get(key).push(cat);
    }
    return [...groups.entries()].map(([label, items]) => ({ label, items }));
  });

  function hitKey(hit) {
    return `${hit.source}:${hit.projectId}`;
  }

  let searchToken = 0;

  async function runSearch() {
    searching.value = true;
    searchError.value = null;
    offset.value = 0;
    const token = ++searchToken;
    try {
      const hits = await fetchPage(0);
      if (token !== searchToken) return; // a newer search already superseded this one
      results.value = hits;
      hasMore.value = hits.length >= pageSize;
    } catch (e) {
      if (token !== searchToken) return;
      searchError.value = String(e);
    } finally {
      if (token === searchToken) searching.value = false;
    }
  }

  async function loadMore() {
    loadingMore.value = true;
    searchError.value = null;
    const token = searchToken;
    try {
      const nextOffset = offset.value + pageSize;
      const hits = await fetchPage(nextOffset);
      if (token !== searchToken) return;
      results.value.push(...hits);
      offset.value = nextOffset;
      hasMore.value = hits.length >= pageSize;
    } catch (e) {
      if (token === searchToken) searchError.value = String(e);
    } finally {
      if (token === searchToken) loadingMore.value = false;
    }
  }

  let debounceHandle = null;
  watch(query, () => {
    clearTimeout(debounceHandle);
    debounceHandle = setTimeout(runSearch, DEBOUNCE_MS);
  });
  // Filter/sort changes are discrete, low-frequency events (not
  // per-keystroke) so search right away rather than debouncing those too.
  watch([sortKey, selectedCategories], runSearch, { deep: true });

  async function loadCategories() {
    loadingCategories.value = true;
    selectedCategories.value = [];
    try {
      categories.value = await fetchCategories(source.value);
    } catch {
      categories.value = []; // e.g. CurseForge picked without a key configured
    } finally {
      loadingCategories.value = false;
    }
  }

  async function selectSource(key, curseforgeAvailable) {
    if (key === source.value || (key === 'curseforge' && !curseforgeAvailable)) return;
    source.value = key;
    sortKey.value = defaultSortForSource(key);
    await loadCategories();
    runSearch();
  }

  return {
    source,
    query,
    results,
    searching,
    loadingMore,
    searchError,
    offset,
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
    selectSource,
  };
}
