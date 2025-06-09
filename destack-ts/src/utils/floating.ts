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
  | "left-bottom"
  | "inside-top"
  | "inside-top-left"
  | "inside-top-right";

export type FloatingOptions = {
  /* Place floating relative to the reference */
  placement: FloatingPlacement;
  /* Margin around the container on all axes */
  containerMargin?: number;
  /* Margin around the reference along the main axis */
  referenceMargin?: number;
  /* Offset the reference position */
  offset?: { x: number; y: number } | "referenceWidth" | "-referenceWidth" | "referenceHeight" | "-referenceHeight";
};

/**
 * Gets the position of the floating element relative to the reference while staying in the container.
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

  // convenience offset the reference via options
  if (options.offset == "referenceWidth") {
    reference.x += reference.width;
  } else if (options.offset == "-referenceWidth") {
    reference.x -= reference.width;
  } else if (options.offset == "referenceHeight") {
    reference.y += reference.height;
  } else if (options.offset == "-referenceHeight") {
    reference.y -= reference.height;
  } else if (options.offset != null) {
    reference.x += options.offset.x;
    reference.y += options.offset.y;
  }

  const computePosition = () => {
    switch (placement) {
      case "top":
        x = reference.x + reference.width / 2 - floating.width / 2;
        y = reference.y - floating.height - referenceMargin;
        break;
      case "top-left":
        x = reference.x - reference.width;
        y = reference.y - floating.height - referenceMargin;
        break;
      case "top-right":
        x = reference.x + reference.width;
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
        x = reference.x - reference.width;
        y = reference.y + reference.height + referenceMargin;
        break;
      case "bottom-right":
        x = reference.x + reference.width;
        y = reference.y + reference.height + referenceMargin;
        break;
      case "left":
        x = reference.x - floating.width - referenceMargin;
        y = reference.y + reference.height / 2 - floating.height / 2;
        break;
      case "left-top":
        x = reference.x - floating.width - referenceMargin;
        y = reference.y + referenceMargin;
        break;
      case "left-bottom":
        x = reference.x - floating.width - referenceMargin;
        y = reference.y + reference.height - floating.height;
        break;
      case "inside-top":
        x = reference.x + reference.width / 2 - floating.width / 2;
        y = reference.y + referenceMargin;
        break;
      case "inside-top-left":
        x = reference.x + referenceMargin;
        y = reference.y + referenceMargin;
        break;
      case "inside-top-right":
        x = reference.x + reference.width - floating.width - referenceMargin;
        y = reference.y + referenceMargin;
        break;
    }
  };

  // compute initial position
  computePosition();

  // flip and recompute if needed
  if (y < container.y && placement.startsWith("top")) {
    placement = placement.replace("top", "bottom") as FloatingPlacement;
  } else if (y + floating.height > container.y + container.height && placement.startsWith("bottom")) {
    placement = placement.replace("bottom", "top") as FloatingPlacement;
  } else if (x + floating.width > container.x + container.width && placement.startsWith("right")) {
    placement = placement.replace("right", "left") as FloatingPlacement;
  } else if (x < container.x && placement.startsWith("left")) {
    placement = placement.replace("left", "right") as FloatingPlacement;
  }
  if (options.placement != placement) {
    computePosition();
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
  isEnabled?: Ref<boolean>;
  watchElements?: boolean;
  keepPlacement?: boolean;
}): {
  recompute: () => void;
  floatingPosition: Ref<{ x: number; y: number }>;
  placement: Ref<FloatingPlacement | null>;
} {
  const optionsRef = toRef(float.options ?? shallowRef({})) as Ref<Partial<FloatingOptions>>;
  const floatingPosition = shallowRef({ x: 0, y: 0 });
  const placement: Ref<FloatingPlacement | null> = shallowRef(null);
  const foundContainer = shallowRef<HTMLElement | SVGElement | null | undefined>(unrefElement(float.container));
  const lockedPlacement = shallowRef<FloatingPlacement | null>(null);

  // recomputes & applies the floating position
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
    const options = { 
      placement: lockedPlacement.value ?? "top", 
      ...optionsRef.value,
      // Override placement with locked value if keepPlacement is true
      ...(float.keepPlacement && lockedPlacement.value ? { placement: lockedPlacement.value } : {})
    } as FloatingOptions;
    
    const newFloat = getFloatingPosition({
      floating: { width: floatingRect.width, height: floatingRect.height },
      reference: referenceRect,
      container: containerRect,
      options,
    });
    floatingPosition.value = { x: newFloat.x, y: newFloat.y };
    placement.value = newFloat.placement;

    // Store first successful placement if keepPlacement is true
    if (float.keepPlacement && lockedPlacement.value === null) {
      lockedPlacement.value = newFloat.placement;
    }

    // apply positions
    floating.style.position = "fixed";
    floating.style.left = `${floatingPosition.value.x}px`;
    floating.style.top = `${floatingPosition.value.y}px`;
  };

  // recompute if the refs change (ignore element positions/size changes by default)
  watch(
    () =>
      // only get properties if enabled (enabled may guard some potentially expensive or unset properties)
      float.isEnabled == null || float.isEnabled?.value
        ? [float.floating.value, float.reference.value, optionsRef.value]
        : [],
    () => {
      if (float.isEnabled != null && !float.isEnabled?.value) return;
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
