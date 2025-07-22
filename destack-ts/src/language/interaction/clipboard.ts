import type {
  Branch,
  Graph,
  GraphConnection,
  IsActor,
  NodeReference,
  Session,
  Snapshot,
  Space,
} from "@destack/language/core";
import {
  ACTIVE_BRANCH,
  ACTIVE_SNAPSHOT,
  ACTIVE_SPACE,
  Entity,
  Event,
  EventStatus,
  Node,
  NodeType,
  StructType,
} from "@destack/language/core";
import { InputEvent } from "@destack/language/interaction/input";
import { STRUCT_CLASS_BY_TYPE, registerNodeClass } from "@destack/language/registry";
import type { Client } from "@destack/language/universe";
import { hashInt, hashString } from "@destack/utils/hash";
import { Temporal } from "temporal-polyfill";

/* ==== DESTACK_GENERATED_START:NODE:2000500 ==== */
/**
 * A ClipboardEvent is an InputEvent that corresponds to some direct user input with a clipboard.
 */
export abstract class ClipboardEvent extends InputEvent {
  static metatype: NodeType = NodeType.CLIPBOARD_EVENT;

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
   * The time this Event was created (system time).
   */
  declare readonly createdAt: Temporal.ZonedDateTime;

  /**
   * The logical time this Event was created (system time).
   */
  declare readonly createdEpoch: number;

  /**
   * The Actor that created this Event.
   */
  abstract get createdBy(): (Entity & IsActor) | null;
  declare readonly createdByPtr: NodeReference | null;

  /**
   * The Client that created this Event.
   */
  abstract get client(): Client | null;
  declare readonly clientPtr: NodeReference | null;

  /**
   * The nonce of the Client that created this Event.
   */
  declare readonly clientNonce: string | null;

  /**
   * The time in the Client when it created this Event.
   */
  declare readonly clientCreatedAt: Temporal.ZonedDateTime;

  /**
   * The logical time in the Client when it created this Event.
   */
  declare readonly clientEpoch: number;

  /**
   * The status of the Event.
   */
  declare readonly status: EventStatus;

  /**
   * InputEvent.node
   */
  abstract get node(): Node | null;
  declare readonly nodePtr: NodeReference | null;

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerNodeClass(NodeType.CLIPBOARD_EVENT, ClipboardEvent);
/* ==== DESTACK_GENERATED_END:NODE:2000500 ==== */

/* ==== DESTACK_GENERATED_START:NODE:2000501 ==== */
/**
 * A CopyEvent is a ClipboardEvent when a copy is performed.
 */
export class CopyEvent extends ClipboardEvent {
  static metatype: NodeType = NodeType.COPY_EVENT;

  /**
   * The Space this Node is in.
   */
  get space(): Space | null {
    const nodePtr: NodeReference | null = this.spacePtr;
    if (nodePtr != null) {
      return this._graph.get(nodePtr.id) as Space | null;
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
      return this._graph.get(nodePtr.id) as Entity | null;
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
      return this._graph.get(nodePtr.id) as Branch | null;
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
      return this._graph.get(nodePtr.id) as Snapshot | null;
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
      return this._graph.get(nodePtr.id) as Event | null;
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
      return this._graph.get(nodePtr.id) as Event | null;
    }
    return null;
  }
  readonly causedByPtr: NodeReference | null;

  /**
   * The time this Event was created (system time).
   */
  readonly createdAt: Temporal.ZonedDateTime;

  /**
   * The logical time this Event was created (system time).
   */
  readonly createdEpoch: number;

  /**
   * The Actor that created this Event.
   */
  get createdBy(): (Entity & IsActor) | null {
    const nodePtr: NodeReference | null = this.createdByPtr;
    if (nodePtr != null) {
      return this._graph.get(nodePtr.id) as (Entity & IsActor) | null;
    }
    return null;
  }
  readonly createdByPtr: NodeReference | null;

  /**
   * The Client that created this Event.
   */
  get client(): Client | null {
    const nodePtr: NodeReference | null = this.clientPtr;
    if (nodePtr != null) {
      return this._graph.get(nodePtr.id) as Client | null;
    }
    return null;
  }
  readonly clientPtr: NodeReference | null;

  /**
   * The nonce of the Client that created this Event.
   */
  readonly clientNonce: string | null;

  /**
   * The time in the Client when it created this Event.
   */
  readonly clientCreatedAt: Temporal.ZonedDateTime;

  /**
   * The logical time in the Client when it created this Event.
   */
  readonly clientEpoch: number;

  /**
   * The status of the Event.
   */
  readonly status: EventStatus;

  /**
   * InputEvent.node
   */
  get node(): Node | null {
    const nodePtr: NodeReference | null = this.nodePtr;
    if (nodePtr != null) {
      return this._graph.get(nodePtr.id) as Node | null;
    }
    return null;
  }
  readonly nodePtr: NodeReference | null;

  constructor(options: {
    id?: string;
    space?: Space | NodeReference;
    definition?: Entity | NodeReference | null;
    branch?: Branch | NodeReference;
    snapshot?: Snapshot | NodeReference;
    precededBy?: Event | NodeReference | null;
    causedBy?: Event | NodeReference | null;
    createdAt?: Temporal.ZonedDateTime;
    createdEpoch?: number;
    createdBy?: (Entity & IsActor) | NodeReference | null;
    client?: Client | NodeReference | null;
    clientNonce?: string | null;
    clientCreatedAt?: Temporal.ZonedDateTime;
    clientEpoch?: number;
    status?: EventStatus;
    node?: Node | NodeReference | null;
    _session?: Session | null;
    _graph?: Graph | null;
    _connection?: GraphConnection | null;
  }) {
    /* super */
    super(
      /* id */
      options.id ?? null,
      /* parent */
      null,
      /* session */
      options._session ?? null,
      /* graph */
      options._graph ?? null,
      /* connection */
      options._connection ?? null,
      /* _isNew */
      options.id == null,
    );

    /* properties */
    let _space = options.space ?? null;
    if (_space != null && _space.constructor.name != "NodeReference") {
      _space = (_space as Node).toRef();
    }
    if (_space === null) {
      _space = ACTIVE_SPACE.get();
      if (_space === null) {
        throw new Error(`no active Space for CopyEvent`);
      }
      _space = _space.toRef();
    }
    if (_space === null) {
      throw new Error(`CopyEvent.space is required`);
    }
    this.spacePtr = _space as NodeReference;
    let _definition = options.definition ?? null;
    if (_definition != null && _definition.constructor.name != "NodeReference") {
      _definition = (_definition as Node).toRef();
    }
    this.definitionPtr = _definition as NodeReference | null;
    let _branch = options.branch ?? null;
    if (_branch != null && _branch.constructor.name != "NodeReference") {
      _branch = (_branch as Node).toRef();
    }
    if (_branch === null) {
      _branch = ACTIVE_BRANCH.get();
      if (_branch === null) {
        throw new Error(`no active Branch for CopyEvent`);
      }
      _branch = _branch.toRef();
    }
    if (_branch === null) {
      throw new Error(`CopyEvent.branch is required`);
    }
    this.branchPtr = _branch as NodeReference;
    let _snapshot = options.snapshot ?? null;
    if (_snapshot != null && _snapshot.constructor.name != "NodeReference") {
      _snapshot = (_snapshot as Node).toRef();
    }
    if (_snapshot === null) {
      _snapshot = ACTIVE_SNAPSHOT.get();
      if (_snapshot === null) {
        throw new Error(`no active Snapshot for CopyEvent`);
      }
      _snapshot = _snapshot.toRef();
    }
    if (_snapshot === null) {
      throw new Error(`CopyEvent.snapshot is required`);
    }
    this.snapshotPtr = _snapshot as NodeReference;
    let _precededBy = options.precededBy ?? null;
    if (_precededBy != null && _precededBy.constructor.name != "NodeReference") {
      _precededBy = (_precededBy as Node).toRef();
    }
    this.precededByPtr = _precededBy as NodeReference | null;
    let _causedBy = options.causedBy ?? null;
    if (_causedBy != null && _causedBy.constructor.name != "NodeReference") {
      _causedBy = (_causedBy as Node).toRef();
    }
    this.causedByPtr = _causedBy as NodeReference | null;
    let _client = options.client ?? null;
    if (_client != null && _client.constructor.name != "NodeReference") {
      _client = (_client as Node).toRef();
    }
    this.clientPtr = _client as NodeReference | null;
    let _clientNonce = options.clientNonce ?? null;
    this.clientNonce = _clientNonce;
    let _status = options.status ?? null;
    if (_status === null) {
      _status = 1 /* EventStatus.PENDING */;
    }
    if (_status === null) {
      throw new Error(`CopyEvent.status is required`);
    }
    this.status = _status;
    let _node = options.node ?? null;
    if (_node != null && _node.constructor.name != "NodeReference") {
      _node = (_node as Node).toRef();
    }
    this.nodePtr = _node as NodeReference | null;

    /* identity */
    if (options.id == null) {
      const now = Temporal.Now.zonedDateTimeISO("UTC");
      const epoch = this._session.epoch;
      this.createdAt = now;
      this.createdEpoch = epoch;
      this.createdByPtr = null;
      this.clientCreatedAt = now;
      this.clientEpoch = epoch;
    } else {
      if (
        options.createdAt == null ||
        options.createdEpoch == null ||
        options.clientCreatedAt == null ||
        options.clientEpoch == null
      ) {
        throw new Error(`CopyEvent.createdAt is required for existing Events`);
      }
      this.createdAt = options.createdAt;
      this.createdEpoch = options.createdEpoch;
      this.createdByPtr =
        options.createdBy != null
          ? options.createdBy.constructor.name == "NodeReference"
            ? (options.createdBy as NodeReference)
            : (options.createdBy as Node).toRef()
          : null;
      this.clientCreatedAt = options.clientCreatedAt;
      this.clientEpoch = options.clientEpoch;
    }
  }

  equals(other: any): boolean {
    if (!(this.metatype === other.metatype)) {
      return false;
    }
    if (!(this.nodePtr?.id === other.nodePtr?.id)) {
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
    if (!(this.clientPtr?.id === other.clientPtr?.id)) {
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
    if (this.nodePtr != null) {
      h = (h * 31 + hashString(this.nodePtr.id)) & 0xffffffff;
    }
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
    if (this.createdByPtr != null) {
      h = (h * 31 + hashString(this.createdByPtr.id)) & 0xffffffff;
    }
    if (this.clientPtr != null) {
      h = (h * 31 + hashString(this.clientPtr.id)) & 0xffffffff;
    }
    if (this.clientNonce != null) {
      h = (h * 31 + hashString(this.clientNonce.toString())) & 0xffffffff;
    }
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
      type: NodeType.COPY_EVENT,
      id: this.id,
      spaceId: this.spacePtr?.id ?? null,
      definitionId: this.definitionPtr?.id ?? null,
      branchId: this.branchPtr?.id ?? null,
      snapshotId: this.snapshotPtr?.id ?? null,
      _session: this._session,
      _graph: this._graph,
    });
  }

  get _pathKey(): string {
    return `CopyEvent[id=${this.id}]`;
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
    return `<CopyEvent "${this.path}" ${propertyReprs.join(" ")}>`;
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerNodeClass(NodeType.COPY_EVENT, CopyEvent);
/* ==== DESTACK_GENERATED_END:NODE:2000501 ==== */

/* ==== DESTACK_GENERATED_START:NODE:2000502 ==== */
/**
 * A CutEvent is a ClipboardEvent when a cut is performed.
 */
export class CutEvent extends ClipboardEvent {
  static metatype: NodeType = NodeType.CUT_EVENT;

  /**
   * The Space this Node is in.
   */
  get space(): Space | null {
    const nodePtr: NodeReference | null = this.spacePtr;
    if (nodePtr != null) {
      return this._graph.get(nodePtr.id) as Space | null;
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
      return this._graph.get(nodePtr.id) as Entity | null;
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
      return this._graph.get(nodePtr.id) as Branch | null;
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
      return this._graph.get(nodePtr.id) as Snapshot | null;
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
      return this._graph.get(nodePtr.id) as Event | null;
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
      return this._graph.get(nodePtr.id) as Event | null;
    }
    return null;
  }
  readonly causedByPtr: NodeReference | null;

  /**
   * The time this Event was created (system time).
   */
  readonly createdAt: Temporal.ZonedDateTime;

  /**
   * The logical time this Event was created (system time).
   */
  readonly createdEpoch: number;

  /**
   * The Actor that created this Event.
   */
  get createdBy(): (Entity & IsActor) | null {
    const nodePtr: NodeReference | null = this.createdByPtr;
    if (nodePtr != null) {
      return this._graph.get(nodePtr.id) as (Entity & IsActor) | null;
    }
    return null;
  }
  readonly createdByPtr: NodeReference | null;

  /**
   * The Client that created this Event.
   */
  get client(): Client | null {
    const nodePtr: NodeReference | null = this.clientPtr;
    if (nodePtr != null) {
      return this._graph.get(nodePtr.id) as Client | null;
    }
    return null;
  }
  readonly clientPtr: NodeReference | null;

  /**
   * The nonce of the Client that created this Event.
   */
  readonly clientNonce: string | null;

  /**
   * The time in the Client when it created this Event.
   */
  readonly clientCreatedAt: Temporal.ZonedDateTime;

  /**
   * The logical time in the Client when it created this Event.
   */
  readonly clientEpoch: number;

  /**
   * The status of the Event.
   */
  readonly status: EventStatus;

  /**
   * InputEvent.node
   */
  get node(): Node | null {
    const nodePtr: NodeReference | null = this.nodePtr;
    if (nodePtr != null) {
      return this._graph.get(nodePtr.id) as Node | null;
    }
    return null;
  }
  readonly nodePtr: NodeReference | null;

  constructor(options: {
    id?: string;
    space?: Space | NodeReference;
    definition?: Entity | NodeReference | null;
    branch?: Branch | NodeReference;
    snapshot?: Snapshot | NodeReference;
    precededBy?: Event | NodeReference | null;
    causedBy?: Event | NodeReference | null;
    createdAt?: Temporal.ZonedDateTime;
    createdEpoch?: number;
    createdBy?: (Entity & IsActor) | NodeReference | null;
    client?: Client | NodeReference | null;
    clientNonce?: string | null;
    clientCreatedAt?: Temporal.ZonedDateTime;
    clientEpoch?: number;
    status?: EventStatus;
    node?: Node | NodeReference | null;
    _session?: Session | null;
    _graph?: Graph | null;
    _connection?: GraphConnection | null;
  }) {
    /* super */
    super(
      /* id */
      options.id ?? null,
      /* parent */
      null,
      /* session */
      options._session ?? null,
      /* graph */
      options._graph ?? null,
      /* connection */
      options._connection ?? null,
      /* _isNew */
      options.id == null,
    );

    /* properties */
    let _space = options.space ?? null;
    if (_space != null && _space.constructor.name != "NodeReference") {
      _space = (_space as Node).toRef();
    }
    if (_space === null) {
      _space = ACTIVE_SPACE.get();
      if (_space === null) {
        throw new Error(`no active Space for CutEvent`);
      }
      _space = _space.toRef();
    }
    if (_space === null) {
      throw new Error(`CutEvent.space is required`);
    }
    this.spacePtr = _space as NodeReference;
    let _definition = options.definition ?? null;
    if (_definition != null && _definition.constructor.name != "NodeReference") {
      _definition = (_definition as Node).toRef();
    }
    this.definitionPtr = _definition as NodeReference | null;
    let _branch = options.branch ?? null;
    if (_branch != null && _branch.constructor.name != "NodeReference") {
      _branch = (_branch as Node).toRef();
    }
    if (_branch === null) {
      _branch = ACTIVE_BRANCH.get();
      if (_branch === null) {
        throw new Error(`no active Branch for CutEvent`);
      }
      _branch = _branch.toRef();
    }
    if (_branch === null) {
      throw new Error(`CutEvent.branch is required`);
    }
    this.branchPtr = _branch as NodeReference;
    let _snapshot = options.snapshot ?? null;
    if (_snapshot != null && _snapshot.constructor.name != "NodeReference") {
      _snapshot = (_snapshot as Node).toRef();
    }
    if (_snapshot === null) {
      _snapshot = ACTIVE_SNAPSHOT.get();
      if (_snapshot === null) {
        throw new Error(`no active Snapshot for CutEvent`);
      }
      _snapshot = _snapshot.toRef();
    }
    if (_snapshot === null) {
      throw new Error(`CutEvent.snapshot is required`);
    }
    this.snapshotPtr = _snapshot as NodeReference;
    let _precededBy = options.precededBy ?? null;
    if (_precededBy != null && _precededBy.constructor.name != "NodeReference") {
      _precededBy = (_precededBy as Node).toRef();
    }
    this.precededByPtr = _precededBy as NodeReference | null;
    let _causedBy = options.causedBy ?? null;
    if (_causedBy != null && _causedBy.constructor.name != "NodeReference") {
      _causedBy = (_causedBy as Node).toRef();
    }
    this.causedByPtr = _causedBy as NodeReference | null;
    let _client = options.client ?? null;
    if (_client != null && _client.constructor.name != "NodeReference") {
      _client = (_client as Node).toRef();
    }
    this.clientPtr = _client as NodeReference | null;
    let _clientNonce = options.clientNonce ?? null;
    this.clientNonce = _clientNonce;
    let _status = options.status ?? null;
    if (_status === null) {
      _status = 1 /* EventStatus.PENDING */;
    }
    if (_status === null) {
      throw new Error(`CutEvent.status is required`);
    }
    this.status = _status;
    let _node = options.node ?? null;
    if (_node != null && _node.constructor.name != "NodeReference") {
      _node = (_node as Node).toRef();
    }
    this.nodePtr = _node as NodeReference | null;

    /* identity */
    if (options.id == null) {
      const now = Temporal.Now.zonedDateTimeISO("UTC");
      const epoch = this._session.epoch;
      this.createdAt = now;
      this.createdEpoch = epoch;
      this.createdByPtr = null;
      this.clientCreatedAt = now;
      this.clientEpoch = epoch;
    } else {
      if (
        options.createdAt == null ||
        options.createdEpoch == null ||
        options.clientCreatedAt == null ||
        options.clientEpoch == null
      ) {
        throw new Error(`CutEvent.createdAt is required for existing Events`);
      }
      this.createdAt = options.createdAt;
      this.createdEpoch = options.createdEpoch;
      this.createdByPtr =
        options.createdBy != null
          ? options.createdBy.constructor.name == "NodeReference"
            ? (options.createdBy as NodeReference)
            : (options.createdBy as Node).toRef()
          : null;
      this.clientCreatedAt = options.clientCreatedAt;
      this.clientEpoch = options.clientEpoch;
    }
  }

  equals(other: any): boolean {
    if (!(this.metatype === other.metatype)) {
      return false;
    }
    if (!(this.nodePtr?.id === other.nodePtr?.id)) {
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
    if (!(this.clientPtr?.id === other.clientPtr?.id)) {
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
    if (this.nodePtr != null) {
      h = (h * 31 + hashString(this.nodePtr.id)) & 0xffffffff;
    }
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
    if (this.createdByPtr != null) {
      h = (h * 31 + hashString(this.createdByPtr.id)) & 0xffffffff;
    }
    if (this.clientPtr != null) {
      h = (h * 31 + hashString(this.clientPtr.id)) & 0xffffffff;
    }
    if (this.clientNonce != null) {
      h = (h * 31 + hashString(this.clientNonce.toString())) & 0xffffffff;
    }
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
      type: NodeType.CUT_EVENT,
      id: this.id,
      spaceId: this.spacePtr?.id ?? null,
      definitionId: this.definitionPtr?.id ?? null,
      branchId: this.branchPtr?.id ?? null,
      snapshotId: this.snapshotPtr?.id ?? null,
      _session: this._session,
      _graph: this._graph,
    });
  }

  get _pathKey(): string {
    return `CutEvent[id=${this.id}]`;
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
    return `<CutEvent "${this.path}" ${propertyReprs.join(" ")}>`;
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerNodeClass(NodeType.CUT_EVENT, CutEvent);
/* ==== DESTACK_GENERATED_END:NODE:2000502 ==== */

/* ==== DESTACK_GENERATED_START:NODE:2000503 ==== */
/**
 * A PasteEvent is a ClipboardEvent when a paste is performed.
 */
export class PasteEvent extends ClipboardEvent {
  static metatype: NodeType = NodeType.PASTE_EVENT;

  /**
   * The Space this Node is in.
   */
  get space(): Space | null {
    const nodePtr: NodeReference | null = this.spacePtr;
    if (nodePtr != null) {
      return this._graph.get(nodePtr.id) as Space | null;
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
      return this._graph.get(nodePtr.id) as Entity | null;
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
      return this._graph.get(nodePtr.id) as Branch | null;
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
      return this._graph.get(nodePtr.id) as Snapshot | null;
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
      return this._graph.get(nodePtr.id) as Event | null;
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
      return this._graph.get(nodePtr.id) as Event | null;
    }
    return null;
  }
  readonly causedByPtr: NodeReference | null;

  /**
   * The time this Event was created (system time).
   */
  readonly createdAt: Temporal.ZonedDateTime;

  /**
   * The logical time this Event was created (system time).
   */
  readonly createdEpoch: number;

  /**
   * The Actor that created this Event.
   */
  get createdBy(): (Entity & IsActor) | null {
    const nodePtr: NodeReference | null = this.createdByPtr;
    if (nodePtr != null) {
      return this._graph.get(nodePtr.id) as (Entity & IsActor) | null;
    }
    return null;
  }
  readonly createdByPtr: NodeReference | null;

  /**
   * The Client that created this Event.
   */
  get client(): Client | null {
    const nodePtr: NodeReference | null = this.clientPtr;
    if (nodePtr != null) {
      return this._graph.get(nodePtr.id) as Client | null;
    }
    return null;
  }
  readonly clientPtr: NodeReference | null;

  /**
   * The nonce of the Client that created this Event.
   */
  readonly clientNonce: string | null;

  /**
   * The time in the Client when it created this Event.
   */
  readonly clientCreatedAt: Temporal.ZonedDateTime;

  /**
   * The logical time in the Client when it created this Event.
   */
  readonly clientEpoch: number;

  /**
   * The status of the Event.
   */
  readonly status: EventStatus;

  /**
   * InputEvent.node
   */
  get node(): Node | null {
    const nodePtr: NodeReference | null = this.nodePtr;
    if (nodePtr != null) {
      return this._graph.get(nodePtr.id) as Node | null;
    }
    return null;
  }
  readonly nodePtr: NodeReference | null;

  constructor(options: {
    id?: string;
    space?: Space | NodeReference;
    definition?: Entity | NodeReference | null;
    branch?: Branch | NodeReference;
    snapshot?: Snapshot | NodeReference;
    precededBy?: Event | NodeReference | null;
    causedBy?: Event | NodeReference | null;
    createdAt?: Temporal.ZonedDateTime;
    createdEpoch?: number;
    createdBy?: (Entity & IsActor) | NodeReference | null;
    client?: Client | NodeReference | null;
    clientNonce?: string | null;
    clientCreatedAt?: Temporal.ZonedDateTime;
    clientEpoch?: number;
    status?: EventStatus;
    node?: Node | NodeReference | null;
    _session?: Session | null;
    _graph?: Graph | null;
    _connection?: GraphConnection | null;
  }) {
    /* super */
    super(
      /* id */
      options.id ?? null,
      /* parent */
      null,
      /* session */
      options._session ?? null,
      /* graph */
      options._graph ?? null,
      /* connection */
      options._connection ?? null,
      /* _isNew */
      options.id == null,
    );

    /* properties */
    let _space = options.space ?? null;
    if (_space != null && _space.constructor.name != "NodeReference") {
      _space = (_space as Node).toRef();
    }
    if (_space === null) {
      _space = ACTIVE_SPACE.get();
      if (_space === null) {
        throw new Error(`no active Space for PasteEvent`);
      }
      _space = _space.toRef();
    }
    if (_space === null) {
      throw new Error(`PasteEvent.space is required`);
    }
    this.spacePtr = _space as NodeReference;
    let _definition = options.definition ?? null;
    if (_definition != null && _definition.constructor.name != "NodeReference") {
      _definition = (_definition as Node).toRef();
    }
    this.definitionPtr = _definition as NodeReference | null;
    let _branch = options.branch ?? null;
    if (_branch != null && _branch.constructor.name != "NodeReference") {
      _branch = (_branch as Node).toRef();
    }
    if (_branch === null) {
      _branch = ACTIVE_BRANCH.get();
      if (_branch === null) {
        throw new Error(`no active Branch for PasteEvent`);
      }
      _branch = _branch.toRef();
    }
    if (_branch === null) {
      throw new Error(`PasteEvent.branch is required`);
    }
    this.branchPtr = _branch as NodeReference;
    let _snapshot = options.snapshot ?? null;
    if (_snapshot != null && _snapshot.constructor.name != "NodeReference") {
      _snapshot = (_snapshot as Node).toRef();
    }
    if (_snapshot === null) {
      _snapshot = ACTIVE_SNAPSHOT.get();
      if (_snapshot === null) {
        throw new Error(`no active Snapshot for PasteEvent`);
      }
      _snapshot = _snapshot.toRef();
    }
    if (_snapshot === null) {
      throw new Error(`PasteEvent.snapshot is required`);
    }
    this.snapshotPtr = _snapshot as NodeReference;
    let _precededBy = options.precededBy ?? null;
    if (_precededBy != null && _precededBy.constructor.name != "NodeReference") {
      _precededBy = (_precededBy as Node).toRef();
    }
    this.precededByPtr = _precededBy as NodeReference | null;
    let _causedBy = options.causedBy ?? null;
    if (_causedBy != null && _causedBy.constructor.name != "NodeReference") {
      _causedBy = (_causedBy as Node).toRef();
    }
    this.causedByPtr = _causedBy as NodeReference | null;
    let _client = options.client ?? null;
    if (_client != null && _client.constructor.name != "NodeReference") {
      _client = (_client as Node).toRef();
    }
    this.clientPtr = _client as NodeReference | null;
    let _clientNonce = options.clientNonce ?? null;
    this.clientNonce = _clientNonce;
    let _status = options.status ?? null;
    if (_status === null) {
      _status = 1 /* EventStatus.PENDING */;
    }
    if (_status === null) {
      throw new Error(`PasteEvent.status is required`);
    }
    this.status = _status;
    let _node = options.node ?? null;
    if (_node != null && _node.constructor.name != "NodeReference") {
      _node = (_node as Node).toRef();
    }
    this.nodePtr = _node as NodeReference | null;

    /* identity */
    if (options.id == null) {
      const now = Temporal.Now.zonedDateTimeISO("UTC");
      const epoch = this._session.epoch;
      this.createdAt = now;
      this.createdEpoch = epoch;
      this.createdByPtr = null;
      this.clientCreatedAt = now;
      this.clientEpoch = epoch;
    } else {
      if (
        options.createdAt == null ||
        options.createdEpoch == null ||
        options.clientCreatedAt == null ||
        options.clientEpoch == null
      ) {
        throw new Error(`PasteEvent.createdAt is required for existing Events`);
      }
      this.createdAt = options.createdAt;
      this.createdEpoch = options.createdEpoch;
      this.createdByPtr =
        options.createdBy != null
          ? options.createdBy.constructor.name == "NodeReference"
            ? (options.createdBy as NodeReference)
            : (options.createdBy as Node).toRef()
          : null;
      this.clientCreatedAt = options.clientCreatedAt;
      this.clientEpoch = options.clientEpoch;
    }
  }

  equals(other: any): boolean {
    if (!(this.metatype === other.metatype)) {
      return false;
    }
    if (!(this.nodePtr?.id === other.nodePtr?.id)) {
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
    if (!(this.clientPtr?.id === other.clientPtr?.id)) {
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
    if (this.nodePtr != null) {
      h = (h * 31 + hashString(this.nodePtr.id)) & 0xffffffff;
    }
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
    if (this.createdByPtr != null) {
      h = (h * 31 + hashString(this.createdByPtr.id)) & 0xffffffff;
    }
    if (this.clientPtr != null) {
      h = (h * 31 + hashString(this.clientPtr.id)) & 0xffffffff;
    }
    if (this.clientNonce != null) {
      h = (h * 31 + hashString(this.clientNonce.toString())) & 0xffffffff;
    }
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
      type: NodeType.PASTE_EVENT,
      id: this.id,
      spaceId: this.spacePtr?.id ?? null,
      definitionId: this.definitionPtr?.id ?? null,
      branchId: this.branchPtr?.id ?? null,
      snapshotId: this.snapshotPtr?.id ?? null,
      _session: this._session,
      _graph: this._graph,
    });
  }

  get _pathKey(): string {
    return `PasteEvent[id=${this.id}]`;
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
    return `<PasteEvent "${this.path}" ${propertyReprs.join(" ")}>`;
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerNodeClass(NodeType.PASTE_EVENT, PasteEvent);
/* ==== DESTACK_GENERATED_END:NODE:2000503 ==== */
