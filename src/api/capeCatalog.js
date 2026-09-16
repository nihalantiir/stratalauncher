import { invoke } from '@tauri-apps/api/core';

// null when nothing remote is available yet (no repo configured, or configured but never
// fetched successfully); callers fall back to the bundled `src/lib/knownCapes.js` list.
export const getCapeCatalog = () => invoke('get_cape_catalog');
