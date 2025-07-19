import { packProtoTimestamp, unpackProtoTimestamp } from "@destack/grpc";
import type {
  Branch,
  Graph,
  Icon,
  IsActor,
  NodeClass,
  NodeReference,
  QueryConnection,
  Session,
  Snapshot,
  Space,
  Supergraph,
  Value,
} from "@destack/language/core";
import {
  ACTIVE_BRANCH,
  ACTIVE_SNAPSHOT,
  ACTIVE_SPACE,
  Entity,
  EnumType,
  Event,
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
import type { Script } from "@destack/language/logic";
import {
  STRUCT_CLASS_BY_TYPE,
  registerEnumClass,
  registerNodeClass,
  registerStructClass,
} from "@destack/language/registry";
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
import { hashBool, hashString } from "@destack/utils/hash";
import { Temporal } from "temporal-polyfill";

/* ==== DESTACK_GENERATED_START:ENUM:1000005 ==== */
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
/* ==== DESTACK_GENERATED_END:ENUM:1000005 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:1000001 ==== */
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
      if (this.galaxyName != null) {
        propertyReprs.push(`galaxyName=${`"${this.galaxyName}"`}`);
      }
      propertyReprs.push(`externalName=${`"${this.externalName}"`}`);
      if (this.customSchemaName != null) {
        propertyReprs.push(`customSchemaName=${`"${this.customSchemaName}"`}`);
      }
      propertyReprs.push(`tenancy=${Tenancy[this.tenancy]}`);
      // @ts-expect-error(readonly)
      this._repr = `<DatabaseInfo ${propertyReprs.join(" ")}>`;
    }
    return this._repr;
  }

  hash(): number {
    if (this._hash != null) {
      return this._hash;
    }

    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    h = (h * 31 + this.type) & 0xffffffff;
    h = (h * 31 + this.region) & 0xffffffff;
    if (this.galaxyName != null) {
      h = (h * 31 + hashString(this.galaxyName)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.externalName)) & 0xffffffff;
    if (this.customSchemaName != null) {
      h = (h * 31 + hashString(this.customSchemaName)) & 0xffffffff;
    }
    h = (h * 31 + this.tenancy) & 0xffffffff;
    if (this.connectionUrl != null) {
      h = (h * 31 + hashString(this.connectionUrl)) & 0xffffffff;
    }

    // @ts-expect-error(readonly)
    this._hash = h;
    return h;
  }

  validate(): void {
    throw new Error("not implemented");
  }

  toValue(): { readonly [key: string]: any } {
    if (this._value === null) {
      // @ts-expect-error(readonly)
      this._value = DatabaseInfo.__packValue__(this);
    }
    return this._value;
  }

  static __packValue__(object: DatabaseInfo): { readonly [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 1000001;
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
    objectValue: { readonly [key: string]: any },
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
    objectValue: { readonly [key: string]: any },
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
    const objectProto: Partial<DatabaseInfoProto> = { metatype: 1000001 };
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
/* ==== DESTACK_GENERATED_END:STRUCT:1000001 ==== */

/* ==== DESTACK_GENERATED_START:NODE:1000000 ==== */
/**
 * A primary storage Database of some flavor.
 */
export class Database extends Resource {
  static metatype: NodeType = NodeType.DATABASE;

  /**
   * The parent of this Entity. Most Entities can be attached to any other Entity.
   */
  get parent(): Entity | null {
    const nodePtr: NodeReference | null = this.parentPtr;
    if (nodePtr != null) {
      return this._supergraph.get(nodePtr.id) as Entity | null;
    }
    return null;
  }
  readonly parentPtr: NodeReference | null;

  /**
   * The Space this Node is in.
   */
  get space(): Space | null {
    const nodePtr: NodeReference | null = this.spacePtr;
    if (nodePtr != null) {
      return this._supergraph.get(nodePtr.id) as Space | null;
    }
    return null;
  }
  readonly spacePtr: NodeReference;

  /**
   * Entity.materialization
   */
  readonly materialization: Materialization;

  /**
   * The definition this CustomEntity is an instance of.
   */
  get definition(): Entity | null {
    const nodePtr: NodeReference | null = this.definitionPtr;
    if (nodePtr != null) {
      return this._supergraph.get(nodePtr.id) as Entity | null;
    }
    return null;
  }
  readonly definitionPtr: NodeReference | null;

  /**
   * The Branch this Entity is part of.
   */
  get branch(): Branch | null {
    const nodePtr: NodeReference | null = this.branchPtr;
    if (nodePtr != null) {
      return this._supergraph.get(nodePtr.id) as Branch | null;
    }
    return null;
  }
  readonly branchPtr: NodeReference;

  /**
   * The Snapshot this Entity is part of.
   */
  get snapshot(): Snapshot | null {
    const nodePtr: NodeReference | null = this.snapshotPtr;
    if (nodePtr != null) {
      return this._supergraph.get(nodePtr.id) as Snapshot | null;
    }
    return null;
  }
  readonly snapshotPtr: NodeReference;

  /**
   * The previous Entity this Entity is based on (from the base Branch, if any).
   * This invariant must hold: `Entity.preceded_by.branch == Entity.branch.preceded_by`.
   */
  get precededBy(): Database | null {
    const nodePtr: NodeReference | null = this.precededByPtr;
    if (nodePtr != null) {
      return this._supergraph.get(nodePtr.id) as Database | null;
    }
    return null;
  }
  readonly precededByPtr: NodeReference | null;

  /**
   * The (root) Entity that is being instantiated.
   */
  get instance(): Entity | null {
    const nodePtr: NodeReference | null = this.instancePtr;
    if (nodePtr != null) {
      return this._supergraph.get(nodePtr.id) as Entity | null;
    }
    return null;
  }
  readonly instancePtr: NodeReference | null;

  /**
   * The time this Entity was created (system time).
   */
  readonly createdAt: Temporal.ZonedDateTime;

  /**
   * The logical time this Entity was created (system time).
   */
  readonly createdEpoch: number;

  /**
   * The Actor that created this Entity.
   */
  get createdBy(): (Entity & IsActor) | null {
    const nodePtr: NodeReference | null = this.createdByPtr;
    if (nodePtr != null) {
      return this._supergraph.get(nodePtr.id) as (Entity & IsActor) | null;
    }
    return null;
  }
  readonly createdByPtr: NodeReference | null;

  /**
   * The time this Entity was last updated (system time).
   */
  readonly updatedAt: Temporal.ZonedDateTime;

  /**
   * The logical time this Entity was last updated (system time).
   */
  readonly updatedEpoch: number;

  /**
   * The Actor that last updated this Entity.
   */
  get updatedBy(): (Entity & IsActor) | null {
    const nodePtr: NodeReference | null = this.updatedByPtr;
    if (nodePtr != null) {
      return this._supergraph.get(nodePtr.id) as (Entity & IsActor) | null;
    }
    return null;
  }
  readonly updatedByPtr: NodeReference | null;

  /**
   * The time this Entity was deleted (system time).
   * Only set if the Entity is currently 'deleted'.
   * Deleting and restoring an Entity counts as an update, and thus updates updated_at/updated_epoch.
   */
  readonly deletedAt: Temporal.ZonedDateTime | null;

  /**
   * The custom Values of this Node, keyed by custom Property id. May hold both static and instance values.
   */
  /**
   * The custom Values of this Node, keyed by custom Property id. May hold both static and instance values.
   */
  get customValues(): { readonly [key: string]: Value } {
    return this._customValues;
  }
  set customValues(value: { readonly [key: string]: Value }) {
    const prop = (this.constructor as NodeClass).__properties__["custom_values"];
    this._session.updateSetProperty(this, prop, value);
    this._customValues = value;
  }
  _customValues: { readonly [key: string]: Value };

  /**
   * IsOwnable.ownedBy
   */
  get ownedBy(): (Entity & IsActor) | null {
    const nodePtr: NodeReference | null = this.ownedByPtr;
    if (nodePtr != null) {
      return this._supergraph.get(nodePtr.id) as (Entity & IsActor) | null;
    }
    return null;
  }
  set ownedBy(node: (Entity & IsActor) | null) {
    if (node === null) {
      this.ownedByPtr = null;
    } else {
      this.ownedByPtr = node.toRef();
    }
  }
  /**
   * IsOwnable.ownedBy
   */
  get ownedByPtr(): NodeReference | null {
    return this._ownedByPtr;
  }
  set ownedByPtr(value: NodeReference | null) {
    const prop = (this.constructor as NodeClass).__properties__["owned_by"];
    this._session.updateSetProperty(this, prop, value);
    this._ownedByPtr = value;
  }
  _ownedByPtr: NodeReference | null;

  /**
   * Resource.status
   */
  /**
   * Resource.status
   */
  get status(): ResourceStatus {
    return this._status;
  }
  set status(value: ResourceStatus) {
    const prop = (this.constructor as NodeClass).__properties__["status"];
    this._session.updateSetProperty(this, prop, value);
    this._status = value;
  }
  _status: ResourceStatus;

  /**
   * Entity.name
   */
  /**
   * Entity.name
   */
  get name(): string {
    return this._name;
  }
  set name(value: string) {
    const prop = (this.constructor as NodeClass).__properties__["name"];
    this._session.updateSetProperty(this, prop, value);
    this._name = value;
  }
  _name: string;

  /**
   * The main / root Script of this Node.
   */
  get script(): Script | null {
    const nodePtr: NodeReference | null = this.scriptPtr;
    if (nodePtr != null) {
      return this._supergraph.get(nodePtr.id) as Script | null;
    }
    return null;
  }
  set script(node: Script | null) {
    if (node === null) {
      this.scriptPtr = null;
    } else {
      this.scriptPtr = node.toRef();
    }
  }
  /**
   * The main / root Script of this Node.
   */
  get scriptPtr(): NodeReference | null {
    return this._scriptPtr;
  }
  set scriptPtr(value: NodeReference | null) {
    const prop = (this.constructor as NodeClass).__properties__["script"];
    this._session.updateSetProperty(this, prop, value);
    this._scriptPtr = value;
  }
  _scriptPtr: NodeReference | null;

  /**
   * Whether this Node is extensible (whether it can be instanced).
   */
  readonly isExtensible: boolean;

  /**
   * Database.type
   */
  /**
   * Database.type
   */
  get type(): DatabaseType {
    return this._type;
  }
  set type(value: DatabaseType) {
    const prop = (this.constructor as NodeClass).__properties__["type"];
    this._session.updateSetProperty(this, prop, value);
    this._type = value;
  }
  _type: DatabaseType;

  /**
   * Database.icon
   */
  /**
   * Database.icon
   */
  get icon(): Icon | null {
    return this._icon;
  }
  set icon(value: Icon | null) {
    const prop = (this.constructor as NodeClass).__properties__["icon"];
    this._session.updateSetProperty(this, prop, value);
    this._icon = value;
  }
  _icon: Icon | null;

  /**
   * Database.region
   */
  /**
   * Database.region
   */
  get region(): Region {
    return this._region;
  }
  set region(value: Region) {
    const prop = (this.constructor as NodeClass).__properties__["region"];
    this._session.updateSetProperty(this, prop, value);
    this._region = value;
  }
  _region: Region;

  /**
   * Database.galaxyName
   */
  /**
   * Database.galaxyName
   */
  get galaxyName(): string | null {
    return this._galaxyName;
  }
  set galaxyName(value: string | null) {
    const prop = (this.constructor as NodeClass).__properties__["galaxy_name"];
    this._session.updateSetProperty(this, prop, value);
    this._galaxyName = value;
  }
  _galaxyName: string | null;

  /**
   * Database.externalName
   */
  /**
   * Database.externalName
   */
  get externalName(): string {
    return this._externalName;
  }
  set externalName(value: string) {
    const prop = (this.constructor as NodeClass).__properties__["external_name"];
    this._session.updateSetProperty(this, prop, value);
    this._externalName = value;
  }
  _externalName: string;

  /**
   * Database.customSchemaName
   */
  /**
   * Database.customSchemaName
   */
  get customSchemaName(): string | null {
    return this._customSchemaName;
  }
  set customSchemaName(value: string | null) {
    const prop = (this.constructor as NodeClass).__properties__["custom_schema_name"];
    this._session.updateSetProperty(this, prop, value);
    this._customSchemaName = value;
  }
  _customSchemaName: string | null;

  /**
   * Database.tenancy
   */
  /**
   * Database.tenancy
   */
  get tenancy(): Tenancy {
    return this._tenancy;
  }
  set tenancy(value: Tenancy) {
    const prop = (this.constructor as NodeClass).__properties__["tenancy"];
    this._session.updateSetProperty(this, prop, value);
    this._tenancy = value;
  }
  _tenancy: Tenancy;

  /**
   * Database.connectionUrl
   */
  /**
   * Database.connectionUrl
   */
  get connectionUrl(): string | null {
    return this._connectionUrl;
  }
  set connectionUrl(value: string | null) {
    const prop = (this.constructor as NodeClass).__properties__["connection_url"];
    this._session.updateSetProperty(this, prop, value);
    this._connectionUrl = value;
  }
  _connectionUrl: string | null;

  constructor(options: {
    id?: string;
    parent?: Entity | NodeReference | null;
    space?: Space | NodeReference;
    materialization?: Materialization;
    definition?: Entity | NodeReference | null;
    branch?: Branch | NodeReference;
    snapshot?: Snapshot | NodeReference;
    precededBy?: Database | NodeReference | null;
    instance?: Entity | NodeReference | null;
    createdAt?: Temporal.ZonedDateTime;
    createdEpoch?: number;
    createdBy?: (Entity & IsActor) | NodeReference | null;
    updatedAt?: Temporal.ZonedDateTime;
    updatedEpoch?: number;
    updatedBy?: (Entity & IsActor) | NodeReference | null;
    deletedAt?: Temporal.ZonedDateTime | null;
    customValues?: { readonly [key: string]: Value };
    ownedBy?: (Entity & IsActor) | NodeReference | null;
    status?: ResourceStatus;
    name?: string;
    script?: Script | NodeReference | null;
    isExtensible?: boolean;
    type: DatabaseType;
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
    if (_space === null) {
      _space = ACTIVE_SPACE.get();
      if (_space === null) {
        throw new Error(`no active Space for Database`);
      }
      _space = _space.toRef();
    }
    if (_space === null) {
      throw new Error(`Database.space is required`);
    }
    this.spacePtr = _space;
    let _materialization = options.materialization ?? null;
    if (_materialization === null) {
      _materialization = 11 /* Materialization.ROOT */;
    }
    if (_materialization === null) {
      throw new Error(`Database.materialization is required`);
    }
    this.materialization = _materialization;
    let _definition = options.definition ?? null;
    if (_definition != null && _definition.metatype != StructType.NODE_REFERENCE) {
      _definition = (_definition as Node).toRef();
    }
    this.definitionPtr = _definition;
    let _branch = options.branch ?? null;
    if (_branch != null && _branch.metatype != StructType.NODE_REFERENCE) {
      _branch = (_branch as Node).toRef();
    }
    if (_branch === null) {
      _branch = ACTIVE_BRANCH.get();
      if (_branch === null) {
        throw new Error(`no active Branch for Database`);
      }
      _branch = _branch.toRef();
    }
    if (_branch === null) {
      throw new Error(`Database.branch is required`);
    }
    this.branchPtr = _branch;
    let _snapshot = options.snapshot ?? null;
    if (_snapshot != null && _snapshot.metatype != StructType.NODE_REFERENCE) {
      _snapshot = (_snapshot as Node).toRef();
    }
    if (_snapshot === null) {
      _snapshot = ACTIVE_SNAPSHOT.get();
      if (_snapshot === null) {
        throw new Error(`no active Snapshot for Database`);
      }
      _snapshot = _snapshot.toRef();
    }
    if (_snapshot === null) {
      throw new Error(`Database.snapshot is required`);
    }
    this.snapshotPtr = _snapshot;
    let _precededBy = options.precededBy ?? null;
    if (_precededBy != null && _precededBy.metatype != StructType.NODE_REFERENCE) {
      _precededBy = (_precededBy as Node).toRef();
    }
    this.precededByPtr = _precededBy;
    let _instance = options.instance ?? null;
    if (_instance != null && _instance.metatype != StructType.NODE_REFERENCE) {
      _instance = (_instance as Node).toRef();
    }
    this.instancePtr = _instance;
    let _deletedAt = options.deletedAt ?? null;
    this.deletedAt = _deletedAt;
    let _customValues = options.customValues ?? null;
    if (_customValues === null) {
      _customValues = {};
    }
    this._customValues = _customValues;
    let _ownedBy = options.ownedBy ?? null;
    if (_ownedBy != null && _ownedBy.metatype != StructType.NODE_REFERENCE) {
      _ownedBy = (_ownedBy as Node).toRef();
    }
    this._ownedByPtr = _ownedBy;
    let _status = options.status ?? null;
    if (_status === null) {
      _status = 1 /* ResourceStatus.PENDING */;
    }
    if (_status === null) {
      throw new Error(`Database.status is required`);
    }
    this._status = _status;
    let _name = options.name ?? null;
    if (_name === null) {
      _name = "Database";
    }
    if (_name === null) {
      throw new Error(`Database.name is required`);
    }
    this._name = _name;
    let _script = options.script ?? null;
    if (_script != null && _script.metatype != StructType.NODE_REFERENCE) {
      _script = (_script as Node).toRef();
    }
    this._scriptPtr = _script;
    let _isExtensible = options.isExtensible ?? null;
    if (_isExtensible === null) {
      _isExtensible = false;
    }
    if (_isExtensible === null) {
      throw new Error(`Database.isExtensible is required`);
    }
    this.isExtensible = _isExtensible;
    let _type = options.type;
    if (_type === null) {
      throw new Error(`Database.type is required`);
    }
    this._type = _type;
    let _icon = options.icon ?? null;
    this._icon = _icon;
    let _region = options.region;
    if (_region === null) {
      throw new Error(`Database.region is required`);
    }
    this._region = _region;
    let _galaxyName = options.galaxyName ?? null;
    this._galaxyName = _galaxyName;
    let _externalName = options.externalName;
    if (_externalName === null) {
      throw new Error(`Database.externalName is required`);
    }
    this._externalName = _externalName;
    let _customSchemaName = options.customSchemaName ?? null;
    this._customSchemaName = _customSchemaName;
    let _tenancy = options.tenancy ?? null;
    if (_tenancy === null) {
      _tenancy = 1 /* Tenancy.DEDICATED */;
    }
    if (_tenancy === null) {
      throw new Error(`Database.tenancy is required`);
    }
    this._tenancy = _tenancy;
    let _connectionUrl = options.connectionUrl ?? null;
    this._connectionUrl = _connectionUrl;

    // identity
    if (options.id == null) {
      const now = Temporal.Now.zonedDateTimeISO("UTC");
      const epoch = this._session.epoch;
      this.createdAt = now;
      this.createdEpoch = epoch;
      this.createdByPtr = null;
      this.updatedAt = now;
      this.updatedEpoch = epoch;
      this.updatedByPtr = null;
    } else {
      if (
        options.createdAt == null ||
        options.updatedAt == null ||
        options.createdEpoch == null ||
        options.updatedEpoch == null
      ) {
        throw new Error(
          `Database.createdAt and Database.updatedAt are required for existing Nodes`,
        );
      }
      this.createdAt = options.createdAt;
      this.createdEpoch = options.createdEpoch;
      this.createdByPtr =
        options.createdBy != null
          ? options.createdBy.metatype == StructType.NODE_REFERENCE
            ? (options.createdBy as NodeReference)
            : (options.createdBy as Node).toRef()
          : null;
      this.updatedAt = options.updatedAt;
      this.updatedEpoch = options.updatedEpoch;
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
    if (
      (this._icon == null) !== (other._icon == null) ||
      (this._icon != null && !this._icon.equals(other._icon))
    ) {
      return false;
    }
    if (!(this._type === other._type)) {
      return false;
    }
    if (!(this._region === other._region)) {
      return false;
    }
    if (!(this._galaxyName === other._galaxyName)) {
      return false;
    }
    if (!(this._externalName === other._externalName)) {
      return false;
    }
    if (!(this._customSchemaName === other._customSchemaName)) {
      return false;
    }
    if (!(this._tenancy === other._tenancy)) {
      return false;
    }
    if (!(this._connectionUrl === other._connectionUrl)) {
      return false;
    }
    if (!(this._status === other._status)) {
      return false;
    }
    if (!(this.isExtensible === other.isExtensible)) {
      return false;
    }
    if (!(this._ownedByPtr?.id === other._ownedByPtr?.id)) {
      return false;
    }
    if (!(this.definitionPtr?.id === other.definitionPtr?.id)) {
      return false;
    }
    if (!(this._name === other._name)) {
      return false;
    }
    if (!(this._scriptPtr?.id === other._scriptPtr?.id)) {
      return false;
    }
    if (!(this.spacePtr.id === other.spacePtr.id)) {
      return false;
    }
    if (Object.keys(this._customValues).length !== Object.keys(other._customValues).length) {
      return false;
    }
    for (const key in this._customValues) {
      if (!(key in other._customValues)) {
        return false;
      }
      if (!this._customValues[key].equals(other._customValues[key])) {
        return false;
      }
    }
    return true;
  }

  hash(): number {
    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    if (this._icon != null) {
      h = (h * 31 + this._icon.hash()) & 0xffffffff;
    }
    h = (h * 31 + this._type) & 0xffffffff;
    h = (h * 31 + this._region) & 0xffffffff;
    if (this._galaxyName != null) {
      h = (h * 31 + hashString(this._galaxyName)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this._externalName)) & 0xffffffff;
    if (this._customSchemaName != null) {
      h = (h * 31 + hashString(this._customSchemaName)) & 0xffffffff;
    }
    h = (h * 31 + this._tenancy) & 0xffffffff;
    if (this._connectionUrl != null) {
      h = (h * 31 + hashString(this._connectionUrl)) & 0xffffffff;
    }
    h = (h * 31 + this._status) & 0xffffffff;
    h = (h * 31 + hashBool(this.isExtensible)) & 0xffffffff;
    if (this._ownedByPtr != null) {
      h = (h * 31 + hashString(this._ownedByPtr.id)) & 0xffffffff;
    }
    if (this.parentPtr != null) {
      h = (h * 31 + hashString(this.parentPtr.id)) & 0xffffffff;
    }
    if (this.definitionPtr != null) {
      h = (h * 31 + hashString(this.definitionPtr.id)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.createdAt.toString({ timeZoneName: "never" }))) & 0xffffffff;
    if (this.createdByPtr != null) {
      h = (h * 31 + hashString(this.createdByPtr.id)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.updatedAt.toString({ timeZoneName: "never" }))) & 0xffffffff;
    if (this.updatedByPtr != null) {
      h = (h * 31 + hashString(this.updatedByPtr.id)) & 0xffffffff;
    }
    if (this.deletedAt != null) {
      h = (h * 31 + hashString(this.deletedAt.toString({ timeZoneName: "never" }))) & 0xffffffff;
    }
    h = (h * 31 + hashString(this._name)) & 0xffffffff;
    if (this._scriptPtr != null) {
      h = (h * 31 + hashString(this._scriptPtr.id)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.id.toString())) & 0xffffffff;
    h = (h * 31 + hashString(this.spacePtr.id)) & 0xffffffff;
    if (this._customValues && Object.keys(this._customValues).length > 0) {
      for (const [_key, _value] of Object.entries(this._customValues)) {
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
      branchId: this.branchPtr?.id ?? null,
      snapshotId: this.snapshotPtr?.id ?? null,
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
    let node: Entity | Event | null = this;
    let lastNode: Entity | Event | null = this;
    while (node != null) {
      pathParts.push(node._pathKey);
      lastNode = node;
      node = node.parent;
    }
    if (!lastNode.isRoot) {
      pathParts.push("<detached>");
    }
    return pathParts.reverse().join("/");
  }

  repr(): string {
    const propertyReprs: string[] = [];
    propertyReprs.push(`type=${DatabaseType[this.type]}`);
    propertyReprs.push(`region=${Region[this.region]}`);
    if (this.galaxyName != null) {
      propertyReprs.push(`galaxyName=${`"${this.galaxyName}"`}`);
    }
    propertyReprs.push(`externalName=${`"${this.externalName}"`}`);
    if (this.customSchemaName != null) {
      propertyReprs.push(`customSchemaName=${`"${this.customSchemaName}"`}`);
    }
    propertyReprs.push(`tenancy=${Tenancy[this.tenancy]}`);
    if (this.ownedBy != null) {
      propertyReprs.push(`ownedBy=${this.ownedBy?.repr()}`);
    }
    propertyReprs.push(`name=${`"${this.name}"`}`);
    return `<Database "${this.path}" ${propertyReprs.join(" ")}>`;
  }

  toValue(): { readonly [key: string]: any } {
    return Database.__packValue__(this);
  }

  static __packValue__(object: Database): { readonly [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 1000000;
    objectValue["2"] = String(object.id);
    if (object.parentPtr != null) {
      objectValue["3"] = object.parentPtr.toValue();
    }
    objectValue["5"] = object.spacePtr.toValue();
    objectValue["10"] = object.materialization;
    if (object.definitionPtr != null) {
      objectValue["11"] = object.definitionPtr.toValue();
    }
    objectValue["12"] = object.branchPtr.toValue();
    objectValue["13"] = object.snapshotPtr.toValue();
    if (object.precededByPtr != null) {
      objectValue["14"] = object.precededByPtr.toValue();
    }
    if (object.instancePtr != null) {
      objectValue["15"] = object.instancePtr.toValue();
    }
    objectValue["20"] = object.createdAt.toString({ timeZoneName: "never" });
    objectValue["21"] = object.createdEpoch;
    if (object.createdByPtr != null) {
      objectValue["22"] = object.createdByPtr.toValue();
    }
    objectValue["23"] = object.updatedAt.toString({ timeZoneName: "never" });
    objectValue["24"] = object.updatedEpoch;
    if (object.updatedByPtr != null) {
      objectValue["25"] = object.updatedByPtr.toValue();
    }
    if (object.deletedAt != null) {
      objectValue["26"] = object.deletedAt.toString({ timeZoneName: "never" });
    }
    if (Object.keys(object._customValues).length > 0) {
      const packedCustomValues: { [key: string]: any } = {} as any;
      for (const [key, value] of Object.entries(object._customValues)) {
        packedCustomValues[String(String(key))] = value.toValue();
      }
      objectValue["30"] = packedCustomValues;
    }
    if (object._ownedByPtr != null) {
      objectValue["32"] = object._ownedByPtr.toValue();
    }
    objectValue["40"] = object._status;
    objectValue["50"] = object._name;
    if (object._scriptPtr != null) {
      objectValue["80"] = object._scriptPtr.toValue();
    }
    objectValue["90"] = object.isExtensible;
    objectValue["100"] = object._type;
    if (object._icon != null) {
      objectValue["102"] = object._icon.toValue();
    }
    objectValue["110"] = object._region;
    if (object._galaxyName != null) {
      objectValue["111"] = object._galaxyName;
    }
    objectValue["112"] = object._externalName;
    if (object._customSchemaName != null) {
      objectValue["113"] = object._customSchemaName;
    }
    objectValue["115"] = object._tenancy;
    if (object._connectionUrl != null) {
      objectValue["118"] = object._connectionUrl;
    }
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { readonly [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Database {
    const _Value = STRUCT_CLASS_BY_TYPE[StructType.VALUE] as typeof Value;
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    const _Icon = STRUCT_CLASS_BY_TYPE[StructType.ICON] as typeof Icon;
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
    const ownedByPtrValue = objectValue["32"];
    const unpackedOwnedByPtr =
      ownedByPtrValue != undefined
        ? _NodeReference.fromValue(ownedByPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const parentPtrValue = objectValue["3"];
    const unpackedParentPtr =
      parentPtrValue != undefined
        ? _NodeReference.fromValue(parentPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const definitionPtrValue = objectValue["11"];
    const unpackedDefinitionPtr =
      definitionPtrValue != undefined
        ? _NodeReference.fromValue(definitionPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const precededByPtrValue = objectValue["14"];
    const unpackedPrecededByPtr =
      precededByPtrValue != undefined
        ? _NodeReference.fromValue(precededByPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const instancePtrValue = objectValue["15"];
    const unpackedInstancePtr =
      instancePtrValue != undefined
        ? _NodeReference.fromValue(instancePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const createdByPtrValue = objectValue["22"];
    const unpackedCreatedByPtr =
      createdByPtrValue != undefined
        ? _NodeReference.fromValue(createdByPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const updatedByPtrValue = objectValue["25"];
    const unpackedUpdatedByPtr =
      updatedByPtrValue != undefined
        ? _NodeReference.fromValue(updatedByPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const deletedAtValue = objectValue["26"];
    const unpackedDeletedAt =
      deletedAtValue != undefined
        ? Temporal.Instant.from(deletedAtValue).toZonedDateTimeISO("UTC")
        : null;
    const scriptPtrValue = objectValue["80"];
    const unpackedScriptPtr =
      scriptPtrValue != undefined
        ? _NodeReference.fromValue(scriptPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const unpackedCustomValues = {} as any;
    if (objectValue["30"] != undefined) {
      for (const [key, value] of Object.entries(objectValue["30"])) {
        unpackedCustomValues[String(key)] = _Value.fromValue(
          value as any,
          _session,
          _supergraph,
          _graph,
          _connection,
        );
      }
    }
    return new Database({
      icon: unpackedIcon,
      type: Number(objectValue["100"]),
      region: Number(objectValue["110"]),
      galaxyName: unpackedGalaxyName,
      externalName: objectValue["112"],
      customSchemaName: unpackedCustomSchemaName,
      tenancy: Number(objectValue["115"]),
      connectionUrl: unpackedConnectionUrl,
      status: Number(objectValue["40"]),
      isExtensible: objectValue["90"],
      ownedBy: unpackedOwnedByPtr,
      parent: unpackedParentPtr,
      materialization: Number(objectValue["10"]),
      definition: unpackedDefinitionPtr,
      branch: _NodeReference.fromValue(
        objectValue["12"],
        _session,
        _supergraph,
        _graph,
        _connection,
      ),
      snapshot: _NodeReference.fromValue(
        objectValue["13"],
        _session,
        _supergraph,
        _graph,
        _connection,
      ),
      precededBy: unpackedPrecededByPtr,
      instance: unpackedInstancePtr,
      createdAt: Temporal.Instant.from(objectValue["20"]).toZonedDateTimeISO("UTC"),
      createdEpoch: Number(objectValue["21"]),
      createdBy: unpackedCreatedByPtr,
      updatedAt: Temporal.Instant.from(objectValue["23"]).toZonedDateTimeISO("UTC"),
      updatedEpoch: Number(objectValue["24"]),
      updatedBy: unpackedUpdatedByPtr,
      deletedAt: unpackedDeletedAt,
      name: objectValue["50"],
      script: unpackedScriptPtr,
      id: String(objectValue["2"]),
      space: _NodeReference.fromValue(objectValue["5"], _session, _supergraph, _graph, _connection),
      customValues: unpackedCustomValues,
      _session,
      _graph,
      _connection,
    });
  }

  static fromValue(
    objectValue: { readonly [key: string]: any },
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
    const objectProto: Partial<DatabaseProto> = { metatype: 1000000 };
    objectProto.id = String(object.id);
    if (object.parentPtr != null) {
      objectProto.parentPtr = object.parentPtr.toProto();
    }
    objectProto.spacePtr = object.spacePtr.toProto();
    objectProto.materialization = Number(object.materialization) as MaterializationProto;
    if (object.definitionPtr != null) {
      objectProto.definitionPtr = object.definitionPtr.toProto();
    }
    objectProto.branchPtr = object.branchPtr.toProto();
    objectProto.snapshotPtr = object.snapshotPtr.toProto();
    if (object.precededByPtr != null) {
      objectProto.precededByPtr = object.precededByPtr.toProto();
    }
    if (object.instancePtr != null) {
      objectProto.instancePtr = object.instancePtr.toProto();
    }
    objectProto.createdAt = packProtoTimestamp(object.createdAt);
    objectProto.createdEpoch = object.createdEpoch;
    if (object.createdByPtr != null) {
      objectProto.createdByPtr = object.createdByPtr.toProto();
    }
    objectProto.updatedAt = packProtoTimestamp(object.updatedAt);
    objectProto.updatedEpoch = object.updatedEpoch;
    if (object.updatedByPtr != null) {
      objectProto.updatedByPtr = object.updatedByPtr.toProto();
    }
    if (object.deletedAt != null) {
      objectProto.deletedAt = packProtoTimestamp(object.deletedAt);
    }
    if (object._customValues) {
      objectProto.customValues = {} as any;
      for (const [key, value] of Object.entries(object._customValues)) {
        objectProto.customValues![String(key)] = value.toProto();
      }
    }
    if (object._ownedByPtr != null) {
      objectProto.ownedByPtr = object._ownedByPtr.toProto();
    }
    objectProto.status = Number(object._status) as ResourceStatusProto;
    objectProto.name = object._name;
    if (object._scriptPtr != null) {
      objectProto.scriptPtr = object._scriptPtr.toProto();
    }
    objectProto.isExtensible = object.isExtensible;
    objectProto.type = Number(object._type) as DatabaseTypeProto;
    if (object._icon != null) {
      objectProto.icon = object._icon.toProto();
    }
    objectProto.region = Number(object._region) as RegionProto;
    if (object._galaxyName != null) {
      objectProto.galaxyName = object._galaxyName;
    }
    objectProto.externalName = object._externalName;
    if (object._customSchemaName != null) {
      objectProto.customSchemaName = object._customSchemaName;
    }
    objectProto.tenancy = Number(object._tenancy) as TenancyProto;
    if (object._connectionUrl != null) {
      objectProto.connectionUrl = object._connectionUrl;
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
    const _Value = STRUCT_CLASS_BY_TYPE[StructType.VALUE] as typeof Value;
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    const _Icon = STRUCT_CLASS_BY_TYPE[StructType.ICON] as typeof Icon;
    const unpackedCustomValues = {} as any;
    if (objectProto.customValues) {
      for (const [key, value] of Object.entries(objectProto.customValues)) {
        unpackedCustomValues.set(
          String(key),
          _Value.fromProto((value as any)!, _session, _supergraph, _graph, _connection),
        );
      }
    }
    return new Database({
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
      status: Number(objectProto.status) as ResourceStatus,
      isExtensible: objectProto.isExtensible,
      ownedBy:
        objectProto.ownedByPtr != undefined
          ? _NodeReference.fromProto(
              objectProto.ownedByPtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
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
      materialization: Number(objectProto.materialization) as Materialization,
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
      branch: _NodeReference.fromProto(
        objectProto.branchPtr!,
        _session,
        _supergraph,
        _graph,
        _connection,
      ),
      snapshot: _NodeReference.fromProto(
        objectProto.snapshotPtr!,
        _session,
        _supergraph,
        _graph,
        _connection,
      ),
      precededBy:
        objectProto.precededByPtr != undefined
          ? _NodeReference.fromProto(
              objectProto.precededByPtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      instance:
        objectProto.instancePtr != undefined
          ? _NodeReference.fromProto(
              objectProto.instancePtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      createdAt: unpackProtoTimestamp(objectProto.createdAt!),
      createdEpoch: Number(objectProto.createdEpoch),
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
      updatedEpoch: Number(objectProto.updatedEpoch),
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
      deletedAt:
        objectProto.deletedAt != undefined ? unpackProtoTimestamp(objectProto.deletedAt!) : null,
      name: objectProto.name,
      script:
        objectProto.scriptPtr != undefined
          ? _NodeReference.fromProto(
              objectProto.scriptPtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      id: String(objectProto.id),
      space: _NodeReference.fromProto(
        objectProto.spacePtr!,
        _session,
        _supergraph,
        _graph,
        _connection,
      ),
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
/* ==== DESTACK_GENERATED_END:NODE:1000000 ==== */
