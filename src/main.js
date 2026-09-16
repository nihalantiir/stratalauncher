import { createApp } from 'vue';
import { createPinia } from 'pinia';
import App from './App.vue';
import { i18n } from './i18n';
import { router } from './router';
import { vTooltip } from './directives/tooltip';
// Eager, not lazy: applies the saved UI scale before first paint. Importing
// this only from SettingsView would mean the scale doesn't apply until Settings is actually visited.
import './composables/useAccessibility';
// Same eager-import reasoning as useAccessibility above: density must be
// applied before first paint, not only once Settings happens to be visited.
import './composables/useDensity';
import './styles/tokens.css';
import './styles/fonts.css';
import './styles/base.css';

createApp(App).use(createPinia()).use(i18n).use(router).directive('tooltip', vTooltip).mount('#app');
