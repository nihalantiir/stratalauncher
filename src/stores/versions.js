import { defineStore } from 'pinia';
import { listVersions } from '../api/versions';
import { groupVersions } from '../lib/versionCodenames';

const EXPERIMENTAL_STORAGE_KEY = 'strata-show-experimental-versions';

function loadShowExperimental() {
  try {
    return localStorage.getItem(EXPERIMENTAL_STORAGE_KEY) === '1';
  } catch {
    return false;
  }
}

export const useVersionsStore = defineStore('versions', {
  state: () => ({
    manifest: null,
    loading: false,
    error: null,
    selected: null,
    showExperimental: loadShowExperimental(),
  }),
  getters: {
    releases(state) {
      return state.manifest ? state.manifest.versions.filter((v) => v.type === 'release') : [];
    },
    // Releases always shown; snapshots/old_beta/old_alpha only when the
    // "show experimental" toggle is on, matching how most launchers do it.
    selectableVersions(state) {
      if (!state.manifest) return [];
      if (state.showExperimental) return state.manifest.versions;
      return state.manifest.versions.filter((v) => v.type === 'release');
    },
    // Same list, bucketed into real update subcategories (e.g. "Chaos Cubed")
    // for a grouped <select>, using the same logic as the panorama picker so the two always agree.
    groupedSelectableVersions() {
      return groupVersions(this.selectableVersions, this.manifest?.versions);
    },
  },
  actions: {
    async fetch() {
      this.loading = true;
      this.error = null;
      try {
        this.manifest = await listVersions();
        if (!this.selected && this.manifest?.latest?.release) {
          this.selected = this.manifest.latest.release;
        }
      } catch (e) {
        this.error = String(e);
      } finally {
        this.loading = false;
      }
    },

    setShowExperimental(value) {
      this.showExperimental = value;
      try {
        localStorage.setItem(EXPERIMENTAL_STORAGE_KEY, value ? '1' : '0');
      } catch {
        // ignore storage errors (private mode, etc.)
      }
    },
  },
});
