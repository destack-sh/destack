import { Agent, BuiltinObject, ACTIVE_SESSION, Session, StructFrozen, Graph, ResourceStatus, Struct, activeSession, StructType, NodeType, Supergraph, QueryConnection, NodeReference, MaterializationType, User, Spatial, Space, Resource, Entity, EnumType, Client, Node, IsTracked } from '@/language';
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

  set client(node: Client | null) {
      if (node === null) {
          this.clientPtr = null;
      } else {
          this.clientPtr = node.toRef();
      }
  }
  ;
  clientPtr: NodeReference | null
  readonly cpu: number;
  readonly ram: number;
  readonly width: number;
  readonly height: number;
  readonly isHeadless: boolean;

  constructor(options: {
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
  }) {
    const session = options._session ?? ACTIVE_SESSION.get();
    const supergraph = options._supergraph ?? session.supergraph;
    super(options.id, options.parent, session, supergraph, options._graph, options._connection);
    this.type = options.type ?? MachineType.RUNTIME;
    this.status = options.status ?? ResourceStatus.PENDING;
    this.targetStatus = options.targetStatus ?? null;
    this.version = options.version ?? "2025.06.22.0";
    this.externalName = options.externalName ?? null;
    this.externalId = options.externalId ?? null;
    this.imageId = options.imageId ?? null;
    this.grpcUrl = options.grpcUrl ?? null;
    this.vncUrl = options.vncUrl ?? null;
    this.clientPtr = options.client != null ? (options.client.metatype == StructType.NODE_REFERENCE ? (options.client as NodeReference) : (options.client as Node).toRef()) : null;
    this.cpu = options.cpu ?? 1.0;
    this.ram = options.ram ?? 1.0;
    this.width = options.width ?? 1280;
    this.height = options.height ?? 960;
    this.isHeadless = options.isHeadless ?? false;
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