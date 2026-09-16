<script setup>
import { useI18n } from 'vue-i18n';
import ImportScanPanel from '../components/migration/ImportScanPanel.vue';

const { t } = useI18n();
</script>

<template>
  <section class="view">
    <p class="view-intro">{{ t('wizard.importBody') }}</p>
    <ImportScanPanel />
  </section>
</template>
