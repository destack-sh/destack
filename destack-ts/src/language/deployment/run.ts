import type {
  Branch,
  Datetime,
  Duration,
  Materialization,
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
import { hashInt, hashString } from "@destack/utils/hash";
import { Temporal } from "temporal-polyfill";

/* ==== DESTACK_GENERATED_START:ENUM:1110000 ==== */
/**
 * RunStatus
 */
export enum RunStatus {
  SCHEDULED = 2,
  RUNNING = 10,
  PAUSED = 21,
  YIELDED = 23,
  CANCELLED = 51,
  ABORTED = 52,
  FAILED = 53,
  COMPLETED = 54,

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerEnumClass(EnumType.RUN_STATUS, RunStatus);
/* ==== DESTACK_GENERATED_END:ENUM:1110000 ==== */

/* ==== DESTACK_GENERATED_START:NODE:1110001 ==== */
/**
 * An Event regarding a Run.
 */
export abstract class RunEvent extends Event {
  static metatype: NodeType = NodeType.RUN_EVENT;

  /**
   * The Space this Node is in.
   */
  abstract get space(): Space | null;
  declare readonly spaceRef: NodeReference;

  /**
   * The definition this Event is an instance of.
   */
  abstract get definition(): Entity | null;
  declare readonly definitionRef: NodeReference | null;

  /**
   * The Branch this Event originated from.
   */
  abstract get branch(): Branch | null;
  declare readonly branchRef: NodeReference;

  /**
   * The Snapshot this Event originated from.
   */
  abstract get snapshot(): Snapshot | null;
  declare readonly snapshotRef: NodeReference;

  /**
   * The previous Event that this Event follows.
   */
  abstract get precededBy(): Event | null;
  declare readonly precededByRef: NodeReference | null;

  /**
   * The Event that caused this Event (if any).
   */
  abstract get causedBy(): Event | null;
  declare readonly causedByRef: NodeReference | null;

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
  declare readonly createdByRef: NodeReference;

  /**
   * The Client that created this Event (client).
   */
  abstract get client(): Client | null;
  declare readonly clientRef: NodeReference;

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
   * RunEvent.node
   */
  abstract get node(): Run | null;
  declare readonly nodeRef: NodeReference;

  /**
   * RunEvent.target
   */
  abstract get target(): Entity | null;
  declare readonly targetRef: NodeReference | null;

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerNodeClass(NodeType.RUN_EVENT, RunEvent);
/* ==== DESTACK_GENERATED_END:NODE:1110001 ==== */

/* ==== DESTACK_GENERATED_START:NODE:1110002 ==== */
/**
 * A Run was started.
 */
export class RunStartedEvent extends RunEvent {
  static metatype: NodeType = NodeType.RUN_STARTED_EVENT;

  /**
   * The Space this Node is in.
   */
  get space(): Space | null {
    const nodeRef: NodeReference | null = this.spaceRef;
    if (nodeRef != null) {
      return this._session.graph.get(nodeRef) as Space | null;
    }
    return null;
  }
  readonly spaceRef: NodeReference;

  /**
   * The definition this Event is an instance of.
   */
  get definition(): Entity | null {
    const nodeRef: NodeReference | null = this.definitionRef;
    if (nodeRef != null) {
      return this._session.graph.get(nodeRef) as Entity | null;
    }
    return null;
  }
  readonly definitionRef: NodeReference | null;

  /**
   * The Branch this Event originated from.
   */
  get branch(): Branch | null {
    const nodeRef: NodeReference | null = this.branchRef;
    if (nodeRef != null) {
      return this._session.graph.get(nodeRef) as Branch | null;
    }
    return null;
  }
  readonly branchRef: NodeReference;

  /**
   * The Snapshot this Event originated from.
   */
  get snapshot(): Snapshot | null {
    const nodeRef: NodeReference | null = this.snapshotRef;
    if (nodeRef != null) {
      return this._session.graph.get(nodeRef) as Snapshot | null;
    }
    return null;
  }
  readonly snapshotRef: NodeReference;

  /**
   * The previous Event that this Event follows.
   */
  get precededBy(): Event | null {
    const nodeRef: NodeReference | null = this.precededByRef;
    if (nodeRef != null) {
      return this._session.graph.get(nodeRef) as Event | null;
    }
    return null;
  }
  readonly precededByRef: NodeReference | null;

  /**
   * The Event that caused this Event (if any).
   */
  get causedBy(): Event | null {
    const nodeRef: NodeReference | null = this.causedByRef;
    if (nodeRef != null) {
      return this._session.graph.get(nodeRef) as Event | null;
    }
    return null;
  }
  readonly causedByRef: NodeReference | null;

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
    const nodeRef: NodeReference | null = this.createdByRef;
    if (nodeRef != null) {
      return this._session.graph.get(nodeRef) as Entity | null;
    }
    return null;
  }
  readonly createdByRef: NodeReference;

  /**
   * The Client that created this Event (client).
   */
  get client(): Client | null {
    const nodeRef: NodeReference | null = this.clientRef;
    if (nodeRef != null) {
      return this._session.graph.get(nodeRef) as Client | null;
    }
    return null;
  }
  readonly clientRef: NodeReference;

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
   * RunEvent.node
   */
  get node(): Run | null {
    const nodeRef: NodeReference | null = this.nodeRef;
    if (nodeRef != null) {
      return this._session.graph.get(nodeRef) as Run | null;
    }
    return null;
  }
  readonly nodeRef: NodeReference;

  /**
   * RunEvent.target
   */
  get target(): Entity | null {
    const nodeRef: NodeReference | null = this.targetRef;
    if (nodeRef != null) {
      return this._session.graph.get(nodeRef) as Entity | null;
    }
    return null;
  }
  readonly targetRef: NodeReference | null;

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
    node: Run | NodeReference;
    target?: Entity | NodeReference | null;
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
        throw new Error(`no active Space for RunStartedEvent`);
      }
      _space = _space.toRef();
    }
    if (_space == null) {
      throw new Error(`RunStartedEvent.space is required`);
    }
    this.spaceRef = _space as NodeReference;
    let _definition = options.definition ?? null;
    if (_definition != null && _definition.constructor.name !== "NodeReference") {
      _definition = (_definition as Node).toRef();
    }
    this.definitionRef = _definition as NodeReference | null;
    let _branch = options.branch ?? null;
    if (_branch != null && _branch.constructor.name !== "NodeReference") {
      _branch = (_branch as Node).toRef();
    }
    if (_branch == null) {
      _branch = ACTIVE_BRANCH.get();
      if (_branch == null) {
        throw new Error(`no active Branch for RunStartedEvent`);
      }
      _branch = _branch.toRef();
    }
    if (_branch == null) {
      throw new Error(`RunStartedEvent.branch is required`);
    }
    this.branchRef = _branch as NodeReference;
    let _snapshot = options.snapshot ?? null;
    if (_snapshot != null && _snapshot.constructor.name !== "NodeReference") {
      _snapshot = (_snapshot as Node).toRef();
    }
    if (_snapshot == null) {
      _snapshot = ACTIVE_SNAPSHOT.get();
      if (_snapshot == null) {
        throw new Error(`no active Snapshot for RunStartedEvent`);
      }
      _snapshot = _snapshot.toRef();
    }
    if (_snapshot == null) {
      throw new Error(`RunStartedEvent.snapshot is required`);
    }
    this.snapshotRef = _snapshot as NodeReference;
    let _precededBy = options.precededBy ?? null;
    if (_precededBy != null && _precededBy.constructor.name !== "NodeReference") {
      _precededBy = (_precededBy as Node).toRef();
    }
    this.precededByRef = _precededBy as NodeReference | null;
    let _causedBy = options.causedBy ?? null;
    if (_causedBy != null && _causedBy.constructor.name !== "NodeReference") {
      _causedBy = (_causedBy as Node).toRef();
    }
    this.causedByRef = _causedBy as NodeReference | null;
    let _client = options.client ?? null;
    if (_client != null && _client.constructor.name !== "NodeReference") {
      _client = (_client as Node).toRef();
    }
    if (_client == null) {
      _client = this._session.clientRef;
    }
    if (_client == null) {
      throw new Error(`RunStartedEvent.client is required`);
    }
    this.clientRef = _client as NodeReference;
    let _clientNonce = options.clientNonce ?? null;
    if (_clientNonce == null) {
      _clientNonce = this._session.clientNonce;
    }
    if (_clientNonce == null) {
      throw new Error(`RunStartedEvent.clientNonce is required`);
    }
    this.clientNonce = _clientNonce;
    let _status = options.status ?? null;
    if (_status == null) {
      _status = 1 /* EventStatus.PENDING */;
    }
    if (_status == null) {
      throw new Error(`RunStartedEvent.status is required`);
    }
    this.status = _status;
    let _node = options.node;
    if (_node != null && _node.constructor.name !== "NodeReference") {
      _node = (_node as Node).toRef();
    }
    if (_node == null) {
      throw new Error(`RunStartedEvent.node is required`);
    }
    this.nodeRef = _node as NodeReference;
    let _target = options.target ?? null;
    if (_target != null && _target.constructor.name !== "NodeReference") {
      _target = (_target as Node).toRef();
    }
    this.targetRef = _target as NodeReference | null;

    /* identity */
    if (options.id == null) {
      const now = Temporal.Now.zonedDateTimeISO("UTC");
      const epoch = this._session.remoteEpoch;
      this.createdAt = now;
      this.createdEpoch = epoch;
      this.createdByRef = this._session.actorRef;
      this.clientCreatedAt = now;
      this.clientEpoch = epoch;
    } else {
      if (
        options.createdAt == null ||
        options.createdEpoch == null ||
        options.clientCreatedAt == null ||
        options.clientEpoch == null
      ) {
        throw new Error(`RunStartedEvent.createdAt is required for existing Events`);
      }
      this.createdAt = options.createdAt;
      this.createdEpoch = options.createdEpoch;
      this.createdByRef =
        options.createdBy != null ? options.createdBy.toRef() : this._session.actorRef;
      this.clientCreatedAt = options.clientCreatedAt;
      this.clientEpoch = options.clientEpoch;
    }
  }

  equals(other: any): boolean {
    if (!(this.metatype === other.metatype)) {
      return false;
    }
    if (!(this.nodeRef.id === other.nodeRef.id)) {
      return false;
    }
    if (!(this.targetRef?.id === other.targetRef?.id)) {
      return false;
    }
    if (!(this.definitionRef?.id === other.definitionRef?.id)) {
      return false;
    }
    if (!(this.branchRef.id === other.branchRef.id)) {
      return false;
    }
    if (!(this.snapshotRef.id === other.snapshotRef.id)) {
      return false;
    }
    if (!(this.precededByRef?.id === other.precededByRef?.id)) {
      return false;
    }
    if (!(this.causedByRef?.id === other.causedByRef?.id)) {
      return false;
    }
    if (!(this.clientRef.id === other.clientRef.id)) {
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
    if (!(this.spaceRef.id === other.spaceRef.id)) {
      return false;
    }
    return true;
  }

  hash(): number {
    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    h = (h * 31 + hashString(this.nodeRef.id)) & 0xffffffff;
    if (this.targetRef != null) {
      h = (h * 31 + hashString(this.targetRef.id)) & 0xffffffff;
    }
    if (this.definitionRef != null) {
      h = (h * 31 + hashString(this.definitionRef.id)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.branchRef.id)) & 0xffffffff;
    h = (h * 31 + hashString(this.snapshotRef.id)) & 0xffffffff;
    if (this.precededByRef != null) {
      h = (h * 31 + hashString(this.precededByRef.id)) & 0xffffffff;
    }
    if (this.causedByRef != null) {
      h = (h * 31 + hashString(this.causedByRef.id)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.createdAt.toString({ timeZoneName: "never" }))) & 0xffffffff;
    h = (h * 31 + hashString(this.createdByRef.id)) & 0xffffffff;
    h = (h * 31 + hashString(this.clientRef.id)) & 0xffffffff;
    h = (h * 31 + hashString(this.clientNonce.toString())) & 0xffffffff;
    h =
      (h * 31 + hashString(this.clientCreatedAt.toString({ timeZoneName: "never" }))) & 0xffffffff;
    h = (h * 31 + hashInt(this.clientEpoch)) & 0xffffffff;
    h = (h * 31 + this.status) & 0xffffffff;
    h = (h * 31 + hashString(this.id.toString())) & 0xffffffff;
    h = (h * 31 + hashString(this.spaceRef.id)) & 0xffffffff;

    return h;
  }

  __toRef__(): NodeReference {
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    return new _NodeReference({
      type: NodeType.RUN_STARTED_EVENT,
      id: this.id,
      spaceId: this.spaceRef.id,
      definitionId: this.definitionRef?.id ?? null,
      branchId: this.branchRef.id,
      snapshotId: this.snapshotRef.id,
      _session: this._session,
    });
  }

  get _pathKey(): string {
    return `RunStartedEvent[id=${this.id}]`;
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
    return `<RunStartedEvent "${this.path}" ${propertyReprs.join(" ")}>`;
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerNodeClass(NodeType.RUN_STARTED_EVENT, RunStartedEvent);
/* ==== DESTACK_GENERATED_END:NODE:1110002 ==== */

/* ==== DESTACK_GENERATED_START:NODE:1110003 ==== */
/**
 * A Run was paused.
 */
export class RunPauseRequestedEvent extends RunEvent {
  static metatype: NodeType = NodeType.RUN_PAUSE_REQUESTED_EVENT;

  /**
   * The Space this Node is in.
   */
  get space(): Space | null {
    const nodeRef: NodeReference | null = this.spaceRef;
    if (nodeRef != null) {
      return this._session.graph.get(nodeRef) as Space | null;
    }
    return null;
  }
  readonly spaceRef: NodeReference;

  /**
   * The definition this Event is an instance of.
   */
  get definition(): Entity | null {
    const nodeRef: NodeReference | null = this.definitionRef;
    if (nodeRef != null) {
      return this._session.graph.get(nodeRef) as Entity | null;
    }
    return null;
  }
  readonly definitionRef: NodeReference | null;

  /**
   * The Branch this Event originated from.
   */
  get branch(): Branch | null {
    const nodeRef: NodeReference | null = this.branchRef;
    if (nodeRef != null) {
      return this._session.graph.get(nodeRef) as Branch | null;
    }
    return null;
  }
  readonly branchRef: NodeReference;

  /**
   * The Snapshot this Event originated from.
   */
  get snapshot(): Snapshot | null {
    const nodeRef: NodeReference | null = this.snapshotRef;
    if (nodeRef != null) {
      return this._session.graph.get(nodeRef) as Snapshot | null;
    }
    return null;
  }
  readonly snapshotRef: NodeReference;

  /**
   * The previous Event that this Event follows.
   */
  get precededBy(): Event | null {
    const nodeRef: NodeReference | null = this.precededByRef;
    if (nodeRef != null) {
      return this._session.graph.get(nodeRef) as Event | null;
    }
    return null;
  }
  readonly precededByRef: NodeReference | null;

  /**
   * The Event that caused this Event (if any).
   */
  get causedBy(): Event | null {
    const nodeRef: NodeReference | null = this.causedByRef;
    if (nodeRef != null) {
      return this._session.graph.get(nodeRef) as Event | null;
    }
    return null;
  }
  readonly causedByRef: NodeReference | null;

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
    const nodeRef: NodeReference | null = this.createdByRef;
    if (nodeRef != null) {
      return this._session.graph.get(nodeRef) as Entity | null;
    }
    return null;
  }
  readonly createdByRef: NodeReference;

  /**
   * The Client that created this Event (client).
   */
  get client(): Client | null {
    const nodeRef: NodeReference | null = this.clientRef;
    if (nodeRef != null) {
      return this._session.graph.get(nodeRef) as Client | null;
    }
    return null;
  }
  readonly clientRef: NodeReference;

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
   * RunEvent.node
   */
  get node(): Run | null {
    const nodeRef: NodeReference | null = this.nodeRef;
    if (nodeRef != null) {
      return this._session.graph.get(nodeRef) as Run | null;
    }
    return null;
  }
  readonly nodeRef: NodeReference;

  /**
   * RunEvent.target
   */
  get target(): Entity | null {
    const nodeRef: NodeReference | null = this.targetRef;
    if (nodeRef != null) {
      return this._session.graph.get(nodeRef) as Entity | null;
    }
    return null;
  }
  readonly targetRef: NodeReference | null;

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
    node: Run | NodeReference;
    target?: Entity | NodeReference | null;
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
        throw new Error(`no active Space for RunPauseRequestedEvent`);
      }
      _space = _space.toRef();
    }
    if (_space == null) {
      throw new Error(`RunPauseRequestedEvent.space is required`);
    }
    this.spaceRef = _space as NodeReference;
    let _definition = options.definition ?? null;
    if (_definition != null && _definition.constructor.name !== "NodeReference") {
      _definition = (_definition as Node).toRef();
    }
    this.definitionRef = _definition as NodeReference | null;
    let _branch = options.branch ?? null;
    if (_branch != null && _branch.constructor.name !== "NodeReference") {
      _branch = (_branch as Node).toRef();
    }
    if (_branch == null) {
      _branch = ACTIVE_BRANCH.get();
      if (_branch == null) {
        throw new Error(`no active Branch for RunPauseRequestedEvent`);
      }
      _branch = _branch.toRef();
    }
    if (_branch == null) {
      throw new Error(`RunPauseRequestedEvent.branch is required`);
    }
    this.branchRef = _branch as NodeReference;
    let _snapshot = options.snapshot ?? null;
    if (_snapshot != null && _snapshot.constructor.name !== "NodeReference") {
      _snapshot = (_snapshot as Node).toRef();
    }
    if (_snapshot == null) {
      _snapshot = ACTIVE_SNAPSHOT.get();
      if (_snapshot == null) {
        throw new Error(`no active Snapshot for RunPauseRequestedEvent`);
      }
      _snapshot = _snapshot.toRef();
    }
    if (_snapshot == null) {
      throw new Error(`RunPauseRequestedEvent.snapshot is required`);
    }
    this.snapshotRef = _snapshot as NodeReference;
    let _precededBy = options.precededBy ?? null;
    if (_precededBy != null && _precededBy.constructor.name !== "NodeReference") {
      _precededBy = (_precededBy as Node).toRef();
    }
    this.precededByRef = _precededBy as NodeReference | null;
    let _causedBy = options.causedBy ?? null;
    if (_causedBy != null && _causedBy.constructor.name !== "NodeReference") {
      _causedBy = (_causedBy as Node).toRef();
    }
    this.causedByRef = _causedBy as NodeReference | null;
    let _client = options.client ?? null;
    if (_client != null && _client.constructor.name !== "NodeReference") {
      _client = (_client as Node).toRef();
    }
    if (_client == null) {
      _client = this._session.clientRef;
    }
    if (_client == null) {
      throw new Error(`RunPauseRequestedEvent.client is required`);
    }
    this.clientRef = _client as NodeReference;
    let _clientNonce = options.clientNonce ?? null;
    if (_clientNonce == null) {
      _clientNonce = this._session.clientNonce;
    }
    if (_clientNonce == null) {
      throw new Error(`RunPauseRequestedEvent.clientNonce is required`);
    }
    this.clientNonce = _clientNonce;
    let _status = options.status ?? null;
    if (_status == null) {
      _status = 1 /* EventStatus.PENDING */;
    }
    if (_status == null) {
      throw new Error(`RunPauseRequestedEvent.status is required`);
    }
    this.status = _status;
    let _node = options.node;
    if (_node != null && _node.constructor.name !== "NodeReference") {
      _node = (_node as Node).toRef();
    }
    if (_node == null) {
      throw new Error(`RunPauseRequestedEvent.node is required`);
    }
    this.nodeRef = _node as NodeReference;
    let _target = options.target ?? null;
    if (_target != null && _target.constructor.name !== "NodeReference") {
      _target = (_target as Node).toRef();
    }
    this.targetRef = _target as NodeReference | null;

    /* identity */
    if (options.id == null) {
      const now = Temporal.Now.zonedDateTimeISO("UTC");
      const epoch = this._session.remoteEpoch;
      this.createdAt = now;
      this.createdEpoch = epoch;
      this.createdByRef = this._session.actorRef;
      this.clientCreatedAt = now;
      this.clientEpoch = epoch;
    } else {
      if (
        options.createdAt == null ||
        options.createdEpoch == null ||
        options.clientCreatedAt == null ||
        options.clientEpoch == null
      ) {
        throw new Error(`RunPauseRequestedEvent.createdAt is required for existing Events`);
      }
      this.createdAt = options.createdAt;
      this.createdEpoch = options.createdEpoch;
      this.createdByRef =
        options.createdBy != null ? options.createdBy.toRef() : this._session.actorRef;
      this.clientCreatedAt = options.clientCreatedAt;
      this.clientEpoch = options.clientEpoch;
    }
  }

  equals(other: any): boolean {
    if (!(this.metatype === other.metatype)) {
      return false;
    }
    if (!(this.nodeRef.id === other.nodeRef.id)) {
      return false;
    }
    if (!(this.targetRef?.id === other.targetRef?.id)) {
      return false;
    }
    if (!(this.definitionRef?.id === other.definitionRef?.id)) {
      return false;
    }
    if (!(this.branchRef.id === other.branchRef.id)) {
      return false;
    }
    if (!(this.snapshotRef.id === other.snapshotRef.id)) {
      return false;
    }
    if (!(this.precededByRef?.id === other.precededByRef?.id)) {
      return false;
    }
    if (!(this.causedByRef?.id === other.causedByRef?.id)) {
      return false;
    }
    if (!(this.clientRef.id === other.clientRef.id)) {
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
    if (!(this.spaceRef.id === other.spaceRef.id)) {
      return false;
    }
    return true;
  }

  hash(): number {
    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    h = (h * 31 + hashString(this.nodeRef.id)) & 0xffffffff;
    if (this.targetRef != null) {
      h = (h * 31 + hashString(this.targetRef.id)) & 0xffffffff;
    }
    if (this.definitionRef != null) {
      h = (h * 31 + hashString(this.definitionRef.id)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.branchRef.id)) & 0xffffffff;
    h = (h * 31 + hashString(this.snapshotRef.id)) & 0xffffffff;
    if (this.precededByRef != null) {
      h = (h * 31 + hashString(this.precededByRef.id)) & 0xffffffff;
    }
    if (this.causedByRef != null) {
      h = (h * 31 + hashString(this.causedByRef.id)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.createdAt.toString({ timeZoneName: "never" }))) & 0xffffffff;
    h = (h * 31 + hashString(this.createdByRef.id)) & 0xffffffff;
    h = (h * 31 + hashString(this.clientRef.id)) & 0xffffffff;
    h = (h * 31 + hashString(this.clientNonce.toString())) & 0xffffffff;
    h =
      (h * 31 + hashString(this.clientCreatedAt.toString({ timeZoneName: "never" }))) & 0xffffffff;
    h = (h * 31 + hashInt(this.clientEpoch)) & 0xffffffff;
    h = (h * 31 + this.status) & 0xffffffff;
    h = (h * 31 + hashString(this.id.toString())) & 0xffffffff;
    h = (h * 31 + hashString(this.spaceRef.id)) & 0xffffffff;

    return h;
  }

  __toRef__(): NodeReference {
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    return new _NodeReference({
      type: NodeType.RUN_PAUSE_REQUESTED_EVENT,
      id: this.id,
      spaceId: this.spaceRef.id,
      definitionId: this.definitionRef?.id ?? null,
      branchId: this.branchRef.id,
      snapshotId: this.snapshotRef.id,
      _session: this._session,
    });
  }

  get _pathKey(): string {
    return `RunPauseRequestedEvent[id=${this.id}]`;
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
    return `<RunPauseRequestedEvent "${this.path}" ${propertyReprs.join(" ")}>`;
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerNodeClass(NodeType.RUN_PAUSE_REQUESTED_EVENT, RunPauseRequestedEvent);
/* ==== DESTACK_GENERATED_END:NODE:1110003 ==== */

/* ==== DESTACK_GENERATED_START:NODE:1110004 ==== */
/**
 * A Run was paused.
 */
export class RunPausedEvent extends RunEvent {
  static metatype: NodeType = NodeType.RUN_PAUSED_EVENT;

  /**
   * The Space this Node is in.
   */
  get space(): Space | null {
    const nodeRef: NodeReference | null = this.spaceRef;
    if (nodeRef != null) {
      return this._session.graph.get(nodeRef) as Space | null;
    }
    return null;
  }
  readonly spaceRef: NodeReference;

  /**
   * The definition this Event is an instance of.
   */
  get definition(): Entity | null {
    const nodeRef: NodeReference | null = this.definitionRef;
    if (nodeRef != null) {
      return this._session.graph.get(nodeRef) as Entity | null;
    }
    return null;
  }
  readonly definitionRef: NodeReference | null;

  /**
   * The Branch this Event originated from.
   */
  get branch(): Branch | null {
    const nodeRef: NodeReference | null = this.branchRef;
    if (nodeRef != null) {
      return this._session.graph.get(nodeRef) as Branch | null;
    }
    return null;
  }
  readonly branchRef: NodeReference;

  /**
   * The Snapshot this Event originated from.
   */
  get snapshot(): Snapshot | null {
    const nodeRef: NodeReference | null = this.snapshotRef;
    if (nodeRef != null) {
      return this._session.graph.get(nodeRef) as Snapshot | null;
    }
    return null;
  }
  readonly snapshotRef: NodeReference;

  /**
   * The previous Event that this Event follows.
   */
  get precededBy(): Event | null {
    const nodeRef: NodeReference | null = this.precededByRef;
    if (nodeRef != null) {
      return this._session.graph.get(nodeRef) as Event | null;
    }
    return null;
  }
  readonly precededByRef: NodeReference | null;

  /**
   * The Event that caused this Event (if any).
   */
  get causedBy(): Event | null {
    const nodeRef: NodeReference | null = this.causedByRef;
    if (nodeRef != null) {
      return this._session.graph.get(nodeRef) as Event | null;
    }
    return null;
  }
  readonly causedByRef: NodeReference | null;

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
    const nodeRef: NodeReference | null = this.createdByRef;
    if (nodeRef != null) {
      return this._session.graph.get(nodeRef) as Entity | null;
    }
    return null;
  }
  readonly createdByRef: NodeReference;

  /**
   * The Client that created this Event (client).
   */
  get client(): Client | null {
    const nodeRef: NodeReference | null = this.clientRef;
    if (nodeRef != null) {
      return this._session.graph.get(nodeRef) as Client | null;
    }
    return null;
  }
  readonly clientRef: NodeReference;

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
   * RunEvent.node
   */
  get node(): Run | null {
    const nodeRef: NodeReference | null = this.nodeRef;
    if (nodeRef != null) {
      return this._session.graph.get(nodeRef) as Run | null;
    }
    return null;
  }
  readonly nodeRef: NodeReference;

  /**
   * RunEvent.target
   */
  get target(): Entity | null {
    const nodeRef: NodeReference | null = this.targetRef;
    if (nodeRef != null) {
      return this._session.graph.get(nodeRef) as Entity | null;
    }
    return null;
  }
  readonly targetRef: NodeReference | null;

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
    node: Run | NodeReference;
    target?: Entity | NodeReference | null;
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
        throw new Error(`no active Space for RunPausedEvent`);
      }
      _space = _space.toRef();
    }
    if (_space == null) {
      throw new Error(`RunPausedEvent.space is required`);
    }
    this.spaceRef = _space as NodeReference;
    let _definition = options.definition ?? null;
    if (_definition != null && _definition.constructor.name !== "NodeReference") {
      _definition = (_definition as Node).toRef();
    }
    this.definitionRef = _definition as NodeReference | null;
    let _branch = options.branch ?? null;
    if (_branch != null && _branch.constructor.name !== "NodeReference") {
      _branch = (_branch as Node).toRef();
    }
    if (_branch == null) {
      _branch = ACTIVE_BRANCH.get();
      if (_branch == null) {
        throw new Error(`no active Branch for RunPausedEvent`);
      }
      _branch = _branch.toRef();
    }
    if (_branch == null) {
      throw new Error(`RunPausedEvent.branch is required`);
    }
    this.branchRef = _branch as NodeReference;
    let _snapshot = options.snapshot ?? null;
    if (_snapshot != null && _snapshot.constructor.name !== "NodeReference") {
      _snapshot = (_snapshot as Node).toRef();
    }
    if (_snapshot == null) {
      _snapshot = ACTIVE_SNAPSHOT.get();
      if (_snapshot == null) {
        throw new Error(`no active Snapshot for RunPausedEvent`);
      }
      _snapshot = _snapshot.toRef();
    }
    if (_snapshot == null) {
      throw new Error(`RunPausedEvent.snapshot is required`);
    }
    this.snapshotRef = _snapshot as NodeReference;
    let _precededBy = options.precededBy ?? null;
    if (_precededBy != null && _precededBy.constructor.name !== "NodeReference") {
      _precededBy = (_precededBy as Node).toRef();
    }
    this.precededByRef = _precededBy as NodeReference | null;
    let _causedBy = options.causedBy ?? null;
    if (_causedBy != null && _causedBy.constructor.name !== "NodeReference") {
      _causedBy = (_causedBy as Node).toRef();
    }
    this.causedByRef = _causedBy as NodeReference | null;
    let _client = options.client ?? null;
    if (_client != null && _client.constructor.name !== "NodeReference") {
      _client = (_client as Node).toRef();
    }
    if (_client == null) {
      _client = this._session.clientRef;
    }
    if (_client == null) {
      throw new Error(`RunPausedEvent.client is required`);
    }
    this.clientRef = _client as NodeReference;
    let _clientNonce = options.clientNonce ?? null;
    if (_clientNonce == null) {
      _clientNonce = this._session.clientNonce;
    }
    if (_clientNonce == null) {
      throw new Error(`RunPausedEvent.clientNonce is required`);
    }
    this.clientNonce = _clientNonce;
    let _status = options.status ?? null;
    if (_status == null) {
      _status = 1 /* EventStatus.PENDING */;
    }
    if (_status == null) {
      throw new Error(`RunPausedEvent.status is required`);
    }
    this.status = _status;
    let _node = options.node;
    if (_node != null && _node.constructor.name !== "NodeReference") {
      _node = (_node as Node).toRef();
    }
    if (_node == null) {
      throw new Error(`RunPausedEvent.node is required`);
    }
    this.nodeRef = _node as NodeReference;
    let _target = options.target ?? null;
    if (_target != null && _target.constructor.name !== "NodeReference") {
      _target = (_target as Node).toRef();
    }
    this.targetRef = _target as NodeReference | null;

    /* identity */
    if (options.id == null) {
      const now = Temporal.Now.zonedDateTimeISO("UTC");
      const epoch = this._session.remoteEpoch;
      this.createdAt = now;
      this.createdEpoch = epoch;
      this.createdByRef = this._session.actorRef;
      this.clientCreatedAt = now;
      this.clientEpoch = epoch;
    } else {
      if (
        options.createdAt == null ||
        options.createdEpoch == null ||
        options.clientCreatedAt == null ||
        options.clientEpoch == null
      ) {
        throw new Error(`RunPausedEvent.createdAt is required for existing Events`);
      }
      this.createdAt = options.createdAt;
      this.createdEpoch = options.createdEpoch;
      this.createdByRef =
        options.createdBy != null ? options.createdBy.toRef() : this._session.actorRef;
      this.clientCreatedAt = options.clientCreatedAt;
      this.clientEpoch = options.clientEpoch;
    }
  }

  equals(other: any): boolean {
    if (!(this.metatype === other.metatype)) {
      return false;
    }
    if (!(this.nodeRef.id === other.nodeRef.id)) {
      return false;
    }
    if (!(this.targetRef?.id === other.targetRef?.id)) {
      return false;
    }
    if (!(this.definitionRef?.id === other.definitionRef?.id)) {
      return false;
    }
    if (!(this.branchRef.id === other.branchRef.id)) {
      return false;
    }
    if (!(this.snapshotRef.id === other.snapshotRef.id)) {
      return false;
    }
    if (!(this.precededByRef?.id === other.precededByRef?.id)) {
      return false;
    }
    if (!(this.causedByRef?.id === other.causedByRef?.id)) {
      return false;
    }
    if (!(this.clientRef.id === other.clientRef.id)) {
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
    if (!(this.spaceRef.id === other.spaceRef.id)) {
      return false;
    }
    return true;
  }

  hash(): number {
    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    h = (h * 31 + hashString(this.nodeRef.id)) & 0xffffffff;
    if (this.targetRef != null) {
      h = (h * 31 + hashString(this.targetRef.id)) & 0xffffffff;
    }
    if (this.definitionRef != null) {
      h = (h * 31 + hashString(this.definitionRef.id)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.branchRef.id)) & 0xffffffff;
    h = (h * 31 + hashString(this.snapshotRef.id)) & 0xffffffff;
    if (this.precededByRef != null) {
      h = (h * 31 + hashString(this.precededByRef.id)) & 0xffffffff;
    }
    if (this.causedByRef != null) {
      h = (h * 31 + hashString(this.causedByRef.id)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.createdAt.toString({ timeZoneName: "never" }))) & 0xffffffff;
    h = (h * 31 + hashString(this.createdByRef.id)) & 0xffffffff;
    h = (h * 31 + hashString(this.clientRef.id)) & 0xffffffff;
    h = (h * 31 + hashString(this.clientNonce.toString())) & 0xffffffff;
    h =
      (h * 31 + hashString(this.clientCreatedAt.toString({ timeZoneName: "never" }))) & 0xffffffff;
    h = (h * 31 + hashInt(this.clientEpoch)) & 0xffffffff;
    h = (h * 31 + this.status) & 0xffffffff;
    h = (h * 31 + hashString(this.id.toString())) & 0xffffffff;
    h = (h * 31 + hashString(this.spaceRef.id)) & 0xffffffff;

    return h;
  }

  __toRef__(): NodeReference {
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    return new _NodeReference({
      type: NodeType.RUN_PAUSED_EVENT,
      id: this.id,
      spaceId: this.spaceRef.id,
      definitionId: this.definitionRef?.id ?? null,
      branchId: this.branchRef.id,
      snapshotId: this.snapshotRef.id,
      _session: this._session,
    });
  }

  get _pathKey(): string {
    return `RunPausedEvent[id=${this.id}]`;
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
    return `<RunPausedEvent "${this.path}" ${propertyReprs.join(" ")}>`;
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerNodeClass(NodeType.RUN_PAUSED_EVENT, RunPausedEvent);
/* ==== DESTACK_GENERATED_END:NODE:1110004 ==== */

/* ==== DESTACK_GENERATED_START:NODE:1110005 ==== */
/**
 * A Run was resumed.
 */
export class RunResumeRequestedEvent extends RunEvent {
  static metatype: NodeType = NodeType.RUN_RESUME_REQUESTED_EVENT;

  /**
   * The Space this Node is in.
   */
  get space(): Space | null {
    const nodeRef: NodeReference | null = this.spaceRef;
    if (nodeRef != null) {
      return this._session.graph.get(nodeRef) as Space | null;
    }
    return null;
  }
  readonly spaceRef: NodeReference;

  /**
   * The definition this Event is an instance of.
   */
  get definition(): Entity | null {
    const nodeRef: NodeReference | null = this.definitionRef;
    if (nodeRef != null) {
      return this._session.graph.get(nodeRef) as Entity | null;
    }
    return null;
  }
  readonly definitionRef: NodeReference | null;

  /**
   * The Branch this Event originated from.
   */
  get branch(): Branch | null {
    const nodeRef: NodeReference | null = this.branchRef;
    if (nodeRef != null) {
      return this._session.graph.get(nodeRef) as Branch | null;
    }
    return null;
  }
  readonly branchRef: NodeReference;

  /**
   * The Snapshot this Event originated from.
   */
  get snapshot(): Snapshot | null {
    const nodeRef: NodeReference | null = this.snapshotRef;
    if (nodeRef != null) {
      return this._session.graph.get(nodeRef) as Snapshot | null;
    }
    return null;
  }
  readonly snapshotRef: NodeReference;

  /**
   * The previous Event that this Event follows.
   */
  get precededBy(): Event | null {
    const nodeRef: NodeReference | null = this.precededByRef;
    if (nodeRef != null) {
      return this._session.graph.get(nodeRef) as Event | null;
    }
    return null;
  }
  readonly precededByRef: NodeReference | null;

  /**
   * The Event that caused this Event (if any).
   */
  get causedBy(): Event | null {
    const nodeRef: NodeReference | null = this.causedByRef;
    if (nodeRef != null) {
      return this._session.graph.get(nodeRef) as Event | null;
    }
    return null;
  }
  readonly causedByRef: NodeReference | null;

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
    const nodeRef: NodeReference | null = this.createdByRef;
    if (nodeRef != null) {
      return this._session.graph.get(nodeRef) as Entity | null;
    }
    return null;
  }
  readonly createdByRef: NodeReference;

  /**
   * The Client that created this Event (client).
   */
  get client(): Client | null {
    const nodeRef: NodeReference | null = this.clientRef;
    if (nodeRef != null) {
      return this._session.graph.get(nodeRef) as Client | null;
    }
    return null;
  }
  readonly clientRef: NodeReference;

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
   * RunEvent.node
   */
  get node(): Run | null {
    const nodeRef: NodeReference | null = this.nodeRef;
    if (nodeRef != null) {
      return this._session.graph.get(nodeRef) as Run | null;
    }
    return null;
  }
  readonly nodeRef: NodeReference;

  /**
   * RunEvent.target
   */
  get target(): Entity | null {
    const nodeRef: NodeReference | null = this.targetRef;
    if (nodeRef != null) {
      return this._session.graph.get(nodeRef) as Entity | null;
    }
    return null;
  }
  readonly targetRef: NodeReference | null;

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
    node: Run | NodeReference;
    target?: Entity | NodeReference | null;
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
        throw new Error(`no active Space for RunResumeRequestedEvent`);
      }
      _space = _space.toRef();
    }
    if (_space == null) {
      throw new Error(`RunResumeRequestedEvent.space is required`);
    }
    this.spaceRef = _space as NodeReference;
    let _definition = options.definition ?? null;
    if (_definition != null && _definition.constructor.name !== "NodeReference") {
      _definition = (_definition as Node).toRef();
    }
    this.definitionRef = _definition as NodeReference | null;
    let _branch = options.branch ?? null;
    if (_branch != null && _branch.constructor.name !== "NodeReference") {
      _branch = (_branch as Node).toRef();
    }
    if (_branch == null) {
      _branch = ACTIVE_BRANCH.get();
      if (_branch == null) {
        throw new Error(`no active Branch for RunResumeRequestedEvent`);
      }
      _branch = _branch.toRef();
    }
    if (_branch == null) {
      throw new Error(`RunResumeRequestedEvent.branch is required`);
    }
    this.branchRef = _branch as NodeReference;
    let _snapshot = options.snapshot ?? null;
    if (_snapshot != null && _snapshot.constructor.name !== "NodeReference") {
      _snapshot = (_snapshot as Node).toRef();
    }
    if (_snapshot == null) {
      _snapshot = ACTIVE_SNAPSHOT.get();
      if (_snapshot == null) {
        throw new Error(`no active Snapshot for RunResumeRequestedEvent`);
      }
      _snapshot = _snapshot.toRef();
    }
    if (_snapshot == null) {
      throw new Error(`RunResumeRequestedEvent.snapshot is required`);
    }
    this.snapshotRef = _snapshot as NodeReference;
    let _precededBy = options.precededBy ?? null;
    if (_precededBy != null && _precededBy.constructor.name !== "NodeReference") {
      _precededBy = (_precededBy as Node).toRef();
    }
    this.precededByRef = _precededBy as NodeReference | null;
    let _causedBy = options.causedBy ?? null;
    if (_causedBy != null && _causedBy.constructor.name !== "NodeReference") {
      _causedBy = (_causedBy as Node).toRef();
    }
    this.causedByRef = _causedBy as NodeReference | null;
    let _client = options.client ?? null;
    if (_client != null && _client.constructor.name !== "NodeReference") {
      _client = (_client as Node).toRef();
    }
    if (_client == null) {
      _client = this._session.clientRef;
    }
    if (_client == null) {
      throw new Error(`RunResumeRequestedEvent.client is required`);
    }
    this.clientRef = _client as NodeReference;
    let _clientNonce = options.clientNonce ?? null;
    if (_clientNonce == null) {
      _clientNonce = this._session.clientNonce;
    }
    if (_clientNonce == null) {
      throw new Error(`RunResumeRequestedEvent.clientNonce is required`);
    }
    this.clientNonce = _clientNonce;
    let _status = options.status ?? null;
    if (_status == null) {
      _status = 1 /* EventStatus.PENDING */;
    }
    if (_status == null) {
      throw new Error(`RunResumeRequestedEvent.status is required`);
    }
    this.status = _status;
    let _node = options.node;
    if (_node != null && _node.constructor.name !== "NodeReference") {
      _node = (_node as Node).toRef();
    }
    if (_node == null) {
      throw new Error(`RunResumeRequestedEvent.node is required`);
    }
    this.nodeRef = _node as NodeReference;
    let _target = options.target ?? null;
    if (_target != null && _target.constructor.name !== "NodeReference") {
      _target = (_target as Node).toRef();
    }
    this.targetRef = _target as NodeReference | null;

    /* identity */
    if (options.id == null) {
      const now = Temporal.Now.zonedDateTimeISO("UTC");
      const epoch = this._session.remoteEpoch;
      this.createdAt = now;
      this.createdEpoch = epoch;
      this.createdByRef = this._session.actorRef;
      this.clientCreatedAt = now;
      this.clientEpoch = epoch;
    } else {
      if (
        options.createdAt == null ||
        options.createdEpoch == null ||
        options.clientCreatedAt == null ||
        options.clientEpoch == null
      ) {
        throw new Error(`RunResumeRequestedEvent.createdAt is required for existing Events`);
      }
      this.createdAt = options.createdAt;
      this.createdEpoch = options.createdEpoch;
      this.createdByRef =
        options.createdBy != null ? options.createdBy.toRef() : this._session.actorRef;
      this.clientCreatedAt = options.clientCreatedAt;
      this.clientEpoch = options.clientEpoch;
    }
  }

  equals(other: any): boolean {
    if (!(this.metatype === other.metatype)) {
      return false;
    }
    if (!(this.nodeRef.id === other.nodeRef.id)) {
      return false;
    }
    if (!(this.targetRef?.id === other.targetRef?.id)) {
      return false;
    }
    if (!(this.definitionRef?.id === other.definitionRef?.id)) {
      return false;
    }
    if (!(this.branchRef.id === other.branchRef.id)) {
      return false;
    }
    if (!(this.snapshotRef.id === other.snapshotRef.id)) {
      return false;
    }
    if (!(this.precededByRef?.id === other.precededByRef?.id)) {
      return false;
    }
    if (!(this.causedByRef?.id === other.causedByRef?.id)) {
      return false;
    }
    if (!(this.clientRef.id === other.clientRef.id)) {
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
    if (!(this.spaceRef.id === other.spaceRef.id)) {
      return false;
    }
    return true;
  }

  hash(): number {
    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    h = (h * 31 + hashString(this.nodeRef.id)) & 0xffffffff;
    if (this.targetRef != null) {
      h = (h * 31 + hashString(this.targetRef.id)) & 0xffffffff;
    }
    if (this.definitionRef != null) {
      h = (h * 31 + hashString(this.definitionRef.id)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.branchRef.id)) & 0xffffffff;
    h = (h * 31 + hashString(this.snapshotRef.id)) & 0xffffffff;
    if (this.precededByRef != null) {
      h = (h * 31 + hashString(this.precededByRef.id)) & 0xffffffff;
    }
    if (this.causedByRef != null) {
      h = (h * 31 + hashString(this.causedByRef.id)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.createdAt.toString({ timeZoneName: "never" }))) & 0xffffffff;
    h = (h * 31 + hashString(this.createdByRef.id)) & 0xffffffff;
    h = (h * 31 + hashString(this.clientRef.id)) & 0xffffffff;
    h = (h * 31 + hashString(this.clientNonce.toString())) & 0xffffffff;
    h =
      (h * 31 + hashString(this.clientCreatedAt.toString({ timeZoneName: "never" }))) & 0xffffffff;
    h = (h * 31 + hashInt(this.clientEpoch)) & 0xffffffff;
    h = (h * 31 + this.status) & 0xffffffff;
    h = (h * 31 + hashString(this.id.toString())) & 0xffffffff;
    h = (h * 31 + hashString(this.spaceRef.id)) & 0xffffffff;

    return h;
  }

  __toRef__(): NodeReference {
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    return new _NodeReference({
      type: NodeType.RUN_RESUME_REQUESTED_EVENT,
      id: this.id,
      spaceId: this.spaceRef.id,
      definitionId: this.definitionRef?.id ?? null,
      branchId: this.branchRef.id,
      snapshotId: this.snapshotRef.id,
      _session: this._session,
    });
  }

  get _pathKey(): string {
    return `RunResumeRequestedEvent[id=${this.id}]`;
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
    return `<RunResumeRequestedEvent "${this.path}" ${propertyReprs.join(" ")}>`;
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerNodeClass(NodeType.RUN_RESUME_REQUESTED_EVENT, RunResumeRequestedEvent);
/* ==== DESTACK_GENERATED_END:NODE:1110005 ==== */

/* ==== DESTACK_GENERATED_START:NODE:1110006 ==== */
/**
 * A Run was resumed.
 */
export class RunResumedEvent extends RunEvent {
  static metatype: NodeType = NodeType.RUN_RESUMED_EVENT;

  /**
   * The Space this Node is in.
   */
  get space(): Space | null {
    const nodeRef: NodeReference | null = this.spaceRef;
    if (nodeRef != null) {
      return this._session.graph.get(nodeRef) as Space | null;
    }
    return null;
  }
  readonly spaceRef: NodeReference;

  /**
   * The definition this Event is an instance of.
   */
  get definition(): Entity | null {
    const nodeRef: NodeReference | null = this.definitionRef;
    if (nodeRef != null) {
      return this._session.graph.get(nodeRef) as Entity | null;
    }
    return null;
  }
  readonly definitionRef: NodeReference | null;

  /**
   * The Branch this Event originated from.
   */
  get branch(): Branch | null {
    const nodeRef: NodeReference | null = this.branchRef;
    if (nodeRef != null) {
      return this._session.graph.get(nodeRef) as Branch | null;
    }
    return null;
  }
  readonly branchRef: NodeReference;

  /**
   * The Snapshot this Event originated from.
   */
  get snapshot(): Snapshot | null {
    const nodeRef: NodeReference | null = this.snapshotRef;
    if (nodeRef != null) {
      return this._session.graph.get(nodeRef) as Snapshot | null;
    }
    return null;
  }
  readonly snapshotRef: NodeReference;

  /**
   * The previous Event that this Event follows.
   */
  get precededBy(): Event | null {
    const nodeRef: NodeReference | null = this.precededByRef;
    if (nodeRef != null) {
      return this._session.graph.get(nodeRef) as Event | null;
    }
    return null;
  }
  readonly precededByRef: NodeReference | null;

  /**
   * The Event that caused this Event (if any).
   */
  get causedBy(): Event | null {
    const nodeRef: NodeReference | null = this.causedByRef;
    if (nodeRef != null) {
      return this._session.graph.get(nodeRef) as Event | null;
    }
    return null;
  }
  readonly causedByRef: NodeReference | null;

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
    const nodeRef: NodeReference | null = this.createdByRef;
    if (nodeRef != null) {
      return this._session.graph.get(nodeRef) as Entity | null;
    }
    return null;
  }
  readonly createdByRef: NodeReference;

  /**
   * The Client that created this Event (client).
   */
  get client(): Client | null {
    const nodeRef: NodeReference | null = this.clientRef;
    if (nodeRef != null) {
      return this._session.graph.get(nodeRef) as Client | null;
    }
    return null;
  }
  readonly clientRef: NodeReference;

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
   * RunEvent.node
   */
  get node(): Run | null {
    const nodeRef: NodeReference | null = this.nodeRef;
    if (nodeRef != null) {
      return this._session.graph.get(nodeRef) as Run | null;
    }
    return null;
  }
  readonly nodeRef: NodeReference;

  /**
   * RunEvent.target
   */
  get target(): Entity | null {
    const nodeRef: NodeReference | null = this.targetRef;
    if (nodeRef != null) {
      return this._session.graph.get(nodeRef) as Entity | null;
    }
    return null;
  }
  readonly targetRef: NodeReference | null;

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
    node: Run | NodeReference;
    target?: Entity | NodeReference | null;
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
        throw new Error(`no active Space for RunResumedEvent`);
      }
      _space = _space.toRef();
    }
    if (_space == null) {
      throw new Error(`RunResumedEvent.space is required`);
    }
    this.spaceRef = _space as NodeReference;
    let _definition = options.definition ?? null;
    if (_definition != null && _definition.constructor.name !== "NodeReference") {
      _definition = (_definition as Node).toRef();
    }
    this.definitionRef = _definition as NodeReference | null;
    let _branch = options.branch ?? null;
    if (_branch != null && _branch.constructor.name !== "NodeReference") {
      _branch = (_branch as Node).toRef();
    }
    if (_branch == null) {
      _branch = ACTIVE_BRANCH.get();
      if (_branch == null) {
        throw new Error(`no active Branch for RunResumedEvent`);
      }
      _branch = _branch.toRef();
    }
    if (_branch == null) {
      throw new Error(`RunResumedEvent.branch is required`);
    }
    this.branchRef = _branch as NodeReference;
    let _snapshot = options.snapshot ?? null;
    if (_snapshot != null && _snapshot.constructor.name !== "NodeReference") {
      _snapshot = (_snapshot as Node).toRef();
    }
    if (_snapshot == null) {
      _snapshot = ACTIVE_SNAPSHOT.get();
      if (_snapshot == null) {
        throw new Error(`no active Snapshot for RunResumedEvent`);
      }
      _snapshot = _snapshot.toRef();
    }
    if (_snapshot == null) {
      throw new Error(`RunResumedEvent.snapshot is required`);
    }
    this.snapshotRef = _snapshot as NodeReference;
    let _precededBy = options.precededBy ?? null;
    if (_precededBy != null && _precededBy.constructor.name !== "NodeReference") {
      _precededBy = (_precededBy as Node).toRef();
    }
    this.precededByRef = _precededBy as NodeReference | null;
    let _causedBy = options.causedBy ?? null;
    if (_causedBy != null && _causedBy.constructor.name !== "NodeReference") {
      _causedBy = (_causedBy as Node).toRef();
    }
    this.causedByRef = _causedBy as NodeReference | null;
    let _client = options.client ?? null;
    if (_client != null && _client.constructor.name !== "NodeReference") {
      _client = (_client as Node).toRef();
    }
    if (_client == null) {
      _client = this._session.clientRef;
    }
    if (_client == null) {
      throw new Error(`RunResumedEvent.client is required`);
    }
    this.clientRef = _client as NodeReference;
    let _clientNonce = options.clientNonce ?? null;
    if (_clientNonce == null) {
      _clientNonce = this._session.clientNonce;
    }
    if (_clientNonce == null) {
      throw new Error(`RunResumedEvent.clientNonce is required`);
    }
    this.clientNonce = _clientNonce;
    let _status = options.status ?? null;
    if (_status == null) {
      _status = 1 /* EventStatus.PENDING */;
    }
    if (_status == null) {
      throw new Error(`RunResumedEvent.status is required`);
    }
    this.status = _status;
    let _node = options.node;
    if (_node != null && _node.constructor.name !== "NodeReference") {
      _node = (_node as Node).toRef();
    }
    if (_node == null) {
      throw new Error(`RunResumedEvent.node is required`);
    }
    this.nodeRef = _node as NodeReference;
    let _target = options.target ?? null;
    if (_target != null && _target.constructor.name !== "NodeReference") {
      _target = (_target as Node).toRef();
    }
    this.targetRef = _target as NodeReference | null;

    /* identity */
    if (options.id == null) {
      const now = Temporal.Now.zonedDateTimeISO("UTC");
      const epoch = this._session.remoteEpoch;
      this.createdAt = now;
      this.createdEpoch = epoch;
      this.createdByRef = this._session.actorRef;
      this.clientCreatedAt = now;
      this.clientEpoch = epoch;
    } else {
      if (
        options.createdAt == null ||
        options.createdEpoch == null ||
        options.clientCreatedAt == null ||
        options.clientEpoch == null
      ) {
        throw new Error(`RunResumedEvent.createdAt is required for existing Events`);
      }
      this.createdAt = options.createdAt;
      this.createdEpoch = options.createdEpoch;
      this.createdByRef =
        options.createdBy != null ? options.createdBy.toRef() : this._session.actorRef;
      this.clientCreatedAt = options.clientCreatedAt;
      this.clientEpoch = options.clientEpoch;
    }
  }

  equals(other: any): boolean {
    if (!(this.metatype === other.metatype)) {
      return false;
    }
    if (!(this.nodeRef.id === other.nodeRef.id)) {
      return false;
    }
    if (!(this.targetRef?.id === other.targetRef?.id)) {
      return false;
    }
    if (!(this.definitionRef?.id === other.definitionRef?.id)) {
      return false;
    }
    if (!(this.branchRef.id === other.branchRef.id)) {
      return false;
    }
    if (!(this.snapshotRef.id === other.snapshotRef.id)) {
      return false;
    }
    if (!(this.precededByRef?.id === other.precededByRef?.id)) {
      return false;
    }
    if (!(this.causedByRef?.id === other.causedByRef?.id)) {
      return false;
    }
    if (!(this.clientRef.id === other.clientRef.id)) {
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
    if (!(this.spaceRef.id === other.spaceRef.id)) {
      return false;
    }
    return true;
  }

  hash(): number {
    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    h = (h * 31 + hashString(this.nodeRef.id)) & 0xffffffff;
    if (this.targetRef != null) {
      h = (h * 31 + hashString(this.targetRef.id)) & 0xffffffff;
    }
    if (this.definitionRef != null) {
      h = (h * 31 + hashString(this.definitionRef.id)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.branchRef.id)) & 0xffffffff;
    h = (h * 31 + hashString(this.snapshotRef.id)) & 0xffffffff;
    if (this.precededByRef != null) {
      h = (h * 31 + hashString(this.precededByRef.id)) & 0xffffffff;
    }
    if (this.causedByRef != null) {
      h = (h * 31 + hashString(this.causedByRef.id)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.createdAt.toString({ timeZoneName: "never" }))) & 0xffffffff;
    h = (h * 31 + hashString(this.createdByRef.id)) & 0xffffffff;
    h = (h * 31 + hashString(this.clientRef.id)) & 0xffffffff;
    h = (h * 31 + hashString(this.clientNonce.toString())) & 0xffffffff;
    h =
      (h * 31 + hashString(this.clientCreatedAt.toString({ timeZoneName: "never" }))) & 0xffffffff;
    h = (h * 31 + hashInt(this.clientEpoch)) & 0xffffffff;
    h = (h * 31 + this.status) & 0xffffffff;
    h = (h * 31 + hashString(this.id.toString())) & 0xffffffff;
    h = (h * 31 + hashString(this.spaceRef.id)) & 0xffffffff;

    return h;
  }

  __toRef__(): NodeReference {
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    return new _NodeReference({
      type: NodeType.RUN_RESUMED_EVENT,
      id: this.id,
      spaceId: this.spaceRef.id,
      definitionId: this.definitionRef?.id ?? null,
      branchId: this.branchRef.id,
      snapshotId: this.snapshotRef.id,
      _session: this._session,
    });
  }

  get _pathKey(): string {
    return `RunResumedEvent[id=${this.id}]`;
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
    return `<RunResumedEvent "${this.path}" ${propertyReprs.join(" ")}>`;
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerNodeClass(NodeType.RUN_RESUMED_EVENT, RunResumedEvent);
/* ==== DESTACK_GENERATED_END:NODE:1110006 ==== */

/* ==== DESTACK_GENERATED_START:NODE:1110007 ==== */
/**
 * A Run was stopped.
 */
export class RunStopRequestedEvent extends RunEvent {
  static metatype: NodeType = NodeType.RUN_STOP_REQUESTED_EVENT;

  /**
   * The Space this Node is in.
   */
  get space(): Space | null {
    const nodeRef: NodeReference | null = this.spaceRef;
    if (nodeRef != null) {
      return this._session.graph.get(nodeRef) as Space | null;
    }
    return null;
  }
  readonly spaceRef: NodeReference;

  /**
   * The definition this Event is an instance of.
   */
  get definition(): Entity | null {
    const nodeRef: NodeReference | null = this.definitionRef;
    if (nodeRef != null) {
      return this._session.graph.get(nodeRef) as Entity | null;
    }
    return null;
  }
  readonly definitionRef: NodeReference | null;

  /**
   * The Branch this Event originated from.
   */
  get branch(): Branch | null {
    const nodeRef: NodeReference | null = this.branchRef;
    if (nodeRef != null) {
      return this._session.graph.get(nodeRef) as Branch | null;
    }
    return null;
  }
  readonly branchRef: NodeReference;

  /**
   * The Snapshot this Event originated from.
   */
  get snapshot(): Snapshot | null {
    const nodeRef: NodeReference | null = this.snapshotRef;
    if (nodeRef != null) {
      return this._session.graph.get(nodeRef) as Snapshot | null;
    }
    return null;
  }
  readonly snapshotRef: NodeReference;

  /**
   * The previous Event that this Event follows.
   */
  get precededBy(): Event | null {
    const nodeRef: NodeReference | null = this.precededByRef;
    if (nodeRef != null) {
      return this._session.graph.get(nodeRef) as Event | null;
    }
    return null;
  }
  readonly precededByRef: NodeReference | null;

  /**
   * The Event that caused this Event (if any).
   */
  get causedBy(): Event | null {
    const nodeRef: NodeReference | null = this.causedByRef;
    if (nodeRef != null) {
      return this._session.graph.get(nodeRef) as Event | null;
    }
    return null;
  }
  readonly causedByRef: NodeReference | null;

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
    const nodeRef: NodeReference | null = this.createdByRef;
    if (nodeRef != null) {
      return this._session.graph.get(nodeRef) as Entity | null;
    }
    return null;
  }
  readonly createdByRef: NodeReference;

  /**
   * The Client that created this Event (client).
   */
  get client(): Client | null {
    const nodeRef: NodeReference | null = this.clientRef;
    if (nodeRef != null) {
      return this._session.graph.get(nodeRef) as Client | null;
    }
    return null;
  }
  readonly clientRef: NodeReference;

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
   * RunEvent.node
   */
  get node(): Run | null {
    const nodeRef: NodeReference | null = this.nodeRef;
    if (nodeRef != null) {
      return this._session.graph.get(nodeRef) as Run | null;
    }
    return null;
  }
  readonly nodeRef: NodeReference;

  /**
   * RunEvent.target
   */
  get target(): Entity | null {
    const nodeRef: NodeReference | null = this.targetRef;
    if (nodeRef != null) {
      return this._session.graph.get(nodeRef) as Entity | null;
    }
    return null;
  }
  readonly targetRef: NodeReference | null;

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
    node: Run | NodeReference;
    target?: Entity | NodeReference | null;
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
        throw new Error(`no active Space for RunStopRequestedEvent`);
      }
      _space = _space.toRef();
    }
    if (_space == null) {
      throw new Error(`RunStopRequestedEvent.space is required`);
    }
    this.spaceRef = _space as NodeReference;
    let _definition = options.definition ?? null;
    if (_definition != null && _definition.constructor.name !== "NodeReference") {
      _definition = (_definition as Node).toRef();
    }
    this.definitionRef = _definition as NodeReference | null;
    let _branch = options.branch ?? null;
    if (_branch != null && _branch.constructor.name !== "NodeReference") {
      _branch = (_branch as Node).toRef();
    }
    if (_branch == null) {
      _branch = ACTIVE_BRANCH.get();
      if (_branch == null) {
        throw new Error(`no active Branch for RunStopRequestedEvent`);
      }
      _branch = _branch.toRef();
    }
    if (_branch == null) {
      throw new Error(`RunStopRequestedEvent.branch is required`);
    }
    this.branchRef = _branch as NodeReference;
    let _snapshot = options.snapshot ?? null;
    if (_snapshot != null && _snapshot.constructor.name !== "NodeReference") {
      _snapshot = (_snapshot as Node).toRef();
    }
    if (_snapshot == null) {
      _snapshot = ACTIVE_SNAPSHOT.get();
      if (_snapshot == null) {
        throw new Error(`no active Snapshot for RunStopRequestedEvent`);
      }
      _snapshot = _snapshot.toRef();
    }
    if (_snapshot == null) {
      throw new Error(`RunStopRequestedEvent.snapshot is required`);
    }
    this.snapshotRef = _snapshot as NodeReference;
    let _precededBy = options.precededBy ?? null;
    if (_precededBy != null && _precededBy.constructor.name !== "NodeReference") {
      _precededBy = (_precededBy as Node).toRef();
    }
    this.precededByRef = _precededBy as NodeReference | null;
    let _causedBy = options.causedBy ?? null;
    if (_causedBy != null && _causedBy.constructor.name !== "NodeReference") {
      _causedBy = (_causedBy as Node).toRef();
    }
    this.causedByRef = _causedBy as NodeReference | null;
    let _client = options.client ?? null;
    if (_client != null && _client.constructor.name !== "NodeReference") {
      _client = (_client as Node).toRef();
    }
    if (_client == null) {
      _client = this._session.clientRef;
    }
    if (_client == null) {
      throw new Error(`RunStopRequestedEvent.client is required`);
    }
    this.clientRef = _client as NodeReference;
    let _clientNonce = options.clientNonce ?? null;
    if (_clientNonce == null) {
      _clientNonce = this._session.clientNonce;
    }
    if (_clientNonce == null) {
      throw new Error(`RunStopRequestedEvent.clientNonce is required`);
    }
    this.clientNonce = _clientNonce;
    let _status = options.status ?? null;
    if (_status == null) {
      _status = 1 /* EventStatus.PENDING */;
    }
    if (_status == null) {
      throw new Error(`RunStopRequestedEvent.status is required`);
    }
    this.status = _status;
    let _node = options.node;
    if (_node != null && _node.constructor.name !== "NodeReference") {
      _node = (_node as Node).toRef();
    }
    if (_node == null) {
      throw new Error(`RunStopRequestedEvent.node is required`);
    }
    this.nodeRef = _node as NodeReference;
    let _target = options.target ?? null;
    if (_target != null && _target.constructor.name !== "NodeReference") {
      _target = (_target as Node).toRef();
    }
    this.targetRef = _target as NodeReference | null;

    /* identity */
    if (options.id == null) {
      const now = Temporal.Now.zonedDateTimeISO("UTC");
      const epoch = this._session.remoteEpoch;
      this.createdAt = now;
      this.createdEpoch = epoch;
      this.createdByRef = this._session.actorRef;
      this.clientCreatedAt = now;
      this.clientEpoch = epoch;
    } else {
      if (
        options.createdAt == null ||
        options.createdEpoch == null ||
        options.clientCreatedAt == null ||
        options.clientEpoch == null
      ) {
        throw new Error(`RunStopRequestedEvent.createdAt is required for existing Events`);
      }
      this.createdAt = options.createdAt;
      this.createdEpoch = options.createdEpoch;
      this.createdByRef =
        options.createdBy != null ? options.createdBy.toRef() : this._session.actorRef;
      this.clientCreatedAt = options.clientCreatedAt;
      this.clientEpoch = options.clientEpoch;
    }
  }

  equals(other: any): boolean {
    if (!(this.metatype === other.metatype)) {
      return false;
    }
    if (!(this.nodeRef.id === other.nodeRef.id)) {
      return false;
    }
    if (!(this.targetRef?.id === other.targetRef?.id)) {
      return false;
    }
    if (!(this.definitionRef?.id === other.definitionRef?.id)) {
      return false;
    }
    if (!(this.branchRef.id === other.branchRef.id)) {
      return false;
    }
    if (!(this.snapshotRef.id === other.snapshotRef.id)) {
      return false;
    }
    if (!(this.precededByRef?.id === other.precededByRef?.id)) {
      return false;
    }
    if (!(this.causedByRef?.id === other.causedByRef?.id)) {
      return false;
    }
    if (!(this.clientRef.id === other.clientRef.id)) {
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
    if (!(this.spaceRef.id === other.spaceRef.id)) {
      return false;
    }
    return true;
  }

  hash(): number {
    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    h = (h * 31 + hashString(this.nodeRef.id)) & 0xffffffff;
    if (this.targetRef != null) {
      h = (h * 31 + hashString(this.targetRef.id)) & 0xffffffff;
    }
    if (this.definitionRef != null) {
      h = (h * 31 + hashString(this.definitionRef.id)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.branchRef.id)) & 0xffffffff;
    h = (h * 31 + hashString(this.snapshotRef.id)) & 0xffffffff;
    if (this.precededByRef != null) {
      h = (h * 31 + hashString(this.precededByRef.id)) & 0xffffffff;
    }
    if (this.causedByRef != null) {
      h = (h * 31 + hashString(this.causedByRef.id)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.createdAt.toString({ timeZoneName: "never" }))) & 0xffffffff;
    h = (h * 31 + hashString(this.createdByRef.id)) & 0xffffffff;
    h = (h * 31 + hashString(this.clientRef.id)) & 0xffffffff;
    h = (h * 31 + hashString(this.clientNonce.toString())) & 0xffffffff;
    h =
      (h * 31 + hashString(this.clientCreatedAt.toString({ timeZoneName: "never" }))) & 0xffffffff;
    h = (h * 31 + hashInt(this.clientEpoch)) & 0xffffffff;
    h = (h * 31 + this.status) & 0xffffffff;
    h = (h * 31 + hashString(this.id.toString())) & 0xffffffff;
    h = (h * 31 + hashString(this.spaceRef.id)) & 0xffffffff;

    return h;
  }

  __toRef__(): NodeReference {
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    return new _NodeReference({
      type: NodeType.RUN_STOP_REQUESTED_EVENT,
      id: this.id,
      spaceId: this.spaceRef.id,
      definitionId: this.definitionRef?.id ?? null,
      branchId: this.branchRef.id,
      snapshotId: this.snapshotRef.id,
      _session: this._session,
    });
  }

  get _pathKey(): string {
    return `RunStopRequestedEvent[id=${this.id}]`;
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
    return `<RunStopRequestedEvent "${this.path}" ${propertyReprs.join(" ")}>`;
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerNodeClass(NodeType.RUN_STOP_REQUESTED_EVENT, RunStopRequestedEvent);
/* ==== DESTACK_GENERATED_END:NODE:1110007 ==== */

/* ==== DESTACK_GENERATED_START:NODE:1110008 ==== */
/**
 * A Run failed.
 */
export class RunFailedEvent extends RunEvent {
  static metatype: NodeType = NodeType.RUN_FAILED_EVENT;

  /**
   * The Space this Node is in.
   */
  get space(): Space | null {
    const nodeRef: NodeReference | null = this.spaceRef;
    if (nodeRef != null) {
      return this._session.graph.get(nodeRef) as Space | null;
    }
    return null;
  }
  readonly spaceRef: NodeReference;

  /**
   * The definition this Event is an instance of.
   */
  get definition(): Entity | null {
    const nodeRef: NodeReference | null = this.definitionRef;
    if (nodeRef != null) {
      return this._session.graph.get(nodeRef) as Entity | null;
    }
    return null;
  }
  readonly definitionRef: NodeReference | null;

  /**
   * The Branch this Event originated from.
   */
  get branch(): Branch | null {
    const nodeRef: NodeReference | null = this.branchRef;
    if (nodeRef != null) {
      return this._session.graph.get(nodeRef) as Branch | null;
    }
    return null;
  }
  readonly branchRef: NodeReference;

  /**
   * The Snapshot this Event originated from.
   */
  get snapshot(): Snapshot | null {
    const nodeRef: NodeReference | null = this.snapshotRef;
    if (nodeRef != null) {
      return this._session.graph.get(nodeRef) as Snapshot | null;
    }
    return null;
  }
  readonly snapshotRef: NodeReference;

  /**
   * The previous Event that this Event follows.
   */
  get precededBy(): Event | null {
    const nodeRef: NodeReference | null = this.precededByRef;
    if (nodeRef != null) {
      return this._session.graph.get(nodeRef) as Event | null;
    }
    return null;
  }
  readonly precededByRef: NodeReference | null;

  /**
   * The Event that caused this Event (if any).
   */
  get causedBy(): Event | null {
    const nodeRef: NodeReference | null = this.causedByRef;
    if (nodeRef != null) {
      return this._session.graph.get(nodeRef) as Event | null;
    }
    return null;
  }
  readonly causedByRef: NodeReference | null;

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
    const nodeRef: NodeReference | null = this.createdByRef;
    if (nodeRef != null) {
      return this._session.graph.get(nodeRef) as Entity | null;
    }
    return null;
  }
  readonly createdByRef: NodeReference;

  /**
   * The Client that created this Event (client).
   */
  get client(): Client | null {
    const nodeRef: NodeReference | null = this.clientRef;
    if (nodeRef != null) {
      return this._session.graph.get(nodeRef) as Client | null;
    }
    return null;
  }
  readonly clientRef: NodeReference;

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
   * RunEvent.node
   */
  get node(): Run | null {
    const nodeRef: NodeReference | null = this.nodeRef;
    if (nodeRef != null) {
      return this._session.graph.get(nodeRef) as Run | null;
    }
    return null;
  }
  readonly nodeRef: NodeReference;

  /**
   * RunEvent.target
   */
  get target(): Entity | null {
    const nodeRef: NodeReference | null = this.targetRef;
    if (nodeRef != null) {
      return this._session.graph.get(nodeRef) as Entity | null;
    }
    return null;
  }
  readonly targetRef: NodeReference | null;

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
    node: Run | NodeReference;
    target?: Entity | NodeReference | null;
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
        throw new Error(`no active Space for RunFailedEvent`);
      }
      _space = _space.toRef();
    }
    if (_space == null) {
      throw new Error(`RunFailedEvent.space is required`);
    }
    this.spaceRef = _space as NodeReference;
    let _definition = options.definition ?? null;
    if (_definition != null && _definition.constructor.name !== "NodeReference") {
      _definition = (_definition as Node).toRef();
    }
    this.definitionRef = _definition as NodeReference | null;
    let _branch = options.branch ?? null;
    if (_branch != null && _branch.constructor.name !== "NodeReference") {
      _branch = (_branch as Node).toRef();
    }
    if (_branch == null) {
      _branch = ACTIVE_BRANCH.get();
      if (_branch == null) {
        throw new Error(`no active Branch for RunFailedEvent`);
      }
      _branch = _branch.toRef();
    }
    if (_branch == null) {
      throw new Error(`RunFailedEvent.branch is required`);
    }
    this.branchRef = _branch as NodeReference;
    let _snapshot = options.snapshot ?? null;
    if (_snapshot != null && _snapshot.constructor.name !== "NodeReference") {
      _snapshot = (_snapshot as Node).toRef();
    }
    if (_snapshot == null) {
      _snapshot = ACTIVE_SNAPSHOT.get();
      if (_snapshot == null) {
        throw new Error(`no active Snapshot for RunFailedEvent`);
      }
      _snapshot = _snapshot.toRef();
    }
    if (_snapshot == null) {
      throw new Error(`RunFailedEvent.snapshot is required`);
    }
    this.snapshotRef = _snapshot as NodeReference;
    let _precededBy = options.precededBy ?? null;
    if (_precededBy != null && _precededBy.constructor.name !== "NodeReference") {
      _precededBy = (_precededBy as Node).toRef();
    }
    this.precededByRef = _precededBy as NodeReference | null;
    let _causedBy = options.causedBy ?? null;
    if (_causedBy != null && _causedBy.constructor.name !== "NodeReference") {
      _causedBy = (_causedBy as Node).toRef();
    }
    this.causedByRef = _causedBy as NodeReference | null;
    let _client = options.client ?? null;
    if (_client != null && _client.constructor.name !== "NodeReference") {
      _client = (_client as Node).toRef();
    }
    if (_client == null) {
      _client = this._session.clientRef;
    }
    if (_client == null) {
      throw new Error(`RunFailedEvent.client is required`);
    }
    this.clientRef = _client as NodeReference;
    let _clientNonce = options.clientNonce ?? null;
    if (_clientNonce == null) {
      _clientNonce = this._session.clientNonce;
    }
    if (_clientNonce == null) {
      throw new Error(`RunFailedEvent.clientNonce is required`);
    }
    this.clientNonce = _clientNonce;
    let _status = options.status ?? null;
    if (_status == null) {
      _status = 1 /* EventStatus.PENDING */;
    }
    if (_status == null) {
      throw new Error(`RunFailedEvent.status is required`);
    }
    this.status = _status;
    let _node = options.node;
    if (_node != null && _node.constructor.name !== "NodeReference") {
      _node = (_node as Node).toRef();
    }
    if (_node == null) {
      throw new Error(`RunFailedEvent.node is required`);
    }
    this.nodeRef = _node as NodeReference;
    let _target = options.target ?? null;
    if (_target != null && _target.constructor.name !== "NodeReference") {
      _target = (_target as Node).toRef();
    }
    this.targetRef = _target as NodeReference | null;

    /* identity */
    if (options.id == null) {
      const now = Temporal.Now.zonedDateTimeISO("UTC");
      const epoch = this._session.remoteEpoch;
      this.createdAt = now;
      this.createdEpoch = epoch;
      this.createdByRef = this._session.actorRef;
      this.clientCreatedAt = now;
      this.clientEpoch = epoch;
    } else {
      if (
        options.createdAt == null ||
        options.createdEpoch == null ||
        options.clientCreatedAt == null ||
        options.clientEpoch == null
      ) {
        throw new Error(`RunFailedEvent.createdAt is required for existing Events`);
      }
      this.createdAt = options.createdAt;
      this.createdEpoch = options.createdEpoch;
      this.createdByRef =
        options.createdBy != null ? options.createdBy.toRef() : this._session.actorRef;
      this.clientCreatedAt = options.clientCreatedAt;
      this.clientEpoch = options.clientEpoch;
    }
  }

  equals(other: any): boolean {
    if (!(this.metatype === other.metatype)) {
      return false;
    }
    if (!(this.nodeRef.id === other.nodeRef.id)) {
      return false;
    }
    if (!(this.targetRef?.id === other.targetRef?.id)) {
      return false;
    }
    if (!(this.definitionRef?.id === other.definitionRef?.id)) {
      return false;
    }
    if (!(this.branchRef.id === other.branchRef.id)) {
      return false;
    }
    if (!(this.snapshotRef.id === other.snapshotRef.id)) {
      return false;
    }
    if (!(this.precededByRef?.id === other.precededByRef?.id)) {
      return false;
    }
    if (!(this.causedByRef?.id === other.causedByRef?.id)) {
      return false;
    }
    if (!(this.clientRef.id === other.clientRef.id)) {
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
    if (!(this.spaceRef.id === other.spaceRef.id)) {
      return false;
    }
    return true;
  }

  hash(): number {
    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    h = (h * 31 + hashString(this.nodeRef.id)) & 0xffffffff;
    if (this.targetRef != null) {
      h = (h * 31 + hashString(this.targetRef.id)) & 0xffffffff;
    }
    if (this.definitionRef != null) {
      h = (h * 31 + hashString(this.definitionRef.id)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.branchRef.id)) & 0xffffffff;
    h = (h * 31 + hashString(this.snapshotRef.id)) & 0xffffffff;
    if (this.precededByRef != null) {
      h = (h * 31 + hashString(this.precededByRef.id)) & 0xffffffff;
    }
    if (this.causedByRef != null) {
      h = (h * 31 + hashString(this.causedByRef.id)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.createdAt.toString({ timeZoneName: "never" }))) & 0xffffffff;
    h = (h * 31 + hashString(this.createdByRef.id)) & 0xffffffff;
    h = (h * 31 + hashString(this.clientRef.id)) & 0xffffffff;
    h = (h * 31 + hashString(this.clientNonce.toString())) & 0xffffffff;
    h =
      (h * 31 + hashString(this.clientCreatedAt.toString({ timeZoneName: "never" }))) & 0xffffffff;
    h = (h * 31 + hashInt(this.clientEpoch)) & 0xffffffff;
    h = (h * 31 + this.status) & 0xffffffff;
    h = (h * 31 + hashString(this.id.toString())) & 0xffffffff;
    h = (h * 31 + hashString(this.spaceRef.id)) & 0xffffffff;

    return h;
  }

  __toRef__(): NodeReference {
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    return new _NodeReference({
      type: NodeType.RUN_FAILED_EVENT,
      id: this.id,
      spaceId: this.spaceRef.id,
      definitionId: this.definitionRef?.id ?? null,
      branchId: this.branchRef.id,
      snapshotId: this.snapshotRef.id,
      _session: this._session,
    });
  }

  get _pathKey(): string {
    return `RunFailedEvent[id=${this.id}]`;
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
    return `<RunFailedEvent "${this.path}" ${propertyReprs.join(" ")}>`;
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerNodeClass(NodeType.RUN_FAILED_EVENT, RunFailedEvent);
/* ==== DESTACK_GENERATED_END:NODE:1110008 ==== */

/* ==== DESTACK_GENERATED_START:NODE:1110009 ==== */
/**
 * A Run completed.
 */
export class RunCompletedEvent extends RunEvent {
  static metatype: NodeType = NodeType.RUN_COMPLETED_EVENT;

  /**
   * The Space this Node is in.
   */
  get space(): Space | null {
    const nodeRef: NodeReference | null = this.spaceRef;
    if (nodeRef != null) {
      return this._session.graph.get(nodeRef) as Space | null;
    }
    return null;
  }
  readonly spaceRef: NodeReference;

  /**
   * The definition this Event is an instance of.
   */
  get definition(): Entity | null {
    const nodeRef: NodeReference | null = this.definitionRef;
    if (nodeRef != null) {
      return this._session.graph.get(nodeRef) as Entity | null;
    }
    return null;
  }
  readonly definitionRef: NodeReference | null;

  /**
   * The Branch this Event originated from.
   */
  get branch(): Branch | null {
    const nodeRef: NodeReference | null = this.branchRef;
    if (nodeRef != null) {
      return this._session.graph.get(nodeRef) as Branch | null;
    }
    return null;
  }
  readonly branchRef: NodeReference;

  /**
   * The Snapshot this Event originated from.
   */
  get snapshot(): Snapshot | null {
    const nodeRef: NodeReference | null = this.snapshotRef;
    if (nodeRef != null) {
      return this._session.graph.get(nodeRef) as Snapshot | null;
    }
    return null;
  }
  readonly snapshotRef: NodeReference;

  /**
   * The previous Event that this Event follows.
   */
  get precededBy(): Event | null {
    const nodeRef: NodeReference | null = this.precededByRef;
    if (nodeRef != null) {
      return this._session.graph.get(nodeRef) as Event | null;
    }
    return null;
  }
  readonly precededByRef: NodeReference | null;

  /**
   * The Event that caused this Event (if any).
   */
  get causedBy(): Event | null {
    const nodeRef: NodeReference | null = this.causedByRef;
    if (nodeRef != null) {
      return this._session.graph.get(nodeRef) as Event | null;
    }
    return null;
  }
  readonly causedByRef: NodeReference | null;

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
    const nodeRef: NodeReference | null = this.createdByRef;
    if (nodeRef != null) {
      return this._session.graph.get(nodeRef) as Entity | null;
    }
    return null;
  }
  readonly createdByRef: NodeReference;

  /**
   * The Client that created this Event (client).
   */
  get client(): Client | null {
    const nodeRef: NodeReference | null = this.clientRef;
    if (nodeRef != null) {
      return this._session.graph.get(nodeRef) as Client | null;
    }
    return null;
  }
  readonly clientRef: NodeReference;

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
   * RunEvent.node
   */
  get node(): Run | null {
    const nodeRef: NodeReference | null = this.nodeRef;
    if (nodeRef != null) {
      return this._session.graph.get(nodeRef) as Run | null;
    }
    return null;
  }
  readonly nodeRef: NodeReference;

  /**
   * RunEvent.target
   */
  get target(): Entity | null {
    const nodeRef: NodeReference | null = this.targetRef;
    if (nodeRef != null) {
      return this._session.graph.get(nodeRef) as Entity | null;
    }
    return null;
  }
  readonly targetRef: NodeReference | null;

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
    node: Run | NodeReference;
    target?: Entity | NodeReference | null;
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
        throw new Error(`no active Space for RunCompletedEvent`);
      }
      _space = _space.toRef();
    }
    if (_space == null) {
      throw new Error(`RunCompletedEvent.space is required`);
    }
    this.spaceRef = _space as NodeReference;
    let _definition = options.definition ?? null;
    if (_definition != null && _definition.constructor.name !== "NodeReference") {
      _definition = (_definition as Node).toRef();
    }
    this.definitionRef = _definition as NodeReference | null;
    let _branch = options.branch ?? null;
    if (_branch != null && _branch.constructor.name !== "NodeReference") {
      _branch = (_branch as Node).toRef();
    }
    if (_branch == null) {
      _branch = ACTIVE_BRANCH.get();
      if (_branch == null) {
        throw new Error(`no active Branch for RunCompletedEvent`);
      }
      _branch = _branch.toRef();
    }
    if (_branch == null) {
      throw new Error(`RunCompletedEvent.branch is required`);
    }
    this.branchRef = _branch as NodeReference;
    let _snapshot = options.snapshot ?? null;
    if (_snapshot != null && _snapshot.constructor.name !== "NodeReference") {
      _snapshot = (_snapshot as Node).toRef();
    }
    if (_snapshot == null) {
      _snapshot = ACTIVE_SNAPSHOT.get();
      if (_snapshot == null) {
        throw new Error(`no active Snapshot for RunCompletedEvent`);
      }
      _snapshot = _snapshot.toRef();
    }
    if (_snapshot == null) {
      throw new Error(`RunCompletedEvent.snapshot is required`);
    }
    this.snapshotRef = _snapshot as NodeReference;
    let _precededBy = options.precededBy ?? null;
    if (_precededBy != null && _precededBy.constructor.name !== "NodeReference") {
      _precededBy = (_precededBy as Node).toRef();
    }
    this.precededByRef = _precededBy as NodeReference | null;
    let _causedBy = options.causedBy ?? null;
    if (_causedBy != null && _causedBy.constructor.name !== "NodeReference") {
      _causedBy = (_causedBy as Node).toRef();
    }
    this.causedByRef = _causedBy as NodeReference | null;
    let _client = options.client ?? null;
    if (_client != null && _client.constructor.name !== "NodeReference") {
      _client = (_client as Node).toRef();
    }
    if (_client == null) {
      _client = this._session.clientRef;
    }
    if (_client == null) {
      throw new Error(`RunCompletedEvent.client is required`);
    }
    this.clientRef = _client as NodeReference;
    let _clientNonce = options.clientNonce ?? null;
    if (_clientNonce == null) {
      _clientNonce = this._session.clientNonce;
    }
    if (_clientNonce == null) {
      throw new Error(`RunCompletedEvent.clientNonce is required`);
    }
    this.clientNonce = _clientNonce;
    let _status = options.status ?? null;
    if (_status == null) {
      _status = 1 /* EventStatus.PENDING */;
    }
    if (_status == null) {
      throw new Error(`RunCompletedEvent.status is required`);
    }
    this.status = _status;
    let _node = options.node;
    if (_node != null && _node.constructor.name !== "NodeReference") {
      _node = (_node as Node).toRef();
    }
    if (_node == null) {
      throw new Error(`RunCompletedEvent.node is required`);
    }
    this.nodeRef = _node as NodeReference;
    let _target = options.target ?? null;
    if (_target != null && _target.constructor.name !== "NodeReference") {
      _target = (_target as Node).toRef();
    }
    this.targetRef = _target as NodeReference | null;

    /* identity */
    if (options.id == null) {
      const now = Temporal.Now.zonedDateTimeISO("UTC");
      const epoch = this._session.remoteEpoch;
      this.createdAt = now;
      this.createdEpoch = epoch;
      this.createdByRef = this._session.actorRef;
      this.clientCreatedAt = now;
      this.clientEpoch = epoch;
    } else {
      if (
        options.createdAt == null ||
        options.createdEpoch == null ||
        options.clientCreatedAt == null ||
        options.clientEpoch == null
      ) {
        throw new Error(`RunCompletedEvent.createdAt is required for existing Events`);
      }
      this.createdAt = options.createdAt;
      this.createdEpoch = options.createdEpoch;
      this.createdByRef =
        options.createdBy != null ? options.createdBy.toRef() : this._session.actorRef;
      this.clientCreatedAt = options.clientCreatedAt;
      this.clientEpoch = options.clientEpoch;
    }
  }

  equals(other: any): boolean {
    if (!(this.metatype === other.metatype)) {
      return false;
    }
    if (!(this.nodeRef.id === other.nodeRef.id)) {
      return false;
    }
    if (!(this.targetRef?.id === other.targetRef?.id)) {
      return false;
    }
    if (!(this.definitionRef?.id === other.definitionRef?.id)) {
      return false;
    }
    if (!(this.branchRef.id === other.branchRef.id)) {
      return false;
    }
    if (!(this.snapshotRef.id === other.snapshotRef.id)) {
      return false;
    }
    if (!(this.precededByRef?.id === other.precededByRef?.id)) {
      return false;
    }
    if (!(this.causedByRef?.id === other.causedByRef?.id)) {
      return false;
    }
    if (!(this.clientRef.id === other.clientRef.id)) {
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
    if (!(this.spaceRef.id === other.spaceRef.id)) {
      return false;
    }
    return true;
  }

  hash(): number {
    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    h = (h * 31 + hashString(this.nodeRef.id)) & 0xffffffff;
    if (this.targetRef != null) {
      h = (h * 31 + hashString(this.targetRef.id)) & 0xffffffff;
    }
    if (this.definitionRef != null) {
      h = (h * 31 + hashString(this.definitionRef.id)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.branchRef.id)) & 0xffffffff;
    h = (h * 31 + hashString(this.snapshotRef.id)) & 0xffffffff;
    if (this.precededByRef != null) {
      h = (h * 31 + hashString(this.precededByRef.id)) & 0xffffffff;
    }
    if (this.causedByRef != null) {
      h = (h * 31 + hashString(this.causedByRef.id)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.createdAt.toString({ timeZoneName: "never" }))) & 0xffffffff;
    h = (h * 31 + hashString(this.createdByRef.id)) & 0xffffffff;
    h = (h * 31 + hashString(this.clientRef.id)) & 0xffffffff;
    h = (h * 31 + hashString(this.clientNonce.toString())) & 0xffffffff;
    h =
      (h * 31 + hashString(this.clientCreatedAt.toString({ timeZoneName: "never" }))) & 0xffffffff;
    h = (h * 31 + hashInt(this.clientEpoch)) & 0xffffffff;
    h = (h * 31 + this.status) & 0xffffffff;
    h = (h * 31 + hashString(this.id.toString())) & 0xffffffff;
    h = (h * 31 + hashString(this.spaceRef.id)) & 0xffffffff;

    return h;
  }

  __toRef__(): NodeReference {
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    return new _NodeReference({
      type: NodeType.RUN_COMPLETED_EVENT,
      id: this.id,
      spaceId: this.spaceRef.id,
      definitionId: this.definitionRef?.id ?? null,
      branchId: this.branchRef.id,
      snapshotId: this.snapshotRef.id,
      _session: this._session,
    });
  }

  get _pathKey(): string {
    return `RunCompletedEvent[id=${this.id}]`;
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
    return `<RunCompletedEvent "${this.path}" ${propertyReprs.join(" ")}>`;
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerNodeClass(NodeType.RUN_COMPLETED_EVENT, RunCompletedEvent);
/* ==== DESTACK_GENERATED_END:NODE:1110009 ==== */

/* ==== DESTACK_GENERATED_START:NODE:1110000 ==== */
/**
 * Run of a Runnable.
 */
export abstract class Run extends Entity {
  static metatype: NodeType = NodeType.RUN;

  /**
   * Run.parent
   */
  abstract get parent(): Space | Run | null;
  declare readonly parentRef: NodeReference | null;

  /**
   * The Space this Node is in.
   */
  abstract get space(): Space | null;
  declare readonly spaceRef: NodeReference;

  /**
   * Entity.materialization
   */
  declare readonly materialization: Materialization;

  /**
   * The definition this Entity is an instance of.
   */
  abstract get definition(): Entity | null;
  declare readonly definitionRef: NodeReference | null;

  /**
   * The Branch this Entity is part of.
   */
  abstract get branch(): Branch | null;
  declare readonly branchRef: NodeReference;

  /**
   * The Snapshot this Entity is part of.
   */
  abstract get snapshot(): Snapshot | null;
  declare readonly snapshotRef: NodeReference;

  /**
   * The previous Entity this Entity is based on (from the base Branch, if any).
   * This invariant must hold: `Entity.preceded_by.branch == Entity.branch.preceded_by`.
   */
  abstract get precededBy(): Run | null;
  declare readonly precededByRef: NodeReference | null;

  /**
   * The (root) Entity that is being instantiated.
   */
  abstract get instance(): Entity | null;
  declare readonly instanceRef: NodeReference | null;

  /**
   * The time this Entity was created (system time).
   */
  declare readonly createdAt: Datetime;

  /**
   * The logical time this Entity was created (system time).
   */
  declare readonly createdEpoch: UInt128;

  /**
   * The Actor that created this Entity.
   */
  abstract get createdBy(): Entity | null;
  declare readonly createdByRef: NodeReference;

  /**
   * The time this Entity was last updated (system time).
   */
  declare readonly updatedAt: Datetime;

  /**
   * The logical time this Entity was last updated (system time).
   */
  declare readonly updatedEpoch: UInt128;

  /**
   * The Actor that last updated this Entity.
   */
  abstract get updatedBy(): Entity | null;
  declare readonly updatedByRef: NodeReference;

  /**
   * The time this Entity was deleted (system time).
   * Only set if the Entity is currently 'deleted'.
   * Deleting and restoring an Entity counts as an update, and thus updates updated_at/updated_epoch.
   */
  declare readonly deletedAt: Datetime | null;

  /**
   * Entity.ownedBy
   */
  abstract get ownedBy(): Entity | null;
  abstract set ownedBy(value: Entity | null);
  /**
   * Entity.ownedBy
   */
  abstract get ownedByRef(): NodeReference | null;
  abstract set ownedByRef(value: NodeReference | null);

  /**
   * Entity.name
   */
  /**
   * Entity.name
   */
  abstract get name(): string;
  abstract set name(value: string);

  /**
   * The absolute order key of this Entity in its parent.
   */
  declare readonly orderKey: string;

  /**
   * The custom Values of this Entity, keyed by custom Property id..
   */
  /**
   * The custom Values of this Entity, keyed by custom Property id..
   */
  abstract get customValues(): { readonly [key: UUID]: Value };
  abstract set customValues(value: { readonly [key: UUID]: Value });

  /**
   * The Script of this Entity.
   */
  abstract get script(): Script | null;
  abstract set script(value: Script | null);
  /**
   * The Script of this Entity.
   */
  abstract get scriptRef(): NodeReference | null;
  abstract set scriptRef(value: NodeReference | null);

  /**
   * Whether this Entity can be instanced.
   */
  declare readonly isExtensible: boolean | null;

  /**
   * The Script that defines this Node.
   */
  abstract get source(): Script | null;
  declare readonly sourceRef: NodeReference | null;

  /**
   * The key to uniquely identify this Node in reconciliation. If not set, name is used.
   */
  /**
   * The key to uniquely identify this Node in reconciliation. If not set, name is used.
   */
  abstract get key(): string | null;
  abstract set key(value: string | null);

  /**
   * Run.target
   */
  abstract get target(): Entity | null;
  abstract set target(value: Entity | null);
  /**
   * Run.target
   */
  abstract get targetRef(): NodeReference | null;
  abstract set targetRef(value: NodeReference | null);

  /**
   * Run.status
   */
  /**
   * Run.status
   */
  abstract get status(): RunStatus;
  abstract set status(value: RunStatus);

  /**
   * Duration from first attempt start to last attempt termination.
   */
  /**
   * Duration from first attempt start to last attempt termination.
   */
  abstract get duration(): Duration | null;
  abstract set duration(value: Duration | null);

  /**
   * When the Run is scheduled to start.
   */
  /**
   * When the Run is scheduled to start.
   */
  abstract get scheduledAt(): Datetime | null;
  abstract set scheduledAt(value: Datetime | null);

  /**
   * When the Run first started.
   */
  /**
   * When the Run first started.
   */
  abstract get startedAt(): Datetime | null;
  abstract set startedAt(value: Datetime | null);

  /**
   * When the Run was last active.
   */
  /**
   * When the Run was last active.
   */
  abstract get seenAt(): Datetime | null;
  abstract set seenAt(value: Datetime | null);

  /**
   * When the Run was interrupted.
   */
  /**
   * When the Run was interrupted.
   */
  abstract get interruptedAt(): Datetime | null;
  abstract set interruptedAt(value: Datetime | null);

  /**
   * When the Run was last terminated.
   */
  /**
   * When the Run was last terminated.
   */
  abstract get terminatedAt(): Datetime | null;
  abstract set terminatedAt(value: Datetime | null);

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerNodeClass(NodeType.RUN, Run);
/* ==== DESTACK_GENERATED_END:NODE:1110000 ==== */
