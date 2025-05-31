import type { Transaction } from "@/language/core/transaction";
import { ObjectType, Orientation, RectangleData, type ViewData } from "@/proto/wire";
import { activeSelection } from "@/ui/drag";
import { roundToDigits } from "@/utils/functools";
import {
  useElementSize,
  useEventListener,
  useMouseInElement,
  useMousePressed,
  useScroll,
  whenever,
} from "@vueuse/core";
import { computed, ref, toRef, watch, type MaybeRef, type Ref } from "vue";

// our own 'dragging' state so we can block pointer events at the root component
export const IS_DRAGGING = ref(false);
export const IS_DRAGGING_OR_SELECTING = computed(() => IS_DRAGGING.value || activeSelection.value != null);

export const MIN_SPLIT_SIZE = 250;
export const DEFAULT_ORIENTATION = Orientation.HORIZONTAL;
export const DEFAULT_RELATIVE_UNITS = 1000;
export const MIN_THUMB_SIZE = 20;

export type SizedView = {
  view: ViewData;
  left: number;
  top: number;
  width: number;
  height: number;
};

export type SplitLayout = {
  orientation: Orientation;
  minPx: number;
  dividerSize: number; // should this even affect the layout?
};

/**
 * Calculates positions and sizes for views in a split view container.
 * Sizes are distributed alongside the given orientation (the relative views subdividing the space not taken up by absolute views).
 * All views just get the container size for the other dimension.
 */
export function useSplit(
  viewsRef: Ref<ViewData[]>,
  containerRef: Ref<{ width: number; height: number }>,
  layoutRef: Ref<SplitLayout>,
): {
  sizedViews: Ref<SizedView[]>;
  updateSeparator: (sepIdx: number, toPx: number) => [Partial<ViewData>, Partial<ViewData>];
} {
  const getAbsolutePx = (view: ViewData) => {
    if (layoutRef.value.orientation == Orientation.HORIZONTAL) {
      return view.size?.widthRelative != null ? null : view.size?.width;
    } else {
      return view.size?.heightRelative != null ? null : view.size?.height;
    }
  };
  const getRelativeUnits = (view: ViewData) => {
    return layoutRef.value.orientation == Orientation.HORIZONTAL ? view.size?.widthRelative : view.size?.heightRelative;
  };
  const getMinSize = (view: ViewData) => {
    if (layoutRef.value.orientation == Orientation.HORIZONTAL) {
      return view.constraint?.minWidth ?? MIN_SPLIT_SIZE;
    } else {
      return view.constraint?.minHeight ?? MIN_SPLIT_SIZE;
    }
  };
  const getMaxSize = (view: ViewData) => {
    if (layoutRef.value.orientation == Orientation.HORIZONTAL) {
      return view.constraint?.maxWidth ?? Number.MAX_SAFE_INTEGER;
    } else {
      return view.constraint?.maxHeight ?? Number.MAX_SAFE_INTEGER;
    }
  };

  /* Figure out assigned space */
  function getTotals() {
    const px =
      layoutRef.value.orientation === Orientation.HORIZONTAL ? containerRef.value.width : containerRef.value.height;
    const absolutePx = viewsRef.value.reduce((acc, view) => (getAbsolutePx(view) ?? 0) + acc, 0);
    const relativePx = px - absolutePx;
    const relativeUnits = viewsRef.value
      .filter((view) => getAbsolutePx(view) == null)
      .reduce((acc, view) => (getRelativeUnits(view) ?? DEFAULT_RELATIVE_UNITS) + acc, 0);
    return { px, absolutePx, relativePx, relativeUnits };
  }

  const sizedViews = computed(() => {
    const container = containerRef.value;
    const isHorizontal = layoutRef.value.orientation === Orientation.HORIZONTAL;
    const total = getTotals();

    // distribute the relative space
    const perViewPx: number[] = [];
    for (const view of viewsRef.value) {
      const absolutePx = getAbsolutePx(view) ?? 0;
      if (absolutePx > 0) {
        perViewPx.push(absolutePx);
      } else {
        const relativeUnits = getRelativeUnits(view) ?? DEFAULT_RELATIVE_UNITS;
        const relativePx = roundToDigits((relativeUnits / total.relativeUnits) * total.relativePx, 0);
        perViewPx.push(relativePx);
      }
    }

    // layout in order
    let offsetPx = 0;
    const sizedViews: SizedView[] = [];
    for (let i = 0; i < viewsRef.value.length; i++) {
      const view = viewsRef.value[i];
      const width = isHorizontal ? perViewPx[i] : container.width;
      const height = isHorizontal ? container.height : perViewPx[i];
      sizedViews.push({
        view,
        left: isHorizontal ? offsetPx : 0,
        top: isHorizontal ? 0 : offsetPx,
        width,
        height,
      });
      offsetPx += perViewPx[i];
    }

    return sizedViews;
  });

  /**
   * Redistribute the relative space between two views to where the separator is dragged.
   * Separator index is the index of the first view (left of / above the separator).
   */
  function updateSeparator(sepIdx: number, sepAtPx: number): [Partial<ViewData>, Partial<ViewData>] {
    const isHorizontal = layoutRef.value.orientation === Orientation.HORIZONTAL;

    // figure out new target sizes
    const viewA = sizedViews.value[sepIdx];
    const viewB = sizedViews.value[sepIdx + 1];
    let viewATargetPx = sepAtPx - (isHorizontal ? viewA.left : viewA.top);
    let viewBTargetPx = isHorizontal ? viewB.left + viewB.width - sepAtPx : viewB.top + viewB.height - sepAtPx;

    // Get constraints for both views
    const viewAMin = getMinSize(viewA.view);
    const viewBMin = getMinSize(viewB.view);
    const viewAMax = getMaxSize(viewA.view);
    const viewBMax = getMaxSize(viewB.view);

    // Calculate total available space
    const totalSpace = viewATargetPx + viewBTargetPx;

    // check min constraints (these take priority)
    if (viewATargetPx < viewAMin) {
      viewATargetPx = viewAMin;
      viewBTargetPx = totalSpace - viewATargetPx;
    }
    if (viewBTargetPx < viewBMin) {
      viewBTargetPx = viewBMin;
      viewATargetPx = totalSpace - viewBTargetPx;
    }

    // check max constraints
    if (viewATargetPx > viewAMax) {
      viewATargetPx = viewAMax;
      viewBTargetPx = totalSpace - viewATargetPx;
    }
    if (viewBTargetPx > viewBMax) {
      viewBTargetPx = viewBMax;
      viewATargetPx = totalSpace - viewBTargetPx;
    }

    // validate min constraints
    if (viewATargetPx < viewAMin) {
      viewATargetPx = viewAMin;
      viewBTargetPx = totalSpace - viewATargetPx;
    }
    if (viewBTargetPx < viewBMin) {
      viewBTargetPx = viewBMin;
      viewATargetPx = totalSpace - viewBTargetPx;
    }

    // create a partial view update to set the view to a specific size in pixels (relative or absolute)
    const updateViewPx = (view: ViewData, targetPx: number): Partial<ViewData> => {
      const isAbsolute = getAbsolutePx(view) != null;
      const update = { id: view.id };
      if (isAbsolute) {
        const newSize = isHorizontal ? { width: targetPx } : { height: targetPx };
        return { ...update, size: { metatype: ObjectType.RECTANGLE, ...newSize } };
      } else {
        const total = getTotals();
        const targetRelativeUnits = roundToDigits((targetPx / total.relativePx) * total.relativeUnits, 3);
        const newSize = isHorizontal ? { widthRelative: targetRelativeUnits } : { heightRelative: targetRelativeUnits };
        return { ...update, size: { metatype: ObjectType.RECTANGLE, ...newSize } };
      }
    };

    return [updateViewPx(viewA.view, viewATargetPx), updateViewPx(viewB.view, viewBTargetPx)];
  }

  return { sizedViews, updateSeparator };
}

export const mousePressed = useMousePressed();
export const mouseNotPressed = computed(() => !mousePressed.pressed.value);

export function onMouseReleasedOnce(callback: () => void) {
  whenever(mouseNotPressed, callback, { once: true });
}

/**
 * Split a view in a container with draggable separators.
 * Set 'draggingIdx' to track which separator is being dragged (=index of lower view).
 * 'draggingIdx' is automatically reset when the mouse is released.
 */
export function useSplitView(
  viewsRef: Ref<ViewData[]>,
  sizeRef: Ref<{ width: number; height: number }>,
  containerRef: Ref<HTMLElement | null>,
  layoutRef: Ref<SplitLayout>,
  txFactory: () => Transaction,
) {
  const { sizedViews, updateSeparator } = useSplit(viewsRef, sizeRef, layoutRef);
  const { elementX: mouseRelativeX, elementY: mouseRelativeY } = useMouseInElement(containerRef);
  const draggingIdx = ref<number | null>(null);

  // track dragging state
  // NOTE: draggingIdx may be set before 'mousePressed' is true, so we wait for both.
  whenever(
    // wait for mouse press once to start dragging
    computed(() => draggingIdx.value != null && mousePressed.pressed.value),
    () => {
      IS_DRAGGING.value = true;

      // apply dragging
      const stop = watch([mouseRelativeX, mouseRelativeY], () => {
        if (draggingIdx.value == null) return;
        const draggedToPx =
          layoutRef.value.orientation == Orientation.HORIZONTAL ? mouseRelativeX.value : mouseRelativeY.value;
        const [aUpdate, bUpdate] = updateSeparator(draggingIdx.value, draggedToPx);
        const tx = txFactory();
        tx.update(viewsRef.value[draggingIdx.value], { size: aUpdate.size }, { debounce: "long" });
        tx.update(viewsRef.value[draggingIdx.value + 1], { size: bUpdate.size }, { debounce: "long" });
      });

      // and stop dragging once mouse is released
      whenever(
        computed(() => !mousePressed.pressed.value),
        () => {
          draggingIdx.value = null;
          IS_DRAGGING.value = false;
          stop();
        },
        { once: true },
      );
    },
  );

  return { sizedViews, draggingIdx };
}

/**
 * Gets the exact half sized box for a view (for both dimensions, for absolute and relative)
 */
export function splitBox(size?: RectangleData): RectangleData {
  size = size ?? {
    metatype: ObjectType.RECTANGLE,
    widthRelative: DEFAULT_RELATIVE_UNITS,
    heightRelative: DEFAULT_RELATIVE_UNITS,
  };
  return {
    metatype: ObjectType.RECTANGLE,
    width: size.width != null ? size.width / 2 : undefined,
    widthRelative: size.width != null ? undefined : (size.widthRelative ?? DEFAULT_RELATIVE_UNITS) / 2,
    height: size.height != null ? size.height / 2 : undefined,
    heightRelative: size.height != null ? undefined : (size.heightRelative ?? DEFAULT_RELATIVE_UNITS) / 2,
  };
}

type Rect = { left: number; top: number; width: number; height: number };

export enum ScrollbarWidth {
  sm = 4,
  md = 8,
  lg = 12,
}

/**
 * Defines a scrollable area with a settable thumb position (and dynamic size.)
 */
export function useScrollArea(area: {
  container: Ref<HTMLElement | null>;
  inner: Ref<HTMLElement | null>;
  orientation: MaybeRef<Orientation | undefined>;
  trackWidth: MaybeRef<ScrollbarWidth>;
}): {
  thumb: Ref<Rect>;
  setThumb: (newThumb: { left: number; top: number }) => void;
  moveThumb: (movement: { x: number; y: number }) => void;
  isThumbScrolling: Ref<boolean>;
  isNativeScrolling: Ref<boolean>;
  isOverflown: Ref<boolean>;
  isAtStart: Ref<boolean>;
  isCloseToStart: Ref<boolean>;
  isAtEnd: Ref<boolean>;
  isCloseToEnd: Ref<boolean>;
  scroll: ReturnType<typeof useScroll>;
  containerSize: ReturnType<typeof useElementSize>;
  innerSize: ReturnType<typeof useElementSize>;
} {
  const CLOSE_THRESHOLD = 0.25; // % of container size
  const orientationRef = toRef(area.orientation) as Ref<Orientation>;
  const trackWidthRef = toRef(area.trackWidth) as Ref<ScrollbarWidth>;
  const scroll = useScroll(area.container);
  const containerSize = useElementSize(area.container, undefined, { box: "border-box" });
  const innerSize = useElementSize(area.inner, undefined, { box: "border-box" });

  //
  // track scrolling state
  //

  const isOverflown = computed(() => {
    if (area.container.value == null) return false;
    const isHorizontal = (orientationRef.value ?? DEFAULT_ORIENTATION) === Orientation.HORIZONTAL;
    if (isHorizontal) {
      return area.container.value.scrollWidth > containerSize.width.value;
    } else {
      return area.container.value.scrollHeight > containerSize.height.value;
    }
  });

  const thumb: Ref<Rect> = computed(() => {
    {
      if (area.container.value == null) return { left: 0, top: 0, width: 0, height: 0 };
      const isHorizontal = (orientationRef.value ?? DEFAULT_ORIENTATION) === Orientation.HORIZONTAL;
      const scrollSize = isHorizontal ? area.container.value.scrollWidth : area.container.value.scrollHeight;
      const clientSize = isHorizontal ? containerSize.width.value : containerSize.height.value;
      const scrollPos = isHorizontal ? scroll.x.value : scroll.y.value;
      const thumbSize = Math.max((clientSize / scrollSize) * clientSize, MIN_THUMB_SIZE);
      const thumbPos = (scrollPos / scrollSize) * clientSize;
      // eslint-disable-next-line @typescript-eslint/no-unused-expressions
      innerSize.width.value + innerSize.height.value; // trigger reactivity

      if (isHorizontal) {
        return { left: thumbPos, top: 0, width: thumbSize, height: trackWidthRef.value };
      } else {
        return { left: 0, top: thumbPos, width: trackWidthRef.value, height: thumbSize };
      }
    }
  });

  //
  // manual scrolling
  //

  const isThumbScrolling = ref(false);
  const { pressed } = useMousePressed();
  useEventListener(["mousemove"], (e) => {
    if (isThumbScrolling.value) {
      moveThumb({ x: e.movementX, y: e.movementY });
    }
  });

  function setThumb(newThumb: { left: number; top: number }) {
    if (area.container.value == null) return;
    const isHorizontal = (orientationRef.value ?? DEFAULT_ORIENTATION) === Orientation.HORIZONTAL;
    const clientSize = isHorizontal ? containerSize.width.value : containerSize.height.value;
    const scrollSize = isHorizontal ? area.container.value.scrollWidth : area.container.value.scrollHeight;

    // calculate new scroll position (capped to scroll bounds)
    let newScrollPos = ((isHorizontal ? newThumb.left : newThumb.top) / clientSize) * scrollSize;
    newScrollPos = Math.max(0, Math.min(scrollSize - clientSize, newScrollPos));
    if (isHorizontal) {
      scroll.x.value = newScrollPos;
    } else {
      scroll.y.value = newScrollPos;
    }
  }

  function moveThumb(movement: { x: number; y: number }) {
    setThumb({ left: thumb.value.left + movement.x, top: thumb.value.top + movement.y });
  }

  watch(pressed, () => {
    if (!pressed.value) {
      isThumbScrolling.value = false;
    }
  });
  watch(isThumbScrolling, () => (IS_DRAGGING.value = isThumbScrolling.value));

  function getStartPosition() {
    if (area.container.value == null) return null;
    const isHorizontal = (orientationRef.value ?? DEFAULT_ORIENTATION) === Orientation.HORIZONTAL;
    const scrollPos = isHorizontal ? scroll.x.value : scroll.y.value;
    const clientSize = isHorizontal ? containerSize.width.value : containerSize.height.value;
    return { scrollPos, clientSize };
  }

  function getEndPosition() {
    if (area.container.value == null) return null;
    const isHorizontal = (orientationRef.value ?? DEFAULT_ORIENTATION) === Orientation.HORIZONTAL;
    const scrollSize = isHorizontal ? area.container.value.scrollWidth : area.container.value.scrollHeight;
    const clientSize = isHorizontal ? containerSize.width.value : containerSize.height.value;
    const scrollPos = isHorizontal ? scroll.x.value : scroll.y.value;
    return { scrollPos, clientSize, scrollSize };
  }

  const isAtStart = computed(() => {
    const pos = getStartPosition();
    if (!pos) return false;
    return pos.scrollPos <= 0;
  });

  const isCloseToStart = computed(() => {
    const pos = getStartPosition();
    if (!pos) return false;
    return pos.scrollPos <= pos.clientSize * CLOSE_THRESHOLD;
  });

  const isAtEnd = computed(() => {
    const pos = getEndPosition();
    if (!pos) return false;
    return pos.scrollPos + pos.clientSize >= pos.scrollSize - 1;
  });

  const isCloseToEnd = computed(() => {
    const pos = getEndPosition();
    if (!pos) return false;
    const remainingScroll = pos.scrollSize - (pos.scrollPos + pos.clientSize);
    return remainingScroll <= pos.clientSize * CLOSE_THRESHOLD;
  });

  return {
    thumb,
    setThumb,
    moveThumb,
    isThumbScrolling,
    isNativeScrolling: scroll.isScrolling,
    isOverflown,
    isAtStart,
    isCloseToStart,
    isAtEnd,
    isCloseToEnd,
    scroll,
    containerSize,
    innerSize,
  };
}
