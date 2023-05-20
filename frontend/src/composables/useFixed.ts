import { computed, ref, watch, type Ref } from "vue";

export function pinAbsoluteElement(
  el: Ref<null | HTMLElement>,
  fix: {
    pos?: boolean;
    width?: boolean;
    height?: boolean;
    pushIntoView?: boolean;
  }
) {
  // fixes the element at the first available position
  const fixed: Ref<{ x: number; y: number; width: number; height: number } | null> = ref(null);

  watch(el, (el) => {
    if (el == null && fixed.value != null) fixed.value = null;
    if (fixed.value != null) return;
    if (el == null) return;
    // fix element
    const rect = el.getBoundingClientRect();
    fixed.value = { x: rect.left, y: rect.top, width: 200, height: rect.height };
    if (fix.pos) {
      el.style.position = "fixed";
      el.style.left = `${rect.x}px`;
      el.style.top = `${rect.y}px`;
    }
    if (fix.width) el.style.width = `${rect.width}px`;
    if (fix.height) el.style.height = `${rect.height}px`;
  });

  return {
    fixed,
    pinned: computed(() => fixed.value != null),
  };
}
