import type { ViewComponent } from "@/views/common";
import {
  unrefElement,
  useEventListener,
  useMutationObserver,
  useResizeObserver,
  type MaybeComputedElementRef,
  type MaybeElement,
} from "@vueuse/core";
import {
  computed,
  onUpdated,
  ref,
  watch,
  type ComponentInstance,
  type ComponentPublicInstance,
  type Directive,
  type Ref,
} from "vue";

export function blurDocument() {
  if (document.activeElement instanceof HTMLElement) document.activeElement.blur();
}

/** Gets the underlying HTML/SVG element of an HTML/SVG/Vue thing */
export function getElement(el: MaybeElement | ViewComponent): HTMLElement | SVGElement | null {
  if (el instanceof HTMLElement || el instanceof SVGElement) return el;
  else if ((el as ComponentPublicInstance<any>).$el) return (el as ComponentPublicInstance<any>).$el;
  else return (el as ComponentInstance<any>).subTree?.el as HTMLElement;
}

export function getElementRef(el: Ref<MaybeElement | ViewComponent>): Ref<HTMLElement | SVGElement | null> {
  return computed(() => (el.value != null ? getElement(el.value) : null));
}

export function useElementSizeOnUpdateOnly(target: MaybeComputedElementRef) {
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

export interface UseElementBoundingOptions {
  windowResize?: boolean;
  /**
   * Listen to window scroll event
   *
   * @default true
   */
  windowScroll?: boolean;

  /**
   * Immediately call update on component mounted
   *
   * @default true
   */
  immediate?: boolean;
}

/**
 * React to an element's bounding rect changes.
 */
export function watchElementBounding(
  target: MaybeComputedElementRef,
  callback: (el: HTMLElement | SVGElement) => void,
  options: {
    windowResize?: boolean;
    windowScroll?: boolean;
  } = {},
) {
  const { windowResize = true, windowScroll = true } = options;

  function trigger() {
    const el = unrefElement(target);
    if (el != null) callback(el);
  }

  useResizeObserver(target, trigger);
  watch(
    () => unrefElement(target),
    (ele) => !ele && trigger(),
  );
  // trigger by css or style
  useMutationObserver(target, trigger, {
    attributeFilter: ["style", "class"],
  });

  if (windowScroll) useEventListener("scroll", trigger, { capture: true, passive: true });
  if (windowResize) useEventListener("resize", trigger, { passive: true });
}
