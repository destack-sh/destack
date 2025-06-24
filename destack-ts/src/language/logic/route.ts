import {
  Entity,
  Graph,
  HasName,
  IsDeletable,
  IsOrdered,
  IsOwnable,
  IsOwner,
  IsSubject,
  IsTaggable,
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
import { Folder } from "@destack/language/folder";
import { Scene } from "@destack/language/scene";
import { Space } from "@destack/language/space";
import { Temporal } from "temporal-polyfill";

/* ==== DESTACK_GENERATED_START:NODE:3030 ==== */
/**
 * A Route is a path to a Scene.
 */
export class Route extends Node implements Spatial, Entity, HasName, IsDeletable, IsOrdered, IsOwnable, IsTaggable {
  static metatype: NodeType = NodeType.ROUTE;
  static __traits__: TraitType[] = [
    TraitType.SPATIAL,
    TraitType.TAGGABLE,
    TraitType.ENTITY,
    TraitType.TRACKED,
    TraitType.OWNABLE,
    TraitType.DELETABLE,
    TraitType.ORDERED,
  ];
  static __rootType__: NodeType | null = NodeType.SPACE;
  static __parentTypes__: NodeType[] = [NodeType.FOLDER];
  static __childTypes__: NodeType[] = [NodeType.TAGGING];
  static __ancestorTypes__: NodeType[] = [NodeType.FOLDER, NodeType.SPACE];
  static __descendantTypes__: NodeType[] = [NodeType.TAGGING];

  /**
   * Route.parent
   */
  get parent(): Folder | null {
    const nodePtr: NodeReference | null = this.parentPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Folder | null;
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
   * IsOrdered.orderKey
   */
  readonly orderKey: string;

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
   * The Scene to route to.
   */
  get scene(): Scene | null {
    const nodePtr: NodeReference | null = this.scenePtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Scene | null;
    }
    return null;
  }
  set scene(node: Scene | null) {
    if (node === null) {
      this.scenePtr = null;
    } else {
      this.scenePtr = node.toRef();
    }
  }
  scenePtr: NodeReference | null;

  constructor(options: {
    id?: string;
    parent?: Folder | NodeReference | null;
    space?: Space | NodeReference | null;
    materialization?: MaterializationType;
    createdAt?: Temporal.ZonedDateTime;
    createdBy?: (Node & IsSubject) | NodeReference | null;
    updatedAt?: Temporal.ZonedDateTime;
    updatedBy?: (Node & IsSubject) | NodeReference | null;
    deletedAt?: Temporal.ZonedDateTime | null;
    orderKey?: string;
    ownedBy?: (Node & IsOwner) | NodeReference | null;
    name: string;
    scene?: Scene | NodeReference | null;
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
      throw new Error(`Route.materialization is required`);
    }
    this.materialization = _materialization;
    let _deletedAt = options.deletedAt ?? null;
    this.deletedAt = _deletedAt;
    let _orderKey = options.orderKey ?? null;
    if (_orderKey === null) {
      _orderKey = "a0";
    }
    if (_orderKey === null) {
      throw new Error(`Route.orderKey is required`);
    }
    this.orderKey = _orderKey;
    let _ownedBy = options.ownedBy ?? null;
    if (_ownedBy != null && _ownedBy instanceof Node) {
      _ownedBy = _ownedBy.toRef();
    }
    this.ownedByPtr = _ownedBy;
    let _name = options.name;
    if (_name === null) {
      throw new Error(`Route.name is required`);
    }
    this.name = _name;
    let _scene = options.scene ?? null;
    if (_scene != null && _scene instanceof Node) {
      _scene = _scene.toRef();
    }
    this.scenePtr = _scene;

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
      nodeType: NodeType.ROUTE,
      id: this.id,
      spaceId: this.spacePtr?.id ?? null,
      _session: this._session,
      _supergraph: this._supergraph,
    });
  }

  get _pathKey(): string {
    return this.name;
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
/* ==== DESTACK_GENERATED_END:NODE:3030 ==== */
