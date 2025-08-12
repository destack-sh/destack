import type {
  Branch,
  Bytes,
  DateTime,
  IsActor,
  IsFollowable,
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
  Entity,
  EnumType,
  type Event,
  type Materialization,
  type Node,
  NodeType,
  StructType,
} from "@destack/language/core";
import type { Script } from "@destack/language/logic";
import {
  registerEnumClass,
  registerNodeClass,
  STRUCT_CLASS_BY_TYPE,
} from "@destack/language/registry";
import type { Handle } from "@destack/language/universe/handle";
import { hashBool, hashBytes, hashString } from "@destack/utils/hash";
import { Temporal } from "temporal-polyfill";

/* ==== DESTACK_GENERATED_START:NODE:121000 ==== */
/**
 * A User is a human using Destack.
 */
export class User extends Entity implements IsActor, IsFollowable {
  static metatype: NodeType = NodeType.USER;

  /**
   * User.parent
   */
  get parent(): Space | null {
    const nodeRef: NodeReference | null = this.parentRef;
    if (nodeRef != null) {
      return this._session.graph.get(nodeRef) as Space | null;
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
  get precededBy(): User | null {
    const nodeRef: NodeReference | null = this.precededByRef;
    if (nodeRef != null) {
      return this._session.graph.get(nodeRef) as User | null;
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
  readonly createdAt: DateTime;

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
  readonly updatedAt: DateTime;

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
  readonly deletedAt: DateTime | null;

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
  get lastLoggedInAt(): DateTime | null {
    return this._lastLoggedInAt;
  }
  set lastLoggedInAt(value: DateTime | null) {
    const prop = (this.constructor as NodeClass).__properties__["last_logged_in_at"];
    this._session.updateSetProperty(this, prop, value);
    this._lastLoggedInAt = value;
  }
  _lastLoggedInAt: DateTime | null;

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
    const nodeRef: NodeReference | null = this.handleRef;
    if (nodeRef != null) {
      return this._session.graph.get(nodeRef) as Handle | null;
    }
    return null;
  }
  set handle(node: Handle | null) {
    if (node === null) {
      this.handleRef = null;
    } else {
      this.handleRef = node.toRef();
    }
  }
  /**
   * User.handle
   */
  get handleRef(): NodeReference | null {
    return this._handleRef;
  }
  set handleRef(value: NodeReference | null) {
    const prop = (this.constructor as NodeClass).__properties__["handle"];
    this._session.updateSetProperty(this, prop, value);
    this._handleRef = value;
  }
  _handleRef: NodeReference | null;

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
  get passwordSalt(): Bytes | null {
    return this._passwordSalt;
  }
  set passwordSalt(value: Bytes | null) {
    const prop = (this.constructor as NodeClass).__properties__["password_salt"];
    this._session.updateSetProperty(this, prop, value);
    this._passwordSalt = value;
  }
  _passwordSalt: Bytes | null;

  /**
   * User.passwordHash
   */
  /**
   * User.passwordHash
   */
  get passwordHash(): Bytes | null {
    return this._passwordHash;
  }
  set passwordHash(value: Bytes | null) {
    const prop = (this.constructor as NodeClass).__properties__["password_hash"];
    this._session.updateSetProperty(this, prop, value);
    this._passwordHash = value;
  }
  _passwordHash: Bytes | null;

  constructor(options: {
    id?: UUID;
    parent?: Space | NodeReference | null;
    space?: Space | NodeReference;
    materialization?: Materialization;
    definition?: Entity | NodeReference | null;
    branch?: Branch | NodeReference;
    snapshot?: Snapshot | NodeReference;
    precededBy?: User | NodeReference | null;
    instance?: Entity | NodeReference | null;
    createdAt?: DateTime;
    createdEpoch?: UInt128;
    createdBy?: Entity | NodeReference;
    updatedAt?: DateTime;
    updatedEpoch?: UInt128;
    updatedBy?: Entity | NodeReference;
    deletedAt?: DateTime | null;
    ownedBy?: Entity | NodeReference | null;
    name?: string;
    orderKey?: string;
    customValues?: { readonly [key: UUID]: Value };
    script?: Script | NodeReference | null;
    isExtensible?: boolean | null;
    source?: Script | NodeReference | null;
    key?: string | null;
    slug: string;
    status?: UserStatus;
    lastLoggedInAt?: DateTime | null;
    isStaff?: boolean;
    handle?: Handle | NodeReference | null;
    email?: string | null;
    passwordSalt?: Bytes | null;
    passwordHash?: Bytes | null;
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
        throw new Error(`no active Space for User`);
      }
      _space = _space.toRef();
    }
    if (_space == null) {
      throw new Error(`User.space is required`);
    }
    this.spaceRef = _space as NodeReference;
    let _materialization = options.materialization ?? null;
    if (_materialization == null) {
      _materialization = 11 /* Materialization.ROOT */;
    }
    if (_materialization == null) {
      throw new Error(`User.materialization is required`);
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
        throw new Error(`no active Branch for User`);
      }
      _branch = _branch.toRef();
    }
    if (_branch == null) {
      throw new Error(`User.branch is required`);
    }
    this.branchRef = _branch as NodeReference;
    let _snapshot = options.snapshot ?? null;
    if (_snapshot != null && _snapshot.constructor.name !== "NodeReference") {
      _snapshot = (_snapshot as Node).toRef();
    }
    if (_snapshot == null) {
      _snapshot = ACTIVE_SNAPSHOT.get();
      if (_snapshot == null) {
        throw new Error(`no active Snapshot for User`);
      }
      _snapshot = _snapshot.toRef();
    }
    if (_snapshot == null) {
      throw new Error(`User.snapshot is required`);
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
      _name = "User";
    }
    if (_name == null) {
      throw new Error(`User.name is required`);
    }
    this._name = _name;
    let _orderKey = options.orderKey ?? null;
    if (_orderKey == null) {
      _orderKey = "a0";
    }
    if (_orderKey == null) {
      throw new Error(`User.orderKey is required`);
    }
    this.orderKey = _orderKey;
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
    let _slug = options.slug;
    if (_slug == null) {
      throw new Error(`User.slug is required`);
    }
    this._slug = _slug;
    let _status = options.status ?? null;
    if (_status == null) {
      _status = 2 /* UserStatus.CREATING */;
    }
    if (_status == null) {
      throw new Error(`User.status is required`);
    }
    this._status = _status;
    let _lastLoggedInAt = options.lastLoggedInAt ?? null;
    this._lastLoggedInAt = _lastLoggedInAt;
    let _isStaff = options.isStaff ?? null;
    if (_isStaff == null) {
      _isStaff = false;
    }
    if (_isStaff == null) {
      throw new Error(`User.isStaff is required`);
    }
    this._isStaff = _isStaff;
    let _handle = options.handle ?? null;
    if (_handle != null && _handle.constructor.name !== "NodeReference") {
      _handle = (_handle as Node).toRef();
    }
    this._handleRef = _handle as NodeReference | null;
    let _email = options.email ?? null;
    this._email = _email;
    let _passwordSalt = options.passwordSalt ?? null;
    this._passwordSalt = _passwordSalt;
    let _passwordHash = options.passwordHash ?? null;
    this._passwordHash = _passwordHash;

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
        throw new Error(`User.createdAt and User.updatedAt are required for existing Nodes`);
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
    if (!(this._handleRef?.id === other._handleRef?.id)) {
      return false;
    }
    if (!(this._email === other._email)) {
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
    if (this.parentRef != null) {
      h = (h * 31 + hashString(this.parentRef.id)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this._slug)) & 0xffffffff;
    h = (h * 31 + this._status) & 0xffffffff;
    if (this._lastLoggedInAt != null) {
      h =
        (h * 31 + hashString(this._lastLoggedInAt.toString({ timeZoneName: "never" }))) &
        0xffffffff;
    }
    h = (h * 31 + hashBool(this._isStaff)) & 0xffffffff;
    if (this._handleRef != null) {
      h = (h * 31 + hashString(this._handleRef.id)) & 0xffffffff;
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
      type: NodeType.USER,
      id: this.id,
      spaceId: this.spaceRef.id,
      definitionId: this.definitionRef?.id ?? null,
      branchId: this.branchRef.id,
      snapshotId: this.snapshotRef.id,
      _session: this._session,
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
    if (this.ownedBy != null) {
      propertyReprs.push(`ownedBy=${this.ownedBy?.repr()}`);
    }
    propertyReprs.push(`name=${`"${this.name}"`}`);
    return `<User "${this.path}" ${propertyReprs.join(" ")}>`;
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerNodeClass(NodeType.USER, User);
/* ==== DESTACK_GENERATED_END:NODE:121000 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:121000 ==== */
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
/* ==== DESTACK_GENERATED_END:ENUM:121000 ==== */
