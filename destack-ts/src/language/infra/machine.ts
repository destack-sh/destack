import {
  Graph,
  IsSubject,
  MaterializationType,
  Node,
  NodeReference,
  NodeType,
  QueryConnection,
  Resource,
  ResourceStatus,
  Session,
  Spatial,
  StructType,
  Supergraph,
  TraitType,
} from "@destack/language/core";
import { Client, Space } from "@destack/language/space";
import { Temporal } from "temporal-polyfill";

/* ==== DESTACK_GENERATED_START:ENUM:7600 ==== */
/**
 * MachineType
 */
export enum MachineType {
  RUNTIME = 10,
  UBUNTU = 1000,
  MAC = 1100,
  WINDOWS = 1200,
  CUSTOM = 9000,
}
/* ==== DESTACK_GENERATED_END:ENUM:7600 ==== */

/* ==== DESTACK_GENERATED_START:NODE:7600 ==== */
/**
 * A Machine provides physical compute.
 * NOTE :RichComputing: Machines also need Deployments/Endpoints/...?
 */
export class Machine extends Node implements Spatial, Resource {
  static metatype: NodeType = NodeType.MACHINE;
  static __traits__: TraitType[] = [TraitType.TRACKED, TraitType.SPATIAL, TraitType.ENTITY, TraitType.RESOURCE];
  static __rootType__: NodeType | null = NodeType.SPACE;
  static __parentTypes__: NodeType[] = [NodeType.SPACE];
  static __childTypes__: NodeType[] = [];
  static __ancestorTypes__: NodeType[] = [NodeType.SPACE];
  static __descendantTypes__: NodeType[] = [];

  /**
   * Spatial.parent
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
   * Machine.type
   */
  type: MachineType;

  /**
   * Resource.status
   */
  status: ResourceStatus;

  /**
   * Resource.targetStatus
   */
  targetStatus: Temporal.ZonedDateTime | null;

  /**
   * Machine.version
   */
  version: string;

  /**
   * Machine.externalName
   */
  readonly externalName: string | null;

  /**
   * Machine.externalId
   */
  readonly externalId: string | null;

  /**
   * Machine.imageId
   */
  readonly imageId: string | null;

  /**
   * Machine.grpcUrl
   */
  readonly grpcUrl: string | null;

  /**
   * Machine.vncUrl
   */
  readonly vncUrl: string | null;

  /**
   * Machine.client
   */
  get client(): Client | null {
    const nodePtr: NodeReference | null = this.clientPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Client | null;
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
  clientPtr: NodeReference | null;

  /**
   * vCPU count
   */
  readonly cpu: number;

  /**
   * GB
   */
  readonly ram: number;

  /**
   * Machine.width
   */
  readonly width: number;

  /**
   * Machine.height
   */
  readonly height: number;

  /**
   * Machine.isHeadless
   */
  readonly isHeadless: boolean;

  constructor(options: {
    id?: string;
    parent?: Space | NodeReference | null;
    space?: Space | NodeReference | null;
    materialization?: MaterializationType;
    createdAt?: Temporal.ZonedDateTime;
    createdBy?: (Node & IsSubject) | NodeReference | null;
    updatedAt?: Temporal.ZonedDateTime;
    updatedBy?: (Node & IsSubject) | NodeReference | null;
    type?: MachineType;
    status?: ResourceStatus;
    targetStatus?: Temporal.ZonedDateTime | null;
    version?: string;
    externalName?: string | null;
    externalId?: string | null;
    imageId?: string | null;
    grpcUrl?: string | null;
    vncUrl?: string | null;
    client?: Client | NodeReference | null;
    cpu?: number;
    ram?: number;
    width?: number;
    height?: number;
    isHeadless?: boolean;
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
      throw new Error(`Machine.materialization is required`);
    }
    this.materialization = _materialization;
    let _type = options.type ?? null;
    if (_type === null) {
      _type = MachineType.RUNTIME;
    }
    if (_type === null) {
      throw new Error(`Machine.type is required`);
    }
    this.type = _type;
    let _status = options.status ?? null;
    if (_status === null) {
      _status = ResourceStatus.PENDING;
    }
    if (_status === null) {
      throw new Error(`Machine.status is required`);
    }
    this.status = _status;
    let _targetStatus = options.targetStatus ?? null;
    this.targetStatus = _targetStatus;
    let _version = options.version ?? null;
    if (_version === null) {
      _version = "2025.06.23.0";
    }
    if (_version === null) {
      throw new Error(`Machine.version is required`);
    }
    this.version = _version;
    let _externalName = options.externalName ?? null;
    this.externalName = _externalName;
    let _externalId = options.externalId ?? null;
    this.externalId = _externalId;
    let _imageId = options.imageId ?? null;
    this.imageId = _imageId;
    let _grpcUrl = options.grpcUrl ?? null;
    this.grpcUrl = _grpcUrl;
    let _vncUrl = options.vncUrl ?? null;
    this.vncUrl = _vncUrl;
    let _client = options.client ?? null;
    if (_client != null && _client instanceof Node) {
      _client = _client.toRef();
    }
    this.clientPtr = _client;
    let _cpu = options.cpu ?? null;
    if (_cpu === null) {
      _cpu = 1.0;
    }
    if (_cpu === null) {
      throw new Error(`Machine.cpu is required`);
    }
    this.cpu = _cpu;
    let _ram = options.ram ?? null;
    if (_ram === null) {
      _ram = 1.0;
    }
    if (_ram === null) {
      throw new Error(`Machine.ram is required`);
    }
    this.ram = _ram;
    let _width = options.width ?? null;
    if (_width === null) {
      _width = 1280;
    }
    if (_width === null) {
      throw new Error(`Machine.width is required`);
    }
    this.width = _width;
    let _height = options.height ?? null;
    if (_height === null) {
      _height = 960;
    }
    if (_height === null) {
      throw new Error(`Machine.height is required`);
    }
    this.height = _height;
    let _isHeadless = options.isHeadless ?? null;
    if (_isHeadless === null) {
      _isHeadless = false;
    }
    if (_isHeadless === null) {
      throw new Error(`Machine.isHeadless is required`);
    }
    this.isHeadless = _isHeadless;

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
      nodeType: NodeType.MACHINE,
      id: this.id,
      spaceId: this.spacePtr?.id ?? null,
      _session: this._session,
      _supergraph: this._supergraph,
    });
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
