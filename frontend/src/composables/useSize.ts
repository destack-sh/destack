import { unrefElement, type MaybeComputedElementRef } from "@vueuse/core";
import { onUpdated, ref } from "vue";

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
