<script setup>
import { computed } from 'vue';
import { useFloatingDropdown } from '../../composables/useFloatingDropdown';

const props = defineProps({
  modelValue: { type: [String, Number], default: '' },
  options: { type: Array, required: true }, // [{ value, label }]
  disabled: { type: Boolean, default: false },
  id: { type: String, default: undefined },
});
const emit = defineEmits(['update:modelValue']);
const { open, rootEl, triggerEl, ddEl, ddStyle, toggle, close } = useFloatingDropdown();

const selectedLabel = computed(() => {
  const match = props.options.find((o) => o.value === props.modelValue);
  return match?.label ?? '';
});

function select(value) {
  emit('update:modelValue', value);
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
      :disabled="disabled"
      @click="toggle(disabled)"
    >
      <span class="vst-text">{{ selectedLabel }}</span>
      <svg class="chev" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M6 9l6 6 6-6" /></svg>
    </button>

    <Teleport to="body">
      <div v-if="open" ref="ddEl" class="dropdown version-dd" :style="ddStyle" @click.stop>
        <button
          v-for="opt in options"
          :key="opt.value"
          type="button"
          class="switch-opt version-opt"
          :class="{ on: opt.value === modelValue }"
          @click="select(opt.value)"
        >
          <span class="vst-text">{{ opt.label }}</span>
          <svg v-if="opt.value === modelValue" class="chk" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3">
            <path d="M20 6L9 17l-5-5" />
          </svg>
        </button>
      </div>
    </Teleport>
  </div>
</template>
