import { Session, Spatial, Entity, Particle, IsDeletable, Organization, Analytic, User, IsFrozen, NodeReference, BuiltinObject, IsOwnable, Agent, MaterializationType, Folder, IsTracked, Indexed, Event, Graph, Role, LikeInvite, Thread, Struct, RoleType, Supergraph, Global, QueryConnection, NodeType, Team, Node, Space } from '@/language';
import { Temporal } from 'temporal-polyfill'; // until Temporal ships natively

/* ==== DESTACK_GENERATED_START:ENUM:510 ==== */
export enum InviteEventType {
  SENT = 1,
  RESCINDED = 2,
  ACCEPTED = 3,
  REJECTED = 4,
}
/* ==== DESTACK_GENERATED_END:ENUM:510 ==== */

/* ==== DESTACK_GENERATED_START:NODE:511 ==== */
export class InviteEvent extends Node implements Spatial, Particle, Analytic, Indexed, Event, IsFrozen, IsTracked {
  readonly id: string;
  get parent(): Space | null {
      const nodePtr: NodeReference | null = this.parentPtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Space | null;
      }
      return null;
  }
  ;
  parentPtr: NodeReference | null
  get space(): Space | null {
      const nodePtr: NodeReference | null = this.spacePtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Space | null;
      }
      return null;
  }

  set space(value: Space | null) {
      if (value === null) {
          this.spacePtr = null;
      } else {
          this.spacePtr = value.toRef();
      }
  }
  ;
  spacePtr: NodeReference | null
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
  get node(): Invite | null {
      const nodePtr: NodeReference | null = this.nodePtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Invite | null;
      }
      return null;
  }

  set node(value: Invite | null) {
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

  set joinable(value: Folder | Thread | Organization | Space | Team | null) {
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

  set member(value: Agent | User | null) {
      if (value === null) {
          this.memberPtr = null;
      } else {
          this.memberPtr = value.toRef();
      }
  }
  ;
  memberPtr: NodeReference
  get role(): Role | null {
      const nodePtr: NodeReference | null = this.rolePtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Role | null;
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


  static create(): InviteEvent {

    return new InviteEvent();
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
    return new NodeReference(NodeType.INVITE_EVENT, this.id, this.spacePtr?.id ?? null, null, this._supergraph);
  }

  get _pathKey(): string {
      return "InviteEvent[id={this.id}]";
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
/* ==== DESTACK_GENERATED_END:NODE:511 ==== */

/* ==== DESTACK_GENERATED_START:NODE:510 ==== */
export class Invite extends Node implements Global, Spatial, Entity, IsTracked, IsDeletable, IsOwnable, LikeInvite {
  readonly id: string;
  get parent(): Folder | Thread | Organization | Space | Team | null {
      const nodePtr: NodeReference | null = this.parentPtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Folder | Thread | Organization | Space | Team | null;
      }
      return null;
  }
  ;
  parentPtr: NodeReference | null
  get space(): Space | null {
      const nodePtr: NodeReference | null = this.spacePtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Space | null;
      }
      return null;
  }

  set space(value: Space | null) {
      if (value === null) {
          this.spacePtr = null;
      } else {
          this.spacePtr = value.toRef();
      }
  }
  ;
  spacePtr: NodeReference | null
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
  readonly deletedAt: Temporal.ZonedDateTime | null;
  get ownedBy(): Role | Agent | Organization | Team | User | null {
      const nodePtr: NodeReference | null = this.ownedByPtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Role | Agent | Organization | Team | User | null;
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
  get member(): Agent | User | null {
      const nodePtr: NodeReference | null = this.memberPtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Agent | User | null;
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
  get role(): Role | null {
      const nodePtr: NodeReference | null = this.rolePtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Role | null;
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


  static create(): Invite {

    return new Invite();
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
    return new NodeReference(NodeType.INVITE, this.id, this.spacePtr?.id ?? null, null, this._supergraph);
  }

  get _pathKey(): string {
      return "Invite[id={this.id}]";
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
/* ==== DESTACK_GENERATED_END:NODE:510 ==== */