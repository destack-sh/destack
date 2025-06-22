import { Agent, IsFollowable, BuiltinObject, ACTIVE_SESSION, Session, StructFrozen, ThreadCursor, Graph, Struct, ScreenCursor, activeSession, Global, StructType, NodeType, IsSubject, EventCursor, Icon, Supergraph, QueryConnection, NodeReference, MaterializationType, Handle, Space, Entity, EnumType, IsOwner, Node, IsTracked } from '@/language';
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

  constructor(options: {
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
  }) {
    const session = options._session ?? ACTIVE_SESSION.get();
    const supergraph = options._supergraph ?? session.supergraph;
    super(options.id, options.parent, session, supergraph, options._graph, options._connection);
    this.name = options.name;
    this.slug = options.slug;
    this.icon = options.icon ?? null;
    this.status = options.status ?? UserStatus.CREATING;
    this.lastLoggedInAt = options.lastLoggedInAt ?? null;
    this.isStaff = options.isStaff ?? false;
    this.spacePtr = options.space != null ? (options.space.metatype == StructType.NODE_REFERENCE ? (options.space as NodeReference) : (options.space as Node).toRef()) : null;
    this.handlePtr = options.handle != null ? (options.handle.metatype == StructType.NODE_REFERENCE ? (options.handle as NodeReference) : (options.handle as Node).toRef()) : null;
    this.cursorPtr = options.cursor != null ? (options.cursor.metatype == StructType.NODE_REFERENCE ? (options.cursor as NodeReference) : (options.cursor as Node).toRef()) : null;
    this.email = options.email ?? null;
    this.passwordSalt = options.passwordSalt ?? null;
    this.passwordHash = options.passwordHash ?? null;
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