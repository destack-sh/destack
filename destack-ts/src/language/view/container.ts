import type {
  Align,
  Axis2,
  Axis3,
  Corners,
  Dimension,
  Direction,
  Distribute,
  Grid,
  GridSpan,
  Insets,
  IsActor,
  Layout,
  Materialization,
  NodeReference,
  Position,
  Snapshot,
  Value,
  Vector2f,
} from "@destack/language/core";
import { Entity, NodeType } from "@destack/language/core";
import type { Script } from "@destack/language/logic";
import { registerNodeClass } from "@destack/language/registry";
import type { Layer } from "@destack/language/scene";
import type { Border, Fill, Shadow } from "@destack/language/style";
import type { Space } from "@destack/language/universe";
import { View } from "@destack/language/view/view";
import { Temporal } from "temporal-polyfill";

/* ==== DESTACK_GENERATED_START:NODE:1800100 ==== */
/**
 * A container View contains other Views.
 * Containers can be laid out as stacks or grids.
 */
export abstract class ContainerView extends View {
  static metatype: NodeType = NodeType.CONTAINER_VIEW;

  /**
   * View.parent
   */
  abstract get parent(): Layer | ContainerView | null;
  declare readonly parentPtr: NodeReference | null;

  /**
   * The Space this Node is in.
   */
  abstract get space(): Space | null;
  declare readonly spacePtr: NodeReference;

  /**
   * The definition this CustomEntity is an instance of.
   */
  abstract get definition(): Entity | null;
  declare readonly definitionPtr: NodeReference | null;

  /**
   * Entity.materialization
   */
  declare readonly materialization: Materialization;

  /**
   * The Snapshot this Entity is part of.
   */
  abstract get snapshot(): Snapshot | null;
  declare readonly snapshotPtr: NodeReference | null;

  /**
   * The previous Entity this Entity is based on (from another Snapshot).
   */
  abstract get precededBy(): ContainerView | null;
  declare readonly precededByPtr: NodeReference | null;

  /**
   * The time this Entity was created.
   */
  declare readonly createdAt: Temporal.ZonedDateTime;

  /**
   * The Actor that created this Entity.
   */
  abstract get createdBy(): (Entity & IsActor) | null;
  declare readonly createdByPtr: NodeReference | null;

  /**
   * The time this Entity was last updated.
   */
  declare readonly updatedAt: Temporal.ZonedDateTime;

  /**
   * The Actor that last updated this Entity.
   */
  abstract get updatedBy(): (Entity & IsActor) | null;
  declare readonly updatedByPtr: NodeReference | null;

  /**
   * IsDeletable.deletedAt
   */
  declare readonly deletedAt: Temporal.ZonedDateTime | null;

  /**
   * The custom Values of this Node, keyed by custom Property id. May hold both static and instance values.
   */
  /**
   * The custom Values of this Node, keyed by custom Property id. May hold both static and instance values.
   */
  abstract get customValues(): { readonly [key: string]: Value };
  abstract set customValues(value: { readonly [key: string]: Value });

  /**
   * The absolute order key of this Node in its parent.
   */
  declare readonly orderKey: string;

  /**
   * Entity.name
   */
  /**
   * Entity.name
   */
  abstract get name(): string;
  abstract set name(value: string);

  /**
   * The Script that defines this Node.
   */
  abstract get source(): Script | null;
  declare readonly sourcePtr: NodeReference | null;

  /**
   * The key to uniquely identify this Node in reconciliation. If not set, name is used.
   */
  /**
   * The key to uniquely identify this Node in reconciliation. If not set, name is used.
   */
  abstract get key(): string | null;
  abstract set key(value: string | null);

  /**
   * The main / root Script of this Node.
   */
  abstract get script(): Script | null;
  abstract set script(value: Script | null);
  /**
   * The main / root Script of this Node.
   */
  abstract get scriptPtr(): NodeReference | null;
  abstract set scriptPtr(value: NodeReference | null);

  /**
   * Whether this Node is extensible (whether it can be instanced).
   */
  declare readonly isExtensible: boolean;

  /**
   * View.position
   */
  /**
   * View.position
   */
  abstract get position(): Position | null;
  abstract set position(value: Position | null);

  /**
   * View.width
   */
  /**
   * View.width
   */
  abstract get width(): Dimension | null;
  abstract set width(value: Dimension | null);

  /**
   * View.height
   */
  /**
   * View.height
   */
  abstract get height(): Dimension | null;
  abstract set height(value: Dimension | null);

  /**
   * View.minWidth
   */
  /**
   * View.minWidth
   */
  abstract get minWidth(): Dimension | null;
  abstract set minWidth(value: Dimension | null);

  /**
   * View.minHeight
   */
  /**
   * View.minHeight
   */
  abstract get minHeight(): Dimension | null;
  abstract set minHeight(value: Dimension | null);

  /**
   * View.maxWidth
   */
  /**
   * View.maxWidth
   */
  abstract get maxWidth(): Dimension | null;
  abstract set maxWidth(value: Dimension | null);

  /**
   * View.maxHeight
   */
  /**
   * View.maxHeight
   */
  abstract get maxHeight(): Dimension | null;
  abstract set maxHeight(value: Dimension | null);

  /**
   * ContainerView.layout
   */
  /**
   * ContainerView.layout
   */
  abstract get layout(): Layout | null;
  abstract set layout(value: Layout | null);

  /**
   * ContainerView.direction
   */
  /**
   * ContainerView.direction
   */
  abstract get direction(): Direction | null;
  abstract set direction(value: Direction | null);

  /**
   * ContainerView.distribute
   */
  /**
   * ContainerView.distribute
   */
  abstract get distribute(): Distribute | null;
  abstract set distribute(value: Distribute | null);

  /**
   * ContainerView.align
   */
  /**
   * ContainerView.align
   */
  abstract get align(): Align | null;
  abstract set align(value: Align | null);

  /**
   * ContainerView.gap
   */
  /**
   * ContainerView.gap
   */
  abstract get gap(): Axis2 | null;
  abstract set gap(value: Axis2 | null);

  /**
   * ContainerView.padding
   */
  /**
   * ContainerView.padding
   */
  abstract get padding(): Insets | null;
  abstract set padding(value: Insets | null);

  /**
   * ContainerView.grid
   */
  /**
   * ContainerView.grid
   */
  abstract get grid(): Grid | null;
  abstract set grid(value: Grid | null);

  /**
   * ContainerView.gridSpan
   */
  /**
   * ContainerView.gridSpan
   */
  abstract get gridSpan(): GridSpan | null;
  abstract set gridSpan(value: GridSpan | null);

  /**
   * ContainerView.aspectRatio
   */
  /**
   * ContainerView.aspectRatio
   */
  abstract get aspectRatio(): number | null;
  abstract set aspectRatio(value: number | null);

  /**
   * ContainerView.isWrap
   */
  /**
   * ContainerView.isWrap
   */
  abstract get isWrap(): boolean | null;
  abstract set isWrap(value: boolean | null);

  /**
   * ContainerView.isVisible
   */
  /**
   * ContainerView.isVisible
   */
  abstract get isVisible(): boolean | null;
  abstract set isVisible(value: boolean | null);

  /**
   * ContainerView.opacity
   */
  /**
   * ContainerView.opacity
   */
  abstract get opacity(): number | null;
  abstract set opacity(value: number | null);

  /**
   * ContainerView.fill
   */
  /**
   * ContainerView.fill
   */
  abstract get fill(): Fill | null;
  abstract set fill(value: Fill | null);

  /**
   * ContainerView.rotation
   */
  /**
   * ContainerView.rotation
   */
  abstract get rotation(): Axis3 | null;
  abstract set rotation(value: Axis3 | null);

  /**
   * ContainerView.skew
   */
  /**
   * ContainerView.skew
   */
  abstract get skew(): Vector2f | null;
  abstract set skew(value: Vector2f | null);

  /**
   * ContainerView.scale
   */
  /**
   * ContainerView.scale
   */
  abstract get scale(): number | null;
  abstract set scale(value: number | null);

  /**
   * ContainerView.shadow
   */
  /**
   * ContainerView.shadow
   */
  abstract get shadow(): Shadow | null;
  abstract set shadow(value: Shadow | null);

  /**
   * ContainerView.border
   */
  /**
   * ContainerView.border
   */
  abstract get border(): Border | null;
  abstract set border(value: Border | null);

  /**
   * ContainerView.radius
   */
  /**
   * ContainerView.radius
   */
  abstract get radius(): Corners | null;
  abstract set radius(value: Corners | null);

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerNodeClass(NodeType.CONTAINER_VIEW, ContainerView);
/* ==== DESTACK_GENERATED_END:NODE:1800100 ==== */
