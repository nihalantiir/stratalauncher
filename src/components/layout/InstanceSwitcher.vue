<script setup>
import { ref, computed } from 'vue';
import { useI18n } from 'vue-i18n';
import { useInstancesStore } from '../../stores/instances';
import { useFloatingDropdown } from '../../composables/useFloatingDropdown';
import NewInstanceModal from '../library/NewInstanceModal.vue';

defineProps({ collapsed: { type: Boolean, default: false } });

const { t } = useI18n();
const instances = useInstancesStore();
const showCreate = ref(false);
const { open, rootEl, triggerEl, ddEl, ddStyle, toggle, close } = useFloatingDropdown({ minWidth: 220 });

const loaderLabel = computed(() => t(`loaders.${instances.current?.loader}`));

function select(id) {
  instances.setCurrent(id);
  close();
}
</script>

<template>
  <div v-if="instances.current" ref="rootEl" style="position: relative">
    <div
      ref="triggerEl"
      class="instance-switch"
      role="button"
      tabindex="0"
      v-tooltip="collapsed ? instances.current.name : undefined"
      @click="toggle()"
      @keyup.enter="toggle()"
    >
      <img src="/grass-block.png" alt="" class="swatch" />
      <div class="info">
        <div class="n">{{ instances.current.name }}</div>
        <div class="m">{{ loaderLabel }}</div>
      </div>
      <svg class="chev" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M6 9l6 6 6-6" /></svg>
    </div>

    <Teleport to="body">
      <div v-if="open" ref="ddEl" class="dropdown instance-dd" :style="ddStyle" @click.stop>
        <button
          v-for="inst in instances.list"
          :key="inst.id"
          type="button"
          class="switch-opt"
          :class="{ on: inst.id === instances.current.id }"
          @click="select(inst.id)"
        >
          <img src="/grass-block.png" alt="" class="swatch sm" />
          {{ inst.name }}
          <svg
            v-if="inst.id === instances.current.id"
            class="chk"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="3"
          >
            <path d="M20 6L9 17l-5-5" />
          </svg>
        </button>
        <button type="button" class="switch-opt" style="color: var(--mineral); font-weight: 600" @click.stop="showCreate = true; close()">
          + {{ t('library.newInstance') }}
        </button>
      </div>
    </Teleport>

    <Transition name="modal">
      <NewInstanceModal v-if="showCreate" @close="showCreate = false" />
    </Transition>
  </div>
</template>
