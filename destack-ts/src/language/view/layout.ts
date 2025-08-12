import type {
  Branch,
  DateTime,
  Float32,
  Materialization,
  NodeReference,
  Snapshot,
  Space,
  UInt128,
  UUID,
  Value,
} from "@destack/language/core";
import { type Entity, NodeType } from "@destack/language/core";
import type {
  Align,
  Anchor,
  Axis2,
  Corner2,
  Direction,
  Distribute,
  Grid2,
  GridSpan2,
  Inset2,
  Layout,
  Length,
  Offset2,
  Vector2,
} from "@destack/language/geometry";
import type { Script } from "@destack/language/logic";
import { registerNodeClass } from "@destack/language/registry";
import type { Border, Fill, Shadow } from "@destack/language/style";
import { View } from "@destack/language/view/view";

/* ==== DESTACK_GENERATED_START:NODE:1800100 ==== */
/**
 * A Layout View defines how its children  are laid out.
 */
export abstract class LayoutView extends View {
  static metatype: NodeType = NodeType.LAYOUT_VIEW;

  /**
   * The parent of this Entity. Most Entities can be attached to any other Entity.
   */
  abstract get parent(): Entity | null;
  declare readonly parentRef: NodeReference | null;

  /**
   * The Space this Node is in.
   */
  abstract get space(): Space | null;
  declare readonly spaceRef: NodeReference;

  /**
   * Entity.materialization
   */
  declare readonly materialization: Materialization;

  /**
   * The definition this Entity is an instance of.
   */
  abstract get definition(): Entity | null;
  declare readonly definitionRef: NodeReference | null;

  /**
   * The Branch this Entity is part of.
   */
  abstract get branch(): Branch | null;
  declare readonly branchRef: NodeReference;

  /**
   * The Snapshot this Entity is part of.
   */
  abstract get snapshot(): Snapshot | null;
  declare readonly snapshotRef: NodeReference;

  /**
   * The previous Entity this Entity is based on (from the base Branch, if any).
   * This invariant must hold: `Entity.preceded_by.branch == Entity.branch.preceded_by`.
   */
  abstract get precededBy(): LayoutView | null;
  declare readonly precededByRef: NodeReference | null;

  /**
   * The (root) Entity that is being instantiated.
   */
  abstract get instance(): Entity | null;
  declare readonly instanceRef: NodeReference | null;

  /**
   * The time this Entity was created (system time).
   */
  declare readonly createdAt: DateTime;

  /**
   * The logical time this Entity was created (system time).
   */
  declare readonly createdEpoch: UInt128;

  /**
   * The Actor that created this Entity.
   */
  abstract get createdBy(): Entity | null;
  declare readonly createdByRef: NodeReference;

  /**
   * The time this Entity was last updated (system time).
   */
  declare readonly updatedAt: DateTime;

  /**
   * The logical time this Entity was last updated (system time).
   */
  declare readonly updatedEpoch: UInt128;

  /**
   * The Actor that last updated this Entity.
   */
  abstract get updatedBy(): Entity | null;
  declare readonly updatedByRef: NodeReference;

  /**
   * The time this Entity was deleted (system time).
   * Only set if the Entity is currently 'deleted'.
   * Deleting and restoring an Entity counts as an update, and thus updates updated_at/updated_epoch.
   */
  declare readonly deletedAt: DateTime | null;

  /**
   * Entity.ownedBy
   */
  abstract get ownedBy(): Entity | null;
  abstract set ownedBy(value: Entity | null);
  /**
   * Entity.ownedBy
   */
  abstract get ownedByRef(): NodeReference | null;
  abstract set ownedByRef(value: NodeReference | null);

  /**
   * Entity.name
   */
  /**
   * Entity.name
   */
  abstract get name(): string;
  abstract set name(value: string);

  /**
   * The absolute order key of this Entity in its parent.
   */
  declare readonly orderKey: string;

  /**
   * The custom Values of this Entity, keyed by custom Property id..
   */
  /**
   * The custom Values of this Entity, keyed by custom Property id..
   */
  abstract get customValues(): { readonly [key: UUID]: Value };
  abstract set customValues(value: { readonly [key: UUID]: Value });

  /**
   * The Script of this Entity.
   */
  abstract get script(): Script | null;
  abstract set script(value: Script | null);
  /**
   * The Script of this Entity.
   */
  abstract get scriptRef(): NodeReference | null;
  abstract set scriptRef(value: NodeReference | null);

  /**
   * Whether this Entity can be instanced.
   */
  declare readonly isExtensible: boolean | null;

  /**
   * The Script that defines this Node.
   */
  abstract get source(): Script | null;
  declare readonly sourceRef: NodeReference | null;

  /**
   * The key to uniquely identify this Node in reconciliation. If not set, name is used.
   */
  /**
   * The key to uniquely identify this Node in reconciliation. If not set, name is used.
   */
  abstract get key(): string | null;
  abstract set key(value: string | null);

  /**
   * Entity2D.position
   */
  /**
   * Entity2D.position
   */
  abstract get position(): Vector2 | null;
  abstract set position(value: Vector2 | null);

  /**
   * Entity2D.offset
   */
  /**
   * Entity2D.offset
   */
  abstract get offset(): Offset2 | null;
  abstract set offset(value: Offset2 | null);

  /**
   * Entity2D.scale
   */
  /**
   * Entity2D.scale
   */
  abstract get scale(): Vector2 | null;
  abstract set scale(value: Vector2 | null);

  /**
   * Entity2D.rotation
   */
  /**
   * Entity2D.rotation
   */
  abstract get rotation(): Vector2 | null;
  abstract set rotation(value: Vector2 | null);

  /**
   * Entity2D.skew
   */
  /**
   * Entity2D.skew
   */
  abstract get skew(): Vector2 | null;
  abstract set skew(value: Vector2 | null);

  /**
   * Entity2D.origin
   */
  /**
   * Entity2D.origin
   */
  abstract get origin(): Vector2 | null;
  abstract set origin(value: Vector2 | null);

  /**
   * Entity2D.anchor
   */
  /**
   * Entity2D.anchor
   */
  abstract get anchor(): Anchor | null;
  abstract set anchor(value: Anchor | null);

  /**
   * View.width
   */
  /**
   * View.width
   */
  abstract get width(): Length | null;
  abstract set width(value: Length | null);

  /**
   * View.height
   */
  /**
   * View.height
   */
  abstract get height(): Length | null;
  abstract set height(value: Length | null);

  /**
   * View.minWidth
   */
  /**
   * View.minWidth
   */
  abstract get minWidth(): Length | null;
  abstract set minWidth(value: Length | null);

  /**
   * View.minHeight
   */
  /**
   * View.minHeight
   */
  abstract get minHeight(): Length | null;
  abstract set minHeight(value: Length | null);

  /**
   * View.maxWidth
   */
  /**
   * View.maxWidth
   */
  abstract get maxWidth(): Length | null;
  abstract set maxWidth(value: Length | null);

  /**
   * View.maxHeight
   */
  /**
   * View.maxHeight
   */
  abstract get maxHeight(): Length | null;
  abstract set maxHeight(value: Length | null);

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
  abstract get opacity(): Float32 | null;
  abstract set opacity(value: Float32 | null);

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
  abstract get radius(): Corner2 | null;
  abstract set radius(value: Corner2 | null);

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
  abstract get padding(): Inset2 | null;
  abstract set padding(value: Inset2 | null);

  /**
   * LayoutView.grid
   */
  /**
   * LayoutView.grid
   */
  abstract get grid(): Grid2 | null;
  abstract set grid(value: Grid2 | null);

  /**
   * LayoutView.gridSpan
   */
  /**
   * LayoutView.gridSpan
   */
  abstract get gridSpan(): GridSpan2 | null;
  abstract set gridSpan(value: GridSpan2 | null);

  /**
   * LayoutView.aspectRatio
   */
  /**
   * LayoutView.aspectRatio
   */
  abstract get aspectRatio(): Float32 | null;
  abstract set aspectRatio(value: Float32 | null);

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
