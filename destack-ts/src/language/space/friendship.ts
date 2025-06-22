import { Agent, BuiltinObject, ACTIVE_SESSION, Session, LikeInvite, StructFrozen, IsFrozen, Graph, Struct, Event, activeSession, Global, Particle, StructType, NodeType, Indexed, Supergraph, QueryConnection, NodeReference, MaterializationType, IsOwnable, User, Analytic, Spatial, Space, Entity, EnumType, Node, IsTracked } from '@/language';
import { Temporal } from 'temporal-polyfill'; // until Temporal ships natively

/* ==== DESTACK_GENERATED_START:ENUM:31 ==== */
export enum FriendshipInviteEventType {
  SENT = 1,
  RESCINDED = 2,
  ACCEPTED = 3,
  REJECTED = 4,
}
/* ==== DESTACK_GENERATED_END:ENUM:31 ==== */

/* ==== DESTACK_GENERATED_START:NODE:30 ==== */
export class Friendship extends Node implements Global, Entity, IsTracked {
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
  get userA(): User | null {
      const nodePtr: NodeReference | null = this.userAPtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as User | null;
      }
      return null;
  }
  ;
  userAPtr: NodeReference
  get userB(): User | null {
      const nodePtr: NodeReference | null = this.userBPtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as User | null;
      }
      return null;
  }
  ;
  userBPtr: NodeReference

  constructor(options: {
    userA: User | NodeReference,
    userB: User | NodeReference,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: Graph | null,
    _connection?: QueryConnection | null
  }) {
    const session = options._session ?? ACTIVE_SESSION.get();
    const supergraph = options._supergraph ?? session.supergraph;
    super(options.id, options.parent, session, supergraph, options._graph, options._connection);
    this.userAPtr = options.userA != null ? (options.userA.metatype == StructType.NODE_REFERENCE ? (options.userA as NodeReference) : (options.userA as Node).toRef()) : null;
    this.userBPtr = options.userB != null ? (options.userB.metatype == StructType.NODE_REFERENCE ? (options.userB as NodeReference) : (options.userB as Node).toRef()) : null;
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
    return new NodeReference(NodeType.FRIENDSHIP, this.id, null, null, this._supergraph);

  get _pathKey(): string {
      return "Friendship[id={this.id}]";
  }

  get path(): string {
      return "Friendship[id={this.id}]";
  }
}
/* ==== DESTACK_GENERATED_END:NODE:30 ==== */

/* ==== DESTACK_GENERATED_START:NODE:32 ==== */
export class FriendshipInviteEvent extends Node implements Spatial, Particle, Analytic, Indexed, Event, IsFrozen, IsTracked {
  readonly id: string;
  get parent(): Space | null | null {
      const nodePtr: NodeReference | null = this.parentPtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Space | null | null;
      }
      return null;
  }
  ;
  parentPtr: NodeReference | null
  get space(): Space | null | null {
      const nodePtr: NodeReference | null = this.spacePtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Space | null | null;
      }
      return null;
  }
  ;
  spacePtr: NodeReference | null
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
  type: FriendshipInviteEventType;
  get node(): FriendshipInvite | null {
      const nodePtr: NodeReference | null = this.nodePtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as FriendshipInvite | null;
      }
      return null;
  }

  set node(node: FriendshipInvite) {
      this.nodePtr = node.toRef();
  }
  ;
  nodePtr: NodeReference

  constructor(options: {
    type: FriendshipInviteEventType,
    node: FriendshipInvite | NodeReference,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: Graph | null,
    _connection?: QueryConnection | null
  }) {
    const session = options._session ?? ACTIVE_SESSION.get();
    const supergraph = options._supergraph ?? session.supergraph;
    super(options.id, options.parent, session, supergraph, options._graph, options._connection);
    this.type = options.type;
    this.nodePtr = options.node != null ? (options.node.metatype == StructType.NODE_REFERENCE ? (options.node as NodeReference) : (options.node as Node).toRef()) : null;
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
    return new NodeReference(NodeType.FRIENDSHIP_INVITE_EVENT, this.id, this.spacePtr?.id ?? null, null, this._supergraph);
  }

  get _pathKey(): string {
      return "FriendshipInviteEvent[id={this.id}]";
  }

  get path(): string {
      const pathParts: string[] = [];
      let node: Node | null = this;
      while (node !== null) {
          pathParts.push(node._pathKey);
          node = node.parent;
      }
      if (!this._isAttached) {
          pathParts.push("<detached>");
      }
      return pathParts.reverse().join("/");
  }
}
/* ==== DESTACK_GENERATED_END:NODE:32 ==== */

/* ==== DESTACK_GENERATED_START:NODE:31 ==== */
export class FriendshipInvite extends Node implements Global, Entity, IsTracked, IsOwnable, LikeInvite {
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
  get ownedBy(): Agent | User | null {
      const nodePtr: NodeReference | null = this.ownedByPtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Agent | User | null;
      }
      return null;
  }

  set ownedBy(node: Agent | User) {
      this.ownedByPtr = node.toRef();
  }
  ;
  ownedByPtr: NodeReference
  get member(): Agent | User | null {
      const nodePtr: NodeReference | null = this.memberPtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Agent | User | null;
      }
      return null;
  }

  set member(node: Agent | User) {
      this.memberPtr = node.toRef();
  }
  ;
  memberPtr: NodeReference

  constructor(options: {
    ownedBy: Agent | User | NodeReference,
    member: Agent | User | NodeReference,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: Graph | null,
    _connection?: QueryConnection | null
  }) {
    const session = options._session ?? ACTIVE_SESSION.get();
    const supergraph = options._supergraph ?? session.supergraph;
    super(options.id, options.parent, session, supergraph, options._graph, options._connection);
    this.ownedByPtr = options.ownedBy != null ? (options.ownedBy.metatype == StructType.NODE_REFERENCE ? (options.ownedBy as NodeReference) : (options.ownedBy as Node).toRef()) : null;
    this.memberPtr = options.member != null ? (options.member.metatype == StructType.NODE_REFERENCE ? (options.member as NodeReference) : (options.member as Node).toRef()) : null;
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
    return new NodeReference(NodeType.FRIENDSHIP_INVITE, this.id, null, null, this._supergraph);

  get _pathKey(): string {
      return "FriendshipInvite[id={this.id}]";
  }

  get path(): string {
      return "FriendshipInvite[id={this.id}]";
  }
}
/* ==== DESTACK_GENERATED_END:NODE:31 ==== */