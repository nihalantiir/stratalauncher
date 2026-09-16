import { invoke } from '@tauri-apps/api/core';

export const getRequiredJava = (mcVersion) => invoke('get_required_java', { mcVersion });

export const probeJavaAt = (path) => invoke('probe_java_at', { path });

export const listJavaInstallations = () => invoke('list_java_installations');

export const listDownloadedRuntimes = () => invoke('list_downloaded_runtimes');

export const deleteDownloadedRuntime = (component) => invoke('delete_downloaded_runtime', { component });
