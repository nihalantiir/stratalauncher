<script setup>
// A loading state for real network waits (the Skin & Cape page's Mojang round-trip). The chicken
// jockey gif was supplied directly by the user, so it avoids the "no real Minecraft textures" concern.
defineProps({
  heading: { type: String, required: true },
  subtext: { type: String, default: '' },
});
</script>

<template>
  <div class="mc-loading">
    <h2 class="mc-loading-heading">{{ heading }}</h2>
    <img src="/loading/chicken-jockey.gif" alt="" class="mc-loading-jockey" />
    <p v-if="subtext" class="mc-loading-subtext">{{ subtext }}</p>
  </div>
</template>

<style scoped>
.mc-loading {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 12px;
  padding: 56px 20px;
  text-align: center;
}
.mc-loading-heading {
  font-family: var(--font-mc);
  font-size: 32px;
  font-weight: 400;
  letter-spacing: 0.01em;
  margin: 0;
  color: var(--text);
}
.mc-loading-jockey {
  width: 120px;
  height: 120px;
  object-fit: contain;
}
.mc-loading-subtext {
  font-family: var(--font-mc-body);
  font-size: 13px;
  color: var(--text-muted);
  margin: 0;
}
</style>
