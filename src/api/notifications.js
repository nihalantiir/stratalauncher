import { invoke } from '@tauri-apps/api/core';

export const listNotifications = () => invoke('list_notifications');

export const dismissNotification = (id) => invoke('dismiss_notification', { id });

export const runSyncCheck = () => invoke('run_sync_check');
