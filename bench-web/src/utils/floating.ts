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
  margin?: number; // relative to the container and reference
  arrow?: boolean; // whether to include an arrow pointing to the reference
  arrowMargin?: number; // relative to the floating element
};

/**
 * Gets the position of the floating element relative to the reference while staying in the container.
 * The arrow always points to the relative center of the reference (up to the floating size - margin).
 */
export function getFloatingPosition(
  floating: { width: number; height: number },
  reference: { x: number; y: number; width: number; height: number },
  container: { x: number; y: number; width: number; height: number },
  options: FloatingOptions,
  arrow?: { width: number; height: number },
): { x: number; y: number; placement: FloatingPlacement; arrow?: { x: number; y: number } } {
  let x = 0;
  let y = 0;
  const margin = options.margin ?? 0;
  let placement = options.placement;

  const recomputePosition = () => {
    switch (placement) {
      case "top":
        x = reference.x + reference.width / 2 - floating.width / 2;
        y = reference.y - floating.height - margin;
        break;
      case "top-left":
        x = reference.x;
        y = reference.y - floating.height - margin;
        break;
      case "top-right":
        x = reference.x + reference.width - floating.width;
        y = reference.y - floating.height - margin;
        break;
      case "right":
        x = reference.x + reference.width + margin;
        y = reference.y + reference.height / 2 - floating.height / 2;
        break;
      case "right-top":
        x = reference.x + reference.width + margin;
        y = reference.y;
        break;
      case "right-bottom":
        x = reference.x + reference.width + margin;
        y = reference.y + reference.height - floating.height;
        break;
      case "bottom":
        x = reference.x + reference.width / 2 - floating.width / 2;
        y = reference.y + reference.height + margin;
        break;
      case "bottom-left":
        x = reference.x;
        y = reference.y + reference.height + margin;
        break;
      case "bottom-right":
        x = reference.x + reference.width - floating.width;
        y = reference.y + reference.height + margin;
        break;
      case "left":
        x = reference.x - floating.width - margin;
        y = reference.y + reference.height / 2 - floating.height / 2;
        break;
      case "left-top":
        x = reference.x - floating.width - margin;
        y = reference.y;
        break;
      case "left-bottom":
        x = reference.x - floating.width - margin;
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
  x = Math.max(container.x + margin, Math.min(container.x + container.width - floating.width - margin, x));
  y = Math.max(container.y + margin, Math.min(container.y + container.height - floating.height - margin, y));

  // calculate arrow position
  // (relative to new placement's center of reference, up to the floating bounds - margin)
  let arrowPosition: { x: number; y: number } | undefined;
  if (options.arrow) {
    if (arrow == null) throw new Error("arrow is required for arrow placement");
    // get target reference center
    let referenceCenter: { x: number; y: number };
    if (placement.startsWith("top")) {
      referenceCenter = { x: reference.x + reference.width / 2, y: reference.y };
    } else if (placement.startsWith("right")) {
      referenceCenter = { x: reference.x + reference.width, y: reference.y + reference.height / 2 };
    } else if (placement.startsWith("bottom")) {
      referenceCenter = { x: reference.x + reference.width / 2, y: reference.y + reference.height };
    } /* left */ else {
      referenceCenter = { x: reference.x, y: reference.y + reference.height / 2 };
    }
    arrowPosition = { x: referenceCenter.x, y: referenceCenter.y };

    // bound arrow to floating bounds
    const arrowMargin = options.arrowMargin ?? 0;
    arrowPosition.x = Math.max(x + arrowMargin, Math.min(x + floating.width - arrowMargin, arrowPosition.x));
    arrowPosition.y = Math.max(y + arrowMargin, Math.min(y + floating.height - arrowMargin, arrowPosition.y));

    // subtract arrow size
    arrowPosition.y -= arrow.height / 2;
    arrowPosition.x -= arrow.width / 2;
  }

  return { x, y, placement, arrow: arrowPosition };
}

/**
 * Positions the floating element to 'float' as requested relative to the reference, while staying in the container.
 * We automatically look for a container element with 'data-root-element', else use the viewport.
 *  */
export function useFloating(float: {
  floating: Ref<MaybeElement>;
  reference: Ref<MaybeElement>;
  arrow?: Ref<MaybeElement>;
  options?: MaybeRef<Partial<FloatingOptions>>;
  enabled?: Ref<boolean>;
  watchElements?: boolean;
}): {
  recompute: () => void;
  floatingPosition: Ref<{ x: number; y: number }>;
  arrowPosition: Ref<{ x: number; y: number } | null>;
} {
  const optionsRef = toRef(float.options ?? shallowRef({})) as Ref<Partial<FloatingOptions>>;
  const floatingPosition = shallowRef({ x: 0, y: 0 });
  const arrowPosition = shallowRef({ x: 0, y: 0 } as { x: number; y: number } | null);
  const foundContainer = shallowRef<HTMLElement | null | undefined>(undefined);

  // finds the closest annotated root container
  const findContainer = (el: HTMLElement | SVGElement | null) => {
    while (el != null) {
      if (el.hasAttribute("data-root-element")) return el as HTMLElement;
      el = el.parentElement;
    }
    return null;
  };

  // recomputes & applies the floating (and arrow) position
  const recompute = () => {
    // get elements bounding
    const floating = getElement(float.floating.value);
    const reference = getElement(float.reference.value);
    const arrow = float.arrow?.value != null ? getElement(float.arrow.value) : null;
    const floatingRect = floating.getBoundingClientRect();
    const referenceRect = reference.getBoundingClientRect();
    const arrowRect = arrow?.getBoundingClientRect();
    if (foundContainer.value === undefined) {
      foundContainer.value = findContainer(floating);
    }
    const containerRect =
      foundContainer.value?.getBoundingClientRect() ?? document.documentElement.getBoundingClientRect();

    // recompute positions in fixed coordinate space
    const options = { placement: "top", margin: 0, arrow: false, ...optionsRef.value } as FloatingOptions;
    if (float.arrow != null) options.arrow = true;
    const {
      x,
      y,
      arrow: newArrowPosition,
    } = getFloatingPosition(
      { width: floatingRect.width, height: floatingRect.height },
      { x: referenceRect.x, y: referenceRect.y, width: referenceRect.width, height: referenceRect.height },
      { x: containerRect.x, y: containerRect.y, width: containerRect.width, height: containerRect.height },
      options,
      arrowRect,
    );
    floatingPosition.value = { x, y };
    arrowPosition.value = newArrowPosition ?? null;

    // apply positions (fixed)
    // TODO :Robustness: :UI: use absolute positioning for floating elements
    floating.style.position = "fixed";
    floating.style.left = `${x}px`;
    floating.style.top = `${y}px`;
    if (arrow != null && arrowPosition.value != null) {
      const arrowElement = unrefElement(float.arrow);
      if (arrowElement != null) {
        arrowElement.style.position = "fixed";
        arrowElement.style.left = `${arrowPosition.value.x}px`;
        arrowElement.style.top = `${arrowPosition.value.y}px`;
      }
    }
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

  return { recompute, floatingPosition, arrowPosition };
}
