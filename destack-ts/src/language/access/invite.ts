import { packProtoTimestamp, unpackProtoTimestamp } from "@destack/grpc";
import { Role } from "@destack/language/access";
import { Graph, NodeReference, QueryConnection, Session, Supergraph } from "@destack/language/core";
import {
  IsDeletable,
  IsGlobal,
  IsJoinable,
  IsOwnable,
  IsOwner,
  IsSpatial,
  IsSubject,
  Node,
  NodeType,
  RoleType,
  StructType,
} from "@destack/language/core/builtin";
import { Entity, Event } from "@destack/language/core/common";
import { registerNodeClass } from "@destack/language/registry";
import { Space } from "@destack/language/space";
import {
  InviteAcceptedEventProto,
  InviteProto,
  InviteRejectedEventProto,
  InviteRescindedEventProto,
  InviteSentEventProto,
  RoleTypeProto,
} from "@destack/proto";
import { base64Decode } from "@destack/utils";
import { hashString } from "@destack/utils/hash";
import { Temporal } from "temporal-polyfill";

/* ==== DESTACK_GENERATED_START:NODE:521 ==== */
/**
 * A Event regarding an Invite.
 */
export abstract class InviteEvent extends Event {
  static metatype: NodeType = NodeType.INVITE_EVENT;

  /**
   * IsSpatial.parent
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
   * InviteEvent.node
   */
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
  declare nodePtr: NodeReference;

  /**
   * InviteEvent.joinable
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
   * InviteEvent.member
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
registerNodeClass(NodeType.INVITE_EVENT, InviteEvent);
/* ==== DESTACK_GENERATED_END:NODE:521 ==== */

/* ==== DESTACK_GENERATED_START:NODE:522 ==== */
/**
 * An Invite was sent.
 */
export class InviteSentEvent extends InviteEvent {
  static metatype: NodeType = NodeType.INVITE_SENT_EVENT;

  /**
   * IsSpatial.parent
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
   * InviteEvent.node
   */
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

  /**
   * InviteEvent.joinable
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
   * InviteEvent.member
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
   * InviteSentEvent.role
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
   * InviteSentEvent.roleType
   */
  roleType: RoleType;

  constructor(options: {
    id?: string;
    parent?: Space | NodeReference | null;
    space?: Space | NodeReference | null;
    createdAt?: Temporal.ZonedDateTime;
    createdBy?: (Node & IsSubject) | NodeReference | null;
    node: Invite | NodeReference;
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
      throw new Error(`InviteSentEvent.node is required`);
    }
    this.nodePtr = _node;
    let _joinable = options.joinable;
    if (_joinable != null && _joinable instanceof Node) {
      _joinable = _joinable.toRef();
    }
    if (_joinable === null) {
      throw new Error(`InviteSentEvent.joinable is required`);
    }
    this.joinablePtr = _joinable;
    let _member = options.member;
    if (_member != null && _member instanceof Node) {
      _member = _member.toRef();
    }
    if (_member === null) {
      throw new Error(`InviteSentEvent.member is required`);
    }
    this.memberPtr = _member;
    let _role = options.role;
    if (_role != null && _role instanceof Node) {
      _role = _role.toRef();
    }
    if (_role === null) {
      throw new Error(`InviteSentEvent.role is required`);
    }
    this.rolePtr = _role;
    let _roleType = options.roleType;
    if (_roleType === null) {
      throw new Error(`InviteSentEvent.roleType is required`);
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
          ? options.createdBy instanceof Node
            ? options.createdBy.toRef()
            : options.createdBy
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
    h = (h * 31 + hashString(this.createdAt.toString({ timeZoneName: "never" }))) & 0xffffffff;
    if (this.createdByPtr !== null) {
      h = (h * 31 + hashString(this.createdByPtr.id)) & 0xffffffff;
    }
    if (this.parentPtr !== null) {
      h = (h * 31 + hashString(this.parentPtr.id)) & 0xffffffff;
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
    return new NodeReference({
      nodeType: NodeType.INVITE_SENT_EVENT,
      id: this.id,
      spaceId: this.spacePtr?.id ?? null,
      _session: this._session,
      _supergraph: this._supergraph,
    });
  }

  get _pathKey(): string {
    return "InviteSentEvent[id={this.id}]";
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
    return `<InviteSentEvent '${this.path}'>`;
  }

  toValue(): { [key: string]: any } {
    return InviteSentEvent.__packValue__(this);
  }

  static __packValue__(object: InviteSentEvent): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 522;
    objectValue["2"] = String(object.id);
    if (object.parentPtr != null) {
      objectValue["3"] = object.parentPtr.toValue();
    }
    if (object.spacePtr != null) {
      objectValue["5"] = object.spacePtr.toValue();
    }
    objectValue["15"] = object.createdAt.toString({ timeZoneName: "never" });
    if (object.createdByPtr != null) {
      objectValue["16"] = object.createdByPtr.toValue();
    }
    objectValue["35"] = object.nodePtr.toValue();
    objectValue["40"] = object.joinablePtr.toValue();
    objectValue["41"] = object.memberPtr.toValue();
    objectValue["50"] = object.rolePtr.toValue();
    objectValue["51"] = object.roleType;
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): InviteSentEvent {
    const createdByPtrValue = objectValue["16"];
    const unpackedCreatedByPtr =
      createdByPtrValue != undefined
        ? NodeReference.fromValue(createdByPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const parentPtrValue = objectValue["3"];
    const unpackedParentPtr =
      parentPtrValue != undefined
        ? NodeReference.fromValue(parentPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const spacePtrValue = objectValue["5"];
    const unpackedSpacePtr =
      spacePtrValue != undefined
        ? NodeReference.fromValue(spacePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    return new InviteSentEvent({
      role: NodeReference.fromValue(objectValue["50"], _session, _supergraph, _graph, _connection),
      roleType: Number(objectValue["51"]),
      node: NodeReference.fromValue(objectValue["35"], _session, _supergraph, _graph, _connection),
      joinable: NodeReference.fromValue(
        objectValue["40"],
        _session,
        _supergraph,
        _graph,
        _connection,
      ),
      member: NodeReference.fromValue(
        objectValue["41"],
        _session,
        _supergraph,
        _graph,
        _connection,
      ),
      createdAt: Temporal.Instant.from(objectValue["15"]).toZonedDateTimeISO("UTC"),
      createdBy: unpackedCreatedByPtr,
      parent: unpackedParentPtr,
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
  ): InviteSentEvent {
    return InviteSentEvent.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
  }

  toProto(): InviteSentEventProto {
    return InviteSentEvent.__packProto__(this);
  }

  static __packProto__(object: InviteSentEvent): InviteSentEventProto {
    const objectProto: Partial<InviteSentEventProto> = { metatype: 522 };
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
    return objectProto as InviteSentEventProto;
  }

  static __unpackProto__(
    objectProto: InviteSentEventProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): InviteSentEvent {
    return new InviteSentEvent({
      role: NodeReference.fromProto(
        objectProto.rolePtr!,
        _session,
        _supergraph,
        _graph,
        _connection,
      ),
      roleType: Number(objectProto.roleType) as RoleType,
      node: NodeReference.fromProto(
        objectProto.nodePtr!,
        _session,
        _supergraph,
        _graph,
        _connection,
      ),
      joinable: NodeReference.fromProto(
        objectProto.joinablePtr!,
        _session,
        _supergraph,
        _graph,
        _connection,
      ),
      member: NodeReference.fromProto(
        objectProto.memberPtr!,
        _session,
        _supergraph,
        _graph,
        _connection,
      ),
      createdAt: unpackProtoTimestamp(objectProto.createdAt!),
      createdBy:
        objectProto.createdByPtr != undefined
          ? NodeReference.fromProto(
              objectProto.createdByPtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      parent:
        objectProto.parentPtr != undefined
          ? NodeReference.fromProto(
              objectProto.parentPtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      space:
        objectProto.spacePtr != undefined
          ? NodeReference.fromProto(
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
    objectProto: InviteSentEventProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): InviteSentEvent {
    return InviteSentEvent.__unpackProto__(objectProto, _session, _supergraph, _graph, _connection);
  }

  static fromProtoString(packedProtoString: string): InviteSentEvent {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = InviteSentEventProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerNodeClass(NodeType.INVITE_SENT_EVENT, InviteSentEvent);
/* ==== DESTACK_GENERATED_END:NODE:522 ==== */

/* ==== DESTACK_GENERATED_START:NODE:523 ==== */
/**
 * An Invite was rescinded.
 */
export class InviteRescindedEvent extends InviteEvent {
  static metatype: NodeType = NodeType.INVITE_RESCINDED_EVENT;

  /**
   * IsSpatial.parent
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
   * InviteEvent.node
   */
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

  /**
   * InviteEvent.joinable
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
   * InviteEvent.member
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
    node: Invite | NodeReference;
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
      throw new Error(`InviteRescindedEvent.node is required`);
    }
    this.nodePtr = _node;
    let _joinable = options.joinable;
    if (_joinable != null && _joinable instanceof Node) {
      _joinable = _joinable.toRef();
    }
    if (_joinable === null) {
      throw new Error(`InviteRescindedEvent.joinable is required`);
    }
    this.joinablePtr = _joinable;
    let _member = options.member;
    if (_member != null && _member instanceof Node) {
      _member = _member.toRef();
    }
    if (_member === null) {
      throw new Error(`InviteRescindedEvent.member is required`);
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
          ? options.createdBy instanceof Node
            ? options.createdBy.toRef()
            : options.createdBy
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
    h = (h * 31 + hashString(this.createdAt.toString({ timeZoneName: "never" }))) & 0xffffffff;
    if (this.createdByPtr !== null) {
      h = (h * 31 + hashString(this.createdByPtr.id)) & 0xffffffff;
    }
    if (this.parentPtr !== null) {
      h = (h * 31 + hashString(this.parentPtr.id)) & 0xffffffff;
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
    return new NodeReference({
      nodeType: NodeType.INVITE_RESCINDED_EVENT,
      id: this.id,
      spaceId: this.spacePtr?.id ?? null,
      _session: this._session,
      _supergraph: this._supergraph,
    });
  }

  get _pathKey(): string {
    return "InviteRescindedEvent[id={this.id}]";
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
    return `<InviteRescindedEvent '${this.path}'>`;
  }

  toValue(): { [key: string]: any } {
    return InviteRescindedEvent.__packValue__(this);
  }

  static __packValue__(object: InviteRescindedEvent): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 523;
    objectValue["2"] = String(object.id);
    if (object.parentPtr != null) {
      objectValue["3"] = object.parentPtr.toValue();
    }
    if (object.spacePtr != null) {
      objectValue["5"] = object.spacePtr.toValue();
    }
    objectValue["15"] = object.createdAt.toString({ timeZoneName: "never" });
    if (object.createdByPtr != null) {
      objectValue["16"] = object.createdByPtr.toValue();
    }
    objectValue["35"] = object.nodePtr.toValue();
    objectValue["40"] = object.joinablePtr.toValue();
    objectValue["41"] = object.memberPtr.toValue();
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): InviteRescindedEvent {
    const createdByPtrValue = objectValue["16"];
    const unpackedCreatedByPtr =
      createdByPtrValue != undefined
        ? NodeReference.fromValue(createdByPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const parentPtrValue = objectValue["3"];
    const unpackedParentPtr =
      parentPtrValue != undefined
        ? NodeReference.fromValue(parentPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const spacePtrValue = objectValue["5"];
    const unpackedSpacePtr =
      spacePtrValue != undefined
        ? NodeReference.fromValue(spacePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    return new InviteRescindedEvent({
      node: NodeReference.fromValue(objectValue["35"], _session, _supergraph, _graph, _connection),
      joinable: NodeReference.fromValue(
        objectValue["40"],
        _session,
        _supergraph,
        _graph,
        _connection,
      ),
      member: NodeReference.fromValue(
        objectValue["41"],
        _session,
        _supergraph,
        _graph,
        _connection,
      ),
      createdAt: Temporal.Instant.from(objectValue["15"]).toZonedDateTimeISO("UTC"),
      createdBy: unpackedCreatedByPtr,
      parent: unpackedParentPtr,
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
  ): InviteRescindedEvent {
    return InviteRescindedEvent.__unpackValue__(
      objectValue,
      _session,
      _supergraph,
      _graph,
      _connection,
    );
  }

  toProto(): InviteRescindedEventProto {
    return InviteRescindedEvent.__packProto__(this);
  }

  static __packProto__(object: InviteRescindedEvent): InviteRescindedEventProto {
    const objectProto: Partial<InviteRescindedEventProto> = { metatype: 523 };
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
    return objectProto as InviteRescindedEventProto;
  }

  static __unpackProto__(
    objectProto: InviteRescindedEventProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): InviteRescindedEvent {
    return new InviteRescindedEvent({
      node: NodeReference.fromProto(
        objectProto.nodePtr!,
        _session,
        _supergraph,
        _graph,
        _connection,
      ),
      joinable: NodeReference.fromProto(
        objectProto.joinablePtr!,
        _session,
        _supergraph,
        _graph,
        _connection,
      ),
      member: NodeReference.fromProto(
        objectProto.memberPtr!,
        _session,
        _supergraph,
        _graph,
        _connection,
      ),
      createdAt: unpackProtoTimestamp(objectProto.createdAt!),
      createdBy:
        objectProto.createdByPtr != undefined
          ? NodeReference.fromProto(
              objectProto.createdByPtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      parent:
        objectProto.parentPtr != undefined
          ? NodeReference.fromProto(
              objectProto.parentPtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      space:
        objectProto.spacePtr != undefined
          ? NodeReference.fromProto(
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
    objectProto: InviteRescindedEventProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): InviteRescindedEvent {
    return InviteRescindedEvent.__unpackProto__(
      objectProto,
      _session,
      _supergraph,
      _graph,
      _connection,
    );
  }

  static fromProtoString(packedProtoString: string): InviteRescindedEvent {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = InviteRescindedEventProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerNodeClass(NodeType.INVITE_RESCINDED_EVENT, InviteRescindedEvent);
/* ==== DESTACK_GENERATED_END:NODE:523 ==== */

/* ==== DESTACK_GENERATED_START:NODE:524 ==== */
/**
 * An Invite was accepted.
 */
export class InviteAcceptedEvent extends InviteEvent {
  static metatype: NodeType = NodeType.INVITE_ACCEPTED_EVENT;

  /**
   * IsSpatial.parent
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
   * InviteEvent.node
   */
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

  /**
   * InviteEvent.joinable
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
   * InviteEvent.member
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
   * InviteAcceptedEvent.role
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
   * InviteAcceptedEvent.roleType
   */
  roleType: RoleType;

  constructor(options: {
    id?: string;
    parent?: Space | NodeReference | null;
    space?: Space | NodeReference | null;
    createdAt?: Temporal.ZonedDateTime;
    createdBy?: (Node & IsSubject) | NodeReference | null;
    node: Invite | NodeReference;
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
      throw new Error(`InviteAcceptedEvent.node is required`);
    }
    this.nodePtr = _node;
    let _joinable = options.joinable;
    if (_joinable != null && _joinable instanceof Node) {
      _joinable = _joinable.toRef();
    }
    if (_joinable === null) {
      throw new Error(`InviteAcceptedEvent.joinable is required`);
    }
    this.joinablePtr = _joinable;
    let _member = options.member;
    if (_member != null && _member instanceof Node) {
      _member = _member.toRef();
    }
    if (_member === null) {
      throw new Error(`InviteAcceptedEvent.member is required`);
    }
    this.memberPtr = _member;
    let _role = options.role;
    if (_role != null && _role instanceof Node) {
      _role = _role.toRef();
    }
    if (_role === null) {
      throw new Error(`InviteAcceptedEvent.role is required`);
    }
    this.rolePtr = _role;
    let _roleType = options.roleType;
    if (_roleType === null) {
      throw new Error(`InviteAcceptedEvent.roleType is required`);
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
          ? options.createdBy instanceof Node
            ? options.createdBy.toRef()
            : options.createdBy
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
    h = (h * 31 + hashString(this.createdAt.toString({ timeZoneName: "never" }))) & 0xffffffff;
    if (this.createdByPtr !== null) {
      h = (h * 31 + hashString(this.createdByPtr.id)) & 0xffffffff;
    }
    if (this.parentPtr !== null) {
      h = (h * 31 + hashString(this.parentPtr.id)) & 0xffffffff;
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
    return new NodeReference({
      nodeType: NodeType.INVITE_ACCEPTED_EVENT,
      id: this.id,
      spaceId: this.spacePtr?.id ?? null,
      _session: this._session,
      _supergraph: this._supergraph,
    });
  }

  get _pathKey(): string {
    return "InviteAcceptedEvent[id={this.id}]";
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
    return `<InviteAcceptedEvent '${this.path}'>`;
  }

  toValue(): { [key: string]: any } {
    return InviteAcceptedEvent.__packValue__(this);
  }

  static __packValue__(object: InviteAcceptedEvent): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 524;
    objectValue["2"] = String(object.id);
    if (object.parentPtr != null) {
      objectValue["3"] = object.parentPtr.toValue();
    }
    if (object.spacePtr != null) {
      objectValue["5"] = object.spacePtr.toValue();
    }
    objectValue["15"] = object.createdAt.toString({ timeZoneName: "never" });
    if (object.createdByPtr != null) {
      objectValue["16"] = object.createdByPtr.toValue();
    }
    objectValue["35"] = object.nodePtr.toValue();
    objectValue["40"] = object.joinablePtr.toValue();
    objectValue["41"] = object.memberPtr.toValue();
    objectValue["50"] = object.rolePtr.toValue();
    objectValue["51"] = object.roleType;
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): InviteAcceptedEvent {
    const createdByPtrValue = objectValue["16"];
    const unpackedCreatedByPtr =
      createdByPtrValue != undefined
        ? NodeReference.fromValue(createdByPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const parentPtrValue = objectValue["3"];
    const unpackedParentPtr =
      parentPtrValue != undefined
        ? NodeReference.fromValue(parentPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const spacePtrValue = objectValue["5"];
    const unpackedSpacePtr =
      spacePtrValue != undefined
        ? NodeReference.fromValue(spacePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    return new InviteAcceptedEvent({
      role: NodeReference.fromValue(objectValue["50"], _session, _supergraph, _graph, _connection),
      roleType: Number(objectValue["51"]),
      node: NodeReference.fromValue(objectValue["35"], _session, _supergraph, _graph, _connection),
      joinable: NodeReference.fromValue(
        objectValue["40"],
        _session,
        _supergraph,
        _graph,
        _connection,
      ),
      member: NodeReference.fromValue(
        objectValue["41"],
        _session,
        _supergraph,
        _graph,
        _connection,
      ),
      createdAt: Temporal.Instant.from(objectValue["15"]).toZonedDateTimeISO("UTC"),
      createdBy: unpackedCreatedByPtr,
      parent: unpackedParentPtr,
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
  ): InviteAcceptedEvent {
    return InviteAcceptedEvent.__unpackValue__(
      objectValue,
      _session,
      _supergraph,
      _graph,
      _connection,
    );
  }

  toProto(): InviteAcceptedEventProto {
    return InviteAcceptedEvent.__packProto__(this);
  }

  static __packProto__(object: InviteAcceptedEvent): InviteAcceptedEventProto {
    const objectProto: Partial<InviteAcceptedEventProto> = { metatype: 524 };
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
    return objectProto as InviteAcceptedEventProto;
  }

  static __unpackProto__(
    objectProto: InviteAcceptedEventProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): InviteAcceptedEvent {
    return new InviteAcceptedEvent({
      role: NodeReference.fromProto(
        objectProto.rolePtr!,
        _session,
        _supergraph,
        _graph,
        _connection,
      ),
      roleType: Number(objectProto.roleType) as RoleType,
      node: NodeReference.fromProto(
        objectProto.nodePtr!,
        _session,
        _supergraph,
        _graph,
        _connection,
      ),
      joinable: NodeReference.fromProto(
        objectProto.joinablePtr!,
        _session,
        _supergraph,
        _graph,
        _connection,
      ),
      member: NodeReference.fromProto(
        objectProto.memberPtr!,
        _session,
        _supergraph,
        _graph,
        _connection,
      ),
      createdAt: unpackProtoTimestamp(objectProto.createdAt!),
      createdBy:
        objectProto.createdByPtr != undefined
          ? NodeReference.fromProto(
              objectProto.createdByPtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      parent:
        objectProto.parentPtr != undefined
          ? NodeReference.fromProto(
              objectProto.parentPtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      space:
        objectProto.spacePtr != undefined
          ? NodeReference.fromProto(
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
    objectProto: InviteAcceptedEventProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): InviteAcceptedEvent {
    return InviteAcceptedEvent.__unpackProto__(
      objectProto,
      _session,
      _supergraph,
      _graph,
      _connection,
    );
  }

  static fromProtoString(packedProtoString: string): InviteAcceptedEvent {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = InviteAcceptedEventProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerNodeClass(NodeType.INVITE_ACCEPTED_EVENT, InviteAcceptedEvent);
/* ==== DESTACK_GENERATED_END:NODE:524 ==== */

/* ==== DESTACK_GENERATED_START:NODE:520 ==== */
/**
 * An Invite to a Joinable.
 */
export class Invite extends Entity implements IsGlobal, IsSpatial, IsOwnable, IsDeletable {
  static metatype: NodeType = NodeType.INVITE;

  /**
   * Invite.parent
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
   * Invite.member
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
   * Invite.role
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
   * Invite.roleType
   */
  roleType: RoleType | null;

  constructor(options: {
    id?: string;
    parent?: (Node & IsJoinable) | NodeReference | null;
    space?: Space | NodeReference | null;
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
    h = (h * 31 + hashString(this.id.toString())) & 0xffffffff;
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

    return h;
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

  repr(): string {
    const propertyReprs: string[] = [];
    if (this.ownedBy !== null) {
      propertyReprs.push(`ownedBy=${this.ownedBy?.repr()}`);
    }
    if (propertyReprs.length > 0) {
      return `<Invite '${this.path}' ${propertyReprs.join(" ")}>`;
    } else {
      return `<Invite '${this.path}'>`;
    }
  }

  toValue(): { [key: string]: any } {
    return Invite.__packValue__(this);
  }

  static __packValue__(object: Invite): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 520;
    objectValue["2"] = String(object.id);
    if (object.parentPtr != null) {
      objectValue["3"] = object.parentPtr.toValue();
    }
    if (object.spacePtr != null) {
      objectValue["5"] = object.spacePtr.toValue();
    }
    objectValue["15"] = object.createdAt.toString({ timeZoneName: "never" });
    if (object.createdByPtr != null) {
      objectValue["16"] = object.createdByPtr.toValue();
    }
    objectValue["17"] = object.updatedAt.toString({ timeZoneName: "never" });
    if (object.updatedByPtr != null) {
      objectValue["18"] = object.updatedByPtr.toValue();
    }
    if (object.deletedAt != null) {
      objectValue["20"] = object.deletedAt.toString({ timeZoneName: "never" });
    }
    if (object.ownedByPtr != null) {
      objectValue["25"] = object.ownedByPtr.toValue();
    }
    objectValue["40"] = object.memberPtr.toValue();
    if (object.rolePtr != null) {
      objectValue["41"] = object.rolePtr.toValue();
    }
    if (object.roleType != null) {
      objectValue["42"] = object.roleType;
    }
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Invite {
    const parentPtrValue = objectValue["3"];
    const unpackedParentPtr =
      parentPtrValue != undefined
        ? NodeReference.fromValue(parentPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const rolePtrValue = objectValue["41"];
    const unpackedRolePtr =
      rolePtrValue != undefined
        ? NodeReference.fromValue(rolePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const roleTypeValue = objectValue["42"];
    const unpackedRoleType = roleTypeValue != undefined ? Number(roleTypeValue) : null;
    const spacePtrValue = objectValue["5"];
    const unpackedSpacePtr =
      spacePtrValue != undefined
        ? NodeReference.fromValue(spacePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const ownedByPtrValue = objectValue["25"];
    const unpackedOwnedByPtr =
      ownedByPtrValue != undefined
        ? NodeReference.fromValue(ownedByPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const deletedAtValue = objectValue["20"];
    const unpackedDeletedAt =
      deletedAtValue != undefined
        ? Temporal.Instant.from(deletedAtValue).toZonedDateTimeISO("UTC")
        : null;
    const createdByPtrValue = objectValue["16"];
    const unpackedCreatedByPtr =
      createdByPtrValue != undefined
        ? NodeReference.fromValue(createdByPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const updatedByPtrValue = objectValue["18"];
    const unpackedUpdatedByPtr =
      updatedByPtrValue != undefined
        ? NodeReference.fromValue(updatedByPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    return new Invite({
      parent: unpackedParentPtr,
      member: NodeReference.fromValue(
        objectValue["40"],
        _session,
        _supergraph,
        _graph,
        _connection,
      ),
      role: unpackedRolePtr,
      roleType: unpackedRoleType,
      id: String(objectValue["2"]),
      space: unpackedSpacePtr,
      ownedBy: unpackedOwnedByPtr,
      deletedAt: unpackedDeletedAt,
      createdAt: Temporal.Instant.from(objectValue["15"]).toZonedDateTimeISO("UTC"),
      createdBy: unpackedCreatedByPtr,
      updatedAt: Temporal.Instant.from(objectValue["17"]).toZonedDateTimeISO("UTC"),
      updatedBy: unpackedUpdatedByPtr,
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
  ): Invite {
    return Invite.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
  }

  toProto(): InviteProto {
    return Invite.__packProto__(this);
  }

  static __packProto__(object: Invite): InviteProto {
    const objectProto: Partial<InviteProto> = { metatype: 520 };
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
    return objectProto as InviteProto;
  }

  static __unpackProto__(
    objectProto: InviteProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Invite {
    return new Invite({
      parent:
        objectProto.parentPtr != undefined
          ? NodeReference.fromProto(
              objectProto.parentPtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      member: NodeReference.fromProto(
        objectProto.memberPtr!,
        _session,
        _supergraph,
        _graph,
        _connection,
      ),
      role:
        objectProto.rolePtr != undefined
          ? NodeReference.fromProto(
              objectProto.rolePtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      roleType:
        objectProto.roleType != undefined ? (Number(objectProto.roleType) as RoleType) : null,
      id: String(objectProto.id),
      space:
        objectProto.spacePtr != undefined
          ? NodeReference.fromProto(
              objectProto.spacePtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      ownedBy:
        objectProto.ownedByPtr != undefined
          ? NodeReference.fromProto(
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
          ? NodeReference.fromProto(
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
          ? NodeReference.fromProto(
              objectProto.updatedByPtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      _session,
      _graph,
      _connection,
    });
  }

  static fromProto(
    objectProto: InviteProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Invite {
    return Invite.__unpackProto__(objectProto, _session, _supergraph, _graph, _connection);
  }

  static fromProtoString(packedProtoString: string): Invite {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = InviteProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerNodeClass(NodeType.INVITE, Invite);
/* ==== DESTACK_GENERATED_END:NODE:520 ==== */

/* ==== DESTACK_GENERATED_START:NODE:525 ==== */
/**
 * An Invite was rejected.
 */
export class InviteRejectedEvent extends InviteEvent {
  static metatype: NodeType = NodeType.INVITE_REJECTED_EVENT;

  /**
   * IsSpatial.parent
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
   * InviteEvent.node
   */
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

  /**
   * InviteEvent.joinable
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
   * InviteEvent.member
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
    node: Invite | NodeReference;
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
      throw new Error(`InviteRejectedEvent.node is required`);
    }
    this.nodePtr = _node;
    let _joinable = options.joinable;
    if (_joinable != null && _joinable instanceof Node) {
      _joinable = _joinable.toRef();
    }
    if (_joinable === null) {
      throw new Error(`InviteRejectedEvent.joinable is required`);
    }
    this.joinablePtr = _joinable;
    let _member = options.member;
    if (_member != null && _member instanceof Node) {
      _member = _member.toRef();
    }
    if (_member === null) {
      throw new Error(`InviteRejectedEvent.member is required`);
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
          ? options.createdBy instanceof Node
            ? options.createdBy.toRef()
            : options.createdBy
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
    h = (h * 31 + hashString(this.createdAt.toString({ timeZoneName: "never" }))) & 0xffffffff;
    if (this.createdByPtr !== null) {
      h = (h * 31 + hashString(this.createdByPtr.id)) & 0xffffffff;
    }
    if (this.parentPtr !== null) {
      h = (h * 31 + hashString(this.parentPtr.id)) & 0xffffffff;
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
    return new NodeReference({
      nodeType: NodeType.INVITE_REJECTED_EVENT,
      id: this.id,
      spaceId: this.spacePtr?.id ?? null,
      _session: this._session,
      _supergraph: this._supergraph,
    });
  }

  get _pathKey(): string {
    return "InviteRejectedEvent[id={this.id}]";
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
    return `<InviteRejectedEvent '${this.path}'>`;
  }

  toValue(): { [key: string]: any } {
    return InviteRejectedEvent.__packValue__(this);
  }

  static __packValue__(object: InviteRejectedEvent): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 525;
    objectValue["2"] = String(object.id);
    if (object.parentPtr != null) {
      objectValue["3"] = object.parentPtr.toValue();
    }
    if (object.spacePtr != null) {
      objectValue["5"] = object.spacePtr.toValue();
    }
    objectValue["15"] = object.createdAt.toString({ timeZoneName: "never" });
    if (object.createdByPtr != null) {
      objectValue["16"] = object.createdByPtr.toValue();
    }
    objectValue["35"] = object.nodePtr.toValue();
    objectValue["40"] = object.joinablePtr.toValue();
    objectValue["41"] = object.memberPtr.toValue();
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): InviteRejectedEvent {
    const createdByPtrValue = objectValue["16"];
    const unpackedCreatedByPtr =
      createdByPtrValue != undefined
        ? NodeReference.fromValue(createdByPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const parentPtrValue = objectValue["3"];
    const unpackedParentPtr =
      parentPtrValue != undefined
        ? NodeReference.fromValue(parentPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const spacePtrValue = objectValue["5"];
    const unpackedSpacePtr =
      spacePtrValue != undefined
        ? NodeReference.fromValue(spacePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    return new InviteRejectedEvent({
      node: NodeReference.fromValue(objectValue["35"], _session, _supergraph, _graph, _connection),
      joinable: NodeReference.fromValue(
        objectValue["40"],
        _session,
        _supergraph,
        _graph,
        _connection,
      ),
      member: NodeReference.fromValue(
        objectValue["41"],
        _session,
        _supergraph,
        _graph,
        _connection,
      ),
      createdAt: Temporal.Instant.from(objectValue["15"]).toZonedDateTimeISO("UTC"),
      createdBy: unpackedCreatedByPtr,
      parent: unpackedParentPtr,
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
  ): InviteRejectedEvent {
    return InviteRejectedEvent.__unpackValue__(
      objectValue,
      _session,
      _supergraph,
      _graph,
      _connection,
    );
  }

  toProto(): InviteRejectedEventProto {
    return InviteRejectedEvent.__packProto__(this);
  }

  static __packProto__(object: InviteRejectedEvent): InviteRejectedEventProto {
    const objectProto: Partial<InviteRejectedEventProto> = { metatype: 525 };
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
    return objectProto as InviteRejectedEventProto;
  }

  static __unpackProto__(
    objectProto: InviteRejectedEventProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): InviteRejectedEvent {
    return new InviteRejectedEvent({
      node: NodeReference.fromProto(
        objectProto.nodePtr!,
        _session,
        _supergraph,
        _graph,
        _connection,
      ),
      joinable: NodeReference.fromProto(
        objectProto.joinablePtr!,
        _session,
        _supergraph,
        _graph,
        _connection,
      ),
      member: NodeReference.fromProto(
        objectProto.memberPtr!,
        _session,
        _supergraph,
        _graph,
        _connection,
      ),
      createdAt: unpackProtoTimestamp(objectProto.createdAt!),
      createdBy:
        objectProto.createdByPtr != undefined
          ? NodeReference.fromProto(
              objectProto.createdByPtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      parent:
        objectProto.parentPtr != undefined
          ? NodeReference.fromProto(
              objectProto.parentPtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      space:
        objectProto.spacePtr != undefined
          ? NodeReference.fromProto(
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
    objectProto: InviteRejectedEventProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): InviteRejectedEvent {
    return InviteRejectedEvent.__unpackProto__(
      objectProto,
      _session,
      _supergraph,
      _graph,
      _connection,
    );
  }

  static fromProtoString(packedProtoString: string): InviteRejectedEvent {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = InviteRejectedEventProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerNodeClass(NodeType.INVITE_REJECTED_EVENT, InviteRejectedEvent);
/* ==== DESTACK_GENERATED_END:NODE:525 ==== */
