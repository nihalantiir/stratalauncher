import { useI18n } from 'vue-i18n';
import { useRouter } from 'vue-router';
import { openPath } from '@tauri-apps/plugin-opener';
import { useInstancesStore } from '../stores/instances';
import { useAccountsStore } from '../stores/accounts';
import { launchInstance } from '../api/launch';
import { getInstanceDir } from '../api/instances';

// Shared by InstanceCard.vue (the grid tiles) and LibraryView.vue's hero card
// for the current instance, so the menu-building logic lives in one place instead of two components drifting apart.
export function useInstanceContextMenu() {
  const { t } = useI18n();
  const router = useRouter();
  const instances = useInstancesStore();
  const accounts = useAccountsStore();

  async function openFolder(instance) {
    const dir = await getInstanceDir(instance.id);
    await openPath(dir);
  }

  // "Delete instance" below deliberately navigates here instead of deleting
  // directly from the context menu, avoiding a second, less-safe path to the same irreversible action.
  async function goToSettings(instance) {
    await instances.setCurrent(instance.id);
    router.push('/instance-settings');
  }

  function buildInstanceMenuItems(instance) {
    const isRunning = instances.isRunning(instance.id);
    return [
      isRunning
        ? { label: t('library.stop'), icon: 'stop', action: () => instances.stopInstance(instance.id) }
        : {
            label: t('library.play'),
            icon: 'play',
            disabled: !accounts.active,
            action: () => launchInstance(instance.id).then(() => instances.refresh()),
          },
      'separator',
      { label: t('library.openFolder'), icon: 'folder', action: () => openFolder(instance) },
      { label: t('instances.settingsTitle'), icon: 'settings', action: () => goToSettings(instance) },
      'separator',
      { label: t('instances.deleteInstance'), icon: 'trash', danger: true, action: () => goToSettings(instance) },
    ];
  }

  return { buildInstanceMenuItems };
}
