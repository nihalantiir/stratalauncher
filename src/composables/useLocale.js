import { ref, watch } from 'vue';
import { i18n, SUPPORTED_LOCALES } from '../i18n';

// i18n/index.js already sets the correct initial locale at app boot (before
// this module ever loads, since Settings is a lazy-loaded route). This ref
// just mirrors that starting value and persists any change the user makes.
const locale = ref(i18n.global.locale.value);

watch(locale, (value) => {
  i18n.global.locale.value = value;
  try {
    localStorage.setItem('strata-locale', value);
  } catch {
    // ignore storage errors (private mode, etc.)
  }
});

export function useLocale() {
  return { locale, locales: SUPPORTED_LOCALES };
}
