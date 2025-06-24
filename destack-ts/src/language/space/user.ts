import {
  Entity,
  Global,
  Graph,
  HasIcon,
  HasName,
  HasSlug,
  Icon,
  IsFollowable,
  IsOwner,
  IsSubject,
  MaterializationType,
  Node,
  NodeReference,
  NodeType,
  QueryConnection,
  Session,
  StructType,
  Supergraph,
  TraitType,
} from "@destack/language/core";
import { Cursor } from "@destack/language/logic";
import { Handle, Space } from "@destack/language/space";
import { Temporal } from "temporal-polyfill";

/* ==== DESTACK_GENERATED_START:ENUM:20 ==== */
/**
 * UserStatus
 */
export enum UserStatus {
  CREATING = 2,
  ACTIVE = 10,
}
/* ==== DESTACK_GENERATED_END:ENUM:20 ==== */

/* ==== DESTACK_GENERATED_START:NODE:20 ==== */
/**
 * A User is a human using Destack.
 */
export class User extends Node implements Global, Entity, HasName, HasIcon, HasSlug, IsOwner, IsFollowable, IsSubject {
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
  static __childTypes__: NodeType[] = [NodeType.ENTITLEMENT, NodeType.SANCTION, NodeType.FOLLOW, NodeType.CLIENT];
  static __ancestorTypes__: NodeType[] = [];
  static __descendantTypes__: NodeType[] = [NodeType.FOLLOW, NodeType.CLIENT, NodeType.SANCTION, NodeType.ENTITLEMENT];

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
    materialization?: MaterializationType;
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
    let _materialization = options.materialization ?? null;
    if (_materialization === null) {
      _materialization = MaterializationType.FULL_GRAPH;
    }
    if (_materialization === null) {
      throw new Error(`User.materialization is required`);
    }
    this.materialization = _materialization;
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
    objectValue["1"] = 20;
    objectValue["2"] = String(object.id);
    if (object.parentPtr !== null) {
      objectValue["3"] = object.parentPtr.toValue();
    }
    objectValue["7"] = object.materialization;
    objectValue["15"] = object.createdAt.toString();
    if (object.createdByPtr !== null) {
      objectValue["16"] = object.createdByPtr.toValue();
    }
    objectValue["17"] = object.updatedAt.toString();
    if (object.updatedByPtr !== null) {
      objectValue["18"] = object.updatedByPtr.toValue();
    }
    objectValue["31"] = object.name;
    objectValue["33"] = object.slug;
    if (object.icon !== null) {
      objectValue["34"] = object.icon.toValue();
    }
    objectValue["40"] = object.status;
    if (object.lastLoggedInAt !== null) {
      objectValue["41"] = object.lastLoggedInAt.toString();
    }
    objectValue["45"] = object.isStaff;
    objectValue["50"] = object.spacePtr.toValue();
    if (object.handlePtr !== null) {
      objectValue["51"] = object.handlePtr.toValue();
    }
    if (object.cursorPtr !== null) {
      objectValue["52"] = object.cursorPtr.toValue();
    }
    if (object.email !== null) {
      objectValue["60"] = object.email;
    }
    if (object.passwordSalt !== null) {
      objectValue["61"] = Buffer.from(object.passwordSalt).toString("base64");
    }
    if (object.passwordHash !== null) {
      objectValue["62"] = Buffer.from(object.passwordHash).toString("base64");
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
      lastLoggedInAtValue !== undefined ? Temporal.ZonedDateTime.from(lastLoggedInAtValue) : null;
    const emailValue = objectValue["60"];
    const unpackedEmail = emailValue !== undefined ? emailValue : null;
    const passwordSaltValue = objectValue["61"];
    const unpackedPasswordSalt = passwordSaltValue !== undefined ? Buffer.from(passwordSaltValue, "base64") : null;
    const passwordHashValue = objectValue["62"];
    const unpackedPasswordHash = passwordHashValue !== undefined ? Buffer.from(passwordHashValue, "base64") : null;
    const iconValue = objectValue["34"];
    const unpackedIcon =
      iconValue !== undefined ? Icon.fromValue(iconValue, _session, _supergraph, _graph, _connection) : null;
    const handleValue = objectValue["51"];
    const unpackedHandle =
      handleValue !== undefined
        ? NodeReference.fromValue(handleValue, _session, _supergraph, _graph, _connection)
        : null;
    const cursorValue = objectValue["52"];
    const unpackedCursor =
      cursorValue !== undefined
        ? NodeReference.fromValue(cursorValue, _session, _supergraph, _graph, _connection)
        : null;
    const parentValue = objectValue["3"];
    const unpackedParent =
      parentValue !== undefined
        ? NodeReference.fromValue(parentValue, _session, _supergraph, _graph, _connection)
        : null;
    const createdByValue = objectValue["16"];
    const unpackedCreatedBy =
      createdByValue !== undefined
        ? NodeReference.fromValue(createdByValue, _session, _supergraph, _graph, _connection)
        : null;
    const updatedByValue = objectValue["18"];
    const unpackedUpdatedBy =
      updatedByValue !== undefined
        ? NodeReference.fromValue(updatedByValue, _session, _supergraph, _graph, _connection)
        : null;
    return new User({
      name: objectValue["31"],
      slug: objectValue["33"],
      status: Number(objectValue["40"]),
      lastLoggedInAt: unpackedLastLoggedInAt,
      isStaff: objectValue["45"],
      email: unpackedEmail,
      passwordSalt: unpackedPasswordSalt,
      passwordHash: unpackedPasswordHash,
      id: String(objectValue["2"]),
      materialization: Number(objectValue["7"]),
      createdAt: Temporal.ZonedDateTime.from(objectValue["15"]),
      updatedAt: Temporal.ZonedDateTime.from(objectValue["17"]),
      icon: unpackedIcon,
      space: NodeReference.fromValue(objectValue["50"], _session, _supergraph, _graph, _connection),
      handle: unpackedHandle,
      cursor: unpackedCursor,
      parent: unpackedParent,
      createdBy: unpackedCreatedBy,
      updatedBy: unpackedUpdatedBy,
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
}
/* ==== DESTACK_GENERATED_END:NODE:20 ==== */
