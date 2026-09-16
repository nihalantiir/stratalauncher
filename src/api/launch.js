import { invoke } from '@tauri-apps/api/core';

export const launchInstance = (instanceId) => invoke('launch_instance', { instanceId });

export const listRunningInstances = () => invoke('list_running_instances');

export const stopInstance = (instanceId) => invoke('stop_instance', { instanceId });
