import { Session, Spatial, Entity, Region, Organization, IsJoinable, User, NodeReference, BuiltinObject, IsOwnable, Handle, Agent, MaterializationType, Folder, IsTracked, IsStarable, Graph, Role, IsFollowable, Icon, Database, Struct, Supergraph, Global, QueryConnection, NodeType, Team, Node } from '@/language';
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
  get parent(): Node | null {
      const nodePtr: NodeReference | null = this.parentPtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Node | null;
      }
      return null;
  }
  ;
  parentPtr: NodeReference | null
  get space(): Space | null {
      const nodePtr: NodeReference | null = this.spacePtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Space | null;
      }
      return null;
  }

  set space(value: Space | null) {
      if (value === null) {
          this.spacePtr = null;
      } else {
          this.spacePtr = value.toRef();
      }
  }
  ;
  spacePtr: NodeReference | null
  materialization: MaterializationType;
  readonly createdAt: Temporal.ZonedDateTime;
  get createdBy(): Agent | User | null {
      const nodePtr: NodeReference | null = this.createdByPtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Agent | User | null;
      }
      return null;
  }
  ;
  createdByPtr: NodeReference | null
  readonly updatedAt: Temporal.ZonedDateTime;
  get updatedBy(): Agent | User | null {
      const nodePtr: NodeReference | null = this.updatedByPtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Agent | User | null;
      }
      return null;
  }
  ;
  updatedByPtr: NodeReference | null
  get ownedBy(): Role | Agent | Organization | Team | User | null {
      const nodePtr: NodeReference | null = this.ownedByPtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Role | Agent | Organization | Team | User | null;
      }
      return null;
  }

  set ownedBy(value: Role | Agent | Organization | Team | User | null) {
      if (value === null) {
          this.ownedByPtr = null;
      } else {
          this.ownedByPtr = value.toRef();
      }
  }
  ;
  ownedByPtr: NodeReference | null
  name: string;
  slug: string;
  icon: Icon | null;
  readonly status: SpaceStatus;
  get handle(): Handle | null {
      const nodePtr: NodeReference | null = this.handlePtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Handle | null;
      }
      return null;
  }
  ;
  handlePtr: NodeReference | null
  get systemFolder(): Folder | null {
      const nodePtr: NodeReference | null = this.systemFolderPtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Folder | null;
      }
      return null;
  }
  ;
  systemFolderPtr: NodeReference | null
  get homeFolder(): Folder | null {
      const nodePtr: NodeReference | null = this.homeFolderPtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Folder | null;
      }
      return null;
  }
  ;
  homeFolderPtr: NodeReference | null
  readonly region: Region;
  readonly galaxyName: string | null;
  get database(): Database | null {
      const nodePtr: NodeReference | null = this.databasePtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Database | null;
      }
      return null;
  }
  ;
  databasePtr: NodeReference | null

  constructor(
    id: string,
    parentPtr: NodeReference | null,
    spacePtr: NodeReference | null,
    materialization: MaterializationType,
    createdAt: Temporal.ZonedDateTime,
    createdByPtr: NodeReference | null,
    updatedAt: Temporal.ZonedDateTime,
    updatedByPtr: NodeReference | null,
    ownedByPtr: NodeReference | null,
    name: string,
    slug: string,
    icon: Icon | null,
    status: SpaceStatus,
    handlePtr: NodeReference | null,
    systemFolderPtr: NodeReference | null,
    homeFolderPtr: NodeReference | null,
    region: Region,
    galaxyName: string | null,
    databasePtr: NodeReference | null,
    _session: Session,
    _supergraph: Supergraph,
    _graph: Graph,
    _connection: QueryConnection | null
  ) {
    super(id, _session, _supergraph, _graph, _connection);
    this.id = id;
    this.parentPtr = parentPtr;
    this.spacePtr = spacePtr;
    this.materialization = materialization;
    this.createdAt = createdAt;
    this.createdByPtr = createdByPtr;
    this.updatedAt = updatedAt;
    this.updatedByPtr = updatedByPtr;
    this.ownedByPtr = ownedByPtr;
    this.name = name;
    this.slug = slug;
    this.icon = icon;
    this.status = status;
    this.handlePtr = handlePtr;
    this.systemFolderPtr = systemFolderPtr;
    this.homeFolderPtr = homeFolderPtr;
    this.region = region;
    this.galaxyName = galaxyName;
    this.databasePtr = databasePtr;
  }


  static create(): Space {

    return new Space();
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
      return this.slug or this.name;
  }

  get path(): string {
      return this.slug or this.name;
  }
}
/* ==== DESTACK_GENERATED_END:NODE:1 ==== */