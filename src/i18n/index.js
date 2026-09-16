import { createI18n } from 'vue-i18n';
import en from './locales/en.json';
import es from './locales/es.json';
import fr from './locales/fr.json';
import pl from './locales/pl.json';

export const SUPPORTED_LOCALES = ['en', 'es', 'fr', 'pl'];

// Reads the saved/detected locale eagerly, here rather than in useLocale.js,
// because this module is imported directly by main.js before the app
// mounts. useLocale.js only loads once the (lazy-loaded) Settings route is
// visited, so the app would otherwise always boot in the hardcoded default
// below and only pick up the real saved language after a Settings visit.
function detectLocale() {
  try {
    const stored = localStorage.getItem('strata-locale');
    if (stored && SUPPORTED_LOCALES.includes(stored)) return stored;
  } catch {
    // ignore storage errors (private mode, etc.)
  }
  const nav = (navigator.language || 'en').slice(0, 2).toLowerCase();
  return SUPPORTED_LOCALES.includes(nav) ? nav : 'en';
}

export const i18n = createI18n({
  legacy: false,
  locale: detectLocale(),
  fallbackLocale: 'en',
  messages: { en, es, fr, pl },
});
