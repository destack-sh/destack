import {
  Align,
  Axis2,
  Axis3,
  Corners,
  Direction,
  Distribute,
  Grid,
  GridSpan,
  Insets,
  IsExtensible,
  Layout,
  Vector2,
} from "@destack/language/core";
import { TraitType } from "@destack/language/core/builtin";
import { Border, Fill, Shadow } from "@destack/language/style";
import { View } from "@destack/language/view";

/* ==== DESTACK_GENERATED_START:TRAIT:10000 ==== */
/**
 * A container View contains other Views.
 */
export interface ContainerView extends IsExtensible, View {
  /**
   * ContainerView.layout
   */
  layout: Layout | null;

  /**
   * ContainerView.direction
   */
  direction: Direction | null;

  /**
   * ContainerView.distribute
   */
  distribute: Distribute | null;

  /**
   * ContainerView.align
   */
  align: Align | null;

  /**
   * ContainerView.gap
   */
  gap: Axis2 | null;

  /**
   * ContainerView.padding
   */
  padding: Insets | null;

  /**
   * ContainerView.grid
   */
  grid: Grid | null;

  /**
   * ContainerView.gridSpan
   */
  gridSpan: GridSpan | null;

  /**
   * ContainerView.aspectRatio
   */
  aspectRatio: number | null;

  /**
   * ContainerView.isWrap
   */
  isWrap: boolean | null;

  /**
   * ContainerView.isVisible
   */
  isVisible: boolean | null;

  /**
   * ContainerView.opacity
   */
  opacity: number | null;

  /**
   * ContainerView.fill
   */
  fill: Fill | null;

  /**
   * ContainerView.rotation
   */
  rotation: Axis3 | null;

  /**
   * ContainerView.skew
   */
  skew: Vector2 | null;

  /**
   * ContainerView.scale
   */
  scale: number | null;

  /**
   * ContainerView.shadow
   */
  shadow: Shadow | null;

  /**
   * ContainerView.border
   */
  border: Border | null;

  /**
   * ContainerView.radius
   */
  radius: Corners | null;

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}

class ContainerView$Type extends TraitFacade {}
export const ContainerView = new ContainerView$Type(TraitType.CONTAINER_VIEW);
/* ==== DESTACK_GENERATED_END:TRAIT:10000 ==== */
