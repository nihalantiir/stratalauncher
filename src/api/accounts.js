import { invoke } from '@tauri-apps/api/core';

export const listAccounts = () => invoke('list_accounts');
export const getActiveAccount = () => invoke('get_active_account');
export const setActiveAccount = (id) => invoke('set_active_account', { id });
export const removeAccount = (id) => invoke('remove_account', { id });
export const createOfflineProfile = (username) => invoke('create_offline_profile', { username });
export const startMicrosoftLogin = () => invoke('start_microsoft_login');
export const cancelMicrosoftLogin = () => invoke('cancel_microsoft_login');
