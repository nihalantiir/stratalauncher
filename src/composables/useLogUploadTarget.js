import { ref, watch } from 'vue';

// Which log-upload destination Upload uses: a persistent Settings choice (default mclo.gs,
// needing no credential), not a picker shown inline each time. Same shape as useDensity.
function detectTarget() {
  try {
    const stored = localStorage.getItem('strata-log-upload-target');
    if (stored === 'mclogs' || stored === 'pastebin') return stored;
  } catch {
    // ignore storage errors
  }
  return 'mclogs';
}

const uploadTarget = ref(detectTarget());

watch(uploadTarget, (value) => {
  try {
    localStorage.setItem('strata-log-upload-target', value);
  } catch {
    // ignore storage errors
  }
});

export const UPLOAD_TARGET_LABELS = { mclogs: 'mclo.gs', pastebin: 'Pastebin' };

export function useLogUploadTarget() {
  return { uploadTarget };
}
