import type {
  Branch,
  Datetime,
  NodeClass,
  NodeReference,
  Session,
  Snapshot,
  Space,
  UInt128,
  UUID,
  Value,
} from "@destack/language/core";
import {
  ACTIVE_BRANCH,
  ACTIVE_SNAPSHOT,
  ACTIVE_SPACE,
  ClientType,
  Entity,
  type Event,
  type Materialization,
  type Node,
  NodeType,
  StructType,
} from "@destack/language/core";
import type { Machine } from "@destack/language/infrastructure";
import type { Script } from "@destack/language/logic";
import { registerNodeClass, STRUCT_CLASS_BY_TYPE } from "@destack/language/registry";
import type { User } from "@destack/language/universe/user";
import { hashBool, hashString } from "@destack/utils/hash";
import { Temporal } from "temporal-polyfill";

/* ==== DESTACK_GENERATED_START:NODE:121300 ==== */
/**
 * A Client to connect with the system.
 */
export class Client extends Entity {
  static metatype: NodeType = NodeType.CLIENT;

  /**
   * The parent of this Entity. Most Entities can be attached to any other Entity.
   */
  get parent(): Entity | null {
    const nodeRef: NodeReference | null = this.parentRef;
    if (nodeRef != null) {
      return this._session.graph.get(nodeRef) as Entity | null;
    }
    return null;
  }
  readonly parentRef: NodeReference | null;

  /**
   * The Space this Node is in.
   */
  get space(): Space | null {
    const nodeRef: NodeReference | null = this.spaceRef;
    if (nodeRef != null) {
      return this._session.graph.get(nodeRef) as Space | null;
    }
    return null;
  }
  readonly spaceRef: NodeReference;

  /**
   * Entity.materialization
   */
  readonly materialization: Materialization;

  /**
   * The definition this Entity is an instance of.
   */
  get definition(): Entity | null {
    const nodeRef: NodeReference | null = this.definitionRef;
    if (nodeRef != null) {
      return this._session.graph.get(nodeRef) as Entity | null;
    }
    return null;
  }
  readonly definitionRef: NodeReference | null;

  /**
   * The Branch this Entity is part of.
   */
  get branch(): Branch | null {
    const nodeRef: NodeReference | null = this.branchRef;
    if (nodeRef != null) {
      return this._session.graph.get(nodeRef) as Branch | null;
    }
    return null;
  }
  readonly branchRef: NodeReference;

  /**
   * The Snapshot this Entity is part of.
   */
  get snapshot(): Snapshot | null {
    const nodeRef: NodeReference | null = this.snapshotRef;
    if (nodeRef != null) {
      return this._session.graph.get(nodeRef) as Snapshot | null;
    }
    return null;
  }
  readonly snapshotRef: NodeReference;

  /**
   * The previous Entity this Entity is based on (from the base Branch, if any).
   * This invariant must hold: `Entity.preceded_by.branch == Entity.branch.preceded_by`.
   */
  get precededBy(): Client | null {
    const nodeRef: NodeReference | null = this.precededByRef;
    if (nodeRef != null) {
      return this._session.graph.get(nodeRef) as Client | null;
    }
    return null;
  }
  readonly precededByRef: NodeReference | null;

  /**
   * The (root) Entity that is being instantiated.
   */
  get instance(): Entity | null {
    const nodeRef: NodeReference | null = this.instanceRef;
    if (nodeRef != null) {
      return this._session.graph.get(nodeRef) as Entity | null;
    }
    return null;
  }
  readonly instanceRef: NodeReference | null;

  /**
   * The time this Entity was created (system time).
   */
  readonly createdAt: Datetime;

  /**
   * The logical time this Entity was created (system time).
   */
  readonly createdEpoch: UInt128;

  /**
   * The Actor that created this Entity.
   */
  get createdBy(): Entity | null {
    const nodeRef: NodeReference | null = this.createdByRef;
    if (nodeRef != null) {
      return this._session.graph.get(nodeRef) as Entity | null;
    }
    return null;
  }
  readonly createdByRef: NodeReference;

  /**
   * The time this Entity was last updated (system time).
   */
  readonly updatedAt: Datetime;

  /**
   * The logical time this Entity was last updated (system time).
   */
  readonly updatedEpoch: UInt128;

  /**
   * The Actor that last updated this Entity.
   */
  get updatedBy(): Entity | null {
    const nodeRef: NodeReference | null = this.updatedByRef;
    if (nodeRef != null) {
      return this._session.graph.get(nodeRef) as Entity | null;
    }
    return null;
  }
  readonly updatedByRef: NodeReference;

  /**
   * The time this Entity was deleted (system time).
   * Only set if the Entity is currently 'deleted'.
   * Deleting and restoring an Entity counts as an update, and thus updates updated_at/updated_epoch.
   */
  readonly deletedAt: Datetime | null;

  /**
   * Entity.ownedBy
   */
  get ownedBy(): Entity | null {
    const nodeRef: NodeReference | null = this.ownedByRef;
    if (nodeRef != null) {
      return this._session.graph.get(nodeRef) as Entity | null;
    }
    return null;
  }
  set ownedBy(node: Entity | null) {
    if (node === null) {
      this.ownedByRef = null;
    } else {
      this.ownedByRef = node.toRef();
    }
  }
  /**
   * Entity.ownedBy
   */
  get ownedByRef(): NodeReference | null {
    return this._ownedByRef;
  }
  set ownedByRef(value: NodeReference | null) {
    const prop = (this.constructor as NodeClass).__properties__["owned_by"];
    this._session.updateSetProperty(this, prop, value);
    this._ownedByRef = value;
  }
  _ownedByRef: NodeReference | null;

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
   * The absolute order key of this Entity in its parent.
   */
  readonly orderKey: string;

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
   * The custom Values of this Entity, keyed by custom Property id..
   */
  /**
   * The custom Values of this Entity, keyed by custom Property id..
   */
  get customValues(): { readonly [key: UUID]: Value } {
    return this._customValues;
  }
  set customValues(value: { readonly [key: UUID]: Value }) {
    const prop = (this.constructor as NodeClass).__properties__["custom_values"];
    this._session.updateSetProperty(this, prop, value);
    this._customValues = value;
  }
  _customValues: { readonly [key: UUID]: Value };

  /**
   * The Script of this Entity.
   */
  get script(): Script | null {
    const nodeRef: NodeReference | null = this.scriptRef;
    if (nodeRef != null) {
      return this._session.graph.get(nodeRef) as Script | null;
    }
    return null;
  }
  set script(node: Script | null) {
    if (node === null) {
      this.scriptRef = null;
    } else {
      this.scriptRef = node.toRef();
    }
  }
  /**
   * The Script of this Entity.
   */
  get scriptRef(): NodeReference | null {
    return this._scriptRef;
  }
  set scriptRef(value: NodeReference | null) {
    const prop = (this.constructor as NodeClass).__properties__["script"];
    this._session.updateSetProperty(this, prop, value);
    this._scriptRef = value;
  }
  _scriptRef: NodeReference | null;

  /**
   * Whether this Entity can be instanced.
   */
  readonly isExtensible: boolean | null;

  /**
   * The Script that defines this Node.
   */
  get source(): Script | null {
    const nodeRef: NodeReference | null = this.sourceRef;
    if (nodeRef != null) {
      return this._session.graph.get(nodeRef) as Script | null;
    }
    return null;
  }
  readonly sourceRef: NodeReference | null;

  /**
   * The key to uniquely identify this Node in reconciliation. If not set, name is used.
   */
  /**
   * The key to uniquely identify this Node in reconciliation. If not set, name is used.
   */
  get key(): string | null {
    return this._key;
  }
  set key(value: string | null) {
    const prop = (this.constructor as NodeClass).__properties__["key"];
    this._session.updateSetProperty(this, prop, value);
    this._key = value;
  }
  _key: string | null;

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
    const nodeRef: NodeReference | null = this.machineRef;
    if (nodeRef != null) {
      return this._session.graph.get(nodeRef) as Machine | null;
    }
    return null;
  }
  set machine(node: Machine | null) {
    if (node === null) {
      this.machineRef = null;
    } else {
      this.machineRef = node.toRef();
    }
  }
  /**
   * Client.machine
   */
  get machineRef(): NodeReference | null {
    return this._machineRef;
  }
  set machineRef(value: NodeReference | null) {
    const prop = (this.constructor as NodeClass).__properties__["machine"];
    this._session.updateSetProperty(this, prop, value);
    this._machineRef = value;
  }
  _machineRef: NodeReference | null;

  /**
   * Client.user
   */
  get user(): User | null {
    const nodeRef: NodeReference | null = this.userRef;
    if (nodeRef != null) {
      return this._session.graph.get(nodeRef) as User | null;
    }
    return null;
  }
  set user(node: User | null) {
    if (node === null) {
      this.userRef = null;
    } else {
      this.userRef = node.toRef();
    }
  }
  /**
   * Client.user
   */
  get userRef(): NodeReference | null {
    return this._userRef;
  }
  set userRef(value: NodeReference | null) {
    const prop = (this.constructor as NodeClass).__properties__["user"];
    this._session.updateSetProperty(this, prop, value);
    this._userRef = value;
  }
  _userRef: NodeReference | null;

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
  get seenAt(): Datetime | null {
    return this._seenAt;
  }
  set seenAt(value: Datetime | null) {
    const prop = (this.constructor as NodeClass).__properties__["seen_at"];
    this._session.updateSetProperty(this, prop, value);
    this._seenAt = value;
  }
  _seenAt: Datetime | null;

  /**
   * Client.loggedInAt
   */
  /**
   * Client.loggedInAt
   */
  get loggedInAt(): Datetime | null {
    return this._loggedInAt;
  }
  set loggedInAt(value: Datetime | null) {
    const prop = (this.constructor as NodeClass).__properties__["logged_in_at"];
    this._session.updateSetProperty(this, prop, value);
    this._loggedInAt = value;
  }
  _loggedInAt: Datetime | null;

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
    id?: UUID;
    parent?: Entity | NodeReference | null;
    space?: Space | NodeReference;
    materialization?: Materialization;
    definition?: Entity | NodeReference | null;
    branch?: Branch | NodeReference;
    snapshot?: Snapshot | NodeReference;
    precededBy?: Client | NodeReference | null;
    instance?: Entity | NodeReference | null;
    createdAt?: Datetime;
    createdEpoch?: UInt128;
    createdBy?: Entity | NodeReference;
    updatedAt?: Datetime;
    updatedEpoch?: UInt128;
    updatedBy?: Entity | NodeReference;
    deletedAt?: Datetime | null;
    ownedBy?: Entity | NodeReference | null;
    name?: string;
    orderKey?: string;
    browserVersion?: string | null;
    customValues?: { readonly [key: UUID]: Value };
    script?: Script | NodeReference | null;
    isExtensible?: boolean | null;
    source?: Script | NodeReference | null;
    key?: string | null;
    type: ClientType;
    machine?: Machine | NodeReference | null;
    user?: User | NodeReference | null;
    accessToken?: string | null;
    seenAt?: Datetime | null;
    loggedInAt?: Datetime | null;
    deviceType?: string | null;
    deviceName?: string | null;
    operatingSystem?: string | null;
    browserName?: string | null;
    _session?: Session | null;
  }) {
    /* super */
    super(
      /* id */
      options.id ?? null,
      /* parent */
      options.parent != null ? options.parent.toRef() : null,
      /* session */
      options._session ?? null,
      /* _isNew */
      options.id == null,
    );

    /* properties */
    let _parent = options.parent ?? null;
    if (_parent != null && _parent.constructor.name !== "NodeReference") {
      _parent = (_parent as Node).toRef();
    }
    this.parentRef = _parent as NodeReference | null;
    let _space = options.space ?? null;
    if (_space != null && _space.constructor.name !== "NodeReference") {
      _space = (_space as Node).toRef();
    }
    if (_space == null) {
      _space = ACTIVE_SPACE.get();
      if (_space == null) {
        throw new Error(`no active Space for Client`);
      }
      _space = _space.toRef();
    }
    if (_space == null) {
      throw new Error(`Client.space is required`);
    }
    this.spaceRef = _space as NodeReference;
    let _materialization = options.materialization ?? null;
    if (_materialization == null) {
      _materialization = 11 /* Materialization.ROOT */;
    }
    if (_materialization == null) {
      throw new Error(`Client.materialization is required`);
    }
    this.materialization = _materialization;
    let _definition = options.definition ?? null;
    if (_definition != null && _definition.constructor.name !== "NodeReference") {
      _definition = (_definition as Node).toRef();
    }
    this.definitionRef = _definition as NodeReference | null;
    let _branch = options.branch ?? null;
    if (_branch != null && _branch.constructor.name !== "NodeReference") {
      _branch = (_branch as Node).toRef();
    }
    if (_branch == null) {
      _branch = ACTIVE_BRANCH.get();
      if (_branch == null) {
        throw new Error(`no active Branch for Client`);
      }
      _branch = _branch.toRef();
    }
    if (_branch == null) {
      throw new Error(`Client.branch is required`);
    }
    this.branchRef = _branch as NodeReference;
    let _snapshot = options.snapshot ?? null;
    if (_snapshot != null && _snapshot.constructor.name !== "NodeReference") {
      _snapshot = (_snapshot as Node).toRef();
    }
    if (_snapshot == null) {
      _snapshot = ACTIVE_SNAPSHOT.get();
      if (_snapshot == null) {
        throw new Error(`no active Snapshot for Client`);
      }
      _snapshot = _snapshot.toRef();
    }
    if (_snapshot == null) {
      throw new Error(`Client.snapshot is required`);
    }
    this.snapshotRef = _snapshot as NodeReference;
    let _precededBy = options.precededBy ?? null;
    if (_precededBy != null && _precededBy.constructor.name !== "NodeReference") {
      _precededBy = (_precededBy as Node).toRef();
    }
    this.precededByRef = _precededBy as NodeReference | null;
    let _instance = options.instance ?? null;
    if (_instance != null && _instance.constructor.name !== "NodeReference") {
      _instance = (_instance as Node).toRef();
    }
    this.instanceRef = _instance as NodeReference | null;
    let _deletedAt = options.deletedAt ?? null;
    this.deletedAt = _deletedAt;
    let _ownedBy = options.ownedBy ?? null;
    if (_ownedBy != null && _ownedBy.constructor.name !== "NodeReference") {
      _ownedBy = (_ownedBy as Node).toRef();
    }
    this._ownedByRef = _ownedBy as NodeReference | null;
    let _name = options.name ?? null;
    if (_name == null) {
      _name = "Client";
    }
    if (_name == null) {
      throw new Error(`Client.name is required`);
    }
    this._name = _name;
    let _orderKey = options.orderKey ?? null;
    if (_orderKey == null) {
      _orderKey = "a0";
    }
    if (_orderKey == null) {
      throw new Error(`Client.orderKey is required`);
    }
    this.orderKey = _orderKey;
    let _browserVersion = options.browserVersion ?? null;
    this._browserVersion = _browserVersion;
    let _customValues = options.customValues ?? null;
    if (_customValues == null) {
      _customValues = {};
    }
    this._customValues = _customValues;
    let _script = options.script ?? null;
    if (_script != null && _script.constructor.name !== "NodeReference") {
      _script = (_script as Node).toRef();
    }
    this._scriptRef = _script as NodeReference | null;
    let _isExtensible = options.isExtensible ?? null;
    this.isExtensible = _isExtensible;
    let _source = options.source ?? null;
    if (_source != null && _source.constructor.name !== "NodeReference") {
      _source = (_source as Node).toRef();
    }
    this.sourceRef = _source as NodeReference | null;
    let _key = options.key ?? null;
    this._key = _key;
    let _type = options.type;
    if (_type == null) {
      throw new Error(`Client.type is required`);
    }
    this._type = _type;
    let _machine = options.machine ?? null;
    if (_machine != null && _machine.constructor.name !== "NodeReference") {
      _machine = (_machine as Node).toRef();
    }
    this._machineRef = _machine as NodeReference | null;
    let _user = options.user ?? null;
    if (_user != null && _user.constructor.name !== "NodeReference") {
      _user = (_user as Node).toRef();
    }
    this._userRef = _user as NodeReference | null;
    let _accessToken = options.accessToken ?? null;
    this._accessToken = _accessToken;
    let _seenAt = options.seenAt ?? null;
    this._seenAt = _seenAt;
    let _loggedInAt = options.loggedInAt ?? null;
    this._loggedInAt = _loggedInAt;
    let _deviceType = options.deviceType ?? null;
    this._deviceType = _deviceType;
    let _deviceName = options.deviceName ?? null;
    this._deviceName = _deviceName;
    let _operatingSystem = options.operatingSystem ?? null;
    this._operatingSystem = _operatingSystem;
    let _browserName = options.browserName ?? null;
    this._browserName = _browserName;

    /* identity */
    if (options.id == null) {
      const now = Temporal.Now.zonedDateTimeISO("UTC");
      const epoch = this._session.remoteEpoch;
      this.createdAt = now;
      this.createdEpoch = epoch;
      this.createdByRef = this._session.actorRef;
      this.updatedAt = now;
      this.updatedEpoch = epoch;
      this.updatedByRef = this._session.actorRef;
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
      this.createdByRef =
        options.createdBy != null ? options.createdBy.toRef() : this._session.actorRef;
      this.updatedAt = options.updatedAt;
      this.updatedEpoch = options.updatedEpoch;
      this.updatedByRef =
        options.updatedBy != null ? options.updatedBy.toRef() : this._session.actorRef;
    }
  }

  equals(other: any): boolean {
    if (!(this.metatype === other.metatype)) {
      return false;
    }
    if (!(this._type === other._type)) {
      return false;
    }
    if (!(this._machineRef?.id === other._machineRef?.id)) {
      return false;
    }
    if (!(this._userRef?.id === other._userRef?.id)) {
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
    if (!(this.definitionRef?.id === other.definitionRef?.id)) {
      return false;
    }
    if (!(this._ownedByRef?.id === other._ownedByRef?.id)) {
      return false;
    }
    if (!(this._name === other._name)) {
      return false;
    }
    if (JSON.stringify(this._customValues) !== JSON.stringify(other._customValues)) {
      return false;
    }
    if (!(this._scriptRef?.id === other._scriptRef?.id)) {
      return false;
    }
    if (!(this.isExtensible === other.isExtensible)) {
      return false;
    }
    if (!(this.sourceRef?.id === other.sourceRef?.id)) {
      return false;
    }
    if (!(this._key === other._key)) {
      return false;
    }
    if (!(this.spaceRef.id === other.spaceRef.id)) {
      return false;
    }
    return true;
  }

  hash(): number {
    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    h = (h * 31 + this._type) & 0xffffffff;
    if (this._machineRef != null) {
      h = (h * 31 + hashString(this._machineRef.id)) & 0xffffffff;
    }
    if (this._userRef != null) {
      h = (h * 31 + hashString(this._userRef.id)) & 0xffffffff;
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
    if (this.parentRef != null) {
      h = (h * 31 + hashString(this.parentRef.id)) & 0xffffffff;
    }
    if (this.definitionRef != null) {
      h = (h * 31 + hashString(this.definitionRef.id)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.createdAt.toString({ timeZoneName: "never" }))) & 0xffffffff;
    h = (h * 31 + hashString(this.createdByRef.id)) & 0xffffffff;
    h = (h * 31 + hashString(this.updatedAt.toString({ timeZoneName: "never" }))) & 0xffffffff;
    h = (h * 31 + hashString(this.updatedByRef.id)) & 0xffffffff;
    if (this.deletedAt != null) {
      h = (h * 31 + hashString(this.deletedAt.toString({ timeZoneName: "never" }))) & 0xffffffff;
    }
    if (this._ownedByRef != null) {
      h = (h * 31 + hashString(this._ownedByRef.id)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this._name)) & 0xffffffff;
    h = (h * 31 + hashString(this.orderKey)) & 0xffffffff;
    if (this._customValues && Object.keys(this._customValues).length > 0) {
      for (const [_key, _value] of Object.entries(this._customValues)) {
        h = (h * 31 + hashString(_key.toString())) & 0xffffffff;
        h = (h * 31 + _value.hash()) & 0xffffffff;
      }
    }
    if (this._scriptRef != null) {
      h = (h * 31 + hashString(this._scriptRef.id)) & 0xffffffff;
    }
    if (this.isExtensible != null) {
      h = (h * 31 + hashBool(this.isExtensible)) & 0xffffffff;
    }
    if (this.sourceRef != null) {
      h = (h * 31 + hashString(this.sourceRef.id)) & 0xffffffff;
    }
    if (this._key != null) {
      h = (h * 31 + hashString(this._key)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.id.toString())) & 0xffffffff;
    h = (h * 31 + hashString(this.spaceRef.id)) & 0xffffffff;

    return h;
  }

  __toRef__(): NodeReference {
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    return new _NodeReference({
      type: NodeType.CLIENT,
      id: this.id,
      spaceId: this.spaceRef.id,
      definitionId: this.definitionRef?.id ?? null,
      branchId: this.branchRef.id,
      snapshotId: this.snapshotRef.id,
      _session: this._session,
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
    if (this.ownedBy != null) {
      propertyReprs.push(`ownedBy=${this.ownedBy?.repr()}`);
    }
    propertyReprs.push(`name=${`"${this.name}"`}`);
    return `<Client "${this.path}" ${propertyReprs.join(" ")}>`;
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerNodeClass(NodeType.CLIENT, Client);
/* ==== DESTACK_GENERATED_END:NODE:121300 ==== */
