import { invoke } from '@tauri-apps/api/core';

export const getAppSettings = () => invoke('get_app_settings');

export const updateAppSettings = (settings) => invoke('update_app_settings', { input: settings });

export const getDataDirInfo = () => invoke('get_data_dir_info');

export const setPendingDataDir = (path) => invoke('set_pending_data_dir', { path });

export const resetDataDir = () => invoke('reset_data_dir');

export const restartApp = () => invoke('restart_app');

export const getRecommendedMemoryMb = () => invoke('get_recommended_memory_mb');
