import { invoke } from '@tauri-apps/api/core';

export const listInstalledContent = (instanceId, kind) =>
  invoke('list_installed_content', { instanceId, kind });

export const toggleContent = (instanceId, kind, filename) =>
  invoke('toggle_content', { instanceId, kind, filename });

export const removeContent = (instanceId, kind, filename) =>
  invoke('remove_content', { instanceId, kind, filename });

export const searchContent = (instanceId, kind, source, query, sort, categories, mcVersion, offset = 0) =>
  invoke('search_content', { instanceId, kind, source, query, sort, categories, mcVersion, offset });

export const installContent = (instanceId, kind, source, projectId, title, iconUrl, mcVersion) =>
  invoke('install_content', { instanceId, kind, source, projectId, title, iconUrl, mcVersion });

export const checkContentUpdates = (instanceId, kind) =>
  invoke('check_content_updates', { instanceId, kind });

export const curseforgeConfigured = () => invoke('curseforge_configured');

export const listContentCategories = (source, kind) => invoke('list_content_categories', { source, kind });

export const getContentDir = (instanceId, kind) => invoke('get_content_dir', { instanceId, kind });

export const listContentVersions = (source, projectId) => invoke('list_content_versions', { source, projectId });

export const installContentVersion = (instanceId, kind, source, projectId, title, iconUrl, version) =>
  invoke('install_content_version', { instanceId, kind, source, projectId, title, iconUrl, version });
