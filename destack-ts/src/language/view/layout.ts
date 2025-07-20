import type {
  Branch,
  IsActor,
  Materialization,
  NodeReference,
  Snapshot,
  Space,
  Value,
} from "@destack/language/core";
import { Entity, NodeType } from "@destack/language/core";
import type { Vector2 } from "@destack/language/geometry";
import type { Script } from "@destack/language/logic";
import { registerNodeClass } from "@destack/language/registry";
import type { Border, Fill, Shadow } from "@destack/language/style";
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
  Layout,
  Position,
} from "@destack/language/view/common";
import { View } from "@destack/language/view/view";
import { Temporal } from "temporal-polyfill";

/* ==== DESTACK_GENERATED_START:NODE:1800100 ==== */
/**
 * A Layout View defines how its children Views are laid out.
 */
export abstract class LayoutView extends View {
  static metatype: NodeType = NodeType.LAYOUT_VIEW;

  /**
   * The parent of this Entity. Most Entities can be attached to any other Entity.
   */
  abstract get parent(): Entity | null;
  declare readonly parentPtr: NodeReference | null;

  /**
   * The Space this Node is in.
   */
  abstract get space(): Space | null;
  declare readonly spacePtr: NodeReference;

  /**
   * Entity.materialization
   */
  declare readonly materialization: Materialization;

  /**
   * The definition this CustomEntity is an instance of.
   */
  abstract get definition(): Entity | null;
  declare readonly definitionPtr: NodeReference | null;

  /**
   * The Branch this Entity is part of.
   */
  abstract get branch(): Branch | null;
  declare readonly branchPtr: NodeReference;

  /**
   * The Snapshot this Entity is part of.
   */
  abstract get snapshot(): Snapshot | null;
  declare readonly snapshotPtr: NodeReference;

  /**
   * The previous Entity this Entity is based on (from the base Branch, if any).
   * This invariant must hold: `Entity.preceded_by.branch == Entity.branch.preceded_by`.
   */
  abstract get precededBy(): LayoutView | null;
  declare readonly precededByPtr: NodeReference | null;

  /**
   * The (root) Entity that is being instantiated.
   */
  abstract get instance(): Entity | null;
  declare readonly instancePtr: NodeReference | null;

  /**
   * The time this Entity was created (system time).
   */
  declare readonly createdAt: Temporal.ZonedDateTime;

  /**
   * The logical time this Entity was created (system time).
   */
  declare readonly createdEpoch: number;

  /**
   * The Actor that created this Entity.
   */
  abstract get createdBy(): (Entity & IsActor) | null;
  declare readonly createdByPtr: NodeReference | null;

  /**
   * The time this Entity was last updated (system time).
   */
  declare readonly updatedAt: Temporal.ZonedDateTime;

  /**
   * The logical time this Entity was last updated (system time).
   */
  declare readonly updatedEpoch: number;

  /**
   * The Actor that last updated this Entity.
   */
  abstract get updatedBy(): (Entity & IsActor) | null;
  declare readonly updatedByPtr: NodeReference | null;

  /**
   * The time this Entity was deleted (system time).
   * Only set if the Entity is currently 'deleted'.
   * Deleting and restoring an Entity counts as an update, and thus updates updated_at/updated_epoch.
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
   * Entity.name
   */
  /**
   * Entity.name
   */
  abstract get name(): string;
  abstract set name(value: string);

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
   * View.scale
   */
  /**
   * View.scale
   */
  abstract get scale(): number | null;
  abstract set scale(value: number | null);

  /**
   * View.rotation
   */
  /**
   * View.rotation
   */
  abstract get rotation(): Axis3 | null;
  abstract set rotation(value: Axis3 | null);

  /**
   * View.skew
   */
  /**
   * View.skew
   */
  abstract get skew(): Vector2 | null;
  abstract set skew(value: Vector2 | null);

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
   * View.isVisible
   */
  /**
   * View.isVisible
   */
  abstract get isVisible(): boolean | null;
  abstract set isVisible(value: boolean | null);

  /**
   * View.opacity
   */
  /**
   * View.opacity
   */
  abstract get opacity(): number | null;
  abstract set opacity(value: number | null);

  /**
   * View.fill
   */
  /**
   * View.fill
   */
  abstract get fill(): Fill | null;
  abstract set fill(value: Fill | null);

  /**
   * View.shadow
   */
  /**
   * View.shadow
   */
  abstract get shadow(): Shadow | null;
  abstract set shadow(value: Shadow | null);

  /**
   * View.border
   */
  /**
   * View.border
   */
  abstract get border(): Border | null;
  abstract set border(value: Border | null);

  /**
   * View.radius
   */
  /**
   * View.radius
   */
  abstract get radius(): Corners | null;
  abstract set radius(value: Corners | null);

  /**
   * LayoutView.layout
   */
  /**
   * LayoutView.layout
   */
  abstract get layout(): Layout | null;
  abstract set layout(value: Layout | null);

  /**
   * LayoutView.direction
   */
  /**
   * LayoutView.direction
   */
  abstract get direction(): Direction | null;
  abstract set direction(value: Direction | null);

  /**
   * LayoutView.distribute
   */
  /**
   * LayoutView.distribute
   */
  abstract get distribute(): Distribute | null;
  abstract set distribute(value: Distribute | null);

  /**
   * LayoutView.align
   */
  /**
   * LayoutView.align
   */
  abstract get align(): Align | null;
  abstract set align(value: Align | null);

  /**
   * LayoutView.gap
   */
  /**
   * LayoutView.gap
   */
  abstract get gap(): Axis2 | null;
  abstract set gap(value: Axis2 | null);

  /**
   * LayoutView.padding
   */
  /**
   * LayoutView.padding
   */
  abstract get padding(): Insets | null;
  abstract set padding(value: Insets | null);

  /**
   * LayoutView.grid
   */
  /**
   * LayoutView.grid
   */
  abstract get grid(): Grid | null;
  abstract set grid(value: Grid | null);

  /**
   * LayoutView.gridSpan
   */
  /**
   * LayoutView.gridSpan
   */
  abstract get gridSpan(): GridSpan | null;
  abstract set gridSpan(value: GridSpan | null);

  /**
   * LayoutView.aspectRatio
   */
  /**
   * LayoutView.aspectRatio
   */
  abstract get aspectRatio(): number | null;
  abstract set aspectRatio(value: number | null);

  /**
   * LayoutView.isWrap
   */
  /**
   * LayoutView.isWrap
   */
  abstract get isWrap(): boolean | null;
  abstract set isWrap(value: boolean | null);

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerNodeClass(NodeType.LAYOUT_VIEW, LayoutView);
/* ==== DESTACK_GENERATED_END:NODE:1800100 ==== */
