import { invoke } from '@tauri-apps/api/core';

export const checkForUpdate = () => invoke('check_for_update');

export const installUpdate = (info) => invoke('install_update', { downloadUrl: info.downloadUrl, sha256: info.sha256 });
