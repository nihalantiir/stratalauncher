<script setup>
import { ref, computed, onMounted, onBeforeUnmount } from 'vue';
import { useI18n } from 'vue-i18n';
import { useRoute, useRouter } from 'vue-router';
import { openUrl } from '@tauri-apps/plugin-opener';
import { writeText } from '@tauri-apps/plugin-clipboard-manager';
import { useAccountsStore } from '../../stores/accounts';
import { useInstancesStore } from '../../stores/instances';
import { useNotificationsStore } from '../../stores/notifications';
import LoginModal from '../auth/LoginModal.vue';
import SkinHead from '../common/SkinHead.vue';
import { useContextMenu } from '../../composables/useContextMenu';

const { t } = useI18n();
const route = useRoute();
const router = useRouter();
const accounts = useAccountsStore();
const instances = useInstancesStore();
const notifications = useNotificationsStore();
const { openContextMenu } = useContextMenu();
const menuOpen = ref(false);
const notifOpen = ref(false);
const loginOpen = ref(false);
const rootEl = ref(null);
const notifRootEl = ref(null);

const pageTitle = computed(() => t(`topbar.title.${route.name ?? 'library'}`));
const activeLabel = computed(() => accounts.active?.username || t('topbar.signIn'));
const activeSub = computed(() => {
  if (!accounts.active) return '';
  return accounts.active.kind === 'microsoft' ? t('topbar.microsoftAccount') : t('topbar.offlineProfile');
});

function selectAccount(id) {
  accounts.setActive(id);
  menuOpen.value = false;
}

function openAdd() {
  menuOpen.value = false;
  loginOpen.value = true;
}

function remove(id, event) {
  event.stopPropagation();
  accounts.remove(id);
}

function onDocClick(event) {
  if (menuOpen.value && rootEl.value && !rootEl.value.contains(event.target)) {
    menuOpen.value = false;
  }
  if (notifOpen.value && notifRootEl.value && !notifRootEl.value.contains(event.target)) {
    notifOpen.value = false;
  }
}

onMounted(() => {
  document.addEventListener('click', onDocClick);
  notifications.refresh();
});
onBeforeUnmount(() => document.removeEventListener('click', onDocClick));

async function goToNotification(n) {
  notifOpen.value = false;
  if (n.instanceId) {
    await instances.setCurrent(n.instanceId);
    router.push('/version');
  } else if (n.url) {
    openUrl(n.url);
  }
  await notifications.dismiss(n.id);
}

function notifKind(n) {
  if (n.id.startsWith('mc-release')) return 'release';
  if (n.id.startsWith('mc-snapshot')) return 'snapshot';
  if (n.id.startsWith('loader-')) return 'loader';
  if (n.id.startsWith('launcher-update')) return 'update';
  return 'info';
}

function relativeTime(iso) {
  const then = new Date(iso).getTime();
  if (Number.isNaN(then)) return '';
  const diffSec = Math.max(0, Math.floor((Date.now() - then) / 1000));
  if (diffSec < 60) return t('notifications.justNow');
  const diffMin = Math.floor(diffSec / 60);
  if (diffMin < 60) return t('notifications.minutesAgo', { count: diffMin });
  const diffHr = Math.floor(diffMin / 60);
  if (diffHr < 24) return t('notifications.hoursAgo', { count: diffHr });
  const diffDay = Math.floor(diffHr / 24);
  if (diffDay < 7) return t('notifications.daysAgo', { count: diffDay });
  return new Date(iso).toLocaleDateString();
}

function onNotificationContextMenu(event, n) {
  openContextMenu(event, [
    ...(n.instanceId || n.url ? [{ label: t('notifications.open'), icon: 'eye', action: () => goToNotification(n) }] : []),
    { label: t('notifications.dismiss'), icon: 'trash', action: () => notifications.dismiss(n.id) },
    ...(notifications.list.length > 1
      ? ['separator', { label: t('notifications.clearAll'), icon: 'trash', danger: true, action: () => notifications.clearAll() }]
      : []),
  ]);
}

function onAccountContextMenu(event, acc) {
  const isActive = acc.id === accounts.active?.id;
  openContextMenu(event, [
    ...(isActive ? [] : [{ label: t('topbar.setActive'), icon: 'power', action: () => selectAccount(acc.id) }]),
    { label: t('topbar.copyUsername'), icon: 'copy', action: () => writeText(acc.username) },
    'separator',
    { label: t('topbar.remove'), icon: 'trash', danger: true, action: () => accounts.remove(acc.id) },
  ]);
}
</script>

<template>
  <header class="topbar">
    <h2 class="page-title">{{ pageTitle }}</h2>
    <div style="display: flex; align-items: center; gap: 10px; position: relative">
    <div ref="notifRootEl">
      <button class="bell-btn" type="button" :aria-label="t('notifications.title')" @click="notifOpen = !notifOpen">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8">
          <path d="M18 8a6 6 0 0 0-12 0c0 7-3 9-3 9h18s-3-2-3-9" />
          <path d="M13.73 21a2 2 0 0 1-3.46 0" />
        </svg>
        <span v-if="notifications.list.length" class="notif-badge">{{ notifications.list.length > 9 ? '9+' : notifications.list.length }}</span>
      </button>

      <div class="dropdown" v-if="notifOpen" style="width: 340px">
        <div class="dropdown-head">
          <h4>{{ t('notifications.title') }}</h4>
          <button
            v-if="notifications.list.length"
            class="btn btn-ghost btn-sm"
            type="button"
            @click="notifications.clearAll()"
          >
            {{ t('notifications.clearAll') }}
          </button>
        </div>

        <div v-if="notifications.list.length === 0" class="empty-state" style="margin: 6px">
          {{ t('notifications.empty') }}
        </div>

        <div
          v-for="n in notifications.list"
          :key="n.id"
          class="dropdown-item notif-item"
          style="cursor: default"
          @contextmenu="onNotificationContextMenu($event, n)"
        >
          <div class="notif-icon" :class="`kind-${notifKind(n)}`">
            <svg v-if="notifKind(n) === 'release' || notifKind(n) === 'snapshot'" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8">
              <polygon points="12 3 3 8 12 13 21 8 12 3" />
              <polyline points="3 12 12 17 21 12" />
              <polyline points="3 16 12 21 21 16" />
            </svg>
            <svg v-else-if="notifKind(n) === 'loader'" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8">
              <path d="M12 3l3 3-3 3-3-3 3-3z" />
              <path d="M8 12l3 3-3 3-3-3 3-3z" />
              <path d="M16 12l3 3-3 3-3-3 3-3z" />
              <path d="M12 15l3 3-3 3-3-3 3-3z" />
            </svg>
            <svg v-else-if="notifKind(n) === 'update'" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8">
              <path d="M12 3v12M7 10l5 5 5-5" />
              <path d="M4 18v1.5A1.5 1.5 0 0 0 5.5 21h13a1.5 1.5 0 0 0 1.5-1.5V18" />
            </svg>
            <svg v-else viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8">
              <path d="M18 8a6 6 0 0 0-12 0c0 7-3 9-3 9h18s-3-2-3-9" />
              <path d="M13.73 21a2 2 0 0 1-3.46 0" />
            </svg>
          </div>
          <div style="flex: 1; min-width: 0; cursor: pointer" @click="goToNotification(n)">
            <p>{{ n.title }}</p>
            <small>{{ n.body }}</small>
            <small class="notif-time">{{ relativeTime(n.createdAt) }}</small>
          </div>
          <button class="icon-btn" type="button" v-tooltip="t('notifications.dismiss')" @click="notifications.dismiss(n.id)">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M6 6l12 12M18 6L6 18" /></svg>
          </button>
        </div>
      </div>
    </div>

    <div ref="rootEl" style="position: relative">
      <button
        class="account-btn"
        type="button"
        @click="menuOpen = !menuOpen"
        @contextmenu="accounts.active && onAccountContextMenu($event, accounts.active)"
      >
        <div class="avatar" :class="{ offline: accounts.active?.kind === 'offline' }">
          <SkinHead v-if="accounts.active" :skin-url="accounts.active?.skinUrl" :size="38" />
        </div>
        <div>
          <div class="name">{{ activeLabel }}</div>
          <div class="sub" v-if="activeSub">{{ activeSub }}</div>
        </div>
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M6 9l6 6 6-6" /></svg>
      </button>

      <div class="dropdown" v-if="menuOpen">
        <div class="dropdown-head">
          <h4>{{ t('topbar.accountsHeading') }}</h4>
          <span>{{ t('topbar.signedInCount', { count: accounts.accounts.length }) }}</span>
        </div>

        <div
          v-for="acc in accounts.accounts"
          :key="acc.id"
          class="dropdown-item"
          role="button"
          tabindex="0"
          style="align-items: center; cursor: pointer"
          @click="selectAccount(acc.id)"
          @keyup.enter="selectAccount(acc.id)"
          @contextmenu="onAccountContextMenu($event, acc)"
        >
          <div
            class="avatar"
            :class="{ offline: acc.kind === 'offline' }"
            style="width: 32px; height: 32px; flex: 0 0 32px"
          >
            <SkinHead :skin-url="acc.skinUrl" :size="32" />
          </div>
          <div style="flex: 1; min-width: 0">
            <p>
              {{ acc.username }}
              <span v-if="acc.isActive" style="color: var(--mineral); font-weight: 500">· {{ t('topbar.active') }}</span>
            </p>
            <small>{{ acc.kind === 'microsoft' ? t('topbar.microsoftAccount') : t('topbar.offlineProfile') }}</small>
          </div>
          <button class="btn btn-danger-ghost btn-sm" type="button" @click="remove(acc.id, $event)">
            {{ t('topbar.remove') }}
          </button>
        </div>

        <div v-if="accounts.accounts.length === 0" class="empty-state" style="margin: 6px;">
          {{ t('topbar.signIn') }}
        </div>

        <div style="padding: 8px 10px 4px">
          <button class="btn btn-ghost btn-block" type="button" @click="openAdd">
            {{ t('topbar.addAccount') }}
          </button>
        </div>
      </div>
    </div>
    </div>

    <Transition name="modal">
      <LoginModal v-if="loginOpen" @close="loginOpen = false" />
    </Transition>
  </header>
</template>
