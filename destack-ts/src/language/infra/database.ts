import { Agent, BuiltinObject, ACTIVE_SESSION, Session, StructFrozen, Graph, ResourceStatus, Struct, activeSession, StructType, NodeType, Supergraph, QueryConnection, NodeReference, MaterializationType, Tenancy, User, Spatial, Region, Space, Resource, Entity, EnumType, Node, IsTracked } from '@/language';
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
    const session = options._session ?? ACTIVE_SESSION.get();
    const supergraph = options._supergraph ?? session.supergraph;
    super(supergraph);
    this.type = options.type;
    this.region = options.region;
    this.galaxyName = options.galaxyName ?? null;
    this.externalName = options.externalName;
    this.customSchemaName = options.customSchemaName ?? null;
    this.tenancy = options.tenancy ?? Tenancy.DEDICATED;
    this.connectionUrl = options.connectionUrl ?? null;
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

  constructor(options: {
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
    const session = options._session ?? ACTIVE_SESSION.get();
    const supergraph = options._supergraph ?? session.supergraph;
    super(options.id, options.parent, session, supergraph, options._graph, options._connection);
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