import { createRouter, createWebHashHistory } from 'vue-router';
// Library is the app's landing page, kept as a static import so first paint never waits on an
// extra chunk fetch. Every other route is lazy since SkinCapeView alone pulls in skinview3d (three.js).
import LibraryView from '../views/LibraryView.vue';

export const router = createRouter({
  history: createWebHashHistory(),
  routes: [
    { path: '/', name: 'library', component: LibraryView },
    { path: '/version', name: 'version', component: () => import('../views/VersionView.vue') },
    { path: '/mods', name: 'mods', component: () => import('../views/ContentView.vue'), props: { kind: 'mod' } },
    {
      path: '/resourcepacks',
      name: 'resourcepacks',
      component: () => import('../views/ContentView.vue'),
      props: { kind: 'resourcepack' },
    },
    { path: '/shaders', name: 'shaders', component: () => import('../views/ContentView.vue'), props: { kind: 'shader' } },
    { path: '/worlds', name: 'worlds', component: () => import('../views/WorldsView.vue') },
    { path: '/servers', name: 'servers', component: () => import('../views/ServersView.vue') },
    { path: '/screenshots', name: 'screenshots', component: () => import('../views/ScreenshotsView.vue') },
    { path: '/logs', name: 'logs', component: () => import('../views/LogsView.vue') },
    { path: '/instance-settings', name: 'instanceSettings', component: () => import('../views/InstanceSettingsView.vue') },
    { path: '/skin', name: 'skin', component: () => import('../views/SkinCapeView.vue') },
    { path: '/import', name: 'import', component: () => import('../views/ImportView.vue') },
    { path: '/settings', name: 'settings', component: () => import('../views/SettingsView.vue') },
  ],
});
