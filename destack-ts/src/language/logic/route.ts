import type {
  IsDeletable,
  IsOrdered,
  IsOwnable,
  IsSubject,
  IsTaggable,
  Materialization,
  NodeReference,
  Snapshot,
} from "@destack/language/core";
import { Entity, NodeType } from "@destack/language/core";
import { registerNodeClass } from "@destack/language/registry";
import type { Folder } from "@destack/language/space";
import type { Space } from "@destack/language/universe";
import { Temporal } from "temporal-polyfill";

/* ==== DESTACK_GENERATED_START:NODE:110000 ==== */
/**
 * A Route is a path to something (a Scene, a View in a Scene, an Action, etc.).
 */
export abstract class Route
  extends Entity
  implements IsDeletable, IsOrdered, IsOwnable, IsTaggable
{
  static metatype: NodeType = NodeType.ROUTE;

  /**
   * Route.parent
   */
  abstract get parent(): Folder | null;
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
   * The Snapshot this Entity is part of.
   */
  abstract get snapshot(): Snapshot | null;
  declare readonly snapshotPtr: NodeReference | null;

  /**
   * The previous Entity this Entity is based on (from another Snapshot).
   */
  abstract get predecessor(): Route | null;
  declare readonly predecessorPtr: NodeReference | null;

  /**
   * The template this Entity instance is based on (from the template tree).
   */
  abstract get template(): Route | null;
  declare readonly templatePtr: NodeReference | null;

  /**
   * The time this Entity was created.
   */
  declare readonly createdAt: Temporal.ZonedDateTime;

  /**
   * The Subject that created this Entity.
   */
  abstract get createdBy(): (Entity & IsSubject) | null;
  declare readonly createdByPtr: NodeReference | null;

  /**
   * The time this Entity was last updated.
   */
  declare readonly updatedAt: Temporal.ZonedDateTime;

  /**
   * The Subject that last updated this Entity.
   */
  abstract get updatedBy(): (Entity & IsSubject) | null;
  declare readonly updatedByPtr: NodeReference | null;

  /**
   * IsDeletable.deletedAt
   */
  declare readonly deletedAt: Temporal.ZonedDateTime | null;

  /**
   * The absolute order key of this Node in its parent.
   */
  declare readonly orderKey: string;

  /**
   * IsOwnable.ownedBy
   */
  abstract get ownedBy(): (Entity & IsSubject) | null;
  abstract set ownedBy(value: (Entity & IsSubject) | null);
  /**
   * IsOwnable.ownedBy
   */
  abstract get ownedByPtr(): NodeReference | null;
  abstract set ownedByPtr(value: NodeReference | null);

  /**
   * The name of the Route.
   */
  /**
   * The name of the Route.
   */
  abstract get name(): string;
  abstract set name(value: string);

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerNodeClass(NodeType.ROUTE, Route);
/* ==== DESTACK_GENERATED_END:NODE:110000 ==== */
