import { IsTracked, EnumType, Session, IsFollowable, IsOwner, Supergraph, Handle, Space, ThreadCursor, EventCursor, MaterializationType, Global, Icon, Entity, Struct, QueryConnection, BuiltinObject, StructFrozen, IsSubject, StructType, ScreenCursor, Node, NodeType, NodeReference, Agent, Graph } from '@/language';
import { Temporal } from 'temporal-polyfill'; // until Temporal ships natively

/* ==== DESTACK_GENERATED_START:ENUM:20 ==== */
export enum UserStatus {
  CREATING = 2,
  ACTIVE = 10,
}
/* ==== DESTACK_GENERATED_END:ENUM:20 ==== */

/* ==== DESTACK_GENERATED_START:NODE:20 ==== */
export class User extends Node implements Global, Entity, IsTracked, IsSubject, IsOwner, IsFollowable {
  readonly id: string;
  get parent(): Node | null | null {
      const nodePtr: NodeReference | null = this.parentPtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Node | null | null;
      }
      return null;
  }
  ;
  parentPtr: NodeReference | null
  readonly materialization: MaterializationType;
  readonly createdAt: Temporal.ZonedDateTime;
  get createdBy(): Agent | User | null | null {
      const nodePtr: NodeReference | null = this.createdByPtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Agent | User | null | null;
      }
      return null;
  }
  ;
  createdByPtr: NodeReference | null
  readonly updatedAt: Temporal.ZonedDateTime;
  get updatedBy(): Agent | User | null | null {
      const nodePtr: NodeReference | null = this.updatedByPtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Agent | User | null | null;
      }
      return null;
  }
  ;
  updatedByPtr: NodeReference | null
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
  ;
  spacePtr: NodeReference
  get handle(): Handle | null | null {
      const nodePtr: NodeReference | null = this.handlePtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Handle | null | null;
      }
      return null;
  }
  ;
  handlePtr: NodeReference | null
  get cursor(): EventCursor | ScreenCursor | ThreadCursor | null | null {
      const nodePtr: NodeReference | null = this.cursorPtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as EventCursor | ScreenCursor | ThreadCursor | null | null;
      }
      return null;
  }
  ;
  cursorPtr: NodeReference | null
  readonly email: string | null;
  readonly passwordSalt: Uint8Array | null;
  readonly passwordHash: Uint8Array | null;

  constructor(
    id: string,
    parentPtr: NodeReference | null,
    materialization: MaterializationType,
    createdAt: Temporal.ZonedDateTime,
    createdByPtr: NodeReference | null,
    updatedAt: Temporal.ZonedDateTime,
    updatedByPtr: NodeReference | null,
    name: string,
    slug: string,
    icon: Icon | null,
    status: UserStatus,
    lastLoggedInAt: Temporal.ZonedDateTime | null,
    isStaff: boolean,
    spacePtr: NodeReference,
    handlePtr: NodeReference | null,
    cursorPtr: NodeReference | null,
    email: string | null,
    passwordSalt: Uint8Array | null,
    passwordHash: Uint8Array | null,
    _session: Session,
    _supergraph: Supergraph,
    _graph: Graph,
    _connection: QueryConnection | null
  ) {
    super(id, _session, _supergraph, _graph, _connection);
    this.id = id;
    this.parentPtr = parentPtr;
    this.materialization = materialization;
    this.createdAt = createdAt;
    this.createdByPtr = createdByPtr;
    this.updatedAt = updatedAt;
    this.updatedByPtr = updatedByPtr;
    this.name = name;
    this.slug = slug;
    this.icon = icon;
    this.status = status;
    this.lastLoggedInAt = lastLoggedInAt;
    this.isStaff = isStaff;
    this.spacePtr = spacePtr;
    this.handlePtr = handlePtr;
    this.cursorPtr = cursorPtr;
    this.email = email;
    this.passwordSalt = passwordSalt;
    this.passwordHash = passwordHash;
  }


  static from(options: {
    name: string,
    slug: string,
    icon?: Icon | null,
    status?: UserStatus,
    lastLoggedInAt?: Temporal.ZonedDateTime | null,
    isStaff?: boolean,
    space: Space | NodeReference,
    handle?: Handle | NodeReference | null,
    cursor?: EventCursor | ScreenCursor | ThreadCursor | NodeReference | null,
    email?: string | null,
    passwordSalt?: Uint8Array | null,
    passwordHash?: Uint8Array | null,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: Graph | null,
    _connection?: QueryConnection | null
  }): User {
    const session = options._session ?? ACTIVE_SESSION.get();
    const supergraph = options._supergraph ?? session.supergraph;
    return new User(
      options.name,
      options.slug,
      options.icon ?? null,
      options.status ?? UserStatus.CREATING,
      options.lastLoggedInAt ?? null,
      options.isStaff ?? false,
      options.space != null ? (options.space.metatype == StructType.NODE_REFERENCE ? options.space : options.space.toRef()) : null,
      options.handle != null ? (options.handle.metatype == StructType.NODE_REFERENCE ? options.handle : options.handle.toRef()) : null,
      options.cursor != null ? (options.cursor.metatype == StructType.NODE_REFERENCE ? options.cursor : options.cursor.toRef()) : null,
      options.email ?? null,
      options.passwordSalt ?? null,
      options.passwordHash ?? null,
      session,
      supergraph,
      options._graph,
      options._connection
    );
  }

  equals(other: any): boolean {
    throw new Error("Not implemented");
  }

  hash(): number {
    throw new Error("Not implemented");
  }

  validate(): void {
    throw new Error("Not implemented");
  }

  __toRef__(): NodeReference {
    return new NodeReference(NodeType.USER, this.id, null, null, this._supergraph);

  get _pathKey(): string {
      return this.slug ?? this.name;
  }

  get path(): string {
      return this.slug ?? this.name;
  }
}
/* ==== DESTACK_GENERATED_END:NODE:20 ==== */