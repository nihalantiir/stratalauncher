import { invoke } from '@tauri-apps/api/core';

export const listInstances = () => invoke('list_instances');
export const getCurrentInstance = () => invoke('get_current_instance');
export const setCurrentInstance = (id) => invoke('set_current_instance', { id });
export const getInstanceDir = (id) => invoke('get_instance_dir', { id });

export const createInstance = (name, mcVersion, iconBiome, loader = null, loaderVersion = null, groupName = null) =>
  invoke('create_instance', { input: { name, mcVersion, iconBiome, loader, loaderVersion, groupName } });

export const updateInstance = (instance) =>
  invoke('update_instance', {
    input: {
      id: instance.id,
      name: instance.name,
      mcVersion: instance.mcVersion,
      memoryMb: instance.memoryMb ?? null,
      minMemoryMb: instance.minMemoryMb ?? null,
      jvmArgs: instance.jvmArgs ?? null,
      javaPath: instance.javaPath ?? null,
      iconBiome: instance.iconBiome,
      groupName: instance.groupName ?? null,
      windowWidth: instance.windowWidth ?? null,
      windowHeight: instance.windowHeight ?? null,
      windowMaximized: instance.windowMaximized ?? false,
      skipJavaCheck: instance.skipJavaCheck ?? false,
      envVars: instance.envVars ?? null,
      preLaunchCmd: instance.preLaunchCmd ?? null,
      wrapperCmd: instance.wrapperCmd ?? null,
      postExitCmd: instance.postExitCmd ?? null,
      consoleMode: instance.consoleMode ?? 'never',
      quickPlayMode: instance.quickPlayMode ?? 'off',
      quickPlayTarget: instance.quickPlayTarget ?? null,
    },
  });

export const deleteInstance = (id) => invoke('delete_instance', { id });

export const setInstanceLoader = (id, loader, loaderVersion, mcVersion) =>
  invoke('set_instance_loader', { input: { id, loader, loaderVersion, mcVersion } });

export const setCustomClientJar = (id, path) => invoke('set_custom_client_jar', { id, path });

export const getCoremodIcon = (path) => invoke('get_coremod_icon', { path });
