import { packProtoTimestamp, unpackProtoTimestamp } from "@destack/grpc";
import { Graph, NodeReference, QueryConnection, Session, Supergraph } from "@destack/language/core";
import {
  Entity,
  EnumType,
  Global,
  HasIcon,
  HasName,
  HasSlug,
  IsFollowable,
  IsOwner,
  IsSubject,
  Node,
  NodeType,
  StructType,
  TraitType,
} from "@destack/language/core/builtin";
import { Icon } from "@destack/language/core/common";
import { Cursor } from "@destack/language/logic";
import { registerEnumClass, registerNodeClass } from "@destack/language/registry";
import { Handle, Space } from "@destack/language/space";
import { UserProto, UserStatusProto } from "@destack/proto";
import { base64Decode, base64Encode } from "@destack/utils";
import { Temporal } from "temporal-polyfill";

/* ==== DESTACK_GENERATED_START:ENUM:20 ==== */
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
/* ==== DESTACK_GENERATED_END:ENUM:20 ==== */

/* ==== DESTACK_GENERATED_START:NODE:40 ==== */
/**
 * A User is a human using Destack.
 */
export class User
  extends Node
  implements Global, Entity, HasName, HasIcon, HasSlug, IsOwner, IsFollowable, IsSubject
{
  static metatype: NodeType = NodeType.USER;
  static __traits__: TraitType[] = [
    TraitType.GLOBAL,
    TraitType.ENTITY,
    TraitType.TRACKED,
    TraitType.SUBJECT,
    TraitType.OWNER,
    TraitType.FOLLOWABLE,
  ];
  static __rootType__: NodeType | null = null;
  static __parentTypes__: NodeType[] = [];
  static __childTypes__: NodeType[] = [
    NodeType.ENTITLEMENT,
    NodeType.SANCTION,
    NodeType.FOLLOW,
    NodeType.CLIENT,
  ];
  static __ancestorTypes__: NodeType[] = [];
  static __descendantTypes__: NodeType[] = [
    NodeType.CLIENT,
    NodeType.ENTITLEMENT,
    NodeType.FOLLOW,
    NodeType.SANCTION,
  ];

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
   * User.name
   */
  name: string;

  /**
   * User.slug
   */
  slug: string;

  /**
   * HasIcon.icon
   */
  icon: Icon | null;

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
  get cursor(): (Node & Cursor) | null {
    const nodePtr: NodeReference | null = this.cursorPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as (Node & Cursor) | null;
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
    createdAt?: Temporal.ZonedDateTime;
    createdBy?: (Node & IsSubject) | NodeReference | null;
    updatedAt?: Temporal.ZonedDateTime;
    updatedBy?: (Node & IsSubject) | NodeReference | null;
    name: string;
    slug: string;
    icon?: Icon | null;
    status?: UserStatus;
    lastLoggedInAt?: Temporal.ZonedDateTime | null;
    isStaff?: boolean;
    space: Space | NodeReference;
    handle?: Handle | NodeReference | null;
    cursor?: (Node & Cursor) | NodeReference | null;
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
    if (_parent != null && _parent instanceof Node) {
      _parent = _parent.toRef();
    }
    this.parentPtr = _parent;
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
    let _icon = options.icon ?? null;
    this.icon = _icon;
    let _status = options.status ?? null;
    if (_status === null) {
      _status = UserStatus.CREATING;
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
    if (_space != null && _space instanceof Node) {
      _space = _space.toRef();
    }
    if (_space === null) {
      throw new Error(`User.space is required`);
    }
    this.spacePtr = _space;
    let _handle = options.handle ?? null;
    if (_handle != null && _handle instanceof Node) {
      _handle = _handle.toRef();
    }
    this.handlePtr = _handle;
    let _cursor = options.cursor ?? null;
    if (_cursor != null && _cursor instanceof Node) {
      _cursor = _cursor.toRef();
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
      const now = Temporal.Now.zonedDateTimeISO();
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
    if (
      (this.icon == null) !== (other.icon == null) ||
      (this.icon != null && !this.icon.equals(other.icon))
    ) {
      return false;
    }
    return true;
  }

  hash(): number {
    throw new Error("not implemented");
  }

  validate(): void {
    throw new Error("not implemented");
  }

  __toRef__(): NodeReference {
    return new NodeReference({
      nodeType: NodeType.USER,
      id: this.id,
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

  toValue(): { [key: string]: any } {
    return User.__packValue__(this);
  }

  static __packValue__(object: User): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 40;
    objectValue["2"] = String(object.id);
    if (object.parentPtr != null) {
      objectValue["3"] = object.parentPtr.toValue();
    }
    objectValue["15"] = object.createdAt.toString();
    if (object.createdByPtr != null) {
      objectValue["16"] = object.createdByPtr.toValue();
    }
    objectValue["17"] = object.updatedAt.toString();
    if (object.updatedByPtr != null) {
      objectValue["18"] = object.updatedByPtr.toValue();
    }
    objectValue["31"] = object.name;
    objectValue["33"] = object.slug;
    if (object.icon != null) {
      objectValue["34"] = object.icon.toValue();
    }
    objectValue["40"] = object.status;
    if (object.lastLoggedInAt != null) {
      objectValue["41"] = object.lastLoggedInAt.toString();
    }
    objectValue["45"] = object.isStaff;
    objectValue["50"] = object.spacePtr.toValue();
    if (object.handlePtr != null) {
      objectValue["51"] = object.handlePtr.toValue();
    }
    if (object.cursorPtr != null) {
      objectValue["52"] = object.cursorPtr.toValue();
    }
    if (object.email != null) {
      objectValue["60"] = object.email;
    }
    if (object.passwordSalt != null) {
      objectValue["61"] = base64Encode(object.passwordSalt);
    }
    if (object.passwordHash != null) {
      objectValue["62"] = base64Encode(object.passwordHash);
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
    const lastLoggedInAtValue = objectValue["41"];
    const unpackedLastLoggedInAt =
      lastLoggedInAtValue != undefined ? Temporal.ZonedDateTime.from(lastLoggedInAtValue) : null;
    const handlePtrValue = objectValue["51"];
    const unpackedHandlePtr =
      handlePtrValue != undefined
        ? NodeReference.fromValue(handlePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const cursorPtrValue = objectValue["52"];
    const unpackedCursorPtr =
      cursorPtrValue != undefined
        ? NodeReference.fromValue(cursorPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const emailValue = objectValue["60"];
    const unpackedEmail = emailValue != undefined ? emailValue : null;
    const passwordSaltValue = objectValue["61"];
    const unpackedPasswordSalt =
      passwordSaltValue != undefined ? base64Decode(passwordSaltValue) : null;
    const passwordHashValue = objectValue["62"];
    const unpackedPasswordHash =
      passwordHashValue != undefined ? base64Decode(passwordHashValue) : null;
    const parentPtrValue = objectValue["3"];
    const unpackedParentPtr =
      parentPtrValue != undefined
        ? NodeReference.fromValue(parentPtrValue, _session, _supergraph, _graph, _connection)
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
    const iconValue = objectValue["34"];
    const unpackedIcon =
      iconValue != undefined
        ? Icon.fromValue(iconValue, _session, _supergraph, _graph, _connection)
        : null;
    return new User({
      name: objectValue["31"],
      slug: objectValue["33"],
      status: Number(objectValue["40"]),
      lastLoggedInAt: unpackedLastLoggedInAt,
      isStaff: objectValue["45"],
      space: NodeReference.fromValue(objectValue["50"], _session, _supergraph, _graph, _connection),
      handle: unpackedHandlePtr,
      cursor: unpackedCursorPtr,
      email: unpackedEmail,
      passwordSalt: unpackedPasswordSalt,
      passwordHash: unpackedPasswordHash,
      id: String(objectValue["2"]),
      parent: unpackedParentPtr,
      createdAt: Temporal.ZonedDateTime.from(objectValue["15"]),
      createdBy: unpackedCreatedByPtr,
      updatedAt: Temporal.ZonedDateTime.from(objectValue["17"]),
      updatedBy: unpackedUpdatedByPtr,
      icon: unpackedIcon,
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
    const objectProto: Partial<UserProto> = { metatype: 40 };
    objectProto.id = String(object.id);
    if (object.parentPtr != null) {
      objectProto.parentPtr = object.parentPtr.toProto();
    }
    objectProto.createdAt = packProtoTimestamp(object.createdAt);
    if (object.createdByPtr != null) {
      objectProto.createdByPtr = object.createdByPtr.toProto();
    }
    objectProto.updatedAt = packProtoTimestamp(object.updatedAt);
    if (object.updatedByPtr != null) {
      objectProto.updatedByPtr = object.updatedByPtr.toProto();
    }
    objectProto.name = object.name;
    objectProto.slug = object.slug;
    if (object.icon != null) {
      objectProto.icon = object.icon.toProto();
    }
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
    return new User({
      name: objectProto.name,
      slug: objectProto.slug,
      status: Number(objectProto.status) as UserStatus,
      lastLoggedInAt:
        objectProto.lastLoggedInAt != undefined
          ? unpackProtoTimestamp(objectProto.lastLoggedInAt!)
          : null,
      isStaff: objectProto.isStaff,
      space: NodeReference.fromProto(
        objectProto.spacePtr!,
        _session,
        _supergraph,
        _graph,
        _connection,
      ),
      handle:
        objectProto.handlePtr != undefined
          ? NodeReference.fromProto(
              objectProto.handlePtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      cursor:
        objectProto.cursorPtr != undefined
          ? NodeReference.fromProto(
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
      id: String(objectProto.id),
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
      icon:
        objectProto.icon != undefined
          ? Icon.fromProto(objectProto.icon!, _session, _supergraph, _graph, _connection)
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
/* ==== DESTACK_GENERATED_END:NODE:40 ==== */
