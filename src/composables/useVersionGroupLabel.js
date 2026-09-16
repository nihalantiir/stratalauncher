import { useI18n } from 'vue-i18n';

/** Turns a versions.groupedSelectableVersions() entry into its <optgroup> label. */
export function useVersionGroupLabel() {
  const { t } = useI18n();
  return (group) => {
    if (group.key === 'old_alpha') return t('versions.type.old_alpha');
    if (group.key === 'old_beta') return t('versions.type.old_beta');
    return group.name ?? group.fallbackVersion;
  };
}
