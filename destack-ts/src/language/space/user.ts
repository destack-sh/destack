import {
  Cursor,
  Entity,
  Global,
  Graph,
  Handle,
  Icon,
  IsFollowable,
  IsOwner,
  IsSubject,
  IsTracked,
  MaterializationType,
  Node,
  NodeReference,
  NodeType,
  QueryConnection,
  Session,
  Space,
  StructType,
  Supergraph,
  TraitType,
} from "@destack/language";
import { Temporal } from "temporal-polyfill";

/* ==== DESTACK_GENERATED_START:ENUM:20 ==== */
export enum UserStatus {
  CREATING = 2,
  ACTIVE = 10,
}
/* ==== DESTACK_GENERATED_END:ENUM:20 ==== */

/* ==== DESTACK_GENERATED_START:NODE:20 ==== */
export class User extends Node implements Global, Entity, IsTracked, IsSubject, IsOwner, IsFollowable {
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

  get parent(): Node | null | null {
    const nodePtr: NodeReference | null = this.parentPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Node | null | null;
    }
    return null;
  }
  readonly parentPtr: NodeReference | null;
  readonly materialization: MaterializationType;
  readonly createdAt: Temporal.ZonedDateTime;
  get createdBy(): (Node & IsSubject) | null | null {
    const nodePtr: NodeReference | null = this.createdByPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as (Node & IsSubject) | null | null;
    }
    return null;
  }
  readonly createdByPtr: NodeReference | null;
  readonly updatedAt: Temporal.ZonedDateTime;
  get updatedBy(): (Node & IsSubject) | null | null {
    const nodePtr: NodeReference | null = this.updatedByPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as (Node & IsSubject) | null | null;
    }
    return null;
  }
  readonly updatedByPtr: NodeReference | null;
  name: string;
  slug: string;
  icon: Icon | null;
  readonly status: UserStatus;
  readonly lastLoggedInAt: Temporal.ZonedDateTime | null;
  readonly isStaff: boolean;
  get space(): Space | null {
    const nodePtr: NodeReference | null = this.spacePtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Space | null;
    }
    return null;
  }
  readonly spacePtr: NodeReference;
  get handle(): Handle | null | null {
    const nodePtr: NodeReference | null = this.handlePtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Handle | null | null;
    }
    return null;
  }
  readonly handlePtr: NodeReference | null;
  get cursor(): (Node & Cursor) | null | null {
    const nodePtr: NodeReference | null = this.cursorPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as (Node & Cursor) | null | null;
    }
    return null;
  }
  readonly cursorPtr: NodeReference | null;
  readonly email: string | null;
  readonly passwordSalt: Uint8Array | null;
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
}
/* ==== DESTACK_GENERATED_END:NODE:20 ==== */
