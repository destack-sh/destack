import { packProtoTimestamp, unpackProtoTimestamp } from "@destack/grpc";
import type {
  Graph,
  IsCustomizable,
  IsFollowable,
  IsGlobal,
  IsOwner,
  IsSubject,
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
  StructType,
} from "@destack/language/core";
import type { Cursor } from "@destack/language/logic";
import {
  STRUCT_CLASS_BY_TYPE,
  registerEnumClass,
  registerNodeClass,
} from "@destack/language/registry";
import type { Handle } from "@destack/language/space/handle";
import type { Space } from "@destack/language/space/space";
import { MaterializationProto, UserProto, UserStatusProto } from "@destack/proto";
import { base64Decode, base64Encode } from "@destack/utils";
import { hashBool, hashBytes, hashString } from "@destack/utils/hash";
import { Temporal } from "temporal-polyfill";

/* ==== DESTACK_GENERATED_START:ENUM:10200 ==== */
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
/* ==== DESTACK_GENERATED_END:ENUM:10200 ==== */

/* ==== DESTACK_GENERATED_START:NODE:10200 ==== */
/**
 * A User is a human using Destack.
 */
export class User
  extends Entity
  implements IsGlobal, IsOwner, IsFollowable, IsSubject, IsCustomizable
{
  static metatype: NodeType = NodeType.USER;

  /**
   * Trait.parent
   */
  get parent(): Node | null {
    const nodePtr: NodeReference | null = this.parentPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Node | null;
    }
    return null;
  }
  readonly parentPtr: NodeReference | null;

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
  get predecessor(): User | null {
    const nodePtr: NodeReference | null = this.predecessorPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as User | null;
    }
    return null;
  }
  readonly predecessorPtr: NodeReference | null;

  /**
   * The template this Entity instance is based on (from the template tree).
   */
  get template(): User | null {
    const nodePtr: NodeReference | null = this.templatePtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as User | null;
    }
    return null;
  }
  readonly templatePtr: NodeReference | null;

  /**
   * The (root) Entity in this Entity's instance tree (not the template tree).
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
   * The custom Values of this Node, keyed by custom Property id. May hold both static and instance values.
   */
  customValues: Map<string, Value>;

  /**
   * User.name
   */
  name: string;

  /**
   * User.slug
   */
  slug: string;

  /**
   * User.status
   */
  readonly status: UserStatus;

  /**
   * User.lastLoggedInAt
   */
  readonly lastLoggedInAt: Temporal.ZonedDateTime | null;

  /**
   * User.isStaff
   */
  readonly isStaff: boolean;

  /**
   * User.space
   */
  get space(): Space | null {
    const nodePtr: NodeReference | null = this.spacePtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Space | null;
    }
    return null;
  }
  readonly spacePtr: NodeReference;

  /**
   * User.handle
   */
  get handle(): Handle | null {
    const nodePtr: NodeReference | null = this.handlePtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Handle | null;
    }
    return null;
  }
  readonly handlePtr: NodeReference | null;

  /**
   * User.cursor
   */
  get cursor(): Cursor | null {
    const nodePtr: NodeReference | null = this.cursorPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Cursor | null;
    }
    return null;
  }
  readonly cursorPtr: NodeReference | null;

  /**
   * User.email
   */
  readonly email: string | null;

  /**
   * User.passwordSalt
   */
  readonly passwordSalt: Uint8Array | null;

  /**
   * User.passwordHash
   */
  readonly passwordHash: Uint8Array | null;

  constructor(options: {
    id?: string;
    parent?: Node | NodeReference | null;
    materialization?: Materialization;
    snapshot?: Snapshot | NodeReference | null;
    predecessor?: User | NodeReference | null;
    template?: User | NodeReference | null;
    instanceRoot?: Entity | NodeReference | null;
    createdAt?: Temporal.ZonedDateTime;
    createdBy?: (Node & IsSubject) | NodeReference | null;
    updatedAt?: Temporal.ZonedDateTime;
    updatedBy?: (Node & IsSubject) | NodeReference | null;
    customValues?: Map<string, Value>;
    name: string;
    slug: string;
    status?: UserStatus;
    lastLoggedInAt?: Temporal.ZonedDateTime | null;
    isStaff?: boolean;
    space: Space | NodeReference;
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
      // is_attached
      true,
    );

    // properties
    let _parent = options.parent ?? null;
    if (_parent != null && _parent.metatype != StructType.NODE_REFERENCE) {
      _parent = (_parent as Node).toRef();
    }
    this.parentPtr = _parent;
    let _materialization = options.materialization ?? null;
    if (_materialization === null) {
      _materialization = 32 /* Materialization.FULL */;
    }
    if (_materialization === null) {
      throw new Error(`User.materialization is required`);
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
    let _customValues = options.customValues ?? null;
    if (_customValues === null) {
      _customValues = new Map();
    }
    this.customValues = _customValues;
    let _name = options.name;
    if (_name === null) {
      throw new Error(`User.name is required`);
    }
    this.name = _name;
    let _slug = options.slug;
    if (_slug === null) {
      throw new Error(`User.slug is required`);
    }
    this.slug = _slug;
    let _status = options.status ?? null;
    if (_status === null) {
      _status = 2 /* UserStatus.CREATING */;
    }
    if (_status === null) {
      throw new Error(`User.status is required`);
    }
    this.status = _status;
    let _lastLoggedInAt = options.lastLoggedInAt ?? null;
    this.lastLoggedInAt = _lastLoggedInAt;
    let _isStaff = options.isStaff ?? null;
    if (_isStaff === null) {
      _isStaff = false;
    }
    if (_isStaff === null) {
      throw new Error(`User.isStaff is required`);
    }
    this.isStaff = _isStaff;
    let _space = options.space;
    if (_space != null && _space.metatype != StructType.NODE_REFERENCE) {
      _space = (_space as Node).toRef();
    }
    if (_space === null) {
      throw new Error(`User.space is required`);
    }
    this.spacePtr = _space;
    let _handle = options.handle ?? null;
    if (_handle != null && _handle.metatype != StructType.NODE_REFERENCE) {
      _handle = (_handle as Node).toRef();
    }
    this.handlePtr = _handle;
    let _cursor = options.cursor ?? null;
    if (_cursor != null && _cursor.metatype != StructType.NODE_REFERENCE) {
      _cursor = (_cursor as Node).toRef();
    }
    this.cursorPtr = _cursor;
    let _email = options.email ?? null;
    this.email = _email;
    let _passwordSalt = options.passwordSalt ?? null;
    this.passwordSalt = _passwordSalt;
    let _passwordHash = options.passwordHash ?? null;
    this.passwordHash = _passwordHash;

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
    if (!(this.slug === other.slug)) {
      return false;
    }
    if (!(this.status === other.status)) {
      return false;
    }
    if (!(this.lastLoggedInAt === other.lastLoggedInAt)) {
      return false;
    }
    if (!(this.isStaff === other.isStaff)) {
      return false;
    }
    if (!(this.spacePtr.id === other.spacePtr.id)) {
      return false;
    }
    if (!(this.handlePtr?.id === other.handlePtr?.id)) {
      return false;
    }
    if (!(this.cursorPtr?.id === other.cursorPtr?.id)) {
      return false;
    }
    if (!(this.email === other.email)) {
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
    return true;
  }

  hash(): number {
    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    h = (h * 31 + hashString(this.name)) & 0xffffffff;
    h = (h * 31 + hashString(this.slug)) & 0xffffffff;
    h = (h * 31 + this.status) & 0xffffffff;
    if (this.lastLoggedInAt !== null) {
      h =
        (h * 31 + hashString(this.lastLoggedInAt.toString({ timeZoneName: "never" }))) & 0xffffffff;
    }
    h = (h * 31 + hashBool(this.isStaff)) & 0xffffffff;
    h = (h * 31 + hashString(this.spacePtr.id)) & 0xffffffff;
    if (this.handlePtr !== null) {
      h = (h * 31 + hashString(this.handlePtr.id)) & 0xffffffff;
    }
    if (this.cursorPtr !== null) {
      h = (h * 31 + hashString(this.cursorPtr.id)) & 0xffffffff;
    }
    if (this.email !== null) {
      h = (h * 31 + hashString(this.email)) & 0xffffffff;
    }
    if (this.passwordSalt !== null) {
      h = (h * 31 + hashBytes(this.passwordSalt)) & 0xffffffff;
    }
    if (this.passwordHash !== null) {
      h = (h * 31 + hashBytes(this.passwordHash)) & 0xffffffff;
    }
    if (this.customValues && Object.keys(this.customValues).length > 0) {
      for (const [_key, _value] of Object.entries(this.customValues)) {
        h = (h * 31 + hashString(_key.toString())) & 0xffffffff;
        h = (h * 31 + _value.hash()) & 0xffffffff;
      }
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
    h = (h * 31 + hashString(this.id.toString())) & 0xffffffff;
    if (this.parentPtr !== null) {
      h = (h * 31 + hashString(this.parentPtr.id)) & 0xffffffff;
    }

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
      snapshotId: this.snapshotPtr?.id ?? null,
      _session: this._session,
      _supergraph: this._supergraph,
    });
  }

  get _pathKey(): string {
    return this.slug ?? this.name;
  }

  get path(): string {
    return this.slug ?? this.name;
  }

  repr(): string {
    const propertyReprs: string[] = [];
    propertyReprs.push(`name=${this.name}`);
    propertyReprs.push(`slug=${this.slug}`);
    propertyReprs.push(`status=${UserStatus[this.status]}`);
    return `<User '${this.path}' ${propertyReprs.join(" ")}>`;
  }

  toValue(): { [key: string]: any } {
    return User.__packValue__(this);
  }

  static __packValue__(object: User): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 10200;
    objectValue["2"] = String(object.id);
    if (object.parentPtr != null) {
      objectValue["3"] = object.parentPtr.toValue();
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
    if (object.customValues.size > 0) {
      const packedCustomValues: { [key: string]: any } = {};
      for (const [key, value] of object.customValues) {
        packedCustomValues[String(String(key))] = value.toValue();
      }
      objectValue["26"] = packedCustomValues;
    }
    objectValue["101"] = object.name;
    objectValue["102"] = object.slug;
    objectValue["110"] = object.status;
    if (object.lastLoggedInAt != null) {
      objectValue["111"] = object.lastLoggedInAt.toString({ timeZoneName: "never" });
    }
    objectValue["112"] = object.isStaff;
    objectValue["120"] = object.spacePtr.toValue();
    if (object.handlePtr != null) {
      objectValue["121"] = object.handlePtr.toValue();
    }
    if (object.cursorPtr != null) {
      objectValue["122"] = object.cursorPtr.toValue();
    }
    if (object.email != null) {
      objectValue["130"] = object.email;
    }
    if (object.passwordSalt != null) {
      objectValue["131"] = base64Encode(object.passwordSalt);
    }
    if (object.passwordHash != null) {
      objectValue["132"] = base64Encode(object.passwordHash);
    }
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): User {
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    const _Value = STRUCT_CLASS_BY_TYPE[StructType.VALUE] as typeof Value;
    const lastLoggedInAtValue = objectValue["111"];
    const unpackedLastLoggedInAt =
      lastLoggedInAtValue != undefined
        ? Temporal.Instant.from(lastLoggedInAtValue).toZonedDateTimeISO("UTC")
        : null;
    const handlePtrValue = objectValue["121"];
    const unpackedHandlePtr =
      handlePtrValue != undefined
        ? _NodeReference.fromValue(handlePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const cursorPtrValue = objectValue["122"];
    const unpackedCursorPtr =
      cursorPtrValue != undefined
        ? _NodeReference.fromValue(cursorPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const emailValue = objectValue["130"];
    const unpackedEmail = emailValue != undefined ? emailValue : null;
    const passwordSaltValue = objectValue["131"];
    const unpackedPasswordSalt =
      passwordSaltValue != undefined ? base64Decode(passwordSaltValue) : null;
    const passwordHashValue = objectValue["132"];
    const unpackedPasswordHash =
      passwordHashValue != undefined ? base64Decode(passwordHashValue) : null;
    const unpackedCustomValues = new Map();
    if (objectValue["26"] != undefined) {
      for (const [key, value] of Object.entries(objectValue["26"])) {
        unpackedCustomValues.set(
          String(key),
          _Value.fromValue(value as any, _session, _supergraph, _graph, _connection),
        );
      }
    }
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
    const parentPtrValue = objectValue["3"];
    const unpackedParentPtr =
      parentPtrValue != undefined
        ? _NodeReference.fromValue(parentPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    return new User({
      name: objectValue["101"],
      slug: objectValue["102"],
      status: Number(objectValue["110"]),
      lastLoggedInAt: unpackedLastLoggedInAt,
      isStaff: objectValue["112"],
      space: _NodeReference.fromValue(
        objectValue["120"],
        _session,
        _supergraph,
        _graph,
        _connection,
      ),
      handle: unpackedHandlePtr,
      cursor: unpackedCursorPtr,
      email: unpackedEmail,
      passwordSalt: unpackedPasswordSalt,
      passwordHash: unpackedPasswordHash,
      customValues: unpackedCustomValues,
      materialization: Number(objectValue["10"]),
      snapshot: unpackedSnapshotPtr,
      predecessor: unpackedPredecessorPtr,
      template: unpackedTemplatePtr,
      instanceRoot: unpackedInstanceRootPtr,
      createdAt: Temporal.Instant.from(objectValue["20"]).toZonedDateTimeISO("UTC"),
      createdBy: unpackedCreatedByPtr,
      updatedAt: Temporal.Instant.from(objectValue["22"]).toZonedDateTimeISO("UTC"),
      updatedBy: unpackedUpdatedByPtr,
      id: String(objectValue["2"]),
      parent: unpackedParentPtr,
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
  ): User {
    return User.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
  }

  toProto(): UserProto {
    return User.__packProto__(this);
  }

  static __packProto__(object: User): UserProto {
    const objectProto: Partial<UserProto> = { metatype: 10200 };
    objectProto.id = String(object.id);
    if (object.parentPtr != null) {
      objectProto.parentPtr = object.parentPtr.toProto();
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
    if (object.customValues) {
      objectProto.customValues = {};
      for (const [key, value] of object.customValues) {
        objectProto.customValues![String(key)] = value.toProto();
      }
    }
    objectProto.name = object.name;
    objectProto.slug = object.slug;
    objectProto.status = Number(object.status) as UserStatusProto;
    if (object.lastLoggedInAt != null) {
      objectProto.lastLoggedInAt = packProtoTimestamp(object.lastLoggedInAt);
    }
    objectProto.isStaff = object.isStaff;
    objectProto.spacePtr = object.spacePtr.toProto();
    if (object.handlePtr != null) {
      objectProto.handlePtr = object.handlePtr.toProto();
    }
    if (object.cursorPtr != null) {
      objectProto.cursorPtr = object.cursorPtr.toProto();
    }
    if (object.email != null) {
      objectProto.email = object.email;
    }
    if (object.passwordSalt != null) {
      objectProto.passwordSalt = object.passwordSalt;
    }
    if (object.passwordHash != null) {
      objectProto.passwordHash = object.passwordHash;
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
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    const _Value = STRUCT_CLASS_BY_TYPE[StructType.VALUE] as typeof Value;
    const unpackedCustomValues = new Map();
    if (objectProto.customValues) {
      for (const [key, value] of Object.entries(objectProto.customValues)) {
        unpackedCustomValues.set(
          String(key),
          _Value.fromProto((value as any)!, _session, _supergraph, _graph, _connection),
        );
      }
    }
    return new User({
      name: objectProto.name,
      slug: objectProto.slug,
      status: Number(objectProto.status) as UserStatus,
      lastLoggedInAt:
        objectProto.lastLoggedInAt != undefined
          ? unpackProtoTimestamp(objectProto.lastLoggedInAt!)
          : null,
      isStaff: objectProto.isStaff,
      space: _NodeReference.fromProto(
        objectProto.spacePtr!,
        _session,
        _supergraph,
        _graph,
        _connection,
      ),
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
      customValues: unpackedCustomValues,
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
      id: String(objectProto.id),
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
/* ==== DESTACK_GENERATED_END:NODE:10200 ==== */
