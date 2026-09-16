import { invoke } from '@tauri-apps/api/core';

// Routed through our own command, not @tauri-apps/plugin-opener's openPath:
// its IPC scope can't express Strata's dynamic data directory, so it always denies.
export const openPath = (path) => invoke('open_in_explorer', { path });
