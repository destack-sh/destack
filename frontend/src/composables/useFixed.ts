import { computed, inject, ref, watch, type Ref } from "vue";
import { type PanelContext, PANEL_CONTEXT } from "@/state/bench";
import { unrefElement } from "@vueuse/core";

export const VIEW_MARGIN = 8;

export function pinAbsoluteElement(
  target: Ref<HTMLElement | any | null>,
  fix: {
    pos?: boolean;
    width?: boolean;
    height?: boolean;
    keepInView?: boolean;
    sourcePos?: Ref<{ x: number; y: number } | null>;
  }
) {
  // fixes the element at the first available position
  const fixed: Ref<{ x: number; y: number; width: number; height: number } | null> = ref(null);
  const panelContext = inject<PanelContext<any> | null>(PANEL_CONTEXT, null);
  if (fix.keepInView && panelContext == null) {
    throw new Error("keepInView requires editor context");
  }
  if (fix.keepInView && !fix.pos) {
    throw new Error("keepInView requires pos");
  }

  watch([target, () => fix.sourcePos?.value, () => panelContext?.pos.value, () => panelContext?.size.value], () => {
    const el = unrefElement(target);
    if (el == null && fixed.value != null) fixed.value = null; // reset
    if (el == null) return; // no element
    if (fixed.value != null && !fix.keepInView) return; // already fixed
    // (re)fix element in view
    const rect = el.getBoundingClientRect();
    if (fix.sourcePos?.value != null) {
      // use given pos
      rect.x = fix.sourcePos.value.x;
      rect.y = fix.sourcePos.value.y;
    }
    fixed.value = { x: rect.left, y: rect.top, width: rect.width, height: rect.height };
    if (fix.keepInView && panelContext != null) {
      const cpos = panelContext.pos.value;
      const crect = panelContext.size.value;
      if (rect.x + rect.width > cpos.left + crect.width - VIEW_MARGIN) {
        // crosses on the right, move left
        fixed.value.x -= rect.right - (cpos.left + crect.width - VIEW_MARGIN);
      }
      if (rect.y + rect.height > cpos.top + crect.height - VIEW_MARGIN) {
        // crosses on the bottom, move up
        fixed.value.y -= rect.bottom - (cpos.top + crect.height - VIEW_MARGIN);
      }
      if (rect.left < cpos.left + VIEW_MARGIN) {
        // crosses on the left, move right
        fixed.value.x += cpos.left + VIEW_MARGIN - rect.left;
      }
      if (rect.top < cpos.top + VIEW_MARGIN) {
        // crosses on the top, move down
        fixed.value.y += cpos.top + VIEW_MARGIN - rect.top;
      }
    }

    if (fix.pos) {
      el.style.position = "fixed";
      el.style.left = `${fixed.value.x}px`;
      el.style.top = `${fixed.value.y}px`;
    }
    if (fix.width) el.style.width = `${rect.width}px`;
    if (fix.height) el.style.height = `${rect.height}px`;
  });

  return {
    fixed,
    pinned: computed(() => fixed.value != null),
  };
}
