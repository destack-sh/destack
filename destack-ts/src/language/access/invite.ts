import { packProtoTimestamp, unpackProtoTimestamp } from "@destack/grpc";
import type { Role } from "@destack/language/access/role";
import type {
  Graph,
  IsActor,
  IsDeletable,
  IsExtensible,
  IsJoinable,
  IsOwnable,
  NodeClass,
  NodeReference,
  QueryConnection,
  Session,
  Snapshot,
  Supergraph,
  Value,
} from "@destack/language/core";
import {
  ACTIVE_SPACE,
  Entity,
  Event,
  EventStatus,
  Materialization,
  Node,
  NodeType,
  RoleType,
  StructType,
} from "@destack/language/core";
import type { Script } from "@destack/language/logic";
import { STRUCT_CLASS_BY_TYPE, registerNodeClass } from "@destack/language/registry";
import type { Client, Space } from "@destack/language/universe";
import {
  EventStatusProto,
  InviteAcceptedEventProto,
  InviteProto,
  InviteRejectedEventProto,
  InviteRescindedEventProto,
  InviteSentEventProto,
  MaterializationProto,
  RoleTypeProto,
} from "@destack/proto";
import { base64Decode } from "@destack/utils";
import { hashBool, hashString } from "@destack/utils/hash";
import { Temporal } from "temporal-polyfill";

/* ==== DESTACK_GENERATED_START:NODE:60101 ==== */
/**
 * A Event regarding an Invite.
 */
export abstract class InviteEvent extends Event {
  static metatype: NodeType = NodeType.INVITE_EVENT;

  /**
   * The Space this Node is in.
   */
  abstract get space(): Space | null;
  declare readonly spacePtr: NodeReference;

  /**
   * The Snapshot this Event originated from.
   */
  abstract get snapshot(): Snapshot | null;
  declare readonly snapshotPtr: NodeReference | null;

  /**
   * Event.createdAt
   */
  declare readonly createdAt: Temporal.ZonedDateTime;

  /**
   * Event.createdBy
   */
  abstract get createdBy(): (Entity & IsActor) | null;
  declare readonly createdByPtr: NodeReference | null;

  /**
   * Event.client
   */
  abstract get client(): Client | null;
  declare readonly clientPtr: NodeReference | null;

  /**
   * Event.clientNonce
   */
  declare readonly clientNonce: string | null;

  /**
   * The status of the Event.
   */
  declare readonly status: EventStatus;

  /**
   * InviteEvent.node
   */
  abstract get node(): Invite | null;
  declare readonly nodePtr: NodeReference;

  /**
   * InviteEvent.joinable
   */
  abstract get joinable(): (Entity & IsJoinable) | null;
  declare readonly joinablePtr: NodeReference;

  /**
   * InviteEvent.member
   */
  abstract get member(): (Entity & IsActor) | null;
  declare readonly memberPtr: NodeReference;

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerNodeClass(NodeType.INVITE_EVENT, InviteEvent);
/* ==== DESTACK_GENERATED_END:NODE:60101 ==== */

/* ==== DESTACK_GENERATED_START:NODE:60102 ==== */
/**
 * An Invite was sent.
 */
export class InviteSentEvent extends InviteEvent {
  static metatype: NodeType = NodeType.INVITE_SENT_EVENT;

  /**
   * The Space this Node is in.
   */
  get space(): Space | null {
    const nodePtr: NodeReference | null = this.spacePtr;
    if (nodePtr != null) {
      return this._supergraph.get(nodePtr.id) as Space | null;
    }
    return null;
  }
  readonly spacePtr: NodeReference;

  /**
   * The Snapshot this Event originated from.
   */
  get snapshot(): Snapshot | null {
    const nodePtr: NodeReference | null = this.snapshotPtr;
    if (nodePtr != null) {
      return this._supergraph.get(nodePtr.id) as Snapshot | null;
    }
    return null;
  }
  readonly snapshotPtr: NodeReference | null;

  /**
   * Event.createdAt
   */
  readonly createdAt: Temporal.ZonedDateTime;

  /**
   * Event.createdBy
   */
  get createdBy(): (Entity & IsActor) | null {
    const nodePtr: NodeReference | null = this.createdByPtr;
    if (nodePtr != null) {
      return this._supergraph.get(nodePtr.id) as (Entity & IsActor) | null;
    }
    return null;
  }
  readonly createdByPtr: NodeReference | null;

  /**
   * Event.client
   */
  get client(): Client | null {
    const nodePtr: NodeReference | null = this.clientPtr;
    if (nodePtr != null) {
      return this._supergraph.get(nodePtr.id) as Client | null;
    }
    return null;
  }
  readonly clientPtr: NodeReference | null;

  /**
   * Event.clientNonce
   */
  readonly clientNonce: string | null;

  /**
   * The status of the Event.
   */
  readonly status: EventStatus;

  /**
   * InviteEvent.node
   */
  get node(): Invite | null {
    const nodePtr: NodeReference | null = this.nodePtr;
    if (nodePtr != null) {
      return this._supergraph.get(nodePtr.id) as Invite | null;
    }
    return null;
  }
  readonly nodePtr: NodeReference;

  /**
   * InviteEvent.joinable
   */
  get joinable(): (Entity & IsJoinable) | null {
    const nodePtr: NodeReference | null = this.joinablePtr;
    if (nodePtr != null) {
      return this._supergraph.get(nodePtr.id) as (Entity & IsJoinable) | null;
    }
    return null;
  }
  readonly joinablePtr: NodeReference;

  /**
   * InviteEvent.member
   */
  get member(): (Entity & IsActor) | null {
    const nodePtr: NodeReference | null = this.memberPtr;
    if (nodePtr != null) {
      return this._supergraph.get(nodePtr.id) as (Entity & IsActor) | null;
    }
    return null;
  }
  readonly memberPtr: NodeReference;

  /**
   * InviteSentEvent.role
   */
  get role(): Role | null {
    const nodePtr: NodeReference | null = this.rolePtr;
    if (nodePtr != null) {
      return this._supergraph.get(nodePtr.id) as Role | null;
    }
    return null;
  }
  readonly rolePtr: NodeReference;

  /**
   * InviteSentEvent.roleType
   */
  readonly roleType: RoleType;

  constructor(options: {
    id?: string;
    space?: Space | NodeReference;
    snapshot?: Snapshot | NodeReference | null;
    createdAt?: Temporal.ZonedDateTime;
    createdBy?: (Entity & IsActor) | NodeReference | null;
    client?: Client | NodeReference | null;
    clientNonce?: string | null;
    status?: EventStatus;
    node: Invite | NodeReference;
    joinable: (Entity & IsJoinable) | NodeReference;
    member: (Entity & IsActor) | NodeReference;
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
      null,
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
    );

    // properties
    let _space = options.space ?? null;
    if (_space != null && _space.metatype != StructType.NODE_REFERENCE) {
      _space = (_space as Node).toRef();
    }
    if (_space === null) {
      if (this._session === null) {
        throw new Error(`InviteSentEvent has no Session`);
      }
      _space = ACTIVE_SPACE.get();
      if (_space === null) {
        throw new Error(`InviteSentEvent has no Space`);
      }
      _space = _space.toRef();
    }
    if (_space === null) {
      throw new Error(`InviteSentEvent.space is required`);
    }
    this.spacePtr = _space;
    let _snapshot = options.snapshot ?? null;
    if (_snapshot != null && _snapshot.metatype != StructType.NODE_REFERENCE) {
      _snapshot = (_snapshot as Node).toRef();
    }
    this.snapshotPtr = _snapshot;
    let _client = options.client ?? null;
    if (_client != null && _client.metatype != StructType.NODE_REFERENCE) {
      _client = (_client as Node).toRef();
    }
    this.clientPtr = _client;
    let _clientNonce = options.clientNonce ?? null;
    this.clientNonce = _clientNonce;
    let _status = options.status ?? null;
    if (_status === null) {
      _status = 1 /* EventStatus.PENDING */;
    }
    if (_status === null) {
      throw new Error(`InviteSentEvent.status is required`);
    }
    this.status = _status;
    let _node = options.node;
    if (_node != null && _node.metatype != StructType.NODE_REFERENCE) {
      _node = (_node as Node).toRef();
    }
    if (_node === null) {
      throw new Error(`InviteSentEvent.node is required`);
    }
    this.nodePtr = _node;
    let _joinable = options.joinable;
    if (_joinable != null && _joinable.metatype != StructType.NODE_REFERENCE) {
      _joinable = (_joinable as Node).toRef();
    }
    if (_joinable === null) {
      throw new Error(`InviteSentEvent.joinable is required`);
    }
    this.joinablePtr = _joinable;
    let _member = options.member;
    if (_member != null && _member.metatype != StructType.NODE_REFERENCE) {
      _member = (_member as Node).toRef();
    }
    if (_member === null) {
      throw new Error(`InviteSentEvent.member is required`);
    }
    this.memberPtr = _member;
    let _role = options.role;
    if (_role != null && _role.metatype != StructType.NODE_REFERENCE) {
      _role = (_role as Node).toRef();
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
        throw new Error(`InviteSentEvent.createdAt is required for existing Events`);
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
    if (!(this.snapshotPtr?.id === other.snapshotPtr?.id)) {
      return false;
    }
    if (!(this.clientPtr?.id === other.clientPtr?.id)) {
      return false;
    }
    if (!(this.clientNonce === other.clientNonce)) {
      return false;
    }
    if (!(this.status === other.status)) {
      return false;
    }
    if (!(this.spacePtr.id === other.spacePtr.id)) {
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
    if (this.snapshotPtr != null) {
      h = (h * 31 + hashString(this.snapshotPtr.id)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.createdAt.toString({ timeZoneName: "never" }))) & 0xffffffff;
    if (this.createdByPtr != null) {
      h = (h * 31 + hashString(this.createdByPtr.id)) & 0xffffffff;
    }
    if (this.clientPtr != null) {
      h = (h * 31 + hashString(this.clientPtr.id)) & 0xffffffff;
    }
    if (this.clientNonce != null) {
      h = (h * 31 + hashString(this.clientNonce.toString())) & 0xffffffff;
    }
    h = (h * 31 + this.status) & 0xffffffff;
    h = (h * 31 + hashString(this.id.toString())) & 0xffffffff;
    h = (h * 31 + hashString(this.spacePtr.id)) & 0xffffffff;

    return h;
  }

  validate(): void {
    throw new Error("not implemented");
  }

  __toRef__(): NodeReference {
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    return new _NodeReference({
      type: NodeType.INVITE_SENT_EVENT,
      id: this.id,
      spaceId: this.spacePtr?.id ?? null,
      snapshotId: this.snapshotPtr?.id ?? null,
      _session: this._session,
      _supergraph: this._supergraph,
    });
  }

  get _pathKey(): string {
    return `InviteSentEvent[id=${this.id}]`;
  }

  get path(): string {
    const pathParts: string[] = [];
    let node: Entity | Event | null = this;
    let lastNode: Entity | Event | null = this;
    while (node != null) {
      pathParts.push(node._pathKey);
      lastNode = node;
      node = node.parent;
    }
    if (!lastNode.isRoot) {
      pathParts.push("<detached>");
    }
    return pathParts.reverse().join("/");
  }

  repr(): string {
    const propertyReprs: string[] = [];
    propertyReprs.push(`status=${EventStatus[this.status]}`);
    return `<InviteSentEvent "${this.path}" ${propertyReprs.join(" ")}>`;
  }

  toValue(): { readonly [key: string]: any } {
    return InviteSentEvent.__packValue__(this);
  }

  static __packValue__(object: InviteSentEvent): { readonly [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 60102;
    objectValue["2"] = String(object.id);
    objectValue["5"] = object.spacePtr.toValue();
    if (object.snapshotPtr != null) {
      objectValue["11"] = object.snapshotPtr.toValue();
    }
    objectValue["20"] = object.createdAt.toString({ timeZoneName: "never" });
    if (object.createdByPtr != null) {
      objectValue["21"] = object.createdByPtr.toValue();
    }
    if (object.clientPtr != null) {
      objectValue["22"] = object.clientPtr.toValue();
    }
    if (object.clientNonce != null) {
      objectValue["23"] = String(object.clientNonce);
    }
    objectValue["30"] = object.status;
    objectValue["101"] = object.nodePtr.toValue();
    objectValue["102"] = object.joinablePtr.toValue();
    objectValue["103"] = object.memberPtr.toValue();
    objectValue["110"] = object.rolePtr.toValue();
    objectValue["111"] = object.roleType;
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { readonly [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): InviteSentEvent {
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    const snapshotPtrValue = objectValue["11"];
    const unpackedSnapshotPtr =
      snapshotPtrValue != undefined
        ? _NodeReference.fromValue(snapshotPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const createdByPtrValue = objectValue["21"];
    const unpackedCreatedByPtr =
      createdByPtrValue != undefined
        ? _NodeReference.fromValue(createdByPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const clientPtrValue = objectValue["22"];
    const unpackedClientPtr =
      clientPtrValue != undefined
        ? _NodeReference.fromValue(clientPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const clientNonceValue = objectValue["23"];
    const unpackedClientNonce = clientNonceValue != undefined ? String(clientNonceValue) : null;
    return new InviteSentEvent({
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
      snapshot: unpackedSnapshotPtr,
      createdAt: Temporal.Instant.from(objectValue["20"]).toZonedDateTimeISO("UTC"),
      createdBy: unpackedCreatedByPtr,
      client: unpackedClientPtr,
      clientNonce: unpackedClientNonce,
      status: Number(objectValue["30"]),
      id: String(objectValue["2"]),
      space: _NodeReference.fromValue(objectValue["5"], _session, _supergraph, _graph, _connection),
      _session,
      _graph,
      _connection,
    });
  }

  static fromValue(
    objectValue: { readonly [key: string]: any },
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
    const objectProto: Partial<InviteSentEventProto> = { metatype: 60102 };
    objectProto.id = String(object.id);
    objectProto.spacePtr = object.spacePtr.toProto();
    if (object.snapshotPtr != null) {
      objectProto.snapshotPtr = object.snapshotPtr.toProto();
    }
    objectProto.createdAt = packProtoTimestamp(object.createdAt);
    if (object.createdByPtr != null) {
      objectProto.createdByPtr = object.createdByPtr.toProto();
    }
    if (object.clientPtr != null) {
      objectProto.clientPtr = object.clientPtr.toProto();
    }
    if (object.clientNonce != null) {
      objectProto.clientNonce = String(object.clientNonce);
    }
    objectProto.status = Number(object.status) as EventStatusProto;
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
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    return new InviteSentEvent({
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
      snapshot:
        objectProto.snapshotPtr != undefined
          ? _NodeReference.fromProto(
              objectProto.snapshotPtr!,
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
      client:
        objectProto.clientPtr != undefined
          ? _NodeReference.fromProto(
              objectProto.clientPtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      clientNonce: objectProto.clientNonce != undefined ? String(objectProto.clientNonce) : null,
      status: Number(objectProto.status) as EventStatus,
      id: String(objectProto.id),
      space: _NodeReference.fromProto(
        objectProto.spacePtr!,
        _session,
        _supergraph,
        _graph,
        _connection,
      ),
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
/* ==== DESTACK_GENERATED_END:NODE:60102 ==== */

/* ==== DESTACK_GENERATED_START:NODE:60103 ==== */
/**
 * An Invite was rescinded.
 */
export class InviteRescindedEvent extends InviteEvent {
  static metatype: NodeType = NodeType.INVITE_RESCINDED_EVENT;

  /**
   * The Space this Node is in.
   */
  get space(): Space | null {
    const nodePtr: NodeReference | null = this.spacePtr;
    if (nodePtr != null) {
      return this._supergraph.get(nodePtr.id) as Space | null;
    }
    return null;
  }
  readonly spacePtr: NodeReference;

  /**
   * The Snapshot this Event originated from.
   */
  get snapshot(): Snapshot | null {
    const nodePtr: NodeReference | null = this.snapshotPtr;
    if (nodePtr != null) {
      return this._supergraph.get(nodePtr.id) as Snapshot | null;
    }
    return null;
  }
  readonly snapshotPtr: NodeReference | null;

  /**
   * Event.createdAt
   */
  readonly createdAt: Temporal.ZonedDateTime;

  /**
   * Event.createdBy
   */
  get createdBy(): (Entity & IsActor) | null {
    const nodePtr: NodeReference | null = this.createdByPtr;
    if (nodePtr != null) {
      return this._supergraph.get(nodePtr.id) as (Entity & IsActor) | null;
    }
    return null;
  }
  readonly createdByPtr: NodeReference | null;

  /**
   * Event.client
   */
  get client(): Client | null {
    const nodePtr: NodeReference | null = this.clientPtr;
    if (nodePtr != null) {
      return this._supergraph.get(nodePtr.id) as Client | null;
    }
    return null;
  }
  readonly clientPtr: NodeReference | null;

  /**
   * Event.clientNonce
   */
  readonly clientNonce: string | null;

  /**
   * The status of the Event.
   */
  readonly status: EventStatus;

  /**
   * InviteEvent.node
   */
  get node(): Invite | null {
    const nodePtr: NodeReference | null = this.nodePtr;
    if (nodePtr != null) {
      return this._supergraph.get(nodePtr.id) as Invite | null;
    }
    return null;
  }
  readonly nodePtr: NodeReference;

  /**
   * InviteEvent.joinable
   */
  get joinable(): (Entity & IsJoinable) | null {
    const nodePtr: NodeReference | null = this.joinablePtr;
    if (nodePtr != null) {
      return this._supergraph.get(nodePtr.id) as (Entity & IsJoinable) | null;
    }
    return null;
  }
  readonly joinablePtr: NodeReference;

  /**
   * InviteEvent.member
   */
  get member(): (Entity & IsActor) | null {
    const nodePtr: NodeReference | null = this.memberPtr;
    if (nodePtr != null) {
      return this._supergraph.get(nodePtr.id) as (Entity & IsActor) | null;
    }
    return null;
  }
  readonly memberPtr: NodeReference;

  constructor(options: {
    id?: string;
    space?: Space | NodeReference;
    snapshot?: Snapshot | NodeReference | null;
    createdAt?: Temporal.ZonedDateTime;
    createdBy?: (Entity & IsActor) | NodeReference | null;
    client?: Client | NodeReference | null;
    clientNonce?: string | null;
    status?: EventStatus;
    node: Invite | NodeReference;
    joinable: (Entity & IsJoinable) | NodeReference;
    member: (Entity & IsActor) | NodeReference;
    _session?: Session | null;
    _supergraph?: Supergraph | null;
    _graph?: Graph | null;
    _connection?: QueryConnection | null;
  }) {
    super(
      // id
      options.id ?? null,
      // parent
      null,
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
    );

    // properties
    let _space = options.space ?? null;
    if (_space != null && _space.metatype != StructType.NODE_REFERENCE) {
      _space = (_space as Node).toRef();
    }
    if (_space === null) {
      if (this._session === null) {
        throw new Error(`InviteRescindedEvent has no Session`);
      }
      _space = ACTIVE_SPACE.get();
      if (_space === null) {
        throw new Error(`InviteRescindedEvent has no Space`);
      }
      _space = _space.toRef();
    }
    if (_space === null) {
      throw new Error(`InviteRescindedEvent.space is required`);
    }
    this.spacePtr = _space;
    let _snapshot = options.snapshot ?? null;
    if (_snapshot != null && _snapshot.metatype != StructType.NODE_REFERENCE) {
      _snapshot = (_snapshot as Node).toRef();
    }
    this.snapshotPtr = _snapshot;
    let _client = options.client ?? null;
    if (_client != null && _client.metatype != StructType.NODE_REFERENCE) {
      _client = (_client as Node).toRef();
    }
    this.clientPtr = _client;
    let _clientNonce = options.clientNonce ?? null;
    this.clientNonce = _clientNonce;
    let _status = options.status ?? null;
    if (_status === null) {
      _status = 1 /* EventStatus.PENDING */;
    }
    if (_status === null) {
      throw new Error(`InviteRescindedEvent.status is required`);
    }
    this.status = _status;
    let _node = options.node;
    if (_node != null && _node.metatype != StructType.NODE_REFERENCE) {
      _node = (_node as Node).toRef();
    }
    if (_node === null) {
      throw new Error(`InviteRescindedEvent.node is required`);
    }
    this.nodePtr = _node;
    let _joinable = options.joinable;
    if (_joinable != null && _joinable.metatype != StructType.NODE_REFERENCE) {
      _joinable = (_joinable as Node).toRef();
    }
    if (_joinable === null) {
      throw new Error(`InviteRescindedEvent.joinable is required`);
    }
    this.joinablePtr = _joinable;
    let _member = options.member;
    if (_member != null && _member.metatype != StructType.NODE_REFERENCE) {
      _member = (_member as Node).toRef();
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
        throw new Error(`InviteRescindedEvent.createdAt is required for existing Events`);
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
    if (!(this.snapshotPtr?.id === other.snapshotPtr?.id)) {
      return false;
    }
    if (!(this.clientPtr?.id === other.clientPtr?.id)) {
      return false;
    }
    if (!(this.clientNonce === other.clientNonce)) {
      return false;
    }
    if (!(this.status === other.status)) {
      return false;
    }
    if (!(this.spacePtr.id === other.spacePtr.id)) {
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
    if (this.snapshotPtr != null) {
      h = (h * 31 + hashString(this.snapshotPtr.id)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.createdAt.toString({ timeZoneName: "never" }))) & 0xffffffff;
    if (this.createdByPtr != null) {
      h = (h * 31 + hashString(this.createdByPtr.id)) & 0xffffffff;
    }
    if (this.clientPtr != null) {
      h = (h * 31 + hashString(this.clientPtr.id)) & 0xffffffff;
    }
    if (this.clientNonce != null) {
      h = (h * 31 + hashString(this.clientNonce.toString())) & 0xffffffff;
    }
    h = (h * 31 + this.status) & 0xffffffff;
    h = (h * 31 + hashString(this.id.toString())) & 0xffffffff;
    h = (h * 31 + hashString(this.spacePtr.id)) & 0xffffffff;

    return h;
  }

  validate(): void {
    throw new Error("not implemented");
  }

  __toRef__(): NodeReference {
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    return new _NodeReference({
      type: NodeType.INVITE_RESCINDED_EVENT,
      id: this.id,
      spaceId: this.spacePtr?.id ?? null,
      snapshotId: this.snapshotPtr?.id ?? null,
      _session: this._session,
      _supergraph: this._supergraph,
    });
  }

  get _pathKey(): string {
    return `InviteRescindedEvent[id=${this.id}]`;
  }

  get path(): string {
    const pathParts: string[] = [];
    let node: Entity | Event | null = this;
    let lastNode: Entity | Event | null = this;
    while (node != null) {
      pathParts.push(node._pathKey);
      lastNode = node;
      node = node.parent;
    }
    if (!lastNode.isRoot) {
      pathParts.push("<detached>");
    }
    return pathParts.reverse().join("/");
  }

  repr(): string {
    const propertyReprs: string[] = [];
    propertyReprs.push(`status=${EventStatus[this.status]}`);
    return `<InviteRescindedEvent "${this.path}" ${propertyReprs.join(" ")}>`;
  }

  toValue(): { readonly [key: string]: any } {
    return InviteRescindedEvent.__packValue__(this);
  }

  static __packValue__(object: InviteRescindedEvent): { readonly [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 60103;
    objectValue["2"] = String(object.id);
    objectValue["5"] = object.spacePtr.toValue();
    if (object.snapshotPtr != null) {
      objectValue["11"] = object.snapshotPtr.toValue();
    }
    objectValue["20"] = object.createdAt.toString({ timeZoneName: "never" });
    if (object.createdByPtr != null) {
      objectValue["21"] = object.createdByPtr.toValue();
    }
    if (object.clientPtr != null) {
      objectValue["22"] = object.clientPtr.toValue();
    }
    if (object.clientNonce != null) {
      objectValue["23"] = String(object.clientNonce);
    }
    objectValue["30"] = object.status;
    objectValue["101"] = object.nodePtr.toValue();
    objectValue["102"] = object.joinablePtr.toValue();
    objectValue["103"] = object.memberPtr.toValue();
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { readonly [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): InviteRescindedEvent {
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    const snapshotPtrValue = objectValue["11"];
    const unpackedSnapshotPtr =
      snapshotPtrValue != undefined
        ? _NodeReference.fromValue(snapshotPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const createdByPtrValue = objectValue["21"];
    const unpackedCreatedByPtr =
      createdByPtrValue != undefined
        ? _NodeReference.fromValue(createdByPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const clientPtrValue = objectValue["22"];
    const unpackedClientPtr =
      clientPtrValue != undefined
        ? _NodeReference.fromValue(clientPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const clientNonceValue = objectValue["23"];
    const unpackedClientNonce = clientNonceValue != undefined ? String(clientNonceValue) : null;
    return new InviteRescindedEvent({
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
      snapshot: unpackedSnapshotPtr,
      createdAt: Temporal.Instant.from(objectValue["20"]).toZonedDateTimeISO("UTC"),
      createdBy: unpackedCreatedByPtr,
      client: unpackedClientPtr,
      clientNonce: unpackedClientNonce,
      status: Number(objectValue["30"]),
      id: String(objectValue["2"]),
      space: _NodeReference.fromValue(objectValue["5"], _session, _supergraph, _graph, _connection),
      _session,
      _graph,
      _connection,
    });
  }

  static fromValue(
    objectValue: { readonly [key: string]: any },
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
    const objectProto: Partial<InviteRescindedEventProto> = { metatype: 60103 };
    objectProto.id = String(object.id);
    objectProto.spacePtr = object.spacePtr.toProto();
    if (object.snapshotPtr != null) {
      objectProto.snapshotPtr = object.snapshotPtr.toProto();
    }
    objectProto.createdAt = packProtoTimestamp(object.createdAt);
    if (object.createdByPtr != null) {
      objectProto.createdByPtr = object.createdByPtr.toProto();
    }
    if (object.clientPtr != null) {
      objectProto.clientPtr = object.clientPtr.toProto();
    }
    if (object.clientNonce != null) {
      objectProto.clientNonce = String(object.clientNonce);
    }
    objectProto.status = Number(object.status) as EventStatusProto;
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
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    return new InviteRescindedEvent({
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
      snapshot:
        objectProto.snapshotPtr != undefined
          ? _NodeReference.fromProto(
              objectProto.snapshotPtr!,
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
      client:
        objectProto.clientPtr != undefined
          ? _NodeReference.fromProto(
              objectProto.clientPtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      clientNonce: objectProto.clientNonce != undefined ? String(objectProto.clientNonce) : null,
      status: Number(objectProto.status) as EventStatus,
      id: String(objectProto.id),
      space: _NodeReference.fromProto(
        objectProto.spacePtr!,
        _session,
        _supergraph,
        _graph,
        _connection,
      ),
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
/* ==== DESTACK_GENERATED_END:NODE:60103 ==== */

/* ==== DESTACK_GENERATED_START:NODE:60104 ==== */
/**
 * An Invite was accepted.
 */
export class InviteAcceptedEvent extends InviteEvent {
  static metatype: NodeType = NodeType.INVITE_ACCEPTED_EVENT;

  /**
   * The Space this Node is in.
   */
  get space(): Space | null {
    const nodePtr: NodeReference | null = this.spacePtr;
    if (nodePtr != null) {
      return this._supergraph.get(nodePtr.id) as Space | null;
    }
    return null;
  }
  readonly spacePtr: NodeReference;

  /**
   * The Snapshot this Event originated from.
   */
  get snapshot(): Snapshot | null {
    const nodePtr: NodeReference | null = this.snapshotPtr;
    if (nodePtr != null) {
      return this._supergraph.get(nodePtr.id) as Snapshot | null;
    }
    return null;
  }
  readonly snapshotPtr: NodeReference | null;

  /**
   * Event.createdAt
   */
  readonly createdAt: Temporal.ZonedDateTime;

  /**
   * Event.createdBy
   */
  get createdBy(): (Entity & IsActor) | null {
    const nodePtr: NodeReference | null = this.createdByPtr;
    if (nodePtr != null) {
      return this._supergraph.get(nodePtr.id) as (Entity & IsActor) | null;
    }
    return null;
  }
  readonly createdByPtr: NodeReference | null;

  /**
   * Event.client
   */
  get client(): Client | null {
    const nodePtr: NodeReference | null = this.clientPtr;
    if (nodePtr != null) {
      return this._supergraph.get(nodePtr.id) as Client | null;
    }
    return null;
  }
  readonly clientPtr: NodeReference | null;

  /**
   * Event.clientNonce
   */
  readonly clientNonce: string | null;

  /**
   * The status of the Event.
   */
  readonly status: EventStatus;

  /**
   * InviteEvent.node
   */
  get node(): Invite | null {
    const nodePtr: NodeReference | null = this.nodePtr;
    if (nodePtr != null) {
      return this._supergraph.get(nodePtr.id) as Invite | null;
    }
    return null;
  }
  readonly nodePtr: NodeReference;

  /**
   * InviteEvent.joinable
   */
  get joinable(): (Entity & IsJoinable) | null {
    const nodePtr: NodeReference | null = this.joinablePtr;
    if (nodePtr != null) {
      return this._supergraph.get(nodePtr.id) as (Entity & IsJoinable) | null;
    }
    return null;
  }
  readonly joinablePtr: NodeReference;

  /**
   * InviteEvent.member
   */
  get member(): (Entity & IsActor) | null {
    const nodePtr: NodeReference | null = this.memberPtr;
    if (nodePtr != null) {
      return this._supergraph.get(nodePtr.id) as (Entity & IsActor) | null;
    }
    return null;
  }
  readonly memberPtr: NodeReference;

  /**
   * InviteAcceptedEvent.role
   */
  get role(): Role | null {
    const nodePtr: NodeReference | null = this.rolePtr;
    if (nodePtr != null) {
      return this._supergraph.get(nodePtr.id) as Role | null;
    }
    return null;
  }
  readonly rolePtr: NodeReference;

  /**
   * InviteAcceptedEvent.roleType
   */
  readonly roleType: RoleType;

  constructor(options: {
    id?: string;
    space?: Space | NodeReference;
    snapshot?: Snapshot | NodeReference | null;
    createdAt?: Temporal.ZonedDateTime;
    createdBy?: (Entity & IsActor) | NodeReference | null;
    client?: Client | NodeReference | null;
    clientNonce?: string | null;
    status?: EventStatus;
    node: Invite | NodeReference;
    joinable: (Entity & IsJoinable) | NodeReference;
    member: (Entity & IsActor) | NodeReference;
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
      null,
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
    );

    // properties
    let _space = options.space ?? null;
    if (_space != null && _space.metatype != StructType.NODE_REFERENCE) {
      _space = (_space as Node).toRef();
    }
    if (_space === null) {
      if (this._session === null) {
        throw new Error(`InviteAcceptedEvent has no Session`);
      }
      _space = ACTIVE_SPACE.get();
      if (_space === null) {
        throw new Error(`InviteAcceptedEvent has no Space`);
      }
      _space = _space.toRef();
    }
    if (_space === null) {
      throw new Error(`InviteAcceptedEvent.space is required`);
    }
    this.spacePtr = _space;
    let _snapshot = options.snapshot ?? null;
    if (_snapshot != null && _snapshot.metatype != StructType.NODE_REFERENCE) {
      _snapshot = (_snapshot as Node).toRef();
    }
    this.snapshotPtr = _snapshot;
    let _client = options.client ?? null;
    if (_client != null && _client.metatype != StructType.NODE_REFERENCE) {
      _client = (_client as Node).toRef();
    }
    this.clientPtr = _client;
    let _clientNonce = options.clientNonce ?? null;
    this.clientNonce = _clientNonce;
    let _status = options.status ?? null;
    if (_status === null) {
      _status = 1 /* EventStatus.PENDING */;
    }
    if (_status === null) {
      throw new Error(`InviteAcceptedEvent.status is required`);
    }
    this.status = _status;
    let _node = options.node;
    if (_node != null && _node.metatype != StructType.NODE_REFERENCE) {
      _node = (_node as Node).toRef();
    }
    if (_node === null) {
      throw new Error(`InviteAcceptedEvent.node is required`);
    }
    this.nodePtr = _node;
    let _joinable = options.joinable;
    if (_joinable != null && _joinable.metatype != StructType.NODE_REFERENCE) {
      _joinable = (_joinable as Node).toRef();
    }
    if (_joinable === null) {
      throw new Error(`InviteAcceptedEvent.joinable is required`);
    }
    this.joinablePtr = _joinable;
    let _member = options.member;
    if (_member != null && _member.metatype != StructType.NODE_REFERENCE) {
      _member = (_member as Node).toRef();
    }
    if (_member === null) {
      throw new Error(`InviteAcceptedEvent.member is required`);
    }
    this.memberPtr = _member;
    let _role = options.role;
    if (_role != null && _role.metatype != StructType.NODE_REFERENCE) {
      _role = (_role as Node).toRef();
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
        throw new Error(`InviteAcceptedEvent.createdAt is required for existing Events`);
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
    if (!(this.snapshotPtr?.id === other.snapshotPtr?.id)) {
      return false;
    }
    if (!(this.clientPtr?.id === other.clientPtr?.id)) {
      return false;
    }
    if (!(this.clientNonce === other.clientNonce)) {
      return false;
    }
    if (!(this.status === other.status)) {
      return false;
    }
    if (!(this.spacePtr.id === other.spacePtr.id)) {
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
    if (this.snapshotPtr != null) {
      h = (h * 31 + hashString(this.snapshotPtr.id)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.createdAt.toString({ timeZoneName: "never" }))) & 0xffffffff;
    if (this.createdByPtr != null) {
      h = (h * 31 + hashString(this.createdByPtr.id)) & 0xffffffff;
    }
    if (this.clientPtr != null) {
      h = (h * 31 + hashString(this.clientPtr.id)) & 0xffffffff;
    }
    if (this.clientNonce != null) {
      h = (h * 31 + hashString(this.clientNonce.toString())) & 0xffffffff;
    }
    h = (h * 31 + this.status) & 0xffffffff;
    h = (h * 31 + hashString(this.id.toString())) & 0xffffffff;
    h = (h * 31 + hashString(this.spacePtr.id)) & 0xffffffff;

    return h;
  }

  validate(): void {
    throw new Error("not implemented");
  }

  __toRef__(): NodeReference {
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    return new _NodeReference({
      type: NodeType.INVITE_ACCEPTED_EVENT,
      id: this.id,
      spaceId: this.spacePtr?.id ?? null,
      snapshotId: this.snapshotPtr?.id ?? null,
      _session: this._session,
      _supergraph: this._supergraph,
    });
  }

  get _pathKey(): string {
    return `InviteAcceptedEvent[id=${this.id}]`;
  }

  get path(): string {
    const pathParts: string[] = [];
    let node: Entity | Event | null = this;
    let lastNode: Entity | Event | null = this;
    while (node != null) {
      pathParts.push(node._pathKey);
      lastNode = node;
      node = node.parent;
    }
    if (!lastNode.isRoot) {
      pathParts.push("<detached>");
    }
    return pathParts.reverse().join("/");
  }

  repr(): string {
    const propertyReprs: string[] = [];
    propertyReprs.push(`status=${EventStatus[this.status]}`);
    return `<InviteAcceptedEvent "${this.path}" ${propertyReprs.join(" ")}>`;
  }

  toValue(): { readonly [key: string]: any } {
    return InviteAcceptedEvent.__packValue__(this);
  }

  static __packValue__(object: InviteAcceptedEvent): { readonly [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 60104;
    objectValue["2"] = String(object.id);
    objectValue["5"] = object.spacePtr.toValue();
    if (object.snapshotPtr != null) {
      objectValue["11"] = object.snapshotPtr.toValue();
    }
    objectValue["20"] = object.createdAt.toString({ timeZoneName: "never" });
    if (object.createdByPtr != null) {
      objectValue["21"] = object.createdByPtr.toValue();
    }
    if (object.clientPtr != null) {
      objectValue["22"] = object.clientPtr.toValue();
    }
    if (object.clientNonce != null) {
      objectValue["23"] = String(object.clientNonce);
    }
    objectValue["30"] = object.status;
    objectValue["101"] = object.nodePtr.toValue();
    objectValue["102"] = object.joinablePtr.toValue();
    objectValue["103"] = object.memberPtr.toValue();
    objectValue["110"] = object.rolePtr.toValue();
    objectValue["111"] = object.roleType;
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { readonly [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): InviteAcceptedEvent {
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    const snapshotPtrValue = objectValue["11"];
    const unpackedSnapshotPtr =
      snapshotPtrValue != undefined
        ? _NodeReference.fromValue(snapshotPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const createdByPtrValue = objectValue["21"];
    const unpackedCreatedByPtr =
      createdByPtrValue != undefined
        ? _NodeReference.fromValue(createdByPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const clientPtrValue = objectValue["22"];
    const unpackedClientPtr =
      clientPtrValue != undefined
        ? _NodeReference.fromValue(clientPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const clientNonceValue = objectValue["23"];
    const unpackedClientNonce = clientNonceValue != undefined ? String(clientNonceValue) : null;
    return new InviteAcceptedEvent({
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
      snapshot: unpackedSnapshotPtr,
      createdAt: Temporal.Instant.from(objectValue["20"]).toZonedDateTimeISO("UTC"),
      createdBy: unpackedCreatedByPtr,
      client: unpackedClientPtr,
      clientNonce: unpackedClientNonce,
      status: Number(objectValue["30"]),
      id: String(objectValue["2"]),
      space: _NodeReference.fromValue(objectValue["5"], _session, _supergraph, _graph, _connection),
      _session,
      _graph,
      _connection,
    });
  }

  static fromValue(
    objectValue: { readonly [key: string]: any },
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
    const objectProto: Partial<InviteAcceptedEventProto> = { metatype: 60104 };
    objectProto.id = String(object.id);
    objectProto.spacePtr = object.spacePtr.toProto();
    if (object.snapshotPtr != null) {
      objectProto.snapshotPtr = object.snapshotPtr.toProto();
    }
    objectProto.createdAt = packProtoTimestamp(object.createdAt);
    if (object.createdByPtr != null) {
      objectProto.createdByPtr = object.createdByPtr.toProto();
    }
    if (object.clientPtr != null) {
      objectProto.clientPtr = object.clientPtr.toProto();
    }
    if (object.clientNonce != null) {
      objectProto.clientNonce = String(object.clientNonce);
    }
    objectProto.status = Number(object.status) as EventStatusProto;
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
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    return new InviteAcceptedEvent({
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
      snapshot:
        objectProto.snapshotPtr != undefined
          ? _NodeReference.fromProto(
              objectProto.snapshotPtr!,
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
      client:
        objectProto.clientPtr != undefined
          ? _NodeReference.fromProto(
              objectProto.clientPtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      clientNonce: objectProto.clientNonce != undefined ? String(objectProto.clientNonce) : null,
      status: Number(objectProto.status) as EventStatus,
      id: String(objectProto.id),
      space: _NodeReference.fromProto(
        objectProto.spacePtr!,
        _session,
        _supergraph,
        _graph,
        _connection,
      ),
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
/* ==== DESTACK_GENERATED_END:NODE:60104 ==== */

/* ==== DESTACK_GENERATED_START:NODE:60105 ==== */
/**
 * An Invite was rejected.
 */
export class InviteRejectedEvent extends InviteEvent {
  static metatype: NodeType = NodeType.INVITE_REJECTED_EVENT;

  /**
   * The Space this Node is in.
   */
  get space(): Space | null {
    const nodePtr: NodeReference | null = this.spacePtr;
    if (nodePtr != null) {
      return this._supergraph.get(nodePtr.id) as Space | null;
    }
    return null;
  }
  readonly spacePtr: NodeReference;

  /**
   * The Snapshot this Event originated from.
   */
  get snapshot(): Snapshot | null {
    const nodePtr: NodeReference | null = this.snapshotPtr;
    if (nodePtr != null) {
      return this._supergraph.get(nodePtr.id) as Snapshot | null;
    }
    return null;
  }
  readonly snapshotPtr: NodeReference | null;

  /**
   * Event.createdAt
   */
  readonly createdAt: Temporal.ZonedDateTime;

  /**
   * Event.createdBy
   */
  get createdBy(): (Entity & IsActor) | null {
    const nodePtr: NodeReference | null = this.createdByPtr;
    if (nodePtr != null) {
      return this._supergraph.get(nodePtr.id) as (Entity & IsActor) | null;
    }
    return null;
  }
  readonly createdByPtr: NodeReference | null;

  /**
   * Event.client
   */
  get client(): Client | null {
    const nodePtr: NodeReference | null = this.clientPtr;
    if (nodePtr != null) {
      return this._supergraph.get(nodePtr.id) as Client | null;
    }
    return null;
  }
  readonly clientPtr: NodeReference | null;

  /**
   * Event.clientNonce
   */
  readonly clientNonce: string | null;

  /**
   * The status of the Event.
   */
  readonly status: EventStatus;

  /**
   * InviteEvent.node
   */
  get node(): Invite | null {
    const nodePtr: NodeReference | null = this.nodePtr;
    if (nodePtr != null) {
      return this._supergraph.get(nodePtr.id) as Invite | null;
    }
    return null;
  }
  readonly nodePtr: NodeReference;

  /**
   * InviteEvent.joinable
   */
  get joinable(): (Entity & IsJoinable) | null {
    const nodePtr: NodeReference | null = this.joinablePtr;
    if (nodePtr != null) {
      return this._supergraph.get(nodePtr.id) as (Entity & IsJoinable) | null;
    }
    return null;
  }
  readonly joinablePtr: NodeReference;

  /**
   * InviteEvent.member
   */
  get member(): (Entity & IsActor) | null {
    const nodePtr: NodeReference | null = this.memberPtr;
    if (nodePtr != null) {
      return this._supergraph.get(nodePtr.id) as (Entity & IsActor) | null;
    }
    return null;
  }
  readonly memberPtr: NodeReference;

  constructor(options: {
    id?: string;
    space?: Space | NodeReference;
    snapshot?: Snapshot | NodeReference | null;
    createdAt?: Temporal.ZonedDateTime;
    createdBy?: (Entity & IsActor) | NodeReference | null;
    client?: Client | NodeReference | null;
    clientNonce?: string | null;
    status?: EventStatus;
    node: Invite | NodeReference;
    joinable: (Entity & IsJoinable) | NodeReference;
    member: (Entity & IsActor) | NodeReference;
    _session?: Session | null;
    _supergraph?: Supergraph | null;
    _graph?: Graph | null;
    _connection?: QueryConnection | null;
  }) {
    super(
      // id
      options.id ?? null,
      // parent
      null,
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
    );

    // properties
    let _space = options.space ?? null;
    if (_space != null && _space.metatype != StructType.NODE_REFERENCE) {
      _space = (_space as Node).toRef();
    }
    if (_space === null) {
      if (this._session === null) {
        throw new Error(`InviteRejectedEvent has no Session`);
      }
      _space = ACTIVE_SPACE.get();
      if (_space === null) {
        throw new Error(`InviteRejectedEvent has no Space`);
      }
      _space = _space.toRef();
    }
    if (_space === null) {
      throw new Error(`InviteRejectedEvent.space is required`);
    }
    this.spacePtr = _space;
    let _snapshot = options.snapshot ?? null;
    if (_snapshot != null && _snapshot.metatype != StructType.NODE_REFERENCE) {
      _snapshot = (_snapshot as Node).toRef();
    }
    this.snapshotPtr = _snapshot;
    let _client = options.client ?? null;
    if (_client != null && _client.metatype != StructType.NODE_REFERENCE) {
      _client = (_client as Node).toRef();
    }
    this.clientPtr = _client;
    let _clientNonce = options.clientNonce ?? null;
    this.clientNonce = _clientNonce;
    let _status = options.status ?? null;
    if (_status === null) {
      _status = 1 /* EventStatus.PENDING */;
    }
    if (_status === null) {
      throw new Error(`InviteRejectedEvent.status is required`);
    }
    this.status = _status;
    let _node = options.node;
    if (_node != null && _node.metatype != StructType.NODE_REFERENCE) {
      _node = (_node as Node).toRef();
    }
    if (_node === null) {
      throw new Error(`InviteRejectedEvent.node is required`);
    }
    this.nodePtr = _node;
    let _joinable = options.joinable;
    if (_joinable != null && _joinable.metatype != StructType.NODE_REFERENCE) {
      _joinable = (_joinable as Node).toRef();
    }
    if (_joinable === null) {
      throw new Error(`InviteRejectedEvent.joinable is required`);
    }
    this.joinablePtr = _joinable;
    let _member = options.member;
    if (_member != null && _member.metatype != StructType.NODE_REFERENCE) {
      _member = (_member as Node).toRef();
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
        throw new Error(`InviteRejectedEvent.createdAt is required for existing Events`);
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
    if (!(this.snapshotPtr?.id === other.snapshotPtr?.id)) {
      return false;
    }
    if (!(this.clientPtr?.id === other.clientPtr?.id)) {
      return false;
    }
    if (!(this.clientNonce === other.clientNonce)) {
      return false;
    }
    if (!(this.status === other.status)) {
      return false;
    }
    if (!(this.spacePtr.id === other.spacePtr.id)) {
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
    if (this.snapshotPtr != null) {
      h = (h * 31 + hashString(this.snapshotPtr.id)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.createdAt.toString({ timeZoneName: "never" }))) & 0xffffffff;
    if (this.createdByPtr != null) {
      h = (h * 31 + hashString(this.createdByPtr.id)) & 0xffffffff;
    }
    if (this.clientPtr != null) {
      h = (h * 31 + hashString(this.clientPtr.id)) & 0xffffffff;
    }
    if (this.clientNonce != null) {
      h = (h * 31 + hashString(this.clientNonce.toString())) & 0xffffffff;
    }
    h = (h * 31 + this.status) & 0xffffffff;
    h = (h * 31 + hashString(this.id.toString())) & 0xffffffff;
    h = (h * 31 + hashString(this.spacePtr.id)) & 0xffffffff;

    return h;
  }

  validate(): void {
    throw new Error("not implemented");
  }

  __toRef__(): NodeReference {
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    return new _NodeReference({
      type: NodeType.INVITE_REJECTED_EVENT,
      id: this.id,
      spaceId: this.spacePtr?.id ?? null,
      snapshotId: this.snapshotPtr?.id ?? null,
      _session: this._session,
      _supergraph: this._supergraph,
    });
  }

  get _pathKey(): string {
    return `InviteRejectedEvent[id=${this.id}]`;
  }

  get path(): string {
    const pathParts: string[] = [];
    let node: Entity | Event | null = this;
    let lastNode: Entity | Event | null = this;
    while (node != null) {
      pathParts.push(node._pathKey);
      lastNode = node;
      node = node.parent;
    }
    if (!lastNode.isRoot) {
      pathParts.push("<detached>");
    }
    return pathParts.reverse().join("/");
  }

  repr(): string {
    const propertyReprs: string[] = [];
    propertyReprs.push(`status=${EventStatus[this.status]}`);
    return `<InviteRejectedEvent "${this.path}" ${propertyReprs.join(" ")}>`;
  }

  toValue(): { readonly [key: string]: any } {
    return InviteRejectedEvent.__packValue__(this);
  }

  static __packValue__(object: InviteRejectedEvent): { readonly [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 60105;
    objectValue["2"] = String(object.id);
    objectValue["5"] = object.spacePtr.toValue();
    if (object.snapshotPtr != null) {
      objectValue["11"] = object.snapshotPtr.toValue();
    }
    objectValue["20"] = object.createdAt.toString({ timeZoneName: "never" });
    if (object.createdByPtr != null) {
      objectValue["21"] = object.createdByPtr.toValue();
    }
    if (object.clientPtr != null) {
      objectValue["22"] = object.clientPtr.toValue();
    }
    if (object.clientNonce != null) {
      objectValue["23"] = String(object.clientNonce);
    }
    objectValue["30"] = object.status;
    objectValue["101"] = object.nodePtr.toValue();
    objectValue["102"] = object.joinablePtr.toValue();
    objectValue["103"] = object.memberPtr.toValue();
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { readonly [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): InviteRejectedEvent {
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    const snapshotPtrValue = objectValue["11"];
    const unpackedSnapshotPtr =
      snapshotPtrValue != undefined
        ? _NodeReference.fromValue(snapshotPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const createdByPtrValue = objectValue["21"];
    const unpackedCreatedByPtr =
      createdByPtrValue != undefined
        ? _NodeReference.fromValue(createdByPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const clientPtrValue = objectValue["22"];
    const unpackedClientPtr =
      clientPtrValue != undefined
        ? _NodeReference.fromValue(clientPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const clientNonceValue = objectValue["23"];
    const unpackedClientNonce = clientNonceValue != undefined ? String(clientNonceValue) : null;
    return new InviteRejectedEvent({
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
      snapshot: unpackedSnapshotPtr,
      createdAt: Temporal.Instant.from(objectValue["20"]).toZonedDateTimeISO("UTC"),
      createdBy: unpackedCreatedByPtr,
      client: unpackedClientPtr,
      clientNonce: unpackedClientNonce,
      status: Number(objectValue["30"]),
      id: String(objectValue["2"]),
      space: _NodeReference.fromValue(objectValue["5"], _session, _supergraph, _graph, _connection),
      _session,
      _graph,
      _connection,
    });
  }

  static fromValue(
    objectValue: { readonly [key: string]: any },
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
    const objectProto: Partial<InviteRejectedEventProto> = { metatype: 60105 };
    objectProto.id = String(object.id);
    objectProto.spacePtr = object.spacePtr.toProto();
    if (object.snapshotPtr != null) {
      objectProto.snapshotPtr = object.snapshotPtr.toProto();
    }
    objectProto.createdAt = packProtoTimestamp(object.createdAt);
    if (object.createdByPtr != null) {
      objectProto.createdByPtr = object.createdByPtr.toProto();
    }
    if (object.clientPtr != null) {
      objectProto.clientPtr = object.clientPtr.toProto();
    }
    if (object.clientNonce != null) {
      objectProto.clientNonce = String(object.clientNonce);
    }
    objectProto.status = Number(object.status) as EventStatusProto;
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
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    return new InviteRejectedEvent({
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
      snapshot:
        objectProto.snapshotPtr != undefined
          ? _NodeReference.fromProto(
              objectProto.snapshotPtr!,
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
      client:
        objectProto.clientPtr != undefined
          ? _NodeReference.fromProto(
              objectProto.clientPtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      clientNonce: objectProto.clientNonce != undefined ? String(objectProto.clientNonce) : null,
      status: Number(objectProto.status) as EventStatus,
      id: String(objectProto.id),
      space: _NodeReference.fromProto(
        objectProto.spacePtr!,
        _session,
        _supergraph,
        _graph,
        _connection,
      ),
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
/* ==== DESTACK_GENERATED_END:NODE:60105 ==== */

/* ==== DESTACK_GENERATED_START:NODE:60100 ==== */
/**
 * An Invite to a Joinable.
 */
export class Invite extends Entity implements IsOwnable, IsDeletable, IsExtensible {
  static metatype: NodeType = NodeType.INVITE;

  /**
   * Invite.parent
   */
  get parent(): (Entity & IsJoinable) | null {
    const nodePtr: NodeReference | null = this.parentPtr;
    if (nodePtr != null) {
      return this._supergraph.get(nodePtr.id) as (Entity & IsJoinable) | null;
    }
    return null;
  }
  readonly parentPtr: NodeReference | null;

  /**
   * The Space this Node is in.
   */
  get space(): Space | null {
    const nodePtr: NodeReference | null = this.spacePtr;
    if (nodePtr != null) {
      return this._supergraph.get(nodePtr.id) as Space | null;
    }
    return null;
  }
  readonly spacePtr: NodeReference;

  /**
   * The definition this CustomEntity is an instance of.
   */
  get definition(): (Entity & IsExtensible) | null {
    const nodePtr: NodeReference | null = this.definitionPtr;
    if (nodePtr != null) {
      return this._supergraph.get(nodePtr.id) as (Entity & IsExtensible) | null;
    }
    return null;
  }
  readonly definitionPtr: NodeReference | null;

  /**
   * Entity.materialization
   */
  readonly materialization: Materialization;

  /**
   * The Snapshot this Entity is part of.
   */
  get snapshot(): Snapshot | null {
    const nodePtr: NodeReference | null = this.snapshotPtr;
    if (nodePtr != null) {
      return this._supergraph.get(nodePtr.id) as Snapshot | null;
    }
    return null;
  }
  readonly snapshotPtr: NodeReference | null;

  /**
   * The previous Entity this Entity is based on (from another Snapshot).
   */
  get predecessor(): Invite | null {
    const nodePtr: NodeReference | null = this.predecessorPtr;
    if (nodePtr != null) {
      return this._supergraph.get(nodePtr.id) as Invite | null;
    }
    return null;
  }
  readonly predecessorPtr: NodeReference | null;

  /**
   * The time this Entity was created.
   */
  readonly createdAt: Temporal.ZonedDateTime;

  /**
   * The Actor that created this Entity.
   */
  get createdBy(): (Entity & IsActor) | null {
    const nodePtr: NodeReference | null = this.createdByPtr;
    if (nodePtr != null) {
      return this._supergraph.get(nodePtr.id) as (Entity & IsActor) | null;
    }
    return null;
  }
  readonly createdByPtr: NodeReference | null;

  /**
   * The time this Entity was last updated.
   */
  readonly updatedAt: Temporal.ZonedDateTime;

  /**
   * The Actor that last updated this Entity.
   */
  get updatedBy(): (Entity & IsActor) | null {
    const nodePtr: NodeReference | null = this.updatedByPtr;
    if (nodePtr != null) {
      return this._supergraph.get(nodePtr.id) as (Entity & IsActor) | null;
    }
    return null;
  }
  readonly updatedByPtr: NodeReference | null;

  /**
   * IsDeletable.deletedAt
   */
  readonly deletedAt: Temporal.ZonedDateTime | null;

  /**
   * The custom Values of this Node, keyed by custom Property id. May hold both static and instance values.
   */
  /**
   * The custom Values of this Node, keyed by custom Property id. May hold both static and instance values.
   */
  get customValues(): { readonly [key: string]: Value } {
    return this._customValues;
  }
  set customValues(value: { readonly [key: string]: Value }) {
    const prop = (this.constructor as NodeClass).__properties__["custom_values"];
    this._session.updateSetProperty(this, prop, value);
    this._customValues = value;
  }
  _customValues: { readonly [key: string]: Value };

  /**
   * IsOwnable.ownedBy
   */
  get ownedBy(): (Entity & IsActor) | null {
    const nodePtr: NodeReference | null = this.ownedByPtr;
    if (nodePtr != null) {
      return this._supergraph.get(nodePtr.id) as (Entity & IsActor) | null;
    }
    return null;
  }
  set ownedBy(node: (Entity & IsActor) | null) {
    if (node === null) {
      this.ownedByPtr = null;
    } else {
      this.ownedByPtr = node.toRef();
    }
  }
  /**
   * IsOwnable.ownedBy
   */
  get ownedByPtr(): NodeReference | null {
    return this._ownedByPtr;
  }
  set ownedByPtr(value: NodeReference | null) {
    const prop = (this.constructor as NodeClass).__properties__["owned_by"];
    this._session.updateSetProperty(this, prop, value);
    this._ownedByPtr = value;
  }
  _ownedByPtr: NodeReference | null;

  /**
   * The main / root Script of this Node.
   */
  get script(): Script | null {
    const nodePtr: NodeReference | null = this.scriptPtr;
    if (nodePtr != null) {
      return this._supergraph.get(nodePtr.id) as Script | null;
    }
    return null;
  }
  set script(node: Script | null) {
    if (node === null) {
      this.scriptPtr = null;
    } else {
      this.scriptPtr = node.toRef();
    }
  }
  /**
   * The main / root Script of this Node.
   */
  get scriptPtr(): NodeReference | null {
    return this._scriptPtr;
  }
  set scriptPtr(value: NodeReference | null) {
    const prop = (this.constructor as NodeClass).__properties__["script"];
    this._session.updateSetProperty(this, prop, value);
    this._scriptPtr = value;
  }
  _scriptPtr: NodeReference | null;

  /**
   * Whether this Node is extensible (whether it can be instanced).
   */
  readonly isExtensible: boolean;

  /**
   * Invite.member
   */
  get member(): (Entity & IsActor) | null {
    const nodePtr: NodeReference | null = this.memberPtr;
    if (nodePtr != null) {
      return this._supergraph.get(nodePtr.id) as (Entity & IsActor) | null;
    }
    return null;
  }
  set member(node: Entity & IsActor) {
    this.memberPtr = node.toRef();
  }
  /**
   * Invite.member
   */
  get memberPtr(): NodeReference {
    return this._memberPtr;
  }
  set memberPtr(value: NodeReference) {
    const prop = (this.constructor as NodeClass).__properties__["member"];
    this._session.updateSetProperty(this, prop, value);
    this._memberPtr = value;
  }
  _memberPtr: NodeReference;

  /**
   * Invite.role
   */
  get role(): Role | null {
    const nodePtr: NodeReference | null = this.rolePtr;
    if (nodePtr != null) {
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
  /**
   * Invite.role
   */
  get rolePtr(): NodeReference | null {
    return this._rolePtr;
  }
  set rolePtr(value: NodeReference | null) {
    const prop = (this.constructor as NodeClass).__properties__["role"];
    this._session.updateSetProperty(this, prop, value);
    this._rolePtr = value;
  }
  _rolePtr: NodeReference | null;

  /**
   * Invite.roleType
   */
  /**
   * Invite.roleType
   */
  get roleType(): RoleType | null {
    return this._roleType;
  }
  set roleType(value: RoleType | null) {
    const prop = (this.constructor as NodeClass).__properties__["role_type"];
    this._session.updateSetProperty(this, prop, value);
    this._roleType = value;
  }
  _roleType: RoleType | null;

  constructor(options: {
    id?: string;
    parent?: (Entity & IsJoinable) | NodeReference | null;
    space?: Space | NodeReference;
    definition?: (Entity & IsExtensible) | NodeReference | null;
    materialization?: Materialization;
    snapshot?: Snapshot | NodeReference | null;
    predecessor?: Invite | NodeReference | null;
    createdAt?: Temporal.ZonedDateTime;
    createdBy?: (Entity & IsActor) | NodeReference | null;
    updatedAt?: Temporal.ZonedDateTime;
    updatedBy?: (Entity & IsActor) | NodeReference | null;
    deletedAt?: Temporal.ZonedDateTime | null;
    customValues?: { readonly [key: string]: Value };
    ownedBy?: (Entity & IsActor) | NodeReference | null;
    script?: Script | NodeReference | null;
    isExtensible?: boolean;
    member: (Entity & IsActor) | NodeReference;
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
    if (_space === null) {
      if (this._session === null) {
        throw new Error(`Invite has no Session`);
      }
      _space = ACTIVE_SPACE.get();
      if (_space === null) {
        throw new Error(`Invite has no Space`);
      }
      _space = _space.toRef();
    }
    if (_space === null) {
      throw new Error(`Invite.space is required`);
    }
    this.spacePtr = _space;
    let _definition = options.definition ?? null;
    if (_definition != null && _definition.metatype != StructType.NODE_REFERENCE) {
      _definition = (_definition as Node).toRef();
    }
    this.definitionPtr = _definition;
    let _materialization = options.materialization ?? null;
    if (_materialization === null) {
      _materialization = 3 /* Materialization.ROOT */;
    }
    if (_materialization === null) {
      throw new Error(`Invite.materialization is required`);
    }
    this.materialization = _materialization;
    let _snapshot = options.snapshot ?? null;
    if (_snapshot != null && _snapshot.metatype != StructType.NODE_REFERENCE) {
      _snapshot = (_snapshot as Node).toRef();
    }
    this.snapshotPtr = _snapshot;
    let _predecessor = options.predecessor ?? null;
    if (_predecessor != null && _predecessor.metatype != StructType.NODE_REFERENCE) {
      _predecessor = (_predecessor as Node).toRef();
    }
    this.predecessorPtr = _predecessor;
    let _deletedAt = options.deletedAt ?? null;
    this.deletedAt = _deletedAt;
    let _customValues = options.customValues ?? null;
    if (_customValues === null) {
      _customValues = {};
    }
    this._customValues = _customValues;
    let _ownedBy = options.ownedBy ?? null;
    if (_ownedBy != null && _ownedBy.metatype != StructType.NODE_REFERENCE) {
      _ownedBy = (_ownedBy as Node).toRef();
    }
    this._ownedByPtr = _ownedBy;
    let _script = options.script ?? null;
    if (_script != null && _script.metatype != StructType.NODE_REFERENCE) {
      _script = (_script as Node).toRef();
    }
    this._scriptPtr = _script;
    let _isExtensible = options.isExtensible ?? null;
    if (_isExtensible === null) {
      _isExtensible = false;
    }
    if (_isExtensible === null) {
      throw new Error(`Invite.isExtensible is required`);
    }
    this.isExtensible = _isExtensible;
    let _member = options.member;
    if (_member != null && _member.metatype != StructType.NODE_REFERENCE) {
      _member = (_member as Node).toRef();
    }
    if (_member === null) {
      throw new Error(`Invite.member is required`);
    }
    this._memberPtr = _member;
    let _role = options.role ?? null;
    if (_role != null && _role.metatype != StructType.NODE_REFERENCE) {
      _role = (_role as Node).toRef();
    }
    this._rolePtr = _role;
    let _roleType = options.roleType ?? null;
    this._roleType = _roleType;

    // identity
    if (options.id == null) {
      const now = Temporal.Now.zonedDateTimeISO("UTC");
      this.createdAt = now;
      this.createdByPtr = null;
      this.updatedAt = now;
      this.updatedByPtr = null;
    } else {
      if (options.createdAt == null || options.updatedAt == null) {
        throw new Error(`Invite.createdAt and Invite.updatedAt are required for existing Nodes`);
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
    if (!(this._memberPtr.id === other._memberPtr.id)) {
      return false;
    }
    if (!(this._rolePtr?.id === other._rolePtr?.id)) {
      return false;
    }
    if (!(this._roleType === other._roleType)) {
      return false;
    }
    if (!(this._ownedByPtr?.id === other._ownedByPtr?.id)) {
      return false;
    }
    if (!(this.definitionPtr?.id === other.definitionPtr?.id)) {
      return false;
    }
    if (!(this.isExtensible === other.isExtensible)) {
      return false;
    }
    if (!(this.snapshotPtr?.id === other.snapshotPtr?.id)) {
      return false;
    }
    if (!(this.predecessorPtr?.id === other.predecessorPtr?.id)) {
      return false;
    }
    if (Object.keys(this._customValues).length !== Object.keys(other._customValues).length) {
      return false;
    }
    for (const key in this._customValues) {
      if (!(key in other._customValues)) {
        return false;
      }
      if (!this._customValues[key].equals(other._customValues[key])) {
        return false;
      }
    }
    if (!(this._scriptPtr?.id === other._scriptPtr?.id)) {
      return false;
    }
    if (!(this.spacePtr.id === other.spacePtr.id)) {
      return false;
    }
    return true;
  }

  hash(): number {
    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    if (this.parentPtr != null) {
      h = (h * 31 + hashString(this.parentPtr.id)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this._memberPtr.id)) & 0xffffffff;
    if (this._rolePtr != null) {
      h = (h * 31 + hashString(this._rolePtr.id)) & 0xffffffff;
    }
    if (this._roleType != null) {
      h = (h * 31 + this._roleType) & 0xffffffff;
    }
    if (this._ownedByPtr != null) {
      h = (h * 31 + hashString(this._ownedByPtr.id)) & 0xffffffff;
    }
    if (this.deletedAt != null) {
      h = (h * 31 + hashString(this.deletedAt.toString({ timeZoneName: "never" }))) & 0xffffffff;
    }
    if (this.definitionPtr != null) {
      h = (h * 31 + hashString(this.definitionPtr.id)) & 0xffffffff;
    }
    h = (h * 31 + hashBool(this.isExtensible)) & 0xffffffff;
    if (this.snapshotPtr != null) {
      h = (h * 31 + hashString(this.snapshotPtr.id)) & 0xffffffff;
    }
    if (this.predecessorPtr != null) {
      h = (h * 31 + hashString(this.predecessorPtr.id)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.createdAt.toString({ timeZoneName: "never" }))) & 0xffffffff;
    if (this.createdByPtr != null) {
      h = (h * 31 + hashString(this.createdByPtr.id)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.updatedAt.toString({ timeZoneName: "never" }))) & 0xffffffff;
    if (this.updatedByPtr != null) {
      h = (h * 31 + hashString(this.updatedByPtr.id)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.id.toString())) & 0xffffffff;
    if (this._customValues && Object.keys(this._customValues).length > 0) {
      for (const [_key, _value] of Object.entries(this._customValues)) {
        h = (h * 31 + hashString(_key.toString())) & 0xffffffff;
        h = (h * 31 + _value.hash()) & 0xffffffff;
      }
    }
    if (this._scriptPtr != null) {
      h = (h * 31 + hashString(this._scriptPtr.id)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.spacePtr.id)) & 0xffffffff;

    return h;
  }

  validate(): void {
    throw new Error("not implemented");
  }

  __toRef__(): NodeReference {
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    return new _NodeReference({
      type: NodeType.INVITE,
      id: this.id,
      spaceId: this.spacePtr?.id ?? null,
      snapshotId: this.snapshotPtr?.id ?? null,
      definitionId: this.definitionPtr?.id ?? null,
      _session: this._session,
      _supergraph: this._supergraph,
    });
  }

  get _pathKey(): string {
    return `Invite[id=${this.id}]`;
  }

  get path(): string {
    const pathParts: string[] = [];
    let node: Entity | Event | null = this;
    let lastNode: Entity | Event | null = this;
    while (node != null) {
      pathParts.push(node._pathKey);
      lastNode = node;
      node = node.parent;
    }
    if (!lastNode.isRoot) {
      pathParts.push("<detached>");
    }
    return pathParts.reverse().join("/");
  }

  repr(): string {
    const propertyReprs: string[] = [];
    if (this.ownedBy != null) {
      propertyReprs.push(`ownedBy=${this.ownedBy?.repr()}`);
    }
    if (propertyReprs.length > 0) {
      return `<Invite "${this.path}" ${propertyReprs.join(" ")}>`;
    } else {
      return `<Invite "${this.path}">`;
    }
  }

  toValue(): { readonly [key: string]: any } {
    return Invite.__packValue__(this);
  }

  static __packValue__(object: Invite): { readonly [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 60100;
    objectValue["2"] = String(object.id);
    if (object.parentPtr != null) {
      objectValue["3"] = object.parentPtr.toValue();
    }
    objectValue["5"] = object.spacePtr.toValue();
    if (object.definitionPtr != null) {
      objectValue["6"] = object.definitionPtr.toValue();
    }
    objectValue["10"] = object.materialization;
    if (object.snapshotPtr != null) {
      objectValue["11"] = object.snapshotPtr.toValue();
    }
    if (object.predecessorPtr != null) {
      objectValue["12"] = object.predecessorPtr.toValue();
    }
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
    if (Object.keys(object._customValues).length > 0) {
      const packedCustomValues: { [key: string]: any } = {} as any;
      for (const [key, value] of Object.entries(object._customValues)) {
        packedCustomValues[String(String(key))] = value.toValue();
      }
      objectValue["26"] = packedCustomValues;
    }
    if (object._ownedByPtr != null) {
      objectValue["28"] = object._ownedByPtr.toValue();
    }
    if (object._scriptPtr != null) {
      objectValue["80"] = object._scriptPtr.toValue();
    }
    objectValue["90"] = object.isExtensible;
    objectValue["110"] = object._memberPtr.toValue();
    if (object._rolePtr != null) {
      objectValue["111"] = object._rolePtr.toValue();
    }
    if (object._roleType != null) {
      objectValue["112"] = object._roleType;
    }
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { readonly [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Invite {
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    const _Value = STRUCT_CLASS_BY_TYPE[StructType.VALUE] as typeof Value;
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
    const definitionPtrValue = objectValue["6"];
    const unpackedDefinitionPtr =
      definitionPtrValue != undefined
        ? _NodeReference.fromValue(definitionPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const snapshotPtrValue = objectValue["11"];
    const unpackedSnapshotPtr =
      snapshotPtrValue != undefined
        ? _NodeReference.fromValue(snapshotPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const predecessorPtrValue = objectValue["12"];
    const unpackedPredecessorPtr =
      predecessorPtrValue != undefined
        ? _NodeReference.fromValue(predecessorPtrValue, _session, _supergraph, _graph, _connection)
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
    const unpackedCustomValues = {} as any;
    if (objectValue["26"] != undefined) {
      for (const [key, value] of Object.entries(objectValue["26"])) {
        unpackedCustomValues[String(key)] = _Value.fromValue(
          value as any,
          _session,
          _supergraph,
          _graph,
          _connection,
        );
      }
    }
    const scriptPtrValue = objectValue["80"];
    const unpackedScriptPtr =
      scriptPtrValue != undefined
        ? _NodeReference.fromValue(scriptPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    return new Invite({
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
      ownedBy: unpackedOwnedByPtr,
      deletedAt: unpackedDeletedAt,
      definition: unpackedDefinitionPtr,
      isExtensible: objectValue["90"],
      materialization: Number(objectValue["10"]),
      snapshot: unpackedSnapshotPtr,
      predecessor: unpackedPredecessorPtr,
      createdAt: Temporal.Instant.from(objectValue["20"]).toZonedDateTimeISO("UTC"),
      createdBy: unpackedCreatedByPtr,
      updatedAt: Temporal.Instant.from(objectValue["22"]).toZonedDateTimeISO("UTC"),
      updatedBy: unpackedUpdatedByPtr,
      id: String(objectValue["2"]),
      customValues: unpackedCustomValues,
      script: unpackedScriptPtr,
      space: _NodeReference.fromValue(objectValue["5"], _session, _supergraph, _graph, _connection),
      _session,
      _graph,
      _connection,
    });
  }

  static fromValue(
    objectValue: { readonly [key: string]: any },
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
    const objectProto: Partial<InviteProto> = { metatype: 60100 };
    objectProto.id = String(object.id);
    if (object.parentPtr != null) {
      objectProto.parentPtr = object.parentPtr.toProto();
    }
    objectProto.spacePtr = object.spacePtr.toProto();
    if (object.definitionPtr != null) {
      objectProto.definitionPtr = object.definitionPtr.toProto();
    }
    objectProto.materialization = Number(object.materialization) as MaterializationProto;
    if (object.snapshotPtr != null) {
      objectProto.snapshotPtr = object.snapshotPtr.toProto();
    }
    if (object.predecessorPtr != null) {
      objectProto.predecessorPtr = object.predecessorPtr.toProto();
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
    if (object._customValues) {
      objectProto.customValues = {} as any;
      for (const [key, value] of Object.entries(object._customValues)) {
        objectProto.customValues![String(key)] = value.toProto();
      }
    }
    if (object._ownedByPtr != null) {
      objectProto.ownedByPtr = object._ownedByPtr.toProto();
    }
    if (object._scriptPtr != null) {
      objectProto.scriptPtr = object._scriptPtr.toProto();
    }
    objectProto.isExtensible = object.isExtensible;
    objectProto.memberPtr = object._memberPtr.toProto();
    if (object._rolePtr != null) {
      objectProto.rolePtr = object._rolePtr.toProto();
    }
    if (object._roleType != null) {
      objectProto.roleType = Number(object._roleType) as RoleTypeProto;
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
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    const _Value = STRUCT_CLASS_BY_TYPE[StructType.VALUE] as typeof Value;
    const unpackedCustomValues = {} as any;
    if (objectProto.customValues) {
      for (const [key, value] of Object.entries(objectProto.customValues)) {
        unpackedCustomValues.set(
          String(key),
          _Value.fromProto((value as any)!, _session, _supergraph, _graph, _connection),
        );
      }
    }
    return new Invite({
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
      definition:
        objectProto.definitionPtr != undefined
          ? _NodeReference.fromProto(
              objectProto.definitionPtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      isExtensible: objectProto.isExtensible,
      materialization: Number(objectProto.materialization) as Materialization,
      snapshot:
        objectProto.snapshotPtr != undefined
          ? _NodeReference.fromProto(
              objectProto.snapshotPtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      predecessor:
        objectProto.predecessorPtr != undefined
          ? _NodeReference.fromProto(
              objectProto.predecessorPtr!,
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
      id: String(objectProto.id),
      customValues: unpackedCustomValues,
      script:
        objectProto.scriptPtr != undefined
          ? _NodeReference.fromProto(
              objectProto.scriptPtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      space: _NodeReference.fromProto(
        objectProto.spacePtr!,
        _session,
        _supergraph,
        _graph,
        _connection,
      ),
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
/* ==== DESTACK_GENERATED_END:NODE:60100 ==== */
