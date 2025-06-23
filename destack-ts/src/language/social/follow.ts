import { IsTracked, ACTIVE_SESSION, IsOwnable, Spatial, Global, EnumType, MaterializationType, Entity, Space, StructType, Struct, NodeReference, NodeType, Graph, User, Agent, Node, QueryConnection, StructFrozen, LikeFollow, Supergraph, IsDeletable, Folder, BuiltinObject, Session, activeSession } from '@/language';
import { Temporal } from 'temporal-polyfill'; // until Temporal ships natively

/* ==== DESTACK_GENERATED_START:NODE:5530 ==== */
export class Follow extends Node implements Global, Spatial, Entity, IsTracked, IsDeletable, IsOwnable, LikeFollow {
  readonly id: string;
  get parent(): Folder | Agent | Space | User | null | null {
      const nodePtr: NodeReference | null = this.parentPtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Folder | Agent | Space | User | null | null;
      }
      return null;
  }
  ;
  parentPtr: NodeReference | null
  get space(): Space | null | null {
      const nodePtr: NodeReference | null = this.spacePtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Space | null | null;
      }
      return null;
  }
  ;
  spacePtr: NodeReference | null
  readonly materialization: MaterializationType;
  readonly createdAt: Temporal.ZonedDateTime;
  get createdBy(): Agent | User | null | null {
      const nodePtr: NodeReference | null = this.createdByPtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Agent | User | null | null;
      }
      return null;
  }
  ;
  createdByPtr: NodeReference | null
  readonly updatedAt: Temporal.ZonedDateTime;
  get updatedBy(): Agent | User | null | null {
      const nodePtr: NodeReference | null = this.updatedByPtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Agent | User | null | null;
      }
      return null;
  }
  ;
  updatedByPtr: NodeReference | null
  readonly deletedAt: Temporal.ZonedDateTime | null;
  get ownedBy(): Agent | User | null {
      const nodePtr: NodeReference | null = this.ownedByPtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Agent | User | null;
      }
      return null;
  }

  set ownedBy(node: Agent | User) {
      this.ownedByPtr = node.toRef();
  }
  ;
  ownedByPtr: NodeReference

  constructor(options: {
    id: string,
    parent?: Folder | Agent | Space | User | NodeReference | null,
    space?: Space | NodeReference | null,
    materialization?: MaterializationType,
    createdAt: Temporal.ZonedDateTime,
    createdBy?: Agent | User | NodeReference | null,
    updatedAt: Temporal.ZonedDateTime,
    updatedBy?: Agent | User | NodeReference | null,
    deletedAt?: Temporal.ZonedDateTime | null,
    ownedBy: Agent | User | NodeReference,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: Graph | null,
    _connection?: QueryConnection | null
  }) {
    super(
        // id
        options.id,
        // parent
        options.parent != null ? (options.parent.metatype == StructType.NODE_REFERENCE ? (options.parent as NodeReference) : (options.parent as Node).toRef()) : null,
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
        options.id != null,
    );

    this.id = options.id;
    this.parentPtr = options.parent != null ? (options.parent.metatype == StructType.NODE_REFERENCE ? (options.parent as NodeReference) : (options.parent as Node).toRef()) : null;
    this.spacePtr = options.space != null ? (options.space.metatype == StructType.NODE_REFERENCE ? (options.space as NodeReference) : (options.space as Node).toRef()) : null;
    this.materialization = options.materialization ?? MaterializationType.FULL_GRAPH;
    this.createdAt = options.createdAt;
    this.createdByPtr = options.createdBy != null ? (options.createdBy.metatype == StructType.NODE_REFERENCE ? (options.createdBy as NodeReference) : (options.createdBy as Node).toRef()) : null;
    this.updatedAt = options.updatedAt;
    this.updatedByPtr = options.updatedBy != null ? (options.updatedBy.metatype == StructType.NODE_REFERENCE ? (options.updatedBy as NodeReference) : (options.updatedBy as Node).toRef()) : null;
    this.deletedAt = options.deletedAt ?? null;
    this.ownedByPtr = options.ownedBy != null ? (options.ownedBy.metatype == StructType.NODE_REFERENCE ? (options.ownedBy as NodeReference) : (options.ownedBy as Node).toRef()) : null;
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
    return new NodeReference(NodeType.FOLLOW, this.id, this.spacePtr?.id ?? null, null, this._supergraph);
  }

  get _pathKey(): string {
      return "Follow[id={this.id}]";
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
/* ==== DESTACK_GENERATED_END:NODE:5530 ==== */