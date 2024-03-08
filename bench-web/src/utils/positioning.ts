import { BenchType, Orientation, type ViewData } from "@/proto/wire";
import { roundToDigits } from "@/utils/functools";
import { computed, type Ref } from "vue";

export const MIN_WINDOW_SIZE = 200;
export const DEFAULT_ORIENTATION = Orientation.HORIZONTAL;

export type SizedView = {
  view: ViewData;
  left: number;
  top: number;
  width: number;
  height: number;
};

/**
 * Calculates positions and sizes for views in a split view container.
 * Sizes are distributed alongside the given orientation (the relative views subdividing the space not taken up by absolute views).
 * All views just get the container size for the other dimension.
 */
export function splitView(
  viewsRef: Ref<ViewData[]>,
  containerRef: Ref<{ width: number; height: number }>,
  layoutRef: Ref<{ orientation: Orientation; defaultRelativeUnits: number; minPx: number }>,
): {
  sizedViews: Ref<SizedView[]>;
  updateSeparator: (sepIdx: number, toPx: number) => [Partial<ViewData>, Partial<ViewData>];
} {
  const getAbsolutePx = (view: ViewData) => {
    return layoutRef.value.orientation == Orientation.HORIZONTAL ? view.size?.width : view.size?.height;
  };
  const getRelativeUnits = (view: ViewData) => {
    return layoutRef.value.orientation == Orientation.HORIZONTAL ? view.size?.widthRelative : view.size?.heightRelative;
  };

  function getTotals() {
    // figure out assigned space
    const totalPx =
      layoutRef.value.orientation === Orientation.HORIZONTAL ? containerRef.value.width : containerRef.value.height;
    const totalAbsolutePx = viewsRef.value.reduce((acc, view) => (getAbsolutePx(view) ?? 0) + acc, 0);
    const totalRelativePx = totalPx - totalAbsolutePx;
    const totalRelativeUnits = viewsRef.value
      .filter((view) => getAbsolutePx(view) == null)
      .reduce((acc, view) =>  (getRelativeUnits(view) ?? layoutRef.value.defaultRelativeUnits) + acc, 0);
    return { totalPx, totalAbsolutePx, totalRelativePx, totalRelativeUnits };
  }

  const sizedViews = computed(() => {
    const container = containerRef.value;
    const isHorizontal = layoutRef.value.orientation === Orientation.HORIZONTAL;
    const { totalRelativePx, totalRelativeUnits } = getTotals();

    // distribute the relative space
    const perViewPx: number[] = [];
    for (const view of viewsRef.value) {
      const absolutePx = getAbsolutePx(view) ?? 0;
      if (absolutePx > 0) {
        perViewPx.push(absolutePx);
      } else {
        const relativeUnits = getRelativeUnits(view) ?? layoutRef.value.defaultRelativeUnits;
        const relativePx = Math.round((relativeUnits / totalRelativeUnits) * totalRelativePx);
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
        const { totalRelativePx, totalRelativeUnits } = getTotals();
        const targetRelativeUnits = roundToDigits((targetPx / totalRelativePx) * totalRelativeUnits, 3);
        const newSize = isHorizontal ? { widthRelative: targetRelativeUnits } : { heightRelative: targetRelativeUnits };
        return { ...update, size: { metatype: BenchType.BOX, ...newSize } };
      }
    };

    return [updateViewPx(a.view, aTargetPx), updateViewPx(b.view, bTargetPx)];
  }

  return { sizedViews, updateSeparator };
}
