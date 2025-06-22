import { Agent, IsFollowable, BuiltinObject, ACTIVE_SESSION, Organization, Folder, Session, StructFrozen, Graph, Struct, activeSession, Global, StructType, IsStarable, NodeType, Role, Icon, Supergraph, Database, QueryConnection, NodeReference, MaterializationType, IsOwnable, User, Spatial, Team, Region, Handle, Entity, EnumType, IsJoinable, Node, IsTracked } from '@/language';
import { Temporal } from 'temporal-polyfill'; // until Temporal ships natively

/* ==== DESTACK_GENERATED_START:ENUM:1 ==== */
export enum SpaceStatus {
  CREATING = 1,
  QUEUED = 3,
  RUNNING = 10,
  PAUSED = 20,
}
/* ==== DESTACK_GENERATED_END:ENUM:1 ==== */

/* ==== DESTACK_GENERATED_START:NODE:1 ==== */
export class Space extends Node implements Global, Spatial, Entity, IsTracked, IsOwnable, IsJoinable, IsStarable, IsFollowable {
  readonly id: string;
  get parent(): Node | null | null {
      const nodePtr: NodeReference | null = this.parentPtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Node | null | null;
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
  get ownedBy(): Role | Agent | Organization | Team | User | null | null {
      const nodePtr: NodeReference | null = this.ownedByPtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Role | Agent | Organization | Team | User | null | null;
      }
      return null;
  }

  set ownedBy(node: Role | Agent | Organization | Team | User | null) {
      if (node === null) {
          this.ownedByPtr = null;
      } else {
          this.ownedByPtr = node.toRef();
      }
  }
  ;
  ownedByPtr: NodeReference | null
  name: string;
  slug: string;
  icon: Icon | null;
  readonly status: SpaceStatus;
  get handle(): Handle | null | null {
      const nodePtr: NodeReference | null = this.handlePtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Handle | null | null;
      }
      return null;
  }
  ;
  handlePtr: NodeReference | null
  get systemFolder(): Folder | null | null {
      const nodePtr: NodeReference | null = this.systemFolderPtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Folder | null | null;
      }
      return null;
  }
  ;
  systemFolderPtr: NodeReference | null
  get homeFolder(): Folder | null | null {
      const nodePtr: NodeReference | null = this.homeFolderPtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Folder | null | null;
      }
      return null;
  }
  ;
  homeFolderPtr: NodeReference | null
  readonly region: Region;
  readonly galaxyName: string | null;
  get database(): Database | null | null {
      const nodePtr: NodeReference | null = this.databasePtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Database | null | null;
      }
      return null;
  }
  ;
  databasePtr: NodeReference | null

  constructor(options: {
    ownedBy?: Role | Agent | Organization | Team | User | NodeReference | null,
    name: string,
    slug: string,
    icon?: Icon | null,
    status: SpaceStatus,
    handle?: Handle | NodeReference | null,
    systemFolder?: Folder | NodeReference | null,
    homeFolder?: Folder | NodeReference | null,
    region: Region,
    galaxyName?: string | null,
    database?: Database | NodeReference | null,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: Graph | null,
    _connection?: QueryConnection | null
  }) {
    const session = options._session ?? ACTIVE_SESSION.get();
    const supergraph = options._supergraph ?? session.supergraph;
    super(options.id, options.parent, session, supergraph, options._graph, options._connection);
    this.ownedByPtr = options.ownedBy != null ? (options.ownedBy.metatype == StructType.NODE_REFERENCE ? (options.ownedBy as NodeReference) : (options.ownedBy as Node).toRef()) : null;
    this.name = options.name;
    this.slug = options.slug;
    this.icon = options.icon ?? null;
    this.status = options.status;
    this.handlePtr = options.handle != null ? (options.handle.metatype == StructType.NODE_REFERENCE ? (options.handle as NodeReference) : (options.handle as Node).toRef()) : null;
    this.systemFolderPtr = options.systemFolder != null ? (options.systemFolder.metatype == StructType.NODE_REFERENCE ? (options.systemFolder as NodeReference) : (options.systemFolder as Node).toRef()) : null;
    this.homeFolderPtr = options.homeFolder != null ? (options.homeFolder.metatype == StructType.NODE_REFERENCE ? (options.homeFolder as NodeReference) : (options.homeFolder as Node).toRef()) : null;
    this.region = options.region;
    this.galaxyName = options.galaxyName ?? null;
    this.databasePtr = options.database != null ? (options.database.metatype == StructType.NODE_REFERENCE ? (options.database as NodeReference) : (options.database as Node).toRef()) : null;
  }

  equals(other: any): boolean {
    throw new Error("Not implemented");
  }

  hash(): number {
    throw new Error("Not implemented");
  }

  validate(): void {
    throw new Error("Not implemented");
  }

  __toRef__(): NodeReference {
    return new NodeReference(NodeType.SPACE, this.id, this.id, null, this._supergraph);
  }

  get _pathKey(): string {
      return this.slug ?? this.name;
  }

  get path(): string {
      return this.slug ?? this.name;
  }
}
/* ==== DESTACK_GENERATED_END:NODE:1 ==== */