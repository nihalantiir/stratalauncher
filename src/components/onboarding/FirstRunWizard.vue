<script setup>
import { ref } from 'vue';
import { useI18n } from 'vue-i18n';
import LoginModal from '../auth/LoginModal.vue';
import ImportScanPanel from '../migration/ImportScanPanel.vue';

const emit = defineEmits(['done']);
const { t } = useI18n();

const step = ref('welcome'); // welcome -> signin -> import

function startSignIn() {
  step.value = 'signin';
}
function afterSignIn() {
  step.value = 'import';
}
function finish() {
  emit('done');
}
</script>

<template>
  <div class="wizard-backdrop">
    <div class="wizard-card">
      <template v-if="step === 'welcome'">
        <div class="wizard-mark">S</div>
        <h1>{{ t('wizard.welcomeTitle') }}</h1>
        <p class="wizard-sub">{{ t('wizard.welcomeBody') }}</p>
        <button class="btn btn-mineral btn-block" type="button" style="margin-top: 22px" @click="startSignIn">
          {{ t('wizard.getStarted') }}
        </button>
        <button class="btn btn-ghost btn-block" type="button" style="margin-top: 10px" @click="finish">
          {{ t('wizard.skipAll') }}
        </button>
      </template>

      <template v-else-if="step === 'import'">
        <h2 style="font-size: 18px; margin: 0 0 8px; text-align: left">{{ t('wizard.importTitle') }}</h2>
        <p class="wizard-sub" style="text-align: left; margin-bottom: 16px">{{ t('wizard.importBody') }}</p>
        <ImportScanPanel @done="finish" />
        <button class="btn btn-ghost btn-block" type="button" style="margin-top: 14px" @click="finish">
          {{ t('wizard.skip') }}
        </button>
      </template>
    </div>

    <Transition name="modal">
      <LoginModal v-if="step === 'signin'" @close="afterSignIn" />
    </Transition>
  </div>
</template>
