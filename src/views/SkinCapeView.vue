<script setup>
import { ref, computed, onMounted, onBeforeUnmount, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import { useAccountsStore } from '../stores/accounts';
import * as api from '../api/skins';
import { getCapeCatalog } from '../api/capeCatalog';
import { KNOWN_CAPES as BUNDLED_CAPES, CAPE_CATEGORIES } from '../lib/knownCapes';
import SkinView from '../components/skin/SkinView.vue';
import CapeThumb from '../components/skin/CapeThumb.vue';
import LoginModal from '../components/auth/LoginModal.vue';
import MinecraftLoading from '../components/common/MinecraftLoading.vue';

const { t } = useI18n();
const accounts = useAccountsStore();

const profile = ref(null);
const loading = ref(false);
const error = ref(null);
const loginOpen = ref(false);

// Mirrors the account's real classic/slim model until the user overrides it;
// see `discardAllChanges`/`refresh` for where it's kept in sync with Mojang.
const pendingVariant = ref('classic');
// null = no skin change queued; otherwise { kind: 'upload'|'reset', bytes?, previewUrl? }.
// Nothing reaches Mojang until Save; picking a file, toggling the model, choosing a cape, or Reset only updates local state.
const pendingSkin = ref(null);
// undefined = no cape change queued; null = "no cape" explicitly chosen;
// otherwise the chosen cape's id.
const pendingCapeId = ref(undefined);
const savingChanges = ref(false);
const capeBusy = ref(false); // kept only to gate the cape grid while a save is in flight

// 'owned' (the real grid, feeds pendingCapeId, actually saves to Mojang) or 'tryOn' (the curated
// known-capes browser below, preview only; never touches pendingCapeId, so Save sends nothing for it).
const capeBrowseMode = ref('owned');
const tryOnCapeUrl = ref(null);
// Starts with the list bundled into this build, replaced if a remote catalog is available (see loadCapeCatalog);
// no live Mojang API enumerates every cape that's ever existed, so this is always curated data, shipped or fetched.
const knownCapes = ref(BUNDLED_CAPES);

async function loadCapeCatalog() {
  try {
    const remote = await getCapeCatalog();
    if (Array.isArray(remote) && remote.length > 0) knownCapes.value = remote;
  } catch {
    // No repo configured, offline, or a malformed response; the bundled
    // list already assigned above stays in place, silently.
  }
}
// Purely cosmetic, real skinview3d option controlling which slot a worn cape renders into. Elytra rendering
// needs *some* cape texture loaded to show anything, owned or tried-on; it doesn't change what Save sends.
const backEquipment = ref('cape');

const realDefaultSkinUrl = ref(null);
// Gates the preview so it only ever shows a resolved, correct texture, never a placeholder or stale skin
// while a fetch is in flight. Real Steve (or the account's actual skin) or nothing, never in between.
const skinResolved = ref(false);

const isMicrosoft = computed(() => accounts.active?.kind === 'microsoft');
const activeSkin = computed(() => profile.value?.skins?.find((s) => s.state === 'ACTIVE') ?? null);
const activeCape = computed(() => profile.value?.capes?.find((c) => c.state === 'ACTIVE') ?? null);
const activeVariant = computed(() => (activeSkin.value?.variant?.toLowerCase() === 'slim' ? 'slim' : 'classic'));

const hasPendingChanges = computed(
  () =>
    pendingSkin.value !== null ||
    pendingCapeId.value !== undefined ||
    (isMicrosoft.value && pendingVariant.value !== activeVariant.value),
);

const previewSkinUrl = computed(() => {
  if (!skinResolved.value) return null;
  if (!isMicrosoft.value) return realDefaultSkinUrl.value;
  if (pendingSkin.value) {
    return pendingSkin.value.kind === 'reset' ? realDefaultSkinUrl.value : pendingSkin.value.previewUrl;
  }
  return activeSkin.value?.url ?? realDefaultSkinUrl.value;
});
// The model toggle always drives the preview live, whether it's shaping a freshly-picked file or just
// previewing the account's current skin in the other model; no need to save first to see it.
const previewModel = computed(() => {
  if (!isMicrosoft.value) return 'default';
  if (pendingSkin.value?.kind === 'reset') return 'default';
  return pendingVariant.value === 'slim' ? 'slim' : 'default';
});
const previewCapeUrl = computed(() => {
  if (!isMicrosoft.value || !skinResolved.value) return null;
  if (capeBrowseMode.value === 'tryOn') return tryOnCapeUrl.value;
  if (pendingCapeId.value !== undefined) {
    return pendingCapeId.value ? (profile.value?.capes?.find((c) => c.id === pendingCapeId.value)?.url ?? null) : null;
  }
  return activeCape.value?.url ?? null;
});

// Hides a known cape from "Try on" if the account already really owns it, since trying on something you can
// already just equip for real is confusing. Grouped by category so ~30 entries stays scannable.
const tryOnGroups = computed(() => {
  const ownedUrls = new Set((profile.value?.capes ?? []).map((c) => c.url));
  const available = knownCapes.value.filter((c) => !ownedUrls.has(c.url));
  return CAPE_CATEGORIES.map((cat) => ({ ...cat, capes: available.filter((c) => c.category === cat.id) })).filter(
    (group) => group.capes.length > 0,
  );
});

// Mojang doesn't document a rate limit for these endpoints, but repeated 429s reportedly risk account
// suspension, so this throttles client-side after every mutation; a real 429 ("Try again in Ns") overrides the default wait.
const DEFAULT_COOLDOWN_MS = 6000;
const cooldownActive = ref(false);
let cooldownTimer = null;

function startCooldown(errorMessage) {
  const match = errorMessage && String(errorMessage).match(/Try again in (\d+)s/);
  const ms = match ? Number(match[1]) * 1000 : DEFAULT_COOLDOWN_MS;
  cooldownActive.value = true;
  if (cooldownTimer) clearTimeout(cooldownTimer);
  cooldownTimer = setTimeout(() => {
    cooldownActive.value = false;
  }, ms);
}

async function loadRealDefaultSkin() {
  try {
    const bytes = await api.getDefaultSkin();
    const blob = new Blob([new Uint8Array(bytes)], { type: 'image/png' });
    if (realDefaultSkinUrl.value) URL.revokeObjectURL(realDefaultSkinUrl.value);
    realDefaultSkinUrl.value = URL.createObjectURL(blob);
  } catch {
    // No version downloaded yet; there is genuinely nothing real to show.
  }
}

async function refresh() {
  if (!isMicrosoft.value) return;
  loading.value = true;
  error.value = null;
  try {
    profile.value = await api.getSkinProfile();
    pendingVariant.value = activeVariant.value;
  } catch (e) {
    error.value = String(e);
  } finally {
    loading.value = false;
  }
}

async function loadAll() {
  skinResolved.value = false;
  discardAllChanges();
  capeBrowseMode.value = 'owned';
  tryOnCapeUrl.value = null;
  await Promise.all([loadRealDefaultSkin(), refresh()]);
  skinResolved.value = true;
}

onMounted(() => {
  loadAll();
  loadCapeCatalog();
});
watch(
  () => accounts.active?.id,
  () => loadAll(),
);
onBeforeUnmount(() => {
  if (realDefaultSkinUrl.value) URL.revokeObjectURL(realDefaultSkinUrl.value);
  discardPendingSkin();
  if (cooldownTimer) clearTimeout(cooldownTimer);
});

function discardPendingSkin() {
  if (pendingSkin.value?.kind === 'upload' && pendingSkin.value.previewUrl) {
    URL.revokeObjectURL(pendingSkin.value.previewUrl);
  }
  pendingSkin.value = null;
}

function discardAllChanges() {
  discardPendingSkin();
  pendingCapeId.value = undefined;
  pendingVariant.value = activeVariant.value;
  error.value = null;
}

function setCapeBrowseMode(mode) {
  capeBrowseMode.value = mode;
  if (mode === 'owned') tryOnCapeUrl.value = null;
}

function selectTryOnCape(url) {
  tryOnCapeUrl.value = tryOnCapeUrl.value === url ? null : url;
}

function onFileChange(event) {
  const file = event.target.files?.[0];
  event.target.value = '';
  if (!file) return;
  error.value = null;

  const url = URL.createObjectURL(file);
  const img = new Image();
  img.onload = async () => {
    if (img.naturalWidth !== 64 || (img.naturalHeight !== 64 && img.naturalHeight !== 32)) {
      error.value = t('skin.badDimensions');
      URL.revokeObjectURL(url);
      return;
    }
    discardPendingSkin();
    const buf = await file.arrayBuffer();
    pendingSkin.value = { kind: 'upload', bytes: Array.from(new Uint8Array(buf)), previewUrl: url };
  };
  img.onerror = () => {
    error.value = t('skin.badFile');
    URL.revokeObjectURL(url);
  };
  img.src = url;
}

function requestReset() {
  error.value = null;
  discardPendingSkin();
  pendingSkin.value = { kind: 'reset' };
}

function isCapeSelected(capeId) {
  const current = pendingCapeId.value !== undefined ? pendingCapeId.value : (activeCape.value?.id ?? null);
  return current === capeId;
}

function selectCape(capeId) {
  error.value = null;
  // Clicking back to whatever's actually equipped cancels the pending change
  // instead of queuing a real no-op equip/unequip call on Save.
  pendingCapeId.value = capeId === (activeCape.value?.id ?? null) ? undefined : capeId;
}

async function saveChanges() {
  if (!hasPendingChanges.value || savingChanges.value || cooldownActive.value) return;
  savingChanges.value = true;
  capeBusy.value = true;
  error.value = null;
  try {
    let newProfile = profile.value;
    if (pendingSkin.value) {
      newProfile =
        pendingSkin.value.kind === 'reset'
          ? await api.resetSkin()
          : await api.uploadSkin(pendingVariant.value, pendingSkin.value.bytes);
    } else if (pendingVariant.value !== activeVariant.value) {
      newProfile = await api.setSkinVariant(pendingVariant.value);
    }
    if (pendingCapeId.value !== undefined) {
      newProfile = pendingCapeId.value ? await api.equipCape(pendingCapeId.value) : await api.unequipCape();
    }
    profile.value = newProfile;
    discardAllChanges();
    await accounts.refresh();
    startCooldown(null);
  } catch (e) {
    error.value = String(e);
    startCooldown(error.value);
  } finally {
    savingChanges.value = false;
    capeBusy.value = false;
  }
}
</script>

<template>
  <section class="view">
    <div v-if="error" class="error-box">{{ error }}</div>
    <MinecraftLoading v-if="loading" :heading="t('skin.loadingHeading')" :subtext="t('skin.loadingSubtext')" />

    <div v-else class="skin-layout">
      <div class="panel skin-preview-panel">
        <SkinView :skin-url="previewSkinUrl" :cape-url="previewCapeUrl" :model="previewModel" :back-equipment="backEquipment" />
        <div class="segmented-toggle" style="margin-bottom: 0" role="group" :aria-label="t('skin.backEquipmentLabel')">
          <button
            type="button"
            class="segmented-opt"
            :class="{ selected: backEquipment === 'cape' }"
            :disabled="!previewCapeUrl"
            @click="backEquipment = 'cape'"
          >
            {{ t('skin.backEquipmentCape') }}
          </button>
          <button
            type="button"
            class="segmented-opt"
            :class="{ selected: backEquipment === 'elytra' }"
            :disabled="!previewCapeUrl"
            @click="backEquipment = 'elytra'"
          >
            {{ t('skin.backEquipmentElytra') }}
          </button>
        </div>
      </div>

      <div class="skin-controls">
        <template v-if="isMicrosoft">
          <div class="field-group">
            <h4>{{ t('skin.uploadHeading') }}</h4>
            <p class="hint">{{ t('skin.uploadHint') }}</p>

            <label class="btn btn-ghost btn-block file-upload-btn">
              <input type="file" accept="image/png" hidden @change="onFileChange" />
              {{ t('skin.chooseFile') }}
            </label>

            <div class="model-toggle loader-picker">
              <button
                type="button"
                class="loader-opt"
                :class="{ selected: pendingVariant === 'classic' }"
                @click="pendingVariant = 'classic'"
              >
                {{ t('skin.classic') }}
                <span class="lo-ver">{{ t('skin.classicHint') }}</span>
              </button>
              <button
                type="button"
                class="loader-opt"
                :class="{ selected: pendingVariant === 'slim' }"
                @click="pendingVariant = 'slim'"
              >
                {{ t('skin.slim') }}
                <span class="lo-ver">{{ t('skin.slimHint') }}</span>
              </button>
            </div>

            <button
              class="btn btn-danger-ghost btn-block reset-skin-btn"
              type="button"
              :disabled="savingChanges"
              @click="requestReset"
            >
              {{ t('skin.resetSkin') }}
            </button>
          </div>

          <div class="field-group">
            <h4>{{ t('skin.capesHeading') }}</h4>
            <p class="hint">{{ capeBrowseMode === 'tryOn' ? t('skin.capesTryOnHint') : t('skin.capesHint') }}</p>

            <div class="segmented-toggle">
              <button type="button" class="segmented-opt" :class="{ selected: capeBrowseMode === 'owned' }" @click="setCapeBrowseMode('owned')">
                {{ t('skin.capesOwned') }}
              </button>
              <button type="button" class="segmented-opt" :class="{ selected: capeBrowseMode === 'tryOn' }" @click="setCapeBrowseMode('tryOn')">
                {{ t('skin.capesTryOn') }}
              </button>
            </div>

            <template v-if="capeBrowseMode === 'owned'">
              <div v-if="!profile?.capes?.length" class="empty-state">{{ t('skin.noCapes') }}</div>
              <div v-else class="cape-grid">
                <button
                  type="button"
                  class="cape-card no-cape"
                  :class="{ selected: isCapeSelected(null) }"
                  :disabled="capeBusy"
                  @click="selectCape(null)"
                >
                  <span class="cape-thumb-canvas cape-none-icon"></span>
                  <span class="cape-name">{{ t('skin.noCape') }}</span>
                </button>
                <button
                  v-for="cape in profile.capes"
                  :key="cape.id"
                  type="button"
                  class="cape-card"
                  :class="{ selected: isCapeSelected(cape.id) }"
                  :disabled="capeBusy"
                  @click="selectCape(cape.id)"
                >
                  <CapeThumb :url="cape.url" />
                  <span class="cape-name">{{ cape.alias }}</span>
                </button>
              </div>
            </template>
            <template v-else>
              <div v-for="group in tryOnGroups" :key="group.id" class="cape-group">
                <div class="cape-group-label">{{ t(`skin.capeCategory.${group.id}`) }}</div>
                <div class="cape-grid">
                  <button
                    v-for="cape in group.capes"
                    :key="cape.id"
                    type="button"
                    class="cape-card"
                    :class="{ selected: tryOnCapeUrl === cape.url }"
                    @click="selectTryOnCape(cape.url)"
                  >
                    <CapeThumb :url="cape.url" />
                    <span class="cape-name">{{ cape.name }}</span>
                  </button>
                </div>
              </div>
            </template>
          </div>

          <Transition name="fade-slide">
            <div v-if="hasPendingChanges" class="save-bar">
              <span class="save-msg">{{ t('skin.unsavedChanges') }}</span>
              <div class="save-bar-actions">
                <button class="btn btn-ghost" type="button" :disabled="savingChanges" @click="discardAllChanges">
                  {{ t('skin.discardChanges') }}
                </button>
                <button
                  class="btn btn-mineral"
                  type="button"
                  :disabled="savingChanges || cooldownActive"
                  @click="saveChanges"
                >
                  <span v-if="savingChanges" class="spinner"></span>
                  {{ savingChanges ? t('skin.saving') : t('skin.saveChanges') }}
                </button>
              </div>
            </div>
          </Transition>
        </template>

        <div v-else class="field-group">
          <h4>{{ t('skin.offlineHeading') }}</h4>
          <p class="hint">{{ t('skin.offlineNote') }}</p>
          <button class="btn btn-mineral btn-block offline-signin-btn" type="button" @click="loginOpen = true">
            {{ t('login.signInWithMicrosoft') }}
          </button>
        </div>
      </div>
    </div>

    <Transition name="modal">
      <LoginModal v-if="loginOpen" @close="loginOpen = false" />
    </Transition>
  </section>
</template>
