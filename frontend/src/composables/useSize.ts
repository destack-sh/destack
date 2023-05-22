import { unrefElement, type MaybeComputedElementRef } from "@vueuse/core";
import { onUpdated, ref } from "vue";

export function useElementSize(target: MaybeComputedElementRef) {
  /* 
  A very efficient and simple element size observer
  The default useElementSize by vueuse is based on ResizeObserver + watch,
  which seems to want to trigger updates for every component in sequence (one component per frame), 
  I have no idea why, but that causes a lot of lag, especially when using many sizes (e.g. in a grid).
   */
  const width = ref(0);
  const height = ref(0);
  // manually track last width/height because our value refs are also based on watches
  const prevWidth = ref(0);
  const prevHeight = ref(0);

  onUpdated(() => {
    const el = unrefElement(target);
    if (el) {
      const rect = el.getBoundingClientRect();
      if (rect.width != prevWidth.value) {
        prevWidth.value = rect.width;
        width.value = rect.width;
      }
      if (rect.height != prevHeight.value) {
        prevHeight.value = rect.height;
        height.value = rect.height;
      }
    }
  });

  return {
    width,
    height,
  };
}
