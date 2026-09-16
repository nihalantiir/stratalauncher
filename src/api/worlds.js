import { invoke } from '@tauri-apps/api/core';

export const listWorlds = (instanceId) => invoke('list_worlds', { instanceId });

export const backupWorld = (instanceId, folder) => invoke('backup_world', { instanceId, folder });

export const restoreLatestBackup = (instanceId, folder) => invoke('restore_latest_backup', { instanceId, folder });

export const restoreBackup = (instanceId, folder, file) => invoke('restore_backup', { instanceId, folder, file });

export const listBackups = (instanceId, folder) => invoke('list_backups', { instanceId, folder });

export const deleteBackup = (instanceId, file) => invoke('delete_backup', { instanceId, file });

export const deleteWorld = (instanceId, folder) => invoke('delete_world', { instanceId, folder });

export const getSavesDir = (instanceId) => invoke('get_saves_dir', { instanceId });

export const getWorldDir = (instanceId, folder) => invoke('get_world_dir', { instanceId, folder });
