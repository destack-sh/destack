import type {
  Branch,
  EventStatus,
  IsActor,
  IsExtensible,
  IsSourceable,
  IsViewable,
  Materialization,
  NodeReference,
  Snapshot,
  Space,
  Value,
} from "@destack/language/core";
import { Entity, Event, NodeType } from "@destack/language/core";
import type { Script } from "@destack/language/logic";
import { registerNodeClass } from "@destack/language/registry";
import type { Client } from "@destack/language/universe";
import type { Dimension, Position } from "@destack/language/view/common";
import { Temporal } from "temporal-polyfill";

/* ==== DESTACK_GENERATED_START:NODE:1800001 ==== */
/**
 * A Event regarding a View.
 */
export abstract class ViewEvent extends Event {
  static metatype: NodeType = NodeType.VIEW_EVENT;

  /**
   * The Space this Node is in.
   */
  abstract get space(): Space | null;
  declare readonly spacePtr: NodeReference;

  /**
   * The Branch this Event originated from.
   */
  abstract get branch(): Branch | null;
  declare readonly branchPtr: NodeReference;

  /**
   * The Snapshot this Event originated from.
   */
  abstract get snapshot(): Snapshot | null;
  declare readonly snapshotPtr: NodeReference;

  /**
   * The previous Event that this Event follows.
   */
  abstract get precededBy(): Event | null;
  declare readonly precededByPtr: NodeReference | null;

  /**
   * The Event that caused this Event (if any).
   */
  abstract get causedBy(): Event | null;
  declare readonly causedByPtr: NodeReference | null;

  /**
   * The time this Event was created (system time).
   */
  declare readonly createdAt: Temporal.ZonedDateTime;

  /**
   * The logical time this Event was created (system time).
   */
  declare readonly createdEpoch: number;

  /**
   * The Actor that created this Event.
   */
  abstract get createdBy(): (Entity & IsActor) | null;
  declare readonly createdByPtr: NodeReference | null;

  /**
   * The Client that created this Event.
   */
  abstract get client(): Client | null;
  declare readonly clientPtr: NodeReference | null;

  /**
   * The nonce of the Client that created this Event.
   */
  declare readonly clientNonce: string | null;

  /**
   * The time in the Client when it created this Event.
   */
  declare readonly clientCreatedAt: Temporal.ZonedDateTime;

  /**
   * The logical time in the Client when it created this Event.
   */
  declare readonly clientEpoch: number;

  /**
   * The status of the Event.
   */
  declare readonly status: EventStatus;

  /**
   * ViewEvent.node
   */
  abstract get node(): View | null;
  declare readonly nodePtr: NodeReference;

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerNodeClass(NodeType.VIEW_EVENT, ViewEvent);
/* ==== DESTACK_GENERATED_END:NODE:1800001 ==== */

/* ==== DESTACK_GENERATED_START:NODE:1800000 ==== */
/**
 * A View is a graphical interface.
 */
export abstract class View extends Entity implements IsViewable, IsExtensible, IsSourceable {
  static metatype: NodeType = NodeType.VIEW;

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
  abstract get precededBy(): View | null;
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

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerNodeClass(NodeType.VIEW, View);
/* ==== DESTACK_GENERATED_END:NODE:1800000 ==== */
