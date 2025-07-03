import { packProtoTimestamp, unpackProtoTimestamp } from "@destack/grpc";
import type {
  CustomEntityDefinition,
  CustomEventDefinition,
  Graph,
  Icon,
  IsSpatial,
  IsSubject,
  NodeDefinitionReference,
  NodeReference,
  QueryConnection,
  Session,
  Snapshot,
  Supergraph,
  Value,
} from "@destack/language/core";
import {
  Entity,
  EnumType,
  Materialization,
  Node,
  NodeType,
  Region,
  Resource,
  ResourceStatus,
  StructFrozen,
  StructType,
  Tenancy,
} from "@destack/language/core";
import {
  STRUCT_CLASS_BY_TYPE,
  registerEnumClass,
  registerNodeClass,
  registerStructClass,
} from "@destack/language/registry";
import type { Space } from "@destack/language/space";
import {
  DatabaseInfoProto,
  DatabaseProto,
  DatabaseTypeProto,
  MaterializationProto,
  RegionProto,
  ResourceStatusProto,
  TenancyProto,
} from "@destack/proto";
import { base64Decode } from "@destack/utils";
import { hashString } from "@destack/utils/hash";
import { Temporal } from "temporal-polyfill";

/* ==== DESTACK_GENERATED_START:ENUM:160005 ==== */
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
/* ==== DESTACK_GENERATED_END:ENUM:160005 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:160001 ==== */
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
      _tenancy = 1 /* Tenancy.DEDICATED */;
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
    objectValue["1"] = 160001;
    objectValue["100"] = object.type;
    objectValue["110"] = object.region;
    if (object.galaxyName != null) {
      objectValue["111"] = object.galaxyName;
    }
    objectValue["112"] = object.externalName;
    if (object.customSchemaName != null) {
      objectValue["113"] = object.customSchemaName;
    }
    objectValue["115"] = object.tenancy;
    if (object.connectionUrl != null) {
      objectValue["118"] = object.connectionUrl;
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
    const galaxyNameValue = objectValue["111"];
    const unpackedGalaxyName = galaxyNameValue != undefined ? galaxyNameValue : null;
    const customSchemaNameValue = objectValue["113"];
    const unpackedCustomSchemaName =
      customSchemaNameValue != undefined ? customSchemaNameValue : null;
    const connectionUrlValue = objectValue["118"];
    const unpackedConnectionUrl = connectionUrlValue != undefined ? connectionUrlValue : null;
    return new DatabaseInfo({
      type: Number(objectValue["100"]),
      region: Number(objectValue["110"]),
      galaxyName: unpackedGalaxyName,
      externalName: objectValue["112"],
      customSchemaName: unpackedCustomSchemaName,
      tenancy: Number(objectValue["115"]),
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
    const objectProto: Partial<DatabaseInfoProto> = { metatype: 160001 };
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
/* ==== DESTACK_GENERATED_END:STRUCT:160001 ==== */

/* ==== DESTACK_GENERATED_START:NODE:160000 ==== */
/**
 * A primary storage Database of some flavor.
 */
export class Database extends Resource implements IsSpatial {
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
   * The definitionthis CustomEntity is an instance of.
   */
  get definition(): CustomEntityDefinition | CustomEventDefinition | null {
    const nodePtr: NodeReference | null = this.definitionPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as
        | CustomEntityDefinition
        | CustomEventDefinition
        | null;
    }
    return null;
  }
  readonly definitionPtr: NodeReference | null;

  /**
   * Inlined base type of this extensible Node (if extended).
   */
  readonly baseType: NodeDefinitionReference | null;

  /**
   * Entity.materialization
   */
  readonly materialization: Materialization;

  /**
   * The Snapshot this Entity is part of.
   */
  get snapshot(): Snapshot | null {
    const nodePtr: NodeReference | null = this.snapshotPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Snapshot | null;
    }
    return null;
  }
  readonly snapshotPtr: NodeReference | null;

  /**
   * The previous Entity this Entity is based on (from another Snapshot).
   */
  get predecessor(): Database | null {
    const nodePtr: NodeReference | null = this.predecessorPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Database | null;
    }
    return null;
  }
  readonly predecessorPtr: NodeReference | null;

  /**
   * The template this Entity instance is based on.
   */
  get template(): Database | null {
    const nodePtr: NodeReference | null = this.templatePtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Database | null;
    }
    return null;
  }
  readonly templatePtr: NodeReference | null;

  /**
   * The (root) Entity in this Entity's instance tree.
   */
  get instanceRoot(): Entity | null {
    const nodePtr: NodeReference | null = this.instanceRootPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Entity | null;
    }
    return null;
  }
  readonly instanceRootPtr: NodeReference | null;

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
   * The custom Values of this Node, keyed by custom Property id. May hold both static and instance values.
   */
  customValues: Map<string, Value>;

  /**
   * Resource.status
   */
  status: ResourceStatus;

  /**
   * Database.type
   */
  readonly type: DatabaseType;

  /**
   * Database.name
   */
  name: string;

  /**
   * Database.icon
   */
  icon: Icon | null;

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
    definition?: CustomEntityDefinition | CustomEventDefinition | NodeReference | null;
    baseType?: NodeDefinitionReference | null;
    materialization?: Materialization;
    snapshot?: Snapshot | NodeReference | null;
    predecessor?: Database | NodeReference | null;
    template?: Database | NodeReference | null;
    instanceRoot?: Entity | NodeReference | null;
    createdAt?: Temporal.ZonedDateTime;
    createdBy?: (Node & IsSubject) | NodeReference | null;
    updatedAt?: Temporal.ZonedDateTime;
    updatedBy?: (Node & IsSubject) | NodeReference | null;
    deletedAt?: Temporal.ZonedDateTime | null;
    customValues?: Map<string, Value>;
    status?: ResourceStatus;
    type: DatabaseType;
    name: string;
    icon?: Icon | null;
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
    if (_parent != null && _parent.metatype != StructType.NODE_REFERENCE) {
      _parent = (_parent as Node).toRef();
    }
    this.parentPtr = _parent;
    let _space = options.space ?? null;
    if (_space != null && _space.metatype != StructType.NODE_REFERENCE) {
      _space = (_space as Node).toRef();
    }
    this.spacePtr = _space;
    let _definition = options.definition ?? null;
    if (_definition != null && _definition.metatype != StructType.NODE_REFERENCE) {
      _definition = (_definition as Node).toRef();
    }
    this.definitionPtr = _definition;
    let _baseType = options.baseType ?? null;
    this.baseType = _baseType;
    let _materialization = options.materialization ?? null;
    if (_materialization === null) {
      _materialization = 3 /* Materialization.FULL */;
    }
    if (_materialization === null) {
      throw new Error(`Database.materialization is required`);
    }
    this.materialization = _materialization;
    let _snapshot = options.snapshot ?? null;
    if (_snapshot != null && _snapshot.metatype != StructType.NODE_REFERENCE) {
      _snapshot = (_snapshot as Node).toRef();
    }
    this.snapshotPtr = _snapshot;
    let _predecessor = options.predecessor ?? null;
    if (_predecessor != null && _predecessor.metatype != StructType.NODE_REFERENCE) {
      _predecessor = (_predecessor as Node).toRef();
    }
    this.predecessorPtr = _predecessor;
    let _template = options.template ?? null;
    if (_template != null && _template.metatype != StructType.NODE_REFERENCE) {
      _template = (_template as Node).toRef();
    }
    this.templatePtr = _template;
    let _instanceRoot = options.instanceRoot ?? null;
    if (_instanceRoot != null && _instanceRoot.metatype != StructType.NODE_REFERENCE) {
      _instanceRoot = (_instanceRoot as Node).toRef();
    }
    this.instanceRootPtr = _instanceRoot;
    let _deletedAt = options.deletedAt ?? null;
    this.deletedAt = _deletedAt;
    let _customValues = options.customValues ?? null;
    if (_customValues === null) {
      _customValues = new Map();
    }
    this.customValues = _customValues;
    let _status = options.status ?? null;
    if (_status === null) {
      _status = 1 /* ResourceStatus.PENDING */;
    }
    if (_status === null) {
      throw new Error(`Database.status is required`);
    }
    this.status = _status;
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
    let _icon = options.icon ?? null;
    this.icon = _icon;
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
      _tenancy = 1 /* Tenancy.DEDICATED */;
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
          ? options.createdBy.metatype == StructType.NODE_REFERENCE
            ? (options.createdBy as NodeReference)
            : (options.createdBy as Node).toRef()
          : null;
      this.updatedAt = options.updatedAt;
      this.updatedByPtr =
        options.updatedBy != null
          ? options.updatedBy.metatype == StructType.NODE_REFERENCE
            ? (options.updatedBy as NodeReference)
            : (options.updatedBy as Node).toRef()
          : null;
    }
  }

  equals(other: any): boolean {
    if (!(this.metatype === other.metatype)) {
      return false;
    }
    if (!(this.name === other.name)) {
      return false;
    }
    if (
      (this.icon == null) !== (other.icon == null) ||
      (this.icon != null && !this.icon.equals(other.icon))
    ) {
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
    if (!(this.status === other.status)) {
      return false;
    }
    if (!(this.definitionPtr?.id === other.definitionPtr?.id)) {
      return false;
    }
    if (
      (this.baseType == null) !== (other.baseType == null) ||
      (this.baseType != null && !this.baseType.equals(other.baseType))
    ) {
      return false;
    }
    if (!(this.snapshotPtr?.id === other.snapshotPtr?.id)) {
      return false;
    }
    if (!(this.predecessorPtr?.id === other.predecessorPtr?.id)) {
      return false;
    }
    if (!(this.templatePtr?.id === other.templatePtr?.id)) {
      return false;
    }
    if (!(this.instanceRootPtr?.id === other.instanceRootPtr?.id)) {
      return false;
    }
    if (Object.keys(this.customValues).length !== Object.keys(other.customValues).length) {
      return false;
    }
    for (const key in this.customValues) {
      if (!(key in other.customValues)) {
        return false;
      }
      if (!this.customValues.get(key)!.equals(other.customValues.get(key)!)) {
        return false;
      }
    }
    return true;
  }

  hash(): number {
    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    if (this.parentPtr !== null) {
      h = (h * 31 + hashString(this.parentPtr.id)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.name)) & 0xffffffff;
    if (this.icon !== null) {
      h = (h * 31 + this.icon.hash()) & 0xffffffff;
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
    h = (h * 31 + this.status) & 0xffffffff;
    h = (h * 31 + hashString(this.id.toString())) & 0xffffffff;
    if (this.deletedAt !== null) {
      h = (h * 31 + hashString(this.deletedAt.toString({ timeZoneName: "never" }))) & 0xffffffff;
    }
    if (this.definitionPtr !== null) {
      h = (h * 31 + hashString(this.definitionPtr.id)) & 0xffffffff;
    }
    if (this.baseType !== null) {
      h = (h * 31 + this.baseType.hash()) & 0xffffffff;
    }
    if (this.snapshotPtr !== null) {
      h = (h * 31 + hashString(this.snapshotPtr.id)) & 0xffffffff;
    }
    if (this.predecessorPtr !== null) {
      h = (h * 31 + hashString(this.predecessorPtr.id)) & 0xffffffff;
    }
    if (this.templatePtr !== null) {
      h = (h * 31 + hashString(this.templatePtr.id)) & 0xffffffff;
    }
    if (this.instanceRootPtr !== null) {
      h = (h * 31 + hashString(this.instanceRootPtr.id)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.createdAt.toString({ timeZoneName: "never" }))) & 0xffffffff;
    if (this.createdByPtr !== null) {
      h = (h * 31 + hashString(this.createdByPtr.id)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.updatedAt.toString({ timeZoneName: "never" }))) & 0xffffffff;
    if (this.updatedByPtr !== null) {
      h = (h * 31 + hashString(this.updatedByPtr.id)) & 0xffffffff;
    }
    if (this.customValues && Object.keys(this.customValues).length > 0) {
      for (const [_key, _value] of Object.entries(this.customValues)) {
        h = (h * 31 + hashString(_key.toString())) & 0xffffffff;
        h = (h * 31 + _value.hash()) & 0xffffffff;
      }
    }

    return h;
  }

  validate(): void {
    throw new Error("not implemented");
  }

  __toRef__(): NodeReference {
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    return new _NodeReference({
      type: NodeType.DATABASE,
      id: this.id,
      spaceId: this.spacePtr?.id ?? null,
      definitionId: this.definitionPtr?.id ?? null,
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
    propertyReprs.push(`name=${this.name}`);
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
    return `<Database '${this.path}' ${propertyReprs.join(" ")}>`;
  }

  toValue(): { [key: string]: any } {
    return Database.__packValue__(this);
  }

  static __packValue__(object: Database): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 160000;
    objectValue["2"] = String(object.id);
    if (object.parentPtr != null) {
      objectValue["3"] = object.parentPtr.toValue();
    }
    if (object.spacePtr != null) {
      objectValue["5"] = object.spacePtr.toValue();
    }
    if (object.definitionPtr != null) {
      objectValue["6"] = object.definitionPtr.toValue();
    }
    if (object.baseType != null) {
      objectValue["7"] = object.baseType.toValue();
    }
    objectValue["10"] = object.materialization;
    if (object.snapshotPtr != null) {
      objectValue["11"] = object.snapshotPtr.toValue();
    }
    if (object.predecessorPtr != null) {
      objectValue["12"] = object.predecessorPtr.toValue();
    }
    if (object.templatePtr != null) {
      objectValue["13"] = object.templatePtr.toValue();
    }
    if (object.instanceRootPtr != null) {
      objectValue["14"] = object.instanceRootPtr.toValue();
    }
    objectValue["20"] = object.createdAt.toString({ timeZoneName: "never" });
    if (object.createdByPtr != null) {
      objectValue["21"] = object.createdByPtr.toValue();
    }
    objectValue["22"] = object.updatedAt.toString({ timeZoneName: "never" });
    if (object.updatedByPtr != null) {
      objectValue["23"] = object.updatedByPtr.toValue();
    }
    if (object.deletedAt != null) {
      objectValue["25"] = object.deletedAt.toString({ timeZoneName: "never" });
    }
    if (object.customValues.size > 0) {
      const packedCustomValues: { [key: string]: any } = {};
      for (const [key, value] of object.customValues) {
        packedCustomValues[String(String(key))] = value.toValue();
      }
      objectValue["26"] = packedCustomValues;
    }
    objectValue["90"] = object.status;
    objectValue["100"] = object.type;
    objectValue["101"] = object.name;
    if (object.icon != null) {
      objectValue["102"] = object.icon.toValue();
    }
    objectValue["110"] = object.region;
    if (object.galaxyName != null) {
      objectValue["111"] = object.galaxyName;
    }
    objectValue["112"] = object.externalName;
    if (object.customSchemaName != null) {
      objectValue["113"] = object.customSchemaName;
    }
    objectValue["115"] = object.tenancy;
    if (object.connectionUrl != null) {
      objectValue["118"] = object.connectionUrl;
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
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    const _NodeDefinitionReference = STRUCT_CLASS_BY_TYPE[
      StructType.NODE_DEFINITION_REFERENCE
    ] as typeof NodeDefinitionReference;
    const _Value = STRUCT_CLASS_BY_TYPE[StructType.VALUE] as typeof Value;
    const _Icon = STRUCT_CLASS_BY_TYPE[StructType.ICON] as typeof Icon;
    const parentPtrValue = objectValue["3"];
    const unpackedParentPtr =
      parentPtrValue != undefined
        ? _NodeReference.fromValue(parentPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const iconValue = objectValue["102"];
    const unpackedIcon =
      iconValue != undefined
        ? _Icon.fromValue(iconValue, _session, _supergraph, _graph, _connection)
        : null;
    const galaxyNameValue = objectValue["111"];
    const unpackedGalaxyName = galaxyNameValue != undefined ? galaxyNameValue : null;
    const customSchemaNameValue = objectValue["113"];
    const unpackedCustomSchemaName =
      customSchemaNameValue != undefined ? customSchemaNameValue : null;
    const connectionUrlValue = objectValue["118"];
    const unpackedConnectionUrl = connectionUrlValue != undefined ? connectionUrlValue : null;
    const spacePtrValue = objectValue["5"];
    const unpackedSpacePtr =
      spacePtrValue != undefined
        ? _NodeReference.fromValue(spacePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const deletedAtValue = objectValue["25"];
    const unpackedDeletedAt =
      deletedAtValue != undefined
        ? Temporal.Instant.from(deletedAtValue).toZonedDateTimeISO("UTC")
        : null;
    const definitionPtrValue = objectValue["6"];
    const unpackedDefinitionPtr =
      definitionPtrValue != undefined
        ? _NodeReference.fromValue(definitionPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const baseTypeValue = objectValue["7"];
    const unpackedBaseType =
      baseTypeValue != undefined
        ? _NodeDefinitionReference.fromValue(
            baseTypeValue,
            _session,
            _supergraph,
            _graph,
            _connection,
          )
        : null;
    const snapshotPtrValue = objectValue["11"];
    const unpackedSnapshotPtr =
      snapshotPtrValue != undefined
        ? _NodeReference.fromValue(snapshotPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const predecessorPtrValue = objectValue["12"];
    const unpackedPredecessorPtr =
      predecessorPtrValue != undefined
        ? _NodeReference.fromValue(predecessorPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const templatePtrValue = objectValue["13"];
    const unpackedTemplatePtr =
      templatePtrValue != undefined
        ? _NodeReference.fromValue(templatePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const instanceRootPtrValue = objectValue["14"];
    const unpackedInstanceRootPtr =
      instanceRootPtrValue != undefined
        ? _NodeReference.fromValue(instanceRootPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const createdByPtrValue = objectValue["21"];
    const unpackedCreatedByPtr =
      createdByPtrValue != undefined
        ? _NodeReference.fromValue(createdByPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const updatedByPtrValue = objectValue["23"];
    const unpackedUpdatedByPtr =
      updatedByPtrValue != undefined
        ? _NodeReference.fromValue(updatedByPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const unpackedCustomValues = new Map();
    if (objectValue["26"] != undefined) {
      for (const [key, value] of Object.entries(objectValue["26"])) {
        unpackedCustomValues.set(
          String(key),
          _Value.fromValue(value as any, _session, _supergraph, _graph, _connection),
        );
      }
    }
    return new Database({
      parent: unpackedParentPtr,
      name: objectValue["101"],
      icon: unpackedIcon,
      type: Number(objectValue["100"]),
      region: Number(objectValue["110"]),
      galaxyName: unpackedGalaxyName,
      externalName: objectValue["112"],
      customSchemaName: unpackedCustomSchemaName,
      tenancy: Number(objectValue["115"]),
      connectionUrl: unpackedConnectionUrl,
      space: unpackedSpacePtr,
      status: Number(objectValue["90"]),
      id: String(objectValue["2"]),
      deletedAt: unpackedDeletedAt,
      definition: unpackedDefinitionPtr,
      baseType: unpackedBaseType,
      materialization: Number(objectValue["10"]),
      snapshot: unpackedSnapshotPtr,
      predecessor: unpackedPredecessorPtr,
      template: unpackedTemplatePtr,
      instanceRoot: unpackedInstanceRootPtr,
      createdAt: Temporal.Instant.from(objectValue["20"]).toZonedDateTimeISO("UTC"),
      createdBy: unpackedCreatedByPtr,
      updatedAt: Temporal.Instant.from(objectValue["22"]).toZonedDateTimeISO("UTC"),
      updatedBy: unpackedUpdatedByPtr,
      customValues: unpackedCustomValues,
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
    const objectProto: Partial<DatabaseProto> = { metatype: 160000 };
    objectProto.id = String(object.id);
    if (object.parentPtr != null) {
      objectProto.parentPtr = object.parentPtr.toProto();
    }
    if (object.spacePtr != null) {
      objectProto.spacePtr = object.spacePtr.toProto();
    }
    if (object.definitionPtr != null) {
      objectProto.definitionPtr = object.definitionPtr.toProto();
    }
    if (object.baseType != null) {
      objectProto.baseType = object.baseType.toProto();
    }
    objectProto.materialization = Number(object.materialization) as MaterializationProto;
    if (object.snapshotPtr != null) {
      objectProto.snapshotPtr = object.snapshotPtr.toProto();
    }
    if (object.predecessorPtr != null) {
      objectProto.predecessorPtr = object.predecessorPtr.toProto();
    }
    if (object.templatePtr != null) {
      objectProto.templatePtr = object.templatePtr.toProto();
    }
    if (object.instanceRootPtr != null) {
      objectProto.instanceRootPtr = object.instanceRootPtr.toProto();
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
    if (object.customValues) {
      objectProto.customValues = {};
      for (const [key, value] of object.customValues) {
        objectProto.customValues![String(key)] = value.toProto();
      }
    }
    objectProto.status = Number(object.status) as ResourceStatusProto;
    objectProto.type = Number(object.type) as DatabaseTypeProto;
    objectProto.name = object.name;
    if (object.icon != null) {
      objectProto.icon = object.icon.toProto();
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
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    const _NodeDefinitionReference = STRUCT_CLASS_BY_TYPE[
      StructType.NODE_DEFINITION_REFERENCE
    ] as typeof NodeDefinitionReference;
    const _Value = STRUCT_CLASS_BY_TYPE[StructType.VALUE] as typeof Value;
    const _Icon = STRUCT_CLASS_BY_TYPE[StructType.ICON] as typeof Icon;
    const unpackedCustomValues = new Map();
    if (objectProto.customValues) {
      for (const [key, value] of Object.entries(objectProto.customValues)) {
        unpackedCustomValues.set(
          String(key),
          _Value.fromProto((value as any)!, _session, _supergraph, _graph, _connection),
        );
      }
    }
    return new Database({
      parent:
        objectProto.parentPtr != undefined
          ? _NodeReference.fromProto(
              objectProto.parentPtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      name: objectProto.name,
      icon:
        objectProto.icon != undefined
          ? _Icon.fromProto(objectProto.icon!, _session, _supergraph, _graph, _connection)
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
          ? _NodeReference.fromProto(
              objectProto.spacePtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      status: Number(objectProto.status) as ResourceStatus,
      id: String(objectProto.id),
      deletedAt:
        objectProto.deletedAt != undefined ? unpackProtoTimestamp(objectProto.deletedAt!) : null,
      definition:
        objectProto.definitionPtr != undefined
          ? _NodeReference.fromProto(
              objectProto.definitionPtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      baseType:
        objectProto.baseType != undefined
          ? _NodeDefinitionReference.fromProto(
              objectProto.baseType!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      materialization: Number(objectProto.materialization) as Materialization,
      snapshot:
        objectProto.snapshotPtr != undefined
          ? _NodeReference.fromProto(
              objectProto.snapshotPtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      predecessor:
        objectProto.predecessorPtr != undefined
          ? _NodeReference.fromProto(
              objectProto.predecessorPtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      template:
        objectProto.templatePtr != undefined
          ? _NodeReference.fromProto(
              objectProto.templatePtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      instanceRoot:
        objectProto.instanceRootPtr != undefined
          ? _NodeReference.fromProto(
              objectProto.instanceRootPtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      createdAt: unpackProtoTimestamp(objectProto.createdAt!),
      createdBy:
        objectProto.createdByPtr != undefined
          ? _NodeReference.fromProto(
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
          ? _NodeReference.fromProto(
              objectProto.updatedByPtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      customValues: unpackedCustomValues,
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
/* ==== DESTACK_GENERATED_END:NODE:160000 ==== */
