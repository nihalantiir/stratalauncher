import { invoke } from '@tauri-apps/api/core';

export const listScreenshots = (instanceId) => invoke('list_screenshots', { instanceId });

export const deleteScreenshot = (instanceId, fileName) => invoke('delete_screenshot', { instanceId, fileName });

export const getScreenshotsDir = (instanceId) => invoke('get_screenshots_dir', { instanceId });
