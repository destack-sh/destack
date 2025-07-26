import type {
  Branch,
  Datetime,
  NodeClass,
  NodeReference,
  Session,
  Snapshot,
  Space,
  UInt128,
  UUID,
  Value,
} from "@destack/language/core";
import {
  ACTIVE_BRANCH,
  ACTIVE_SNAPSHOT,
  ACTIVE_SPACE,
  Entity,
  EnumType,
  Event,
  EventStatus,
  type Materialization,
  type Node,
  NodeType,
  StructType,
} from "@destack/language/core";
import type { Script } from "@destack/language/logic";
import {
  registerEnumClass,
  registerNodeClass,
  STRUCT_CLASS_BY_TYPE,
} from "@destack/language/registry";
import type { Client } from "@destack/language/universe";
import { hashBool, hashInt, hashString } from "@destack/utils/hash";
import { Temporal } from "temporal-polyfill";

/* ==== DESTACK_GENERATED_START:ENUM:300400 ==== */
/**
 * SanctionType
 */
export enum SanctionType {
  BAN = 1,
  MUTE = 2,

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerEnumClass(EnumType.SANCTION_TYPE, SanctionType);
/* ==== DESTACK_GENERATED_END:ENUM:300400 ==== */

/* ==== DESTACK_GENERATED_START:NODE:360401 ==== */
/**
 * SanctionEvent
 */
export abstract class SanctionEvent extends Event {
  static metatype: NodeType = NodeType.SANCTION_EVENT;

  /**
   * The Space this Node is in.
   */
  abstract get space(): Space | null;
  declare readonly spacePtr: NodeReference;

  /**
   * The definition this Event is an instance of.
   */
  abstract get definition(): Entity | null;
  declare readonly definitionPtr: NodeReference | null;

  /**
   * The Branch this Event originated from.
   */
  abstract get branch(): Branch | null;
  declare readonly branchPtr: NodeReference;

  /**
   * The Snapshot this Event originated from.
   */
  abstract get snapshot(): Snapshot | null;
  declare readonly snapshotPtr: NodeReference;

  /**
   * The previous Event that this Event follows.
   */
  abstract get precededBy(): Event | null;
  declare readonly precededByPtr: NodeReference | null;

  /**
   * The Event that caused this Event (if any).
   */
  abstract get causedBy(): Event | null;
  declare readonly causedByPtr: NodeReference | null;

  /**
   * The time this Event was created (system).
   */
  declare readonly createdAt: Datetime;

  /**
   * The logical time this Event was created (system).
   */
  declare readonly createdEpoch: UInt128;

  /**
   * The Actor that created this Event.
   */
  abstract get createdBy(): Entity | null;
  declare readonly createdByPtr: NodeReference;

  /**
   * The Client that created this Event (client).
   */
  abstract get client(): Client | null;
  declare readonly clientPtr: NodeReference;

  /**
   * The nonce of the Client that created this Event (client).
   */
  declare readonly clientNonce: UUID;

  /**
   * The time in the Client when it created this Event (client).
   */
  declare readonly clientCreatedAt: Datetime;

  /**
   * The logical time in the Client when it created this Event (client).
   */
  declare readonly clientEpoch: UInt128;

  /**
   * The status of the Event (system).
   */
  declare readonly status: EventStatus;

  /**
   * SanctionEvent.node
   */
  abstract get node(): Sanction | null;
  declare readonly nodePtr: NodeReference;

  /**
   * SanctionEvent.target
   */
  abstract get target(): Entity | null;
  declare readonly targetPtr: NodeReference;

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerNodeClass(NodeType.SANCTION_EVENT, SanctionEvent);
/* ==== DESTACK_GENERATED_END:NODE:360401 ==== */

/* ==== DESTACK_GENERATED_START:NODE:360402 ==== */
/**
 * SanctionRequestedEvent
 */
export class SanctionRequestedEvent extends SanctionEvent {
  static metatype: NodeType = NodeType.SANCTION_REQUESTED_EVENT;

  /**
   * The Space this Node is in.
   */
  get space(): Space | null {
    const nodePtr: NodeReference | null = this.spacePtr;
    if (nodePtr != null) {
      return this._session.graph.get(nodePtr) as Space | null;
    }
    return null;
  }
  readonly spacePtr: NodeReference;

  /**
   * The definition this Event is an instance of.
   */
  get definition(): Entity | null {
    const nodePtr: NodeReference | null = this.definitionPtr;
    if (nodePtr != null) {
      return this._session.graph.get(nodePtr) as Entity | null;
    }
    return null;
  }
  readonly definitionPtr: NodeReference | null;

  /**
   * The Branch this Event originated from.
   */
  get branch(): Branch | null {
    const nodePtr: NodeReference | null = this.branchPtr;
    if (nodePtr != null) {
      return this._session.graph.get(nodePtr) as Branch | null;
    }
    return null;
  }
  readonly branchPtr: NodeReference;

  /**
   * The Snapshot this Event originated from.
   */
  get snapshot(): Snapshot | null {
    const nodePtr: NodeReference | null = this.snapshotPtr;
    if (nodePtr != null) {
      return this._session.graph.get(nodePtr) as Snapshot | null;
    }
    return null;
  }
  readonly snapshotPtr: NodeReference;

  /**
   * The previous Event that this Event follows.
   */
  get precededBy(): Event | null {
    const nodePtr: NodeReference | null = this.precededByPtr;
    if (nodePtr != null) {
      return this._session.graph.get(nodePtr) as Event | null;
    }
    return null;
  }
  readonly precededByPtr: NodeReference | null;

  /**
   * The Event that caused this Event (if any).
   */
  get causedBy(): Event | null {
    const nodePtr: NodeReference | null = this.causedByPtr;
    if (nodePtr != null) {
      return this._session.graph.get(nodePtr) as Event | null;
    }
    return null;
  }
  readonly causedByPtr: NodeReference | null;

  /**
   * The time this Event was created (system).
   */
  readonly createdAt: Datetime;

  /**
   * The logical time this Event was created (system).
   */
  readonly createdEpoch: UInt128;

  /**
   * The Actor that created this Event.
   */
  get createdBy(): Entity | null {
    const nodePtr: NodeReference | null = this.createdByPtr;
    if (nodePtr != null) {
      return this._session.graph.get(nodePtr) as Entity | null;
    }
    return null;
  }
  readonly createdByPtr: NodeReference;

  /**
   * The Client that created this Event (client).
   */
  get client(): Client | null {
    const nodePtr: NodeReference | null = this.clientPtr;
    if (nodePtr != null) {
      return this._session.graph.get(nodePtr) as Client | null;
    }
    return null;
  }
  readonly clientPtr: NodeReference;

  /**
   * The nonce of the Client that created this Event (client).
   */
  readonly clientNonce: UUID;

  /**
   * The time in the Client when it created this Event (client).
   */
  readonly clientCreatedAt: Datetime;

  /**
   * The logical time in the Client when it created this Event (client).
   */
  readonly clientEpoch: UInt128;

  /**
   * The status of the Event (system).
   */
  readonly status: EventStatus;

  /**
   * SanctionEvent.node
   */
  get node(): Sanction | null {
    const nodePtr: NodeReference | null = this.nodePtr;
    if (nodePtr != null) {
      return this._session.graph.get(nodePtr) as Sanction | null;
    }
    return null;
  }
  readonly nodePtr: NodeReference;

  /**
   * SanctionEvent.target
   */
  get target(): Entity | null {
    const nodePtr: NodeReference | null = this.targetPtr;
    if (nodePtr != null) {
      return this._session.graph.get(nodePtr) as Entity | null;
    }
    return null;
  }
  readonly targetPtr: NodeReference;

  constructor(options: {
    id?: UUID;
    space?: Space | NodeReference;
    definition?: Entity | NodeReference | null;
    branch?: Branch | NodeReference;
    snapshot?: Snapshot | NodeReference;
    precededBy?: Event | NodeReference | null;
    causedBy?: Event | NodeReference | null;
    createdAt?: Datetime;
    createdEpoch?: UInt128;
    createdBy?: Entity | NodeReference;
    client?: Client | NodeReference;
    clientNonce?: UUID;
    clientCreatedAt?: Datetime;
    clientEpoch?: UInt128;
    status?: EventStatus;
    node: Sanction | NodeReference;
    target: Entity | NodeReference;
    _session?: Session | null;
  }) {
    /* super */
    super(
      /* id */
      options.id ?? null,
      /* parent */
      null,
      /* session */
      options._session ?? null,
      /* _isNew */
      options.id == null,
    );

    /* properties */
    let _space = options.space ?? null;
    if (_space != null && _space.constructor.name !== "NodeReference") {
      _space = (_space as Node).toRef();
    }
    if (_space == null) {
      _space = ACTIVE_SPACE.get();
      if (_space == null) {
        throw new Error(`no active Space for SanctionRequestedEvent`);
      }
      _space = _space.toRef();
    }
    if (_space == null) {
      throw new Error(`SanctionRequestedEvent.space is required`);
    }
    this.spacePtr = _space as NodeReference;
    let _definition = options.definition ?? null;
    if (_definition != null && _definition.constructor.name !== "NodeReference") {
      _definition = (_definition as Node).toRef();
    }
    this.definitionPtr = _definition as NodeReference | null;
    let _branch = options.branch ?? null;
    if (_branch != null && _branch.constructor.name !== "NodeReference") {
      _branch = (_branch as Node).toRef();
    }
    if (_branch == null) {
      _branch = ACTIVE_BRANCH.get();
      if (_branch == null) {
        throw new Error(`no active Branch for SanctionRequestedEvent`);
      }
      _branch = _branch.toRef();
    }
    if (_branch == null) {
      throw new Error(`SanctionRequestedEvent.branch is required`);
    }
    this.branchPtr = _branch as NodeReference;
    let _snapshot = options.snapshot ?? null;
    if (_snapshot != null && _snapshot.constructor.name !== "NodeReference") {
      _snapshot = (_snapshot as Node).toRef();
    }
    if (_snapshot == null) {
      _snapshot = ACTIVE_SNAPSHOT.get();
      if (_snapshot == null) {
        throw new Error(`no active Snapshot for SanctionRequestedEvent`);
      }
      _snapshot = _snapshot.toRef();
    }
    if (_snapshot == null) {
      throw new Error(`SanctionRequestedEvent.snapshot is required`);
    }
    this.snapshotPtr = _snapshot as NodeReference;
    let _precededBy = options.precededBy ?? null;
    if (_precededBy != null && _precededBy.constructor.name !== "NodeReference") {
      _precededBy = (_precededBy as Node).toRef();
    }
    this.precededByPtr = _precededBy as NodeReference | null;
    let _causedBy = options.causedBy ?? null;
    if (_causedBy != null && _causedBy.constructor.name !== "NodeReference") {
      _causedBy = (_causedBy as Node).toRef();
    }
    this.causedByPtr = _causedBy as NodeReference | null;
    let _client = options.client ?? null;
    if (_client != null && _client.constructor.name !== "NodeReference") {
      _client = (_client as Node).toRef();
    }
    if (_client == null) {
      _client = this._session.clientPtr;
    }
    if (_client == null) {
      throw new Error(`SanctionRequestedEvent.client is required`);
    }
    this.clientPtr = _client as NodeReference;
    let _clientNonce = options.clientNonce ?? null;
    if (_clientNonce == null) {
      _clientNonce = this._session.clientNonce;
    }
    if (_clientNonce == null) {
      throw new Error(`SanctionRequestedEvent.clientNonce is required`);
    }
    this.clientNonce = _clientNonce;
    let _status = options.status ?? null;
    if (_status == null) {
      _status = 1 /* EventStatus.PENDING */;
    }
    if (_status == null) {
      throw new Error(`SanctionRequestedEvent.status is required`);
    }
    this.status = _status;
    let _node = options.node;
    if (_node != null && _node.constructor.name !== "NodeReference") {
      _node = (_node as Node).toRef();
    }
    if (_node == null) {
      throw new Error(`SanctionRequestedEvent.node is required`);
    }
    this.nodePtr = _node as NodeReference;
    let _target = options.target;
    if (_target != null && _target.constructor.name !== "NodeReference") {
      _target = (_target as Node).toRef();
    }
    if (_target == null) {
      throw new Error(`SanctionRequestedEvent.target is required`);
    }
    this.targetPtr = _target as NodeReference;

    /* identity */
    if (options.id == null) {
      const now = Temporal.Now.zonedDateTimeISO("UTC");
      const epoch = this._session.epoch;
      this.createdAt = now;
      this.createdEpoch = epoch;
      this.createdByPtr = this._session.actorPtr;
      this.clientCreatedAt = now;
      this.clientEpoch = epoch;
    } else {
      if (
        options.createdAt == null ||
        options.createdEpoch == null ||
        options.clientCreatedAt == null ||
        options.clientEpoch == null
      ) {
        throw new Error(`SanctionRequestedEvent.createdAt is required for existing Events`);
      }
      this.createdAt = options.createdAt;
      this.createdEpoch = options.createdEpoch;
      this.createdByPtr =
        options.createdBy != null ? options.createdBy.toRef() : this._session.actorPtr;
      this.clientCreatedAt = options.clientCreatedAt;
      this.clientEpoch = options.clientEpoch;
    }
  }

  equals(other: any): boolean {
    if (!(this.metatype === other.metatype)) {
      return false;
    }
    if (!(this.nodePtr.id === other.nodePtr.id)) {
      return false;
    }
    if (!(this.targetPtr.id === other.targetPtr.id)) {
      return false;
    }
    if (!(this.definitionPtr?.id === other.definitionPtr?.id)) {
      return false;
    }
    if (!(this.branchPtr.id === other.branchPtr.id)) {
      return false;
    }
    if (!(this.snapshotPtr.id === other.snapshotPtr.id)) {
      return false;
    }
    if (!(this.precededByPtr?.id === other.precededByPtr?.id)) {
      return false;
    }
    if (!(this.causedByPtr?.id === other.causedByPtr?.id)) {
      return false;
    }
    if (!(this.clientPtr.id === other.clientPtr.id)) {
      return false;
    }
    if (!(this.clientNonce === other.clientNonce)) {
      return false;
    }
    if (!(this.clientCreatedAt === other.clientCreatedAt)) {
      return false;
    }
    if (!(this.clientEpoch === other.clientEpoch)) {
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
    h = (h * 31 + hashString(this.targetPtr.id)) & 0xffffffff;
    if (this.definitionPtr != null) {
      h = (h * 31 + hashString(this.definitionPtr.id)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.branchPtr.id)) & 0xffffffff;
    h = (h * 31 + hashString(this.snapshotPtr.id)) & 0xffffffff;
    if (this.precededByPtr != null) {
      h = (h * 31 + hashString(this.precededByPtr.id)) & 0xffffffff;
    }
    if (this.causedByPtr != null) {
      h = (h * 31 + hashString(this.causedByPtr.id)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.createdAt.toString({ timeZoneName: "never" }))) & 0xffffffff;
    h = (h * 31 + hashString(this.createdByPtr.id)) & 0xffffffff;
    h = (h * 31 + hashString(this.clientPtr.id)) & 0xffffffff;
    h = (h * 31 + hashString(this.clientNonce.toString())) & 0xffffffff;
    h =
      (h * 31 + hashString(this.clientCreatedAt.toString({ timeZoneName: "never" }))) & 0xffffffff;
    h = (h * 31 + hashInt(this.clientEpoch)) & 0xffffffff;
    h = (h * 31 + this.status) & 0xffffffff;
    h = (h * 31 + hashString(this.id.toString())) & 0xffffffff;
    h = (h * 31 + hashString(this.spacePtr.id)) & 0xffffffff;

    return h;
  }

  __toRef__(): NodeReference {
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    return new _NodeReference({
      type: NodeType.SANCTION_REQUESTED_EVENT,
      id: this.id,
      spaceId: this.spacePtr.id,
      definitionId: this.definitionPtr?.id ?? null,
      branchId: this.branchPtr.id,
      snapshotId: this.snapshotPtr.id,
      _session: this._session,
    });
  }

  get _pathKey(): string {
    return `SanctionRequestedEvent[id=${this.id}]`;
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
    propertyReprs.push(`createdEpoch=${this.createdEpoch}`);
    propertyReprs.push(`status=${EventStatus[this.status]}`);
    return `<SanctionRequestedEvent "${this.path}" ${propertyReprs.join(" ")}>`;
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerNodeClass(NodeType.SANCTION_REQUESTED_EVENT, SanctionRequestedEvent);
/* ==== DESTACK_GENERATED_END:NODE:360402 ==== */

/* ==== DESTACK_GENERATED_START:NODE:360403 ==== */
/**
 * SanctionGrantedEvent
 */
export class SanctionGrantedEvent extends SanctionEvent {
  static metatype: NodeType = NodeType.SANCTION_GRANTED_EVENT;

  /**
   * The Space this Node is in.
   */
  get space(): Space | null {
    const nodePtr: NodeReference | null = this.spacePtr;
    if (nodePtr != null) {
      return this._session.graph.get(nodePtr) as Space | null;
    }
    return null;
  }
  readonly spacePtr: NodeReference;

  /**
   * The definition this Event is an instance of.
   */
  get definition(): Entity | null {
    const nodePtr: NodeReference | null = this.definitionPtr;
    if (nodePtr != null) {
      return this._session.graph.get(nodePtr) as Entity | null;
    }
    return null;
  }
  readonly definitionPtr: NodeReference | null;

  /**
   * The Branch this Event originated from.
   */
  get branch(): Branch | null {
    const nodePtr: NodeReference | null = this.branchPtr;
    if (nodePtr != null) {
      return this._session.graph.get(nodePtr) as Branch | null;
    }
    return null;
  }
  readonly branchPtr: NodeReference;

  /**
   * The Snapshot this Event originated from.
   */
  get snapshot(): Snapshot | null {
    const nodePtr: NodeReference | null = this.snapshotPtr;
    if (nodePtr != null) {
      return this._session.graph.get(nodePtr) as Snapshot | null;
    }
    return null;
  }
  readonly snapshotPtr: NodeReference;

  /**
   * The previous Event that this Event follows.
   */
  get precededBy(): Event | null {
    const nodePtr: NodeReference | null = this.precededByPtr;
    if (nodePtr != null) {
      return this._session.graph.get(nodePtr) as Event | null;
    }
    return null;
  }
  readonly precededByPtr: NodeReference | null;

  /**
   * The Event that caused this Event (if any).
   */
  get causedBy(): Event | null {
    const nodePtr: NodeReference | null = this.causedByPtr;
    if (nodePtr != null) {
      return this._session.graph.get(nodePtr) as Event | null;
    }
    return null;
  }
  readonly causedByPtr: NodeReference | null;

  /**
   * The time this Event was created (system).
   */
  readonly createdAt: Datetime;

  /**
   * The logical time this Event was created (system).
   */
  readonly createdEpoch: UInt128;

  /**
   * The Actor that created this Event.
   */
  get createdBy(): Entity | null {
    const nodePtr: NodeReference | null = this.createdByPtr;
    if (nodePtr != null) {
      return this._session.graph.get(nodePtr) as Entity | null;
    }
    return null;
  }
  readonly createdByPtr: NodeReference;

  /**
   * The Client that created this Event (client).
   */
  get client(): Client | null {
    const nodePtr: NodeReference | null = this.clientPtr;
    if (nodePtr != null) {
      return this._session.graph.get(nodePtr) as Client | null;
    }
    return null;
  }
  readonly clientPtr: NodeReference;

  /**
   * The nonce of the Client that created this Event (client).
   */
  readonly clientNonce: UUID;

  /**
   * The time in the Client when it created this Event (client).
   */
  readonly clientCreatedAt: Datetime;

  /**
   * The logical time in the Client when it created this Event (client).
   */
  readonly clientEpoch: UInt128;

  /**
   * The status of the Event (system).
   */
  readonly status: EventStatus;

  /**
   * SanctionEvent.node
   */
  get node(): Sanction | null {
    const nodePtr: NodeReference | null = this.nodePtr;
    if (nodePtr != null) {
      return this._session.graph.get(nodePtr) as Sanction | null;
    }
    return null;
  }
  readonly nodePtr: NodeReference;

  /**
   * SanctionEvent.target
   */
  get target(): Entity | null {
    const nodePtr: NodeReference | null = this.targetPtr;
    if (nodePtr != null) {
      return this._session.graph.get(nodePtr) as Entity | null;
    }
    return null;
  }
  readonly targetPtr: NodeReference;

  constructor(options: {
    id?: UUID;
    space?: Space | NodeReference;
    definition?: Entity | NodeReference | null;
    branch?: Branch | NodeReference;
    snapshot?: Snapshot | NodeReference;
    precededBy?: Event | NodeReference | null;
    causedBy?: Event | NodeReference | null;
    createdAt?: Datetime;
    createdEpoch?: UInt128;
    createdBy?: Entity | NodeReference;
    client?: Client | NodeReference;
    clientNonce?: UUID;
    clientCreatedAt?: Datetime;
    clientEpoch?: UInt128;
    status?: EventStatus;
    node: Sanction | NodeReference;
    target: Entity | NodeReference;
    _session?: Session | null;
  }) {
    /* super */
    super(
      /* id */
      options.id ?? null,
      /* parent */
      null,
      /* session */
      options._session ?? null,
      /* _isNew */
      options.id == null,
    );

    /* properties */
    let _space = options.space ?? null;
    if (_space != null && _space.constructor.name !== "NodeReference") {
      _space = (_space as Node).toRef();
    }
    if (_space == null) {
      _space = ACTIVE_SPACE.get();
      if (_space == null) {
        throw new Error(`no active Space for SanctionGrantedEvent`);
      }
      _space = _space.toRef();
    }
    if (_space == null) {
      throw new Error(`SanctionGrantedEvent.space is required`);
    }
    this.spacePtr = _space as NodeReference;
    let _definition = options.definition ?? null;
    if (_definition != null && _definition.constructor.name !== "NodeReference") {
      _definition = (_definition as Node).toRef();
    }
    this.definitionPtr = _definition as NodeReference | null;
    let _branch = options.branch ?? null;
    if (_branch != null && _branch.constructor.name !== "NodeReference") {
      _branch = (_branch as Node).toRef();
    }
    if (_branch == null) {
      _branch = ACTIVE_BRANCH.get();
      if (_branch == null) {
        throw new Error(`no active Branch for SanctionGrantedEvent`);
      }
      _branch = _branch.toRef();
    }
    if (_branch == null) {
      throw new Error(`SanctionGrantedEvent.branch is required`);
    }
    this.branchPtr = _branch as NodeReference;
    let _snapshot = options.snapshot ?? null;
    if (_snapshot != null && _snapshot.constructor.name !== "NodeReference") {
      _snapshot = (_snapshot as Node).toRef();
    }
    if (_snapshot == null) {
      _snapshot = ACTIVE_SNAPSHOT.get();
      if (_snapshot == null) {
        throw new Error(`no active Snapshot for SanctionGrantedEvent`);
      }
      _snapshot = _snapshot.toRef();
    }
    if (_snapshot == null) {
      throw new Error(`SanctionGrantedEvent.snapshot is required`);
    }
    this.snapshotPtr = _snapshot as NodeReference;
    let _precededBy = options.precededBy ?? null;
    if (_precededBy != null && _precededBy.constructor.name !== "NodeReference") {
      _precededBy = (_precededBy as Node).toRef();
    }
    this.precededByPtr = _precededBy as NodeReference | null;
    let _causedBy = options.causedBy ?? null;
    if (_causedBy != null && _causedBy.constructor.name !== "NodeReference") {
      _causedBy = (_causedBy as Node).toRef();
    }
    this.causedByPtr = _causedBy as NodeReference | null;
    let _client = options.client ?? null;
    if (_client != null && _client.constructor.name !== "NodeReference") {
      _client = (_client as Node).toRef();
    }
    if (_client == null) {
      _client = this._session.clientPtr;
    }
    if (_client == null) {
      throw new Error(`SanctionGrantedEvent.client is required`);
    }
    this.clientPtr = _client as NodeReference;
    let _clientNonce = options.clientNonce ?? null;
    if (_clientNonce == null) {
      _clientNonce = this._session.clientNonce;
    }
    if (_clientNonce == null) {
      throw new Error(`SanctionGrantedEvent.clientNonce is required`);
    }
    this.clientNonce = _clientNonce;
    let _status = options.status ?? null;
    if (_status == null) {
      _status = 1 /* EventStatus.PENDING */;
    }
    if (_status == null) {
      throw new Error(`SanctionGrantedEvent.status is required`);
    }
    this.status = _status;
    let _node = options.node;
    if (_node != null && _node.constructor.name !== "NodeReference") {
      _node = (_node as Node).toRef();
    }
    if (_node == null) {
      throw new Error(`SanctionGrantedEvent.node is required`);
    }
    this.nodePtr = _node as NodeReference;
    let _target = options.target;
    if (_target != null && _target.constructor.name !== "NodeReference") {
      _target = (_target as Node).toRef();
    }
    if (_target == null) {
      throw new Error(`SanctionGrantedEvent.target is required`);
    }
    this.targetPtr = _target as NodeReference;

    /* identity */
    if (options.id == null) {
      const now = Temporal.Now.zonedDateTimeISO("UTC");
      const epoch = this._session.epoch;
      this.createdAt = now;
      this.createdEpoch = epoch;
      this.createdByPtr = this._session.actorPtr;
      this.clientCreatedAt = now;
      this.clientEpoch = epoch;
    } else {
      if (
        options.createdAt == null ||
        options.createdEpoch == null ||
        options.clientCreatedAt == null ||
        options.clientEpoch == null
      ) {
        throw new Error(`SanctionGrantedEvent.createdAt is required for existing Events`);
      }
      this.createdAt = options.createdAt;
      this.createdEpoch = options.createdEpoch;
      this.createdByPtr =
        options.createdBy != null ? options.createdBy.toRef() : this._session.actorPtr;
      this.clientCreatedAt = options.clientCreatedAt;
      this.clientEpoch = options.clientEpoch;
    }
  }

  equals(other: any): boolean {
    if (!(this.metatype === other.metatype)) {
      return false;
    }
    if (!(this.nodePtr.id === other.nodePtr.id)) {
      return false;
    }
    if (!(this.targetPtr.id === other.targetPtr.id)) {
      return false;
    }
    if (!(this.definitionPtr?.id === other.definitionPtr?.id)) {
      return false;
    }
    if (!(this.branchPtr.id === other.branchPtr.id)) {
      return false;
    }
    if (!(this.snapshotPtr.id === other.snapshotPtr.id)) {
      return false;
    }
    if (!(this.precededByPtr?.id === other.precededByPtr?.id)) {
      return false;
    }
    if (!(this.causedByPtr?.id === other.causedByPtr?.id)) {
      return false;
    }
    if (!(this.clientPtr.id === other.clientPtr.id)) {
      return false;
    }
    if (!(this.clientNonce === other.clientNonce)) {
      return false;
    }
    if (!(this.clientCreatedAt === other.clientCreatedAt)) {
      return false;
    }
    if (!(this.clientEpoch === other.clientEpoch)) {
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
    h = (h * 31 + hashString(this.targetPtr.id)) & 0xffffffff;
    if (this.definitionPtr != null) {
      h = (h * 31 + hashString(this.definitionPtr.id)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.branchPtr.id)) & 0xffffffff;
    h = (h * 31 + hashString(this.snapshotPtr.id)) & 0xffffffff;
    if (this.precededByPtr != null) {
      h = (h * 31 + hashString(this.precededByPtr.id)) & 0xffffffff;
    }
    if (this.causedByPtr != null) {
      h = (h * 31 + hashString(this.causedByPtr.id)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.createdAt.toString({ timeZoneName: "never" }))) & 0xffffffff;
    h = (h * 31 + hashString(this.createdByPtr.id)) & 0xffffffff;
    h = (h * 31 + hashString(this.clientPtr.id)) & 0xffffffff;
    h = (h * 31 + hashString(this.clientNonce.toString())) & 0xffffffff;
    h =
      (h * 31 + hashString(this.clientCreatedAt.toString({ timeZoneName: "never" }))) & 0xffffffff;
    h = (h * 31 + hashInt(this.clientEpoch)) & 0xffffffff;
    h = (h * 31 + this.status) & 0xffffffff;
    h = (h * 31 + hashString(this.id.toString())) & 0xffffffff;
    h = (h * 31 + hashString(this.spacePtr.id)) & 0xffffffff;

    return h;
  }

  __toRef__(): NodeReference {
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    return new _NodeReference({
      type: NodeType.SANCTION_GRANTED_EVENT,
      id: this.id,
      spaceId: this.spacePtr.id,
      definitionId: this.definitionPtr?.id ?? null,
      branchId: this.branchPtr.id,
      snapshotId: this.snapshotPtr.id,
      _session: this._session,
    });
  }

  get _pathKey(): string {
    return `SanctionGrantedEvent[id=${this.id}]`;
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
    propertyReprs.push(`createdEpoch=${this.createdEpoch}`);
    propertyReprs.push(`status=${EventStatus[this.status]}`);
    return `<SanctionGrantedEvent "${this.path}" ${propertyReprs.join(" ")}>`;
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerNodeClass(NodeType.SANCTION_GRANTED_EVENT, SanctionGrantedEvent);
/* ==== DESTACK_GENERATED_END:NODE:360403 ==== */

/* ==== DESTACK_GENERATED_START:NODE:360404 ==== */
/**
 * SanctionRevokedEvent
 */
export class SanctionRevokedEvent extends SanctionEvent {
  static metatype: NodeType = NodeType.SANCTION_REVOKED_EVENT;

  /**
   * The Space this Node is in.
   */
  get space(): Space | null {
    const nodePtr: NodeReference | null = this.spacePtr;
    if (nodePtr != null) {
      return this._session.graph.get(nodePtr) as Space | null;
    }
    return null;
  }
  readonly spacePtr: NodeReference;

  /**
   * The definition this Event is an instance of.
   */
  get definition(): Entity | null {
    const nodePtr: NodeReference | null = this.definitionPtr;
    if (nodePtr != null) {
      return this._session.graph.get(nodePtr) as Entity | null;
    }
    return null;
  }
  readonly definitionPtr: NodeReference | null;

  /**
   * The Branch this Event originated from.
   */
  get branch(): Branch | null {
    const nodePtr: NodeReference | null = this.branchPtr;
    if (nodePtr != null) {
      return this._session.graph.get(nodePtr) as Branch | null;
    }
    return null;
  }
  readonly branchPtr: NodeReference;

  /**
   * The Snapshot this Event originated from.
   */
  get snapshot(): Snapshot | null {
    const nodePtr: NodeReference | null = this.snapshotPtr;
    if (nodePtr != null) {
      return this._session.graph.get(nodePtr) as Snapshot | null;
    }
    return null;
  }
  readonly snapshotPtr: NodeReference;

  /**
   * The previous Event that this Event follows.
   */
  get precededBy(): Event | null {
    const nodePtr: NodeReference | null = this.precededByPtr;
    if (nodePtr != null) {
      return this._session.graph.get(nodePtr) as Event | null;
    }
    return null;
  }
  readonly precededByPtr: NodeReference | null;

  /**
   * The Event that caused this Event (if any).
   */
  get causedBy(): Event | null {
    const nodePtr: NodeReference | null = this.causedByPtr;
    if (nodePtr != null) {
      return this._session.graph.get(nodePtr) as Event | null;
    }
    return null;
  }
  readonly causedByPtr: NodeReference | null;

  /**
   * The time this Event was created (system).
   */
  readonly createdAt: Datetime;

  /**
   * The logical time this Event was created (system).
   */
  readonly createdEpoch: UInt128;

  /**
   * The Actor that created this Event.
   */
  get createdBy(): Entity | null {
    const nodePtr: NodeReference | null = this.createdByPtr;
    if (nodePtr != null) {
      return this._session.graph.get(nodePtr) as Entity | null;
    }
    return null;
  }
  readonly createdByPtr: NodeReference;

  /**
   * The Client that created this Event (client).
   */
  get client(): Client | null {
    const nodePtr: NodeReference | null = this.clientPtr;
    if (nodePtr != null) {
      return this._session.graph.get(nodePtr) as Client | null;
    }
    return null;
  }
  readonly clientPtr: NodeReference;

  /**
   * The nonce of the Client that created this Event (client).
   */
  readonly clientNonce: UUID;

  /**
   * The time in the Client when it created this Event (client).
   */
  readonly clientCreatedAt: Datetime;

  /**
   * The logical time in the Client when it created this Event (client).
   */
  readonly clientEpoch: UInt128;

  /**
   * The status of the Event (system).
   */
  readonly status: EventStatus;

  /**
   * SanctionEvent.node
   */
  get node(): Sanction | null {
    const nodePtr: NodeReference | null = this.nodePtr;
    if (nodePtr != null) {
      return this._session.graph.get(nodePtr) as Sanction | null;
    }
    return null;
  }
  readonly nodePtr: NodeReference;

  /**
   * SanctionEvent.target
   */
  get target(): Entity | null {
    const nodePtr: NodeReference | null = this.targetPtr;
    if (nodePtr != null) {
      return this._session.graph.get(nodePtr) as Entity | null;
    }
    return null;
  }
  readonly targetPtr: NodeReference;

  constructor(options: {
    id?: UUID;
    space?: Space | NodeReference;
    definition?: Entity | NodeReference | null;
    branch?: Branch | NodeReference;
    snapshot?: Snapshot | NodeReference;
    precededBy?: Event | NodeReference | null;
    causedBy?: Event | NodeReference | null;
    createdAt?: Datetime;
    createdEpoch?: UInt128;
    createdBy?: Entity | NodeReference;
    client?: Client | NodeReference;
    clientNonce?: UUID;
    clientCreatedAt?: Datetime;
    clientEpoch?: UInt128;
    status?: EventStatus;
    node: Sanction | NodeReference;
    target: Entity | NodeReference;
    _session?: Session | null;
  }) {
    /* super */
    super(
      /* id */
      options.id ?? null,
      /* parent */
      null,
      /* session */
      options._session ?? null,
      /* _isNew */
      options.id == null,
    );

    /* properties */
    let _space = options.space ?? null;
    if (_space != null && _space.constructor.name !== "NodeReference") {
      _space = (_space as Node).toRef();
    }
    if (_space == null) {
      _space = ACTIVE_SPACE.get();
      if (_space == null) {
        throw new Error(`no active Space for SanctionRevokedEvent`);
      }
      _space = _space.toRef();
    }
    if (_space == null) {
      throw new Error(`SanctionRevokedEvent.space is required`);
    }
    this.spacePtr = _space as NodeReference;
    let _definition = options.definition ?? null;
    if (_definition != null && _definition.constructor.name !== "NodeReference") {
      _definition = (_definition as Node).toRef();
    }
    this.definitionPtr = _definition as NodeReference | null;
    let _branch = options.branch ?? null;
    if (_branch != null && _branch.constructor.name !== "NodeReference") {
      _branch = (_branch as Node).toRef();
    }
    if (_branch == null) {
      _branch = ACTIVE_BRANCH.get();
      if (_branch == null) {
        throw new Error(`no active Branch for SanctionRevokedEvent`);
      }
      _branch = _branch.toRef();
    }
    if (_branch == null) {
      throw new Error(`SanctionRevokedEvent.branch is required`);
    }
    this.branchPtr = _branch as NodeReference;
    let _snapshot = options.snapshot ?? null;
    if (_snapshot != null && _snapshot.constructor.name !== "NodeReference") {
      _snapshot = (_snapshot as Node).toRef();
    }
    if (_snapshot == null) {
      _snapshot = ACTIVE_SNAPSHOT.get();
      if (_snapshot == null) {
        throw new Error(`no active Snapshot for SanctionRevokedEvent`);
      }
      _snapshot = _snapshot.toRef();
    }
    if (_snapshot == null) {
      throw new Error(`SanctionRevokedEvent.snapshot is required`);
    }
    this.snapshotPtr = _snapshot as NodeReference;
    let _precededBy = options.precededBy ?? null;
    if (_precededBy != null && _precededBy.constructor.name !== "NodeReference") {
      _precededBy = (_precededBy as Node).toRef();
    }
    this.precededByPtr = _precededBy as NodeReference | null;
    let _causedBy = options.causedBy ?? null;
    if (_causedBy != null && _causedBy.constructor.name !== "NodeReference") {
      _causedBy = (_causedBy as Node).toRef();
    }
    this.causedByPtr = _causedBy as NodeReference | null;
    let _client = options.client ?? null;
    if (_client != null && _client.constructor.name !== "NodeReference") {
      _client = (_client as Node).toRef();
    }
    if (_client == null) {
      _client = this._session.clientPtr;
    }
    if (_client == null) {
      throw new Error(`SanctionRevokedEvent.client is required`);
    }
    this.clientPtr = _client as NodeReference;
    let _clientNonce = options.clientNonce ?? null;
    if (_clientNonce == null) {
      _clientNonce = this._session.clientNonce;
    }
    if (_clientNonce == null) {
      throw new Error(`SanctionRevokedEvent.clientNonce is required`);
    }
    this.clientNonce = _clientNonce;
    let _status = options.status ?? null;
    if (_status == null) {
      _status = 1 /* EventStatus.PENDING */;
    }
    if (_status == null) {
      throw new Error(`SanctionRevokedEvent.status is required`);
    }
    this.status = _status;
    let _node = options.node;
    if (_node != null && _node.constructor.name !== "NodeReference") {
      _node = (_node as Node).toRef();
    }
    if (_node == null) {
      throw new Error(`SanctionRevokedEvent.node is required`);
    }
    this.nodePtr = _node as NodeReference;
    let _target = options.target;
    if (_target != null && _target.constructor.name !== "NodeReference") {
      _target = (_target as Node).toRef();
    }
    if (_target == null) {
      throw new Error(`SanctionRevokedEvent.target is required`);
    }
    this.targetPtr = _target as NodeReference;

    /* identity */
    if (options.id == null) {
      const now = Temporal.Now.zonedDateTimeISO("UTC");
      const epoch = this._session.epoch;
      this.createdAt = now;
      this.createdEpoch = epoch;
      this.createdByPtr = this._session.actorPtr;
      this.clientCreatedAt = now;
      this.clientEpoch = epoch;
    } else {
      if (
        options.createdAt == null ||
        options.createdEpoch == null ||
        options.clientCreatedAt == null ||
        options.clientEpoch == null
      ) {
        throw new Error(`SanctionRevokedEvent.createdAt is required for existing Events`);
      }
      this.createdAt = options.createdAt;
      this.createdEpoch = options.createdEpoch;
      this.createdByPtr =
        options.createdBy != null ? options.createdBy.toRef() : this._session.actorPtr;
      this.clientCreatedAt = options.clientCreatedAt;
      this.clientEpoch = options.clientEpoch;
    }
  }

  equals(other: any): boolean {
    if (!(this.metatype === other.metatype)) {
      return false;
    }
    if (!(this.nodePtr.id === other.nodePtr.id)) {
      return false;
    }
    if (!(this.targetPtr.id === other.targetPtr.id)) {
      return false;
    }
    if (!(this.definitionPtr?.id === other.definitionPtr?.id)) {
      return false;
    }
    if (!(this.branchPtr.id === other.branchPtr.id)) {
      return false;
    }
    if (!(this.snapshotPtr.id === other.snapshotPtr.id)) {
      return false;
    }
    if (!(this.precededByPtr?.id === other.precededByPtr?.id)) {
      return false;
    }
    if (!(this.causedByPtr?.id === other.causedByPtr?.id)) {
      return false;
    }
    if (!(this.clientPtr.id === other.clientPtr.id)) {
      return false;
    }
    if (!(this.clientNonce === other.clientNonce)) {
      return false;
    }
    if (!(this.clientCreatedAt === other.clientCreatedAt)) {
      return false;
    }
    if (!(this.clientEpoch === other.clientEpoch)) {
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
    h = (h * 31 + hashString(this.targetPtr.id)) & 0xffffffff;
    if (this.definitionPtr != null) {
      h = (h * 31 + hashString(this.definitionPtr.id)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.branchPtr.id)) & 0xffffffff;
    h = (h * 31 + hashString(this.snapshotPtr.id)) & 0xffffffff;
    if (this.precededByPtr != null) {
      h = (h * 31 + hashString(this.precededByPtr.id)) & 0xffffffff;
    }
    if (this.causedByPtr != null) {
      h = (h * 31 + hashString(this.causedByPtr.id)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.createdAt.toString({ timeZoneName: "never" }))) & 0xffffffff;
    h = (h * 31 + hashString(this.createdByPtr.id)) & 0xffffffff;
    h = (h * 31 + hashString(this.clientPtr.id)) & 0xffffffff;
    h = (h * 31 + hashString(this.clientNonce.toString())) & 0xffffffff;
    h =
      (h * 31 + hashString(this.clientCreatedAt.toString({ timeZoneName: "never" }))) & 0xffffffff;
    h = (h * 31 + hashInt(this.clientEpoch)) & 0xffffffff;
    h = (h * 31 + this.status) & 0xffffffff;
    h = (h * 31 + hashString(this.id.toString())) & 0xffffffff;
    h = (h * 31 + hashString(this.spacePtr.id)) & 0xffffffff;

    return h;
  }

  __toRef__(): NodeReference {
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    return new _NodeReference({
      type: NodeType.SANCTION_REVOKED_EVENT,
      id: this.id,
      spaceId: this.spacePtr.id,
      definitionId: this.definitionPtr?.id ?? null,
      branchId: this.branchPtr.id,
      snapshotId: this.snapshotPtr.id,
      _session: this._session,
    });
  }

  get _pathKey(): string {
    return `SanctionRevokedEvent[id=${this.id}]`;
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
    propertyReprs.push(`createdEpoch=${this.createdEpoch}`);
    propertyReprs.push(`status=${EventStatus[this.status]}`);
    return `<SanctionRevokedEvent "${this.path}" ${propertyReprs.join(" ")}>`;
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerNodeClass(NodeType.SANCTION_REVOKED_EVENT, SanctionRevokedEvent);
/* ==== DESTACK_GENERATED_END:NODE:360404 ==== */

/* ==== DESTACK_GENERATED_START:NODE:360405 ==== */
/**
 * SanctionExpiredEvent
 */
export class SanctionExpiredEvent extends SanctionEvent {
  static metatype: NodeType = NodeType.SANCTION_EXPIRED_EVENT;

  /**
   * The Space this Node is in.
   */
  get space(): Space | null {
    const nodePtr: NodeReference | null = this.spacePtr;
    if (nodePtr != null) {
      return this._session.graph.get(nodePtr) as Space | null;
    }
    return null;
  }
  readonly spacePtr: NodeReference;

  /**
   * The definition this Event is an instance of.
   */
  get definition(): Entity | null {
    const nodePtr: NodeReference | null = this.definitionPtr;
    if (nodePtr != null) {
      return this._session.graph.get(nodePtr) as Entity | null;
    }
    return null;
  }
  readonly definitionPtr: NodeReference | null;

  /**
   * The Branch this Event originated from.
   */
  get branch(): Branch | null {
    const nodePtr: NodeReference | null = this.branchPtr;
    if (nodePtr != null) {
      return this._session.graph.get(nodePtr) as Branch | null;
    }
    return null;
  }
  readonly branchPtr: NodeReference;

  /**
   * The Snapshot this Event originated from.
   */
  get snapshot(): Snapshot | null {
    const nodePtr: NodeReference | null = this.snapshotPtr;
    if (nodePtr != null) {
      return this._session.graph.get(nodePtr) as Snapshot | null;
    }
    return null;
  }
  readonly snapshotPtr: NodeReference;

  /**
   * The previous Event that this Event follows.
   */
  get precededBy(): Event | null {
    const nodePtr: NodeReference | null = this.precededByPtr;
    if (nodePtr != null) {
      return this._session.graph.get(nodePtr) as Event | null;
    }
    return null;
  }
  readonly precededByPtr: NodeReference | null;

  /**
   * The Event that caused this Event (if any).
   */
  get causedBy(): Event | null {
    const nodePtr: NodeReference | null = this.causedByPtr;
    if (nodePtr != null) {
      return this._session.graph.get(nodePtr) as Event | null;
    }
    return null;
  }
  readonly causedByPtr: NodeReference | null;

  /**
   * The time this Event was created (system).
   */
  readonly createdAt: Datetime;

  /**
   * The logical time this Event was created (system).
   */
  readonly createdEpoch: UInt128;

  /**
   * The Actor that created this Event.
   */
  get createdBy(): Entity | null {
    const nodePtr: NodeReference | null = this.createdByPtr;
    if (nodePtr != null) {
      return this._session.graph.get(nodePtr) as Entity | null;
    }
    return null;
  }
  readonly createdByPtr: NodeReference;

  /**
   * The Client that created this Event (client).
   */
  get client(): Client | null {
    const nodePtr: NodeReference | null = this.clientPtr;
    if (nodePtr != null) {
      return this._session.graph.get(nodePtr) as Client | null;
    }
    return null;
  }
  readonly clientPtr: NodeReference;

  /**
   * The nonce of the Client that created this Event (client).
   */
  readonly clientNonce: UUID;

  /**
   * The time in the Client when it created this Event (client).
   */
  readonly clientCreatedAt: Datetime;

  /**
   * The logical time in the Client when it created this Event (client).
   */
  readonly clientEpoch: UInt128;

  /**
   * The status of the Event (system).
   */
  readonly status: EventStatus;

  /**
   * SanctionEvent.node
   */
  get node(): Sanction | null {
    const nodePtr: NodeReference | null = this.nodePtr;
    if (nodePtr != null) {
      return this._session.graph.get(nodePtr) as Sanction | null;
    }
    return null;
  }
  readonly nodePtr: NodeReference;

  /**
   * SanctionEvent.target
   */
  get target(): Entity | null {
    const nodePtr: NodeReference | null = this.targetPtr;
    if (nodePtr != null) {
      return this._session.graph.get(nodePtr) as Entity | null;
    }
    return null;
  }
  readonly targetPtr: NodeReference;

  constructor(options: {
    id?: UUID;
    space?: Space | NodeReference;
    definition?: Entity | NodeReference | null;
    branch?: Branch | NodeReference;
    snapshot?: Snapshot | NodeReference;
    precededBy?: Event | NodeReference | null;
    causedBy?: Event | NodeReference | null;
    createdAt?: Datetime;
    createdEpoch?: UInt128;
    createdBy?: Entity | NodeReference;
    client?: Client | NodeReference;
    clientNonce?: UUID;
    clientCreatedAt?: Datetime;
    clientEpoch?: UInt128;
    status?: EventStatus;
    node: Sanction | NodeReference;
    target: Entity | NodeReference;
    _session?: Session | null;
  }) {
    /* super */
    super(
      /* id */
      options.id ?? null,
      /* parent */
      null,
      /* session */
      options._session ?? null,
      /* _isNew */
      options.id == null,
    );

    /* properties */
    let _space = options.space ?? null;
    if (_space != null && _space.constructor.name !== "NodeReference") {
      _space = (_space as Node).toRef();
    }
    if (_space == null) {
      _space = ACTIVE_SPACE.get();
      if (_space == null) {
        throw new Error(`no active Space for SanctionExpiredEvent`);
      }
      _space = _space.toRef();
    }
    if (_space == null) {
      throw new Error(`SanctionExpiredEvent.space is required`);
    }
    this.spacePtr = _space as NodeReference;
    let _definition = options.definition ?? null;
    if (_definition != null && _definition.constructor.name !== "NodeReference") {
      _definition = (_definition as Node).toRef();
    }
    this.definitionPtr = _definition as NodeReference | null;
    let _branch = options.branch ?? null;
    if (_branch != null && _branch.constructor.name !== "NodeReference") {
      _branch = (_branch as Node).toRef();
    }
    if (_branch == null) {
      _branch = ACTIVE_BRANCH.get();
      if (_branch == null) {
        throw new Error(`no active Branch for SanctionExpiredEvent`);
      }
      _branch = _branch.toRef();
    }
    if (_branch == null) {
      throw new Error(`SanctionExpiredEvent.branch is required`);
    }
    this.branchPtr = _branch as NodeReference;
    let _snapshot = options.snapshot ?? null;
    if (_snapshot != null && _snapshot.constructor.name !== "NodeReference") {
      _snapshot = (_snapshot as Node).toRef();
    }
    if (_snapshot == null) {
      _snapshot = ACTIVE_SNAPSHOT.get();
      if (_snapshot == null) {
        throw new Error(`no active Snapshot for SanctionExpiredEvent`);
      }
      _snapshot = _snapshot.toRef();
    }
    if (_snapshot == null) {
      throw new Error(`SanctionExpiredEvent.snapshot is required`);
    }
    this.snapshotPtr = _snapshot as NodeReference;
    let _precededBy = options.precededBy ?? null;
    if (_precededBy != null && _precededBy.constructor.name !== "NodeReference") {
      _precededBy = (_precededBy as Node).toRef();
    }
    this.precededByPtr = _precededBy as NodeReference | null;
    let _causedBy = options.causedBy ?? null;
    if (_causedBy != null && _causedBy.constructor.name !== "NodeReference") {
      _causedBy = (_causedBy as Node).toRef();
    }
    this.causedByPtr = _causedBy as NodeReference | null;
    let _client = options.client ?? null;
    if (_client != null && _client.constructor.name !== "NodeReference") {
      _client = (_client as Node).toRef();
    }
    if (_client == null) {
      _client = this._session.clientPtr;
    }
    if (_client == null) {
      throw new Error(`SanctionExpiredEvent.client is required`);
    }
    this.clientPtr = _client as NodeReference;
    let _clientNonce = options.clientNonce ?? null;
    if (_clientNonce == null) {
      _clientNonce = this._session.clientNonce;
    }
    if (_clientNonce == null) {
      throw new Error(`SanctionExpiredEvent.clientNonce is required`);
    }
    this.clientNonce = _clientNonce;
    let _status = options.status ?? null;
    if (_status == null) {
      _status = 1 /* EventStatus.PENDING */;
    }
    if (_status == null) {
      throw new Error(`SanctionExpiredEvent.status is required`);
    }
    this.status = _status;
    let _node = options.node;
    if (_node != null && _node.constructor.name !== "NodeReference") {
      _node = (_node as Node).toRef();
    }
    if (_node == null) {
      throw new Error(`SanctionExpiredEvent.node is required`);
    }
    this.nodePtr = _node as NodeReference;
    let _target = options.target;
    if (_target != null && _target.constructor.name !== "NodeReference") {
      _target = (_target as Node).toRef();
    }
    if (_target == null) {
      throw new Error(`SanctionExpiredEvent.target is required`);
    }
    this.targetPtr = _target as NodeReference;

    /* identity */
    if (options.id == null) {
      const now = Temporal.Now.zonedDateTimeISO("UTC");
      const epoch = this._session.epoch;
      this.createdAt = now;
      this.createdEpoch = epoch;
      this.createdByPtr = this._session.actorPtr;
      this.clientCreatedAt = now;
      this.clientEpoch = epoch;
    } else {
      if (
        options.createdAt == null ||
        options.createdEpoch == null ||
        options.clientCreatedAt == null ||
        options.clientEpoch == null
      ) {
        throw new Error(`SanctionExpiredEvent.createdAt is required for existing Events`);
      }
      this.createdAt = options.createdAt;
      this.createdEpoch = options.createdEpoch;
      this.createdByPtr =
        options.createdBy != null ? options.createdBy.toRef() : this._session.actorPtr;
      this.clientCreatedAt = options.clientCreatedAt;
      this.clientEpoch = options.clientEpoch;
    }
  }

  equals(other: any): boolean {
    if (!(this.metatype === other.metatype)) {
      return false;
    }
    if (!(this.nodePtr.id === other.nodePtr.id)) {
      return false;
    }
    if (!(this.targetPtr.id === other.targetPtr.id)) {
      return false;
    }
    if (!(this.definitionPtr?.id === other.definitionPtr?.id)) {
      return false;
    }
    if (!(this.branchPtr.id === other.branchPtr.id)) {
      return false;
    }
    if (!(this.snapshotPtr.id === other.snapshotPtr.id)) {
      return false;
    }
    if (!(this.precededByPtr?.id === other.precededByPtr?.id)) {
      return false;
    }
    if (!(this.causedByPtr?.id === other.causedByPtr?.id)) {
      return false;
    }
    if (!(this.clientPtr.id === other.clientPtr.id)) {
      return false;
    }
    if (!(this.clientNonce === other.clientNonce)) {
      return false;
    }
    if (!(this.clientCreatedAt === other.clientCreatedAt)) {
      return false;
    }
    if (!(this.clientEpoch === other.clientEpoch)) {
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
    h = (h * 31 + hashString(this.targetPtr.id)) & 0xffffffff;
    if (this.definitionPtr != null) {
      h = (h * 31 + hashString(this.definitionPtr.id)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.branchPtr.id)) & 0xffffffff;
    h = (h * 31 + hashString(this.snapshotPtr.id)) & 0xffffffff;
    if (this.precededByPtr != null) {
      h = (h * 31 + hashString(this.precededByPtr.id)) & 0xffffffff;
    }
    if (this.causedByPtr != null) {
      h = (h * 31 + hashString(this.causedByPtr.id)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.createdAt.toString({ timeZoneName: "never" }))) & 0xffffffff;
    h = (h * 31 + hashString(this.createdByPtr.id)) & 0xffffffff;
    h = (h * 31 + hashString(this.clientPtr.id)) & 0xffffffff;
    h = (h * 31 + hashString(this.clientNonce.toString())) & 0xffffffff;
    h =
      (h * 31 + hashString(this.clientCreatedAt.toString({ timeZoneName: "never" }))) & 0xffffffff;
    h = (h * 31 + hashInt(this.clientEpoch)) & 0xffffffff;
    h = (h * 31 + this.status) & 0xffffffff;
    h = (h * 31 + hashString(this.id.toString())) & 0xffffffff;
    h = (h * 31 + hashString(this.spacePtr.id)) & 0xffffffff;

    return h;
  }

  __toRef__(): NodeReference {
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    return new _NodeReference({
      type: NodeType.SANCTION_EXPIRED_EVENT,
      id: this.id,
      spaceId: this.spacePtr.id,
      definitionId: this.definitionPtr?.id ?? null,
      branchId: this.branchPtr.id,
      snapshotId: this.snapshotPtr.id,
      _session: this._session,
    });
  }

  get _pathKey(): string {
    return `SanctionExpiredEvent[id=${this.id}]`;
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
    propertyReprs.push(`createdEpoch=${this.createdEpoch}`);
    propertyReprs.push(`status=${EventStatus[this.status]}`);
    return `<SanctionExpiredEvent "${this.path}" ${propertyReprs.join(" ")}>`;
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerNodeClass(NodeType.SANCTION_EXPIRED_EVENT, SanctionExpiredEvent);
/* ==== DESTACK_GENERATED_END:NODE:360405 ==== */

/* ==== DESTACK_GENERATED_START:NODE:360400 ==== */
/**
 * A Sanction on some Actor.
 */
export class Sanction extends Entity {
  static metatype: NodeType = NodeType.SANCTION;

  /**
   * The parent of this Entity. Most Entities can be attached to any other Entity.
   */
  get parent(): Entity | null {
    const nodePtr: NodeReference | null = this.parentPtr;
    if (nodePtr != null) {
      return this._session.graph.get(nodePtr) as Entity | null;
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
      return this._session.graph.get(nodePtr) as Space | null;
    }
    return null;
  }
  readonly spacePtr: NodeReference;

  /**
   * Entity.materialization
   */
  readonly materialization: Materialization;

  /**
   * The definition this Entity is an instance of.
   */
  get definition(): Entity | null {
    const nodePtr: NodeReference | null = this.definitionPtr;
    if (nodePtr != null) {
      return this._session.graph.get(nodePtr) as Entity | null;
    }
    return null;
  }
  readonly definitionPtr: NodeReference | null;

  /**
   * The Branch this Entity is part of.
   */
  get branch(): Branch | null {
    const nodePtr: NodeReference | null = this.branchPtr;
    if (nodePtr != null) {
      return this._session.graph.get(nodePtr) as Branch | null;
    }
    return null;
  }
  readonly branchPtr: NodeReference;

  /**
   * The Snapshot this Entity is part of.
   */
  get snapshot(): Snapshot | null {
    const nodePtr: NodeReference | null = this.snapshotPtr;
    if (nodePtr != null) {
      return this._session.graph.get(nodePtr) as Snapshot | null;
    }
    return null;
  }
  readonly snapshotPtr: NodeReference;

  /**
   * The previous Entity this Entity is based on (from the base Branch, if any).
   * This invariant must hold: `Entity.preceded_by.branch == Entity.branch.preceded_by`.
   */
  get precededBy(): Sanction | null {
    const nodePtr: NodeReference | null = this.precededByPtr;
    if (nodePtr != null) {
      return this._session.graph.get(nodePtr) as Sanction | null;
    }
    return null;
  }
  readonly precededByPtr: NodeReference | null;

  /**
   * The (root) Entity that is being instantiated.
   */
  get instance(): Entity | null {
    const nodePtr: NodeReference | null = this.instancePtr;
    if (nodePtr != null) {
      return this._session.graph.get(nodePtr) as Entity | null;
    }
    return null;
  }
  readonly instancePtr: NodeReference | null;

  /**
   * The time this Entity was created (system time).
   */
  readonly createdAt: Datetime;

  /**
   * The logical time this Entity was created (system time).
   */
  readonly createdEpoch: UInt128;

  /**
   * The Actor that created this Entity.
   */
  get createdBy(): Entity | null {
    const nodePtr: NodeReference | null = this.createdByPtr;
    if (nodePtr != null) {
      return this._session.graph.get(nodePtr) as Entity | null;
    }
    return null;
  }
  readonly createdByPtr: NodeReference;

  /**
   * The time this Entity was last updated (system time).
   */
  readonly updatedAt: Datetime;

  /**
   * The logical time this Entity was last updated (system time).
   */
  readonly updatedEpoch: UInt128;

  /**
   * The Actor that last updated this Entity.
   */
  get updatedBy(): Entity | null {
    const nodePtr: NodeReference | null = this.updatedByPtr;
    if (nodePtr != null) {
      return this._session.graph.get(nodePtr) as Entity | null;
    }
    return null;
  }
  readonly updatedByPtr: NodeReference;

  /**
   * The time this Entity was deleted (system time).
   * Only set if the Entity is currently 'deleted'.
   * Deleting and restoring an Entity counts as an update, and thus updates updated_at/updated_epoch.
   */
  readonly deletedAt: Datetime | null;

  /**
   * Entity.ownedBy
   */
  get ownedBy(): Entity | null {
    const nodePtr: NodeReference | null = this.ownedByPtr;
    if (nodePtr != null) {
      return this._session.graph.get(nodePtr) as Entity | null;
    }
    return null;
  }
  set ownedBy(node: Entity | null) {
    if (node === null) {
      this.ownedByPtr = null;
    } else {
      this.ownedByPtr = node.toRef();
    }
  }
  /**
   * Entity.ownedBy
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
   * Entity.name
   */
  /**
   * Entity.name
   */
  get name(): string {
    return this._name;
  }
  set name(value: string) {
    const prop = (this.constructor as NodeClass).__properties__["name"];
    this._session.updateSetProperty(this, prop, value);
    this._name = value;
  }
  _name: string;

  /**
   * The absolute order key of this Entity in its parent.
   */
  readonly orderKey: string;

  /**
   * The custom Values of this Entity, keyed by custom Property id..
   */
  /**
   * The custom Values of this Entity, keyed by custom Property id..
   */
  get customValues(): { readonly [key: UUID]: Value } {
    return this._customValues;
  }
  set customValues(value: { readonly [key: UUID]: Value }) {
    const prop = (this.constructor as NodeClass).__properties__["custom_values"];
    this._session.updateSetProperty(this, prop, value);
    this._customValues = value;
  }
  _customValues: { readonly [key: UUID]: Value };

  /**
   * The Script of this Entity.
   */
  get script(): Script | null {
    const nodePtr: NodeReference | null = this.scriptPtr;
    if (nodePtr != null) {
      return this._session.graph.get(nodePtr) as Script | null;
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
   * The Script of this Entity.
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
   * Whether this Entity can be instanced.
   */
  readonly isExtensible: boolean | null;

  /**
   * The Script that defines this Node.
   */
  get source(): Script | null {
    const nodePtr: NodeReference | null = this.sourcePtr;
    if (nodePtr != null) {
      return this._session.graph.get(nodePtr) as Script | null;
    }
    return null;
  }
  readonly sourcePtr: NodeReference | null;

  /**
   * The key to uniquely identify this Node in reconciliation. If not set, name is used.
   */
  /**
   * The key to uniquely identify this Node in reconciliation. If not set, name is used.
   */
  get key(): string | null {
    return this._key;
  }
  set key(value: string | null) {
    const prop = (this.constructor as NodeClass).__properties__["key"];
    this._session.updateSetProperty(this, prop, value);
    this._key = value;
  }
  _key: string | null;

  /**
   * Sanction.type
   */
  /**
   * Sanction.type
   */
  get type(): SanctionType {
    return this._type;
  }
  set type(value: SanctionType) {
    const prop = (this.constructor as NodeClass).__properties__["type"];
    this._session.updateSetProperty(this, prop, value);
    this._type = value;
  }
  _type: SanctionType;

  /**
   * Sanction.expiresAt
   */
  /**
   * Sanction.expiresAt
   */
  get expiresAt(): Datetime | null {
    return this._expiresAt;
  }
  set expiresAt(value: Datetime | null) {
    const prop = (this.constructor as NodeClass).__properties__["expires_at"];
    this._session.updateSetProperty(this, prop, value);
    this._expiresAt = value;
  }
  _expiresAt: Datetime | null;

  /**
   * Sanction.target
   */
  get target(): Entity | null {
    const nodePtr: NodeReference | null = this.targetPtr;
    if (nodePtr != null) {
      return this._session.graph.get(nodePtr) as Entity | null;
    }
    return null;
  }
  set target(node: Entity) {
    this.targetPtr = node.toRef();
  }
  /**
   * Sanction.target
   */
  get targetPtr(): NodeReference {
    return this._targetPtr;
  }
  set targetPtr(value: NodeReference) {
    const prop = (this.constructor as NodeClass).__properties__["target"];
    this._session.updateSetProperty(this, prop, value);
    this._targetPtr = value;
  }
  _targetPtr: NodeReference;

  constructor(options: {
    id?: UUID;
    parent?: Entity | NodeReference | null;
    space?: Space | NodeReference;
    materialization?: Materialization;
    definition?: Entity | NodeReference | null;
    branch?: Branch | NodeReference;
    snapshot?: Snapshot | NodeReference;
    precededBy?: Sanction | NodeReference | null;
    instance?: Entity | NodeReference | null;
    createdAt?: Datetime;
    createdEpoch?: UInt128;
    createdBy?: Entity | NodeReference;
    updatedAt?: Datetime;
    updatedEpoch?: UInt128;
    updatedBy?: Entity | NodeReference;
    deletedAt?: Datetime | null;
    ownedBy?: Entity | NodeReference | null;
    name?: string;
    orderKey?: string;
    customValues?: { readonly [key: UUID]: Value };
    script?: Script | NodeReference | null;
    isExtensible?: boolean | null;
    source?: Script | NodeReference | null;
    key?: string | null;
    type: SanctionType;
    expiresAt?: Datetime | null;
    target: Entity | NodeReference;
    _session?: Session | null;
  }) {
    /* super */
    super(
      /* id */
      options.id ?? null,
      /* parent */
      options.parent != null ? options.parent.toRef() : null,
      /* session */
      options._session ?? null,
      /* _isNew */
      options.id == null,
    );

    /* properties */
    let _parent = options.parent ?? null;
    if (_parent != null && _parent.constructor.name !== "NodeReference") {
      _parent = (_parent as Node).toRef();
    }
    this.parentPtr = _parent as NodeReference | null;
    let _space = options.space ?? null;
    if (_space != null && _space.constructor.name !== "NodeReference") {
      _space = (_space as Node).toRef();
    }
    if (_space == null) {
      _space = ACTIVE_SPACE.get();
      if (_space == null) {
        throw new Error(`no active Space for Sanction`);
      }
      _space = _space.toRef();
    }
    if (_space == null) {
      throw new Error(`Sanction.space is required`);
    }
    this.spacePtr = _space as NodeReference;
    let _materialization = options.materialization ?? null;
    if (_materialization == null) {
      _materialization = 11 /* Materialization.ROOT */;
    }
    if (_materialization == null) {
      throw new Error(`Sanction.materialization is required`);
    }
    this.materialization = _materialization;
    let _definition = options.definition ?? null;
    if (_definition != null && _definition.constructor.name !== "NodeReference") {
      _definition = (_definition as Node).toRef();
    }
    this.definitionPtr = _definition as NodeReference | null;
    let _branch = options.branch ?? null;
    if (_branch != null && _branch.constructor.name !== "NodeReference") {
      _branch = (_branch as Node).toRef();
    }
    if (_branch == null) {
      _branch = ACTIVE_BRANCH.get();
      if (_branch == null) {
        throw new Error(`no active Branch for Sanction`);
      }
      _branch = _branch.toRef();
    }
    if (_branch == null) {
      throw new Error(`Sanction.branch is required`);
    }
    this.branchPtr = _branch as NodeReference;
    let _snapshot = options.snapshot ?? null;
    if (_snapshot != null && _snapshot.constructor.name !== "NodeReference") {
      _snapshot = (_snapshot as Node).toRef();
    }
    if (_snapshot == null) {
      _snapshot = ACTIVE_SNAPSHOT.get();
      if (_snapshot == null) {
        throw new Error(`no active Snapshot for Sanction`);
      }
      _snapshot = _snapshot.toRef();
    }
    if (_snapshot == null) {
      throw new Error(`Sanction.snapshot is required`);
    }
    this.snapshotPtr = _snapshot as NodeReference;
    let _precededBy = options.precededBy ?? null;
    if (_precededBy != null && _precededBy.constructor.name !== "NodeReference") {
      _precededBy = (_precededBy as Node).toRef();
    }
    this.precededByPtr = _precededBy as NodeReference | null;
    let _instance = options.instance ?? null;
    if (_instance != null && _instance.constructor.name !== "NodeReference") {
      _instance = (_instance as Node).toRef();
    }
    this.instancePtr = _instance as NodeReference | null;
    let _deletedAt = options.deletedAt ?? null;
    this.deletedAt = _deletedAt;
    let _ownedBy = options.ownedBy ?? null;
    if (_ownedBy != null && _ownedBy.constructor.name !== "NodeReference") {
      _ownedBy = (_ownedBy as Node).toRef();
    }
    this._ownedByPtr = _ownedBy as NodeReference | null;
    let _name = options.name ?? null;
    if (_name == null) {
      _name = "Sanction";
    }
    if (_name == null) {
      throw new Error(`Sanction.name is required`);
    }
    this._name = _name;
    let _orderKey = options.orderKey ?? null;
    if (_orderKey == null) {
      _orderKey = "a0";
    }
    if (_orderKey == null) {
      throw new Error(`Sanction.orderKey is required`);
    }
    this.orderKey = _orderKey;
    let _customValues = options.customValues ?? null;
    if (_customValues == null) {
      _customValues = {};
    }
    this._customValues = _customValues;
    let _script = options.script ?? null;
    if (_script != null && _script.constructor.name !== "NodeReference") {
      _script = (_script as Node).toRef();
    }
    this._scriptPtr = _script as NodeReference | null;
    let _isExtensible = options.isExtensible ?? null;
    this.isExtensible = _isExtensible;
    let _source = options.source ?? null;
    if (_source != null && _source.constructor.name !== "NodeReference") {
      _source = (_source as Node).toRef();
    }
    this.sourcePtr = _source as NodeReference | null;
    let _key = options.key ?? null;
    this._key = _key;
    let _type = options.type;
    if (_type == null) {
      throw new Error(`Sanction.type is required`);
    }
    this._type = _type;
    let _expiresAt = options.expiresAt ?? null;
    this._expiresAt = _expiresAt;
    let _target = options.target;
    if (_target != null && _target.constructor.name !== "NodeReference") {
      _target = (_target as Node).toRef();
    }
    if (_target == null) {
      throw new Error(`Sanction.target is required`);
    }
    this._targetPtr = _target as NodeReference;

    /* identity */
    if (options.id == null) {
      const now = Temporal.Now.zonedDateTimeISO("UTC");
      const epoch = this._session.epoch;
      this.createdAt = now;
      this.createdEpoch = epoch;
      this.createdByPtr = this._session.actorPtr;
      this.updatedAt = now;
      this.updatedEpoch = epoch;
      this.updatedByPtr = this._session.actorPtr;
    } else {
      if (
        options.createdAt == null ||
        options.updatedAt == null ||
        options.createdEpoch == null ||
        options.updatedEpoch == null
      ) {
        throw new Error(
          `Sanction.createdAt and Sanction.updatedAt are required for existing Nodes`,
        );
      }
      this.createdAt = options.createdAt;
      this.createdEpoch = options.createdEpoch;
      this.createdByPtr =
        options.createdBy != null ? options.createdBy.toRef() : this._session.actorPtr;
      this.updatedAt = options.updatedAt;
      this.updatedEpoch = options.updatedEpoch;
      this.updatedByPtr =
        options.updatedBy != null ? options.updatedBy.toRef() : this._session.actorPtr;
    }
  }

  equals(other: any): boolean {
    if (!(this.metatype === other.metatype)) {
      return false;
    }
    if (!(this._type === other._type)) {
      return false;
    }
    if (!(this._expiresAt === other._expiresAt)) {
      return false;
    }
    if (!(this._targetPtr.id === other._targetPtr.id)) {
      return false;
    }
    if (!(this.definitionPtr?.id === other.definitionPtr?.id)) {
      return false;
    }
    if (!(this._ownedByPtr?.id === other._ownedByPtr?.id)) {
      return false;
    }
    if (!(this._name === other._name)) {
      return false;
    }
    if (JSON.stringify(this._customValues) !== JSON.stringify(other._customValues)) {
      return false;
    }
    if (!(this._scriptPtr?.id === other._scriptPtr?.id)) {
      return false;
    }
    if (!(this.isExtensible === other.isExtensible)) {
      return false;
    }
    if (!(this.sourcePtr?.id === other.sourcePtr?.id)) {
      return false;
    }
    if (!(this._key === other._key)) {
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
    h = (h * 31 + this._type) & 0xffffffff;
    if (this._expiresAt != null) {
      h = (h * 31 + hashString(this._expiresAt.toString({ timeZoneName: "never" }))) & 0xffffffff;
    }
    h = (h * 31 + hashString(this._targetPtr.id)) & 0xffffffff;
    if (this.parentPtr != null) {
      h = (h * 31 + hashString(this.parentPtr.id)) & 0xffffffff;
    }
    if (this.definitionPtr != null) {
      h = (h * 31 + hashString(this.definitionPtr.id)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.createdAt.toString({ timeZoneName: "never" }))) & 0xffffffff;
    h = (h * 31 + hashString(this.createdByPtr.id)) & 0xffffffff;
    h = (h * 31 + hashString(this.updatedAt.toString({ timeZoneName: "never" }))) & 0xffffffff;
    h = (h * 31 + hashString(this.updatedByPtr.id)) & 0xffffffff;
    if (this.deletedAt != null) {
      h = (h * 31 + hashString(this.deletedAt.toString({ timeZoneName: "never" }))) & 0xffffffff;
    }
    if (this._ownedByPtr != null) {
      h = (h * 31 + hashString(this._ownedByPtr.id)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this._name)) & 0xffffffff;
    h = (h * 31 + hashString(this.orderKey)) & 0xffffffff;
    if (this._customValues && Object.keys(this._customValues).length > 0) {
      for (const [_key, _value] of Object.entries(this._customValues)) {
        h = (h * 31 + hashString(_key.toString())) & 0xffffffff;
        h = (h * 31 + _value.hash()) & 0xffffffff;
      }
    }
    if (this._scriptPtr != null) {
      h = (h * 31 + hashString(this._scriptPtr.id)) & 0xffffffff;
    }
    if (this.isExtensible != null) {
      h = (h * 31 + hashBool(this.isExtensible)) & 0xffffffff;
    }
    if (this.sourcePtr != null) {
      h = (h * 31 + hashString(this.sourcePtr.id)) & 0xffffffff;
    }
    if (this._key != null) {
      h = (h * 31 + hashString(this._key)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.id.toString())) & 0xffffffff;
    h = (h * 31 + hashString(this.spacePtr.id)) & 0xffffffff;

    return h;
  }

  __toRef__(): NodeReference {
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    return new _NodeReference({
      type: NodeType.SANCTION,
      id: this.id,
      spaceId: this.spacePtr.id,
      definitionId: this.definitionPtr?.id ?? null,
      branchId: this.branchPtr.id,
      snapshotId: this.snapshotPtr.id,
      _session: this._session,
    });
  }

  get _pathKey(): string {
    return this.name;
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
    propertyReprs.push(`name=${`"${this.name}"`}`);
    return `<Sanction "${this.path}" ${propertyReprs.join(" ")}>`;
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerNodeClass(NodeType.SANCTION, Sanction);
/* ==== DESTACK_GENERATED_END:NODE:360400 ==== */
