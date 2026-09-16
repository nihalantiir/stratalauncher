import { invoke } from '@tauri-apps/api/core';

export const getMediaStatus = () => invoke('media_status');

export const downloadMedia = () => invoke('download_media');
