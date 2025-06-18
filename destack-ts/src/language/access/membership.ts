import { Folder, Thread, LikeMembership, IsFrozen, EnumType, StructType, Node, QueryConnection, User, IsDeletable, Analytic, NodeReference, Graph, Spatial, IsOwnable, Role, Particle, Space, Agent, StructFrozen, Team, Struct, BuiltinObject, Organization, NodeType, Session, MaterializationType, Event, Entity, RoleType, Indexed, Supergraph, Global, IsTracked } from '@/language';
import { Temporal } from 'temporal-polyfill'; // until Temporal ships natively

/* ==== DESTACK_GENERATED_START:ENUM:500 ==== */
export enum MembershipEventType {
  JOIN = 1,
  LEAVE = 2,
  KICK = 3,
  BAN = 4,
}
/* ==== DESTACK_GENERATED_END:ENUM:500 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:501 ==== */
export enum MembershipPermission {
  KICK = 10,
  BAN = 11,
}
/* ==== DESTACK_GENERATED_END:ENUM:501 ==== */

/* ==== DESTACK_GENERATED_START:NODE:501 ==== */
export class MembershipEvent extends Node implements Spatial, Particle, Analytic, Indexed, Event, IsFrozen, IsTracked {
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
  get node(): Membership | null {
      const nodePtr: NodeReference | null = this.nodePtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Membership | null;
      }
      return null;
  }

  set node(value: Membership) {
      if (value === null) {
          this.nodePtr = null;
      } else {
          this.nodePtr = value.toRef();
      }
  }
  ;
  nodePtr: NodeReference
  get joinable(): Folder | Thread | Organization | Space | Team | null {
      const nodePtr: NodeReference | null = this.joinablePtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Folder | Thread | Organization | Space | Team | null;
      }
      return null;
  }

  set joinable(value: Folder | Thread | Organization | Space | Team) {
      if (value === null) {
          this.joinablePtr = null;
      } else {
          this.joinablePtr = value.toRef();
      }
  }
  ;
  joinablePtr: NodeReference
  get member(): Agent | User | null {
      const nodePtr: NodeReference | null = this.memberPtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Agent | User | null;
      }
      return null;
  }

  set member(value: Agent | User) {
      if (value === null) {
          this.memberPtr = null;
      } else {
          this.memberPtr = value.toRef();
      }
  }
  ;
  memberPtr: NodeReference
  get role(): Role | null | null {
      const nodePtr: NodeReference | null = this.rolePtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Role | null | null;
      }
      return null;
  }

  set role(value: Role | null) {
      if (value === null) {
          this.rolePtr = null;
      } else {
          this.rolePtr = value.toRef();
      }
  }
  ;
  rolePtr: NodeReference | null
  roleType: RoleType;

  constructor(
    id: string,
    parentPtr: NodeReference | null,
    spacePtr: NodeReference | null,
    createdAt: Temporal.ZonedDateTime,
    createdByPtr: NodeReference | null,
    updatedAt: Temporal.ZonedDateTime,
    updatedByPtr: NodeReference | null,
    nodePtr: NodeReference,
    joinablePtr: NodeReference,
    memberPtr: NodeReference,
    rolePtr: NodeReference | null,
    roleType: RoleType,
    _session: Session,
    _supergraph: Supergraph,
    _graph: Graph,
    _connection: QueryConnection | null
  ) {
    super(id, _session, _supergraph, _graph, _connection);
    this.id = id;
    this.parentPtr = parentPtr;
    this.spacePtr = spacePtr;
    this.createdAt = createdAt;
    this.createdByPtr = createdByPtr;
    this.updatedAt = updatedAt;
    this.updatedByPtr = updatedByPtr;
    this.nodePtr = nodePtr;
    this.joinablePtr = joinablePtr;
    this.memberPtr = memberPtr;
    this.rolePtr = rolePtr;
    this.roleType = roleType;
  }


  static create(options: {
    node: Membership | NodeReference,
    joinable: Folder | Thread | Organization | Space | Team | NodeReference,
    member: Agent | User | NodeReference,
    role?: Role | NodeReference | null,
    roleType: RoleType,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: Graph | null,
    _connection?: QueryConnection | null
  }): MembershipEvent {
    const session = options._session ?? ACTIVE_SESSION.get();
    const supergraph = options._supergraph ?? session.supergraph;
    return new MembershipEvent(
      options.node != null ? (options.node.metatype == StructType.NODE_REFERENCE ? options.node : options.node.toRef()) : null,
      options.joinable != null ? (options.joinable.metatype == StructType.NODE_REFERENCE ? options.joinable : options.joinable.toRef()) : null,
      options.member != null ? (options.member.metatype == StructType.NODE_REFERENCE ? options.member : options.member.toRef()) : null,
      options.role != null ? (options.role.metatype == StructType.NODE_REFERENCE ? options.role : options.role.toRef()) : null,
      options.roleType,
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
    return new NodeReference(NodeType.MEMBERSHIP_EVENT, this.id, this.spacePtr?.id ?? null, null, this._supergraph);
  }

  get _pathKey(): string {
      return "MembershipEvent[id={this.id}]";
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
/* ==== DESTACK_GENERATED_END:NODE:501 ==== */

/* ==== DESTACK_GENERATED_START:NODE:500 ==== */
export class Membership extends Node implements Global, Spatial, Entity, IsTracked, IsDeletable, IsOwnable, LikeMembership {
  readonly id: string;
  get parent(): Folder | Thread | Organization | Space | Team | null | null {
      const nodePtr: NodeReference | null = this.parentPtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Folder | Thread | Organization | Space | Team | null | null;
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
  readonly deletedAt: Temporal.ZonedDateTime | null;
  get ownedBy(): Role | Agent | Organization | Team | User | null | null {
      const nodePtr: NodeReference | null = this.ownedByPtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Role | Agent | Organization | Team | User | null | null;
      }
      return null;
  }

  set ownedBy(value: Role | Agent | Organization | Team | User | null) {
      if (value === null) {
          this.ownedByPtr = null;
      } else {
          this.ownedByPtr = value.toRef();
      }
  }
  ;
  ownedByPtr: NodeReference | null
  get member(): Agent | User | null | null {
      const nodePtr: NodeReference | null = this.memberPtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Agent | User | null | null;
      }
      return null;
  }

  set member(value: Agent | User | null) {
      if (value === null) {
          this.memberPtr = null;
      } else {
          this.memberPtr = value.toRef();
      }
  }
  ;
  memberPtr: NodeReference | null
  get role(): Role | null | null {
      const nodePtr: NodeReference | null = this.rolePtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Role | null | null;
      }
      return null;
  }

  set role(value: Role | null) {
      if (value === null) {
          this.rolePtr = null;
      } else {
          this.rolePtr = value.toRef();
      }
  }
  ;
  rolePtr: NodeReference | null
  roleType: RoleType | null;

  constructor(
    id: string,
    parentPtr: NodeReference | null,
    spacePtr: NodeReference | null,
    materialization: MaterializationType,
    createdAt: Temporal.ZonedDateTime,
    createdByPtr: NodeReference | null,
    updatedAt: Temporal.ZonedDateTime,
    updatedByPtr: NodeReference | null,
    deletedAt: Temporal.ZonedDateTime | null,
    ownedByPtr: NodeReference | null,
    memberPtr: NodeReference | null,
    rolePtr: NodeReference | null,
    roleType: RoleType | null,
    _session: Session,
    _supergraph: Supergraph,
    _graph: Graph,
    _connection: QueryConnection | null
  ) {
    super(id, _session, _supergraph, _graph, _connection);
    this.id = id;
    this.parentPtr = parentPtr;
    this.spacePtr = spacePtr;
    this.materialization = materialization;
    this.createdAt = createdAt;
    this.createdByPtr = createdByPtr;
    this.updatedAt = updatedAt;
    this.updatedByPtr = updatedByPtr;
    this.deletedAt = deletedAt;
    this.ownedByPtr = ownedByPtr;
    this.memberPtr = memberPtr;
    this.rolePtr = rolePtr;
    this.roleType = roleType;
  }


  static create(options: {
    ownedBy?: Role | Agent | Organization | Team | User | NodeReference | null,
    member?: Agent | User | NodeReference | null,
    role?: Role | NodeReference | null,
    roleType?: RoleType | null,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: Graph | null,
    _connection?: QueryConnection | null
  }): Membership {
    const session = options._session ?? ACTIVE_SESSION.get();
    const supergraph = options._supergraph ?? session.supergraph;
    return new Membership(
      options.ownedBy != null ? (options.ownedBy.metatype == StructType.NODE_REFERENCE ? options.ownedBy : options.ownedBy.toRef()) : null,
      options.member != null ? (options.member.metatype == StructType.NODE_REFERENCE ? options.member : options.member.toRef()) : null,
      options.role != null ? (options.role.metatype == StructType.NODE_REFERENCE ? options.role : options.role.toRef()) : null,
      options.roleType ?? null,
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
    return new NodeReference(NodeType.MEMBERSHIP, this.id, this.spacePtr?.id ?? null, null, this._supergraph);
  }

  get _pathKey(): string {
      return "Membership[id={this.id}]";
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
/* ==== DESTACK_GENERATED_END:NODE:500 ==== */