import { Entity, MaterializationType, NodeType, QueryConnection, StructType, Resource, NodeReference, Space, Graph, User, Struct, ResourceStatus, StructFrozen, Spatial, BuiltinObject, EnumType, Tenancy, Agent, activeSession, Node, IsTracked, Supergraph, Region, ACTIVE_SESSION, Session } from '@/language';
import { Temporal } from 'temporal-polyfill';

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

  constructor(options: {
    type: DatabaseType,
    region: Region,
    galaxyName?: string | null,
    externalName: string,
    customSchemaName?: string | null,
    tenancy?: Tenancy,
    connectionUrl?: string | null,
    _session?: Session | null,
    _supergraph?: Supergraph | null
  }) {
    super(
        // supergraph
        options._supergraph ?? null,
    );

    this.type = options.type;
    this.region = options.region;
    this.galaxyName = options.galaxyName ?? null;
    this.externalName = options.externalName;
    this.customSchemaName = options.customSchemaName ?? null;
    this.tenancy = options.tenancy ?? Tenancy.DEDICATED;
    this.connectionUrl = options.connectionUrl ?? null;
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
  readonly parentPtr: NodeReference | null
  get space(): Space | null | null {
      const nodePtr: NodeReference | null = this.spacePtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Space | null | null;
      }
      return null;
  }
  ;
  readonly spacePtr: NodeReference | null
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
  readonly createdByPtr: NodeReference | null
  readonly updatedAt: Temporal.ZonedDateTime;
  get updatedBy(): Agent | User | null | null {
      const nodePtr: NodeReference | null = this.updatedByPtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Agent | User | null | null;
      }
      return null;
  }
  ;
  readonly updatedByPtr: NodeReference | null
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

  constructor(options: {
    id: string,
    parent?: Space | NodeReference | null,
    space?: Space | NodeReference | null,
    materialization?: MaterializationType,
    createdAt: Temporal.ZonedDateTime,
    createdBy?: Agent | User | NodeReference | null,
    updatedAt: Temporal.ZonedDateTime,
    updatedBy?: Agent | User | NodeReference | null,
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
    this.type = options.type;
    this.name = options.name;
    this.status = options.status ?? ResourceStatus.PENDING;
    this.targetStatus = options.targetStatus ?? null;
    this.region = options.region;
    this.galaxyName = options.galaxyName ?? null;
    this.externalName = options.externalName;
    this.customSchemaName = options.customSchemaName ?? null;
    this.tenancy = options.tenancy ?? Tenancy.DEDICATED;
    this.connectionUrl = options.connectionUrl ?? null;
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
      nodeType: NodeType.DATABASE,
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
/* ==== DESTACK_GENERATED_END:NODE:7500 ==== */