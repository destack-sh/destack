import {
  Entity,
  Graph,
  HasName,
  HasSlug,
  IsDeletable,
  IsOwnable,
  IsOwner,
  IsSubject,
  MaterializationType,
  Node,
  NodeReference,
  NodeType,
  QueryConnection,
  Session,
  Spatial,
  StructType,
  Supergraph,
  TraitType,
} from "@destack/language/core";
import { Space } from "@destack/language/space";
import { Temporal } from "temporal-polyfill";

/* ==== DESTACK_GENERATED_START:NODE:1500 ==== */
/**
 * A Snapshot is a point in Space time.
 */
export class Snapshot extends Node implements Spatial, Entity, HasName, HasSlug, IsOwnable, IsDeletable {
  static metatype: NodeType = NodeType.SNAPSHOT;
  static __traits__: TraitType[] = [
    TraitType.SPATIAL,
    TraitType.ENTITY,
    TraitType.TRACKED,
    TraitType.OWNABLE,
    TraitType.DELETABLE,
  ];
  static __rootType__: NodeType | null = NodeType.SPACE;
  static __parentTypes__: NodeType[] = [NodeType.SPACE, NodeType.BRANCH];
  static __childTypes__: NodeType[] = [];
  static __ancestorTypes__: NodeType[] = [NodeType.SPACE, NodeType.BRANCH];
  static __descendantTypes__: NodeType[] = [];

  /**
   * Snapshot.parent
   */
  get parent(): Space | Branch | null {
    const nodePtr: NodeReference | null = this.parentPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Space | Branch | null;
    }
    return null;
  }
  readonly parentPtr: NodeReference | null;

  /**
   * The Space this Node is in.
   */
  get space(): Space | null {
    const nodePtr: NodeReference | null = this.spacePtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Space | null;
    }
    return null;
  }
  readonly spacePtr: NodeReference | null;

  /**
   * Entity.materialization
   */
  readonly materialization: MaterializationType;

  /**
   * IsTracked.createdAt
   */
  readonly createdAt: Temporal.ZonedDateTime;

  /**
   * IsTracked.createdBy
   */
  get createdBy(): (Node & IsSubject) | null {
    const nodePtr: NodeReference | null = this.createdByPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as (Node & IsSubject) | null;
    }
    return null;
  }
  readonly createdByPtr: NodeReference | null;

  /**
   * IsTracked.updatedAt
   */
  readonly updatedAt: Temporal.ZonedDateTime;

  /**
   * IsTracked.updatedBy
   */
  get updatedBy(): (Node & IsSubject) | null {
    const nodePtr: NodeReference | null = this.updatedByPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as (Node & IsSubject) | null;
    }
    return null;
  }
  readonly updatedByPtr: NodeReference | null;

  /**
   * IsDeletable.deletedAt
   */
  readonly deletedAt: Temporal.ZonedDateTime | null;

  /**
   * IsOwnable.ownedBy
   */
  get ownedBy(): (Node & IsOwner) | null {
    const nodePtr: NodeReference | null = this.ownedByPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as (Node & IsOwner) | null;
    }
    return null;
  }
  set ownedBy(node: (Node & IsOwner) | null) {
    if (node === null) {
      this.ownedByPtr = null;
    } else {
      this.ownedByPtr = node.toRef();
    }
  }
  ownedByPtr: NodeReference | null;

  /**
   * HasName.name
   */
  name: string;

  /**
   * HasSlug.slug
   */
  slug: string | null;

  constructor(options: {
    id?: string;
    parent?: Space | Branch | NodeReference | null;
    space?: Space | NodeReference | null;
    materialization?: MaterializationType;
    createdAt?: Temporal.ZonedDateTime;
    createdBy?: (Node & IsSubject) | NodeReference | null;
    updatedAt?: Temporal.ZonedDateTime;
    updatedBy?: (Node & IsSubject) | NodeReference | null;
    deletedAt?: Temporal.ZonedDateTime | null;
    ownedBy?: (Node & IsOwner) | NodeReference | null;
    name: string;
    slug?: string | null;
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
      // is_attached
      options.id != null || options._graph != null,
    );

    // properties
    let _parent = options.parent ?? null;
    if (_parent != null && _parent instanceof Node) {
      _parent = _parent.toRef();
    }
    this.parentPtr = _parent;
    let _space = options.space ?? null;
    if (_space != null && _space instanceof Node) {
      _space = _space.toRef();
    }
    this.spacePtr = _space;
    let _materialization = options.materialization ?? null;
    if (_materialization === null) {
      _materialization = MaterializationType.FULL_GRAPH;
    }
    if (_materialization === null) {
      throw new Error(`Snapshot.materialization is required`);
    }
    this.materialization = _materialization;
    let _deletedAt = options.deletedAt ?? null;
    this.deletedAt = _deletedAt;
    let _ownedBy = options.ownedBy ?? null;
    if (_ownedBy != null && _ownedBy instanceof Node) {
      _ownedBy = _ownedBy.toRef();
    }
    this.ownedByPtr = _ownedBy;
    let _name = options.name;
    if (_name === null) {
      throw new Error(`Snapshot.name is required`);
    }
    this.name = _name;
    let _slug = options.slug ?? null;
    this.slug = _slug;

    // identity
    if (options.id == null) {
      const now = Temporal.Now.zonedDateTimeISO();
      this.createdAt = now;
      this.createdByPtr = null;
      this.updatedAt = now;
      this.updatedByPtr = null;
    } else {
      if (options.createdAt == null || options.updatedAt == null) {
        throw new Error(`{cls.__name__}.createdAt and {cls.__name__}.updatedAt are required for existing Nodes`);
      }
      this.createdAt = options.createdAt;
      this.createdByPtr =
        options.createdBy != null
          ? options.createdBy instanceof Node
            ? options.createdBy.toRef()
            : options.createdBy
          : null;
      this.updatedAt = options.updatedAt;
      this.updatedByPtr =
        options.updatedBy != null
          ? options.updatedBy instanceof Node
            ? options.updatedBy.toRef()
            : options.updatedBy
          : null;
    }
  }

  equals(other: any): boolean {
    throw new Error("not implemented");
  }

  hash(): number {
    throw new Error("not implemented");
  }

  validate(): void {
    throw new Error("not implemented");
  }

  __toRef__(): NodeReference {
    return new NodeReference({
      nodeType: NodeType.SNAPSHOT,
      id: this.id,
      spaceId: this.spacePtr?.id ?? null,
      _session: this._session,
      _supergraph: this._supergraph,
    });
  }

  get _pathKey(): string {
    return this.slug ?? this.name;
  }

  get path(): string {
    const pathParts: string[] = [];
    let node: Node | null = this;
    while (node !== null) {
      pathParts.push(node._pathKey);
      node = node.parent;
    }
    if (!this._isAttached) {
      pathParts.push("<detached>");
    }
    return pathParts.reverse().join("/");
  }
}
/* ==== DESTACK_GENERATED_END:NODE:1500 ==== */

/* ==== DESTACK_GENERATED_START:NODE:1510 ==== */
/**
 * A Branch is a version of a Snapshot.
 */
export class Branch extends Node implements Spatial, Entity, HasName, HasSlug, IsOwnable, IsDeletable {
  static metatype: NodeType = NodeType.BRANCH;
  static __traits__: TraitType[] = [
    TraitType.SPATIAL,
    TraitType.ENTITY,
    TraitType.TRACKED,
    TraitType.OWNABLE,
    TraitType.DELETABLE,
  ];
  static __rootType__: NodeType | null = NodeType.SPACE;
  static __parentTypes__: NodeType[] = [NodeType.SPACE];
  static __childTypes__: NodeType[] = [NodeType.SNAPSHOT];
  static __ancestorTypes__: NodeType[] = [NodeType.SPACE];
  static __descendantTypes__: NodeType[] = [NodeType.SNAPSHOT];

  /**
   * Branch.parent
   */
  get parent(): Space | null {
    const nodePtr: NodeReference | null = this.parentPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Space | null;
    }
    return null;
  }
  readonly parentPtr: NodeReference | null;

  /**
   * The Space this Node is in.
   */
  get space(): Space | null {
    const nodePtr: NodeReference | null = this.spacePtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Space | null;
    }
    return null;
  }
  readonly spacePtr: NodeReference | null;

  /**
   * Entity.materialization
   */
  readonly materialization: MaterializationType;

  /**
   * IsTracked.createdAt
   */
  readonly createdAt: Temporal.ZonedDateTime;

  /**
   * IsTracked.createdBy
   */
  get createdBy(): (Node & IsSubject) | null {
    const nodePtr: NodeReference | null = this.createdByPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as (Node & IsSubject) | null;
    }
    return null;
  }
  readonly createdByPtr: NodeReference | null;

  /**
   * IsTracked.updatedAt
   */
  readonly updatedAt: Temporal.ZonedDateTime;

  /**
   * IsTracked.updatedBy
   */
  get updatedBy(): (Node & IsSubject) | null {
    const nodePtr: NodeReference | null = this.updatedByPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as (Node & IsSubject) | null;
    }
    return null;
  }
  readonly updatedByPtr: NodeReference | null;

  /**
   * IsDeletable.deletedAt
   */
  readonly deletedAt: Temporal.ZonedDateTime | null;

  /**
   * IsOwnable.ownedBy
   */
  get ownedBy(): (Node & IsOwner) | null {
    const nodePtr: NodeReference | null = this.ownedByPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as (Node & IsOwner) | null;
    }
    return null;
  }
  set ownedBy(node: (Node & IsOwner) | null) {
    if (node === null) {
      this.ownedByPtr = null;
    } else {
      this.ownedByPtr = node.toRef();
    }
  }
  ownedByPtr: NodeReference | null;

  /**
   * HasName.name
   */
  name: string;

  /**
   * HasSlug.slug
   */
  slug: string | null;

  /**
   * Branch.head
   */
  get head(): Snapshot | null {
    const nodePtr: NodeReference | null = this.headPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Snapshot | null;
    }
    return null;
  }
  set head(node: Snapshot | null) {
    if (node === null) {
      this.headPtr = null;
    } else {
      this.headPtr = node.toRef();
    }
  }
  headPtr: NodeReference | null;

  constructor(options: {
    id?: string;
    parent?: Space | NodeReference | null;
    space?: Space | NodeReference | null;
    materialization?: MaterializationType;
    createdAt?: Temporal.ZonedDateTime;
    createdBy?: (Node & IsSubject) | NodeReference | null;
    updatedAt?: Temporal.ZonedDateTime;
    updatedBy?: (Node & IsSubject) | NodeReference | null;
    deletedAt?: Temporal.ZonedDateTime | null;
    ownedBy?: (Node & IsOwner) | NodeReference | null;
    name: string;
    slug?: string | null;
    head?: Snapshot | NodeReference | null;
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
      // is_attached
      options.id != null || options._graph != null,
    );

    // properties
    let _parent = options.parent ?? null;
    if (_parent != null && _parent instanceof Node) {
      _parent = _parent.toRef();
    }
    this.parentPtr = _parent;
    let _space = options.space ?? null;
    if (_space != null && _space instanceof Node) {
      _space = _space.toRef();
    }
    this.spacePtr = _space;
    let _materialization = options.materialization ?? null;
    if (_materialization === null) {
      _materialization = MaterializationType.FULL_GRAPH;
    }
    if (_materialization === null) {
      throw new Error(`Branch.materialization is required`);
    }
    this.materialization = _materialization;
    let _deletedAt = options.deletedAt ?? null;
    this.deletedAt = _deletedAt;
    let _ownedBy = options.ownedBy ?? null;
    if (_ownedBy != null && _ownedBy instanceof Node) {
      _ownedBy = _ownedBy.toRef();
    }
    this.ownedByPtr = _ownedBy;
    let _name = options.name;
    if (_name === null) {
      throw new Error(`Branch.name is required`);
    }
    this.name = _name;
    let _slug = options.slug ?? null;
    this.slug = _slug;
    let _head = options.head ?? null;
    if (_head != null && _head instanceof Node) {
      _head = _head.toRef();
    }
    this.headPtr = _head;

    // identity
    if (options.id == null) {
      const now = Temporal.Now.zonedDateTimeISO();
      this.createdAt = now;
      this.createdByPtr = null;
      this.updatedAt = now;
      this.updatedByPtr = null;
    } else {
      if (options.createdAt == null || options.updatedAt == null) {
        throw new Error(`{cls.__name__}.createdAt and {cls.__name__}.updatedAt are required for existing Nodes`);
      }
      this.createdAt = options.createdAt;
      this.createdByPtr =
        options.createdBy != null
          ? options.createdBy instanceof Node
            ? options.createdBy.toRef()
            : options.createdBy
          : null;
      this.updatedAt = options.updatedAt;
      this.updatedByPtr =
        options.updatedBy != null
          ? options.updatedBy instanceof Node
            ? options.updatedBy.toRef()
            : options.updatedBy
          : null;
    }
  }

  equals(other: any): boolean {
    throw new Error("not implemented");
  }

  hash(): number {
    throw new Error("not implemented");
  }

  validate(): void {
    throw new Error("not implemented");
  }

  __toRef__(): NodeReference {
    return new NodeReference({
      nodeType: NodeType.BRANCH,
      id: this.id,
      spaceId: this.spacePtr?.id ?? null,
      _session: this._session,
      _supergraph: this._supergraph,
    });
  }

  get _pathKey(): string {
    return this.slug ?? this.name;
  }

  get path(): string {
    const pathParts: string[] = [];
    let node: Node | null = this;
    while (node !== null) {
      pathParts.push(node._pathKey);
      node = node.parent;
    }
    if (!this._isAttached) {
      pathParts.push("<detached>");
    }
    return pathParts.reverse().join("/");
  }
}
/* ==== DESTACK_GENERATED_END:NODE:1510 ==== */
