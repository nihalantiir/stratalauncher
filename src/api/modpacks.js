import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';

export const searchModpacks = (source, query, mcVersion, categories, sort, offset = 0) =>
  invoke('search_modpacks', { source, query, mcVersion, categories, sort, offset });

export const listModpackCategories = (source) => invoke('list_modpack_categories', { source });

export const installModpackVersion = (version, name, groupName) =>
  invoke('install_modpack_version', { version, name, groupName });

export const installModpackFile = (path, name, groupName) =>
  invoke('install_modpack_file', { path, name, groupName });

export const pickModpackFile = () =>
  open({ multiple: false, filters: [{ name: 'Modpack', extensions: ['mrpack', 'zip'] }] });
