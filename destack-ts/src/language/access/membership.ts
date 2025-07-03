import { packProtoTimestamp, unpackProtoTimestamp } from "@destack/grpc";
import type { Role } from "@destack/language/access/role";
import type {
  Graph,
  IsDeletable,
  IsGlobal,
  IsJoinable,
  IsOwnable,
  IsOwner,
  IsSpatial,
  IsSubject,
  NodeReference,
  QueryConnection,
  Session,
  Supergraph,
} from "@destack/language/core";
import {
  Entity,
  EnumType,
  Event,
  Materialization,
  Node,
  NodeType,
  RoleType,
  StructType,
} from "@destack/language/core";
import {
  STRUCT_CLASS_BY_TYPE,
  registerEnumClass,
  registerNodeClass,
} from "@destack/language/registry";
import type { Space } from "@destack/language/space";
import {
  MaterializationProto,
  MembershipJoinedEventProto,
  MembershipLeftEventProto,
  MembershipProto,
  RoleTypeProto,
} from "@destack/proto";
import { base64Decode } from "@destack/utils";
import { hashString } from "@destack/utils/hash";
import { Temporal } from "temporal-polyfill";

/* ==== DESTACK_GENERATED_START:ENUM:20001 ==== */
/**
 * MembershipPermission
 */
export enum MembershipPermission {
  KICK = 10,
  BAN = 11,

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerEnumClass(EnumType.MEMBERSHIP_PERMISSION, MembershipPermission);
/* ==== DESTACK_GENERATED_END:ENUM:20001 ==== */

/* ==== DESTACK_GENERATED_START:NODE:20001 ==== */
/**
 * A Event regarding a Membership.
 */
export abstract class MembershipEvent extends Event {
  static metatype: NodeType = NodeType.MEMBERSHIP_EVENT;

  /**
   * Event.parent
   */
  get parent(): Space | null {
    const nodePtr: NodeReference | null = this.parentPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Space | null;
    }
    return null;
  }
  declare readonly parentPtr: NodeReference | null;

  /**
   * The Space this Node is in.
   */
  get space(): Space | null {
    const nodePtr: NodeReference | null = this.spacePtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Space | null;
    }
    return null;
  }
  declare readonly spacePtr: NodeReference | null;

  /**
   * Event.createdAt
   */
  declare readonly createdAt: Temporal.ZonedDateTime;

  /**
   * Event.createdBy
   */
  get createdBy(): (Node & IsSubject) | null {
    const nodePtr: NodeReference | null = this.createdByPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as (Node & IsSubject) | null;
    }
    return null;
  }
  declare readonly createdByPtr: NodeReference | null;

  /**
   * MembershipEvent.node
   */
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
  declare nodePtr: NodeReference;

  /**
   * MembershipEvent.joinable
   */
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
  declare joinablePtr: NodeReference;

  /**
   * MembershipEvent.member
   */
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
  declare memberPtr: NodeReference;

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerNodeClass(NodeType.MEMBERSHIP_EVENT, MembershipEvent);
/* ==== DESTACK_GENERATED_END:NODE:20001 ==== */

/* ==== DESTACK_GENERATED_START:NODE:20002 ==== */
/**
 * A Event regarding a Membership Join.
 */
export class MembershipJoinedEvent extends MembershipEvent {
  static metatype: NodeType = NodeType.MEMBERSHIP_JOINED_EVENT;

  /**
   * Event.parent
   */
  get parent(): Space | null {
    const nodePtr: NodeReference | null = this.parentPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Space | null;
    }
    return null;
  }
  readonly parentPtr: NodeReference | null;

  /**
   * The Space this Node is in.
   */
  get space(): Space | null {
    const nodePtr: NodeReference | null = this.spacePtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Space | null;
    }
    return null;
  }
  readonly spacePtr: NodeReference | null;

  /**
   * Event.createdAt
   */
  readonly createdAt: Temporal.ZonedDateTime;

  /**
   * Event.createdBy
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
   * MembershipEvent.node
   */
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

  /**
   * MembershipEvent.joinable
   */
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

  /**
   * MembershipEvent.member
   */
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

  /**
   * MembershipJoinedEvent.role
   */
  get role(): Role | null {
    const nodePtr: NodeReference | null = this.rolePtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Role | null;
    }
    return null;
  }
  set role(node: Role) {
    this.rolePtr = node.toRef();
  }
  rolePtr: NodeReference;

  /**
   * MembershipJoinedEvent.roleType
   */
  roleType: RoleType;

  constructor(options: {
    id?: string;
    parent?: Space | NodeReference | null;
    space?: Space | NodeReference | null;
    createdAt?: Temporal.ZonedDateTime;
    createdBy?: (Node & IsSubject) | NodeReference | null;
    node: Membership | NodeReference;
    joinable: (Node & IsJoinable) | NodeReference;
    member: (Node & IsSubject) | NodeReference;
    role: Role | NodeReference;
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
    if (_parent != null && _parent.metatype != StructType.NODE_REFERENCE) {
      _parent = (_parent as Node).toRef();
    }
    this.parentPtr = _parent;
    let _space = options.space ?? null;
    if (_space != null && _space.metatype != StructType.NODE_REFERENCE) {
      _space = (_space as Node).toRef();
    }
    this.spacePtr = _space;
    let _node = options.node;
    if (_node != null && _node.metatype != StructType.NODE_REFERENCE) {
      _node = (_node as Node).toRef();
    }
    if (_node === null) {
      throw new Error(`MembershipJoinedEvent.node is required`);
    }
    this.nodePtr = _node;
    let _joinable = options.joinable;
    if (_joinable != null && _joinable.metatype != StructType.NODE_REFERENCE) {
      _joinable = (_joinable as Node).toRef();
    }
    if (_joinable === null) {
      throw new Error(`MembershipJoinedEvent.joinable is required`);
    }
    this.joinablePtr = _joinable;
    let _member = options.member;
    if (_member != null && _member.metatype != StructType.NODE_REFERENCE) {
      _member = (_member as Node).toRef();
    }
    if (_member === null) {
      throw new Error(`MembershipJoinedEvent.member is required`);
    }
    this.memberPtr = _member;
    let _role = options.role;
    if (_role != null && _role.metatype != StructType.NODE_REFERENCE) {
      _role = (_role as Node).toRef();
    }
    if (_role === null) {
      throw new Error(`MembershipJoinedEvent.role is required`);
    }
    this.rolePtr = _role;
    let _roleType = options.roleType;
    if (_roleType === null) {
      throw new Error(`MembershipJoinedEvent.roleType is required`);
    }
    this.roleType = _roleType;

    // identity
    if (options.id == null) {
      const now = Temporal.Now.zonedDateTimeISO("UTC");
      this.createdAt = now;
      this.createdByPtr = null;
    } else {
      if (options.createdAt == null) {
        throw new Error(`{cls.__name__}.createdAt is required for existing Events`);
      }
      this.createdAt = options.createdAt;
      this.createdByPtr =
        options.createdBy != null
          ? options.createdBy.metatype == StructType.NODE_REFERENCE
            ? (options.createdBy as NodeReference)
            : (options.createdBy as Node).toRef()
          : null;
    }
  }

  equals(other: any): boolean {
    if (!(this.metatype === other.metatype)) {
      return false;
    }
    if (!(this.rolePtr.id === other.rolePtr.id)) {
      return false;
    }
    if (!(this.roleType === other.roleType)) {
      return false;
    }
    if (!(this.nodePtr.id === other.nodePtr.id)) {
      return false;
    }
    if (!(this.joinablePtr.id === other.joinablePtr.id)) {
      return false;
    }
    if (!(this.memberPtr.id === other.memberPtr.id)) {
      return false;
    }
    if (!(this.spacePtr?.id === other.spacePtr?.id)) {
      return false;
    }
    return true;
  }

  hash(): number {
    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    h = (h * 31 + hashString(this.rolePtr.id)) & 0xffffffff;
    h = (h * 31 + this.roleType) & 0xffffffff;
    h = (h * 31 + hashString(this.nodePtr.id)) & 0xffffffff;
    h = (h * 31 + hashString(this.joinablePtr.id)) & 0xffffffff;
    h = (h * 31 + hashString(this.memberPtr.id)) & 0xffffffff;
    if (this.parentPtr !== null) {
      h = (h * 31 + hashString(this.parentPtr.id)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.createdAt.toString({ timeZoneName: "never" }))) & 0xffffffff;
    if (this.createdByPtr !== null) {
      h = (h * 31 + hashString(this.createdByPtr.id)) & 0xffffffff;
    }
    if (this.spacePtr !== null) {
      h = (h * 31 + hashString(this.spacePtr.id)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.id.toString())) & 0xffffffff;

    return h;
  }

  validate(): void {
    throw new Error("not implemented");
  }

  __toRef__(): NodeReference {
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    return new _NodeReference({
      type: NodeType.MEMBERSHIP_JOINED_EVENT,
      id: this.id,
      spaceId: this.spacePtr?.id ?? null,
      _session: this._session,
      _supergraph: this._supergraph,
    });
  }

  get _pathKey(): string {
    return "MembershipJoinedEvent[id={this.id}]";
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

  repr(): string {
    return `<MembershipJoinedEvent '${this.path}'>`;
  }

  toValue(): { [key: string]: any } {
    return MembershipJoinedEvent.__packValue__(this);
  }

  static __packValue__(object: MembershipJoinedEvent): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 20002;
    objectValue["2"] = String(object.id);
    if (object.parentPtr != null) {
      objectValue["3"] = object.parentPtr.toValue();
    }
    if (object.spacePtr != null) {
      objectValue["5"] = object.spacePtr.toValue();
    }
    objectValue["20"] = object.createdAt.toString({ timeZoneName: "never" });
    if (object.createdByPtr != null) {
      objectValue["21"] = object.createdByPtr.toValue();
    }
    objectValue["101"] = object.nodePtr.toValue();
    objectValue["102"] = object.joinablePtr.toValue();
    objectValue["103"] = object.memberPtr.toValue();
    objectValue["110"] = object.rolePtr.toValue();
    objectValue["111"] = object.roleType;
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): MembershipJoinedEvent {
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    const parentPtrValue = objectValue["3"];
    const unpackedParentPtr =
      parentPtrValue != undefined
        ? _NodeReference.fromValue(parentPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const createdByPtrValue = objectValue["21"];
    const unpackedCreatedByPtr =
      createdByPtrValue != undefined
        ? _NodeReference.fromValue(createdByPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const spacePtrValue = objectValue["5"];
    const unpackedSpacePtr =
      spacePtrValue != undefined
        ? _NodeReference.fromValue(spacePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    return new MembershipJoinedEvent({
      role: _NodeReference.fromValue(
        objectValue["110"],
        _session,
        _supergraph,
        _graph,
        _connection,
      ),
      roleType: Number(objectValue["111"]),
      node: _NodeReference.fromValue(
        objectValue["101"],
        _session,
        _supergraph,
        _graph,
        _connection,
      ),
      joinable: _NodeReference.fromValue(
        objectValue["102"],
        _session,
        _supergraph,
        _graph,
        _connection,
      ),
      member: _NodeReference.fromValue(
        objectValue["103"],
        _session,
        _supergraph,
        _graph,
        _connection,
      ),
      parent: unpackedParentPtr,
      createdAt: Temporal.Instant.from(objectValue["20"]).toZonedDateTimeISO("UTC"),
      createdBy: unpackedCreatedByPtr,
      space: unpackedSpacePtr,
      id: String(objectValue["2"]),
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
  ): MembershipJoinedEvent {
    return MembershipJoinedEvent.__unpackValue__(
      objectValue,
      _session,
      _supergraph,
      _graph,
      _connection,
    );
  }

  toProto(): MembershipJoinedEventProto {
    return MembershipJoinedEvent.__packProto__(this);
  }

  static __packProto__(object: MembershipJoinedEvent): MembershipJoinedEventProto {
    const objectProto: Partial<MembershipJoinedEventProto> = { metatype: 20002 };
    objectProto.id = String(object.id);
    if (object.parentPtr != null) {
      objectProto.parentPtr = object.parentPtr.toProto();
    }
    if (object.spacePtr != null) {
      objectProto.spacePtr = object.spacePtr.toProto();
    }
    objectProto.createdAt = packProtoTimestamp(object.createdAt);
    if (object.createdByPtr != null) {
      objectProto.createdByPtr = object.createdByPtr.toProto();
    }
    objectProto.nodePtr = object.nodePtr.toProto();
    objectProto.joinablePtr = object.joinablePtr.toProto();
    objectProto.memberPtr = object.memberPtr.toProto();
    objectProto.rolePtr = object.rolePtr.toProto();
    objectProto.roleType = Number(object.roleType) as RoleTypeProto;
    return objectProto as MembershipJoinedEventProto;
  }

  static __unpackProto__(
    objectProto: MembershipJoinedEventProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): MembershipJoinedEvent {
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    return new MembershipJoinedEvent({
      role: _NodeReference.fromProto(
        objectProto.rolePtr!,
        _session,
        _supergraph,
        _graph,
        _connection,
      ),
      roleType: Number(objectProto.roleType) as RoleType,
      node: _NodeReference.fromProto(
        objectProto.nodePtr!,
        _session,
        _supergraph,
        _graph,
        _connection,
      ),
      joinable: _NodeReference.fromProto(
        objectProto.joinablePtr!,
        _session,
        _supergraph,
        _graph,
        _connection,
      ),
      member: _NodeReference.fromProto(
        objectProto.memberPtr!,
        _session,
        _supergraph,
        _graph,
        _connection,
      ),
      parent:
        objectProto.parentPtr != undefined
          ? _NodeReference.fromProto(
              objectProto.parentPtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      createdAt: unpackProtoTimestamp(objectProto.createdAt!),
      createdBy:
        objectProto.createdByPtr != undefined
          ? _NodeReference.fromProto(
              objectProto.createdByPtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      space:
        objectProto.spacePtr != undefined
          ? _NodeReference.fromProto(
              objectProto.spacePtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      id: String(objectProto.id),
      _session,
      _graph,
      _connection,
    });
  }

  static fromProto(
    objectProto: MembershipJoinedEventProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): MembershipJoinedEvent {
    return MembershipJoinedEvent.__unpackProto__(
      objectProto,
      _session,
      _supergraph,
      _graph,
      _connection,
    );
  }

  static fromProtoString(packedProtoString: string): MembershipJoinedEvent {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = MembershipJoinedEventProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerNodeClass(NodeType.MEMBERSHIP_JOINED_EVENT, MembershipJoinedEvent);
/* ==== DESTACK_GENERATED_END:NODE:20002 ==== */

/* ==== DESTACK_GENERATED_START:NODE:20003 ==== */
/**
 * A Event regarding a Membership Leave.
 */
export class MembershipLeftEvent extends MembershipEvent {
  static metatype: NodeType = NodeType.MEMBERSHIP_LEFT_EVENT;

  /**
   * Event.parent
   */
  get parent(): Space | null {
    const nodePtr: NodeReference | null = this.parentPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Space | null;
    }
    return null;
  }
  readonly parentPtr: NodeReference | null;

  /**
   * The Space this Node is in.
   */
  get space(): Space | null {
    const nodePtr: NodeReference | null = this.spacePtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Space | null;
    }
    return null;
  }
  readonly spacePtr: NodeReference | null;

  /**
   * Event.createdAt
   */
  readonly createdAt: Temporal.ZonedDateTime;

  /**
   * Event.createdBy
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
   * MembershipEvent.node
   */
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

  /**
   * MembershipEvent.joinable
   */
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

  /**
   * MembershipEvent.member
   */
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

  constructor(options: {
    id?: string;
    parent?: Space | NodeReference | null;
    space?: Space | NodeReference | null;
    createdAt?: Temporal.ZonedDateTime;
    createdBy?: (Node & IsSubject) | NodeReference | null;
    node: Membership | NodeReference;
    joinable: (Node & IsJoinable) | NodeReference;
    member: (Node & IsSubject) | NodeReference;
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
    if (_parent != null && _parent.metatype != StructType.NODE_REFERENCE) {
      _parent = (_parent as Node).toRef();
    }
    this.parentPtr = _parent;
    let _space = options.space ?? null;
    if (_space != null && _space.metatype != StructType.NODE_REFERENCE) {
      _space = (_space as Node).toRef();
    }
    this.spacePtr = _space;
    let _node = options.node;
    if (_node != null && _node.metatype != StructType.NODE_REFERENCE) {
      _node = (_node as Node).toRef();
    }
    if (_node === null) {
      throw new Error(`MembershipLeftEvent.node is required`);
    }
    this.nodePtr = _node;
    let _joinable = options.joinable;
    if (_joinable != null && _joinable.metatype != StructType.NODE_REFERENCE) {
      _joinable = (_joinable as Node).toRef();
    }
    if (_joinable === null) {
      throw new Error(`MembershipLeftEvent.joinable is required`);
    }
    this.joinablePtr = _joinable;
    let _member = options.member;
    if (_member != null && _member.metatype != StructType.NODE_REFERENCE) {
      _member = (_member as Node).toRef();
    }
    if (_member === null) {
      throw new Error(`MembershipLeftEvent.member is required`);
    }
    this.memberPtr = _member;

    // identity
    if (options.id == null) {
      const now = Temporal.Now.zonedDateTimeISO("UTC");
      this.createdAt = now;
      this.createdByPtr = null;
    } else {
      if (options.createdAt == null) {
        throw new Error(`{cls.__name__}.createdAt is required for existing Events`);
      }
      this.createdAt = options.createdAt;
      this.createdByPtr =
        options.createdBy != null
          ? options.createdBy.metatype == StructType.NODE_REFERENCE
            ? (options.createdBy as NodeReference)
            : (options.createdBy as Node).toRef()
          : null;
    }
  }

  equals(other: any): boolean {
    if (!(this.metatype === other.metatype)) {
      return false;
    }
    if (!(this.nodePtr.id === other.nodePtr.id)) {
      return false;
    }
    if (!(this.joinablePtr.id === other.joinablePtr.id)) {
      return false;
    }
    if (!(this.memberPtr.id === other.memberPtr.id)) {
      return false;
    }
    if (!(this.spacePtr?.id === other.spacePtr?.id)) {
      return false;
    }
    return true;
  }

  hash(): number {
    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    h = (h * 31 + hashString(this.nodePtr.id)) & 0xffffffff;
    h = (h * 31 + hashString(this.joinablePtr.id)) & 0xffffffff;
    h = (h * 31 + hashString(this.memberPtr.id)) & 0xffffffff;
    if (this.parentPtr !== null) {
      h = (h * 31 + hashString(this.parentPtr.id)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.createdAt.toString({ timeZoneName: "never" }))) & 0xffffffff;
    if (this.createdByPtr !== null) {
      h = (h * 31 + hashString(this.createdByPtr.id)) & 0xffffffff;
    }
    if (this.spacePtr !== null) {
      h = (h * 31 + hashString(this.spacePtr.id)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.id.toString())) & 0xffffffff;

    return h;
  }

  validate(): void {
    throw new Error("not implemented");
  }

  __toRef__(): NodeReference {
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    return new _NodeReference({
      type: NodeType.MEMBERSHIP_LEFT_EVENT,
      id: this.id,
      spaceId: this.spacePtr?.id ?? null,
      _session: this._session,
      _supergraph: this._supergraph,
    });
  }

  get _pathKey(): string {
    return "MembershipLeftEvent[id={this.id}]";
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

  repr(): string {
    return `<MembershipLeftEvent '${this.path}'>`;
  }

  toValue(): { [key: string]: any } {
    return MembershipLeftEvent.__packValue__(this);
  }

  static __packValue__(object: MembershipLeftEvent): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 20003;
    objectValue["2"] = String(object.id);
    if (object.parentPtr != null) {
      objectValue["3"] = object.parentPtr.toValue();
    }
    if (object.spacePtr != null) {
      objectValue["5"] = object.spacePtr.toValue();
    }
    objectValue["20"] = object.createdAt.toString({ timeZoneName: "never" });
    if (object.createdByPtr != null) {
      objectValue["21"] = object.createdByPtr.toValue();
    }
    objectValue["101"] = object.nodePtr.toValue();
    objectValue["102"] = object.joinablePtr.toValue();
    objectValue["103"] = object.memberPtr.toValue();
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): MembershipLeftEvent {
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    const parentPtrValue = objectValue["3"];
    const unpackedParentPtr =
      parentPtrValue != undefined
        ? _NodeReference.fromValue(parentPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const createdByPtrValue = objectValue["21"];
    const unpackedCreatedByPtr =
      createdByPtrValue != undefined
        ? _NodeReference.fromValue(createdByPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const spacePtrValue = objectValue["5"];
    const unpackedSpacePtr =
      spacePtrValue != undefined
        ? _NodeReference.fromValue(spacePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    return new MembershipLeftEvent({
      node: _NodeReference.fromValue(
        objectValue["101"],
        _session,
        _supergraph,
        _graph,
        _connection,
      ),
      joinable: _NodeReference.fromValue(
        objectValue["102"],
        _session,
        _supergraph,
        _graph,
        _connection,
      ),
      member: _NodeReference.fromValue(
        objectValue["103"],
        _session,
        _supergraph,
        _graph,
        _connection,
      ),
      parent: unpackedParentPtr,
      createdAt: Temporal.Instant.from(objectValue["20"]).toZonedDateTimeISO("UTC"),
      createdBy: unpackedCreatedByPtr,
      space: unpackedSpacePtr,
      id: String(objectValue["2"]),
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
  ): MembershipLeftEvent {
    return MembershipLeftEvent.__unpackValue__(
      objectValue,
      _session,
      _supergraph,
      _graph,
      _connection,
    );
  }

  toProto(): MembershipLeftEventProto {
    return MembershipLeftEvent.__packProto__(this);
  }

  static __packProto__(object: MembershipLeftEvent): MembershipLeftEventProto {
    const objectProto: Partial<MembershipLeftEventProto> = { metatype: 20003 };
    objectProto.id = String(object.id);
    if (object.parentPtr != null) {
      objectProto.parentPtr = object.parentPtr.toProto();
    }
    if (object.spacePtr != null) {
      objectProto.spacePtr = object.spacePtr.toProto();
    }
    objectProto.createdAt = packProtoTimestamp(object.createdAt);
    if (object.createdByPtr != null) {
      objectProto.createdByPtr = object.createdByPtr.toProto();
    }
    objectProto.nodePtr = object.nodePtr.toProto();
    objectProto.joinablePtr = object.joinablePtr.toProto();
    objectProto.memberPtr = object.memberPtr.toProto();
    return objectProto as MembershipLeftEventProto;
  }

  static __unpackProto__(
    objectProto: MembershipLeftEventProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): MembershipLeftEvent {
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    return new MembershipLeftEvent({
      node: _NodeReference.fromProto(
        objectProto.nodePtr!,
        _session,
        _supergraph,
        _graph,
        _connection,
      ),
      joinable: _NodeReference.fromProto(
        objectProto.joinablePtr!,
        _session,
        _supergraph,
        _graph,
        _connection,
      ),
      member: _NodeReference.fromProto(
        objectProto.memberPtr!,
        _session,
        _supergraph,
        _graph,
        _connection,
      ),
      parent:
        objectProto.parentPtr != undefined
          ? _NodeReference.fromProto(
              objectProto.parentPtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      createdAt: unpackProtoTimestamp(objectProto.createdAt!),
      createdBy:
        objectProto.createdByPtr != undefined
          ? _NodeReference.fromProto(
              objectProto.createdByPtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      space:
        objectProto.spacePtr != undefined
          ? _NodeReference.fromProto(
              objectProto.spacePtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      id: String(objectProto.id),
      _session,
      _graph,
      _connection,
    });
  }

  static fromProto(
    objectProto: MembershipLeftEventProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): MembershipLeftEvent {
    return MembershipLeftEvent.__unpackProto__(
      objectProto,
      _session,
      _supergraph,
      _graph,
      _connection,
    );
  }

  static fromProtoString(packedProtoString: string): MembershipLeftEvent {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = MembershipLeftEventProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerNodeClass(NodeType.MEMBERSHIP_LEFT_EVENT, MembershipLeftEvent);
/* ==== DESTACK_GENERATED_END:NODE:20003 ==== */

/* ==== DESTACK_GENERATED_START:NODE:20000 ==== */
/**
 * A Membership of a Subject in a Joinable.
 */
export class Membership extends Entity implements IsGlobal, IsSpatial, IsOwnable, IsDeletable {
  static metatype: NodeType = NodeType.MEMBERSHIP;

  /**
   * Membership.parent
   */
  get parent(): (Node & IsJoinable) | null {
    const nodePtr: NodeReference | null = this.parentPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as (Node & IsJoinable) | null;
    }
    return null;
  }
  readonly parentPtr: NodeReference | null;

  /**
   * The Space this Node is in.
   */
  get space(): Space | null {
    const nodePtr: NodeReference | null = this.spacePtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Space | null;
    }
    return null;
  }
  readonly spacePtr: NodeReference | null;

  /**
   * Entity.materialization
   */
  readonly materialization: Materialization;

  /**
   * Entity.createdAt
   */
  readonly createdAt: Temporal.ZonedDateTime;

  /**
   * Entity.createdBy
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
   * Entity.updatedAt
   */
  readonly updatedAt: Temporal.ZonedDateTime;

  /**
   * Entity.updatedBy
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
   * IsDeletable.deletedAt
   */
  readonly deletedAt: Temporal.ZonedDateTime | null;

  /**
   * IsOwnable.ownedBy
   */
  get ownedBy(): (Node & IsOwner) | null {
    const nodePtr: NodeReference | null = this.ownedByPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as (Node & IsOwner) | null;
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

  /**
   * Membership.member
   */
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

  /**
   * Membership.role
   */
  get role(): Role | null {
    const nodePtr: NodeReference | null = this.rolePtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Role | null;
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

  /**
   * Membership.roleType
   */
  roleType: RoleType | null;

  constructor(options: {
    id?: string;
    parent?: (Node & IsJoinable) | NodeReference | null;
    space?: Space | NodeReference | null;
    materialization?: Materialization;
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
    if (_parent != null && _parent.metatype != StructType.NODE_REFERENCE) {
      _parent = (_parent as Node).toRef();
    }
    this.parentPtr = _parent;
    let _space = options.space ?? null;
    if (_space != null && _space.metatype != StructType.NODE_REFERENCE) {
      _space = (_space as Node).toRef();
    }
    this.spacePtr = _space;
    let _materialization = options.materialization ?? null;
    if (_materialization === null) {
      _materialization = 3 /* Materialization.FULL */;
    }
    if (_materialization === null) {
      throw new Error(`Membership.materialization is required`);
    }
    this.materialization = _materialization;
    let _deletedAt = options.deletedAt ?? null;
    this.deletedAt = _deletedAt;
    let _ownedBy = options.ownedBy ?? null;
    if (_ownedBy != null && _ownedBy.metatype != StructType.NODE_REFERENCE) {
      _ownedBy = (_ownedBy as Node).toRef();
    }
    this.ownedByPtr = _ownedBy;
    let _member = options.member;
    if (_member != null && _member.metatype != StructType.NODE_REFERENCE) {
      _member = (_member as Node).toRef();
    }
    if (_member === null) {
      throw new Error(`Membership.member is required`);
    }
    this.memberPtr = _member;
    let _role = options.role ?? null;
    if (_role != null && _role.metatype != StructType.NODE_REFERENCE) {
      _role = (_role as Node).toRef();
    }
    this.rolePtr = _role;
    let _roleType = options.roleType ?? null;
    this.roleType = _roleType;

    // identity
    if (options.id == null) {
      const now = Temporal.Now.zonedDateTimeISO("UTC");
      this.createdAt = now;
      this.createdByPtr = null;
      this.updatedAt = now;
      this.updatedByPtr = null;
    } else {
      if (options.createdAt == null || options.updatedAt == null) {
        throw new Error(
          `{cls.__name__}.createdAt and {cls.__name__}.updatedAt are required for existing Nodes`,
        );
      }
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
    }
  }

  equals(other: any): boolean {
    if (!(this.metatype === other.metatype)) {
      return false;
    }
    if (!(this.memberPtr.id === other.memberPtr.id)) {
      return false;
    }
    if (!(this.rolePtr?.id === other.rolePtr?.id)) {
      return false;
    }
    if (!(this.roleType === other.roleType)) {
      return false;
    }
    if (!(this.spacePtr?.id === other.spacePtr?.id)) {
      return false;
    }
    if (!(this.ownedByPtr?.id === other.ownedByPtr?.id)) {
      return false;
    }
    return true;
  }

  hash(): number {
    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    if (this.parentPtr !== null) {
      h = (h * 31 + hashString(this.parentPtr.id)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.memberPtr.id)) & 0xffffffff;
    if (this.rolePtr !== null) {
      h = (h * 31 + hashString(this.rolePtr.id)) & 0xffffffff;
    }
    if (this.roleType !== null) {
      h = (h * 31 + this.roleType) & 0xffffffff;
    }
    if (this.spacePtr !== null) {
      h = (h * 31 + hashString(this.spacePtr.id)) & 0xffffffff;
    }
    if (this.ownedByPtr !== null) {
      h = (h * 31 + hashString(this.ownedByPtr.id)) & 0xffffffff;
    }
    if (this.deletedAt !== null) {
      h = (h * 31 + hashString(this.deletedAt.toString({ timeZoneName: "never" }))) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.createdAt.toString({ timeZoneName: "never" }))) & 0xffffffff;
    if (this.createdByPtr !== null) {
      h = (h * 31 + hashString(this.createdByPtr.id)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.updatedAt.toString({ timeZoneName: "never" }))) & 0xffffffff;
    if (this.updatedByPtr !== null) {
      h = (h * 31 + hashString(this.updatedByPtr.id)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.id.toString())) & 0xffffffff;

    return h;
  }

  validate(): void {
    throw new Error("not implemented");
  }

  __toRef__(): NodeReference {
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    return new _NodeReference({
      type: NodeType.MEMBERSHIP,
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

  repr(): string {
    const propertyReprs: string[] = [];
    if (this.ownedBy !== null) {
      propertyReprs.push(`ownedBy=${this.ownedBy?.repr()}`);
    }
    if (propertyReprs.length > 0) {
      return `<Membership '${this.path}' ${propertyReprs.join(" ")}>`;
    } else {
      return `<Membership '${this.path}'>`;
    }
  }

  toValue(): { [key: string]: any } {
    return Membership.__packValue__(this);
  }

  static __packValue__(object: Membership): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 20000;
    objectValue["2"] = String(object.id);
    if (object.parentPtr != null) {
      objectValue["3"] = object.parentPtr.toValue();
    }
    if (object.spacePtr != null) {
      objectValue["5"] = object.spacePtr.toValue();
    }
    objectValue["10"] = object.materialization;
    objectValue["20"] = object.createdAt.toString({ timeZoneName: "never" });
    if (object.createdByPtr != null) {
      objectValue["21"] = object.createdByPtr.toValue();
    }
    objectValue["22"] = object.updatedAt.toString({ timeZoneName: "never" });
    if (object.updatedByPtr != null) {
      objectValue["23"] = object.updatedByPtr.toValue();
    }
    if (object.deletedAt != null) {
      objectValue["25"] = object.deletedAt.toString({ timeZoneName: "never" });
    }
    if (object.ownedByPtr != null) {
      objectValue["28"] = object.ownedByPtr.toValue();
    }
    objectValue["110"] = object.memberPtr.toValue();
    if (object.rolePtr != null) {
      objectValue["111"] = object.rolePtr.toValue();
    }
    if (object.roleType != null) {
      objectValue["112"] = object.roleType;
    }
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Membership {
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    const parentPtrValue = objectValue["3"];
    const unpackedParentPtr =
      parentPtrValue != undefined
        ? _NodeReference.fromValue(parentPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const rolePtrValue = objectValue["111"];
    const unpackedRolePtr =
      rolePtrValue != undefined
        ? _NodeReference.fromValue(rolePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const roleTypeValue = objectValue["112"];
    const unpackedRoleType = roleTypeValue != undefined ? Number(roleTypeValue) : null;
    const spacePtrValue = objectValue["5"];
    const unpackedSpacePtr =
      spacePtrValue != undefined
        ? _NodeReference.fromValue(spacePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const ownedByPtrValue = objectValue["28"];
    const unpackedOwnedByPtr =
      ownedByPtrValue != undefined
        ? _NodeReference.fromValue(ownedByPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const deletedAtValue = objectValue["25"];
    const unpackedDeletedAt =
      deletedAtValue != undefined
        ? Temporal.Instant.from(deletedAtValue).toZonedDateTimeISO("UTC")
        : null;
    const createdByPtrValue = objectValue["21"];
    const unpackedCreatedByPtr =
      createdByPtrValue != undefined
        ? _NodeReference.fromValue(createdByPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const updatedByPtrValue = objectValue["23"];
    const unpackedUpdatedByPtr =
      updatedByPtrValue != undefined
        ? _NodeReference.fromValue(updatedByPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    return new Membership({
      parent: unpackedParentPtr,
      member: _NodeReference.fromValue(
        objectValue["110"],
        _session,
        _supergraph,
        _graph,
        _connection,
      ),
      role: unpackedRolePtr,
      roleType: unpackedRoleType,
      space: unpackedSpacePtr,
      ownedBy: unpackedOwnedByPtr,
      deletedAt: unpackedDeletedAt,
      createdAt: Temporal.Instant.from(objectValue["20"]).toZonedDateTimeISO("UTC"),
      createdBy: unpackedCreatedByPtr,
      updatedAt: Temporal.Instant.from(objectValue["22"]).toZonedDateTimeISO("UTC"),
      updatedBy: unpackedUpdatedByPtr,
      materialization: Number(objectValue["10"]),
      id: String(objectValue["2"]),
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
  ): Membership {
    return Membership.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
  }

  toProto(): MembershipProto {
    return Membership.__packProto__(this);
  }

  static __packProto__(object: Membership): MembershipProto {
    const objectProto: Partial<MembershipProto> = { metatype: 20000 };
    objectProto.id = String(object.id);
    if (object.parentPtr != null) {
      objectProto.parentPtr = object.parentPtr.toProto();
    }
    if (object.spacePtr != null) {
      objectProto.spacePtr = object.spacePtr.toProto();
    }
    objectProto.materialization = Number(object.materialization) as MaterializationProto;
    objectProto.createdAt = packProtoTimestamp(object.createdAt);
    if (object.createdByPtr != null) {
      objectProto.createdByPtr = object.createdByPtr.toProto();
    }
    objectProto.updatedAt = packProtoTimestamp(object.updatedAt);
    if (object.updatedByPtr != null) {
      objectProto.updatedByPtr = object.updatedByPtr.toProto();
    }
    if (object.deletedAt != null) {
      objectProto.deletedAt = packProtoTimestamp(object.deletedAt);
    }
    if (object.ownedByPtr != null) {
      objectProto.ownedByPtr = object.ownedByPtr.toProto();
    }
    objectProto.memberPtr = object.memberPtr.toProto();
    if (object.rolePtr != null) {
      objectProto.rolePtr = object.rolePtr.toProto();
    }
    if (object.roleType != null) {
      objectProto.roleType = Number(object.roleType) as RoleTypeProto;
    }
    return objectProto as MembershipProto;
  }

  static __unpackProto__(
    objectProto: MembershipProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Membership {
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    return new Membership({
      parent:
        objectProto.parentPtr != undefined
          ? _NodeReference.fromProto(
              objectProto.parentPtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      member: _NodeReference.fromProto(
        objectProto.memberPtr!,
        _session,
        _supergraph,
        _graph,
        _connection,
      ),
      role:
        objectProto.rolePtr != undefined
          ? _NodeReference.fromProto(
              objectProto.rolePtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      roleType:
        objectProto.roleType != undefined ? (Number(objectProto.roleType) as RoleType) : null,
      space:
        objectProto.spacePtr != undefined
          ? _NodeReference.fromProto(
              objectProto.spacePtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      ownedBy:
        objectProto.ownedByPtr != undefined
          ? _NodeReference.fromProto(
              objectProto.ownedByPtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      deletedAt:
        objectProto.deletedAt != undefined ? unpackProtoTimestamp(objectProto.deletedAt!) : null,
      createdAt: unpackProtoTimestamp(objectProto.createdAt!),
      createdBy:
        objectProto.createdByPtr != undefined
          ? _NodeReference.fromProto(
              objectProto.createdByPtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      updatedAt: unpackProtoTimestamp(objectProto.updatedAt!),
      updatedBy:
        objectProto.updatedByPtr != undefined
          ? _NodeReference.fromProto(
              objectProto.updatedByPtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      materialization: Number(objectProto.materialization) as Materialization,
      id: String(objectProto.id),
      _session,
      _graph,
      _connection,
    });
  }

  static fromProto(
    objectProto: MembershipProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Membership {
    return Membership.__unpackProto__(objectProto, _session, _supergraph, _graph, _connection);
  }

  static fromProtoString(packedProtoString: string): Membership {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = MembershipProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerNodeClass(NodeType.MEMBERSHIP, Membership);
/* ==== DESTACK_GENERATED_END:NODE:20000 ==== */
