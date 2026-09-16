<script setup>
import { computed } from 'vue';
import { useI18n } from 'vue-i18n';
import { useVersionGroupLabel } from '../../composables/useVersionGroupLabel';
import { useFloatingDropdown } from '../../composables/useFloatingDropdown';

const props = defineProps({
  modelValue: { type: String, default: '' },
  groups: { type: Array, required: true },
  loading: { type: Boolean, default: false },
  disabled: { type: Boolean, default: false },
  id: { type: String, default: undefined },
  // The closed trigger normally shows "World of Color (1.12)", useful on the
  // Version page; too wide for a narrow sidebar field where the plain number is enough.
  compact: { type: Boolean, default: false },
});
const emit = defineEmits(['update:modelValue']);
const { t } = useI18n();
const groupLabel = useVersionGroupLabel();
const { open, rootEl, triggerEl, ddEl, ddStyle, toggle, close } = useFloatingDropdown();

const selectedInfo = computed(() => {
  for (const group of props.groups) {
    const version = group.versions.find((v) => v.id === props.modelValue);
    if (version) return { group, version };
  }
  return null;
});

// "World of Color (1.12)": the format the version selector shows the
// current pick in, everywhere it's used.
const displayText = computed(() => {
  if (props.loading) return t('versions.loading');
  if (!selectedInfo.value) return props.modelValue || '';
  if (props.compact) return selectedInfo.value.version.id;
  return `${groupLabel(selectedInfo.value.group)} (${selectedInfo.value.version.id})`;
});

function select(id) {
  emit('update:modelValue', id);
  close();
}
</script>

<template>
  <div ref="rootEl" class="version-select">
    <button
      ref="triggerEl"
      :id="id"
      type="button"
      class="field version-select-trigger"
      :disabled="disabled || loading"
      @click="toggle(disabled || loading)"
    >
      <span class="vst-text">{{ displayText }}</span>
      <svg class="chev" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M6 9l6 6 6-6" /></svg>
    </button>

    <Teleport to="body">
      <div v-if="open" ref="ddEl" class="dropdown version-dd" :style="ddStyle" @click.stop>
        <div v-for="group in groups" :key="group.key" class="version-group">
          <div class="version-group-label">{{ groupLabel(group) }}</div>
          <button
            v-for="v in group.versions"
            :key="v.id"
            type="button"
            class="switch-opt version-opt"
            :class="{ on: v.id === modelValue }"
            @click="select(v.id)"
          >
            <span class="vst-text">
              {{ v.id }}<span v-if="v.type !== 'release'" class="version-opt-type"> ({{ t(`versions.type.${v.type}`) }})</span>
            </span>
            <svg v-if="v.id === modelValue" class="chk" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3">
              <path d="M20 6L9 17l-5-5" />
            </svg>
          </button>
        </div>
      </div>
    </Teleport>
  </div>
</template>
