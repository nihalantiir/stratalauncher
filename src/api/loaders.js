import { invoke } from '@tauri-apps/api/core';

export const listLoaderVersions = (loader, mcVersion) =>
  invoke('list_loader_versions', { loader, mcVersion });

export const checkLoaderAvailability = (mcVersion) => invoke('check_loader_availability', { mcVersion });

export const getVersionComponents = (mcVersion, loader, loaderVersion) =>
  invoke('get_version_components', { mcVersion, loader, loaderVersion });
