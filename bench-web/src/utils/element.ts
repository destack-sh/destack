import { unrefElement, type MaybeComputedElementRef } from "@vueuse/core";
import { onUpdated, ref, watch, type Ref, inject, computed } from "vue";

export function useElementSize(target: MaybeComputedElementRef) {
  /* 
  A very efficient and simple element size observer
  The default useElementSize by vueuse is based on ResizeObserver + watch,
  which seems to want to trigger updates for every component in sequence (one component per frame), 
  I have no idea why, but that causes a lot of lag, especially when using many sizes (e.g. in a grid).
  Do *not* use on components that are frequently updated (e.g. editor containers).
   */
  const width = ref(0);
  const height = ref(0);

  onUpdated(() => {
    const el = unrefElement(target);
    if (el) {
      const rect = el.getBoundingClientRect();
      if (rect.width != width.value) {
        width.value = rect.width;
      }
      if (rect.height != height.value) {
        height.value = rect.height;
      }
    }
  });

  return {
    width,
    height,
  };
}

export const VIEW_MARGIN = 8;

export function pinAbsoluteElement(
  target: Ref<HTMLElement | any | null>,
  fix: {
    pos?: boolean;
    width?: boolean;
    height?: boolean;
    keepInView?: boolean;
    sourcePos?: Ref<{ x: number; y: number } | null>;
  },
) {
  // fixes the element at the first available position
  const fixed: Ref<{ x: number; y: number; width: number; height: number } | null> = ref(null);
  // const panelContext = inject<PanelContext<any> | null>(PANEL_CONTEXT, null);
  const view: any = () => {
    throw new Error("not yet implemented - where to get view context?");
    return {} as any;
  };
  if (fix.keepInView && view == null) {
    throw new Error("keepInView requires editor context");
  }
  if (fix.keepInView && !fix.pos) {
    throw new Error("keepInView requires pos");
  }

  watch([target, () => fix.sourcePos?.value, () => view?.pos.value, () => view?.size.value], () => {
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
    if (fix.keepInView && view != null) {
      const cpos = view.pos.value;
      const crect = view.size.value;
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
