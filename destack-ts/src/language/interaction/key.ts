import type {
  Boolean,
  Branch,
  Datetime,
  NodeReference,
  Session,
  Snapshot,
  Space,
  String,
  UInt128,
  UUID,
} from "@destack/language/core";
import {
  ACTIVE_BRANCH,
  ACTIVE_SNAPSHOT,
  ACTIVE_SPACE,
  type Entity,
  type Event,
  EventStatus,
  type Node,
  NodeType,
  StructType,
} from "@destack/language/core";
import { InputEvent } from "@destack/language/interaction/input";
import { registerNodeClass, STRUCT_CLASS_BY_TYPE } from "@destack/language/registry";
import type { Client } from "@destack/language/universe";
import { hashBool, hashInt, hashString } from "@destack/utils/hash";
import { Temporal } from "temporal-polyfill";

/* ==== DESTACK_GENERATED_START:NODE:2000300 ==== */
/**
 * A KeyEvent is an InputEvent that corresponds to some direct user input with a keyboard.
 */
export abstract class KeyEvent extends InputEvent {
  static metatype: NodeType = NodeType.KEY_EVENT;

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
   * InputEvent.node
   */
  abstract get node(): Entity | null;
  declare readonly nodePtr: NodeReference | null;

  /**
   * The character that was pressed (e.g. 'a', 'B', '1', 'Enter').
   */
  declare readonly key: String;

  /**
   * The unaltered key code that was pressed (e.g. 'KeyA', 'KeyB', 'Digit1', 'Enter').
   */
  declare readonly code: String;

  /**
   * Whether the key is being held down.
   */
  declare readonly isRepeat: Boolean;

  /**
   * Whether the key was masked for some reason (e.g., security, privacy).
   */
  declare readonly isRedacted: Boolean;

  /**
   * Whether the Shift key was held.
   */
  declare readonly shiftKey: Boolean;

  /**
   * Whether the Alt key was held.
   */
  declare readonly altKey: Boolean;

  /**
   * Whether the Ctrl key was held.
   */
  declare readonly ctrlKey: Boolean;

  /**
   * Whether the Meta key was held.
   */
  declare readonly metaKey: Boolean;

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerNodeClass(NodeType.KEY_EVENT, KeyEvent);
/* ==== DESTACK_GENERATED_END:NODE:2000300 ==== */

/* ==== DESTACK_GENERATED_START:NODE:2000301 ==== */
/**
 * A KeyDownEvent is a KeyboardEvent when a key is pressed down.
 */
export class KeyDownEvent extends KeyEvent {
  static metatype: NodeType = NodeType.KEY_DOWN_EVENT;

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
   * InputEvent.node
   */
  get node(): Entity | null {
    const nodePtr: NodeReference | null = this.nodePtr;
    if (nodePtr != null) {
      return this._session.graph.get(nodePtr) as Entity | null;
    }
    return null;
  }
  readonly nodePtr: NodeReference | null;

  /**
   * The character that was pressed (e.g. 'a', 'B', '1', 'Enter').
   */
  readonly key: String;

  /**
   * The unaltered key code that was pressed (e.g. 'KeyA', 'KeyB', 'Digit1', 'Enter').
   */
  readonly code: String;

  /**
   * Whether the key is being held down.
   */
  readonly isRepeat: Boolean;

  /**
   * Whether the key was masked for some reason (e.g., security, privacy).
   */
  readonly isRedacted: Boolean;

  /**
   * Whether the Shift key was held.
   */
  readonly shiftKey: Boolean;

  /**
   * Whether the Alt key was held.
   */
  readonly altKey: Boolean;

  /**
   * Whether the Ctrl key was held.
   */
  readonly ctrlKey: Boolean;

  /**
   * Whether the Meta key was held.
   */
  readonly metaKey: Boolean;

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
    node?: Entity | NodeReference | null;
    key: String;
    code: String;
    isRepeat: Boolean;
    isRedacted: Boolean;
    shiftKey: Boolean;
    altKey: Boolean;
    ctrlKey: Boolean;
    metaKey: Boolean;
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
    if (_space === null) {
      _space = ACTIVE_SPACE.get();
      if (_space === null) {
        throw new Error(`no active Space for KeyDownEvent`);
      }
      _space = _space.toRef();
    }
    if (_space === null) {
      throw new Error(`KeyDownEvent.space is required`);
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
    if (_branch === null) {
      _branch = ACTIVE_BRANCH.get();
      if (_branch === null) {
        throw new Error(`no active Branch for KeyDownEvent`);
      }
      _branch = _branch.toRef();
    }
    if (_branch === null) {
      throw new Error(`KeyDownEvent.branch is required`);
    }
    this.branchPtr = _branch as NodeReference;
    let _snapshot = options.snapshot ?? null;
    if (_snapshot != null && _snapshot.constructor.name !== "NodeReference") {
      _snapshot = (_snapshot as Node).toRef();
    }
    if (_snapshot === null) {
      _snapshot = ACTIVE_SNAPSHOT.get();
      if (_snapshot === null) {
        throw new Error(`no active Snapshot for KeyDownEvent`);
      }
      _snapshot = _snapshot.toRef();
    }
    if (_snapshot === null) {
      throw new Error(`KeyDownEvent.snapshot is required`);
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
    if (_client === null) {
      _client = this._session.clientPtr;
    }
    if (_client === null) {
      throw new Error(`KeyDownEvent.client is required`);
    }
    this.clientPtr = _client as NodeReference;
    let _clientNonce = options.clientNonce ?? null;
    if (_clientNonce === null) {
      _clientNonce = this._session.clientNonce;
    }
    if (_clientNonce === null) {
      throw new Error(`KeyDownEvent.clientNonce is required`);
    }
    this.clientNonce = _clientNonce;
    let _status = options.status ?? null;
    if (_status === null) {
      _status = 1 /* EventStatus.PENDING */;
    }
    if (_status === null) {
      throw new Error(`KeyDownEvent.status is required`);
    }
    this.status = _status;
    let _node = options.node ?? null;
    if (_node != null && _node.constructor.name !== "NodeReference") {
      _node = (_node as Node).toRef();
    }
    this.nodePtr = _node as NodeReference | null;
    let _key = options.key;
    if (_key === null) {
      throw new Error(`KeyDownEvent.key is required`);
    }
    this.key = _key;
    let _code = options.code;
    if (_code === null) {
      throw new Error(`KeyDownEvent.code is required`);
    }
    this.code = _code;
    let _isRepeat = options.isRepeat;
    if (_isRepeat === null) {
      throw new Error(`KeyDownEvent.isRepeat is required`);
    }
    this.isRepeat = _isRepeat;
    let _isRedacted = options.isRedacted;
    if (_isRedacted === null) {
      throw new Error(`KeyDownEvent.isRedacted is required`);
    }
    this.isRedacted = _isRedacted;
    let _shiftKey = options.shiftKey;
    if (_shiftKey === null) {
      throw new Error(`KeyDownEvent.shiftKey is required`);
    }
    this.shiftKey = _shiftKey;
    let _altKey = options.altKey;
    if (_altKey === null) {
      throw new Error(`KeyDownEvent.altKey is required`);
    }
    this.altKey = _altKey;
    let _ctrlKey = options.ctrlKey;
    if (_ctrlKey === null) {
      throw new Error(`KeyDownEvent.ctrlKey is required`);
    }
    this.ctrlKey = _ctrlKey;
    let _metaKey = options.metaKey;
    if (_metaKey === null) {
      throw new Error(`KeyDownEvent.metaKey is required`);
    }
    this.metaKey = _metaKey;

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
        throw new Error(`KeyDownEvent.createdAt is required for existing Events`);
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
    if (!(this.key === other.key)) {
      return false;
    }
    if (!(this.code === other.code)) {
      return false;
    }
    if (!(this.isRepeat === other.isRepeat)) {
      return false;
    }
    if (!(this.isRedacted === other.isRedacted)) {
      return false;
    }
    if (!(this.shiftKey === other.shiftKey)) {
      return false;
    }
    if (!(this.altKey === other.altKey)) {
      return false;
    }
    if (!(this.ctrlKey === other.ctrlKey)) {
      return false;
    }
    if (!(this.metaKey === other.metaKey)) {
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
    h = (h * 31 + hashString(this.key)) & 0xffffffff;
    h = (h * 31 + hashString(this.code)) & 0xffffffff;
    h = (h * 31 + hashBool(this.isRepeat)) & 0xffffffff;
    h = (h * 31 + hashBool(this.isRedacted)) & 0xffffffff;
    h = (h * 31 + hashBool(this.shiftKey)) & 0xffffffff;
    h = (h * 31 + hashBool(this.altKey)) & 0xffffffff;
    h = (h * 31 + hashBool(this.ctrlKey)) & 0xffffffff;
    h = (h * 31 + hashBool(this.metaKey)) & 0xffffffff;
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
      type: NodeType.KEY_DOWN_EVENT,
      id: this.id,
      spaceId: this.spacePtr.id,
      definitionId: this.definitionPtr?.id ?? null,
      branchId: this.branchPtr.id,
      snapshotId: this.snapshotPtr.id,
      _session: this._session,
    });
  }

  get _pathKey(): string {
    return `KeyDownEvent[id=${this.id}]`;
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
    propertyReprs.push(`key=${`"${this.key}"`}`);
    propertyReprs.push(`code=${`"${this.code}"`}`);
    propertyReprs.push(`isRepeat=${this.isRepeat}`);
    propertyReprs.push(`createdEpoch=${this.createdEpoch}`);
    propertyReprs.push(`status=${EventStatus[this.status]}`);
    return `<KeyDownEvent "${this.path}" ${propertyReprs.join(" ")}>`;
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerNodeClass(NodeType.KEY_DOWN_EVENT, KeyDownEvent);
/* ==== DESTACK_GENERATED_END:NODE:2000301 ==== */

/* ==== DESTACK_GENERATED_START:NODE:2000302 ==== */
/**
 * A KeyUpEvent is a KeyboardEvent when a key is released.
 */
export class KeyUpEvent extends KeyEvent {
  static metatype: NodeType = NodeType.KEY_UP_EVENT;

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
   * InputEvent.node
   */
  get node(): Entity | null {
    const nodePtr: NodeReference | null = this.nodePtr;
    if (nodePtr != null) {
      return this._session.graph.get(nodePtr) as Entity | null;
    }
    return null;
  }
  readonly nodePtr: NodeReference | null;

  /**
   * The character that was pressed (e.g. 'a', 'B', '1', 'Enter').
   */
  readonly key: String;

  /**
   * The unaltered key code that was pressed (e.g. 'KeyA', 'KeyB', 'Digit1', 'Enter').
   */
  readonly code: String;

  /**
   * Whether the key is being held down.
   */
  readonly isRepeat: Boolean;

  /**
   * Whether the key was masked for some reason (e.g., security, privacy).
   */
  readonly isRedacted: Boolean;

  /**
   * Whether the Shift key was held.
   */
  readonly shiftKey: Boolean;

  /**
   * Whether the Alt key was held.
   */
  readonly altKey: Boolean;

  /**
   * Whether the Ctrl key was held.
   */
  readonly ctrlKey: Boolean;

  /**
   * Whether the Meta key was held.
   */
  readonly metaKey: Boolean;

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
    node?: Entity | NodeReference | null;
    key: String;
    code: String;
    isRepeat: Boolean;
    isRedacted: Boolean;
    shiftKey: Boolean;
    altKey: Boolean;
    ctrlKey: Boolean;
    metaKey: Boolean;
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
    if (_space === null) {
      _space = ACTIVE_SPACE.get();
      if (_space === null) {
        throw new Error(`no active Space for KeyUpEvent`);
      }
      _space = _space.toRef();
    }
    if (_space === null) {
      throw new Error(`KeyUpEvent.space is required`);
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
    if (_branch === null) {
      _branch = ACTIVE_BRANCH.get();
      if (_branch === null) {
        throw new Error(`no active Branch for KeyUpEvent`);
      }
      _branch = _branch.toRef();
    }
    if (_branch === null) {
      throw new Error(`KeyUpEvent.branch is required`);
    }
    this.branchPtr = _branch as NodeReference;
    let _snapshot = options.snapshot ?? null;
    if (_snapshot != null && _snapshot.constructor.name !== "NodeReference") {
      _snapshot = (_snapshot as Node).toRef();
    }
    if (_snapshot === null) {
      _snapshot = ACTIVE_SNAPSHOT.get();
      if (_snapshot === null) {
        throw new Error(`no active Snapshot for KeyUpEvent`);
      }
      _snapshot = _snapshot.toRef();
    }
    if (_snapshot === null) {
      throw new Error(`KeyUpEvent.snapshot is required`);
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
    if (_client === null) {
      _client = this._session.clientPtr;
    }
    if (_client === null) {
      throw new Error(`KeyUpEvent.client is required`);
    }
    this.clientPtr = _client as NodeReference;
    let _clientNonce = options.clientNonce ?? null;
    if (_clientNonce === null) {
      _clientNonce = this._session.clientNonce;
    }
    if (_clientNonce === null) {
      throw new Error(`KeyUpEvent.clientNonce is required`);
    }
    this.clientNonce = _clientNonce;
    let _status = options.status ?? null;
    if (_status === null) {
      _status = 1 /* EventStatus.PENDING */;
    }
    if (_status === null) {
      throw new Error(`KeyUpEvent.status is required`);
    }
    this.status = _status;
    let _node = options.node ?? null;
    if (_node != null && _node.constructor.name !== "NodeReference") {
      _node = (_node as Node).toRef();
    }
    this.nodePtr = _node as NodeReference | null;
    let _key = options.key;
    if (_key === null) {
      throw new Error(`KeyUpEvent.key is required`);
    }
    this.key = _key;
    let _code = options.code;
    if (_code === null) {
      throw new Error(`KeyUpEvent.code is required`);
    }
    this.code = _code;
    let _isRepeat = options.isRepeat;
    if (_isRepeat === null) {
      throw new Error(`KeyUpEvent.isRepeat is required`);
    }
    this.isRepeat = _isRepeat;
    let _isRedacted = options.isRedacted;
    if (_isRedacted === null) {
      throw new Error(`KeyUpEvent.isRedacted is required`);
    }
    this.isRedacted = _isRedacted;
    let _shiftKey = options.shiftKey;
    if (_shiftKey === null) {
      throw new Error(`KeyUpEvent.shiftKey is required`);
    }
    this.shiftKey = _shiftKey;
    let _altKey = options.altKey;
    if (_altKey === null) {
      throw new Error(`KeyUpEvent.altKey is required`);
    }
    this.altKey = _altKey;
    let _ctrlKey = options.ctrlKey;
    if (_ctrlKey === null) {
      throw new Error(`KeyUpEvent.ctrlKey is required`);
    }
    this.ctrlKey = _ctrlKey;
    let _metaKey = options.metaKey;
    if (_metaKey === null) {
      throw new Error(`KeyUpEvent.metaKey is required`);
    }
    this.metaKey = _metaKey;

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
        throw new Error(`KeyUpEvent.createdAt is required for existing Events`);
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
    if (!(this.key === other.key)) {
      return false;
    }
    if (!(this.code === other.code)) {
      return false;
    }
    if (!(this.isRepeat === other.isRepeat)) {
      return false;
    }
    if (!(this.isRedacted === other.isRedacted)) {
      return false;
    }
    if (!(this.shiftKey === other.shiftKey)) {
      return false;
    }
    if (!(this.altKey === other.altKey)) {
      return false;
    }
    if (!(this.ctrlKey === other.ctrlKey)) {
      return false;
    }
    if (!(this.metaKey === other.metaKey)) {
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
    h = (h * 31 + hashString(this.key)) & 0xffffffff;
    h = (h * 31 + hashString(this.code)) & 0xffffffff;
    h = (h * 31 + hashBool(this.isRepeat)) & 0xffffffff;
    h = (h * 31 + hashBool(this.isRedacted)) & 0xffffffff;
    h = (h * 31 + hashBool(this.shiftKey)) & 0xffffffff;
    h = (h * 31 + hashBool(this.altKey)) & 0xffffffff;
    h = (h * 31 + hashBool(this.ctrlKey)) & 0xffffffff;
    h = (h * 31 + hashBool(this.metaKey)) & 0xffffffff;
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
      type: NodeType.KEY_UP_EVENT,
      id: this.id,
      spaceId: this.spacePtr.id,
      definitionId: this.definitionPtr?.id ?? null,
      branchId: this.branchPtr.id,
      snapshotId: this.snapshotPtr.id,
      _session: this._session,
    });
  }

  get _pathKey(): string {
    return `KeyUpEvent[id=${this.id}]`;
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
    propertyReprs.push(`key=${`"${this.key}"`}`);
    propertyReprs.push(`code=${`"${this.code}"`}`);
    propertyReprs.push(`isRepeat=${this.isRepeat}`);
    propertyReprs.push(`createdEpoch=${this.createdEpoch}`);
    propertyReprs.push(`status=${EventStatus[this.status]}`);
    return `<KeyUpEvent "${this.path}" ${propertyReprs.join(" ")}>`;
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerNodeClass(NodeType.KEY_UP_EVENT, KeyUpEvent);
/* ==== DESTACK_GENERATED_END:NODE:2000302 ==== */

/* ==== DESTACK_GENERATED_START:NODE:2000303 ==== */
/**
 * A KeyPressEvent is a KeyboardEvent when a key is pressed.
 */
export class KeyPressEvent extends KeyEvent {
  static metatype: NodeType = NodeType.KEY_PRESS_EVENT;

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
   * InputEvent.node
   */
  get node(): Entity | null {
    const nodePtr: NodeReference | null = this.nodePtr;
    if (nodePtr != null) {
      return this._session.graph.get(nodePtr) as Entity | null;
    }
    return null;
  }
  readonly nodePtr: NodeReference | null;

  /**
   * The character that was pressed (e.g. 'a', 'B', '1', 'Enter').
   */
  readonly key: String;

  /**
   * The unaltered key code that was pressed (e.g. 'KeyA', 'KeyB', 'Digit1', 'Enter').
   */
  readonly code: String;

  /**
   * Whether the key is being held down.
   */
  readonly isRepeat: Boolean;

  /**
   * Whether the key was masked for some reason (e.g., security, privacy).
   */
  readonly isRedacted: Boolean;

  /**
   * Whether the Shift key was held.
   */
  readonly shiftKey: Boolean;

  /**
   * Whether the Alt key was held.
   */
  readonly altKey: Boolean;

  /**
   * Whether the Ctrl key was held.
   */
  readonly ctrlKey: Boolean;

  /**
   * Whether the Meta key was held.
   */
  readonly metaKey: Boolean;

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
    node?: Entity | NodeReference | null;
    key: String;
    code: String;
    isRepeat: Boolean;
    isRedacted: Boolean;
    shiftKey: Boolean;
    altKey: Boolean;
    ctrlKey: Boolean;
    metaKey: Boolean;
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
    if (_space === null) {
      _space = ACTIVE_SPACE.get();
      if (_space === null) {
        throw new Error(`no active Space for KeyPressEvent`);
      }
      _space = _space.toRef();
    }
    if (_space === null) {
      throw new Error(`KeyPressEvent.space is required`);
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
    if (_branch === null) {
      _branch = ACTIVE_BRANCH.get();
      if (_branch === null) {
        throw new Error(`no active Branch for KeyPressEvent`);
      }
      _branch = _branch.toRef();
    }
    if (_branch === null) {
      throw new Error(`KeyPressEvent.branch is required`);
    }
    this.branchPtr = _branch as NodeReference;
    let _snapshot = options.snapshot ?? null;
    if (_snapshot != null && _snapshot.constructor.name !== "NodeReference") {
      _snapshot = (_snapshot as Node).toRef();
    }
    if (_snapshot === null) {
      _snapshot = ACTIVE_SNAPSHOT.get();
      if (_snapshot === null) {
        throw new Error(`no active Snapshot for KeyPressEvent`);
      }
      _snapshot = _snapshot.toRef();
    }
    if (_snapshot === null) {
      throw new Error(`KeyPressEvent.snapshot is required`);
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
    if (_client === null) {
      _client = this._session.clientPtr;
    }
    if (_client === null) {
      throw new Error(`KeyPressEvent.client is required`);
    }
    this.clientPtr = _client as NodeReference;
    let _clientNonce = options.clientNonce ?? null;
    if (_clientNonce === null) {
      _clientNonce = this._session.clientNonce;
    }
    if (_clientNonce === null) {
      throw new Error(`KeyPressEvent.clientNonce is required`);
    }
    this.clientNonce = _clientNonce;
    let _status = options.status ?? null;
    if (_status === null) {
      _status = 1 /* EventStatus.PENDING */;
    }
    if (_status === null) {
      throw new Error(`KeyPressEvent.status is required`);
    }
    this.status = _status;
    let _node = options.node ?? null;
    if (_node != null && _node.constructor.name !== "NodeReference") {
      _node = (_node as Node).toRef();
    }
    this.nodePtr = _node as NodeReference | null;
    let _key = options.key;
    if (_key === null) {
      throw new Error(`KeyPressEvent.key is required`);
    }
    this.key = _key;
    let _code = options.code;
    if (_code === null) {
      throw new Error(`KeyPressEvent.code is required`);
    }
    this.code = _code;
    let _isRepeat = options.isRepeat;
    if (_isRepeat === null) {
      throw new Error(`KeyPressEvent.isRepeat is required`);
    }
    this.isRepeat = _isRepeat;
    let _isRedacted = options.isRedacted;
    if (_isRedacted === null) {
      throw new Error(`KeyPressEvent.isRedacted is required`);
    }
    this.isRedacted = _isRedacted;
    let _shiftKey = options.shiftKey;
    if (_shiftKey === null) {
      throw new Error(`KeyPressEvent.shiftKey is required`);
    }
    this.shiftKey = _shiftKey;
    let _altKey = options.altKey;
    if (_altKey === null) {
      throw new Error(`KeyPressEvent.altKey is required`);
    }
    this.altKey = _altKey;
    let _ctrlKey = options.ctrlKey;
    if (_ctrlKey === null) {
      throw new Error(`KeyPressEvent.ctrlKey is required`);
    }
    this.ctrlKey = _ctrlKey;
    let _metaKey = options.metaKey;
    if (_metaKey === null) {
      throw new Error(`KeyPressEvent.metaKey is required`);
    }
    this.metaKey = _metaKey;

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
        throw new Error(`KeyPressEvent.createdAt is required for existing Events`);
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
    if (!(this.key === other.key)) {
      return false;
    }
    if (!(this.code === other.code)) {
      return false;
    }
    if (!(this.isRepeat === other.isRepeat)) {
      return false;
    }
    if (!(this.isRedacted === other.isRedacted)) {
      return false;
    }
    if (!(this.shiftKey === other.shiftKey)) {
      return false;
    }
    if (!(this.altKey === other.altKey)) {
      return false;
    }
    if (!(this.ctrlKey === other.ctrlKey)) {
      return false;
    }
    if (!(this.metaKey === other.metaKey)) {
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
    h = (h * 31 + hashString(this.key)) & 0xffffffff;
    h = (h * 31 + hashString(this.code)) & 0xffffffff;
    h = (h * 31 + hashBool(this.isRepeat)) & 0xffffffff;
    h = (h * 31 + hashBool(this.isRedacted)) & 0xffffffff;
    h = (h * 31 + hashBool(this.shiftKey)) & 0xffffffff;
    h = (h * 31 + hashBool(this.altKey)) & 0xffffffff;
    h = (h * 31 + hashBool(this.ctrlKey)) & 0xffffffff;
    h = (h * 31 + hashBool(this.metaKey)) & 0xffffffff;
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
      type: NodeType.KEY_PRESS_EVENT,
      id: this.id,
      spaceId: this.spacePtr.id,
      definitionId: this.definitionPtr?.id ?? null,
      branchId: this.branchPtr.id,
      snapshotId: this.snapshotPtr.id,
      _session: this._session,
    });
  }

  get _pathKey(): string {
    return `KeyPressEvent[id=${this.id}]`;
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
    propertyReprs.push(`key=${`"${this.key}"`}`);
    propertyReprs.push(`code=${`"${this.code}"`}`);
    propertyReprs.push(`isRepeat=${this.isRepeat}`);
    propertyReprs.push(`createdEpoch=${this.createdEpoch}`);
    propertyReprs.push(`status=${EventStatus[this.status]}`);
    return `<KeyPressEvent "${this.path}" ${propertyReprs.join(" ")}>`;
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerNodeClass(NodeType.KEY_PRESS_EVENT, KeyPressEvent);
/* ==== DESTACK_GENERATED_END:NODE:2000303 ==== */
