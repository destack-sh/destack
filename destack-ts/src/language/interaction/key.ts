import { packProtoTimestamp, unpackProtoTimestamp } from "@destack/grpc";
import type {
  Branch,
  Graph,
  IsActor,
  NodeReference,
  QueryConnection,
  Session,
  Snapshot,
  Space,
  Supergraph,
  Value,
} from "@destack/language/core";
import {
  ACTIVE_SPACE,
  Entity,
  Event,
  EventStatus,
  Node,
  NodeType,
  StructType,
} from "@destack/language/core";
import { InputEvent } from "@destack/language/interaction/input";
import type { Script } from "@destack/language/logic";
import { STRUCT_CLASS_BY_TYPE, registerNodeClass } from "@destack/language/registry";
import type { Client } from "@destack/language/universe";
import type { View } from "@destack/language/view";
import {
  EventStatusProto,
  KeyDownEventProto,
  KeyPressEventProto,
  KeyUpEventProto,
} from "@destack/proto";
import { base64Decode } from "@destack/utils";
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
   * The Branch this Event originated from.
   */
  abstract get branch(): Branch | null;
  declare readonly branchPtr: NodeReference | null;

  /**
   * The Snapshot this Event originated from.
   */
  abstract get snapshot(): Snapshot | null;
  declare readonly snapshotPtr: NodeReference | null;

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
   * The time this Event was created (set by the system).
   */
  declare readonly createdAt: Temporal.ZonedDateTime;

  /**
   * The logical time this Event was created (set by the system).
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
   * The custom Values of this Node, keyed by custom Property id. May hold both static and instance values.
   */
  declare readonly customValues: { readonly [key: string]: Value };

  /**
   * The status of the Event.
   */
  declare readonly status: EventStatus;

  /**
   * The main / root Script of this Node.
   */
  abstract get script(): Script | null;
  declare readonly scriptPtr: NodeReference | null;

  /**
   * Whether this Node is extensible (whether it can be instanced).
   */
  declare readonly isExtensible: boolean;

  /**
   * InputEvent.node
   */
  abstract get node(): View | null;
  declare readonly nodePtr: NodeReference | null;

  /**
   * The character that was pressed (e.g. 'a', 'B', '1', 'Enter').
   */
  declare readonly key: string;

  /**
   * The unaltered key code that was pressed (e.g. 'KeyA', 'KeyB', 'Digit1', 'Enter').
   */
  declare readonly code: string;

  /**
   * Whether the key is being held down.
   */
  declare readonly isRepeat: boolean;

  /**
   * Whether the key was masked for some reason (e.g., security, privacy).
   */
  declare readonly isRedacted: boolean;

  /**
   * Whether the Shift key was held.
   */
  declare readonly shiftKey: boolean;

  /**
   * Whether the Alt key was held.
   */
  declare readonly altKey: boolean;

  /**
   * Whether the Ctrl key was held.
   */
  declare readonly ctrlKey: boolean;

  /**
   * Whether the Meta key was held.
   */
  declare readonly metaKey: boolean;

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
      return this._supergraph.get(nodePtr.id) as Space | null;
    }
    return null;
  }
  readonly spacePtr: NodeReference;

  /**
   * The Branch this Event originated from.
   */
  get branch(): Branch | null {
    const nodePtr: NodeReference | null = this.branchPtr;
    if (nodePtr != null) {
      return this._supergraph.get(nodePtr.id) as Branch | null;
    }
    return null;
  }
  readonly branchPtr: NodeReference | null;

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
   * The previous Event that this Event follows.
   */
  get precededBy(): Event | null {
    const nodePtr: NodeReference | null = this.precededByPtr;
    if (nodePtr != null) {
      return this._supergraph.get(nodePtr.id) as Event | null;
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
      return this._supergraph.get(nodePtr.id) as Event | null;
    }
    return null;
  }
  readonly causedByPtr: NodeReference | null;

  /**
   * The time this Event was created (set by the system).
   */
  readonly createdAt: Temporal.ZonedDateTime;

  /**
   * The logical time this Event was created (set by the system).
   */
  readonly createdEpoch: number;

  /**
   * The Actor that created this Event.
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
   * The Client that created this Event.
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
   * The custom Values of this Node, keyed by custom Property id. May hold both static and instance values.
   */
  readonly customValues: { readonly [key: string]: Value };

  /**
   * The status of the Event.
   */
  readonly status: EventStatus;

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
  readonly scriptPtr: NodeReference | null;

  /**
   * Whether this Node is extensible (whether it can be instanced).
   */
  readonly isExtensible: boolean;

  /**
   * InputEvent.node
   */
  get node(): View | null {
    const nodePtr: NodeReference | null = this.nodePtr;
    if (nodePtr != null) {
      return this._supergraph.get(nodePtr.id) as View | null;
    }
    return null;
  }
  readonly nodePtr: NodeReference | null;

  /**
   * The character that was pressed (e.g. 'a', 'B', '1', 'Enter').
   */
  readonly key: string;

  /**
   * The unaltered key code that was pressed (e.g. 'KeyA', 'KeyB', 'Digit1', 'Enter').
   */
  readonly code: string;

  /**
   * Whether the key is being held down.
   */
  readonly isRepeat: boolean;

  /**
   * Whether the key was masked for some reason (e.g., security, privacy).
   */
  readonly isRedacted: boolean;

  /**
   * Whether the Shift key was held.
   */
  readonly shiftKey: boolean;

  /**
   * Whether the Alt key was held.
   */
  readonly altKey: boolean;

  /**
   * Whether the Ctrl key was held.
   */
  readonly ctrlKey: boolean;

  /**
   * Whether the Meta key was held.
   */
  readonly metaKey: boolean;

  constructor(options: {
    id?: string;
    space?: Space | NodeReference;
    branch?: Branch | NodeReference | null;
    snapshot?: Snapshot | NodeReference | null;
    precededBy?: Event | NodeReference | null;
    causedBy?: Event | NodeReference | null;
    createdAt?: Temporal.ZonedDateTime;
    createdEpoch?: number;
    createdBy?: (Entity & IsActor) | NodeReference | null;
    client?: Client | NodeReference | null;
    clientNonce?: string | null;
    clientCreatedAt?: Temporal.ZonedDateTime;
    clientEpoch?: number;
    customValues?: { readonly [key: string]: Value };
    status?: EventStatus;
    script?: Script | NodeReference | null;
    isExtensible?: boolean;
    node?: View | NodeReference | null;
    key: string;
    code: string;
    isRepeat: boolean;
    isRedacted: boolean;
    shiftKey: boolean;
    altKey: boolean;
    ctrlKey: boolean;
    metaKey: boolean;
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
      _space = ACTIVE_SPACE.get();
      if (_space === null) {
        throw new Error(`no active Space for KeyDownEvent`);
      }
      _space = _space.toRef();
    }
    if (_space === null) {
      throw new Error(`KeyDownEvent.space is required`);
    }
    this.spacePtr = _space;
    let _branch = options.branch ?? null;
    if (_branch != null && _branch.metatype != StructType.NODE_REFERENCE) {
      _branch = (_branch as Node).toRef();
    }
    this.branchPtr = _branch;
    let _snapshot = options.snapshot ?? null;
    if (_snapshot != null && _snapshot.metatype != StructType.NODE_REFERENCE) {
      _snapshot = (_snapshot as Node).toRef();
    }
    this.snapshotPtr = _snapshot;
    let _precededBy = options.precededBy ?? null;
    if (_precededBy != null && _precededBy.metatype != StructType.NODE_REFERENCE) {
      _precededBy = (_precededBy as Node).toRef();
    }
    this.precededByPtr = _precededBy;
    let _causedBy = options.causedBy ?? null;
    if (_causedBy != null && _causedBy.metatype != StructType.NODE_REFERENCE) {
      _causedBy = (_causedBy as Node).toRef();
    }
    this.causedByPtr = _causedBy;
    let _client = options.client ?? null;
    if (_client != null && _client.metatype != StructType.NODE_REFERENCE) {
      _client = (_client as Node).toRef();
    }
    this.clientPtr = _client;
    let _clientNonce = options.clientNonce ?? null;
    this.clientNonce = _clientNonce;
    let _customValues = options.customValues ?? null;
    if (_customValues === null) {
      _customValues = {};
    }
    this.customValues = _customValues;
    let _status = options.status ?? null;
    if (_status === null) {
      _status = 1 /* EventStatus.PENDING */;
    }
    if (_status === null) {
      throw new Error(`KeyDownEvent.status is required`);
    }
    this.status = _status;
    let _script = options.script ?? null;
    if (_script != null && _script.metatype != StructType.NODE_REFERENCE) {
      _script = (_script as Node).toRef();
    }
    this.scriptPtr = _script;
    let _isExtensible = options.isExtensible ?? null;
    if (_isExtensible === null) {
      _isExtensible = false;
    }
    if (_isExtensible === null) {
      throw new Error(`KeyDownEvent.isExtensible is required`);
    }
    this.isExtensible = _isExtensible;
    let _node = options.node ?? null;
    if (_node != null && _node.metatype != StructType.NODE_REFERENCE) {
      _node = (_node as Node).toRef();
    }
    this.nodePtr = _node;
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

    // identity
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
        throw new Error(`KeyDownEvent.createdAt is required for existing Events`);
      }
      this.createdAt = options.createdAt;
      this.createdEpoch = options.createdEpoch;
      this.createdByPtr =
        options.createdBy != null
          ? options.createdBy.metatype == StructType.NODE_REFERENCE
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
    if (!(this.isExtensible === other.isExtensible)) {
      return false;
    }
    if (!(this.branchPtr?.id === other.branchPtr?.id)) {
      return false;
    }
    if (!(this.snapshotPtr?.id === other.snapshotPtr?.id)) {
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
    if (!(this.scriptPtr?.id === other.scriptPtr?.id)) {
      return false;
    }
    if (!(this.spacePtr.id === other.spacePtr.id)) {
      return false;
    }
    if (Object.keys(this.customValues).length !== Object.keys(other.customValues).length) {
      return false;
    }
    for (const key in this.customValues) {
      if (!(key in other.customValues)) {
        return false;
      }
      if (!this.customValues[key].equals(other.customValues[key])) {
        return false;
      }
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
    h = (h * 31 + hashBool(this.isExtensible)) & 0xffffffff;
    if (this.branchPtr != null) {
      h = (h * 31 + hashString(this.branchPtr.id)) & 0xffffffff;
    }
    if (this.snapshotPtr != null) {
      h = (h * 31 + hashString(this.snapshotPtr.id)) & 0xffffffff;
    }
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
    if (this.scriptPtr != null) {
      h = (h * 31 + hashString(this.scriptPtr.id)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.id.toString())) & 0xffffffff;
    h = (h * 31 + hashString(this.spacePtr.id)) & 0xffffffff;
    if (this.customValues && Object.keys(this.customValues).length > 0) {
      for (const [_key, _value] of Object.entries(this.customValues)) {
        h = (h * 31 + hashString(_key.toString())) & 0xffffffff;
        h = (h * 31 + _value.hash()) & 0xffffffff;
      }
    }

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
      spaceId: this.spacePtr?.id ?? null,
      branchId: this.branchPtr?.id ?? null,
      snapshotId: this.snapshotPtr?.id ?? null,
      definitionId: this.definitionPtr?.id ?? null,
      _session: this._session,
      _supergraph: this._supergraph,
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

  toValue(): { readonly [key: string]: any } {
    return KeyDownEvent.__packValue__(this);
  }

  static __packValue__(object: KeyDownEvent): { readonly [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 2000301;
    objectValue["2"] = String(object.id);
    objectValue["5"] = object.spacePtr.toValue();
    if (object.branchPtr != null) {
      objectValue["10"] = object.branchPtr.toValue();
    }
    if (object.snapshotPtr != null) {
      objectValue["11"] = object.snapshotPtr.toValue();
    }
    if (object.precededByPtr != null) {
      objectValue["12"] = object.precededByPtr.toValue();
    }
    if (object.causedByPtr != null) {
      objectValue["13"] = object.causedByPtr.toValue();
    }
    objectValue["20"] = object.createdAt.toString({ timeZoneName: "never" });
    objectValue["21"] = object.createdEpoch;
    if (object.createdByPtr != null) {
      objectValue["22"] = object.createdByPtr.toValue();
    }
    if (object.clientPtr != null) {
      objectValue["23"] = object.clientPtr.toValue();
    }
    if (object.clientNonce != null) {
      objectValue["24"] = String(object.clientNonce);
    }
    objectValue["25"] = object.clientCreatedAt.toString({ timeZoneName: "never" });
    objectValue["26"] = object.clientEpoch;
    if (Object.keys(object.customValues).length > 0) {
      const packedCustomValues: { [key: string]: any } = {} as any;
      for (const [key, value] of Object.entries(object.customValues)) {
        packedCustomValues[String(String(key))] = value.toValue();
      }
      objectValue["30"] = packedCustomValues;
    }
    objectValue["40"] = object.status;
    if (object.scriptPtr != null) {
      objectValue["80"] = object.scriptPtr.toValue();
    }
    objectValue["90"] = object.isExtensible;
    if (object.nodePtr != null) {
      objectValue["101"] = object.nodePtr.toValue();
    }
    objectValue["110"] = object.key;
    objectValue["111"] = object.code;
    objectValue["112"] = object.isRepeat;
    objectValue["113"] = object.isRedacted;
    objectValue["120"] = object.shiftKey;
    objectValue["121"] = object.altKey;
    objectValue["122"] = object.ctrlKey;
    objectValue["123"] = object.metaKey;
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { readonly [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): KeyDownEvent {
    const _Value = STRUCT_CLASS_BY_TYPE[StructType.VALUE] as typeof Value;
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    const nodePtrValue = objectValue["101"];
    const unpackedNodePtr =
      nodePtrValue != undefined
        ? _NodeReference.fromValue(nodePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const branchPtrValue = objectValue["10"];
    const unpackedBranchPtr =
      branchPtrValue != undefined
        ? _NodeReference.fromValue(branchPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const snapshotPtrValue = objectValue["11"];
    const unpackedSnapshotPtr =
      snapshotPtrValue != undefined
        ? _NodeReference.fromValue(snapshotPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const precededByPtrValue = objectValue["12"];
    const unpackedPrecededByPtr =
      precededByPtrValue != undefined
        ? _NodeReference.fromValue(precededByPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const causedByPtrValue = objectValue["13"];
    const unpackedCausedByPtr =
      causedByPtrValue != undefined
        ? _NodeReference.fromValue(causedByPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const createdByPtrValue = objectValue["22"];
    const unpackedCreatedByPtr =
      createdByPtrValue != undefined
        ? _NodeReference.fromValue(createdByPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const clientPtrValue = objectValue["23"];
    const unpackedClientPtr =
      clientPtrValue != undefined
        ? _NodeReference.fromValue(clientPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const clientNonceValue = objectValue["24"];
    const unpackedClientNonce = clientNonceValue != undefined ? String(clientNonceValue) : null;
    const scriptPtrValue = objectValue["80"];
    const unpackedScriptPtr =
      scriptPtrValue != undefined
        ? _NodeReference.fromValue(scriptPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const unpackedCustomValues = {} as any;
    if (objectValue["30"] != undefined) {
      for (const [key, value] of Object.entries(objectValue["30"])) {
        unpackedCustomValues[String(key)] = _Value.fromValue(
          value as any,
          _session,
          _supergraph,
          _graph,
          _connection,
        );
      }
    }
    return new KeyDownEvent({
      key: objectValue["110"],
      code: objectValue["111"],
      isRepeat: objectValue["112"],
      isRedacted: objectValue["113"],
      shiftKey: objectValue["120"],
      altKey: objectValue["121"],
      ctrlKey: objectValue["122"],
      metaKey: objectValue["123"],
      node: unpackedNodePtr,
      isExtensible: objectValue["90"],
      branch: unpackedBranchPtr,
      snapshot: unpackedSnapshotPtr,
      precededBy: unpackedPrecededByPtr,
      causedBy: unpackedCausedByPtr,
      createdAt: Temporal.Instant.from(objectValue["20"]).toZonedDateTimeISO("UTC"),
      createdEpoch: Number(objectValue["21"]),
      createdBy: unpackedCreatedByPtr,
      client: unpackedClientPtr,
      clientNonce: unpackedClientNonce,
      clientCreatedAt: Temporal.Instant.from(objectValue["25"]).toZonedDateTimeISO("UTC"),
      clientEpoch: Number(objectValue["26"]),
      status: Number(objectValue["40"]),
      script: unpackedScriptPtr,
      id: String(objectValue["2"]),
      space: _NodeReference.fromValue(objectValue["5"], _session, _supergraph, _graph, _connection),
      customValues: unpackedCustomValues,
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
  ): KeyDownEvent {
    return KeyDownEvent.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
  }

  toProto(): KeyDownEventProto {
    return KeyDownEvent.__packProto__(this);
  }

  static __packProto__(object: KeyDownEvent): KeyDownEventProto {
    const objectProto: Partial<KeyDownEventProto> = { metatype: 2000301 };
    objectProto.id = String(object.id);
    objectProto.spacePtr = object.spacePtr.toProto();
    if (object.branchPtr != null) {
      objectProto.branchPtr = object.branchPtr.toProto();
    }
    if (object.snapshotPtr != null) {
      objectProto.snapshotPtr = object.snapshotPtr.toProto();
    }
    if (object.precededByPtr != null) {
      objectProto.precededByPtr = object.precededByPtr.toProto();
    }
    if (object.causedByPtr != null) {
      objectProto.causedByPtr = object.causedByPtr.toProto();
    }
    objectProto.createdAt = packProtoTimestamp(object.createdAt);
    objectProto.createdEpoch = object.createdEpoch;
    if (object.createdByPtr != null) {
      objectProto.createdByPtr = object.createdByPtr.toProto();
    }
    if (object.clientPtr != null) {
      objectProto.clientPtr = object.clientPtr.toProto();
    }
    if (object.clientNonce != null) {
      objectProto.clientNonce = String(object.clientNonce);
    }
    objectProto.clientCreatedAt = packProtoTimestamp(object.clientCreatedAt);
    objectProto.clientEpoch = object.clientEpoch;
    if (object.customValues) {
      objectProto.customValues = {} as any;
      for (const [key, value] of Object.entries(object.customValues)) {
        objectProto.customValues![String(key)] = value.toProto();
      }
    }
    objectProto.status = Number(object.status) as EventStatusProto;
    if (object.scriptPtr != null) {
      objectProto.scriptPtr = object.scriptPtr.toProto();
    }
    objectProto.isExtensible = object.isExtensible;
    if (object.nodePtr != null) {
      objectProto.nodePtr = object.nodePtr.toProto();
    }
    objectProto.key = object.key;
    objectProto.code = object.code;
    objectProto.isRepeat = object.isRepeat;
    objectProto.isRedacted = object.isRedacted;
    objectProto.shiftKey = object.shiftKey;
    objectProto.altKey = object.altKey;
    objectProto.ctrlKey = object.ctrlKey;
    objectProto.metaKey = object.metaKey;
    return objectProto as KeyDownEventProto;
  }

  static __unpackProto__(
    objectProto: KeyDownEventProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): KeyDownEvent {
    const _Value = STRUCT_CLASS_BY_TYPE[StructType.VALUE] as typeof Value;
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    const unpackedCustomValues = {} as any;
    if (objectProto.customValues) {
      for (const [key, value] of Object.entries(objectProto.customValues)) {
        unpackedCustomValues.set(
          String(key),
          _Value.fromProto((value as any)!, _session, _supergraph, _graph, _connection),
        );
      }
    }
    return new KeyDownEvent({
      key: objectProto.key,
      code: objectProto.code,
      isRepeat: objectProto.isRepeat,
      isRedacted: objectProto.isRedacted,
      shiftKey: objectProto.shiftKey,
      altKey: objectProto.altKey,
      ctrlKey: objectProto.ctrlKey,
      metaKey: objectProto.metaKey,
      node:
        objectProto.nodePtr != undefined
          ? _NodeReference.fromProto(
              objectProto.nodePtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      isExtensible: objectProto.isExtensible,
      branch:
        objectProto.branchPtr != undefined
          ? _NodeReference.fromProto(
              objectProto.branchPtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
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
      precededBy:
        objectProto.precededByPtr != undefined
          ? _NodeReference.fromProto(
              objectProto.precededByPtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      causedBy:
        objectProto.causedByPtr != undefined
          ? _NodeReference.fromProto(
              objectProto.causedByPtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      createdAt: unpackProtoTimestamp(objectProto.createdAt!),
      createdEpoch: Number(objectProto.createdEpoch),
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
      clientCreatedAt: unpackProtoTimestamp(objectProto.clientCreatedAt!),
      clientEpoch: Number(objectProto.clientEpoch),
      status: Number(objectProto.status) as EventStatus,
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
      id: String(objectProto.id),
      space: _NodeReference.fromProto(
        objectProto.spacePtr!,
        _session,
        _supergraph,
        _graph,
        _connection,
      ),
      customValues: unpackedCustomValues,
      _session,
      _graph,
      _connection,
    });
  }

  static fromProto(
    objectProto: KeyDownEventProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): KeyDownEvent {
    return KeyDownEvent.__unpackProto__(objectProto, _session, _supergraph, _graph, _connection);
  }

  static fromProtoString(packedProtoString: string): KeyDownEvent {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = KeyDownEventProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
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
      return this._supergraph.get(nodePtr.id) as Space | null;
    }
    return null;
  }
  readonly spacePtr: NodeReference;

  /**
   * The Branch this Event originated from.
   */
  get branch(): Branch | null {
    const nodePtr: NodeReference | null = this.branchPtr;
    if (nodePtr != null) {
      return this._supergraph.get(nodePtr.id) as Branch | null;
    }
    return null;
  }
  readonly branchPtr: NodeReference | null;

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
   * The previous Event that this Event follows.
   */
  get precededBy(): Event | null {
    const nodePtr: NodeReference | null = this.precededByPtr;
    if (nodePtr != null) {
      return this._supergraph.get(nodePtr.id) as Event | null;
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
      return this._supergraph.get(nodePtr.id) as Event | null;
    }
    return null;
  }
  readonly causedByPtr: NodeReference | null;

  /**
   * The time this Event was created (set by the system).
   */
  readonly createdAt: Temporal.ZonedDateTime;

  /**
   * The logical time this Event was created (set by the system).
   */
  readonly createdEpoch: number;

  /**
   * The Actor that created this Event.
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
   * The Client that created this Event.
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
   * The custom Values of this Node, keyed by custom Property id. May hold both static and instance values.
   */
  readonly customValues: { readonly [key: string]: Value };

  /**
   * The status of the Event.
   */
  readonly status: EventStatus;

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
  readonly scriptPtr: NodeReference | null;

  /**
   * Whether this Node is extensible (whether it can be instanced).
   */
  readonly isExtensible: boolean;

  /**
   * InputEvent.node
   */
  get node(): View | null {
    const nodePtr: NodeReference | null = this.nodePtr;
    if (nodePtr != null) {
      return this._supergraph.get(nodePtr.id) as View | null;
    }
    return null;
  }
  readonly nodePtr: NodeReference | null;

  /**
   * The character that was pressed (e.g. 'a', 'B', '1', 'Enter').
   */
  readonly key: string;

  /**
   * The unaltered key code that was pressed (e.g. 'KeyA', 'KeyB', 'Digit1', 'Enter').
   */
  readonly code: string;

  /**
   * Whether the key is being held down.
   */
  readonly isRepeat: boolean;

  /**
   * Whether the key was masked for some reason (e.g., security, privacy).
   */
  readonly isRedacted: boolean;

  /**
   * Whether the Shift key was held.
   */
  readonly shiftKey: boolean;

  /**
   * Whether the Alt key was held.
   */
  readonly altKey: boolean;

  /**
   * Whether the Ctrl key was held.
   */
  readonly ctrlKey: boolean;

  /**
   * Whether the Meta key was held.
   */
  readonly metaKey: boolean;

  constructor(options: {
    id?: string;
    space?: Space | NodeReference;
    branch?: Branch | NodeReference | null;
    snapshot?: Snapshot | NodeReference | null;
    precededBy?: Event | NodeReference | null;
    causedBy?: Event | NodeReference | null;
    createdAt?: Temporal.ZonedDateTime;
    createdEpoch?: number;
    createdBy?: (Entity & IsActor) | NodeReference | null;
    client?: Client | NodeReference | null;
    clientNonce?: string | null;
    clientCreatedAt?: Temporal.ZonedDateTime;
    clientEpoch?: number;
    customValues?: { readonly [key: string]: Value };
    status?: EventStatus;
    script?: Script | NodeReference | null;
    isExtensible?: boolean;
    node?: View | NodeReference | null;
    key: string;
    code: string;
    isRepeat: boolean;
    isRedacted: boolean;
    shiftKey: boolean;
    altKey: boolean;
    ctrlKey: boolean;
    metaKey: boolean;
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
      _space = ACTIVE_SPACE.get();
      if (_space === null) {
        throw new Error(`no active Space for KeyUpEvent`);
      }
      _space = _space.toRef();
    }
    if (_space === null) {
      throw new Error(`KeyUpEvent.space is required`);
    }
    this.spacePtr = _space;
    let _branch = options.branch ?? null;
    if (_branch != null && _branch.metatype != StructType.NODE_REFERENCE) {
      _branch = (_branch as Node).toRef();
    }
    this.branchPtr = _branch;
    let _snapshot = options.snapshot ?? null;
    if (_snapshot != null && _snapshot.metatype != StructType.NODE_REFERENCE) {
      _snapshot = (_snapshot as Node).toRef();
    }
    this.snapshotPtr = _snapshot;
    let _precededBy = options.precededBy ?? null;
    if (_precededBy != null && _precededBy.metatype != StructType.NODE_REFERENCE) {
      _precededBy = (_precededBy as Node).toRef();
    }
    this.precededByPtr = _precededBy;
    let _causedBy = options.causedBy ?? null;
    if (_causedBy != null && _causedBy.metatype != StructType.NODE_REFERENCE) {
      _causedBy = (_causedBy as Node).toRef();
    }
    this.causedByPtr = _causedBy;
    let _client = options.client ?? null;
    if (_client != null && _client.metatype != StructType.NODE_REFERENCE) {
      _client = (_client as Node).toRef();
    }
    this.clientPtr = _client;
    let _clientNonce = options.clientNonce ?? null;
    this.clientNonce = _clientNonce;
    let _customValues = options.customValues ?? null;
    if (_customValues === null) {
      _customValues = {};
    }
    this.customValues = _customValues;
    let _status = options.status ?? null;
    if (_status === null) {
      _status = 1 /* EventStatus.PENDING */;
    }
    if (_status === null) {
      throw new Error(`KeyUpEvent.status is required`);
    }
    this.status = _status;
    let _script = options.script ?? null;
    if (_script != null && _script.metatype != StructType.NODE_REFERENCE) {
      _script = (_script as Node).toRef();
    }
    this.scriptPtr = _script;
    let _isExtensible = options.isExtensible ?? null;
    if (_isExtensible === null) {
      _isExtensible = false;
    }
    if (_isExtensible === null) {
      throw new Error(`KeyUpEvent.isExtensible is required`);
    }
    this.isExtensible = _isExtensible;
    let _node = options.node ?? null;
    if (_node != null && _node.metatype != StructType.NODE_REFERENCE) {
      _node = (_node as Node).toRef();
    }
    this.nodePtr = _node;
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

    // identity
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
        throw new Error(`KeyUpEvent.createdAt is required for existing Events`);
      }
      this.createdAt = options.createdAt;
      this.createdEpoch = options.createdEpoch;
      this.createdByPtr =
        options.createdBy != null
          ? options.createdBy.metatype == StructType.NODE_REFERENCE
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
    if (!(this.isExtensible === other.isExtensible)) {
      return false;
    }
    if (!(this.branchPtr?.id === other.branchPtr?.id)) {
      return false;
    }
    if (!(this.snapshotPtr?.id === other.snapshotPtr?.id)) {
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
    if (!(this.scriptPtr?.id === other.scriptPtr?.id)) {
      return false;
    }
    if (!(this.spacePtr.id === other.spacePtr.id)) {
      return false;
    }
    if (Object.keys(this.customValues).length !== Object.keys(other.customValues).length) {
      return false;
    }
    for (const key in this.customValues) {
      if (!(key in other.customValues)) {
        return false;
      }
      if (!this.customValues[key].equals(other.customValues[key])) {
        return false;
      }
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
    h = (h * 31 + hashBool(this.isExtensible)) & 0xffffffff;
    if (this.branchPtr != null) {
      h = (h * 31 + hashString(this.branchPtr.id)) & 0xffffffff;
    }
    if (this.snapshotPtr != null) {
      h = (h * 31 + hashString(this.snapshotPtr.id)) & 0xffffffff;
    }
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
    if (this.scriptPtr != null) {
      h = (h * 31 + hashString(this.scriptPtr.id)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.id.toString())) & 0xffffffff;
    h = (h * 31 + hashString(this.spacePtr.id)) & 0xffffffff;
    if (this.customValues && Object.keys(this.customValues).length > 0) {
      for (const [_key, _value] of Object.entries(this.customValues)) {
        h = (h * 31 + hashString(_key.toString())) & 0xffffffff;
        h = (h * 31 + _value.hash()) & 0xffffffff;
      }
    }

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
      spaceId: this.spacePtr?.id ?? null,
      branchId: this.branchPtr?.id ?? null,
      snapshotId: this.snapshotPtr?.id ?? null,
      definitionId: this.definitionPtr?.id ?? null,
      _session: this._session,
      _supergraph: this._supergraph,
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

  toValue(): { readonly [key: string]: any } {
    return KeyUpEvent.__packValue__(this);
  }

  static __packValue__(object: KeyUpEvent): { readonly [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 2000302;
    objectValue["2"] = String(object.id);
    objectValue["5"] = object.spacePtr.toValue();
    if (object.branchPtr != null) {
      objectValue["10"] = object.branchPtr.toValue();
    }
    if (object.snapshotPtr != null) {
      objectValue["11"] = object.snapshotPtr.toValue();
    }
    if (object.precededByPtr != null) {
      objectValue["12"] = object.precededByPtr.toValue();
    }
    if (object.causedByPtr != null) {
      objectValue["13"] = object.causedByPtr.toValue();
    }
    objectValue["20"] = object.createdAt.toString({ timeZoneName: "never" });
    objectValue["21"] = object.createdEpoch;
    if (object.createdByPtr != null) {
      objectValue["22"] = object.createdByPtr.toValue();
    }
    if (object.clientPtr != null) {
      objectValue["23"] = object.clientPtr.toValue();
    }
    if (object.clientNonce != null) {
      objectValue["24"] = String(object.clientNonce);
    }
    objectValue["25"] = object.clientCreatedAt.toString({ timeZoneName: "never" });
    objectValue["26"] = object.clientEpoch;
    if (Object.keys(object.customValues).length > 0) {
      const packedCustomValues: { [key: string]: any } = {} as any;
      for (const [key, value] of Object.entries(object.customValues)) {
        packedCustomValues[String(String(key))] = value.toValue();
      }
      objectValue["30"] = packedCustomValues;
    }
    objectValue["40"] = object.status;
    if (object.scriptPtr != null) {
      objectValue["80"] = object.scriptPtr.toValue();
    }
    objectValue["90"] = object.isExtensible;
    if (object.nodePtr != null) {
      objectValue["101"] = object.nodePtr.toValue();
    }
    objectValue["110"] = object.key;
    objectValue["111"] = object.code;
    objectValue["112"] = object.isRepeat;
    objectValue["113"] = object.isRedacted;
    objectValue["120"] = object.shiftKey;
    objectValue["121"] = object.altKey;
    objectValue["122"] = object.ctrlKey;
    objectValue["123"] = object.metaKey;
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { readonly [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): KeyUpEvent {
    const _Value = STRUCT_CLASS_BY_TYPE[StructType.VALUE] as typeof Value;
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    const nodePtrValue = objectValue["101"];
    const unpackedNodePtr =
      nodePtrValue != undefined
        ? _NodeReference.fromValue(nodePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const branchPtrValue = objectValue["10"];
    const unpackedBranchPtr =
      branchPtrValue != undefined
        ? _NodeReference.fromValue(branchPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const snapshotPtrValue = objectValue["11"];
    const unpackedSnapshotPtr =
      snapshotPtrValue != undefined
        ? _NodeReference.fromValue(snapshotPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const precededByPtrValue = objectValue["12"];
    const unpackedPrecededByPtr =
      precededByPtrValue != undefined
        ? _NodeReference.fromValue(precededByPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const causedByPtrValue = objectValue["13"];
    const unpackedCausedByPtr =
      causedByPtrValue != undefined
        ? _NodeReference.fromValue(causedByPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const createdByPtrValue = objectValue["22"];
    const unpackedCreatedByPtr =
      createdByPtrValue != undefined
        ? _NodeReference.fromValue(createdByPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const clientPtrValue = objectValue["23"];
    const unpackedClientPtr =
      clientPtrValue != undefined
        ? _NodeReference.fromValue(clientPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const clientNonceValue = objectValue["24"];
    const unpackedClientNonce = clientNonceValue != undefined ? String(clientNonceValue) : null;
    const scriptPtrValue = objectValue["80"];
    const unpackedScriptPtr =
      scriptPtrValue != undefined
        ? _NodeReference.fromValue(scriptPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const unpackedCustomValues = {} as any;
    if (objectValue["30"] != undefined) {
      for (const [key, value] of Object.entries(objectValue["30"])) {
        unpackedCustomValues[String(key)] = _Value.fromValue(
          value as any,
          _session,
          _supergraph,
          _graph,
          _connection,
        );
      }
    }
    return new KeyUpEvent({
      key: objectValue["110"],
      code: objectValue["111"],
      isRepeat: objectValue["112"],
      isRedacted: objectValue["113"],
      shiftKey: objectValue["120"],
      altKey: objectValue["121"],
      ctrlKey: objectValue["122"],
      metaKey: objectValue["123"],
      node: unpackedNodePtr,
      isExtensible: objectValue["90"],
      branch: unpackedBranchPtr,
      snapshot: unpackedSnapshotPtr,
      precededBy: unpackedPrecededByPtr,
      causedBy: unpackedCausedByPtr,
      createdAt: Temporal.Instant.from(objectValue["20"]).toZonedDateTimeISO("UTC"),
      createdEpoch: Number(objectValue["21"]),
      createdBy: unpackedCreatedByPtr,
      client: unpackedClientPtr,
      clientNonce: unpackedClientNonce,
      clientCreatedAt: Temporal.Instant.from(objectValue["25"]).toZonedDateTimeISO("UTC"),
      clientEpoch: Number(objectValue["26"]),
      status: Number(objectValue["40"]),
      script: unpackedScriptPtr,
      id: String(objectValue["2"]),
      space: _NodeReference.fromValue(objectValue["5"], _session, _supergraph, _graph, _connection),
      customValues: unpackedCustomValues,
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
  ): KeyUpEvent {
    return KeyUpEvent.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
  }

  toProto(): KeyUpEventProto {
    return KeyUpEvent.__packProto__(this);
  }

  static __packProto__(object: KeyUpEvent): KeyUpEventProto {
    const objectProto: Partial<KeyUpEventProto> = { metatype: 2000302 };
    objectProto.id = String(object.id);
    objectProto.spacePtr = object.spacePtr.toProto();
    if (object.branchPtr != null) {
      objectProto.branchPtr = object.branchPtr.toProto();
    }
    if (object.snapshotPtr != null) {
      objectProto.snapshotPtr = object.snapshotPtr.toProto();
    }
    if (object.precededByPtr != null) {
      objectProto.precededByPtr = object.precededByPtr.toProto();
    }
    if (object.causedByPtr != null) {
      objectProto.causedByPtr = object.causedByPtr.toProto();
    }
    objectProto.createdAt = packProtoTimestamp(object.createdAt);
    objectProto.createdEpoch = object.createdEpoch;
    if (object.createdByPtr != null) {
      objectProto.createdByPtr = object.createdByPtr.toProto();
    }
    if (object.clientPtr != null) {
      objectProto.clientPtr = object.clientPtr.toProto();
    }
    if (object.clientNonce != null) {
      objectProto.clientNonce = String(object.clientNonce);
    }
    objectProto.clientCreatedAt = packProtoTimestamp(object.clientCreatedAt);
    objectProto.clientEpoch = object.clientEpoch;
    if (object.customValues) {
      objectProto.customValues = {} as any;
      for (const [key, value] of Object.entries(object.customValues)) {
        objectProto.customValues![String(key)] = value.toProto();
      }
    }
    objectProto.status = Number(object.status) as EventStatusProto;
    if (object.scriptPtr != null) {
      objectProto.scriptPtr = object.scriptPtr.toProto();
    }
    objectProto.isExtensible = object.isExtensible;
    if (object.nodePtr != null) {
      objectProto.nodePtr = object.nodePtr.toProto();
    }
    objectProto.key = object.key;
    objectProto.code = object.code;
    objectProto.isRepeat = object.isRepeat;
    objectProto.isRedacted = object.isRedacted;
    objectProto.shiftKey = object.shiftKey;
    objectProto.altKey = object.altKey;
    objectProto.ctrlKey = object.ctrlKey;
    objectProto.metaKey = object.metaKey;
    return objectProto as KeyUpEventProto;
  }

  static __unpackProto__(
    objectProto: KeyUpEventProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): KeyUpEvent {
    const _Value = STRUCT_CLASS_BY_TYPE[StructType.VALUE] as typeof Value;
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    const unpackedCustomValues = {} as any;
    if (objectProto.customValues) {
      for (const [key, value] of Object.entries(objectProto.customValues)) {
        unpackedCustomValues.set(
          String(key),
          _Value.fromProto((value as any)!, _session, _supergraph, _graph, _connection),
        );
      }
    }
    return new KeyUpEvent({
      key: objectProto.key,
      code: objectProto.code,
      isRepeat: objectProto.isRepeat,
      isRedacted: objectProto.isRedacted,
      shiftKey: objectProto.shiftKey,
      altKey: objectProto.altKey,
      ctrlKey: objectProto.ctrlKey,
      metaKey: objectProto.metaKey,
      node:
        objectProto.nodePtr != undefined
          ? _NodeReference.fromProto(
              objectProto.nodePtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      isExtensible: objectProto.isExtensible,
      branch:
        objectProto.branchPtr != undefined
          ? _NodeReference.fromProto(
              objectProto.branchPtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
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
      precededBy:
        objectProto.precededByPtr != undefined
          ? _NodeReference.fromProto(
              objectProto.precededByPtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      causedBy:
        objectProto.causedByPtr != undefined
          ? _NodeReference.fromProto(
              objectProto.causedByPtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      createdAt: unpackProtoTimestamp(objectProto.createdAt!),
      createdEpoch: Number(objectProto.createdEpoch),
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
      clientCreatedAt: unpackProtoTimestamp(objectProto.clientCreatedAt!),
      clientEpoch: Number(objectProto.clientEpoch),
      status: Number(objectProto.status) as EventStatus,
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
      id: String(objectProto.id),
      space: _NodeReference.fromProto(
        objectProto.spacePtr!,
        _session,
        _supergraph,
        _graph,
        _connection,
      ),
      customValues: unpackedCustomValues,
      _session,
      _graph,
      _connection,
    });
  }

  static fromProto(
    objectProto: KeyUpEventProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): KeyUpEvent {
    return KeyUpEvent.__unpackProto__(objectProto, _session, _supergraph, _graph, _connection);
  }

  static fromProtoString(packedProtoString: string): KeyUpEvent {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = KeyUpEventProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
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
      return this._supergraph.get(nodePtr.id) as Space | null;
    }
    return null;
  }
  readonly spacePtr: NodeReference;

  /**
   * The Branch this Event originated from.
   */
  get branch(): Branch | null {
    const nodePtr: NodeReference | null = this.branchPtr;
    if (nodePtr != null) {
      return this._supergraph.get(nodePtr.id) as Branch | null;
    }
    return null;
  }
  readonly branchPtr: NodeReference | null;

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
   * The previous Event that this Event follows.
   */
  get precededBy(): Event | null {
    const nodePtr: NodeReference | null = this.precededByPtr;
    if (nodePtr != null) {
      return this._supergraph.get(nodePtr.id) as Event | null;
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
      return this._supergraph.get(nodePtr.id) as Event | null;
    }
    return null;
  }
  readonly causedByPtr: NodeReference | null;

  /**
   * The time this Event was created (set by the system).
   */
  readonly createdAt: Temporal.ZonedDateTime;

  /**
   * The logical time this Event was created (set by the system).
   */
  readonly createdEpoch: number;

  /**
   * The Actor that created this Event.
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
   * The Client that created this Event.
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
   * The custom Values of this Node, keyed by custom Property id. May hold both static and instance values.
   */
  readonly customValues: { readonly [key: string]: Value };

  /**
   * The status of the Event.
   */
  readonly status: EventStatus;

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
  readonly scriptPtr: NodeReference | null;

  /**
   * Whether this Node is extensible (whether it can be instanced).
   */
  readonly isExtensible: boolean;

  /**
   * InputEvent.node
   */
  get node(): View | null {
    const nodePtr: NodeReference | null = this.nodePtr;
    if (nodePtr != null) {
      return this._supergraph.get(nodePtr.id) as View | null;
    }
    return null;
  }
  readonly nodePtr: NodeReference | null;

  /**
   * The character that was pressed (e.g. 'a', 'B', '1', 'Enter').
   */
  readonly key: string;

  /**
   * The unaltered key code that was pressed (e.g. 'KeyA', 'KeyB', 'Digit1', 'Enter').
   */
  readonly code: string;

  /**
   * Whether the key is being held down.
   */
  readonly isRepeat: boolean;

  /**
   * Whether the key was masked for some reason (e.g., security, privacy).
   */
  readonly isRedacted: boolean;

  /**
   * Whether the Shift key was held.
   */
  readonly shiftKey: boolean;

  /**
   * Whether the Alt key was held.
   */
  readonly altKey: boolean;

  /**
   * Whether the Ctrl key was held.
   */
  readonly ctrlKey: boolean;

  /**
   * Whether the Meta key was held.
   */
  readonly metaKey: boolean;

  constructor(options: {
    id?: string;
    space?: Space | NodeReference;
    branch?: Branch | NodeReference | null;
    snapshot?: Snapshot | NodeReference | null;
    precededBy?: Event | NodeReference | null;
    causedBy?: Event | NodeReference | null;
    createdAt?: Temporal.ZonedDateTime;
    createdEpoch?: number;
    createdBy?: (Entity & IsActor) | NodeReference | null;
    client?: Client | NodeReference | null;
    clientNonce?: string | null;
    clientCreatedAt?: Temporal.ZonedDateTime;
    clientEpoch?: number;
    customValues?: { readonly [key: string]: Value };
    status?: EventStatus;
    script?: Script | NodeReference | null;
    isExtensible?: boolean;
    node?: View | NodeReference | null;
    key: string;
    code: string;
    isRepeat: boolean;
    isRedacted: boolean;
    shiftKey: boolean;
    altKey: boolean;
    ctrlKey: boolean;
    metaKey: boolean;
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
      _space = ACTIVE_SPACE.get();
      if (_space === null) {
        throw new Error(`no active Space for KeyPressEvent`);
      }
      _space = _space.toRef();
    }
    if (_space === null) {
      throw new Error(`KeyPressEvent.space is required`);
    }
    this.spacePtr = _space;
    let _branch = options.branch ?? null;
    if (_branch != null && _branch.metatype != StructType.NODE_REFERENCE) {
      _branch = (_branch as Node).toRef();
    }
    this.branchPtr = _branch;
    let _snapshot = options.snapshot ?? null;
    if (_snapshot != null && _snapshot.metatype != StructType.NODE_REFERENCE) {
      _snapshot = (_snapshot as Node).toRef();
    }
    this.snapshotPtr = _snapshot;
    let _precededBy = options.precededBy ?? null;
    if (_precededBy != null && _precededBy.metatype != StructType.NODE_REFERENCE) {
      _precededBy = (_precededBy as Node).toRef();
    }
    this.precededByPtr = _precededBy;
    let _causedBy = options.causedBy ?? null;
    if (_causedBy != null && _causedBy.metatype != StructType.NODE_REFERENCE) {
      _causedBy = (_causedBy as Node).toRef();
    }
    this.causedByPtr = _causedBy;
    let _client = options.client ?? null;
    if (_client != null && _client.metatype != StructType.NODE_REFERENCE) {
      _client = (_client as Node).toRef();
    }
    this.clientPtr = _client;
    let _clientNonce = options.clientNonce ?? null;
    this.clientNonce = _clientNonce;
    let _customValues = options.customValues ?? null;
    if (_customValues === null) {
      _customValues = {};
    }
    this.customValues = _customValues;
    let _status = options.status ?? null;
    if (_status === null) {
      _status = 1 /* EventStatus.PENDING */;
    }
    if (_status === null) {
      throw new Error(`KeyPressEvent.status is required`);
    }
    this.status = _status;
    let _script = options.script ?? null;
    if (_script != null && _script.metatype != StructType.NODE_REFERENCE) {
      _script = (_script as Node).toRef();
    }
    this.scriptPtr = _script;
    let _isExtensible = options.isExtensible ?? null;
    if (_isExtensible === null) {
      _isExtensible = false;
    }
    if (_isExtensible === null) {
      throw new Error(`KeyPressEvent.isExtensible is required`);
    }
    this.isExtensible = _isExtensible;
    let _node = options.node ?? null;
    if (_node != null && _node.metatype != StructType.NODE_REFERENCE) {
      _node = (_node as Node).toRef();
    }
    this.nodePtr = _node;
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

    // identity
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
        throw new Error(`KeyPressEvent.createdAt is required for existing Events`);
      }
      this.createdAt = options.createdAt;
      this.createdEpoch = options.createdEpoch;
      this.createdByPtr =
        options.createdBy != null
          ? options.createdBy.metatype == StructType.NODE_REFERENCE
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
    if (!(this.isExtensible === other.isExtensible)) {
      return false;
    }
    if (!(this.branchPtr?.id === other.branchPtr?.id)) {
      return false;
    }
    if (!(this.snapshotPtr?.id === other.snapshotPtr?.id)) {
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
    if (!(this.scriptPtr?.id === other.scriptPtr?.id)) {
      return false;
    }
    if (!(this.spacePtr.id === other.spacePtr.id)) {
      return false;
    }
    if (Object.keys(this.customValues).length !== Object.keys(other.customValues).length) {
      return false;
    }
    for (const key in this.customValues) {
      if (!(key in other.customValues)) {
        return false;
      }
      if (!this.customValues[key].equals(other.customValues[key])) {
        return false;
      }
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
    h = (h * 31 + hashBool(this.isExtensible)) & 0xffffffff;
    if (this.branchPtr != null) {
      h = (h * 31 + hashString(this.branchPtr.id)) & 0xffffffff;
    }
    if (this.snapshotPtr != null) {
      h = (h * 31 + hashString(this.snapshotPtr.id)) & 0xffffffff;
    }
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
    if (this.scriptPtr != null) {
      h = (h * 31 + hashString(this.scriptPtr.id)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.id.toString())) & 0xffffffff;
    h = (h * 31 + hashString(this.spacePtr.id)) & 0xffffffff;
    if (this.customValues && Object.keys(this.customValues).length > 0) {
      for (const [_key, _value] of Object.entries(this.customValues)) {
        h = (h * 31 + hashString(_key.toString())) & 0xffffffff;
        h = (h * 31 + _value.hash()) & 0xffffffff;
      }
    }

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
      spaceId: this.spacePtr?.id ?? null,
      branchId: this.branchPtr?.id ?? null,
      snapshotId: this.snapshotPtr?.id ?? null,
      definitionId: this.definitionPtr?.id ?? null,
      _session: this._session,
      _supergraph: this._supergraph,
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

  toValue(): { readonly [key: string]: any } {
    return KeyPressEvent.__packValue__(this);
  }

  static __packValue__(object: KeyPressEvent): { readonly [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 2000303;
    objectValue["2"] = String(object.id);
    objectValue["5"] = object.spacePtr.toValue();
    if (object.branchPtr != null) {
      objectValue["10"] = object.branchPtr.toValue();
    }
    if (object.snapshotPtr != null) {
      objectValue["11"] = object.snapshotPtr.toValue();
    }
    if (object.precededByPtr != null) {
      objectValue["12"] = object.precededByPtr.toValue();
    }
    if (object.causedByPtr != null) {
      objectValue["13"] = object.causedByPtr.toValue();
    }
    objectValue["20"] = object.createdAt.toString({ timeZoneName: "never" });
    objectValue["21"] = object.createdEpoch;
    if (object.createdByPtr != null) {
      objectValue["22"] = object.createdByPtr.toValue();
    }
    if (object.clientPtr != null) {
      objectValue["23"] = object.clientPtr.toValue();
    }
    if (object.clientNonce != null) {
      objectValue["24"] = String(object.clientNonce);
    }
    objectValue["25"] = object.clientCreatedAt.toString({ timeZoneName: "never" });
    objectValue["26"] = object.clientEpoch;
    if (Object.keys(object.customValues).length > 0) {
      const packedCustomValues: { [key: string]: any } = {} as any;
      for (const [key, value] of Object.entries(object.customValues)) {
        packedCustomValues[String(String(key))] = value.toValue();
      }
      objectValue["30"] = packedCustomValues;
    }
    objectValue["40"] = object.status;
    if (object.scriptPtr != null) {
      objectValue["80"] = object.scriptPtr.toValue();
    }
    objectValue["90"] = object.isExtensible;
    if (object.nodePtr != null) {
      objectValue["101"] = object.nodePtr.toValue();
    }
    objectValue["110"] = object.key;
    objectValue["111"] = object.code;
    objectValue["112"] = object.isRepeat;
    objectValue["113"] = object.isRedacted;
    objectValue["120"] = object.shiftKey;
    objectValue["121"] = object.altKey;
    objectValue["122"] = object.ctrlKey;
    objectValue["123"] = object.metaKey;
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { readonly [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): KeyPressEvent {
    const _Value = STRUCT_CLASS_BY_TYPE[StructType.VALUE] as typeof Value;
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    const nodePtrValue = objectValue["101"];
    const unpackedNodePtr =
      nodePtrValue != undefined
        ? _NodeReference.fromValue(nodePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const branchPtrValue = objectValue["10"];
    const unpackedBranchPtr =
      branchPtrValue != undefined
        ? _NodeReference.fromValue(branchPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const snapshotPtrValue = objectValue["11"];
    const unpackedSnapshotPtr =
      snapshotPtrValue != undefined
        ? _NodeReference.fromValue(snapshotPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const precededByPtrValue = objectValue["12"];
    const unpackedPrecededByPtr =
      precededByPtrValue != undefined
        ? _NodeReference.fromValue(precededByPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const causedByPtrValue = objectValue["13"];
    const unpackedCausedByPtr =
      causedByPtrValue != undefined
        ? _NodeReference.fromValue(causedByPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const createdByPtrValue = objectValue["22"];
    const unpackedCreatedByPtr =
      createdByPtrValue != undefined
        ? _NodeReference.fromValue(createdByPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const clientPtrValue = objectValue["23"];
    const unpackedClientPtr =
      clientPtrValue != undefined
        ? _NodeReference.fromValue(clientPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const clientNonceValue = objectValue["24"];
    const unpackedClientNonce = clientNonceValue != undefined ? String(clientNonceValue) : null;
    const scriptPtrValue = objectValue["80"];
    const unpackedScriptPtr =
      scriptPtrValue != undefined
        ? _NodeReference.fromValue(scriptPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const unpackedCustomValues = {} as any;
    if (objectValue["30"] != undefined) {
      for (const [key, value] of Object.entries(objectValue["30"])) {
        unpackedCustomValues[String(key)] = _Value.fromValue(
          value as any,
          _session,
          _supergraph,
          _graph,
          _connection,
        );
      }
    }
    return new KeyPressEvent({
      key: objectValue["110"],
      code: objectValue["111"],
      isRepeat: objectValue["112"],
      isRedacted: objectValue["113"],
      shiftKey: objectValue["120"],
      altKey: objectValue["121"],
      ctrlKey: objectValue["122"],
      metaKey: objectValue["123"],
      node: unpackedNodePtr,
      isExtensible: objectValue["90"],
      branch: unpackedBranchPtr,
      snapshot: unpackedSnapshotPtr,
      precededBy: unpackedPrecededByPtr,
      causedBy: unpackedCausedByPtr,
      createdAt: Temporal.Instant.from(objectValue["20"]).toZonedDateTimeISO("UTC"),
      createdEpoch: Number(objectValue["21"]),
      createdBy: unpackedCreatedByPtr,
      client: unpackedClientPtr,
      clientNonce: unpackedClientNonce,
      clientCreatedAt: Temporal.Instant.from(objectValue["25"]).toZonedDateTimeISO("UTC"),
      clientEpoch: Number(objectValue["26"]),
      status: Number(objectValue["40"]),
      script: unpackedScriptPtr,
      id: String(objectValue["2"]),
      space: _NodeReference.fromValue(objectValue["5"], _session, _supergraph, _graph, _connection),
      customValues: unpackedCustomValues,
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
  ): KeyPressEvent {
    return KeyPressEvent.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
  }

  toProto(): KeyPressEventProto {
    return KeyPressEvent.__packProto__(this);
  }

  static __packProto__(object: KeyPressEvent): KeyPressEventProto {
    const objectProto: Partial<KeyPressEventProto> = { metatype: 2000303 };
    objectProto.id = String(object.id);
    objectProto.spacePtr = object.spacePtr.toProto();
    if (object.branchPtr != null) {
      objectProto.branchPtr = object.branchPtr.toProto();
    }
    if (object.snapshotPtr != null) {
      objectProto.snapshotPtr = object.snapshotPtr.toProto();
    }
    if (object.precededByPtr != null) {
      objectProto.precededByPtr = object.precededByPtr.toProto();
    }
    if (object.causedByPtr != null) {
      objectProto.causedByPtr = object.causedByPtr.toProto();
    }
    objectProto.createdAt = packProtoTimestamp(object.createdAt);
    objectProto.createdEpoch = object.createdEpoch;
    if (object.createdByPtr != null) {
      objectProto.createdByPtr = object.createdByPtr.toProto();
    }
    if (object.clientPtr != null) {
      objectProto.clientPtr = object.clientPtr.toProto();
    }
    if (object.clientNonce != null) {
      objectProto.clientNonce = String(object.clientNonce);
    }
    objectProto.clientCreatedAt = packProtoTimestamp(object.clientCreatedAt);
    objectProto.clientEpoch = object.clientEpoch;
    if (object.customValues) {
      objectProto.customValues = {} as any;
      for (const [key, value] of Object.entries(object.customValues)) {
        objectProto.customValues![String(key)] = value.toProto();
      }
    }
    objectProto.status = Number(object.status) as EventStatusProto;
    if (object.scriptPtr != null) {
      objectProto.scriptPtr = object.scriptPtr.toProto();
    }
    objectProto.isExtensible = object.isExtensible;
    if (object.nodePtr != null) {
      objectProto.nodePtr = object.nodePtr.toProto();
    }
    objectProto.key = object.key;
    objectProto.code = object.code;
    objectProto.isRepeat = object.isRepeat;
    objectProto.isRedacted = object.isRedacted;
    objectProto.shiftKey = object.shiftKey;
    objectProto.altKey = object.altKey;
    objectProto.ctrlKey = object.ctrlKey;
    objectProto.metaKey = object.metaKey;
    return objectProto as KeyPressEventProto;
  }

  static __unpackProto__(
    objectProto: KeyPressEventProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): KeyPressEvent {
    const _Value = STRUCT_CLASS_BY_TYPE[StructType.VALUE] as typeof Value;
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    const unpackedCustomValues = {} as any;
    if (objectProto.customValues) {
      for (const [key, value] of Object.entries(objectProto.customValues)) {
        unpackedCustomValues.set(
          String(key),
          _Value.fromProto((value as any)!, _session, _supergraph, _graph, _connection),
        );
      }
    }
    return new KeyPressEvent({
      key: objectProto.key,
      code: objectProto.code,
      isRepeat: objectProto.isRepeat,
      isRedacted: objectProto.isRedacted,
      shiftKey: objectProto.shiftKey,
      altKey: objectProto.altKey,
      ctrlKey: objectProto.ctrlKey,
      metaKey: objectProto.metaKey,
      node:
        objectProto.nodePtr != undefined
          ? _NodeReference.fromProto(
              objectProto.nodePtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      isExtensible: objectProto.isExtensible,
      branch:
        objectProto.branchPtr != undefined
          ? _NodeReference.fromProto(
              objectProto.branchPtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
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
      precededBy:
        objectProto.precededByPtr != undefined
          ? _NodeReference.fromProto(
              objectProto.precededByPtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      causedBy:
        objectProto.causedByPtr != undefined
          ? _NodeReference.fromProto(
              objectProto.causedByPtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      createdAt: unpackProtoTimestamp(objectProto.createdAt!),
      createdEpoch: Number(objectProto.createdEpoch),
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
      clientCreatedAt: unpackProtoTimestamp(objectProto.clientCreatedAt!),
      clientEpoch: Number(objectProto.clientEpoch),
      status: Number(objectProto.status) as EventStatus,
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
      id: String(objectProto.id),
      space: _NodeReference.fromProto(
        objectProto.spacePtr!,
        _session,
        _supergraph,
        _graph,
        _connection,
      ),
      customValues: unpackedCustomValues,
      _session,
      _graph,
      _connection,
    });
  }

  static fromProto(
    objectProto: KeyPressEventProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): KeyPressEvent {
    return KeyPressEvent.__unpackProto__(objectProto, _session, _supergraph, _graph, _connection);
  }

  static fromProtoString(packedProtoString: string): KeyPressEvent {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = KeyPressEventProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerNodeClass(NodeType.KEY_PRESS_EVENT, KeyPressEvent);
/* ==== DESTACK_GENERATED_END:NODE:2000303 ==== */
