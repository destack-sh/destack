import type {
  Branch,
  Datetime,
  NodeReference,
  Session,
  Snapshot,
  Space,
  UInt128,
  UUID,
} from "@destack/language/core";
import {
  ACTIVE_BRANCH,
  ACTIVE_SNAPSHOT,
  ACTIVE_SPACE,
  type Entity,
  Event,
  EventStatus,
  type Node,
  NodeType,
  StructType,
} from "@destack/language/core";
import type { Run } from "@destack/language/deployment/run";
import { registerNodeClass, STRUCT_CLASS_BY_TYPE } from "@destack/language/registry";
import type { Client } from "@destack/language/universe";
import { hashInt, hashString } from "@destack/utils/hash";
import { Temporal } from "temporal-polyfill";

/* ==== DESTACK_GENERATED_START:NODE:1110010 ==== */
/**
 * A Span is a trace inside a Run.
 */
export class SpanEvent extends Event {
  static metatype: NodeType = NodeType.SPAN_EVENT;

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
   * SpanEvent.node
   */
  get node(): Run | null {
    const nodePtr: NodeReference | null = this.nodePtr;
    if (nodePtr != null) {
      return this._session.graph.get(nodePtr) as Run | null;
    }
    return null;
  }
  readonly nodePtr: NodeReference;

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
        throw new Error(`no active Space for SpanEvent`);
      }
      _space = _space.toRef();
    }
    if (_space == null) {
      throw new Error(`SpanEvent.space is required`);
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
        throw new Error(`no active Branch for SpanEvent`);
      }
      _branch = _branch.toRef();
    }
    if (_branch == null) {
      throw new Error(`SpanEvent.branch is required`);
    }
    this.branchPtr = _branch as NodeReference;
    let _snapshot = options.snapshot ?? null;
    if (_snapshot != null && _snapshot.constructor.name !== "NodeReference") {
      _snapshot = (_snapshot as Node).toRef();
    }
    if (_snapshot == null) {
      _snapshot = ACTIVE_SNAPSHOT.get();
      if (_snapshot == null) {
        throw new Error(`no active Snapshot for SpanEvent`);
      }
      _snapshot = _snapshot.toRef();
    }
    if (_snapshot == null) {
      throw new Error(`SpanEvent.snapshot is required`);
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
      throw new Error(`SpanEvent.client is required`);
    }
    this.clientPtr = _client as NodeReference;
    let _clientNonce = options.clientNonce ?? null;
    if (_clientNonce == null) {
      _clientNonce = this._session.clientNonce;
    }
    if (_clientNonce == null) {
      throw new Error(`SpanEvent.clientNonce is required`);
    }
    this.clientNonce = _clientNonce;
    let _status = options.status ?? null;
    if (_status == null) {
      _status = 1 /* EventStatus.PENDING */;
    }
    if (_status == null) {
      throw new Error(`SpanEvent.status is required`);
    }
    this.status = _status;
    let _node = options.node;
    if (_node != null && _node.constructor.name !== "NodeReference") {
      _node = (_node as Node).toRef();
    }
    if (_node == null) {
      throw new Error(`SpanEvent.node is required`);
    }
    this.nodePtr = _node as NodeReference;

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
        throw new Error(`SpanEvent.createdAt is required for existing Events`);
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

  validate(): void {
    throw new Error("not implemented");
  }

  __toRef__(): NodeReference {
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    return new _NodeReference({
      type: NodeType.SPAN_EVENT,
      id: this.id,
      spaceId: this.spacePtr.id,
      definitionId: this.definitionPtr?.id ?? null,
      branchId: this.branchPtr.id,
      snapshotId: this.snapshotPtr.id,
      _session: this._session,
    });
  }

  get _pathKey(): string {
    return `SpanEvent[id=${this.id}]`;
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
    return `<SpanEvent "${this.path}" ${propertyReprs.join(" ")}>`;
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerNodeClass(NodeType.SPAN_EVENT, SpanEvent);
/* ==== DESTACK_GENERATED_END:NODE:1110010 ==== */
