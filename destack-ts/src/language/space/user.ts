import { Session, Entity, IsOwner, NodeReference, BuiltinObject, Handle, Agent, MaterializationType, IsTracked, Graph, ScreenCursor, IsSubject, IsFollowable, ThreadCursor, Icon, Struct, Supergraph, Global, QueryConnection, NodeType, EventCursor, Node, Space } from '@/language';
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
  get parent(): Node | null {
      const nodePtr: NodeReference | null = this.parentPtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Node | null;
      }
      return null;
  }
  ;
  parentPtr: NodeReference | null
  materialization: MaterializationType;
  readonly createdAt: Temporal.ZonedDateTime;
  get createdBy(): Agent | User | null {
      const nodePtr: NodeReference | null = this.createdByPtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Agent | User | null;
      }
      return null;
  }
  ;
  createdByPtr: NodeReference | null
  readonly updatedAt: Temporal.ZonedDateTime;
  get updatedBy(): Agent | User | null {
      const nodePtr: NodeReference | null = this.updatedByPtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Agent | User | null;
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
  get handle(): Handle | null {
      const nodePtr: NodeReference | null = this.handlePtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Handle | null;
      }
      return null;
  }
  ;
  handlePtr: NodeReference | null
  get cursor(): EventCursor | ScreenCursor | ThreadCursor | null {
      const nodePtr: NodeReference | null = this.cursorPtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as EventCursor | ScreenCursor | ThreadCursor | null;
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


  static create(): User {

    return new User();
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
      return this.slug or this.name;
  }

  get path(): string {
      return this.slug or this.name;
  }
}
/* ==== DESTACK_GENERATED_END:NODE:20 ==== */