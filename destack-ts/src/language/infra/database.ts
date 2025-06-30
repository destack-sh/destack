import { packProtoTimestamp, unpackProtoTimestamp } from "@destack/grpc";
import { Graph, NodeReference, QueryConnection, Session, Supergraph } from "@destack/language/core";
import {
  EnumType,
  HasName,
  IsSpatial,
  IsSubject,
  Node,
  NodeType,
  Region,
  ResourceStatus,
  StructFrozen,
  StructType,
  Tenancy,
} from "@destack/language/core/builtin";
import { Resource } from "@destack/language/core/common";
import {
  registerEnumClass,
  registerNodeClass,
  registerStructClass,
} from "@destack/language/registry";
import { Space } from "@destack/language/space";
import {
  DatabaseInfoProto,
  DatabaseProto,
  DatabaseTypeProto,
  RegionProto,
  ResourceStatusProto,
  TenancyProto,
} from "@destack/proto";
import { base64Decode } from "@destack/utils";
import { hashString } from "@destack/utils/hash";
import { Temporal } from "temporal-polyfill";

/* ==== DESTACK_GENERATED_START:ENUM:7505 ==== */
/**
 * DatabaseType
 */
export enum DatabaseType {
  POSTGRES = 1,

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerEnumClass(EnumType.DATABASE_TYPE, DatabaseType);
/* ==== DESTACK_GENERATED_END:ENUM:7505 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:7501 ==== */
/**
 * DatabaseInfo
 */
export class DatabaseInfo extends StructFrozen {
  static metatype: StructType = StructType.DATABASE_INFO;
  static __isFrozen__: boolean = true;

  /**
   * DatabaseInfo.type
   */
  readonly type: DatabaseType;

  /**
   * DatabaseInfo.region
   */
  readonly region: Region;

  /**
   * DatabaseInfo.galaxyName
   */
  readonly galaxyName: string | null;

  /**
   * DatabaseInfo.externalName
   */
  readonly externalName: string;

  /**
   * DatabaseInfo.customSchemaName
   */
  readonly customSchemaName: string | null;

  /**
   * DatabaseInfo.tenancy
   */
  readonly tenancy: Tenancy;

  /**
   * DatabaseInfo.connectionUrl
   */
  readonly connectionUrl: string | null;

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
    _hash?: number | null;
    _repr?: string | null;
    _proto?: any | null;
    _value?: { [key: string]: any } | null;
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
    // @ts-expect-error(readonly)
    this._hash = options._hash ?? null;
    // @ts-expect-error(readonly)
    this._repr = options._repr ?? null;
    // @ts-expect-error(readonly)
    this._proto = options._proto ?? null;
    // @ts-expect-error(readonly)
    this._value = options._value ?? null;
  }

  equals(other: any): boolean {
    if (!(this.metatype === other.metatype)) {
      return false;
    }
    if (!(this.type === other.type)) {
      return false;
    }
    if (!(this.region === other.region)) {
      return false;
    }
    if (!(this.galaxyName === other.galaxyName)) {
      return false;
    }
    if (!(this.externalName === other.externalName)) {
      return false;
    }
    if (!(this.customSchemaName === other.customSchemaName)) {
      return false;
    }
    if (!(this.tenancy === other.tenancy)) {
      return false;
    }
    if (!(this.connectionUrl === other.connectionUrl)) {
      return false;
    }
    return true;
  }

  repr(): string {
    if (this._repr === null) {
      const propertyReprs: string[] = [];
      propertyReprs.push(`type=${DatabaseType[this.type]}`);
      propertyReprs.push(`region=${Region[this.region]}`);
      if (this.galaxyName !== null) {
        propertyReprs.push(`galaxyName=${this.galaxyName}`);
      }
      propertyReprs.push(`externalName=${this.externalName}`);
      if (this.customSchemaName !== null) {
        propertyReprs.push(`customSchemaName=${this.customSchemaName}`);
      }
      propertyReprs.push(`tenancy=${Tenancy[this.tenancy]}`);
      // @ts-expect-error(readonly)
      this._repr = `<DatabaseInfo ${propertyReprs.join(" ")}>`;
    }
    return this._repr;
  }

  hash(): number {
    if (this._hash !== null) {
      return this._hash;
    }

    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    h = (h * 31 + this.type) & 0xffffffff;
    h = (h * 31 + this.region) & 0xffffffff;
    if (this.galaxyName !== null) {
      h = (h * 31 + hashString(this.galaxyName)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.externalName)) & 0xffffffff;
    if (this.customSchemaName !== null) {
      h = (h * 31 + hashString(this.customSchemaName)) & 0xffffffff;
    }
    h = (h * 31 + this.tenancy) & 0xffffffff;
    if (this.connectionUrl !== null) {
      h = (h * 31 + hashString(this.connectionUrl)) & 0xffffffff;
    }

    // @ts-expect-error(readonly)
    this._hash = h;
    return h;
  }

  validate(): void {
    throw new Error("not implemented");
  }

  toValue(): { [key: string]: any } {
    if (this._value === null) {
      // @ts-expect-error(readonly)
      this._value = DatabaseInfo.__packValue__(this);
    }
    return this._value;
  }

  static __packValue__(object: DatabaseInfo): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 7501;
    objectValue["30"] = object.type;
    objectValue["50"] = object.region;
    if (object.galaxyName != null) {
      objectValue["51"] = object.galaxyName;
    }
    objectValue["52"] = object.externalName;
    if (object.customSchemaName != null) {
      objectValue["53"] = object.customSchemaName;
    }
    objectValue["55"] = object.tenancy;
    if (object.connectionUrl != null) {
      objectValue["58"] = object.connectionUrl;
    }
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): DatabaseInfo {
    const galaxyNameValue = objectValue["51"];
    const unpackedGalaxyName = galaxyNameValue != undefined ? galaxyNameValue : null;
    const customSchemaNameValue = objectValue["53"];
    const unpackedCustomSchemaName =
      customSchemaNameValue != undefined ? customSchemaNameValue : null;
    const connectionUrlValue = objectValue["58"];
    const unpackedConnectionUrl = connectionUrlValue != undefined ? connectionUrlValue : null;
    return new DatabaseInfo({
      type: Number(objectValue["30"]),
      region: Number(objectValue["50"]),
      galaxyName: unpackedGalaxyName,
      externalName: objectValue["52"],
      customSchemaName: unpackedCustomSchemaName,
      tenancy: Number(objectValue["55"]),
      connectionUrl: unpackedConnectionUrl,
      _value: objectValue,
      _supergraph,
    });
  }

  static fromValue(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): DatabaseInfo {
    return DatabaseInfo.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
  }

  toProto(): DatabaseInfoProto {
    if (this._proto === null) {
      // @ts-expect-error(readonly)
      this._proto = DatabaseInfo.__packProto__(this);
    }
    return this._proto as DatabaseInfoProto;
  }

  static __packProto__(object: DatabaseInfo): DatabaseInfoProto {
    const objectProto: Partial<DatabaseInfoProto> = { metatype: 7501 };
    objectProto.type = Number(object.type) as DatabaseTypeProto;
    objectProto.region = Number(object.region) as RegionProto;
    if (object.galaxyName != null) {
      objectProto.galaxyName = object.galaxyName;
    }
    objectProto.externalName = object.externalName;
    if (object.customSchemaName != null) {
      objectProto.customSchemaName = object.customSchemaName;
    }
    objectProto.tenancy = Number(object.tenancy) as TenancyProto;
    if (object.connectionUrl != null) {
      objectProto.connectionUrl = object.connectionUrl;
    }
    return objectProto as DatabaseInfoProto;
  }

  static __unpackProto__(
    objectProto: DatabaseInfoProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): DatabaseInfo {
    return new DatabaseInfo({
      type: Number(objectProto.type) as DatabaseType,
      region: Number(objectProto.region) as Region,
      galaxyName: objectProto.galaxyName != undefined ? objectProto.galaxyName : null,
      externalName: objectProto.externalName,
      customSchemaName:
        objectProto.customSchemaName != undefined ? objectProto.customSchemaName : null,
      tenancy: Number(objectProto.tenancy) as Tenancy,
      connectionUrl: objectProto.connectionUrl != undefined ? objectProto.connectionUrl : null,
      _proto: objectProto,
      _supergraph,
    });
  }

  static fromProto(
    objectProto: DatabaseInfoProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): DatabaseInfo {
    return DatabaseInfo.__unpackProto__(objectProto, _session, _supergraph, _graph, _connection);
  }

  static fromProtoString(packedProtoString: string): DatabaseInfo {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = DatabaseInfoProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerStructClass(StructType.DATABASE_INFO, DatabaseInfo);
/* ==== DESTACK_GENERATED_END:STRUCT:7501 ==== */

/* ==== DESTACK_GENERATED_START:NODE:7500 ==== */
/**
 * A primary storage Database of some flavor.
 */
export class Database extends Resource implements IsSpatial, HasName {
  static metatype: NodeType = NodeType.DATABASE;

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
   * Entity.createdAt
   */
  readonly createdAt: Temporal.ZonedDateTime;

  /**
   * Entity.createdBy
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
   * Entity.updatedAt
   */
  readonly updatedAt: Temporal.ZonedDateTime;

  /**
   * Entity.updatedBy
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
   * IsDeletable.deletedAt
   */
  readonly deletedAt: Temporal.ZonedDateTime | null;

  /**
   * Database.type
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
   * Database.region
   */
  readonly region: Region;

  /**
   * Database.galaxyName
   */
  readonly galaxyName: string | null;

  /**
   * Database.externalName
   */
  readonly externalName: string;

  /**
   * Database.customSchemaName
   */
  readonly customSchemaName: string | null;

  /**
   * Database.tenancy
   */
  tenancy: Tenancy;

  /**
   * Database.connectionUrl
   */
  readonly connectionUrl: string | null;

  constructor(options: {
    id?: string;
    parent?: Space | NodeReference | null;
    space?: Space | NodeReference | null;
    createdAt?: Temporal.ZonedDateTime;
    createdBy?: (Node & IsSubject) | NodeReference | null;
    updatedAt?: Temporal.ZonedDateTime;
    updatedBy?: (Node & IsSubject) | NodeReference | null;
    deletedAt?: Temporal.ZonedDateTime | null;
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
    let _deletedAt = options.deletedAt ?? null;
    this.deletedAt = _deletedAt;
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
      const now = Temporal.Now.zonedDateTimeISO("UTC");
      this.createdAt = now;
      this.createdByPtr = null;
      this.updatedAt = now;
      this.updatedByPtr = null;
    } else {
      if (options.createdAt == null || options.updatedAt == null) {
        throw new Error(
          `{cls.__name__}.createdAt and {cls.__name__}.updatedAt are required for existing Nodes`,
        );
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
    if (!(this.metatype === other.metatype)) {
      return false;
    }
    if (!(this.type === other.type)) {
      return false;
    }
    if (!(this.region === other.region)) {
      return false;
    }
    if (!(this.galaxyName === other.galaxyName)) {
      return false;
    }
    if (!(this.externalName === other.externalName)) {
      return false;
    }
    if (!(this.customSchemaName === other.customSchemaName)) {
      return false;
    }
    if (!(this.tenancy === other.tenancy)) {
      return false;
    }
    if (!(this.connectionUrl === other.connectionUrl)) {
      return false;
    }
    if (!(this.spacePtr?.id === other.spacePtr?.id)) {
      return false;
    }
    if (!(this.name === other.name)) {
      return false;
    }
    if (!(this.status === other.status)) {
      return false;
    }
    if (!(this.targetStatus === other.targetStatus)) {
      return false;
    }
    return true;
  }

  hash(): number {
    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    if (this.parentPtr !== null) {
      h = (h * 31 + hashString(this.parentPtr.id)) & 0xffffffff;
    }
    h = (h * 31 + this.type) & 0xffffffff;
    h = (h * 31 + this.region) & 0xffffffff;
    if (this.galaxyName !== null) {
      h = (h * 31 + hashString(this.galaxyName)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.externalName)) & 0xffffffff;
    if (this.customSchemaName !== null) {
      h = (h * 31 + hashString(this.customSchemaName)) & 0xffffffff;
    }
    h = (h * 31 + this.tenancy) & 0xffffffff;
    if (this.connectionUrl !== null) {
      h = (h * 31 + hashString(this.connectionUrl)) & 0xffffffff;
    }
    if (this.spacePtr !== null) {
      h = (h * 31 + hashString(this.spacePtr.id)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.name)) & 0xffffffff;
    h = (h * 31 + this.status) & 0xffffffff;
    if (this.targetStatus !== null) {
      h = (h * 31 + hashString(this.targetStatus.toString({ timeZoneName: "never" }))) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.id.toString())) & 0xffffffff;
    if (this.deletedAt !== null) {
      h = (h * 31 + hashString(this.deletedAt.toString({ timeZoneName: "never" }))) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.createdAt.toString({ timeZoneName: "never" }))) & 0xffffffff;
    if (this.createdByPtr !== null) {
      h = (h * 31 + hashString(this.createdByPtr.id)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.updatedAt.toString({ timeZoneName: "never" }))) & 0xffffffff;
    if (this.updatedByPtr !== null) {
      h = (h * 31 + hashString(this.updatedByPtr.id)) & 0xffffffff;
    }

    return h;
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

  repr(): string {
    const propertyReprs: string[] = [];
    propertyReprs.push(`type=${DatabaseType[this.type]}`);
    propertyReprs.push(`region=${Region[this.region]}`);
    if (this.galaxyName !== null) {
      propertyReprs.push(`galaxyName=${this.galaxyName}`);
    }
    propertyReprs.push(`externalName=${this.externalName}`);
    if (this.customSchemaName !== null) {
      propertyReprs.push(`customSchemaName=${this.customSchemaName}`);
    }
    propertyReprs.push(`tenancy=${Tenancy[this.tenancy]}`);
    propertyReprs.push(`name=${this.name}`);
    return `<Database '${this.path}' ${propertyReprs.join(" ")}>`;
  }

  toValue(): { [key: string]: any } {
    return Database.__packValue__(this);
  }

  static __packValue__(object: Database): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 7500;
    objectValue["2"] = String(object.id);
    if (object.parentPtr != null) {
      objectValue["3"] = object.parentPtr.toValue();
    }
    if (object.spacePtr != null) {
      objectValue["5"] = object.spacePtr.toValue();
    }
    objectValue["15"] = object.createdAt.toString({ timeZoneName: "never" });
    if (object.createdByPtr != null) {
      objectValue["16"] = object.createdByPtr.toValue();
    }
    objectValue["17"] = object.updatedAt.toString({ timeZoneName: "never" });
    if (object.updatedByPtr != null) {
      objectValue["18"] = object.updatedByPtr.toValue();
    }
    if (object.deletedAt != null) {
      objectValue["20"] = object.deletedAt.toString({ timeZoneName: "never" });
    }
    objectValue["30"] = object.type;
    objectValue["31"] = object.name;
    objectValue["40"] = object.status;
    if (object.targetStatus != null) {
      objectValue["41"] = object.targetStatus.toString({ timeZoneName: "never" });
    }
    objectValue["50"] = object.region;
    if (object.galaxyName != null) {
      objectValue["51"] = object.galaxyName;
    }
    objectValue["52"] = object.externalName;
    if (object.customSchemaName != null) {
      objectValue["53"] = object.customSchemaName;
    }
    objectValue["55"] = object.tenancy;
    if (object.connectionUrl != null) {
      objectValue["58"] = object.connectionUrl;
    }
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Database {
    const parentPtrValue = objectValue["3"];
    const unpackedParentPtr =
      parentPtrValue != undefined
        ? NodeReference.fromValue(parentPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const galaxyNameValue = objectValue["51"];
    const unpackedGalaxyName = galaxyNameValue != undefined ? galaxyNameValue : null;
    const customSchemaNameValue = objectValue["53"];
    const unpackedCustomSchemaName =
      customSchemaNameValue != undefined ? customSchemaNameValue : null;
    const connectionUrlValue = objectValue["58"];
    const unpackedConnectionUrl = connectionUrlValue != undefined ? connectionUrlValue : null;
    const spacePtrValue = objectValue["5"];
    const unpackedSpacePtr =
      spacePtrValue != undefined
        ? NodeReference.fromValue(spacePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const targetStatusValue = objectValue["41"];
    const unpackedTargetStatus =
      targetStatusValue != undefined
        ? Temporal.Instant.from(targetStatusValue).toZonedDateTimeISO("UTC")
        : null;
    const deletedAtValue = objectValue["20"];
    const unpackedDeletedAt =
      deletedAtValue != undefined
        ? Temporal.Instant.from(deletedAtValue).toZonedDateTimeISO("UTC")
        : null;
    const createdByPtrValue = objectValue["16"];
    const unpackedCreatedByPtr =
      createdByPtrValue != undefined
        ? NodeReference.fromValue(createdByPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const updatedByPtrValue = objectValue["18"];
    const unpackedUpdatedByPtr =
      updatedByPtrValue != undefined
        ? NodeReference.fromValue(updatedByPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    return new Database({
      parent: unpackedParentPtr,
      type: Number(objectValue["30"]),
      region: Number(objectValue["50"]),
      galaxyName: unpackedGalaxyName,
      externalName: objectValue["52"],
      customSchemaName: unpackedCustomSchemaName,
      tenancy: Number(objectValue["55"]),
      connectionUrl: unpackedConnectionUrl,
      space: unpackedSpacePtr,
      name: objectValue["31"],
      status: Number(objectValue["40"]),
      targetStatus: unpackedTargetStatus,
      id: String(objectValue["2"]),
      deletedAt: unpackedDeletedAt,
      createdAt: Temporal.Instant.from(objectValue["15"]).toZonedDateTimeISO("UTC"),
      createdBy: unpackedCreatedByPtr,
      updatedAt: Temporal.Instant.from(objectValue["17"]).toZonedDateTimeISO("UTC"),
      updatedBy: unpackedUpdatedByPtr,
      _session,
      _graph,
      _connection,
    });
  }

  static fromValue(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Database {
    return Database.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
  }

  toProto(): DatabaseProto {
    return Database.__packProto__(this);
  }

  static __packProto__(object: Database): DatabaseProto {
    const objectProto: Partial<DatabaseProto> = { metatype: 7500 };
    objectProto.id = String(object.id);
    if (object.parentPtr != null) {
      objectProto.parentPtr = object.parentPtr.toProto();
    }
    if (object.spacePtr != null) {
      objectProto.spacePtr = object.spacePtr.toProto();
    }
    objectProto.createdAt = packProtoTimestamp(object.createdAt);
    if (object.createdByPtr != null) {
      objectProto.createdByPtr = object.createdByPtr.toProto();
    }
    objectProto.updatedAt = packProtoTimestamp(object.updatedAt);
    if (object.updatedByPtr != null) {
      objectProto.updatedByPtr = object.updatedByPtr.toProto();
    }
    if (object.deletedAt != null) {
      objectProto.deletedAt = packProtoTimestamp(object.deletedAt);
    }
    objectProto.type = Number(object.type) as DatabaseTypeProto;
    objectProto.name = object.name;
    objectProto.status = Number(object.status) as ResourceStatusProto;
    if (object.targetStatus != null) {
      objectProto.targetStatus = packProtoTimestamp(object.targetStatus);
    }
    objectProto.region = Number(object.region) as RegionProto;
    if (object.galaxyName != null) {
      objectProto.galaxyName = object.galaxyName;
    }
    objectProto.externalName = object.externalName;
    if (object.customSchemaName != null) {
      objectProto.customSchemaName = object.customSchemaName;
    }
    objectProto.tenancy = Number(object.tenancy) as TenancyProto;
    if (object.connectionUrl != null) {
      objectProto.connectionUrl = object.connectionUrl;
    }
    return objectProto as DatabaseProto;
  }

  static __unpackProto__(
    objectProto: DatabaseProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Database {
    return new Database({
      parent:
        objectProto.parentPtr != undefined
          ? NodeReference.fromProto(
              objectProto.parentPtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      type: Number(objectProto.type) as DatabaseType,
      region: Number(objectProto.region) as Region,
      galaxyName: objectProto.galaxyName != undefined ? objectProto.galaxyName : null,
      externalName: objectProto.externalName,
      customSchemaName:
        objectProto.customSchemaName != undefined ? objectProto.customSchemaName : null,
      tenancy: Number(objectProto.tenancy) as Tenancy,
      connectionUrl: objectProto.connectionUrl != undefined ? objectProto.connectionUrl : null,
      space:
        objectProto.spacePtr != undefined
          ? NodeReference.fromProto(
              objectProto.spacePtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      name: objectProto.name,
      status: Number(objectProto.status) as ResourceStatus,
      targetStatus:
        objectProto.targetStatus != undefined
          ? unpackProtoTimestamp(objectProto.targetStatus!)
          : null,
      id: String(objectProto.id),
      deletedAt:
        objectProto.deletedAt != undefined ? unpackProtoTimestamp(objectProto.deletedAt!) : null,
      createdAt: unpackProtoTimestamp(objectProto.createdAt!),
      createdBy:
        objectProto.createdByPtr != undefined
          ? NodeReference.fromProto(
              objectProto.createdByPtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      updatedAt: unpackProtoTimestamp(objectProto.updatedAt!),
      updatedBy:
        objectProto.updatedByPtr != undefined
          ? NodeReference.fromProto(
              objectProto.updatedByPtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      _session,
      _graph,
      _connection,
    });
  }

  static fromProto(
    objectProto: DatabaseProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Database {
    return Database.__unpackProto__(objectProto, _session, _supergraph, _graph, _connection);
  }

  static fromProtoString(packedProtoString: string): Database {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = DatabaseProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerNodeClass(NodeType.DATABASE, Database);
/* ==== DESTACK_GENERATED_END:NODE:7500 ==== */
