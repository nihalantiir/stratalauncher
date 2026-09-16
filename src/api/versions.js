import { invoke } from '@tauri-apps/api/core';

export const listVersions = () => invoke('list_versions');
