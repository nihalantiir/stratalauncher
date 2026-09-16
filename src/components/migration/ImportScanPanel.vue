<script setup>
import { ref, onMounted, computed } from 'vue';
import { useI18n } from 'vue-i18n';
import { open } from '@tauri-apps/plugin-dialog';
import { useInstancesStore } from '../../stores/instances';
import { BIOME_KEYS } from '../../biomes';
import * as api from '../../api/migration';
import { firstGrapheme, formatSize } from '../../lib/text';

const emit = defineEmits(['done']);
const { t } = useI18n();
const instances = useInstancesStore();

const KINDS = ['vanilla', 'curseforge', 'prism', 'multimc'];

const scanning = ref(true);
const detected = ref([]);
const error = ref(null);
const importingKey = ref(null);
const importedKeys = ref(new Set());

const keyFor = (d) => `${d.kind}:${d.path}`;

const grouped = computed(() => {
  const g = { vanilla: [], curseforge: [], prism: [], multimc: [] };
  for (const d of detected.value) g[d.kind]?.push(d);
  return g;
});

async function scan() {
  scanning.value = true;
  error.value = null;
  try {
    detected.value = await api.scanAllInstalls();
  } catch (e) {
    error.value = String(e);
  } finally {
    scanning.value = false;
  }
}

onMounted(scan);

async function importOne(d) {
  importingKey.value = keyFor(d);
  error.value = null;
  try {
    await api.importInstance({
      sourcePath: d.path,
      name: d.name,
      mcVersion: d.mcVersion,
      loader: d.loader,
      loaderVersion: d.loaderVersion,
      iconBiome: BIOME_KEYS[Math.floor(Math.random() * BIOME_KEYS.length)],
    });
    importedKeys.value.add(keyFor(d));
    await instances.refresh();
  } catch (e) {
    error.value = String(e);
  } finally {
    importingKey.value = null;
  }
}

async function browseFor(kind) {
  const folder = await open({ directory: true });
  if (!folder) return;
  scanning.value = true;
  error.value = null;
  try {
    const found = await api.scanCustomFolder(kind, folder);
    detected.value = [...detected.value.filter((d) => d.kind !== kind), ...found];
  } catch (e) {
    error.value = String(e);
  } finally {
    scanning.value = false;
  }
}

</script>

<template>
  <div>
    <div v-if="error" class="error-box">{{ error }}</div>
    <div v-if="scanning" class="empty-state">{{ t('import.scanning') }}</div>

    <template v-else>
      <div v-for="kind in KINDS" :key="kind" class="import-group">
        <div class="import-group-head">
          <h4>{{ t(`import.kind.${kind}`) }}</h4>
          <button class="btn btn-ghost btn-sm" type="button" @click="browseFor(kind)">{{ t('import.browse') }}</button>
        </div>

        <div v-if="grouped[kind].length === 0" class="empty-state" style="padding: 12px">
          {{ t('import.noneFound') }}
        </div>
        <div v-for="d in grouped[kind]" :key="keyFor(d)" class="row-card">
          <div class="row-icon">{{ firstGrapheme(d.name).toUpperCase() }}</div>
          <div class="row-info">
            <div class="rtitle">
              <h4>{{ d.name }}</h4>
              <span class="tag mono">{{ d.mcVersion }}</span>
              <span v-if="d.loader !== 'vanilla'" class="tag">{{ t(`loaders.${d.loader}`) }}</span>
            </div>
            <p>{{ formatSize(d.sizeBytes) }} · {{ d.path }}</p>
          </div>
          <div class="row-actions">
            <button v-if="importedKeys.has(keyFor(d))" class="btn btn-installed btn-sm" type="button" disabled>
              {{ t('import.imported') }}
            </button>
            <button
              v-else
              class="btn btn-mineral btn-sm"
              type="button"
              :disabled="importingKey === keyFor(d)"
              @click="importOne(d)"
            >
              {{ importingKey === keyFor(d) ? t('import.importing') : t('import.importAction') }}
            </button>
          </div>
        </div>
      </div>
    </template>
  </div>
</template>
