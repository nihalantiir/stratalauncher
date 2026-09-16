import { invoke } from '@tauri-apps/api/core';

export const listServers = (instanceId) => invoke('list_servers', { instanceId });

export const pingServer = (address) => invoke('ping_server', { address });
