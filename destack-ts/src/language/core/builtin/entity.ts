import { type Tag, Tagging } from "@destack/language/core/builtin/base";
import { EnumType, NodeType, TraitType } from "@destack/language/core/builtin/builtin";
import type { NodeClass } from "@destack/language/core/builtin/node";
import { hasTrait, Node } from "@destack/language/core/builtin/node";
import type { NodeReference } from "@destack/language/core/builtin/relation";
import type { Datetime, UInt128, UUID } from "@destack/language/core/builtin/types";
import type { Value } from "@destack/language/core/builtin/value";
import type { Space } from "@destack/language/core/common/space";
import type { Branch, Snapshot } from "@destack/language/core/common/time";
import type { Script } from "@destack/language/logic";
import {
  NODE_CLASS_BY_TYPE,
  registerEnumClass,
  registerNodeClass,
} from "@destack/language/registry";
import { getOrderKey } from "@destack/utils";

/* ==== DESTACK_GENERATED_START:NODE:2 ==== */
/**
 * An Entity is a named, versioned, mutable Node.
 * Entities can be attached to (most) other Entities to compose richer structures.
 *
 * Updates to Entities can only be affected through Events.
 * Entities are always part of a Snapshot (in their Space).
 *
 * An instance of an Entity is identified by an (id, branch_id, snapshot_id) tuple,
 *  where Snapshots are 'shortcuts' to certain epochs.
 *  (id, definition_id) @ (branch_id, snapshot_id)
 */
export abstract class Entity extends Node {
  static metatype: NodeType = NodeType.ENTITY;

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
  abstract get precededBy(): Entity | null;
  declare readonly precededByPtr: NodeReference | null;

  /**
   * The (root) Entity that is being instantiated.
   */
  abstract get instance(): Entity | null;
  declare readonly instancePtr: NodeReference | null;

  /**
   * The time this Entity was created (system time).
   */
  declare readonly createdAt: Datetime;

  /**
   * The logical time this Entity was created (system time).
   */
  declare readonly createdEpoch: UInt128;

  /**
   * The Actor that created this Entity.
   */
  abstract get createdBy(): Entity | null;
  declare readonly createdByPtr: NodeReference;

  /**
   * The time this Entity was last updated (system time).
   */
  declare readonly updatedAt: Datetime;

  /**
   * The logical time this Entity was last updated (system time).
   */
  declare readonly updatedEpoch: UInt128;

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
  declare readonly deletedAt: Datetime | null;

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

  /* ==== DESTACK_CUSTOM_START ==== */

  /** Get the children of this Node. */
  getChildren(): Node[];
  getChildren<N extends Node>(nodeType?: NodeClass<N>): N[];
  getChildren(nodeType?: NodeClass): Node[] {
    return this._session.graph.getChildren({
      node: this,
      spaceId: this.spacePtr.id,
      branchId: this.branchPtr.id,
      snapshotId: this.snapshotPtr.id,
      type: nodeType?.metatype,
    });
  }

  /** Get a specific child of this Node by name. */
  getChild<N extends Node>(nodeType: NodeClass<N>, name: string): N | null;
  getChild(nodeType: NodeClass, name: string): Node | null;
  getChild(nodeType: NodeClass, name: string): Node | null {
    const children = this._session.graph.getChildren({
      node: this,
      spaceId: this.spacePtr.id,
      branchId: this.branchPtr.id,
      snapshotId: this.snapshotPtr.id,
      type: nodeType.metatype,
    });
    for (const child of children) {
      if ((child as any).name === name) {
        return child;
      }
    }
    return null;
  }

  /** Get a specific child of this Node by name, or raises an error if not found. */
  child<N extends Node>(nodeType: NodeClass<N>, name: string): N;
  child(nodeType: NodeClass, name: string): Node;
  child(nodeType: NodeClass, name: string): Node {
    const child = this.getChild(nodeType, name);
    if (child === null) {
      throw new Error(`no child ${name} of ${this}`);
    }
    return child;
  }

  /** Get the descendants of this Node. */
  getDescendants(): Node[];
  getDescendants<N extends Node>(nodeType?: NodeClass<N>): N[];
  getDescendants(nodeType?: NodeClass): Node[] {
    return this._session.graph.getDescendants({
      node: this,
      spaceId: this.spacePtr.id,
      branchId: this.branchPtr.id,
      snapshotId: this.snapshotPtr.id,
      type: nodeType?.metatype,
    });
  }

  /** Detach this Entity from its parent. Error if it has no parent. */
  detach(): void {
    if (this.parentPtr === null) {
      throw new Error(`${this.repr()} has no parent to detach from`);
    }
    this.moveTo(null);
  }

  /**
   * Move this Entity to a new parent Entity.
   * If the Entity IsOrdered, it will be positioned (relative to after/before).
   * If the Entity is new, it will be automatically created in this Entity's Session (for convenience).
   * (The same applies to all descendants.)
   */
  moveTo(parent: Entity | null, options?: { after?: Entity; before?: Entity }): void {
    const { after, before } = options || {};

    // prepare graph & nodes
    const session = this._session;
    const nodes: Entity[] = [
      this,
      ...(session.graph.getDescendants({
        node: this,
        spaceId: this.spacePtr.id,
        branchId: this.branchPtr.id,
        snapshotId: this.snapshotPtr.id,
      }) as Entity[]),
    ];

    // assign order
    if (parent !== null && hasTrait(this, TraitType.ORDERED)) {
      parent._assignOrder(this, after, before);
    }

    // create new nodes
    if (this._isNew && parent !== null && !parent._isNew) {
      for (const node of nodes) {
        node._ref = null; // invalidate cached ref
        session.create(node);
      }
    }
  }

  /**
   * Add an Entity as a sibling of this Entity.
   * If the Entity IsOrdered, it will be positioned (relative to after/before).
   * If the Entity is new, it will be automatically created in this Entity's Session (for convenience).
   * (The same applies to all descendants.)
   */
  addSibling(sibling: Entity, options?: { after?: Entity; before?: Entity }): this {
    sibling.moveTo(this.parent, options);
    return this;
  }

  /** Add multiple Entities as siblings of this Entity. */
  addSiblings(siblings: Entity[], options?: { after?: Entity; before?: Entity }): this {
    for (const sibling of siblings) {
      sibling.moveTo(this.parent, options);
    }
    return this;
  }

  /**
   * Append an Entity as a child of this Entity (and all its descendants).
   * If the Entity IsOrdered, it will be positioned (relative to after/before).
   * If the Entity is new, it will be automatically created in this Entity's Session (for convenience).
   * (The same applies to all descendants.)
   */
  addChild(child: Entity, options?: { after?: Entity; before?: Entity }): this {
    child.moveTo(this, options);
    return this;
  }

  /** Append multiple Entities as children of this Entity. */
  addChildren(children: Entity[], options?: { after?: Entity; before?: Entity }): this {
    for (const child of children) {
      child.moveTo(this, options);
    }
    return this;
  }

  /**
   * Remove a child Entity from this Entity.
   * The child Entity will NOT be deleted, it will simply be detached.
   */
  removeChild(child: Entity): this {
    child.detach();
    return this;
  }

  /** Add or get a Tagging for a Tag on this Entity. */
  addTag(tag: Tag): Tagging {
    const tagging = this.getChildren(Tagging).find((t) => t.tagPtr.id === tag.id);
    if (tagging) {
      return tagging;
    } else {
      const newTagging = new Tagging({ tag });
      this.addChild(newTagging);
      return newTagging;
    }
  }

  /** Remove a Tag from this Entity. */
  removeTag(tag: Tag): Tagging | null {
    const tagging = this.getChildren(Tagging).find((t) => t.tagPtr.id === tag.id);
    if (tagging) {
      this.removeChild(tagging);
      return tagging;
    } else {
      return null;
    }
  }

  /** Assign an order key to a child Entity. */
  private _assignOrder(
    child: Entity,
    after?: Entity,
    before?: Entity,
    existingNodes?: Entity[],
  ): void {
    const orderTrait =
      child.__inherits__.find((nodeType) =>
        NODE_CLASS_BY_TYPE[nodeType].__definition__.traits.includes(TraitType.ORDERED),
      ) ?? child.metatype;

    if (existingNodes === undefined) {
      const nodeClass = orderTrait
        ? NODE_CLASS_BY_TYPE[orderTrait]
        : (child.constructor as NodeClass);
      existingNodes = this._session.graph.getChildren({
        node: this,
        spaceId: this.spacePtr.id,
        branchId: this.branchPtr.id,
        snapshotId: this.snapshotPtr.id,
        type: nodeClass.metatype,
      }) as Entity[];
    }

    if (existingNodes.length > 0) {
      let afterOrderKey: string | null = null;
      if (after === undefined) {
        after = existingNodes[existingNodes.length - 1];
      }
      if (hasTrait(after, TraitType.ORDERED)) {
        afterOrderKey = (after as Entity).orderKey;
      }

      let beforeOrderKey: string | null = null;
      if (
        hasTrait(before, TraitType.ORDERED) &&
        afterOrderKey !== null &&
        (before as Entity).orderKey > afterOrderKey
      ) {
        beforeOrderKey = (before as Entity).orderKey;
      }

      const orderKey = getOrderKey(afterOrderKey, beforeOrderKey);
      // @ts-expect-error(readonly)
      (child as unknown as Entity).orderKey = orderKey;
    }
  }

  /* ==== DESTACK_CUSTOM_END ==== */
}
registerNodeClass(NodeType.ENTITY, Entity);
/* ==== DESTACK_GENERATED_END:NODE:2 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:14 ==== */
/**
 * Materialization
 */
export enum Materialization {
  VIRTUAL = 1,
  PARTIAL = 2,
  FULL = 10,
  ROOT = 11,

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerEnumClass(EnumType.MATERIALIZATION, Materialization);
/* ==== DESTACK_GENERATED_END:ENUM:14 ==== */
