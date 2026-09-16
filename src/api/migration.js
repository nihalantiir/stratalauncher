import { invoke } from '@tauri-apps/api/core';

export const scanAllInstalls = () => invoke('scan_all_installs');

export const scanCustomFolder = (kind, path) => invoke('scan_custom_folder', { kind, path });

export const importInstance = (input) => invoke('import_instance', { input });
