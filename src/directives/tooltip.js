import { useTooltip } from '../composables/useTooltip';

const { showAt, moveTo, hide } = useTooltip();
const SHOW_DELAY = 450;

// v-tooltip="text": a styled replacement for the native title attribute's unstyled OS popup.
// An empty/falsy value is a no-op, same as an empty title attribute, so call sites need no v-if guard.
export const vTooltip = {
  mounted(el, binding) {
    el.__tooltipText = binding.value;
    let timer = null;

    el.__ttEnter = (event) => {
      if (!el.__tooltipText) return;
      clearTimeout(timer);
      timer = setTimeout(() => showAt(event.clientX, event.clientY, el.__tooltipText), SHOW_DELAY);
    };
    el.__ttMove = (event) => moveTo(event.clientX, event.clientY);
    el.__ttLeave = () => {
      clearTimeout(timer);
      hide();
    };
    el.__ttFocus = () => {
      if (!el.__tooltipText) return;
      const rect = el.getBoundingClientRect();
      showAt(rect.left + rect.width / 2, rect.bottom, el.__tooltipText);
    };

    el.addEventListener('mouseenter', el.__ttEnter);
    el.addEventListener('mousemove', el.__ttMove);
    el.addEventListener('mouseleave', el.__ttLeave);
    el.addEventListener('mousedown', el.__ttLeave);
    el.addEventListener('focus', el.__ttFocus);
    el.addEventListener('blur', el.__ttLeave);
  },
  updated(el, binding) {
    el.__tooltipText = binding.value;
  },
  beforeUnmount(el) {
    el.removeEventListener('mouseenter', el.__ttEnter);
    el.removeEventListener('mousemove', el.__ttMove);
    el.removeEventListener('mouseleave', el.__ttLeave);
    el.removeEventListener('mousedown', el.__ttLeave);
    el.removeEventListener('focus', el.__ttFocus);
    el.removeEventListener('blur', el.__ttLeave);
    hide();
  },
};
