import {
  Analytic,
  Entity,
  Event,
  Global,
  Graph,
  Indexed,
  IsDeletable,
  IsFrozen,
  IsJoinable,
  IsOwnable,
  IsOwner,
  IsSubject,
  IsTracked,
  LikeInvite,
  MaterializationType,
  Node,
  NodeReference,
  NodeType,
  Particle,
  QueryConnection,
  Role,
  RoleType,
  Session,
  Space,
  Spatial,
  StructType,
  Supergraph,
  TraitType,
} from "@/language";
import { Temporal } from "temporal-polyfill";

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
  static metatype: NodeType = NodeType.INVITE_EVENT;
  static __traits__: TraitType[] = [
    TraitType.SPATIAL,
    TraitType.PARTICLE,
    TraitType.ANALYTIC,
    TraitType.INDEXED,
    TraitType.FROZEN,
    TraitType.TRACKED,
    TraitType.EVENT,
  ];
  static __rootType__: NodeType | null = NodeType.SPACE;
  static __parentTypes__: NodeType[] = [NodeType.SPACE];
  static __childTypes__: NodeType[] = [];
  static __ancestorTypes__: NodeType[] = [NodeType.SPACE];
  static __descendantTypes__: NodeType[] = [];

  get parent(): Space | null | null {
    const nodePtr: NodeReference | null = this.parentPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Space | null | null;
    }
    return null;
  }
  readonly parentPtr: NodeReference | null;
  get space(): Space | null | null {
    const nodePtr: NodeReference | null = this.spacePtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Space | null | null;
    }
    return null;
  }
  readonly spacePtr: NodeReference | null;
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
  get node(): Invite | null {
    const nodePtr: NodeReference | null = this.nodePtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Invite | null;
    }
    return null;
  }

  set node(node: Invite) {
    this.nodePtr = node.toRef();
  }
  nodePtr: NodeReference;
  get joinable(): (Node & IsJoinable) | null {
    const nodePtr: NodeReference | null = this.joinablePtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as (Node & IsJoinable) | null;
    }
    return null;
  }

  set joinable(node: Node & IsJoinable) {
    this.joinablePtr = node.toRef();
  }
  joinablePtr: NodeReference;
  get member(): (Node & IsSubject) | null {
    const nodePtr: NodeReference | null = this.memberPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as (Node & IsSubject) | null;
    }
    return null;
  }

  set member(node: Node & IsSubject) {
    this.memberPtr = node.toRef();
  }
  memberPtr: NodeReference;
  get role(): Role | null | null {
    const nodePtr: NodeReference | null = this.rolePtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Role | null | null;
    }
    return null;
  }

  set role(node: Role | null) {
    if (node === null) {
      this.rolePtr = null;
    } else {
      this.rolePtr = node.toRef();
    }
  }
  rolePtr: NodeReference | null;
  roleType: RoleType;

  constructor(options: {
    id?: string;
    parent?: Space | NodeReference | null;
    space?: Space | NodeReference | null;
    createdAt?: Temporal.ZonedDateTime;
    createdBy?: (Node & IsSubject) | NodeReference | null;
    updatedAt?: Temporal.ZonedDateTime;
    updatedBy?: (Node & IsSubject) | NodeReference | null;
    node: Invite | NodeReference;
    joinable: (Node & IsJoinable) | NodeReference;
    member: (Node & IsSubject) | NodeReference;
    role?: Role | NodeReference | null;
    roleType: RoleType;
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
      options.id != null || options._graph != null,
    );

    // properties
    let _parent = options.parent ?? null;
    if (_parent != null && _parent instanceof Node) {
      _parent = _parent.toRef();
    }
    this.parentPtr = _parent;
    let _space = options.space ?? null;
    if (_space != null && _space instanceof Node) {
      _space = _space.toRef();
    }
    this.spacePtr = _space;
    let _node = options.node;
    if (_node != null && _node instanceof Node) {
      _node = _node.toRef();
    }
    if (_node === null) {
      throw new Error(`InviteEvent.node is required`);
    }
    this.nodePtr = _node;
    let _joinable = options.joinable;
    if (_joinable != null && _joinable instanceof Node) {
      _joinable = _joinable.toRef();
    }
    if (_joinable === null) {
      throw new Error(`InviteEvent.joinable is required`);
    }
    this.joinablePtr = _joinable;
    let _member = options.member;
    if (_member != null && _member instanceof Node) {
      _member = _member.toRef();
    }
    if (_member === null) {
      throw new Error(`InviteEvent.member is required`);
    }
    this.memberPtr = _member;
    let _role = options.role ?? null;
    if (_role != null && _role instanceof Node) {
      _role = _role.toRef();
    }
    this.rolePtr = _role;
    let _roleType = options.roleType;
    if (_roleType === null) {
      throw new Error(`InviteEvent.roleType is required`);
    }
    this.roleType = _roleType;

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
      nodeType: NodeType.INVITE_EVENT,
      id: this.id,
      spaceId: this.spacePtr?.id ?? null,
      _session: this._session,
      _supergraph: this._supergraph,
    });
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
  static metatype: NodeType = NodeType.INVITE;
  static __traits__: TraitType[] = [
    TraitType.GLOBAL,
    TraitType.SPATIAL,
    TraitType.ENTITY,
    TraitType.TRACKED,
    TraitType.OWNABLE,
    TraitType.DELETABLE,
    TraitType.INVITE,
  ];
  static __rootType__: NodeType | null = NodeType.SPACE;
  static __parentTypes__: NodeType[] = [
    NodeType.SPACE,
    NodeType.FOLDER,
    NodeType.ORGANIZATION,
    NodeType.TEAM,
    NodeType.THREAD,
  ];
  static __childTypes__: NodeType[] = [];
  static __ancestorTypes__: NodeType[] = [
    NodeType.SPACE,
    NodeType.FOLDER,
    NodeType.ORGANIZATION,
    NodeType.TEAM,
    NodeType.THREAD,
  ];
  static __descendantTypes__: NodeType[] = [];

  get parent(): (Node & IsJoinable) | null | null {
    const nodePtr: NodeReference | null = this.parentPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as (Node & IsJoinable) | null | null;
    }
    return null;
  }
  readonly parentPtr: NodeReference | null;
  get space(): Space | null | null {
    const nodePtr: NodeReference | null = this.spacePtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Space | null | null;
    }
    return null;
  }
  readonly spacePtr: NodeReference | null;
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
  readonly deletedAt: Temporal.ZonedDateTime | null;
  get ownedBy(): (Node & IsOwner) | null | null {
    const nodePtr: NodeReference | null = this.ownedByPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as (Node & IsOwner) | null | null;
    }
    return null;
  }

  set ownedBy(node: (Node & IsOwner) | null) {
    if (node === null) {
      this.ownedByPtr = null;
    } else {
      this.ownedByPtr = node.toRef();
    }
  }
  ownedByPtr: NodeReference | null;
  get member(): (Node & IsSubject) | null {
    const nodePtr: NodeReference | null = this.memberPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as (Node & IsSubject) | null;
    }
    return null;
  }

  set member(node: Node & IsSubject) {
    this.memberPtr = node.toRef();
  }
  memberPtr: NodeReference;
  get role(): Role | null | null {
    const nodePtr: NodeReference | null = this.rolePtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Role | null | null;
    }
    return null;
  }

  set role(node: Role | null) {
    if (node === null) {
      this.rolePtr = null;
    } else {
      this.rolePtr = node.toRef();
    }
  }
  rolePtr: NodeReference | null;
  roleType: RoleType | null;

  constructor(options: {
    id?: string;
    parent?: (Node & IsJoinable) | NodeReference | null;
    space?: Space | NodeReference | null;
    materialization?: MaterializationType;
    createdAt?: Temporal.ZonedDateTime;
    createdBy?: (Node & IsSubject) | NodeReference | null;
    updatedAt?: Temporal.ZonedDateTime;
    updatedBy?: (Node & IsSubject) | NodeReference | null;
    deletedAt?: Temporal.ZonedDateTime | null;
    ownedBy?: (Node & IsOwner) | NodeReference | null;
    member: (Node & IsSubject) | NodeReference;
    role?: Role | NodeReference | null;
    roleType?: RoleType | null;
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
      options.id != null || options._graph != null,
    );

    // properties
    let _parent = options.parent ?? null;
    if (_parent != null && _parent instanceof Node) {
      _parent = _parent.toRef();
    }
    this.parentPtr = _parent;
    let _space = options.space ?? null;
    if (_space != null && _space instanceof Node) {
      _space = _space.toRef();
    }
    this.spacePtr = _space;
    let _materialization = options.materialization ?? null;
    if (_materialization === null) {
      _materialization = MaterializationType.FULL_GRAPH;
    }
    if (_materialization === null) {
      throw new Error(`Invite.materialization is required`);
    }
    this.materialization = _materialization;
    let _deletedAt = options.deletedAt ?? null;
    this.deletedAt = _deletedAt;
    let _ownedBy = options.ownedBy ?? null;
    if (_ownedBy != null && _ownedBy instanceof Node) {
      _ownedBy = _ownedBy.toRef();
    }
    this.ownedByPtr = _ownedBy;
    let _member = options.member;
    if (_member != null && _member instanceof Node) {
      _member = _member.toRef();
    }
    if (_member === null) {
      throw new Error(`Invite.member is required`);
    }
    this.memberPtr = _member;
    let _role = options.role ?? null;
    if (_role != null && _role instanceof Node) {
      _role = _role.toRef();
    }
    this.rolePtr = _role;
    let _roleType = options.roleType ?? null;
    this.roleType = _roleType;

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
      nodeType: NodeType.INVITE,
      id: this.id,
      spaceId: this.spacePtr?.id ?? null,
      _session: this._session,
      _supergraph: this._supergraph,
    });
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
