import { defineStore } from 'pinia';
import { listen } from '@tauri-apps/api/event';
import * as api from '../api/accounts';

export const useAccountsStore = defineStore('accounts', {
  state: () => ({
    accounts: [],
    active: null,
    loading: false,
    signingIn: false,
    loginStage: null,
    loginError: null,
  }),
  actions: {
    async refresh() {
      this.loading = true;
      try {
        const [accounts, active] = await Promise.all([api.listAccounts(), api.getActiveAccount()]);
        this.accounts = accounts;
        this.active = active;
      } finally {
        this.loading = false;
      }
    },

    async signInMicrosoft() {
      this.signingIn = true;
      this.loginError = null;
      this.loginStage = null;
      const unlisten = await listen('auth://login-progress', (event) => {
        this.loginStage = event.payload;
      });
      try {
        const account = await api.startMicrosoftLogin();
        await this.refresh();
        return account;
      } catch (e) {
        this.loginError = String(e);
        throw e;
      } finally {
        this.signingIn = false;
        this.loginStage = null;
        unlisten();
      }
    },

    cancelSignIn() {
      return api.cancelMicrosoftLogin();
    },

    async createOffline(username) {
      this.loginError = null;
      try {
        const account = await api.createOfflineProfile(username);
        await this.refresh();
        return account;
      } catch (e) {
        this.loginError = String(e);
        throw e;
      }
    },

    async setActive(id) {
      await api.setActiveAccount(id);
      await this.refresh();
    },

    async remove(id) {
      await api.removeAccount(id);
      await this.refresh();
    },
  },
});
