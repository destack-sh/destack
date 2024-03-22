import { getElement, watchElementBounding } from "@/utils/element";
import { unrefElement, type MaybeElement } from "@vueuse/core";
import { shallowRef, toRef, watch, type MaybeRef, type Ref } from "vue";

export type FloatingPlacement =
  | "top"
  | "top-left"
  | "top-right"
  | "right"
  | "right-top"
  | "right-bottom"
  | "bottom"
  | "bottom-left"
  | "bottom-right"
  | "left"
  | "left-top"
  | "left-bottom";

export type FloatingOptions = {
  placement: FloatingPlacement; // relative to the reference
  containerMargin?: number; // margin around the container
  referenceMargin?: number; // margin around the reference
};

/**
 * Gets the position of the floating element relative to the reference while staying in the container.
 * The arrow always points to the relative center of the reference (up to the floating size - margin).
 */
export function getFloatingPosition(float: {
  floating: { width: number; height: number };
  reference: { x: number; y: number; width: number; height: number };
  container: { x: number; y: number; width: number; height: number };
  options: FloatingOptions;
}): { x: number; y: number; placement: FloatingPlacement } {
  const { floating, reference, container, options } = float;
  let x = 0;
  let y = 0;
  const referenceMargin = options.referenceMargin ?? 0;
  const containerMargin = options.containerMargin ?? 0;
  let placement = options.placement;

  const recomputePosition = () => {
    switch (placement) {
      case "top":
        x = reference.x + reference.width / 2 - floating.width / 2;
        y = reference.y - floating.height - referenceMargin;
        break;
      case "top-left":
        x = reference.x;
        y = reference.y - floating.height - referenceMargin;
        break;
      case "top-right":
        x = reference.x + reference.width - floating.width;
        y = reference.y - floating.height - referenceMargin;
        break;
      case "right":
        x = reference.x + reference.width + referenceMargin;
        y = reference.y + reference.height / 2 - floating.height / 2;
        break;
      case "right-top":
        x = reference.x + reference.width + referenceMargin;
        y = reference.y;
        break;
      case "right-bottom":
        x = reference.x + reference.width + referenceMargin;
        y = reference.y + reference.height - floating.height;
        break;
      case "bottom":
        x = reference.x + reference.width / 2 - floating.width / 2;
        y = reference.y + reference.height + referenceMargin;
        break;
      case "bottom-left":
        x = reference.x;
        y = reference.y + reference.height + referenceMargin;
        break;
      case "bottom-right":
        x = reference.x + reference.width - floating.width;
        y = reference.y + reference.height + referenceMargin;
        break;
      case "left":
        x = reference.x - floating.width - referenceMargin;
        y = reference.y + reference.height / 2 - floating.height / 2;
        break;
      case "left-top":
        x = reference.x - floating.width - referenceMargin;
        y = reference.y;
        break;
      case "left-bottom":
        x = reference.x - floating.width - referenceMargin;
        y = reference.y + reference.height - floating.height;
        break;
    }
  };

  // compute initial position
  recomputePosition();

  // flip and recompute if needed
  const outOfBounds = {
    top: y < container.y,
    right: x + floating.width > container.x + container.width,
    bottom: y + floating.height > container.y + container.height,
    left: x < container.x,
  };
  if (outOfBounds.top && placement.startsWith("top"))
    placement = placement.replace("top", "bottom") as FloatingPlacement;
  else if (outOfBounds.bottom && placement.startsWith("bottom"))
    placement = placement.replace("bottom", "top") as FloatingPlacement;
  else if (outOfBounds.right && placement.startsWith("right"))
    placement = placement.replace("right", "left") as FloatingPlacement;
  else if (outOfBounds.left && placement.startsWith("left"))
    placement = placement.replace("left", "right") as FloatingPlacement;
  if (options.placement != placement) {
    recomputePosition();
  }

  // fit floating to container bounds
  x = Math.max(
    container.x + containerMargin,
    Math.min(container.x + container.width - floating.width - containerMargin, x),
  );
  y = Math.max(
    container.y + containerMargin,
    Math.min(container.y + container.height - floating.height - containerMargin, y),
  );

  return { x, y, placement };
}

/** Finds the closest floating container */
export function findFloatingContainer(el: HTMLElement | SVGElement | null): HTMLElement | SVGElement | null {
  while (el != null) {
    if (el.hasAttribute("data-root-element")) return el;
    el = el.parentElement;
  }
  return null;
}

/**
 * Positions the floating element to 'float' as requested relative to the reference, while staying in the container.
 * We automatically look for a container element with 'data-root-element', else use the viewport.
 *  */
export function useFloating(float: {
  floating: Ref<MaybeElement>;
  reference: Ref<MaybeElement>;
  container?: MaybeElement;
  options?: MaybeRef<Partial<FloatingOptions>>;
  enabled?: Ref<boolean>;
  watchElements?: boolean;
}): {
  recompute: () => void;
  floatingPosition: Ref<{ x: number; y: number }>;
  placement: Ref<FloatingPlacement | null>;
} {
  const optionsRef = toRef(float.options ?? shallowRef({})) as Ref<Partial<FloatingOptions>>;
  const floatingPosition = shallowRef({ x: 0, y: 0 });
  const placement: Ref<FloatingPlacement | null> = shallowRef(null);
  const foundContainer = shallowRef<HTMLElement | SVGElement | null | undefined>(unrefElement(float.container));

  // recomputes & applies the floating (and arrow) position
  const recompute = () => {
    // get elements bounding
    const floating = getElement(float.floating.value);
    const reference = getElement(float.reference.value);
    if (floating == null) throw new Error("floating element does not exist");
    if (reference == null) throw new Error("reference element does not exist");
    const floatingRect = floating.getBoundingClientRect();
    const referenceRect = reference.getBoundingClientRect();
    if (foundContainer.value === undefined) {
      foundContainer.value = findFloatingContainer(floating);
    }
    const containerRect =
      foundContainer.value?.getBoundingClientRect() ?? document.documentElement.getBoundingClientRect();

    // recompute positions in fixed coordinate space
    const options = { placement: "top", arrow: false, ...optionsRef.value } as FloatingOptions;
    const newFloat = getFloatingPosition({
      floating: { width: floatingRect.width, height: floatingRect.height },
      reference: referenceRect,
      container: containerRect,
      options,
    });
    floatingPosition.value = { x: newFloat.x, y: newFloat.y };
    placement.value = newFloat.placement;

    // apply positions (fixed)
    // TODO :Robustness: :UI: use absolute positioning for floating elements?
    floating.style.position = "fixed";
    floating.style.left = `${floatingPosition.value.x}px`;
    floating.style.top = `${floatingPosition.value.y}px`;

    console.log("floating", { floating, floatingRect, reference, referenceRect, containerRect, newFloat })
  };

  // recompute if the refs change (ignore element positions/size changes by default)
  watch(
    () => [float.floating.value, float.reference.value, optionsRef.value, float.enabled?.value],
    () => {
      if (float.enabled != null && !float.enabled?.value) return;
      const floating = unrefElement(float.floating);
      const reference = unrefElement(float.reference);
      if (floating == null || reference == null) return;

      recompute();
    },
    { immediate: true },
  );

  // recompute if the sizes/positions/scrolls change (if requested)
  if (float.watchElements) {
    watchElementBounding(float.floating, recompute, { windowResize: true, windowScroll: true });
    watchElementBounding(float.reference, recompute, { windowResize: true, windowScroll: true });
    watchElementBounding(foundContainer, recompute, { windowResize: true, windowScroll: true });
  }

  return { recompute, floatingPosition, placement };
}
