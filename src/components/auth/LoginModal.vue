<script setup>
import { ref, computed } from 'vue';
import { useI18n } from 'vue-i18n';
import { openUrl } from '@tauri-apps/plugin-opener';
import { useAccountsStore } from '../../stores/accounts';

const emit = defineEmits(['close']);
const { t } = useI18n();
const accounts = useAccountsStore();

const tab = ref('microsoft');
const username = ref('');
const usernameError = ref(null);
const codeCopied = ref(false);

const stage = computed(() => accounts.loginStage?.stage ?? null);
const isCancelled = computed(() => accounts.loginError === 'auth error: Sign-in cancelled.');

async function beginMicrosoftSignIn() {
  try {
    await accounts.signInMicrosoft();
    emit('close');
  } catch {
    // error surfaced via accounts.loginError
  }
}

function cancel() {
  accounts.cancelSignIn();
}

async function openVerificationUrl() {
  if (accounts.loginStage?.verificationUri) {
    await openUrl(accounts.loginStage.verificationUri);
  }
}

async function copyCode() {
  const code = accounts.loginStage?.userCode;
  if (!code) return;
  try {
    await navigator.clipboard.writeText(code);
    codeCopied.value = true;
    setTimeout(() => (codeCopied.value = false), 1500);
  } catch {
    // clipboard unavailable; the code is already shown on screen
  }
}

function validateUsername(value) {
  return /^[A-Za-z0-9_]{3,16}$/.test(value);
}

async function continueOffline() {
  if (!validateUsername(username.value)) {
    usernameError.value = t('login.usernameHint');
    return;
  }
  usernameError.value = null;
  try {
    await accounts.createOffline(username.value);
    emit('close');
  } catch {
    // error surfaced via accounts.loginError
  }
}
</script>

<template>
  <Teleport to="body">
  <div class="modal-backdrop" @click.self="emit('close')">
    <div class="modal">
      <div class="modal-head">
        <h3>{{ t('login.title') }}</h3>
        <button class="modal-close" type="button" @click="emit('close')">✕</button>
      </div>
      <div class="modal-body">
        <div class="tab-row">
          <button class="tab-btn" :class="{ on: tab === 'microsoft' }" type="button" @click="tab = 'microsoft'">
            {{ t('login.tabMicrosoft') }}
          </button>
          <button class="tab-btn" :class="{ on: tab === 'offline' }" type="button" @click="tab = 'offline'">
            {{ t('login.tabOffline') }}
          </button>
        </div>

        <!-- Microsoft sign-in -->
        <div v-if="tab === 'microsoft'">
          <p class="view-intro">{{ t('login.msDescription') }}</p>

          <div v-if="accounts.loginError && !isCancelled" class="error-box">{{ accounts.loginError }}</div>

          <template v-if="!accounts.signingIn">
            <button class="btn btn-mineral btn-block" type="button" @click="beginMicrosoftSignIn">
              {{ t('login.signInWithMicrosoft') }}
            </button>
          </template>

          <template v-else>
            <div v-if="stage === 'awaiting-code'" class="device-code">
              <p class="view-intro" style="margin: 0">
                {{ t('login.enterCode', { url: accounts.loginStage.verificationUri }) }}
              </p>
              <div class="code mono">{{ accounts.loginStage.userCode }}</div>
              <div style="display: flex; gap: 8px; justify-content: center; margin-bottom: 14px">
                <button class="btn btn-mineral btn-sm" type="button" @click="openVerificationUrl">
                  {{ t('login.openBrowser') }}
                </button>
                <button class="btn btn-ghost btn-sm" type="button" @click="copyCode">
                  {{ codeCopied ? t('login.codeCopied') : t('login.copyCode') }}
                </button>
              </div>
              <div style="display: flex; align-items: center; justify-content: center; gap: 8px; color: var(--text-muted); font-size: 12.5px">
                <span class="spinner"></span>{{ t('login.waitingForSignIn') }}
              </div>
            </div>

            <div v-else style="display: flex; align-items: center; justify-content: center; gap: 10px; padding: 24px 0">
              <span class="spinner"></span>
              <span style="font-size: 13px; color: var(--text-muted)">
                {{
                  stage === 'signing-into-xbox'
                    ? t('login.stageXbox')
                    : stage === 'signing-into-minecraft'
                    ? t('login.stageMinecraft')
                    : t('login.stageProfile')
                }}
              </span>
            </div>

            <button class="btn btn-ghost btn-block" type="button" @click="cancel">{{ t('login.cancel') }}</button>
          </template>
        </div>

        <!-- Offline mode -->
        <div v-else>
          <p class="view-intro">{{ t('login.offlineDescription') }}</p>

          <div v-if="accounts.loginError" class="error-box">{{ accounts.loginError }}</div>

          <label class="field-label" for="offline-username">{{ t('login.usernameLabel') }}</label>
          <input
            id="offline-username"
            v-model="username"
            class="field"
            :placeholder="t('login.usernamePlaceholder')"
            maxlength="16"
            @keyup.enter="continueOffline"
          />
          <p v-if="usernameError" style="color: var(--danger); font-size: 12px; margin: 6px 0 0">{{ usernameError }}</p>
          <p v-else style="color: var(--text-muted); font-size: 12px; margin: 6px 0 0">{{ t('login.usernameHint') }}</p>

          <button class="btn btn-mineral btn-block" type="button" style="margin-top: 16px" @click="continueOffline">
            {{ t('login.continueOffline') }}
          </button>
        </div>
      </div>
    </div>
  </div>
  </Teleport>
</template>
