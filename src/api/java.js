import { invoke } from '@tauri-apps/api/core';

export const getRequiredJava = (mcVersion) => invoke('get_required_java', { mcVersion });

export const probeJavaAt = (path) => invoke('probe_java_at', { path });
