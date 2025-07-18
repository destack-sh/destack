import { packProtoTimestamp, unpackProtoTimestamp } from "@destack/grpc";
import type {
  Branch,
  Graph,
  IsActor,
  NodeClass,
  NodeReference,
  QueryConnection,
  Session,
  Snapshot,
  Space,
  Supergraph,
} from "@destack/language/core";
import {
  ACTIVE_SNAPSHOT,
  ACTIVE_SPACE,
  ClientType,
  Entity,
  Event,
  Materialization,
  Node,
  NodeType,
  StructType,
} from "@destack/language/core";
import type { Machine } from "@destack/language/infrastructure";
import type { Cursor } from "@destack/language/logic";
import { STRUCT_CLASS_BY_TYPE, registerNodeClass } from "@destack/language/registry";
import type { User } from "@destack/language/universe/user";
import { ClientProto, ClientTypeProto, MaterializationProto } from "@destack/proto";
import { base64Decode } from "@destack/utils";
import { hashString } from "@destack/utils/hash";
import { Temporal } from "temporal-polyfill";

/* ==== DESTACK_GENERATED_START:NODE:121300 ==== */
/**
 * A Client to connect with the system.
 */
export class Client extends Entity {
  static metatype: NodeType = NodeType.CLIENT;

  /**
   * Client.parent
   */
  get parent(): (Entity & IsActor) | null {
    const nodePtr: NodeReference | null = this.parentPtr;
    if (nodePtr != null) {
      return this._supergraph.get(nodePtr.id) as (Entity & IsActor) | null;
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
   * The previous Entity this Entity is based on (from the base Snapshot).
   */
  get precededBy(): Client | null {
    const nodePtr: NodeReference | null = this.precededByPtr;
    if (nodePtr != null) {
      return this._supergraph.get(nodePtr.id) as Client | null;
    }
    return null;
  }
  readonly precededByPtr: NodeReference | null;

  /**
   * The (root) Entity that is being instantiated.
   */
  get instantiationRoot(): Entity | null {
    const nodePtr: NodeReference | null = this.instantiationRootPtr;
    if (nodePtr != null) {
      return this._supergraph.get(nodePtr.id) as Entity | null;
    }
    return null;
  }
  readonly instantiationRootPtr: NodeReference | null;

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
   * Client.browserVersion
   */
  /**
   * Client.browserVersion
   */
  get browserVersion(): string | null {
    return this._browserVersion;
  }
  set browserVersion(value: string | null) {
    const prop = (this.constructor as NodeClass).__properties__["browser_version"];
    this._session.updateSetProperty(this, prop, value);
    this._browserVersion = value;
  }
  _browserVersion: string | null;

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
   * Client.type
   */
  /**
   * Client.type
   */
  get type(): ClientType {
    return this._type;
  }
  set type(value: ClientType) {
    const prop = (this.constructor as NodeClass).__properties__["type"];
    this._session.updateSetProperty(this, prop, value);
    this._type = value;
  }
  _type: ClientType;

  /**
   * Client.machine
   */
  get machine(): Machine | null {
    const nodePtr: NodeReference | null = this.machinePtr;
    if (nodePtr != null) {
      return this._supergraph.get(nodePtr.id) as Machine | null;
    }
    return null;
  }
  set machine(node: Machine | null) {
    if (node === null) {
      this.machinePtr = null;
    } else {
      this.machinePtr = node.toRef();
    }
  }
  /**
   * Client.machine
   */
  get machinePtr(): NodeReference | null {
    return this._machinePtr;
  }
  set machinePtr(value: NodeReference | null) {
    const prop = (this.constructor as NodeClass).__properties__["machine"];
    this._session.updateSetProperty(this, prop, value);
    this._machinePtr = value;
  }
  _machinePtr: NodeReference | null;

  /**
   * Client.user
   */
  get user(): User | null {
    const nodePtr: NodeReference | null = this.userPtr;
    if (nodePtr != null) {
      return this._supergraph.get(nodePtr.id) as User | null;
    }
    return null;
  }
  set user(node: User | null) {
    if (node === null) {
      this.userPtr = null;
    } else {
      this.userPtr = node.toRef();
    }
  }
  /**
   * Client.user
   */
  get userPtr(): NodeReference | null {
    return this._userPtr;
  }
  set userPtr(value: NodeReference | null) {
    const prop = (this.constructor as NodeClass).__properties__["user"];
    this._session.updateSetProperty(this, prop, value);
    this._userPtr = value;
  }
  _userPtr: NodeReference | null;

  /**
   * Client.accessToken
   */
  /**
   * Client.accessToken
   */
  get accessToken(): string | null {
    return this._accessToken;
  }
  set accessToken(value: string | null) {
    const prop = (this.constructor as NodeClass).__properties__["access_token"];
    this._session.updateSetProperty(this, prop, value);
    this._accessToken = value;
  }
  _accessToken: string | null;

  /**
   * Client.seenAt
   */
  /**
   * Client.seenAt
   */
  get seenAt(): Temporal.ZonedDateTime | null {
    return this._seenAt;
  }
  set seenAt(value: Temporal.ZonedDateTime | null) {
    const prop = (this.constructor as NodeClass).__properties__["seen_at"];
    this._session.updateSetProperty(this, prop, value);
    this._seenAt = value;
  }
  _seenAt: Temporal.ZonedDateTime | null;

  /**
   * Client.loggedInAt
   */
  /**
   * Client.loggedInAt
   */
  get loggedInAt(): Temporal.ZonedDateTime | null {
    return this._loggedInAt;
  }
  set loggedInAt(value: Temporal.ZonedDateTime | null) {
    const prop = (this.constructor as NodeClass).__properties__["logged_in_at"];
    this._session.updateSetProperty(this, prop, value);
    this._loggedInAt = value;
  }
  _loggedInAt: Temporal.ZonedDateTime | null;

  /**
   * Client.cursor
   */
  get cursor(): Cursor | null {
    const nodePtr: NodeReference | null = this.cursorPtr;
    if (nodePtr != null) {
      return this._supergraph.get(nodePtr.id) as Cursor | null;
    }
    return null;
  }
  set cursor(node: Cursor | null) {
    if (node === null) {
      this.cursorPtr = null;
    } else {
      this.cursorPtr = node.toRef();
    }
  }
  /**
   * Client.cursor
   */
  get cursorPtr(): NodeReference | null {
    return this._cursorPtr;
  }
  set cursorPtr(value: NodeReference | null) {
    const prop = (this.constructor as NodeClass).__properties__["cursor"];
    this._session.updateSetProperty(this, prop, value);
    this._cursorPtr = value;
  }
  _cursorPtr: NodeReference | null;

  /**
   * Client.deviceType
   */
  /**
   * Client.deviceType
   */
  get deviceType(): string | null {
    return this._deviceType;
  }
  set deviceType(value: string | null) {
    const prop = (this.constructor as NodeClass).__properties__["device_type"];
    this._session.updateSetProperty(this, prop, value);
    this._deviceType = value;
  }
  _deviceType: string | null;

  /**
   * Client.deviceName
   */
  /**
   * Client.deviceName
   */
  get deviceName(): string | null {
    return this._deviceName;
  }
  set deviceName(value: string | null) {
    const prop = (this.constructor as NodeClass).__properties__["device_name"];
    this._session.updateSetProperty(this, prop, value);
    this._deviceName = value;
  }
  _deviceName: string | null;

  /**
   * Client.operatingSystem
   */
  /**
   * Client.operatingSystem
   */
  get operatingSystem(): string | null {
    return this._operatingSystem;
  }
  set operatingSystem(value: string | null) {
    const prop = (this.constructor as NodeClass).__properties__["operating_system"];
    this._session.updateSetProperty(this, prop, value);
    this._operatingSystem = value;
  }
  _operatingSystem: string | null;

  /**
   * Client.browserName
   */
  /**
   * Client.browserName
   */
  get browserName(): string | null {
    return this._browserName;
  }
  set browserName(value: string | null) {
    const prop = (this.constructor as NodeClass).__properties__["browser_name"];
    this._session.updateSetProperty(this, prop, value);
    this._browserName = value;
  }
  _browserName: string | null;

  constructor(options: {
    id?: string;
    parent?: (Entity & IsActor) | NodeReference | null;
    space?: Space | NodeReference;
    materialization?: Materialization;
    definition?: Entity | NodeReference | null;
    branch?: Branch | NodeReference;
    snapshot?: Snapshot | NodeReference;
    precededBy?: Client | NodeReference | null;
    instantiationRoot?: Entity | NodeReference | null;
    createdAt?: Temporal.ZonedDateTime;
    createdEpoch?: number;
    createdBy?: (Entity & IsActor) | NodeReference | null;
    updatedAt?: Temporal.ZonedDateTime;
    updatedEpoch?: number;
    updatedBy?: (Entity & IsActor) | NodeReference | null;
    deletedAt?: Temporal.ZonedDateTime | null;
    browserVersion?: string | null;
    name?: string;
    type: ClientType;
    machine?: Machine | NodeReference | null;
    user?: User | NodeReference | null;
    accessToken?: string | null;
    seenAt?: Temporal.ZonedDateTime | null;
    loggedInAt?: Temporal.ZonedDateTime | null;
    cursor?: Cursor | NodeReference | null;
    deviceType?: string | null;
    deviceName?: string | null;
    operatingSystem?: string | null;
    browserName?: string | null;
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
        throw new Error(`no active Space for Client`);
      }
      _space = _space.toRef();
    }
    if (_space === null) {
      throw new Error(`Client.space is required`);
    }
    this.spacePtr = _space;
    let _materialization = options.materialization ?? null;
    if (_materialization === null) {
      _materialization = 11 /* Materialization.ROOT */;
    }
    if (_materialization === null) {
      throw new Error(`Client.materialization is required`);
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
        throw new Error(`no active Branch for Client`);
      }
      _branch = _branch.toRef();
    }
    if (_branch === null) {
      throw new Error(`Client.branch is required`);
    }
    this.branchPtr = _branch;
    let _snapshot = options.snapshot ?? null;
    if (_snapshot != null && _snapshot.metatype != StructType.NODE_REFERENCE) {
      _snapshot = (_snapshot as Node).toRef();
    }
    if (_snapshot === null) {
      _snapshot = ACTIVE_SNAPSHOT.get();
      if (_snapshot === null) {
        throw new Error(`no active Snapshot for Client`);
      }
      _snapshot = _snapshot.toRef();
    }
    if (_snapshot === null) {
      throw new Error(`Client.snapshot is required`);
    }
    this.snapshotPtr = _snapshot;
    let _precededBy = options.precededBy ?? null;
    if (_precededBy != null && _precededBy.metatype != StructType.NODE_REFERENCE) {
      _precededBy = (_precededBy as Node).toRef();
    }
    this.precededByPtr = _precededBy;
    let _instantiationRoot = options.instantiationRoot ?? null;
    if (_instantiationRoot != null && _instantiationRoot.metatype != StructType.NODE_REFERENCE) {
      _instantiationRoot = (_instantiationRoot as Node).toRef();
    }
    this.instantiationRootPtr = _instantiationRoot;
    let _deletedAt = options.deletedAt ?? null;
    this.deletedAt = _deletedAt;
    let _browserVersion = options.browserVersion ?? null;
    this._browserVersion = _browserVersion;
    let _name = options.name ?? null;
    if (_name === null) {
      _name = "Client";
    }
    if (_name === null) {
      throw new Error(`Client.name is required`);
    }
    this._name = _name;
    let _type = options.type;
    if (_type === null) {
      throw new Error(`Client.type is required`);
    }
    this._type = _type;
    let _machine = options.machine ?? null;
    if (_machine != null && _machine.metatype != StructType.NODE_REFERENCE) {
      _machine = (_machine as Node).toRef();
    }
    this._machinePtr = _machine;
    let _user = options.user ?? null;
    if (_user != null && _user.metatype != StructType.NODE_REFERENCE) {
      _user = (_user as Node).toRef();
    }
    this._userPtr = _user;
    let _accessToken = options.accessToken ?? null;
    this._accessToken = _accessToken;
    let _seenAt = options.seenAt ?? null;
    this._seenAt = _seenAt;
    let _loggedInAt = options.loggedInAt ?? null;
    this._loggedInAt = _loggedInAt;
    let _cursor = options.cursor ?? null;
    if (_cursor != null && _cursor.metatype != StructType.NODE_REFERENCE) {
      _cursor = (_cursor as Node).toRef();
    }
    this._cursorPtr = _cursor;
    let _deviceType = options.deviceType ?? null;
    this._deviceType = _deviceType;
    let _deviceName = options.deviceName ?? null;
    this._deviceName = _deviceName;
    let _operatingSystem = options.operatingSystem ?? null;
    this._operatingSystem = _operatingSystem;
    let _browserName = options.browserName ?? null;
    this._browserName = _browserName;

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
        throw new Error(`Client.createdAt and Client.updatedAt are required for existing Nodes`);
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
    if (!(this._type === other._type)) {
      return false;
    }
    if (!(this._machinePtr?.id === other._machinePtr?.id)) {
      return false;
    }
    if (!(this._userPtr?.id === other._userPtr?.id)) {
      return false;
    }
    if (!(this._accessToken === other._accessToken)) {
      return false;
    }
    if (!(this._seenAt === other._seenAt)) {
      return false;
    }
    if (!(this._loggedInAt === other._loggedInAt)) {
      return false;
    }
    if (!(this._cursorPtr?.id === other._cursorPtr?.id)) {
      return false;
    }
    if (!(this._deviceType === other._deviceType)) {
      return false;
    }
    if (!(this._deviceName === other._deviceName)) {
      return false;
    }
    if (!(this._operatingSystem === other._operatingSystem)) {
      return false;
    }
    if (!(this._browserName === other._browserName)) {
      return false;
    }
    if (!(this._browserVersion === other._browserVersion)) {
      return false;
    }
    if (!(this.definitionPtr?.id === other.definitionPtr?.id)) {
      return false;
    }
    if (!(this._name === other._name)) {
      return false;
    }
    if (!(this.spacePtr.id === other.spacePtr.id)) {
      return false;
    }
    return true;
  }

  hash(): number {
    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    if (this.parentPtr != null) {
      h = (h * 31 + hashString(this.parentPtr.id)) & 0xffffffff;
    }
    h = (h * 31 + this._type) & 0xffffffff;
    if (this._machinePtr != null) {
      h = (h * 31 + hashString(this._machinePtr.id)) & 0xffffffff;
    }
    if (this._userPtr != null) {
      h = (h * 31 + hashString(this._userPtr.id)) & 0xffffffff;
    }
    if (this._accessToken != null) {
      h = (h * 31 + hashString(this._accessToken)) & 0xffffffff;
    }
    if (this._seenAt != null) {
      h = (h * 31 + hashString(this._seenAt.toString({ timeZoneName: "never" }))) & 0xffffffff;
    }
    if (this._loggedInAt != null) {
      h = (h * 31 + hashString(this._loggedInAt.toString({ timeZoneName: "never" }))) & 0xffffffff;
    }
    if (this._cursorPtr != null) {
      h = (h * 31 + hashString(this._cursorPtr.id)) & 0xffffffff;
    }
    if (this._deviceType != null) {
      h = (h * 31 + hashString(this._deviceType)) & 0xffffffff;
    }
    if (this._deviceName != null) {
      h = (h * 31 + hashString(this._deviceName)) & 0xffffffff;
    }
    if (this._operatingSystem != null) {
      h = (h * 31 + hashString(this._operatingSystem)) & 0xffffffff;
    }
    if (this._browserName != null) {
      h = (h * 31 + hashString(this._browserName)) & 0xffffffff;
    }
    if (this._browserVersion != null) {
      h = (h * 31 + hashString(this._browserVersion)) & 0xffffffff;
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
    h = (h * 31 + hashString(this.id.toString())) & 0xffffffff;
    h = (h * 31 + hashString(this.spacePtr.id)) & 0xffffffff;

    return h;
  }

  validate(): void {
    throw new Error("not implemented");
  }

  __toRef__(): NodeReference {
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    return new _NodeReference({
      type: NodeType.CLIENT,
      id: this.id,
      spaceId: this.spacePtr?.id ?? null,
      snapshotId: this.snapshotPtr?.id ?? null,
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
    propertyReprs.push(`type=${ClientType[this.type]}`);
    propertyReprs.push(`name=${`"${this.name}"`}`);
    return `<Client "${this.path}" ${propertyReprs.join(" ")}>`;
  }

  toValue(): { readonly [key: string]: any } {
    return Client.__packValue__(this);
  }

  static __packValue__(object: Client): { readonly [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 121300;
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
    if (object.instantiationRootPtr != null) {
      objectValue["15"] = object.instantiationRootPtr.toValue();
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
    if (object._browserVersion != null) {
      objectValue["44"] = object._browserVersion;
    }
    objectValue["50"] = object._name;
    objectValue["100"] = object._type;
    if (object._machinePtr != null) {
      objectValue["110"] = object._machinePtr.toValue();
    }
    if (object._userPtr != null) {
      objectValue["111"] = object._userPtr.toValue();
    }
    if (object._accessToken != null) {
      objectValue["120"] = object._accessToken;
    }
    if (object._seenAt != null) {
      objectValue["121"] = object._seenAt.toString({ timeZoneName: "never" });
    }
    if (object._loggedInAt != null) {
      objectValue["122"] = object._loggedInAt.toString({ timeZoneName: "never" });
    }
    if (object._cursorPtr != null) {
      objectValue["123"] = object._cursorPtr.toValue();
    }
    if (object._deviceType != null) {
      objectValue["130"] = object._deviceType;
    }
    if (object._deviceName != null) {
      objectValue["131"] = object._deviceName;
    }
    if (object._operatingSystem != null) {
      objectValue["132"] = object._operatingSystem;
    }
    if (object._browserName != null) {
      objectValue["133"] = object._browserName;
    }
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { readonly [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Client {
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    const parentPtrValue = objectValue["3"];
    const unpackedParentPtr =
      parentPtrValue != undefined
        ? _NodeReference.fromValue(parentPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const machinePtrValue = objectValue["110"];
    const unpackedMachinePtr =
      machinePtrValue != undefined
        ? _NodeReference.fromValue(machinePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const userPtrValue = objectValue["111"];
    const unpackedUserPtr =
      userPtrValue != undefined
        ? _NodeReference.fromValue(userPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const accessTokenValue = objectValue["120"];
    const unpackedAccessToken = accessTokenValue != undefined ? accessTokenValue : null;
    const seenAtValue = objectValue["121"];
    const unpackedSeenAt =
      seenAtValue != undefined
        ? Temporal.Instant.from(seenAtValue).toZonedDateTimeISO("UTC")
        : null;
    const loggedInAtValue = objectValue["122"];
    const unpackedLoggedInAt =
      loggedInAtValue != undefined
        ? Temporal.Instant.from(loggedInAtValue).toZonedDateTimeISO("UTC")
        : null;
    const cursorPtrValue = objectValue["123"];
    const unpackedCursorPtr =
      cursorPtrValue != undefined
        ? _NodeReference.fromValue(cursorPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const deviceTypeValue = objectValue["130"];
    const unpackedDeviceType = deviceTypeValue != undefined ? deviceTypeValue : null;
    const deviceNameValue = objectValue["131"];
    const unpackedDeviceName = deviceNameValue != undefined ? deviceNameValue : null;
    const operatingSystemValue = objectValue["132"];
    const unpackedOperatingSystem = operatingSystemValue != undefined ? operatingSystemValue : null;
    const browserNameValue = objectValue["133"];
    const unpackedBrowserName = browserNameValue != undefined ? browserNameValue : null;
    const browserVersionValue = objectValue["44"];
    const unpackedBrowserVersion = browserVersionValue != undefined ? browserVersionValue : null;
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
    const instantiationRootPtrValue = objectValue["15"];
    const unpackedInstantiationRootPtr =
      instantiationRootPtrValue != undefined
        ? _NodeReference.fromValue(
            instantiationRootPtrValue,
            _session,
            _supergraph,
            _graph,
            _connection,
          )
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
    return new Client({
      parent: unpackedParentPtr,
      type: Number(objectValue["100"]),
      machine: unpackedMachinePtr,
      user: unpackedUserPtr,
      accessToken: unpackedAccessToken,
      seenAt: unpackedSeenAt,
      loggedInAt: unpackedLoggedInAt,
      cursor: unpackedCursorPtr,
      deviceType: unpackedDeviceType,
      deviceName: unpackedDeviceName,
      operatingSystem: unpackedOperatingSystem,
      browserName: unpackedBrowserName,
      browserVersion: unpackedBrowserVersion,
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
      instantiationRoot: unpackedInstantiationRootPtr,
      createdAt: Temporal.Instant.from(objectValue["20"]).toZonedDateTimeISO("UTC"),
      createdEpoch: Number(objectValue["21"]),
      createdBy: unpackedCreatedByPtr,
      updatedAt: Temporal.Instant.from(objectValue["23"]).toZonedDateTimeISO("UTC"),
      updatedEpoch: Number(objectValue["24"]),
      updatedBy: unpackedUpdatedByPtr,
      deletedAt: unpackedDeletedAt,
      name: objectValue["50"],
      id: String(objectValue["2"]),
      space: _NodeReference.fromValue(objectValue["5"], _session, _supergraph, _graph, _connection),
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
  ): Client {
    return Client.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
  }

  toProto(): ClientProto {
    return Client.__packProto__(this);
  }

  static __packProto__(object: Client): ClientProto {
    const objectProto: Partial<ClientProto> = { metatype: 121300 };
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
    if (object.instantiationRootPtr != null) {
      objectProto.instantiationRootPtr = object.instantiationRootPtr.toProto();
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
    if (object._browserVersion != null) {
      objectProto.browserVersion = object._browserVersion;
    }
    objectProto.name = object._name;
    objectProto.type = Number(object._type) as ClientTypeProto;
    if (object._machinePtr != null) {
      objectProto.machinePtr = object._machinePtr.toProto();
    }
    if (object._userPtr != null) {
      objectProto.userPtr = object._userPtr.toProto();
    }
    if (object._accessToken != null) {
      objectProto.accessToken = object._accessToken;
    }
    if (object._seenAt != null) {
      objectProto.seenAt = packProtoTimestamp(object._seenAt);
    }
    if (object._loggedInAt != null) {
      objectProto.loggedInAt = packProtoTimestamp(object._loggedInAt);
    }
    if (object._cursorPtr != null) {
      objectProto.cursorPtr = object._cursorPtr.toProto();
    }
    if (object._deviceType != null) {
      objectProto.deviceType = object._deviceType;
    }
    if (object._deviceName != null) {
      objectProto.deviceName = object._deviceName;
    }
    if (object._operatingSystem != null) {
      objectProto.operatingSystem = object._operatingSystem;
    }
    if (object._browserName != null) {
      objectProto.browserName = object._browserName;
    }
    return objectProto as ClientProto;
  }

  static __unpackProto__(
    objectProto: ClientProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Client {
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    return new Client({
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
      type: Number(objectProto.type) as ClientType,
      machine:
        objectProto.machinePtr != undefined
          ? _NodeReference.fromProto(
              objectProto.machinePtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      user:
        objectProto.userPtr != undefined
          ? _NodeReference.fromProto(
              objectProto.userPtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      accessToken: objectProto.accessToken != undefined ? objectProto.accessToken : null,
      seenAt: objectProto.seenAt != undefined ? unpackProtoTimestamp(objectProto.seenAt!) : null,
      loggedInAt:
        objectProto.loggedInAt != undefined ? unpackProtoTimestamp(objectProto.loggedInAt!) : null,
      cursor:
        objectProto.cursorPtr != undefined
          ? _NodeReference.fromProto(
              objectProto.cursorPtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      deviceType: objectProto.deviceType != undefined ? objectProto.deviceType : null,
      deviceName: objectProto.deviceName != undefined ? objectProto.deviceName : null,
      operatingSystem:
        objectProto.operatingSystem != undefined ? objectProto.operatingSystem : null,
      browserName: objectProto.browserName != undefined ? objectProto.browserName : null,
      browserVersion: objectProto.browserVersion != undefined ? objectProto.browserVersion : null,
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
      instantiationRoot:
        objectProto.instantiationRootPtr != undefined
          ? _NodeReference.fromProto(
              objectProto.instantiationRootPtr!,
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
      id: String(objectProto.id),
      space: _NodeReference.fromProto(
        objectProto.spacePtr!,
        _session,
        _supergraph,
        _graph,
        _connection,
      ),
      _session,
      _graph,
      _connection,
    });
  }

  static fromProto(
    objectProto: ClientProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Client {
    return Client.__unpackProto__(objectProto, _session, _supergraph, _graph, _connection);
  }

  static fromProtoString(packedProtoString: string): Client {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = ClientProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerNodeClass(NodeType.CLIENT, Client);
/* ==== DESTACK_GENERATED_END:NODE:121300 ==== */
