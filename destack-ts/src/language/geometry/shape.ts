import type {
  Branch,
  Materialization,
  NodeReference,
  Snapshot,
  Space,
  Value,
} from "@destack/language/core";
import { Entity, Entity2D, Entity3D, NodeType } from "@destack/language/core";
import type { Quaternion } from "@destack/language/geometry/quaternion";
import type { Anchor, Offset2 } from "@destack/language/geometry/relative";
import type { Vector2, Vector3 } from "@destack/language/geometry/vector";
import type { Script } from "@destack/language/logic";
import { registerNodeClass } from "@destack/language/registry";
import type { Stroke } from "@destack/language/style";
import { Temporal } from "temporal-polyfill";

/* ==== DESTACK_GENERATED_START:NODE:2410000 ==== */
/**
 * A Shape2D represents 2-dimensional geometric Shapes.
 */
export abstract class Shape2D extends Entity2D {
  static metatype: NodeType = NodeType.SHAPE2D;

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
   * The definition this Entity is an instance of.
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
  abstract get precededBy(): Shape2D | null;
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
  abstract get createdBy(): Entity | null;
  declare readonly createdByPtr: NodeReference;

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
  abstract get updatedBy(): Entity | null;
  declare readonly updatedByPtr: NodeReference;

  /**
   * The time this Entity was deleted (system time).
   * Only set if the Entity is currently 'deleted'.
   * Deleting and restoring an Entity counts as an update, and thus updates updated_at/updated_epoch.
   */
  declare readonly deletedAt: Temporal.ZonedDateTime | null;

  /**
   * Entity.ownedBy
   */
  abstract get ownedBy(): Entity | null;
  abstract set ownedBy(value: Entity | null);
  /**
   * Entity.ownedBy
   */
  abstract get ownedByPtr(): NodeReference | null;
  abstract set ownedByPtr(value: NodeReference | null);

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
  abstract get customValues(): { readonly [key: string]: Value };
  abstract set customValues(value: { readonly [key: string]: Value });

  /**
   * The Script of this Entity.
   */
  abstract get script(): Script | null;
  abstract set script(value: Script | null);
  /**
   * The Script of this Entity.
   */
  abstract get scriptPtr(): NodeReference | null;
  abstract set scriptPtr(value: NodeReference | null);

  /**
   * Whether this Entity can be instanced.
   */
  declare readonly isExtensible: boolean | null;

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
   * Shape2D.stroke
   */
  /**
   * Shape2D.stroke
   */
  abstract get stroke(): Stroke | null;
  abstract set stroke(value: Stroke | null);

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerNodeClass(NodeType.SHAPE2D, Shape2D);
/* ==== DESTACK_GENERATED_END:NODE:2410000 ==== */

/* ==== DESTACK_GENERATED_START:NODE:2415000 ==== */
/**
 * A Shape3D represents 3-dimensional geometric Shapes.
 */
export abstract class Shape3D extends Entity3D {
  static metatype: NodeType = NodeType.SHAPE3D;

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
   * The definition this Entity is an instance of.
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
  abstract get precededBy(): Shape3D | null;
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
  abstract get createdBy(): Entity | null;
  declare readonly createdByPtr: NodeReference;

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
  abstract get updatedBy(): Entity | null;
  declare readonly updatedByPtr: NodeReference;

  /**
   * The time this Entity was deleted (system time).
   * Only set if the Entity is currently 'deleted'.
   * Deleting and restoring an Entity counts as an update, and thus updates updated_at/updated_epoch.
   */
  declare readonly deletedAt: Temporal.ZonedDateTime | null;

  /**
   * Entity.ownedBy
   */
  abstract get ownedBy(): Entity | null;
  abstract set ownedBy(value: Entity | null);
  /**
   * Entity.ownedBy
   */
  abstract get ownedByPtr(): NodeReference | null;
  abstract set ownedByPtr(value: NodeReference | null);

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
  abstract get customValues(): { readonly [key: string]: Value };
  abstract set customValues(value: { readonly [key: string]: Value });

  /**
   * The Script of this Entity.
   */
  abstract get script(): Script | null;
  abstract set script(value: Script | null);
  /**
   * The Script of this Entity.
   */
  abstract get scriptPtr(): NodeReference | null;
  abstract set scriptPtr(value: NodeReference | null);

  /**
   * Whether this Entity can be instanced.
   */
  declare readonly isExtensible: boolean | null;

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
   * Entity3D.position
   */
  /**
   * Entity3D.position
   */
  abstract get position(): Vector3 | null;
  abstract set position(value: Vector3 | null);

  /**
   * Entity3D.scale
   */
  /**
   * Entity3D.scale
   */
  abstract get scale(): Vector3 | null;
  abstract set scale(value: Vector3 | null);

  /**
   * Entity3D.rotation
   */
  /**
   * Entity3D.rotation
   */
  abstract get rotation(): Quaternion | null;
  abstract set rotation(value: Quaternion | null);

  /**
   * Entity3D.skew
   */
  /**
   * Entity3D.skew
   */
  abstract get skew(): Vector3 | null;
  abstract set skew(value: Vector3 | null);

  /**
   * Entity3D.origin
   */
  /**
   * Entity3D.origin
   */
  abstract get origin(): Vector3 | null;
  abstract set origin(value: Vector3 | null);

  /**
   * Entity3D.anchor
   */
  /**
   * Entity3D.anchor
   */
  abstract get anchor(): Anchor | null;
  abstract set anchor(value: Anchor | null);

  /**
   * Shape3D.stroke
   */
  /**
   * Shape3D.stroke
   */
  abstract get stroke(): Stroke | null;
  abstract set stroke(value: Stroke | null);

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerNodeClass(NodeType.SHAPE3D, Shape3D);
/* ==== DESTACK_GENERATED_END:NODE:2415000 ==== */
