import { defineStore } from 'pinia';
import * as api from '../api/notifications';

export const useNotificationsStore = defineStore('notifications', {
  state: () => ({
    list: [],
  }),
  actions: {
    async refresh() {
      this.list = await api.listNotifications();
    },
    async dismiss(id) {
      await api.dismissNotification(id);
      await this.refresh();
    },
    async clearAll() {
      await Promise.all(this.list.map((n) => api.dismissNotification(n.id)));
      await this.refresh();
    },
    async runSyncCheck() {
      this.list = await api.runSyncCheck();
    },
  },
});
