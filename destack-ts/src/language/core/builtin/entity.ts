import { packProtoTimestamp, unpackProtoTimestamp } from "@destack/grpc";
import type { ResourceStatus } from "@destack/language/core/builtin/common";
import { EnumType, NodeType, StructType, TraitType } from "@destack/language/core/builtin/common";
import { ACTIVE_SPACE } from "@destack/language/core/builtin/const";
import { Event } from "@destack/language/core/builtin/event";
import type { NodeClass } from "@destack/language/core/builtin/node";
import { Node, hasTrait } from "@destack/language/core/builtin/node";
import type { NodeReference } from "@destack/language/core/builtin/relation";
import type {
  IsActor,
  IsArchivable,
  IsDeletable,
  IsExtensible,
  IsOwnable,
} from "@destack/language/core/builtin/trait";
import { INTER_ORDER_TYPES, IsOrdered } from "@destack/language/core/builtin/trait";
import type { Icon } from "@destack/language/core/common/icon";
import type { Value } from "@destack/language/core/common/value";
import type { QueryConnection } from "@destack/language/core/runtime/connection";
import type { Graph, Supergraph } from "@destack/language/core/runtime/graph";
import { EntitySingletonGraph } from "@destack/language/core/runtime/graph";
import type { Session } from "@destack/language/core/runtime/session";
import type { Script } from "@destack/language/logic";
import {
  NODE_CLASS_BY_TYPE,
  PARENT_TYPES_BY_NODE_TYPE,
  STRUCT_CLASS_BY_TYPE,
  registerEnumClass,
  registerNodeClass,
} from "@destack/language/registry";
import type { Space } from "@destack/language/universe";
import { MaterializationProto, SnapshotProto, SnapshotStatusProto } from "@destack/proto";
import { base64Decode, getOrderKey } from "@destack/utils";
import { hashString } from "@destack/utils/hash";
import { Temporal } from "temporal-polyfill";

/* ==== DESTACK_GENERATED_START:NODE:2 ==== */
/**
 * An Entity is a named, versioned, stateful Node.
 */
export abstract class Entity extends Node {
  static metatype: NodeType = NodeType.ENTITY;

  /**
   * Entity.parent
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
   * The Snapshot this Entity is part of.
   */
  abstract get snapshot(): Snapshot | null;
  declare readonly snapshotPtr: NodeReference | null;

  /**
   * The previous Entity this Entity is based on (from another Snapshot).
   */
  abstract get precededBy(): Entity | null;
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
   * Entity.name
   */
  /**
   * Entity.name
   */
  abstract get name(): string;
  abstract set name(value: string);

  /* ==== DESTACK_CUSTOM_START ==== */

  /** Get the children of this Node. */
  getChildren(): Node[];
  getChildren<N extends Node>(nodeType?: NodeClass<N>): N[];
  getChildren(nodeType?: NodeClass): Node[] {
    return this._graph.getChildren({ node: this, nodeType: nodeType?.metatype });
  }

  /** Get a specific child of this Node by name. */
  getChild<N extends Node>(nodeType: NodeClass<N>, name: string): N | null;
  getChild(nodeType: NodeClass, name: string): Node | null;
  getChild(nodeType: NodeClass, name: string): Node | null {
    const children = this._graph.getChildren({ node: this, nodeType: nodeType.metatype });
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
    return this._graph.getDescendants({ node: this, nodeType: nodeType?.metatype });
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
    const supergraph = this._supergraph;
    const oldGraph = this._graph;
    let newGraph: Graph<Entity>;
    let parentPtr: NodeReference | null;

    if (parent !== null) {
      // move to new parent
      if (oldGraph.supergraph !== parent._supergraph) {
        throw new Error(`${this.repr()} is not in supergraph of ${parent.repr()}`);
      }

      if (!PARENT_TYPES_BY_NODE_TYPE[this.metatype].includes(parent.metatype)) {
        throw new Error(
          `${parent.repr()} cannot parent ${this.repr()} (allowed: ${PARENT_TYPES_BY_NODE_TYPE[
            this.metatype
          ]
            .map((type) => NodeType[type])
            .join(", ")})`,
        );
      }

      // check space
      if (this.spacePtr!.id !== parent.spacePtr!.id) {
        throw new Error(`cannot move ${this.repr()} to ${parent.repr()} (different Space)`);
      }

      newGraph = parent._graph as Graph<Entity>; // has to be an Entity's Graph
      parentPtr = parent.toRef();

      // promote parent to polygraph if needed
      if (newGraph instanceof EntitySingletonGraph) {
        newGraph = supergraph.promoteToPolygraph(newGraph);
        parent._graph = newGraph;
      }
    } else {
      // detach from parent
      if (this.parentPtr === null) {
        return; // nothing to do
      }
      newGraph = supergraph.createEntityGraph();
      parentPtr = null;
    }

    const session = this._session;
    const nodes: Entity[] = [this, ...(this._graph.getDescendants({ node: this }) as Entity[])];

    // assign order
    if (parent !== null && hasTrait(this, TraitType.ORDERED)) {
      parent._assignOrder(this, after, before);
    }

    // move to new graph
    (this as any).parentPtr = parentPtr;
    if (oldGraph !== newGraph) {
      if (nodes.length === oldGraph.size) {
        // all nodes were moved
        supergraph.removeGraph(oldGraph);
      } else {
        for (const node of nodes) {
          oldGraph.remove(node);
        }
      }
      for (const node of nodes) {
        node._graph = newGraph;
        newGraph.add(node);
      }
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
   * The child Entity will NOT be deleted or archived, it will simply be detached.
   * (The same applies to all descendants.)
   */
  removeChild(child: Entity): this {
    child.detach();
    return this;
  }

  /** Assign an order key to a child Entity. */
  private _assignOrder(
    child: Entity,
    after?: Entity,
    before?: Entity,
    existingNodes?: Entity[],
  ): void {
    const orderTrait = child.__inherits__.find((trait) => INTER_ORDER_TYPES.includes(trait));

    if (existingNodes === undefined) {
      const nodeClass = orderTrait
        ? NODE_CLASS_BY_TYPE[orderTrait]
        : (child.constructor as NodeClass);
      existingNodes = this._graph.getChildren({
        node: this,
        nodeType: nodeClass.metatype,
      }) as Entity[];
    }

    if (existingNodes.length > 0) {
      let afterOrderKey: string | null = null;
      if (after === undefined) {
        after = existingNodes[existingNodes.length - 1];
      }
      if (hasTrait(after, TraitType.ORDERED)) {
        afterOrderKey = (after as unknown as Node & IsOrdered).orderKey;
      }

      let beforeOrderKey: string | null = null;
      if (
        hasTrait(before, TraitType.ORDERED) &&
        afterOrderKey !== null &&
        (before as unknown as Node & IsOrdered).orderKey > afterOrderKey
      ) {
        beforeOrderKey = (before as unknown as Node & IsOrdered).orderKey;
      }

      const orderKey = getOrderKey(afterOrderKey, beforeOrderKey);
      // @ts-expect-error(readonly)
      (child as unknown as Node & IsOrdered).orderKey = orderKey;
    }
  }

  /* ==== DESTACK_CUSTOM_END ==== */
}
registerNodeClass(NodeType.ENTITY, Entity);
/* ==== DESTACK_GENERATED_END:NODE:2 ==== */

/* ==== DESTACK_GENERATED_START:NODE:1000 ==== */
/**
 * A generic Record instance of a CustomEntityDefinition like a relational Table.
 * The Archivable, Deletable, and Ownable traits are always present for plain Records
 *  (but must be explicitly added to the CustomEntityDefinition to use them).
 * More specific base Entity types will be instanced of that base type instead.
 */
export abstract class Record
  extends Entity
  implements IsExtensible, IsArchivable, IsDeletable, IsOwnable
{
  static metatype: NodeType = NodeType.RECORD;

  /**
   * Entity.parent
   */
  abstract get parent(): Entity | null;
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
  abstract get precededBy(): Record | null;
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
   * IsArchivable.archivedAt
   */
  declare readonly archivedAt: Temporal.ZonedDateTime | null;

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
   * IsOwnable.ownedBy
   */
  abstract get ownedBy(): (Entity & IsActor) | null;
  abstract set ownedBy(value: (Entity & IsActor) | null);
  /**
   * IsOwnable.ownedBy
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

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerNodeClass(NodeType.RECORD, Record);
/* ==== DESTACK_GENERATED_END:NODE:1000 ==== */

/* ==== DESTACK_GENERATED_START:NODE:1100 ==== */
/**
 * A Resource represents an external asset outside of Destack.
 * The lifecycle of a Resource may be managed by some Provisioner (Service).
 */
export abstract class Resource extends Entity implements IsExtensible, IsDeletable {
  static metatype: NodeType = NodeType.RESOURCE;

  /**
   * Entity.parent
   */
  abstract get parent(): Entity | null;
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
  abstract get precededBy(): Resource | null;
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
   * Resource.status
   */
  /**
   * Resource.status
   */
  abstract get status(): ResourceStatus;
  abstract set status(value: ResourceStatus);

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

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerNodeClass(NodeType.RESOURCE, Resource);
/* ==== DESTACK_GENERATED_END:NODE:1100 ==== */

/* ==== DESTACK_GENERATED_START:NODE:1300 ==== */
/**
 * A Snapshot is a point in Space time.
 * Snapshots cannot be instanced, and they cannot be part of any other Snapshot.
 */
export class Snapshot extends Entity implements IsOwnable, IsArchivable, IsDeletable {
  static metatype: NodeType = NodeType.SNAPSHOT;

  /**
   * Snapshot.parent
   */
  get parent(): Space | Snapshot | null {
    const nodePtr: NodeReference | null = this.parentPtr;
    if (nodePtr != null) {
      return this._supergraph.get(nodePtr.id) as Space | Snapshot | null;
    }
    return null;
  }
  readonly parentPtr: NodeReference | null;

  /**
   * The Space this Node is in.
   */
  get space(): Space | null {
    const nodePtr: NodeReference | null = this.spacePtr;
    if (nodePtr != null) {
      return this._supergraph.get(nodePtr.id) as Space | null;
    }
    return null;
  }
  readonly spacePtr: NodeReference;

  /**
   * Entity.materialization
   */
  readonly materialization: Materialization;

  /**
   * The Snapshot itself. Cannot be any other Snapshot than this Snapshot
   */
  get snapshot(): Snapshot | null {
    const nodePtr: NodeReference | null = this.snapshotPtr;
    if (nodePtr != null) {
      return this._supergraph.get(nodePtr.id) as Snapshot | null;
    }
    return null;
  }
  readonly snapshotPtr: NodeReference;

  /**
   * The previous Snapshot this Snapshot is based on.
   */
  get precededBy(): Snapshot | null {
    const nodePtr: NodeReference | null = this.precededByPtr;
    if (nodePtr != null) {
      return this._supergraph.get(nodePtr.id) as Snapshot | null;
    }
    return null;
  }
  readonly precededByPtr: NodeReference | null;

  /**
   * The time this Entity was created.
   */
  readonly createdAt: Temporal.ZonedDateTime;

  /**
   * The Actor that created this Entity.
   */
  get createdBy(): (Entity & IsActor) | null {
    const nodePtr: NodeReference | null = this.createdByPtr;
    if (nodePtr != null) {
      return this._supergraph.get(nodePtr.id) as (Entity & IsActor) | null;
    }
    return null;
  }
  readonly createdByPtr: NodeReference | null;

  /**
   * The time this Entity was last updated.
   */
  readonly updatedAt: Temporal.ZonedDateTime;

  /**
   * The Actor that last updated this Entity.
   */
  get updatedBy(): (Entity & IsActor) | null {
    const nodePtr: NodeReference | null = this.updatedByPtr;
    if (nodePtr != null) {
      return this._supergraph.get(nodePtr.id) as (Entity & IsActor) | null;
    }
    return null;
  }
  readonly updatedByPtr: NodeReference | null;

  /**
   * IsArchivable.archivedAt
   */
  readonly archivedAt: Temporal.ZonedDateTime | null;

  /**
   * IsDeletable.deletedAt
   */
  readonly deletedAt: Temporal.ZonedDateTime | null;

  /**
   * IsOwnable.ownedBy
   */
  get ownedBy(): (Entity & IsActor) | null {
    const nodePtr: NodeReference | null = this.ownedByPtr;
    if (nodePtr != null) {
      return this._supergraph.get(nodePtr.id) as (Entity & IsActor) | null;
    }
    return null;
  }
  set ownedBy(node: (Entity & IsActor) | null) {
    if (node === null) {
      this.ownedByPtr = null;
    } else {
      this.ownedByPtr = node.toRef();
    }
  }
  /**
   * IsOwnable.ownedBy
   */
  get ownedByPtr(): NodeReference | null {
    return this._ownedByPtr;
  }
  set ownedByPtr(value: NodeReference | null) {
    const prop = (this.constructor as NodeClass).__properties__["owned_by"];
    this._session.updateSetProperty(this, prop, value);
    this._ownedByPtr = value;
  }
  _ownedByPtr: NodeReference | null;

  /**
   * Entity.name
   */
  /**
   * Entity.name
   */
  get name(): string {
    return this._name;
  }
  set name(value: string) {
    const prop = (this.constructor as NodeClass).__properties__["name"];
    this._session.updateSetProperty(this, prop, value);
    this._name = value;
  }
  _name: string;

  /**
   * Snapshot.status
   */
  /**
   * Snapshot.status
   */
  get status(): SnapshotStatus {
    return this._status;
  }
  set status(value: SnapshotStatus) {
    const prop = (this.constructor as NodeClass).__properties__["status"];
    this._session.updateSetProperty(this, prop, value);
    this._status = value;
  }
  _status: SnapshotStatus;

  constructor(options: {
    id?: string;
    parent?: Space | Snapshot | NodeReference | null;
    space?: Space | NodeReference;
    materialization?: Materialization;
    snapshot?: Snapshot | NodeReference;
    precededBy?: Snapshot | NodeReference | null;
    createdAt?: Temporal.ZonedDateTime;
    createdBy?: (Entity & IsActor) | NodeReference | null;
    updatedAt?: Temporal.ZonedDateTime;
    updatedBy?: (Entity & IsActor) | NodeReference | null;
    archivedAt?: Temporal.ZonedDateTime | null;
    deletedAt?: Temporal.ZonedDateTime | null;
    ownedBy?: (Entity & IsActor) | NodeReference | null;
    name?: string;
    status?: SnapshotStatus;
    _session?: Session | null;
    _supergraph?: Supergraph | null;
    _graph?: Graph | null;
    _connection?: QueryConnection | null;
  }) {
    super(
      // id
      options.id ?? null,
      // parent
      options.parent != null
        ? options.parent.metatype == StructType.NODE_REFERENCE
          ? (options.parent as NodeReference)
          : (options.parent as Node).toRef()
        : null,
      // session
      options._session ?? null,
      // supergraph
      options._supergraph ?? null,
      // graph
      options._graph ?? null,
      // connection
      options._connection ?? null,
      // is_new
      options.id == null,
    );

    // properties
    let _parent = options.parent ?? null;
    if (_parent != null && _parent.metatype != StructType.NODE_REFERENCE) {
      _parent = (_parent as Node).toRef();
    }
    this.parentPtr = _parent;
    let _space = options.space ?? null;
    if (_space != null && _space.metatype != StructType.NODE_REFERENCE) {
      _space = (_space as Node).toRef();
    }
    if (_space === null) {
      if (this._session === null) {
        throw new Error(`Snapshot has no Session`);
      }
      _space = ACTIVE_SPACE.get();
      if (_space === null) {
        throw new Error(`Snapshot has no Space`);
      }
      _space = _space.toRef();
    }
    if (_space === null) {
      throw new Error(`Snapshot.space is required`);
    }
    this.spacePtr = _space;
    let _materialization = options.materialization ?? null;
    if (_materialization === null) {
      _materialization = 3 /* Materialization.ROOT */;
    }
    if (_materialization === null) {
      throw new Error(`Snapshot.materialization is required`);
    }
    this.materialization = _materialization;
    let _snapshot = options.snapshot ?? null;
    if (_snapshot != null && _snapshot.metatype != StructType.NODE_REFERENCE) {
      _snapshot = (_snapshot as Node).toRef();
    }
    if (_snapshot === null) {
      _snapshot = this.toRef();
    }
    if (_snapshot === null) {
      throw new Error(`Snapshot.snapshot is required`);
    }
    this.snapshotPtr = _snapshot;
    let _precededBy = options.precededBy ?? null;
    if (_precededBy != null && _precededBy.metatype != StructType.NODE_REFERENCE) {
      _precededBy = (_precededBy as Node).toRef();
    }
    this.precededByPtr = _precededBy;
    let _archivedAt = options.archivedAt ?? null;
    this.archivedAt = _archivedAt;
    let _deletedAt = options.deletedAt ?? null;
    this.deletedAt = _deletedAt;
    let _ownedBy = options.ownedBy ?? null;
    if (_ownedBy != null && _ownedBy.metatype != StructType.NODE_REFERENCE) {
      _ownedBy = (_ownedBy as Node).toRef();
    }
    this._ownedByPtr = _ownedBy;
    let _name = options.name ?? null;
    if (_name === null) {
      _name = "Snapshot";
    }
    if (_name === null) {
      throw new Error(`Snapshot.name is required`);
    }
    this._name = _name;
    let _status = options.status ?? null;
    if (_status === null) {
      _status = 10 /* SnapshotStatus.ACTIVE */;
    }
    if (_status === null) {
      throw new Error(`Snapshot.status is required`);
    }
    this._status = _status;

    // identity
    if (options.id == null) {
      const now = Temporal.Now.zonedDateTimeISO("UTC");
      this.createdAt = now;
      this.createdByPtr = null;
      this.updatedAt = now;
      this.updatedByPtr = null;
    } else {
      if (options.createdAt == null || options.updatedAt == null) {
        throw new Error(
          `Snapshot.createdAt and Snapshot.updatedAt are required for existing Nodes`,
        );
      }
      this.createdAt = options.createdAt;
      this.createdByPtr =
        options.createdBy != null
          ? options.createdBy.metatype == StructType.NODE_REFERENCE
            ? (options.createdBy as NodeReference)
            : (options.createdBy as Node).toRef()
          : null;
      this.updatedAt = options.updatedAt;
      this.updatedByPtr =
        options.updatedBy != null
          ? options.updatedBy.metatype == StructType.NODE_REFERENCE
            ? (options.updatedBy as NodeReference)
            : (options.updatedBy as Node).toRef()
          : null;
    }
  }

  equals(other: any): boolean {
    if (!(this.metatype === other.metatype)) {
      return false;
    }
    if (!(this.snapshotPtr.id === other.snapshotPtr.id)) {
      return false;
    }
    if (!(this.precededByPtr?.id === other.precededByPtr?.id)) {
      return false;
    }
    if (!(this._status === other._status)) {
      return false;
    }
    if (!(this._ownedByPtr?.id === other._ownedByPtr?.id)) {
      return false;
    }
    if (!(this._name === other._name)) {
      return false;
    }
    if (!(this.spacePtr.id === other.spacePtr.id)) {
      return false;
    }
    return true;
  }

  hash(): number {
    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    if (this.parentPtr != null) {
      h = (h * 31 + hashString(this.parentPtr.id)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.snapshotPtr.id)) & 0xffffffff;
    if (this.precededByPtr != null) {
      h = (h * 31 + hashString(this.precededByPtr.id)) & 0xffffffff;
    }
    h = (h * 31 + this._status) & 0xffffffff;
    if (this._ownedByPtr != null) {
      h = (h * 31 + hashString(this._ownedByPtr.id)) & 0xffffffff;
    }
    if (this.archivedAt != null) {
      h = (h * 31 + hashString(this.archivedAt.toString({ timeZoneName: "never" }))) & 0xffffffff;
    }
    if (this.deletedAt != null) {
      h = (h * 31 + hashString(this.deletedAt.toString({ timeZoneName: "never" }))) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.createdAt.toString({ timeZoneName: "never" }))) & 0xffffffff;
    if (this.createdByPtr != null) {
      h = (h * 31 + hashString(this.createdByPtr.id)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.updatedAt.toString({ timeZoneName: "never" }))) & 0xffffffff;
    if (this.updatedByPtr != null) {
      h = (h * 31 + hashString(this.updatedByPtr.id)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this._name)) & 0xffffffff;
    h = (h * 31 + hashString(this.id.toString())) & 0xffffffff;
    h = (h * 31 + hashString(this.spacePtr.id)) & 0xffffffff;

    return h;
  }

  validate(): void {
    throw new Error("not implemented");
  }

  __toRef__(): NodeReference {
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    return new _NodeReference({
      type: NodeType.SNAPSHOT,
      id: this.id,
      spaceId: this.spacePtr?.id ?? null,
      snapshotId: this.id,
      _session: this._session,
      _supergraph: this._supergraph,
    });
  }

  get _pathKey(): string {
    return this.name;
  }

  get path(): string {
    const pathParts: string[] = [];
    let node: Entity | Event | null = this;
    let lastNode: Entity | Event | null = this;
    while (node != null) {
      pathParts.push(node._pathKey);
      lastNode = node;
      node = node.parent;
    }
    if (!lastNode.isRoot) {
      pathParts.push("<detached>");
    }
    return pathParts.reverse().join("/");
  }

  repr(): string {
    const propertyReprs: string[] = [];
    if (this.ownedBy != null) {
      propertyReprs.push(`ownedBy=${this.ownedBy?.repr()}`);
    }
    propertyReprs.push(`name=${`"${this.name}"`}`);
    return `<Snapshot "${this.path}" ${propertyReprs.join(" ")}>`;
  }

  toValue(): { readonly [key: string]: any } {
    return Snapshot.__packValue__(this);
  }

  static __packValue__(object: Snapshot): { readonly [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 1300;
    objectValue["2"] = String(object.id);
    if (object.parentPtr != null) {
      objectValue["3"] = object.parentPtr.toValue();
    }
    objectValue["5"] = object.spacePtr.toValue();
    objectValue["10"] = object.materialization;
    objectValue["11"] = object.snapshotPtr.toValue();
    if (object.precededByPtr != null) {
      objectValue["12"] = object.precededByPtr.toValue();
    }
    objectValue["20"] = object.createdAt.toString({ timeZoneName: "never" });
    if (object.createdByPtr != null) {
      objectValue["21"] = object.createdByPtr.toValue();
    }
    objectValue["22"] = object.updatedAt.toString({ timeZoneName: "never" });
    if (object.updatedByPtr != null) {
      objectValue["23"] = object.updatedByPtr.toValue();
    }
    if (object.archivedAt != null) {
      objectValue["24"] = object.archivedAt.toString({ timeZoneName: "never" });
    }
    if (object.deletedAt != null) {
      objectValue["25"] = object.deletedAt.toString({ timeZoneName: "never" });
    }
    if (object._ownedByPtr != null) {
      objectValue["28"] = object._ownedByPtr.toValue();
    }
    objectValue["50"] = object._name;
    objectValue["110"] = object._status;
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { readonly [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Snapshot {
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    const parentPtrValue = objectValue["3"];
    const unpackedParentPtr =
      parentPtrValue != undefined
        ? _NodeReference.fromValue(parentPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const precededByPtrValue = objectValue["12"];
    const unpackedPrecededByPtr =
      precededByPtrValue != undefined
        ? _NodeReference.fromValue(precededByPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const ownedByPtrValue = objectValue["28"];
    const unpackedOwnedByPtr =
      ownedByPtrValue != undefined
        ? _NodeReference.fromValue(ownedByPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const archivedAtValue = objectValue["24"];
    const unpackedArchivedAt =
      archivedAtValue != undefined
        ? Temporal.Instant.from(archivedAtValue).toZonedDateTimeISO("UTC")
        : null;
    const deletedAtValue = objectValue["25"];
    const unpackedDeletedAt =
      deletedAtValue != undefined
        ? Temporal.Instant.from(deletedAtValue).toZonedDateTimeISO("UTC")
        : null;
    const createdByPtrValue = objectValue["21"];
    const unpackedCreatedByPtr =
      createdByPtrValue != undefined
        ? _NodeReference.fromValue(createdByPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const updatedByPtrValue = objectValue["23"];
    const unpackedUpdatedByPtr =
      updatedByPtrValue != undefined
        ? _NodeReference.fromValue(updatedByPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    return new Snapshot({
      parent: unpackedParentPtr,
      snapshot: _NodeReference.fromValue(
        objectValue["11"],
        _session,
        _supergraph,
        _graph,
        _connection,
      ),
      precededBy: unpackedPrecededByPtr,
      status: Number(objectValue["110"]),
      ownedBy: unpackedOwnedByPtr,
      archivedAt: unpackedArchivedAt,
      deletedAt: unpackedDeletedAt,
      materialization: Number(objectValue["10"]),
      createdAt: Temporal.Instant.from(objectValue["20"]).toZonedDateTimeISO("UTC"),
      createdBy: unpackedCreatedByPtr,
      updatedAt: Temporal.Instant.from(objectValue["22"]).toZonedDateTimeISO("UTC"),
      updatedBy: unpackedUpdatedByPtr,
      name: objectValue["50"],
      id: String(objectValue["2"]),
      space: _NodeReference.fromValue(objectValue["5"], _session, _supergraph, _graph, _connection),
      _session,
      _graph,
      _connection,
    });
  }

  static fromValue(
    objectValue: { readonly [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Snapshot {
    return Snapshot.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
  }

  toProto(): SnapshotProto {
    return Snapshot.__packProto__(this);
  }

  static __packProto__(object: Snapshot): SnapshotProto {
    const objectProto: Partial<SnapshotProto> = { metatype: 1300 };
    objectProto.id = String(object.id);
    if (object.parentPtr != null) {
      objectProto.parentPtr = object.parentPtr.toProto();
    }
    objectProto.spacePtr = object.spacePtr.toProto();
    objectProto.materialization = Number(object.materialization) as MaterializationProto;
    objectProto.snapshotPtr = object.snapshotPtr.toProto();
    if (object.precededByPtr != null) {
      objectProto.precededByPtr = object.precededByPtr.toProto();
    }
    objectProto.createdAt = packProtoTimestamp(object.createdAt);
    if (object.createdByPtr != null) {
      objectProto.createdByPtr = object.createdByPtr.toProto();
    }
    objectProto.updatedAt = packProtoTimestamp(object.updatedAt);
    if (object.updatedByPtr != null) {
      objectProto.updatedByPtr = object.updatedByPtr.toProto();
    }
    if (object.archivedAt != null) {
      objectProto.archivedAt = packProtoTimestamp(object.archivedAt);
    }
    if (object.deletedAt != null) {
      objectProto.deletedAt = packProtoTimestamp(object.deletedAt);
    }
    if (object._ownedByPtr != null) {
      objectProto.ownedByPtr = object._ownedByPtr.toProto();
    }
    objectProto.name = object._name;
    objectProto.status = Number(object._status) as SnapshotStatusProto;
    return objectProto as SnapshotProto;
  }

  static __unpackProto__(
    objectProto: SnapshotProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Snapshot {
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    return new Snapshot({
      parent:
        objectProto.parentPtr != undefined
          ? _NodeReference.fromProto(
              objectProto.parentPtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      snapshot: _NodeReference.fromProto(
        objectProto.snapshotPtr!,
        _session,
        _supergraph,
        _graph,
        _connection,
      ),
      precededBy:
        objectProto.precededByPtr != undefined
          ? _NodeReference.fromProto(
              objectProto.precededByPtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      status: Number(objectProto.status) as SnapshotStatus,
      ownedBy:
        objectProto.ownedByPtr != undefined
          ? _NodeReference.fromProto(
              objectProto.ownedByPtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      archivedAt:
        objectProto.archivedAt != undefined ? unpackProtoTimestamp(objectProto.archivedAt!) : null,
      deletedAt:
        objectProto.deletedAt != undefined ? unpackProtoTimestamp(objectProto.deletedAt!) : null,
      materialization: Number(objectProto.materialization) as Materialization,
      createdAt: unpackProtoTimestamp(objectProto.createdAt!),
      createdBy:
        objectProto.createdByPtr != undefined
          ? _NodeReference.fromProto(
              objectProto.createdByPtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      updatedAt: unpackProtoTimestamp(objectProto.updatedAt!),
      updatedBy:
        objectProto.updatedByPtr != undefined
          ? _NodeReference.fromProto(
              objectProto.updatedByPtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      name: objectProto.name,
      id: String(objectProto.id),
      space: _NodeReference.fromProto(
        objectProto.spacePtr!,
        _session,
        _supergraph,
        _graph,
        _connection,
      ),
      _session,
      _graph,
      _connection,
    });
  }

  static fromProto(
    objectProto: SnapshotProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Snapshot {
    return Snapshot.__unpackProto__(objectProto, _session, _supergraph, _graph, _connection);
  }

  static fromProtoString(packedProtoString: string): Snapshot {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = SnapshotProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerNodeClass(NodeType.SNAPSHOT, Snapshot);
/* ==== DESTACK_GENERATED_END:NODE:1300 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:14 ==== */
/**
 * Materialization
 */
export enum Materialization {
  INSTANCE = 1,
  COPY = 2,
  ROOT = 3,

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerEnumClass(EnumType.MATERIALIZATION, Materialization);
/* ==== DESTACK_GENERATED_END:ENUM:14 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:1301 ==== */
/**
 * SnapshotStatus
 */
export enum SnapshotStatus {
  CREATING = 1,
  ACTIVE = 10,
  READONLY = 50,

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerEnumClass(EnumType.SNAPSHOT_STATUS, SnapshotStatus);
/* ==== DESTACK_GENERATED_END:ENUM:1301 ==== */

/* ==== DESTACK_GENERATED_START:NODE:1500 ==== */
/**
 * A Variant is an alternative version of an Entity.
 */
export abstract class Variant extends Entity implements IsExtensible, IsOwnable, IsDeletable {
  static metatype: NodeType = NodeType.VARIANT;

  /**
   * Variant.parent
   */
  abstract get parent(): (Entity & IsExtensible) | null;
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
  abstract get precededBy(): Variant | null;
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
   * IsOwnable.ownedBy
   */
  abstract get ownedBy(): (Entity & IsActor) | null;
  abstract set ownedBy(value: (Entity & IsActor) | null);
  /**
   * IsOwnable.ownedBy
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
   * Variant.icon
   */
  /**
   * Variant.icon
   */
  abstract get icon(): Icon | null;
  abstract set icon(value: Icon | null);

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerNodeClass(NodeType.VARIANT, Variant);
/* ==== DESTACK_GENERATED_END:NODE:1500 ==== */
