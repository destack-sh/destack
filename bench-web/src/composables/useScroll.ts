import { useScroll, whenever } from "@vueuse/core";
import { ref, watch, type Ref } from "vue";

export const SCROLL_ACTIVE_TIMEOUT = 2000;

export function useActiveScroll(el: Ref<HTMLElement | undefined | null>) {
  const scroll = useScroll(el, { behavior: "smooth" });

  // add scroll-active whenever isScrolling for SCROLL_ACTIVE_TIMEOUT
  const timeout = ref<any>(null);
  watch(scroll.isScrolling, () => {
    // clear pending timeout
    if (timeout.value != null) {
      clearTimeout(timeout.value);
    }
    // add/remove scroll-active
    if (scroll.isScrolling.value) {
      el.value?.classList.add("scroll-active");
    } else {
      timeout.value = setTimeout(() => {
        el.value?.classList.remove("scroll-active");
      }, SCROLL_ACTIVE_TIMEOUT);
    }
  });

  return scroll;
}
