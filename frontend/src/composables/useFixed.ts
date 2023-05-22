import { computed, inject, ref, watch, type Ref } from "vue";
import { type EditorContext, EDITOR_CONTEXT } from "@/state/editor";

export const VIEW_MARGIN = 8;

export function pinAbsoluteElement(
  el: Ref<null | HTMLElement>,
  fix: {
    pos?: boolean;
    width?: boolean;
    height?: boolean;
    keepInView?: boolean;
  }
) {
  // fixes the element at the first available position
  const fixed: Ref<{ x: number; y: number; width: number; height: number } | null> = ref(null);
  const editorContext = inject<EditorContext>(EDITOR_CONTEXT);
  if (fix.keepInView && editorContext == null) {
    throw new Error("keepInView requires editor context");
  }
  if (fix.keepInView && !fix.pos) {
    throw new Error("keepInView requires pos");
  }

  watch(el, (el) => {
    if (el == null && fixed.value != null) fixed.value = null; // reset
    if (el == null) return; // no element
    if (fixed.value != null) return; // already fixed
    // fix element
    const rect = el.getBoundingClientRect();
    fixed.value = { x: rect.left, y: rect.top, width: 200, height: rect.height };

    if (fix.keepInView && editorContext != null) {
      const cpos = editorContext.pos.value;
      const crect = editorContext.size.value;
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
