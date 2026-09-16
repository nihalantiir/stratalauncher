<script setup>
import { ref, computed } from 'vue';
import { useI18n } from 'vue-i18n';
import { useInstancesStore } from '../../stores/instances';
import { useSidebarMode } from '../../composables/useSidebarMode';
import InstanceSwitcher from './InstanceSwitcher.vue';

const { t } = useI18n();
const instances = useInstancesStore();
const { persistent } = useSidebarMode();
const hovering = ref(false);
const expanded = computed(() => persistent.value || hovering.value);

// A short debounce on the way out avoids a flicker when the pointer just grazes the edge or
// crosses it quickly, since collapsing instantly on every micro mouse-out read as twitchy. Re-entering cancels it outright.
let leaveTimer = null;
function onEnter() {
  clearTimeout(leaveTimer);
  hovering.value = true;
}
function onLeave() {
  leaveTimer = setTimeout(() => {
    hovering.value = false;
  }, 200);
}
</script>

<template>
  <div class="sidebar-rail" :class="{ collapsed: !expanded }" @mouseenter="onEnter" @mouseleave="onLeave">
    <nav class="sidebar" :class="{ collapsed: !expanded }">
      <router-link to="/" class="nav-primary" active-class="active" exact-active-class="active">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8">
          <rect x="3" y="3" width="7" height="7" rx="1.5" />
          <rect x="14" y="3" width="7" height="7" rx="1.5" />
          <rect x="3" y="14" width="7" height="7" rx="1.5" />
          <rect x="14" y="14" width="7" height="7" rx="1.5" />
        </svg>
        <span>{{ t('sidebar.library') }}</span>
      </router-link>

    <template v-if="instances.current">
      <div class="sidebar-divider"></div>
      <InstanceSwitcher :collapsed="!expanded" />

      <div class="nav-sub">
        <router-link to="/version" class="nav-item" active-class="active">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8">
            <polygon points="12 3 3 8 12 13 21 8 12 3" />
            <polyline points="3 12 12 17 21 12" />
            <polyline points="3 16 12 21 21 16" />
          </svg>
          <span>{{ t('sidebar.version') }}</span>
        </router-link>
        <router-link to="/mods" class="nav-item" active-class="active">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8">
            <path d="M12 3l3 3-3 3-3-3 3-3z" />
            <path d="M8 12l3 3-3 3-3-3 3-3z" />
            <path d="M16 12l3 3-3 3-3-3 3-3z" />
            <path d="M12 15l3 3-3 3-3-3 3-3z" />
          </svg>
          <span>{{ t('sidebar.mods') }}</span>
        </router-link>
        <router-link to="/resourcepacks" class="nav-item" active-class="active">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8">
            <rect x="3" y="4" width="18" height="16" rx="2" />
            <circle cx="8.5" cy="9.5" r="1.6" />
            <path d="M4 17l4.5-4.5 3 3L16 10l4 4.5" />
          </svg>
          <span>{{ t('sidebar.resourcePacks') }}</span>
        </router-link>
        <router-link to="/shaders" class="nav-item" active-class="active">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8">
            <circle cx="12" cy="12" r="2.6" />
            <path d="M12 3.5v3M12 17.5v3M3.5 12h3M17.5 12h3M6 6l2.1 2.1M15.9 15.9L18 18M18 6l-2.1 2.1M8.1 15.9L6 18" />
          </svg>
          <span>{{ t('sidebar.shaders') }}</span>
        </router-link>
        <router-link to="/worlds" class="nav-item" active-class="active">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8">
            <circle cx="12" cy="12" r="8.5" />
            <path d="M3.5 12h17M12 3.5c2.6 2.4 4 5.3 4 8.5s-1.4 6.1-4 8.5c-2.6-2.4-4-5.3-4-8.5s1.4-6.1 4-8.5z" />
          </svg>
          <span>{{ t('sidebar.worlds') }}</span>
        </router-link>
        <router-link to="/servers" class="nav-item" active-class="active">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8">
            <rect x="4" y="4" width="16" height="6" rx="1.5" />
            <rect x="4" y="14" width="16" height="6" rx="1.5" />
            <circle cx="7.5" cy="7" r="0.9" fill="currentColor" stroke="none" />
            <circle cx="7.5" cy="17" r="0.9" fill="currentColor" stroke="none" />
          </svg>
          <span>{{ t('sidebar.servers') }}</span>
        </router-link>
        <router-link to="/screenshots" class="nav-item" active-class="active">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8">
            <path d="M9 5l1.4-2h3.2L15 5h3a2 2 0 0 1 2 2v10a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2V7a2 2 0 0 1 2-2z" />
            <circle cx="12" cy="13" r="3.6" />
          </svg>
          <span>{{ t('sidebar.screenshots') }}</span>
        </router-link>
        <router-link to="/logs" class="nav-item" active-class="active">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8">
            <path d="M6 3h9l4 4v14H6z" />
            <path d="M9.5 11h6M9.5 14.5h6M9.5 18h4" />
          </svg>
          <span>{{ t('sidebar.logs') }}</span>
        </router-link>
        <router-link to="/instance-settings" class="nav-item" active-class="active">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8">
            <path d="M14.7 6.3a2 2 0 0 0 2.83 0l1.3-1.3a4 4 0 0 1-5.66 5.66L4 19.8V21h1.2l9.14-9.14a4 4 0 0 1 5.66-5.66l-1.3 1.3a2 2 0 0 0 0 2.83" />
          </svg>
          <span>{{ t('sidebar.instanceSettings') }}</span>
        </router-link>
      </div>
    </template>

    <div class="sidebar-spacer"></div>
    <div class="sidebar-divider"></div>

    <router-link to="/skin" class="nav-item" active-class="active">
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8">
        <rect x="5" y="3" width="14" height="18" rx="2" />
        <path d="M9 3v3a3 3 0 0 0 6 0V3" />
      </svg>
      <span>{{ t('sidebar.skinCape') }}</span>
    </router-link>
    <router-link to="/import" class="nav-item" active-class="active">
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8">
        <path d="M12 3v12M7 10l5 5 5-5" />
        <path d="M4 18v1.5A1.5 1.5 0 0 0 5.5 21h13a1.5 1.5 0 0 0 1.5-1.5V18" />
      </svg>
      <span>{{ t('sidebar.import') }}</span>
    </router-link>

    <router-link to="/settings" class="nav-item" active-class="active">
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8">
        <circle cx="12" cy="12" r="3.2" />
        <path d="M19.4 13.5a7.6 7.6 0 0 0 0-3l1.9-1.5-2-3.4-2.3.6a7.6 7.6 0 0 0-2.6-1.5L14 2.5h-4l-.4 2.2a7.6 7.6 0 0 0-2.6 1.5l-2.3-.6-2 3.4L4.6 10.5a7.6 7.6 0 0 0 0 3l-1.9 1.5 2 3.4 2.3-.6a7.6 7.6 0 0 0 2.6 1.5l.4 2.2h4l.4-2.2a7.6 7.6 0 0 0 2.6-1.5l2.3.6 2-3.4z" />
      </svg>
      <span>{{ t('sidebar.settings') }}</span>
    </router-link>
    </nav>
  </div>
</template>
