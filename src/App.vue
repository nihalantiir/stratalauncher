<script setup>
import { ref, onMounted, onBeforeUnmount } from 'vue';
import Sidebar from './components/layout/Sidebar.vue';
import Topbar from './components/layout/Topbar.vue';
import VideoBackground from './components/layout/VideoBackground.vue';
import FirstRunWizard from './components/onboarding/FirstRunWizard.vue';
import ContextMenu from './components/common/ContextMenu.vue';
import AppTooltip from './components/common/AppTooltip.vue';
import { useInstancesStore } from './stores/instances';
import { useAccountsStore } from './stores/accounts';
import { useVersionsStore } from './stores/versions';

const instances = useInstancesStore();
const accounts = useAccountsStore();
const versions = useVersionsStore();
const showWizard = ref(false);

const ONBOARDED_KEY = 'strata-onboarded';

onMounted(async () => {
  instances.initRunningTracking();

  // Fetch here instead of relying on LibraryView's lifecycle hooks, since
  // booting on any other route would otherwise leave these stores empty.
  instances.refresh();
  if (!versions.manifest) versions.fetch();

  let onboarded = false;
  try {
    onboarded = localStorage.getItem(ONBOARDED_KEY) === '1';
  } catch {
    // ignore storage errors; worst case the wizard shows once more
  }

  await accounts.refresh();
  if (!onboarded && accounts.accounts.length === 0) showWizard.value = true;
});

// This is desktop app chrome, not a web page, so the WebView2's native
// right-click menu is suppressed globally as the fallback for any element without its own custom menu (see useContextMenu).
function suppressNativeContextMenu(event) {
  event.preventDefault();
}
onMounted(() => document.addEventListener('contextmenu', suppressNativeContextMenu));
onBeforeUnmount(() => document.removeEventListener('contextmenu', suppressNativeContextMenu));

function finishWizard() {
  try {
    localStorage.setItem(ONBOARDED_KEY, '1');
  } catch {
    // ignore
  }
  showWizard.value = false;
}
</script>

<template>
  <VideoBackground />
  <div class="app-shell">
    <Sidebar />
    <div class="main">
      <Topbar />
      <router-view v-slot="{ Component, route }">
        <transition name="page" mode="out-in">
          <keep-alive>
            <component :is="Component" :key="route.path" />
          </keep-alive>
        </transition>
      </router-view>
    </div>
  </div>
  <transition name="wizard-fade">
    <FirstRunWizard v-if="showWizard" @done="finishWizard" />
  </transition>
  <ContextMenu />
  <AppTooltip />
</template>

<style>
.app-shell {
  position: relative;
  z-index: 1;
  display: flex;
  height: 100vh;
}
.main {
  position: relative;
  flex: 1;
  display: flex;
  flex-direction: column;
  min-width: 0;
}

.page-enter-active {
  transition: opacity var(--dur) var(--ease-out), transform var(--dur) var(--ease-out);
}
.page-leave-active {
  transition: opacity calc(var(--dur) * 0.6) var(--ease-out);
}
.page-enter-from {
  opacity: 0;
  transform: translateY(8px);
}
.page-leave-to {
  opacity: 0;
}

.wizard-fade-enter-active,
.wizard-fade-leave-active {
  transition: opacity var(--dur) var(--ease-out);
}
.wizard-fade-enter-from,
.wizard-fade-leave-to {
  opacity: 0;
}
</style>
