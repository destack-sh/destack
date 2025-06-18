import { ResourceStatus, Region, EnumType, StructType, Node, QueryConnection, User, NodeReference, Graph, Spatial, Agent, Space, StructFrozen, Struct, Resource, BuiltinObject, NodeType, Session, MaterializationType, Entity, Tenancy, Supergraph, IsTracked } from '@/language';
import { Temporal } from 'temporal-polyfill'; // until Temporal ships natively

/* ==== DESTACK_GENERATED_START:ENUM:7505 ==== */
export enum DatabaseType {
  POSTGRES = 1,
}
/* ==== DESTACK_GENERATED_END:ENUM:7505 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:7501 ==== */
export class DatabaseInfo extends Struct {
  type: DatabaseType;
  region: Region;
  galaxyName: string | null;
  externalName: string;
  customSchemaName: string | null;
  tenancy: Tenancy;
  connectionUrl: string | null;

  constructor(
    type: DatabaseType,
    region: Region,
    galaxyName: string | null,
    externalName: string,
    customSchemaName: string | null,
    tenancy: Tenancy,
    connectionUrl: string | null,
    _supergraph: Supergraph | null
  ) {
    super(_supergraph);
    this.type = type;
    this.region = region;
    this.galaxyName = galaxyName;
    this.externalName = externalName;
    this.customSchemaName = customSchemaName;
    this.tenancy = tenancy;
    this.connectionUrl = connectionUrl;
  }


  static create(options: {
    type: DatabaseType,
    region: Region,
    galaxyName?: string | null,
    externalName: string,
    customSchemaName?: string | null,
    tenancy?: Tenancy,
    connectionUrl?: string | null,
    _session?: Session | null,
    _supergraph?: Supergraph | null
  }): DatabaseInfo {
    const session = options._session ?? ACTIVE_SESSION.get();
    const supergraph = options._supergraph ?? session.supergraph;
    return new DatabaseInfo(
      options.type,
      options.region,
      options.galaxyName ?? null,
      options.externalName,
      options.customSchemaName ?? null,
      options.tenancy ?? Tenancy.DEDICATED,
      options.connectionUrl ?? null,
      supergraph
    );
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
}
/* ==== DESTACK_GENERATED_END:STRUCT:7501 ==== */

/* ==== DESTACK_GENERATED_START:NODE:7500 ==== */
export class Database extends Node implements Spatial, Entity, Resource, IsTracked {
  readonly id: string;
  get parent(): Space | null | null {
      const nodePtr: NodeReference | null = this.parentPtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Space | null | null;
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
  readonly type: DatabaseType;
  name: string;
  status: ResourceStatus;
  targetStatus: Temporal.ZonedDateTime | null;
  readonly region: Region;
  readonly galaxyName: string | null;
  readonly externalName: string;
  readonly customSchemaName: string | null;
  tenancy: Tenancy;
  readonly connectionUrl: string | null;

  constructor(
    id: string,
    parentPtr: NodeReference | null,
    spacePtr: NodeReference | null,
    materialization: MaterializationType,
    createdAt: Temporal.ZonedDateTime,
    createdByPtr: NodeReference | null,
    updatedAt: Temporal.ZonedDateTime,
    updatedByPtr: NodeReference | null,
    type: DatabaseType,
    name: string,
    status: ResourceStatus,
    targetStatus: Temporal.ZonedDateTime | null,
    region: Region,
    galaxyName: string | null,
    externalName: string,
    customSchemaName: string | null,
    tenancy: Tenancy,
    connectionUrl: string | null,
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
    this.type = type;
    this.name = name;
    this.status = status;
    this.targetStatus = targetStatus;
    this.region = region;
    this.galaxyName = galaxyName;
    this.externalName = externalName;
    this.customSchemaName = customSchemaName;
    this.tenancy = tenancy;
    this.connectionUrl = connectionUrl;
  }


  static create(options: {
    type: DatabaseType,
    name: string,
    status?: ResourceStatus,
    targetStatus?: Temporal.ZonedDateTime | null,
    region: Region,
    galaxyName?: string | null,
    externalName: string,
    customSchemaName?: string | null,
    tenancy?: Tenancy,
    connectionUrl?: string | null,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: Graph | null,
    _connection?: QueryConnection | null
  }): Database {
    const session = options._session ?? ACTIVE_SESSION.get();
    const supergraph = options._supergraph ?? session.supergraph;
    return new Database(
      options.type,
      options.name,
      options.status ?? ResourceStatus.PENDING,
      options.targetStatus ?? null,
      options.region,
      options.galaxyName ?? null,
      options.externalName,
      options.customSchemaName ?? null,
      options.tenancy ?? Tenancy.DEDICATED,
      options.connectionUrl ?? null,
      session,
      supergraph,
      options._graph,
      options._connection
    );
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
    return new NodeReference(NodeType.DATABASE, this.id, this.spacePtr?.id ?? null, null, this._supergraph);
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
/* ==== DESTACK_GENERATED_END:NODE:7500 ==== */