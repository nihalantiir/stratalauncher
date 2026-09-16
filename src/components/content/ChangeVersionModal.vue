<script setup>
import { ref, onMounted } from 'vue';
import { useI18n } from 'vue-i18n';
import * as api from '../../api/content';

const props = defineProps({
  instanceId: { type: String, required: true },
  kind: { type: String, required: true },
  item: { type: Object, required: true },
});
const emit = defineEmits(['close', 'changed']);
const { t } = useI18n();

const versions = ref([]);
const loading = ref(false);
const error = ref(null);
const installingId = ref(null);

async function refresh() {
  loading.value = true;
  error.value = null;
  try {
    versions.value = await api.listContentVersions(props.item.source ?? 'modrinth', props.item.projectId);
  } catch (e) {
    error.value = String(e);
  } finally {
    loading.value = false;
  }
}
onMounted(refresh);

function formatDate(iso) {
  return new Date(iso).toLocaleDateString();
}

function tagsFor(v) {
  return [...v.gameVersions, ...v.loaders].join(', ');
}

async function selectVersion(version) {
  if (version.versionId === props.item.versionId || installingId.value) return;
  installingId.value = version.versionId;
  error.value = null;
  try {
    const newItem = await api.installContentVersion(
      props.instanceId,
      props.kind,
      props.item.source ?? 'modrinth',
      props.item.projectId,
      props.item.title,
      props.item.iconUrl,
      version,
    );
    if (newItem.filename !== props.item.filename) {
      await api.removeContent(props.instanceId, props.kind, props.item.filename);
    }
    emit('changed');
    emit('close');
  } catch (e) {
    error.value = String(e);
    installingId.value = null;
  }
}
</script>

<template>
  <Teleport to="body">
    <div class="modal-backdrop" @click.self="emit('close')">
      <div class="modal" style="max-width: 480px">
        <div class="modal-head">
          <h3>{{ t('content.changeVersionFor', { name: item.title }) }}</h3>
          <button class="modal-close" type="button" @click="emit('close')">✕</button>
        </div>
        <div class="modal-body">
          <div v-if="error" class="error-box">{{ error }}</div>
          <div v-if="loading" class="hint">{{ t('version.checking') }}</div>
          <div v-else-if="versions.length === 0" class="empty-state">{{ t('content.noVersions') }}</div>

          <div v-for="v in versions" :key="v.versionId" class="dropdown-item" style="cursor: default; align-items: center">
            <div style="flex: 1; min-width: 0">
              <div style="display: flex; align-items: center; gap: 6px; flex-wrap: wrap">
                <p style="margin: 0">{{ v.versionNumber }}</p>
                <span class="version-badge" :class="`version-${v.versionType}`">{{ v.versionType }}</span>
                <span v-if="v.versionId === item.versionId" class="tag tag-accent">{{ t('content.currentVersion') }}</span>
              </div>
              <small>{{ tagsFor(v) }} · {{ formatDate(v.datePublished) }}</small>
            </div>
            <button
              v-if="v.versionId !== item.versionId"
              class="btn btn-ghost btn-sm"
              type="button"
              :disabled="!!installingId"
              @click="selectVersion(v)"
            >
              {{ installingId === v.versionId ? t('content.installing') : t('content.useVersion') }}
            </button>
          </div>
        </div>
      </div>
    </div>
  </Teleport>
</template>
