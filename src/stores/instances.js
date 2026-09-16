import { defineStore } from 'pinia';
import { listen } from '@tauri-apps/api/event';
import * as api from '../api/instances';
import * as launchApi from '../api/launch';
import { router } from '../router';

let runningTrackingStarted = false;

export const useInstancesStore = defineStore('instances', {
  state: () => ({
    list: [],
    current: null,
    loading: false,
    error: null,
    runningIds: new Set(),
  }),
  actions: {
    async refresh() {
      this.loading = true;
      this.error = null;
      try {
        const [list, current] = await Promise.all([api.listInstances(), api.getCurrentInstance()]);
        this.list = list;
        this.current = current;
      } catch (e) {
        this.error = String(e);
      } finally {
        this.loading = false;
      }
    },

    async create(name, mcVersion, iconBiome, loader = null, loaderVersion = null, groupName = null) {
      const instance = await api.createInstance(name, mcVersion, iconBiome, loader, loaderVersion, groupName);
      await this.refresh();
      return instance;
    },

    async update(instance) {
      await api.updateInstance(instance);
      await this.refresh();
    },

    async setCurrent(id) {
      await api.setCurrentInstance(id);
      await this.refresh();
    },

    async remove(id) {
      await api.deleteInstance(id);
      await this.refresh();
    },

    async setLoader(id, loader, loaderVersion, mcVersion) {
      await api.setInstanceLoader(id, loader, loaderVersion, mcVersion);
      await this.refresh();
    },

    async setCustomClientJar(id, path) {
      await api.setCustomClientJar(id, path);
      await this.refresh();
    },

    isRunning(id) {
      return this.runningIds.has(id);
    },

    async stopInstance(id) {
      await launchApi.stopInstance(id);
    },

    // One-time (idempotent) hookup of live running-state tracking, safe to
    // call from every component that cares; only the first call does anything.
    async initRunningTracking() {
      if (runningTrackingStarted) return;
      runningTrackingStarted = true;

      try {
        const running = await launchApi.listRunningInstances();
        this.runningIds = new Set(running);
      } catch {
        // non-fatal, running-state just starts empty
      }

      await listen('instance://running-changed', (event) => {
        const { instanceId, running, crashed } = event.payload;
        const next = new Set(this.runningIds);
        if (running) next.add(instanceId);
        else next.delete(instanceId);
        this.runningIds = next;

        // Strata has no separate native console window, so "show logs on
        // launch" navigates here instead, but only for the currently-selected instance, so a background session elsewhere doesn't yank the view away.
        if (instanceId === this.current?.id) {
          const mode = this.current?.consoleMode ?? 'never';
          if (running && mode === 'always') {
            router.push('/logs');
          } else if (!running && mode === 'on_crash' && crashed) {
            router.push('/logs');
          }
        }

        if (!running) {
          // A session just ended, so lastPlayedAt/lastCrashed changed server-side.
          this.refresh();
        }
      });
    },
  },
});
