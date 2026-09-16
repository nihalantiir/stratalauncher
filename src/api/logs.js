import { invoke } from '@tauri-apps/api/core';

export const listLogFiles = (instanceId) => invoke('list_log_files', { instanceId });

export const readMinecraftLog = (instanceId, filename) => invoke('read_minecraft_log', { instanceId, filename });

export const readMinecraftLogRaw = (instanceId, filename) => invoke('read_minecraft_log_raw', { instanceId, filename });

export const listLauncherLogFiles = () => invoke('list_launcher_log_files');

export const readLauncherLog = (filename) => invoke('read_launcher_log', { filename });

export const readLauncherLogRaw = (filename) => invoke('read_launcher_log_raw', { filename });

export const getLogsDir = (instanceId) => invoke('get_logs_dir', { instanceId });

export const getLauncherLogsDir = () => invoke('get_launcher_logs_dir');

export const uploadLogText = (content, target) => invoke('upload_log_text', { content, target });

export const listUploadTargets = () => invoke('list_upload_targets');

export const getMclogsInsights = (id) => invoke('get_mclogs_insights', { id });
