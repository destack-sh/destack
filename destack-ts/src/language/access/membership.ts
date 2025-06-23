import {
  Agent,
  Analytic,
  Entity,
  Event,
  Folder,
  Global,
  Graph,
  Indexed,
  IsDeletable,
  IsFrozen,
  IsOwnable,
  IsTracked,
  LikeMembership,
  MaterializationType,
  Node,
  NodeReference,
  NodeType,
  Organization,
  Particle,
  QueryConnection,
  Role,
  RoleType,
  Session,
  Space,
  Spatial,
  StructType,
  Supergraph,
  Team,
  Thread,
  TraitType,
  User,
} from "@/language";
import { Temporal } from "temporal-polyfill";

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
  static metatype: NodeType = NodeType.MEMBERSHIP_EVENT;
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

  readonly id: string;
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
  get createdBy(): Agent | User | null | null {
    const nodePtr: NodeReference | null = this.createdByPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Agent | User | null | null;
    }
    return null;
  }
  readonly createdByPtr: NodeReference | null;
  readonly updatedAt: Temporal.ZonedDateTime;
  get updatedBy(): Agent | User | null | null {
    const nodePtr: NodeReference | null = this.updatedByPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Agent | User | null | null;
    }
    return null;
  }
  readonly updatedByPtr: NodeReference | null;
  get node(): Membership | null {
    const nodePtr: NodeReference | null = this.nodePtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Membership | null;
    }
    return null;
  }

  set node(node: Membership) {
    this.nodePtr = node.toRef();
  }
  nodePtr: NodeReference;
  get joinable(): Folder | Thread | Organization | Space | Team | null {
    const nodePtr: NodeReference | null = this.joinablePtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Folder | Thread | Organization | Space | Team | null;
    }
    return null;
  }

  set joinable(node: Folder | Thread | Organization | Space | Team) {
    this.joinablePtr = node.toRef();
  }
  joinablePtr: NodeReference;
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
    id: string;
    parent?: Space | NodeReference | null;
    space?: Space | NodeReference | null;
    createdAt: Temporal.ZonedDateTime;
    createdBy?: Agent | User | NodeReference | null;
    updatedAt: Temporal.ZonedDateTime;
    updatedBy?: Agent | User | NodeReference | null;
    node: Membership | NodeReference;
    joinable: Folder | Thread | Organization | Space | Team | NodeReference;
    member: Agent | User | NodeReference;
    role?: Role | NodeReference | null;
    roleType: RoleType;
    _session?: Session | null;
    _supergraph?: Supergraph | null;
    _graph?: Graph | null;
    _connection?: QueryConnection | null;
  }) {
    super(
      // id
      options.id,
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
      options.id != null,
    );

    this.id = options.id;
    this.parentPtr =
      options.parent != null
        ? options.parent.metatype == StructType.NODE_REFERENCE
          ? (options.parent as NodeReference)
          : (options.parent as Node).toRef()
        : null;
    this.spacePtr =
      options.space != null
        ? options.space.metatype == StructType.NODE_REFERENCE
          ? (options.space as NodeReference)
          : (options.space as Node).toRef()
        : null;
    this.createdAt = options.createdAt;
    this.createdByPtr =
      options.createdBy != null
        ? options.createdBy.metatype == StructType.NODE_REFERENCE
          ? (options.createdBy as NodeReference)
          : (options.createdBy as Node).toRef()
        : null;
    this.updatedAt = options.updatedAt;
    this.updatedByPtr =
      options.updatedBy != null
        ? options.updatedBy.metatype == StructType.NODE_REFERENCE
          ? (options.updatedBy as NodeReference)
          : (options.updatedBy as Node).toRef()
        : null;
    this.nodePtr =
      options.node != null
        ? options.node.metatype == StructType.NODE_REFERENCE
          ? (options.node as NodeReference)
          : (options.node as Node).toRef()
        : null;
    this.joinablePtr =
      options.joinable != null
        ? options.joinable.metatype == StructType.NODE_REFERENCE
          ? (options.joinable as NodeReference)
          : (options.joinable as Node).toRef()
        : null;
    this.memberPtr =
      options.member != null
        ? options.member.metatype == StructType.NODE_REFERENCE
          ? (options.member as NodeReference)
          : (options.member as Node).toRef()
        : null;
    this.rolePtr =
      options.role != null
        ? options.role.metatype == StructType.NODE_REFERENCE
          ? (options.role as NodeReference)
          : (options.role as Node).toRef()
        : null;
    this.roleType = options.roleType;
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
      nodeType: NodeType.MEMBERSHIP_EVENT,
      id: this.id,
      spaceId: this.spacePtr?.id ?? null,
      _session: this._session,
      _supergraph: this._supergraph,
    });
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
export class Membership
  extends Node
  implements Global, Spatial, Entity, IsTracked, IsDeletable, IsOwnable, LikeMembership
{
  static metatype: NodeType = NodeType.MEMBERSHIP;
  static __traits__: TraitType[] = [
    TraitType.GLOBAL,
    TraitType.SPATIAL,
    TraitType.ENTITY,
    TraitType.TRACKED,
    TraitType.OWNABLE,
    TraitType.DELETABLE,
    TraitType.MEMBERSHIP,
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

  readonly id: string;
  get parent(): Folder | Thread | Organization | Space | Team | null | null {
    const nodePtr: NodeReference | null = this.parentPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Folder | Thread | Organization | Space | Team | null | null;
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
  get createdBy(): Agent | User | null | null {
    const nodePtr: NodeReference | null = this.createdByPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Agent | User | null | null;
    }
    return null;
  }
  readonly createdByPtr: NodeReference | null;
  readonly updatedAt: Temporal.ZonedDateTime;
  get updatedBy(): Agent | User | null | null {
    const nodePtr: NodeReference | null = this.updatedByPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Agent | User | null | null;
    }
    return null;
  }
  readonly updatedByPtr: NodeReference | null;
  readonly deletedAt: Temporal.ZonedDateTime | null;
  get ownedBy(): Role | Agent | Organization | Team | User | null | null {
    const nodePtr: NodeReference | null = this.ownedByPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Role | Agent | Organization | Team | User | null | null;
    }
    return null;
  }

  set ownedBy(node: Role | Agent | Organization | Team | User | null) {
    if (node === null) {
      this.ownedByPtr = null;
    } else {
      this.ownedByPtr = node.toRef();
    }
  }
  ownedByPtr: NodeReference | null;
  get member(): Agent | User | null | null {
    const nodePtr: NodeReference | null = this.memberPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Agent | User | null | null;
    }
    return null;
  }

  set member(node: Agent | User | null) {
    if (node === null) {
      this.memberPtr = null;
    } else {
      this.memberPtr = node.toRef();
    }
  }
  memberPtr: NodeReference | null;
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
    id: string;
    parent?: Folder | Thread | Organization | Space | Team | NodeReference | null;
    space?: Space | NodeReference | null;
    materialization?: MaterializationType;
    createdAt: Temporal.ZonedDateTime;
    createdBy?: Agent | User | NodeReference | null;
    updatedAt: Temporal.ZonedDateTime;
    updatedBy?: Agent | User | NodeReference | null;
    deletedAt?: Temporal.ZonedDateTime | null;
    ownedBy?: Role | Agent | Organization | Team | User | NodeReference | null;
    member?: Agent | User | NodeReference | null;
    role?: Role | NodeReference | null;
    roleType?: RoleType | null;
    _session?: Session | null;
    _supergraph?: Supergraph | null;
    _graph?: Graph | null;
    _connection?: QueryConnection | null;
  }) {
    super(
      // id
      options.id,
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
      options.id != null,
    );

    this.id = options.id;
    this.parentPtr =
      options.parent != null
        ? options.parent.metatype == StructType.NODE_REFERENCE
          ? (options.parent as NodeReference)
          : (options.parent as Node).toRef()
        : null;
    this.spacePtr =
      options.space != null
        ? options.space.metatype == StructType.NODE_REFERENCE
          ? (options.space as NodeReference)
          : (options.space as Node).toRef()
        : null;
    this.materialization = options.materialization ?? MaterializationType.FULL_GRAPH;
    this.createdAt = options.createdAt;
    this.createdByPtr =
      options.createdBy != null
        ? options.createdBy.metatype == StructType.NODE_REFERENCE
          ? (options.createdBy as NodeReference)
          : (options.createdBy as Node).toRef()
        : null;
    this.updatedAt = options.updatedAt;
    this.updatedByPtr =
      options.updatedBy != null
        ? options.updatedBy.metatype == StructType.NODE_REFERENCE
          ? (options.updatedBy as NodeReference)
          : (options.updatedBy as Node).toRef()
        : null;
    this.deletedAt = options.deletedAt ?? null;
    this.ownedByPtr =
      options.ownedBy != null
        ? options.ownedBy.metatype == StructType.NODE_REFERENCE
          ? (options.ownedBy as NodeReference)
          : (options.ownedBy as Node).toRef()
        : null;
    this.memberPtr =
      options.member != null
        ? options.member.metatype == StructType.NODE_REFERENCE
          ? (options.member as NodeReference)
          : (options.member as Node).toRef()
        : null;
    this.rolePtr =
      options.role != null
        ? options.role.metatype == StructType.NODE_REFERENCE
          ? (options.role as NodeReference)
          : (options.role as Node).toRef()
        : null;
    this.roleType = options.roleType ?? null;
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
      nodeType: NodeType.MEMBERSHIP,
      id: this.id,
      spaceId: this.spacePtr?.id ?? null,
      _session: this._session,
      _supergraph: this._supergraph,
    });
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
