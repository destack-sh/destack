import { EnumType, Resource, StructFrozen, Client, Agent, StructType, QueryConnection, NodeType, Supergraph, ResourceStatus, Session, Node, IsTracked, Spatial, Struct, MaterializationType, Graph, User, Entity, BuiltinObject, NodeReference, Space } from '@/language';
import { Temporal } from 'temporal-polyfill'; // until Temporal ships natively

/* ==== DESTACK_GENERATED_START:ENUM:7600 ==== */
export enum MachineType {
  RUNTIME = 10,
  UBUNTU = 1000,
  MAC = 1100,
  WINDOWS = 1200,
  CUSTOM = 9000,
}
/* ==== DESTACK_GENERATED_END:ENUM:7600 ==== */

/* ==== DESTACK_GENERATED_START:NODE:7600 ==== */
export class Machine extends Node implements Spatial, Entity, Resource, IsTracked {
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
  type: MachineType;
  status: ResourceStatus;
  targetStatus: Temporal.ZonedDateTime | null;
  version: string;
  readonly externalName: string | null;
  readonly externalId: string | null;
  readonly imageId: string | null;
  readonly grpcUrl: string | null;
  readonly vncUrl: string | null;
  get client(): Client | null | null {
      const nodePtr: NodeReference | null = this.clientPtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Client | null | null;
      }
      return null;
  }

  set client(value: Client | null) {
      if (value === null) {
          this.clientPtr = null;
      } else {
          this.clientPtr = value.toRef();
      }
  }
  ;
  clientPtr: NodeReference | null
  readonly cpu: number;
  readonly ram: number;
  readonly width: number;
  readonly height: number;
  readonly isHeadless: boolean;

  constructor(
    id: string,
    parentPtr: NodeReference | null,
    spacePtr: NodeReference | null,
    materialization: MaterializationType,
    createdAt: Temporal.ZonedDateTime,
    createdByPtr: NodeReference | null,
    updatedAt: Temporal.ZonedDateTime,
    updatedByPtr: NodeReference | null,
    type: MachineType,
    status: ResourceStatus,
    targetStatus: Temporal.ZonedDateTime | null,
    version: string,
    externalName: string | null,
    externalId: string | null,
    imageId: string | null,
    grpcUrl: string | null,
    vncUrl: string | null,
    clientPtr: NodeReference | null,
    cpu: number,
    ram: number,
    width: number,
    height: number,
    isHeadless: boolean,
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
    this.status = status;
    this.targetStatus = targetStatus;
    this.version = version;
    this.externalName = externalName;
    this.externalId = externalId;
    this.imageId = imageId;
    this.grpcUrl = grpcUrl;
    this.vncUrl = vncUrl;
    this.clientPtr = clientPtr;
    this.cpu = cpu;
    this.ram = ram;
    this.width = width;
    this.height = height;
    this.isHeadless = isHeadless;
  }


  static create(options: {
    type?: MachineType,
    status?: ResourceStatus,
    targetStatus?: Temporal.ZonedDateTime | null,
    version?: string,
    externalName?: string | null,
    externalId?: string | null,
    imageId?: string | null,
    grpcUrl?: string | null,
    vncUrl?: string | null,
    client?: Client | NodeReference | null,
    cpu?: number,
    ram?: number,
    width?: number,
    height?: number,
    isHeadless?: boolean,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: Graph | null,
    _connection?: QueryConnection | null
  }): Machine {
    const session = options._session ?? ACTIVE_SESSION.get();
    const supergraph = options._supergraph ?? session.supergraph;
    return new Machine(
      options.type ?? MachineType.RUNTIME,
      options.status ?? ResourceStatus.PENDING,
      options.targetStatus ?? null,
      options.version ?? "2025.06.19.0",
      options.externalName ?? null,
      options.externalId ?? null,
      options.imageId ?? null,
      options.grpcUrl ?? null,
      options.vncUrl ?? null,
      options.client != null ? (options.client.metatype == StructType.NODE_REFERENCE ? options.client : options.client.toRef()) : null,
      options.cpu ?? 1.0,
      options.ram ?? 1.0,
      options.width ?? 1280,
      options.height ?? 960,
      options.isHeadless ?? false,
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
    return new NodeReference(NodeType.MACHINE, this.id, this.spacePtr?.id ?? null, null, this._supergraph);
  }

  get _pathKey(): string {
      return "Machine[id={this.id}]";
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
/* ==== DESTACK_GENERATED_END:NODE:7600 ==== */