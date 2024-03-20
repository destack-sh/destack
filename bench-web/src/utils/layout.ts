import { BenchType, BoxData, NodeType, Orientation, type ViewData } from "@/proto/wire";
import type { GraphConnection } from "@/system/connection";
import { roundToDigits } from "@/utils/functools";
import { useElementSize, useEventListener, useMouseInElement, useMousePressed, useScroll } from "@vueuse/core";
import { computed, ref, watch, type Ref, type MaybeRef, toRef } from "vue";

// our own 'dragging' state so we can block pointer events at the root component
const _isDragging = ref(false);
export const isDragging = computed(() => _isDragging.value);

export const MIN_WINDOW_SIZE = 200;
export const DEFAULT_ORIENTATION = Orientation.HORIZONTAL;
export const DEFAULT_RELATIVE_UNITS = 1;

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
export function splitView(
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

  function getTotals() {
    // figure out assigned space
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
    const a = sizedViews.value[sepIdx];
    const b = sizedViews.value[sepIdx + 1];
    let aTargetPx = sepAtPx - (isHorizontal ? a.left : a.top);
    let bTargetPx = isHorizontal ? b.left + b.width - sepAtPx : b.top + b.height - sepAtPx;
    // clamp target sizes
    if (aTargetPx < MIN_WINDOW_SIZE) {
      bTargetPx += aTargetPx - MIN_WINDOW_SIZE;
      aTargetPx = MIN_WINDOW_SIZE;
    } else if (bTargetPx < MIN_WINDOW_SIZE) {
      aTargetPx += bTargetPx - MIN_WINDOW_SIZE;
      bTargetPx = MIN_WINDOW_SIZE;
    }

    // create a partial view update to set the view to a specific size in pixels (relative or absolute)
    const updateViewPx = (view: ViewData, targetPx: number): Partial<ViewData> => {
      const isAbsolute = getAbsolutePx(view) != null;
      const update = { id: view.id, ck: view.ck };
      if (isAbsolute) {
        const newSize = isHorizontal ? { width: targetPx } : { height: targetPx };
        return { ...update, size: { metatype: BenchType.BOX, ...newSize } };
      } else {
        const total = getTotals();
        const targetRelativeUnits = roundToDigits((targetPx / total.relativePx) * total.relativeUnits, 3);
        const newSize = isHorizontal ? { widthRelative: targetRelativeUnits } : { heightRelative: targetRelativeUnits };
        return { ...update, size: { metatype: BenchType.BOX, ...newSize } };
      }
    };

    return [updateViewPx(a.view, aTargetPx), updateViewPx(b.view, bTargetPx)];
  }

  return { sizedViews, updateSeparator };
}

/**
 * Split a view in a container with draggable separators.
 * Set 'draggingIdx' to track which separator is being dragged (=index of lower view).
 */
export function useSplitView(
  viewsRef: Ref<ViewData[]>,
  sizeRef: Ref<{ width: number; height: number }>,
  containerRef: Ref<HTMLElement | null>,
  layoutRef: Ref<SplitLayout>,
  graphConnection: GraphConnection,
) {
  const { sizedViews, updateSeparator } = splitView(viewsRef, sizeRef, layoutRef);
  const { pressed } = useMousePressed();
  const { elementX: mouseRelativeX, elementY: mouseRelativeY } = useMouseInElement(containerRef);
  const draggingIdx = ref<number | null>(null);

  watch([pressed, mouseRelativeX, mouseRelativeY], () => {
    if (draggingIdx.value == null) return;
    if (!pressed.value) {
      draggingIdx.value = null;
      return;
    }
    const draggedToPx =
      layoutRef.value.orientation == Orientation.HORIZONTAL ? mouseRelativeX.value : mouseRelativeY.value;
    const [aUpdate, bUpdate] = updateSeparator(draggingIdx.value, draggedToPx);
    graphConnection.sideTx.update({
      metatype: NodeType.VIEW,
      id: viewsRef.value[draggingIdx.value].id,
      size: aUpdate.size,
      debounce: true,
    });
    graphConnection.sideTx.update({
      metatype: NodeType.VIEW,
      id: viewsRef.value[draggingIdx.value + 1].id,
      size: bUpdate.size,
      debounce: true,
    });
  });

  watch(draggingIdx, () => (_isDragging.value = draggingIdx.value != null));

  return { sizedViews, draggingIdx };
}

/**
 * Gets the exact half sized box for a view (for both dimensions, for absolute and relative)
 */
export function splitBox(size?: BoxData): BoxData {
  size = size ?? {
    metatype: BenchType.BOX,
    widthRelative: DEFAULT_RELATIVE_UNITS,
    heightRelative: DEFAULT_RELATIVE_UNITS,
  };
  return {
    metatype: BenchType.BOX,
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
  orientation: MaybeRef<Orientation | undefined>;
  trackWidth: MaybeRef<ScrollbarWidth>;
}): {
  thumb: Ref<Rect>;
  setThumb: (newThumb: { left: number; top: number }) => void;
  moveThumb: (movement: { x: number; y: number }) => void;
  isManualScrolling: Ref<boolean>;
  isNativeScrolling: Ref<boolean>;
  isOverflown: Ref<boolean>;
} {
  const orientationRef = toRef(area.orientation) as Ref<Orientation>;
  const trackWidthRef = toRef(area.trackWidth) as Ref<ScrollbarWidth>;
  const scroll = useScroll(area.container);
  const containerSize = useElementSize(area.container);

  //
  // track scrolling state
  //

  const isOverflown = computed(() => {
    if (area.container.value == null) return false;
    const isHorizontal = (orientationRef.value ?? DEFAULT_ORIENTATION) === Orientation.HORIZONTAL;
    const clientSize = isHorizontal ? containerSize.width.value : containerSize.height.value;
    const scrollSize = isHorizontal ? area.container.value.scrollWidth : area.container.value.scrollHeight;
    return scrollSize > clientSize;
  });

  const thumb: Ref<Rect> = computed(() => {
    {
      if (area.container.value == null) return { left: 0, top: 0, width: 0, height: 0 };
      const isHorizontal = (orientationRef.value ?? DEFAULT_ORIENTATION) === Orientation.HORIZONTAL;
      const scrollSize = isHorizontal ? area.container.value.scrollWidth : area.container.value.scrollHeight;
      const clientSize = isHorizontal ? containerSize.width.value : containerSize.height.value;
      const scrollPos = isHorizontal ? scroll.x.value : scroll.y.value;
      const thumbSize = Math.max((clientSize / scrollSize) * clientSize, 20); // Ensure thumb has a minimum size for usability
      const thumbPos = (scrollPos / scrollSize) * clientSize;

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

  const isManualScrolling = ref(false);
  const { pressed } = useMousePressed();
  useEventListener(["mousemove"], (e) => {
    if (isManualScrolling.value) {
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
      isManualScrolling.value = false;
    }
  });
  watch(isManualScrolling, () => (_isDragging.value = isManualScrolling.value));

  return { thumb, setThumb, moveThumb, isManualScrolling, isNativeScrolling: scroll.isScrolling, isOverflown };
}
