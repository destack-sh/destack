import { packProtoTimestamp, unpackProtoTimestamp } from "@destack/grpc";
import type {
  Branch,
  Graph,
  IsActor,
  IsFollowable,
  IsScriptable,
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
  StructType,
} from "@destack/language/core";
import type { Cursor, Script } from "@destack/language/logic";
import {
  STRUCT_CLASS_BY_TYPE,
  registerEnumClass,
  registerNodeClass,
} from "@destack/language/registry";
import type { Handle } from "@destack/language/universe/handle";
import { MaterializationProto, UserProto, UserStatusProto } from "@destack/proto";
import { base64Decode, base64Encode } from "@destack/utils";
import { hashBool, hashBytes, hashString } from "@destack/utils/hash";
import { Temporal } from "temporal-polyfill";

/* ==== DESTACK_GENERATED_START:ENUM:101000 ==== */
/**
 * UserStatus
 */
export enum UserStatus {
  CREATING = 2,
  ACTIVE = 10,

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerEnumClass(EnumType.USER_STATUS, UserStatus);
/* ==== DESTACK_GENERATED_END:ENUM:101000 ==== */

/* ==== DESTACK_GENERATED_START:NODE:121000 ==== */
/**
 * A User is a human using Destack.
 */
export class User extends Entity implements IsActor, IsFollowable, IsScriptable {
  static metatype: NodeType = NodeType.USER;

  /**
   * User.parent
   */
  get parent(): Space | null {
    const nodePtr: NodeReference | null = this.parentPtr;
    if (nodePtr != null) {
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
  get precededBy(): User | null {
    const nodePtr: NodeReference | null = this.precededByPtr;
    if (nodePtr != null) {
      return this._supergraph.get(nodePtr.id) as User | null;
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
   * User.slug
   */
  /**
   * User.slug
   */
  get slug(): string {
    return this._slug;
  }
  set slug(value: string) {
    const prop = (this.constructor as NodeClass).__properties__["slug"];
    this._session.updateSetProperty(this, prop, value);
    this._slug = value;
  }
  _slug: string;

  /**
   * User.status
   */
  /**
   * User.status
   */
  get status(): UserStatus {
    return this._status;
  }
  set status(value: UserStatus) {
    const prop = (this.constructor as NodeClass).__properties__["status"];
    this._session.updateSetProperty(this, prop, value);
    this._status = value;
  }
  _status: UserStatus;

  /**
   * User.lastLoggedInAt
   */
  /**
   * User.lastLoggedInAt
   */
  get lastLoggedInAt(): Temporal.ZonedDateTime | null {
    return this._lastLoggedInAt;
  }
  set lastLoggedInAt(value: Temporal.ZonedDateTime | null) {
    const prop = (this.constructor as NodeClass).__properties__["last_logged_in_at"];
    this._session.updateSetProperty(this, prop, value);
    this._lastLoggedInAt = value;
  }
  _lastLoggedInAt: Temporal.ZonedDateTime | null;

  /**
   * User.isStaff
   */
  /**
   * User.isStaff
   */
  get isStaff(): boolean {
    return this._isStaff;
  }
  set isStaff(value: boolean) {
    const prop = (this.constructor as NodeClass).__properties__["is_staff"];
    this._session.updateSetProperty(this, prop, value);
    this._isStaff = value;
  }
  _isStaff: boolean;

  /**
   * User.handle
   */
  get handle(): Handle | null {
    const nodePtr: NodeReference | null = this.handlePtr;
    if (nodePtr != null) {
      return this._supergraph.get(nodePtr.id) as Handle | null;
    }
    return null;
  }
  set handle(node: Handle | null) {
    if (node === null) {
      this.handlePtr = null;
    } else {
      this.handlePtr = node.toRef();
    }
  }
  /**
   * User.handle
   */
  get handlePtr(): NodeReference | null {
    return this._handlePtr;
  }
  set handlePtr(value: NodeReference | null) {
    const prop = (this.constructor as NodeClass).__properties__["handle"];
    this._session.updateSetProperty(this, prop, value);
    this._handlePtr = value;
  }
  _handlePtr: NodeReference | null;

  /**
   * User.cursor
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
   * User.cursor
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
   * User.email
   */
  /**
   * User.email
   */
  get email(): string | null {
    return this._email;
  }
  set email(value: string | null) {
    const prop = (this.constructor as NodeClass).__properties__["email"];
    this._session.updateSetProperty(this, prop, value);
    this._email = value;
  }
  _email: string | null;

  /**
   * User.passwordSalt
   */
  /**
   * User.passwordSalt
   */
  get passwordSalt(): Uint8Array | null {
    return this._passwordSalt;
  }
  set passwordSalt(value: Uint8Array | null) {
    const prop = (this.constructor as NodeClass).__properties__["password_salt"];
    this._session.updateSetProperty(this, prop, value);
    this._passwordSalt = value;
  }
  _passwordSalt: Uint8Array | null;

  /**
   * User.passwordHash
   */
  /**
   * User.passwordHash
   */
  get passwordHash(): Uint8Array | null {
    return this._passwordHash;
  }
  set passwordHash(value: Uint8Array | null) {
    const prop = (this.constructor as NodeClass).__properties__["password_hash"];
    this._session.updateSetProperty(this, prop, value);
    this._passwordHash = value;
  }
  _passwordHash: Uint8Array | null;

  constructor(options: {
    id?: string;
    parent?: Space | NodeReference | null;
    space?: Space | NodeReference;
    materialization?: Materialization;
    definition?: Entity | NodeReference | null;
    branch?: Branch | NodeReference;
    snapshot?: Snapshot | NodeReference;
    precededBy?: User | NodeReference | null;
    instance?: Entity | NodeReference | null;
    createdAt?: Temporal.ZonedDateTime;
    createdEpoch?: number;
    createdBy?: (Entity & IsActor) | NodeReference | null;
    updatedAt?: Temporal.ZonedDateTime;
    updatedEpoch?: number;
    updatedBy?: (Entity & IsActor) | NodeReference | null;
    deletedAt?: Temporal.ZonedDateTime | null;
    customValues?: { readonly [key: string]: Value };
    name?: string;
    script?: Script | NodeReference | null;
    slug: string;
    status?: UserStatus;
    lastLoggedInAt?: Temporal.ZonedDateTime | null;
    isStaff?: boolean;
    handle?: Handle | NodeReference | null;
    cursor?: Cursor | NodeReference | null;
    email?: string | null;
    passwordSalt?: Uint8Array | null;
    passwordHash?: Uint8Array | null;
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
        throw new Error(`no active Space for User`);
      }
      _space = _space.toRef();
    }
    if (_space === null) {
      throw new Error(`User.space is required`);
    }
    this.spacePtr = _space;
    let _materialization = options.materialization ?? null;
    if (_materialization === null) {
      _materialization = 11 /* Materialization.ROOT */;
    }
    if (_materialization === null) {
      throw new Error(`User.materialization is required`);
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
        throw new Error(`no active Branch for User`);
      }
      _branch = _branch.toRef();
    }
    if (_branch === null) {
      throw new Error(`User.branch is required`);
    }
    this.branchPtr = _branch;
    let _snapshot = options.snapshot ?? null;
    if (_snapshot != null && _snapshot.metatype != StructType.NODE_REFERENCE) {
      _snapshot = (_snapshot as Node).toRef();
    }
    if (_snapshot === null) {
      _snapshot = ACTIVE_SNAPSHOT.get();
      if (_snapshot === null) {
        throw new Error(`no active Snapshot for User`);
      }
      _snapshot = _snapshot.toRef();
    }
    if (_snapshot === null) {
      throw new Error(`User.snapshot is required`);
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
    let _name = options.name ?? null;
    if (_name === null) {
      _name = "User";
    }
    if (_name === null) {
      throw new Error(`User.name is required`);
    }
    this._name = _name;
    let _script = options.script ?? null;
    if (_script != null && _script.metatype != StructType.NODE_REFERENCE) {
      _script = (_script as Node).toRef();
    }
    this._scriptPtr = _script;
    let _slug = options.slug;
    if (_slug === null) {
      throw new Error(`User.slug is required`);
    }
    this._slug = _slug;
    let _status = options.status ?? null;
    if (_status === null) {
      _status = 2 /* UserStatus.CREATING */;
    }
    if (_status === null) {
      throw new Error(`User.status is required`);
    }
    this._status = _status;
    let _lastLoggedInAt = options.lastLoggedInAt ?? null;
    this._lastLoggedInAt = _lastLoggedInAt;
    let _isStaff = options.isStaff ?? null;
    if (_isStaff === null) {
      _isStaff = false;
    }
    if (_isStaff === null) {
      throw new Error(`User.isStaff is required`);
    }
    this._isStaff = _isStaff;
    let _handle = options.handle ?? null;
    if (_handle != null && _handle.metatype != StructType.NODE_REFERENCE) {
      _handle = (_handle as Node).toRef();
    }
    this._handlePtr = _handle;
    let _cursor = options.cursor ?? null;
    if (_cursor != null && _cursor.metatype != StructType.NODE_REFERENCE) {
      _cursor = (_cursor as Node).toRef();
    }
    this._cursorPtr = _cursor;
    let _email = options.email ?? null;
    this._email = _email;
    let _passwordSalt = options.passwordSalt ?? null;
    this._passwordSalt = _passwordSalt;
    let _passwordHash = options.passwordHash ?? null;
    this._passwordHash = _passwordHash;

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
        throw new Error(`User.createdAt and User.updatedAt are required for existing Nodes`);
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
    if (!(this._slug === other._slug)) {
      return false;
    }
    if (!(this._status === other._status)) {
      return false;
    }
    if (!(this._lastLoggedInAt === other._lastLoggedInAt)) {
      return false;
    }
    if (!(this._isStaff === other._isStaff)) {
      return false;
    }
    if (!(this._handlePtr?.id === other._handlePtr?.id)) {
      return false;
    }
    if (!(this._cursorPtr?.id === other._cursorPtr?.id)) {
      return false;
    }
    if (!(this._email === other._email)) {
      return false;
    }
    if (!(this._scriptPtr?.id === other._scriptPtr?.id)) {
      return false;
    }
    if (!(this.definitionPtr?.id === other.definitionPtr?.id)) {
      return false;
    }
    if (!(this._name === other._name)) {
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
    h = (h * 31 + hashString(this._slug)) & 0xffffffff;
    h = (h * 31 + this._status) & 0xffffffff;
    if (this._lastLoggedInAt != null) {
      h =
        (h * 31 + hashString(this._lastLoggedInAt.toString({ timeZoneName: "never" }))) &
        0xffffffff;
    }
    h = (h * 31 + hashBool(this._isStaff)) & 0xffffffff;
    if (this._handlePtr != null) {
      h = (h * 31 + hashString(this._handlePtr.id)) & 0xffffffff;
    }
    if (this._cursorPtr != null) {
      h = (h * 31 + hashString(this._cursorPtr.id)) & 0xffffffff;
    }
    if (this._email != null) {
      h = (h * 31 + hashString(this._email)) & 0xffffffff;
    }
    if (this._passwordSalt != null) {
      h = (h * 31 + hashBytes(this._passwordSalt)) & 0xffffffff;
    }
    if (this._passwordHash != null) {
      h = (h * 31 + hashBytes(this._passwordHash)) & 0xffffffff;
    }
    if (this._scriptPtr != null) {
      h = (h * 31 + hashString(this._scriptPtr.id)) & 0xffffffff;
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
    if (this._customValues && Object.keys(this._customValues).length > 0) {
      for (const [_key, _value] of Object.entries(this._customValues)) {
        h = (h * 31 + hashString(_key.toString())) & 0xffffffff;
        h = (h * 31 + _value.hash()) & 0xffffffff;
      }
    }
    h = (h * 31 + hashString(this.spacePtr.id)) & 0xffffffff;

    return h;
  }

  validate(): void {
    throw new Error("not implemented");
  }

  __toRef__(): NodeReference {
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    return new _NodeReference({
      type: NodeType.USER,
      id: this.id,
      spaceId: this.spacePtr?.id ?? null,
      branchId: this.branchPtr?.id ?? null,
      snapshotId: this.snapshotPtr?.id ?? null,
      _session: this._session,
      _supergraph: this._supergraph,
    });
  }

  get _pathKey(): string {
    return this.slug ?? this.name;
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
    propertyReprs.push(`slug=${`"${this.slug}"`}`);
    propertyReprs.push(`status=${UserStatus[this.status]}`);
    propertyReprs.push(`name=${`"${this.name}"`}`);
    return `<User "${this.path}" ${propertyReprs.join(" ")}>`;
  }

  toCson(): { [key: string]: any } {
    return User.__packCson__(this);
  }

  static __packCson__(object: User): { [key: string]: any } {
    const objectCson: { [key: string]: any } = {};
    objectCson["1"] = 121000;
    objectCson["2"] = String(object.id);
    if (object.parentPtr != null) {
      objectCson["3"] = object.parentPtr.toCson();
    }
    objectCson["5"] = object.spacePtr.toCson();
    objectCson["10"] = object.materialization;
    if (object.definitionPtr != null) {
      objectCson["11"] = object.definitionPtr.toCson();
    }
    objectCson["12"] = object.branchPtr.toCson();
    objectCson["13"] = object.snapshotPtr.toCson();
    if (object.precededByPtr != null) {
      objectCson["14"] = object.precededByPtr.toCson();
    }
    if (object.instancePtr != null) {
      objectCson["15"] = object.instancePtr.toCson();
    }
    objectCson["20"] = object.createdAt.toString({ timeZoneName: "never" });
    objectCson["21"] = object.createdEpoch;
    if (object.createdByPtr != null) {
      objectCson["22"] = object.createdByPtr.toCson();
    }
    objectCson["23"] = object.updatedAt.toString({ timeZoneName: "never" });
    objectCson["24"] = object.updatedEpoch;
    if (object.updatedByPtr != null) {
      objectCson["25"] = object.updatedByPtr.toCson();
    }
    if (object.deletedAt != null) {
      objectCson["26"] = object.deletedAt.toString({ timeZoneName: "never" });
    }
    if (Object.keys(object._customValues).length > 0) {
      const packedCustomValues: { [key: string]: any } = {} as any;
      for (const [key, value] of Object.entries(object._customValues)) {
        packedCustomValues[String(String(key))] = value.toCson();
      }
      objectCson["30"] = packedCustomValues;
    }
    objectCson["50"] = object._name;
    if (object._scriptPtr != null) {
      objectCson["80"] = object._scriptPtr.toCson();
    }
    objectCson["102"] = object._slug;
    objectCson["110"] = object._status;
    if (object._lastLoggedInAt != null) {
      objectCson["111"] = object._lastLoggedInAt.toString({ timeZoneName: "never" });
    }
    objectCson["112"] = object._isStaff;
    if (object._handlePtr != null) {
      objectCson["121"] = object._handlePtr.toCson();
    }
    if (object._cursorPtr != null) {
      objectCson["122"] = object._cursorPtr.toCson();
    }
    if (object._email != null) {
      objectCson["130"] = object._email;
    }
    if (object._passwordSalt != null) {
      objectCson["131"] = base64Encode(object._passwordSalt);
    }
    if (object._passwordHash != null) {
      objectCson["132"] = base64Encode(object._passwordHash);
    }
    return objectCson;
  }

  static __unpackCson__(
    objectCson: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): User {
    const _Value = STRUCT_CLASS_BY_TYPE[StructType.VALUE] as typeof Value;
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    const parentPtrValue = objectCson["3"];
    const unpackedParentPtr =
      parentPtrValue != undefined
        ? _NodeReference.fromCson(parentPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const lastLoggedInAtValue = objectCson["111"];
    const unpackedLastLoggedInAt =
      lastLoggedInAtValue != undefined
        ? Temporal.Instant.from(lastLoggedInAtValue).toZonedDateTimeISO("UTC")
        : null;
    const handlePtrValue = objectCson["121"];
    const unpackedHandlePtr =
      handlePtrValue != undefined
        ? _NodeReference.fromCson(handlePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const cursorPtrValue = objectCson["122"];
    const unpackedCursorPtr =
      cursorPtrValue != undefined
        ? _NodeReference.fromCson(cursorPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const emailValue = objectCson["130"];
    const unpackedEmail = emailValue != undefined ? emailValue : null;
    const passwordSaltValue = objectCson["131"];
    const unpackedPasswordSalt =
      passwordSaltValue != undefined ? base64Decode(passwordSaltValue) : null;
    const passwordHashValue = objectCson["132"];
    const unpackedPasswordHash =
      passwordHashValue != undefined ? base64Decode(passwordHashValue) : null;
    const scriptPtrValue = objectCson["80"];
    const unpackedScriptPtr =
      scriptPtrValue != undefined
        ? _NodeReference.fromCson(scriptPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const definitionPtrValue = objectCson["11"];
    const unpackedDefinitionPtr =
      definitionPtrValue != undefined
        ? _NodeReference.fromCson(definitionPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const precededByPtrValue = objectCson["14"];
    const unpackedPrecededByPtr =
      precededByPtrValue != undefined
        ? _NodeReference.fromCson(precededByPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const instancePtrValue = objectCson["15"];
    const unpackedInstancePtr =
      instancePtrValue != undefined
        ? _NodeReference.fromCson(instancePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const createdByPtrValue = objectCson["22"];
    const unpackedCreatedByPtr =
      createdByPtrValue != undefined
        ? _NodeReference.fromCson(createdByPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const updatedByPtrValue = objectCson["25"];
    const unpackedUpdatedByPtr =
      updatedByPtrValue != undefined
        ? _NodeReference.fromCson(updatedByPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const deletedAtValue = objectCson["26"];
    const unpackedDeletedAt =
      deletedAtValue != undefined
        ? Temporal.Instant.from(deletedAtValue).toZonedDateTimeISO("UTC")
        : null;
    const unpackedCustomValues = {} as any;
    if (objectCson["30"] != undefined) {
      for (const [key, value] of Object.entries(objectCson["30"])) {
        unpackedCustomValues[String(key)] = _Value.fromCson(
          value as any,
          _session,
          _supergraph,
          _graph,
          _connection,
        );
      }
    }
    return new User({
      parent: unpackedParentPtr,
      slug: objectCson["102"],
      status: Number(objectCson["110"]),
      lastLoggedInAt: unpackedLastLoggedInAt,
      isStaff: objectCson["112"],
      handle: unpackedHandlePtr,
      cursor: unpackedCursorPtr,
      email: unpackedEmail,
      passwordSalt: unpackedPasswordSalt,
      passwordHash: unpackedPasswordHash,
      script: unpackedScriptPtr,
      materialization: Number(objectCson["10"]),
      definition: unpackedDefinitionPtr,
      branch: _NodeReference.fromCson(objectCson["12"], _session, _supergraph, _graph, _connection),
      snapshot: _NodeReference.fromCson(
        objectCson["13"],
        _session,
        _supergraph,
        _graph,
        _connection,
      ),
      precededBy: unpackedPrecededByPtr,
      instance: unpackedInstancePtr,
      createdAt: Temporal.Instant.from(objectCson["20"]).toZonedDateTimeISO("UTC"),
      createdEpoch: Number(objectCson["21"]),
      createdBy: unpackedCreatedByPtr,
      updatedAt: Temporal.Instant.from(objectCson["23"]).toZonedDateTimeISO("UTC"),
      updatedEpoch: Number(objectCson["24"]),
      updatedBy: unpackedUpdatedByPtr,
      deletedAt: unpackedDeletedAt,
      name: objectCson["50"],
      id: String(objectCson["2"]),
      customValues: unpackedCustomValues,
      space: _NodeReference.fromCson(objectCson["5"], _session, _supergraph, _graph, _connection),
      _session,
      _graph,
      _connection,
    });
  }

  static fromCson(
    objectCson: { readonly [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): User {
    return User.__unpackCson__(objectCson, _session, _supergraph, _graph, _connection);
  }

  toProto(): UserProto {
    return User.__packProto__(this);
  }

  static __packProto__(object: User): UserProto {
    const objectProto: Partial<UserProto> = { metatype: 121000 };
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
    objectProto.name = object._name;
    if (object._scriptPtr != null) {
      objectProto.scriptPtr = object._scriptPtr.toProto();
    }
    objectProto.slug = object._slug;
    objectProto.status = Number(object._status) as UserStatusProto;
    if (object._lastLoggedInAt != null) {
      objectProto.lastLoggedInAt = packProtoTimestamp(object._lastLoggedInAt);
    }
    objectProto.isStaff = object._isStaff;
    if (object._handlePtr != null) {
      objectProto.handlePtr = object._handlePtr.toProto();
    }
    if (object._cursorPtr != null) {
      objectProto.cursorPtr = object._cursorPtr.toProto();
    }
    if (object._email != null) {
      objectProto.email = object._email;
    }
    if (object._passwordSalt != null) {
      objectProto.passwordSalt = object._passwordSalt;
    }
    if (object._passwordHash != null) {
      objectProto.passwordHash = object._passwordHash;
    }
    return objectProto as UserProto;
  }

  static __unpackProto__(
    objectProto: UserProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): User {
    const _Value = STRUCT_CLASS_BY_TYPE[StructType.VALUE] as typeof Value;
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    const unpackedCustomValues = {} as any;
    if (objectProto.customValues) {
      for (const [key, value] of Object.entries(objectProto.customValues)) {
        unpackedCustomValues.set(
          String(key),
          _Value.fromProto((value as any)!, _session, _supergraph, _graph, _connection),
        );
      }
    }
    return new User({
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
      slug: objectProto.slug,
      status: Number(objectProto.status) as UserStatus,
      lastLoggedInAt:
        objectProto.lastLoggedInAt != undefined
          ? unpackProtoTimestamp(objectProto.lastLoggedInAt!)
          : null,
      isStaff: objectProto.isStaff,
      handle:
        objectProto.handlePtr != undefined
          ? _NodeReference.fromProto(
              objectProto.handlePtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
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
      email: objectProto.email != undefined ? objectProto.email : null,
      passwordSalt: objectProto.passwordSalt != undefined ? objectProto.passwordSalt : null,
      passwordHash: objectProto.passwordHash != undefined ? objectProto.passwordHash : null,
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
      id: String(objectProto.id),
      customValues: unpackedCustomValues,
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
    objectProto: UserProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): User {
    return User.__unpackProto__(objectProto, _session, _supergraph, _graph, _connection);
  }

  static fromProtoString(packedProtoString: string): User {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = UserProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerNodeClass(NodeType.USER, User);
/* ==== DESTACK_GENERATED_END:NODE:121000 ==== */
