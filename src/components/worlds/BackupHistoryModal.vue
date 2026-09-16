<script setup>
import { ref, onMounted } from 'vue';
import { useI18n } from 'vue-i18n';
import * as api from '../../api/worlds';
import { formatSize } from '../../lib/text';

const props = defineProps({
  instanceId: { type: String, required: true },
  folder: { type: String, required: true },
  worldName: { type: String, required: true },
});
const emit = defineEmits(['close', 'changed']);
const { t } = useI18n();

const backups = ref([]);
const loading = ref(false);
const error = ref(null);
const confirming = ref({});
const busyFile = ref(null);

async function refresh() {
  loading.value = true;
  error.value = null;
  try {
    backups.value = await api.listBackups(props.instanceId, props.folder);
  } catch (e) {
    error.value = String(e);
  } finally {
    loading.value = false;
  }
}

onMounted(refresh);

async function doRestore(file) {
  busyFile.value = file;
  try {
    await api.restoreBackup(props.instanceId, props.folder, file);
    confirming.value = { ...confirming.value, [file]: null };
    emit('changed');
  } catch (e) {
    error.value = String(e);
  } finally {
    busyFile.value = null;
  }
}

async function doDelete(file) {
  busyFile.value = file;
  try {
    await api.deleteBackup(props.instanceId, file);
    confirming.value = { ...confirming.value, [file]: null };
    await refresh();
    emit('changed');
  } catch (e) {
    error.value = String(e);
  } finally {
    busyFile.value = null;
  }
}
</script>

<template>
  <Teleport to="body">
    <div class="modal-backdrop" @click.self="emit('close')">
      <div class="modal" style="max-width: 460px">
        <div class="modal-head">
          <h3>{{ t('worlds.backupHistory', { name: worldName }) }}</h3>
          <button class="modal-close" type="button" @click="emit('close')">✕</button>
        </div>
        <div class="modal-body">
          <div v-if="error" class="error-box">{{ error }}</div>
          <div v-if="!loading && backups.length === 0" class="empty-state">{{ t('worlds.noBackups') }}</div>

          <div v-for="b in backups" :key="b.file" class="dropdown-item" style="cursor: default; align-items: center">
            <div style="flex: 1; min-width: 0">
              <p>{{ new Date(b.createdAt).toLocaleString() }}</p>
              <small>{{ formatSize(b.sizeBytes) }}</small>
            </div>
            <div class="row-actions" v-if="!confirming[b.file]">
              <button class="btn btn-ghost btn-sm" type="button" @click="confirming = { ...confirming, [b.file]: 'restore' }">
                {{ t('worlds.restore') }}
              </button>
              <button
                class="btn btn-danger-ghost btn-sm"
                type="button"
                @click="confirming = { ...confirming, [b.file]: 'delete' }"
              >
                {{ t('worlds.deleteBackup') }}
              </button>
            </div>
            <div class="row-actions" v-else>
              <button
                class="btn btn-ghost btn-sm"
                type="button"
                :disabled="busyFile === b.file"
                @click="confirming = { ...confirming, [b.file]: null }"
              >
                {{ t('instances.cancel') }}
              </button>
              <button
                v-if="confirming[b.file] === 'restore'"
                class="btn btn-danger-ghost btn-sm"
                type="button"
                :disabled="busyFile === b.file"
                @click="doRestore(b.file)"
              >
                {{ busyFile === b.file ? t('worlds.restoring') : t('worlds.confirmRestore') }}
              </button>
              <button v-else class="btn btn-danger-ghost btn-sm" type="button" :disabled="busyFile === b.file" @click="doDelete(b.file)">
                {{ busyFile === b.file ? t('worlds.deleting') : t('worlds.confirmDeleteBackup') }}
              </button>
            </div>
          </div>
        </div>
      </div>
    </div>
  </Teleport>
</template>
