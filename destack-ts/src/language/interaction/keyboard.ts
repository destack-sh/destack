import { packProtoTimestamp, unpackProtoTimestamp } from "@destack/grpc";
import type {
  Graph,
  IsSubject,
  NodeReference,
  QueryConnection,
  Session,
  Snapshot,
  Supergraph,
} from "@destack/language/core";
import {
  ACTIVE_SPACE,
  Entity,
  EventStatus,
  Node,
  NodeType,
  StructType,
} from "@destack/language/core";
import { InputEvent } from "@destack/language/interaction/input";
import { STRUCT_CLASS_BY_TYPE, registerNodeClass } from "@destack/language/registry";
import type { Client, Space } from "@destack/language/universe";
import type { View } from "@destack/language/view";
import {
  EventStatusProto,
  KeyDownEventProto,
  KeyPressEventProto,
  KeyUpEventProto,
} from "@destack/proto";
import { base64Decode } from "@destack/utils";
import { hashBool, hashString } from "@destack/utils/hash";
import { Temporal } from "temporal-polyfill";

/* ==== DESTACK_GENERATED_START:NODE:560300 ==== */
/**
 * A KeyboardEvent is an InputEvent that corresponds to some direct user input with a keyboard.
 */
export abstract class KeyboardEvent extends InputEvent {
  static metatype: NodeType = NodeType.KEYBOARD_EVENT;

  /**
   * Event.parent
   */
  abstract get parent(): Space | null;
  declare readonly parentPtr: NodeReference | null;

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
  abstract get createdBy(): (Entity & IsSubject) | null;
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
   * Whether the KeyboardEvent was masked for some reason (e.g., security, privacy).
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
registerNodeClass(NodeType.KEYBOARD_EVENT, KeyboardEvent);
/* ==== DESTACK_GENERATED_END:NODE:560300 ==== */

/* ==== DESTACK_GENERATED_START:NODE:560301 ==== */
/**
 * A KeyDownEvent is a KeyboardEvent when a key is pressed down.
 */
export class KeyDownEvent extends KeyboardEvent {
  static metatype: NodeType = NodeType.KEY_DOWN_EVENT;

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
  readonly spacePtr: NodeReference;

  /**
   * The Snapshot this Event originated from.
   */
  get snapshot(): Snapshot | null {
    const nodePtr: NodeReference | null = this.snapshotPtr;
    if (nodePtr !== null) {
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
  get createdBy(): (Entity & IsSubject) | null {
    const nodePtr: NodeReference | null = this.createdByPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as (Entity & IsSubject) | null;
    }
    return null;
  }
  readonly createdByPtr: NodeReference | null;

  /**
   * Event.client
   */
  get client(): Client | null {
    const nodePtr: NodeReference | null = this.clientPtr;
    if (nodePtr !== null) {
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
   * InputEvent.node
   */
  get node(): View | null {
    const nodePtr: NodeReference | null = this.nodePtr;
    if (nodePtr !== null) {
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
   * Whether the KeyboardEvent was masked for some reason (e.g., security, privacy).
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
    parent?: Space | NodeReference | null;
    space?: Space | NodeReference;
    snapshot?: Snapshot | NodeReference | null;
    createdAt?: Temporal.ZonedDateTime;
    createdBy?: (Entity & IsSubject) | NodeReference | null;
    client?: Client | NodeReference | null;
    clientNonce?: string | null;
    status?: EventStatus;
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
        throw new Error(`KeyDownEvent has no Session`);
      }
      _space = ACTIVE_SPACE.get();
      if (_space === null) {
        throw new Error(`KeyDownEvent has no Space`);
      }
      _space = _space.toRef();
    }
    if (_space === null) {
      throw new Error(`KeyDownEvent.space is required`);
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
      throw new Error(`KeyDownEvent.status is required`);
    }
    this.status = _status;
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
      this.createdAt = now;
      this.createdByPtr = null;
    } else {
      if (options.createdAt == null) {
        throw new Error(`KeyDownEvent.createdAt is required for existing Events`);
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
    h = (h * 31 + hashString(this.key)) & 0xffffffff;
    h = (h * 31 + hashString(this.code)) & 0xffffffff;
    h = (h * 31 + hashBool(this.isRepeat)) & 0xffffffff;
    h = (h * 31 + hashBool(this.isRedacted)) & 0xffffffff;
    h = (h * 31 + hashBool(this.shiftKey)) & 0xffffffff;
    h = (h * 31 + hashBool(this.altKey)) & 0xffffffff;
    h = (h * 31 + hashBool(this.ctrlKey)) & 0xffffffff;
    h = (h * 31 + hashBool(this.metaKey)) & 0xffffffff;
    if (this.nodePtr !== null) {
      h = (h * 31 + hashString(this.nodePtr.id)) & 0xffffffff;
    }
    if (this.parentPtr !== null) {
      h = (h * 31 + hashString(this.parentPtr.id)) & 0xffffffff;
    }
    if (this.snapshotPtr !== null) {
      h = (h * 31 + hashString(this.snapshotPtr.id)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.createdAt.toString({ timeZoneName: "never" }))) & 0xffffffff;
    if (this.createdByPtr !== null) {
      h = (h * 31 + hashString(this.createdByPtr.id)) & 0xffffffff;
    }
    if (this.clientPtr !== null) {
      h = (h * 31 + hashString(this.clientPtr.id)) & 0xffffffff;
    }
    if (this.clientNonce !== null) {
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
      type: NodeType.KEY_DOWN_EVENT,
      id: this.id,
      spaceId: this.spacePtr?.id ?? null,
      snapshotId: this.snapshotPtr?.id ?? null,
      _session: this._session,
      _supergraph: this._supergraph,
    });
  }

  get _pathKey(): string {
    return `KeyDownEvent[id=${this.id}]`;
  }

  get path(): string {
    const pathParts: string[] = [];
    let node: Node | null = this;
    let lastNode: Node | null = this;
    while (node !== null) {
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
    propertyReprs.push(`status=${EventStatus[this.status]}`);
    return `<KeyDownEvent "${this.path}" ${propertyReprs.join(" ")}>`;
  }

  toValue(): { readonly [key: string]: any } {
    return KeyDownEvent.__packValue__(this);
  }

  static __packValue__(object: KeyDownEvent): { readonly [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 560301;
    objectValue["2"] = String(object.id);
    if (object.parentPtr != null) {
      objectValue["3"] = object.parentPtr.toValue();
    }
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
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    const nodePtrValue = objectValue["101"];
    const unpackedNodePtr =
      nodePtrValue != undefined
        ? _NodeReference.fromValue(nodePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const parentPtrValue = objectValue["3"];
    const unpackedParentPtr =
      parentPtrValue != undefined
        ? _NodeReference.fromValue(parentPtrValue, _session, _supergraph, _graph, _connection)
        : null;
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
      parent: unpackedParentPtr,
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
  ): KeyDownEvent {
    return KeyDownEvent.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
  }

  toProto(): KeyDownEventProto {
    return KeyDownEvent.__packProto__(this);
  }

  static __packProto__(object: KeyDownEvent): KeyDownEventProto {
    const objectProto: Partial<KeyDownEventProto> = { metatype: 560301 };
    objectProto.id = String(object.id);
    if (object.parentPtr != null) {
      objectProto.parentPtr = object.parentPtr.toProto();
    }
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
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
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
/* ==== DESTACK_GENERATED_END:NODE:560301 ==== */

/* ==== DESTACK_GENERATED_START:NODE:560302 ==== */
/**
 * A KeyUpEvent is a KeyboardEvent when a key is released.
 */
export class KeyUpEvent extends KeyboardEvent {
  static metatype: NodeType = NodeType.KEY_UP_EVENT;

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
  readonly spacePtr: NodeReference;

  /**
   * The Snapshot this Event originated from.
   */
  get snapshot(): Snapshot | null {
    const nodePtr: NodeReference | null = this.snapshotPtr;
    if (nodePtr !== null) {
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
  get createdBy(): (Entity & IsSubject) | null {
    const nodePtr: NodeReference | null = this.createdByPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as (Entity & IsSubject) | null;
    }
    return null;
  }
  readonly createdByPtr: NodeReference | null;

  /**
   * Event.client
   */
  get client(): Client | null {
    const nodePtr: NodeReference | null = this.clientPtr;
    if (nodePtr !== null) {
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
   * InputEvent.node
   */
  get node(): View | null {
    const nodePtr: NodeReference | null = this.nodePtr;
    if (nodePtr !== null) {
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
   * Whether the KeyboardEvent was masked for some reason (e.g., security, privacy).
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
    parent?: Space | NodeReference | null;
    space?: Space | NodeReference;
    snapshot?: Snapshot | NodeReference | null;
    createdAt?: Temporal.ZonedDateTime;
    createdBy?: (Entity & IsSubject) | NodeReference | null;
    client?: Client | NodeReference | null;
    clientNonce?: string | null;
    status?: EventStatus;
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
        throw new Error(`KeyUpEvent has no Session`);
      }
      _space = ACTIVE_SPACE.get();
      if (_space === null) {
        throw new Error(`KeyUpEvent has no Space`);
      }
      _space = _space.toRef();
    }
    if (_space === null) {
      throw new Error(`KeyUpEvent.space is required`);
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
      throw new Error(`KeyUpEvent.status is required`);
    }
    this.status = _status;
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
      this.createdAt = now;
      this.createdByPtr = null;
    } else {
      if (options.createdAt == null) {
        throw new Error(`KeyUpEvent.createdAt is required for existing Events`);
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
    h = (h * 31 + hashString(this.key)) & 0xffffffff;
    h = (h * 31 + hashString(this.code)) & 0xffffffff;
    h = (h * 31 + hashBool(this.isRepeat)) & 0xffffffff;
    h = (h * 31 + hashBool(this.isRedacted)) & 0xffffffff;
    h = (h * 31 + hashBool(this.shiftKey)) & 0xffffffff;
    h = (h * 31 + hashBool(this.altKey)) & 0xffffffff;
    h = (h * 31 + hashBool(this.ctrlKey)) & 0xffffffff;
    h = (h * 31 + hashBool(this.metaKey)) & 0xffffffff;
    if (this.nodePtr !== null) {
      h = (h * 31 + hashString(this.nodePtr.id)) & 0xffffffff;
    }
    if (this.parentPtr !== null) {
      h = (h * 31 + hashString(this.parentPtr.id)) & 0xffffffff;
    }
    if (this.snapshotPtr !== null) {
      h = (h * 31 + hashString(this.snapshotPtr.id)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.createdAt.toString({ timeZoneName: "never" }))) & 0xffffffff;
    if (this.createdByPtr !== null) {
      h = (h * 31 + hashString(this.createdByPtr.id)) & 0xffffffff;
    }
    if (this.clientPtr !== null) {
      h = (h * 31 + hashString(this.clientPtr.id)) & 0xffffffff;
    }
    if (this.clientNonce !== null) {
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
      type: NodeType.KEY_UP_EVENT,
      id: this.id,
      spaceId: this.spacePtr?.id ?? null,
      snapshotId: this.snapshotPtr?.id ?? null,
      _session: this._session,
      _supergraph: this._supergraph,
    });
  }

  get _pathKey(): string {
    return `KeyUpEvent[id=${this.id}]`;
  }

  get path(): string {
    const pathParts: string[] = [];
    let node: Node | null = this;
    let lastNode: Node | null = this;
    while (node !== null) {
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
    propertyReprs.push(`status=${EventStatus[this.status]}`);
    return `<KeyUpEvent "${this.path}" ${propertyReprs.join(" ")}>`;
  }

  toValue(): { readonly [key: string]: any } {
    return KeyUpEvent.__packValue__(this);
  }

  static __packValue__(object: KeyUpEvent): { readonly [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 560302;
    objectValue["2"] = String(object.id);
    if (object.parentPtr != null) {
      objectValue["3"] = object.parentPtr.toValue();
    }
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
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    const nodePtrValue = objectValue["101"];
    const unpackedNodePtr =
      nodePtrValue != undefined
        ? _NodeReference.fromValue(nodePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const parentPtrValue = objectValue["3"];
    const unpackedParentPtr =
      parentPtrValue != undefined
        ? _NodeReference.fromValue(parentPtrValue, _session, _supergraph, _graph, _connection)
        : null;
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
      parent: unpackedParentPtr,
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
  ): KeyUpEvent {
    return KeyUpEvent.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
  }

  toProto(): KeyUpEventProto {
    return KeyUpEvent.__packProto__(this);
  }

  static __packProto__(object: KeyUpEvent): KeyUpEventProto {
    const objectProto: Partial<KeyUpEventProto> = { metatype: 560302 };
    objectProto.id = String(object.id);
    if (object.parentPtr != null) {
      objectProto.parentPtr = object.parentPtr.toProto();
    }
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
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
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
/* ==== DESTACK_GENERATED_END:NODE:560302 ==== */

/* ==== DESTACK_GENERATED_START:NODE:560303 ==== */
/**
 * A KeyPressEvent is a KeyboardEvent when a key is pressed.
 */
export class KeyPressEvent extends KeyboardEvent {
  static metatype: NodeType = NodeType.KEY_PRESS_EVENT;

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
  readonly spacePtr: NodeReference;

  /**
   * The Snapshot this Event originated from.
   */
  get snapshot(): Snapshot | null {
    const nodePtr: NodeReference | null = this.snapshotPtr;
    if (nodePtr !== null) {
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
  get createdBy(): (Entity & IsSubject) | null {
    const nodePtr: NodeReference | null = this.createdByPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as (Entity & IsSubject) | null;
    }
    return null;
  }
  readonly createdByPtr: NodeReference | null;

  /**
   * Event.client
   */
  get client(): Client | null {
    const nodePtr: NodeReference | null = this.clientPtr;
    if (nodePtr !== null) {
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
   * InputEvent.node
   */
  get node(): View | null {
    const nodePtr: NodeReference | null = this.nodePtr;
    if (nodePtr !== null) {
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
   * Whether the KeyboardEvent was masked for some reason (e.g., security, privacy).
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
    parent?: Space | NodeReference | null;
    space?: Space | NodeReference;
    snapshot?: Snapshot | NodeReference | null;
    createdAt?: Temporal.ZonedDateTime;
    createdBy?: (Entity & IsSubject) | NodeReference | null;
    client?: Client | NodeReference | null;
    clientNonce?: string | null;
    status?: EventStatus;
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
        throw new Error(`KeyPressEvent has no Session`);
      }
      _space = ACTIVE_SPACE.get();
      if (_space === null) {
        throw new Error(`KeyPressEvent has no Space`);
      }
      _space = _space.toRef();
    }
    if (_space === null) {
      throw new Error(`KeyPressEvent.space is required`);
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
      throw new Error(`KeyPressEvent.status is required`);
    }
    this.status = _status;
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
      this.createdAt = now;
      this.createdByPtr = null;
    } else {
      if (options.createdAt == null) {
        throw new Error(`KeyPressEvent.createdAt is required for existing Events`);
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
    h = (h * 31 + hashString(this.key)) & 0xffffffff;
    h = (h * 31 + hashString(this.code)) & 0xffffffff;
    h = (h * 31 + hashBool(this.isRepeat)) & 0xffffffff;
    h = (h * 31 + hashBool(this.isRedacted)) & 0xffffffff;
    h = (h * 31 + hashBool(this.shiftKey)) & 0xffffffff;
    h = (h * 31 + hashBool(this.altKey)) & 0xffffffff;
    h = (h * 31 + hashBool(this.ctrlKey)) & 0xffffffff;
    h = (h * 31 + hashBool(this.metaKey)) & 0xffffffff;
    if (this.nodePtr !== null) {
      h = (h * 31 + hashString(this.nodePtr.id)) & 0xffffffff;
    }
    if (this.parentPtr !== null) {
      h = (h * 31 + hashString(this.parentPtr.id)) & 0xffffffff;
    }
    if (this.snapshotPtr !== null) {
      h = (h * 31 + hashString(this.snapshotPtr.id)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.createdAt.toString({ timeZoneName: "never" }))) & 0xffffffff;
    if (this.createdByPtr !== null) {
      h = (h * 31 + hashString(this.createdByPtr.id)) & 0xffffffff;
    }
    if (this.clientPtr !== null) {
      h = (h * 31 + hashString(this.clientPtr.id)) & 0xffffffff;
    }
    if (this.clientNonce !== null) {
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
      type: NodeType.KEY_PRESS_EVENT,
      id: this.id,
      spaceId: this.spacePtr?.id ?? null,
      snapshotId: this.snapshotPtr?.id ?? null,
      _session: this._session,
      _supergraph: this._supergraph,
    });
  }

  get _pathKey(): string {
    return `KeyPressEvent[id=${this.id}]`;
  }

  get path(): string {
    const pathParts: string[] = [];
    let node: Node | null = this;
    let lastNode: Node | null = this;
    while (node !== null) {
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
    propertyReprs.push(`status=${EventStatus[this.status]}`);
    return `<KeyPressEvent "${this.path}" ${propertyReprs.join(" ")}>`;
  }

  toValue(): { readonly [key: string]: any } {
    return KeyPressEvent.__packValue__(this);
  }

  static __packValue__(object: KeyPressEvent): { readonly [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 560303;
    objectValue["2"] = String(object.id);
    if (object.parentPtr != null) {
      objectValue["3"] = object.parentPtr.toValue();
    }
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
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    const nodePtrValue = objectValue["101"];
    const unpackedNodePtr =
      nodePtrValue != undefined
        ? _NodeReference.fromValue(nodePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const parentPtrValue = objectValue["3"];
    const unpackedParentPtr =
      parentPtrValue != undefined
        ? _NodeReference.fromValue(parentPtrValue, _session, _supergraph, _graph, _connection)
        : null;
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
      parent: unpackedParentPtr,
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
  ): KeyPressEvent {
    return KeyPressEvent.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
  }

  toProto(): KeyPressEventProto {
    return KeyPressEvent.__packProto__(this);
  }

  static __packProto__(object: KeyPressEvent): KeyPressEventProto {
    const objectProto: Partial<KeyPressEventProto> = { metatype: 560303 };
    objectProto.id = String(object.id);
    if (object.parentPtr != null) {
      objectProto.parentPtr = object.parentPtr.toProto();
    }
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
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
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
/* ==== DESTACK_GENERATED_END:NODE:560303 ==== */
