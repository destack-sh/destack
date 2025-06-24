import {
  Graph,
  HasName,
  IsSubject,
  MaterializationType,
  Node,
  NodeReference,
  NodeType,
  QueryConnection,
  Region,
  Resource,
  ResourceStatus,
  Session,
  Spatial,
  Struct,
  StructType,
  Supergraph,
  Tenancy,
  TraitType,
} from "@destack/language/core";
import { Space } from "@destack/language/space";
import { Temporal } from "temporal-polyfill";

/* ==== DESTACK_GENERATED_START:ENUM:7505 ==== */
/**
 * DatabaseType
 */
export enum DatabaseType {
  POSTGRES = 1,
}
/* ==== DESTACK_GENERATED_END:ENUM:7505 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:7501 ==== */
/**
 * DatabaseInfo
 */
export class DatabaseInfo extends Struct {
  static metatype: StructType = StructType.DATABASE_INFO;
  static __isFrozen__: boolean = false;

  /**
   * DatabaseBase.type
   */
  type: DatabaseType;

  /**
   * DatabaseBase.region
   */
  region: Region;

  /**
   * DatabaseBase.galaxyName
   */
  galaxyName: string | null;

  /**
   * DatabaseBase.externalName
   */
  externalName: string;

  /**
   * DatabaseBase.customSchemaName
   */
  customSchemaName: string | null;

  /**
   * DatabaseBase.tenancy
   */
  tenancy: Tenancy;

  /**
   * DatabaseBase.connectionUrl
   */
  connectionUrl: string | null;

  constructor(options: {
    type: DatabaseType;
    region: Region;
    galaxyName?: string | null;
    externalName: string;
    customSchemaName?: string | null;
    tenancy?: Tenancy;
    connectionUrl?: string | null;
    _session?: Session | null;
    _supergraph?: Supergraph | null;
  }) {
    super(
      // session
      options._session ?? null,
      // supergraph
      options._supergraph ?? null,
    );

    // properties
    let _type = options.type;
    if (_type === null) {
      throw new Error(`DatabaseInfo.type is required`);
    }
    this.type = _type;
    let _region = options.region;
    if (_region === null) {
      throw new Error(`DatabaseInfo.region is required`);
    }
    this.region = _region;
    let _galaxyName = options.galaxyName ?? null;
    this.galaxyName = _galaxyName;
    let _externalName = options.externalName;
    if (_externalName === null) {
      throw new Error(`DatabaseInfo.externalName is required`);
    }
    this.externalName = _externalName;
    let _customSchemaName = options.customSchemaName ?? null;
    this.customSchemaName = _customSchemaName;
    let _tenancy = options.tenancy ?? null;
    if (_tenancy === null) {
      _tenancy = Tenancy.DEDICATED;
    }
    if (_tenancy === null) {
      throw new Error(`DatabaseInfo.tenancy is required`);
    }
    this.tenancy = _tenancy;
    let _connectionUrl = options.connectionUrl ?? null;
    this.connectionUrl = _connectionUrl;

    // identity
    // ...
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
/**
 * A primary storage Database of some flavor.
 */
export class Database extends Node implements Spatial, Resource, HasName {
  static metatype: NodeType = NodeType.DATABASE;
  static __traits__: TraitType[] = [TraitType.TRACKED, TraitType.SPATIAL, TraitType.ENTITY, TraitType.RESOURCE];
  static __rootType__: NodeType | null = NodeType.SPACE;
  static __parentTypes__: NodeType[] = [NodeType.SPACE];
  static __childTypes__: NodeType[] = [];
  static __ancestorTypes__: NodeType[] = [NodeType.SPACE];
  static __descendantTypes__: NodeType[] = [];

  /**
   * Database.parent
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
   * DatabaseBase.type
   */
  readonly type: DatabaseType;

  /**
   * HasName.name
   */
  name: string;

  /**
   * Resource.status
   */
  status: ResourceStatus;

  /**
   * Resource.targetStatus
   */
  targetStatus: Temporal.ZonedDateTime | null;

  /**
   * DatabaseBase.region
   */
  readonly region: Region;

  /**
   * DatabaseBase.galaxyName
   */
  readonly galaxyName: string | null;

  /**
   * DatabaseBase.externalName
   */
  readonly externalName: string;

  /**
   * DatabaseBase.customSchemaName
   */
  readonly customSchemaName: string | null;

  /**
   * DatabaseBase.tenancy
   */
  tenancy: Tenancy;

  /**
   * DatabaseBase.connectionUrl
   */
  readonly connectionUrl: string | null;

  constructor(options: {
    id?: string;
    parent?: Space | NodeReference | null;
    space?: Space | NodeReference | null;
    materialization?: MaterializationType;
    createdAt?: Temporal.ZonedDateTime;
    createdBy?: (Node & IsSubject) | NodeReference | null;
    updatedAt?: Temporal.ZonedDateTime;
    updatedBy?: (Node & IsSubject) | NodeReference | null;
    type: DatabaseType;
    name: string;
    status?: ResourceStatus;
    targetStatus?: Temporal.ZonedDateTime | null;
    region: Region;
    galaxyName?: string | null;
    externalName: string;
    customSchemaName?: string | null;
    tenancy?: Tenancy;
    connectionUrl?: string | null;
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
      throw new Error(`Database.materialization is required`);
    }
    this.materialization = _materialization;
    let _type = options.type;
    if (_type === null) {
      throw new Error(`Database.type is required`);
    }
    this.type = _type;
    let _name = options.name;
    if (_name === null) {
      throw new Error(`Database.name is required`);
    }
    this.name = _name;
    let _status = options.status ?? null;
    if (_status === null) {
      _status = ResourceStatus.PENDING;
    }
    if (_status === null) {
      throw new Error(`Database.status is required`);
    }
    this.status = _status;
    let _targetStatus = options.targetStatus ?? null;
    this.targetStatus = _targetStatus;
    let _region = options.region;
    if (_region === null) {
      throw new Error(`Database.region is required`);
    }
    this.region = _region;
    let _galaxyName = options.galaxyName ?? null;
    this.galaxyName = _galaxyName;
    let _externalName = options.externalName;
    if (_externalName === null) {
      throw new Error(`Database.externalName is required`);
    }
    this.externalName = _externalName;
    let _customSchemaName = options.customSchemaName ?? null;
    this.customSchemaName = _customSchemaName;
    let _tenancy = options.tenancy ?? null;
    if (_tenancy === null) {
      _tenancy = Tenancy.DEDICATED;
    }
    if (_tenancy === null) {
      throw new Error(`Database.tenancy is required`);
    }
    this.tenancy = _tenancy;
    let _connectionUrl = options.connectionUrl ?? null;
    this.connectionUrl = _connectionUrl;

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
