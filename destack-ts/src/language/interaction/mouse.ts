import { packProtoTimestamp, unpackProtoTimestamp } from "@destack/grpc";
import type {
  Graph,
  IsSubject,
  NodeReference,
  QueryConnection,
  Session,
  Snapshot,
  Supergraph,
  Vector2f,
} from "@destack/language/core";
import { EnumType, EventStatus, Node, NodeType, StructType } from "@destack/language/core";
import { PointerEvent } from "@destack/language/interaction/pointer";
import {
  STRUCT_CLASS_BY_TYPE,
  registerEnumClass,
  registerNodeClass,
} from "@destack/language/registry";
import type { Client, Space } from "@destack/language/universe";
import {
  DoubleClickEventProto,
  EventStatusProto,
  LeftClickEventProto,
  MiddleClickEventProto,
  MouseButtonProto,
  RightClickEventProto,
  WheelEventProto,
} from "@destack/proto";
import { base64Decode } from "@destack/utils";
import { hashBool, hashFloat, hashString } from "@destack/utils/hash";
import { Temporal } from "temporal-polyfill";

/* ==== DESTACK_GENERATED_START:ENUM:560010 ==== */
/**
 * MouseButton
 */
export enum MouseButton {
  LEFT = 1,
  RIGHT = 2,
  MIDDLE = 3,

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerEnumClass(EnumType.MOUSE_BUTTON, MouseButton);
/* ==== DESTACK_GENERATED_END:ENUM:560010 ==== */

/* ==== DESTACK_GENERATED_START:NODE:560200 ==== */
/**
 * A MouseEvent is a PointerEvent that corresponds to some direct user input with a mouse.
 */
export abstract class MouseEvent extends PointerEvent {
  static metatype: NodeType = NodeType.MOUSE_EVENT;

  abstract get parent(): Space | null;
  declare readonly parentPtr: NodeReference | null;

  abstract get space(): Space | null;
  declare readonly spacePtr: NodeReference | null;

  abstract get snapshot(): Snapshot | null;
  declare readonly snapshotPtr: NodeReference | null;

  /**
   * Event.createdAt
   */
  declare readonly createdAt: Temporal.ZonedDateTime;

  abstract get createdBy(): (Node & IsSubject) | null;
  declare readonly createdByPtr: NodeReference | null;

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

  abstract get node(): Node | null;
  declare readonly nodePtr: NodeReference | null;

  /**
   * PointerEvent.position
   */
  declare readonly position: Vector2f;

  /**
   * PointerEvent.pressure
   */
  declare readonly pressure: number;

  /**
   * PointerEvent.shiftKey
   */
  declare readonly shiftKey: boolean;

  /**
   * PointerEvent.altKey
   */
  declare readonly altKey: boolean;

  /**
   * PointerEvent.ctrlKey
   */
  declare readonly ctrlKey: boolean;

  /**
   * PointerEvent.metaKey
   */
  declare readonly metaKey: boolean;

  /**
   * PointerEvent.accelKey
   */
  declare readonly accelKey: boolean;

  /**
   * MouseEvent.button
   */
  declare readonly button: MouseButton;

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerNodeClass(NodeType.MOUSE_EVENT, MouseEvent);
/* ==== DESTACK_GENERATED_END:NODE:560200 ==== */

/* ==== DESTACK_GENERATED_START:NODE:560201 ==== */
/**
 * A ClickEvent is an InputEvent that corresponds to some direct user input with a click (left, right, middle).
 */
export abstract class ClickEvent extends MouseEvent {
  static metatype: NodeType = NodeType.CLICK_EVENT;

  abstract get parent(): Space | null;
  declare readonly parentPtr: NodeReference | null;

  abstract get space(): Space | null;
  declare readonly spacePtr: NodeReference | null;

  abstract get snapshot(): Snapshot | null;
  declare readonly snapshotPtr: NodeReference | null;

  /**
   * Event.createdAt
   */
  declare readonly createdAt: Temporal.ZonedDateTime;

  abstract get createdBy(): (Node & IsSubject) | null;
  declare readonly createdByPtr: NodeReference | null;

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

  abstract get node(): Node | null;
  declare readonly nodePtr: NodeReference | null;

  /**
   * PointerEvent.position
   */
  declare readonly position: Vector2f;

  /**
   * PointerEvent.pressure
   */
  declare readonly pressure: number;

  /**
   * PointerEvent.shiftKey
   */
  declare readonly shiftKey: boolean;

  /**
   * PointerEvent.altKey
   */
  declare readonly altKey: boolean;

  /**
   * PointerEvent.ctrlKey
   */
  declare readonly ctrlKey: boolean;

  /**
   * PointerEvent.metaKey
   */
  declare readonly metaKey: boolean;

  /**
   * PointerEvent.accelKey
   */
  declare readonly accelKey: boolean;

  /**
   * MouseEvent.button
   */
  declare readonly button: MouseButton;

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerNodeClass(NodeType.CLICK_EVENT, ClickEvent);
/* ==== DESTACK_GENERATED_END:NODE:560201 ==== */

/* ==== DESTACK_GENERATED_START:NODE:560202 ==== */
/**
 * A LeftClickEvent is a ClickEvent when a pointer is clicked with the left button.
 */
export class LeftClickEvent extends ClickEvent {
  static metatype: NodeType = NodeType.LEFT_CLICK_EVENT;

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
  get createdBy(): (Node & IsSubject) | null {
    const nodePtr: NodeReference | null = this.createdByPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as (Node & IsSubject) | null;
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
   * The Node this Event is about.
   */
  get node(): Node | null {
    const nodePtr: NodeReference | null = this.nodePtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Node | null;
    }
    return null;
  }
  readonly nodePtr: NodeReference | null;

  /**
   * PointerEvent.position
   */
  readonly position: Vector2f;

  /**
   * PointerEvent.pressure
   */
  readonly pressure: number;

  /**
   * PointerEvent.shiftKey
   */
  readonly shiftKey: boolean;

  /**
   * PointerEvent.altKey
   */
  readonly altKey: boolean;

  /**
   * PointerEvent.ctrlKey
   */
  readonly ctrlKey: boolean;

  /**
   * PointerEvent.metaKey
   */
  readonly metaKey: boolean;

  /**
   * PointerEvent.accelKey
   */
  readonly accelKey: boolean;

  /**
   * MouseEvent.button
   */
  readonly button: MouseButton;

  constructor(options: {
    id?: string;
    parent?: Space | NodeReference | null;
    space?: Space | NodeReference | null;
    snapshot?: Snapshot | NodeReference | null;
    createdAt?: Temporal.ZonedDateTime;
    createdBy?: (Node & IsSubject) | NodeReference | null;
    client?: Client | NodeReference | null;
    clientNonce?: string | null;
    status?: EventStatus;
    node?: Node | NodeReference | null;
    position: Vector2f;
    pressure: number;
    shiftKey: boolean;
    altKey: boolean;
    ctrlKey: boolean;
    metaKey: boolean;
    accelKey: boolean;
    button: MouseButton;
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
      throw new Error(`LeftClickEvent.status is required`);
    }
    this.status = _status;
    let _node = options.node ?? null;
    if (_node != null && _node.metatype != StructType.NODE_REFERENCE) {
      _node = (_node as Node).toRef();
    }
    this.nodePtr = _node;
    let _position = options.position;
    if (_position === null) {
      throw new Error(`LeftClickEvent.position is required`);
    }
    this.position = _position;
    let _pressure = options.pressure;
    if (_pressure === null) {
      throw new Error(`LeftClickEvent.pressure is required`);
    }
    this.pressure = _pressure;
    let _shiftKey = options.shiftKey;
    if (_shiftKey === null) {
      throw new Error(`LeftClickEvent.shiftKey is required`);
    }
    this.shiftKey = _shiftKey;
    let _altKey = options.altKey;
    if (_altKey === null) {
      throw new Error(`LeftClickEvent.altKey is required`);
    }
    this.altKey = _altKey;
    let _ctrlKey = options.ctrlKey;
    if (_ctrlKey === null) {
      throw new Error(`LeftClickEvent.ctrlKey is required`);
    }
    this.ctrlKey = _ctrlKey;
    let _metaKey = options.metaKey;
    if (_metaKey === null) {
      throw new Error(`LeftClickEvent.metaKey is required`);
    }
    this.metaKey = _metaKey;
    let _accelKey = options.accelKey;
    if (_accelKey === null) {
      throw new Error(`LeftClickEvent.accelKey is required`);
    }
    this.accelKey = _accelKey;
    let _button = options.button;
    if (_button === null) {
      throw new Error(`LeftClickEvent.button is required`);
    }
    this.button = _button;

    // identity
    if (options.id == null) {
      const now = Temporal.Now.zonedDateTimeISO("UTC");
      this.createdAt = now;
      this.createdByPtr = null;
    } else {
      if (options.createdAt == null) {
        throw new Error(`LeftClickEvent.createdAt is required for existing Events`);
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
    if (!(this.button === other.button)) {
      return false;
    }
    if (!this.position.equals(other.position)) {
      return false;
    }
    if (!(this.pressure === other.pressure || Math.abs(this.pressure - other.pressure) < 1e-10)) {
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
    if (!(this.accelKey === other.accelKey)) {
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
    if (!(this.nodePtr?.id === other.nodePtr?.id)) {
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
    h = (h * 31 + this.button) & 0xffffffff;
    h = (h * 31 + this.position.hash()) & 0xffffffff;
    h = (h * 31 + hashFloat(this.pressure)) & 0xffffffff;
    h = (h * 31 + hashBool(this.shiftKey)) & 0xffffffff;
    h = (h * 31 + hashBool(this.altKey)) & 0xffffffff;
    h = (h * 31 + hashBool(this.ctrlKey)) & 0xffffffff;
    h = (h * 31 + hashBool(this.metaKey)) & 0xffffffff;
    h = (h * 31 + hashBool(this.accelKey)) & 0xffffffff;
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
    if (this.nodePtr !== null) {
      h = (h * 31 + hashString(this.nodePtr.id)) & 0xffffffff;
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
      type: NodeType.LEFT_CLICK_EVENT,
      id: this.id,
      spaceId: this.spacePtr?.id ?? null,
      snapshotId: this.snapshotPtr?.id ?? null,
      _session: this._session,
      _supergraph: this._supergraph,
    });
  }

  get _pathKey(): string {
    return `LeftClickEvent[id=${this.id}]`;
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
    propertyReprs.push(`status=${EventStatus[this.status]}`);
    return `<LeftClickEvent '${this.path}' ${propertyReprs.join(" ")}>`;
  }

  toValue(): { [key: string]: any } {
    return LeftClickEvent.__packValue__(this);
  }

  static __packValue__(object: LeftClickEvent): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 560202;
    objectValue["2"] = String(object.id);
    if (object.parentPtr != null) {
      objectValue["3"] = object.parentPtr.toValue();
    }
    if (object.spacePtr != null) {
      objectValue["5"] = object.spacePtr.toValue();
    }
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
    objectValue["110"] = object.position.toValue();
    objectValue["111"] = object.pressure;
    objectValue["120"] = object.shiftKey;
    objectValue["121"] = object.altKey;
    objectValue["122"] = object.ctrlKey;
    objectValue["123"] = object.metaKey;
    objectValue["124"] = object.accelKey;
    objectValue["130"] = object.button;
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): LeftClickEvent {
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    const _Vector2f = STRUCT_CLASS_BY_TYPE[StructType.VECTOR2F] as typeof Vector2f;
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
    const nodePtrValue = objectValue["101"];
    const unpackedNodePtr =
      nodePtrValue != undefined
        ? _NodeReference.fromValue(nodePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const spacePtrValue = objectValue["5"];
    const unpackedSpacePtr =
      spacePtrValue != undefined
        ? _NodeReference.fromValue(spacePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    return new LeftClickEvent({
      button: Number(objectValue["130"]),
      position: _Vector2f.fromValue(objectValue["110"], _session, _supergraph, _graph, _connection),
      pressure: objectValue["111"],
      shiftKey: objectValue["120"],
      altKey: objectValue["121"],
      ctrlKey: objectValue["122"],
      metaKey: objectValue["123"],
      accelKey: objectValue["124"],
      parent: unpackedParentPtr,
      snapshot: unpackedSnapshotPtr,
      createdAt: Temporal.Instant.from(objectValue["20"]).toZonedDateTimeISO("UTC"),
      createdBy: unpackedCreatedByPtr,
      client: unpackedClientPtr,
      clientNonce: unpackedClientNonce,
      status: Number(objectValue["30"]),
      node: unpackedNodePtr,
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
  ): LeftClickEvent {
    return LeftClickEvent.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
  }

  toProto(): LeftClickEventProto {
    return LeftClickEvent.__packProto__(this);
  }

  static __packProto__(object: LeftClickEvent): LeftClickEventProto {
    const objectProto: Partial<LeftClickEventProto> = { metatype: 560202 };
    objectProto.id = String(object.id);
    if (object.parentPtr != null) {
      objectProto.parentPtr = object.parentPtr.toProto();
    }
    if (object.spacePtr != null) {
      objectProto.spacePtr = object.spacePtr.toProto();
    }
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
    objectProto.position = object.position.toProto();
    objectProto.pressure = object.pressure;
    objectProto.shiftKey = object.shiftKey;
    objectProto.altKey = object.altKey;
    objectProto.ctrlKey = object.ctrlKey;
    objectProto.metaKey = object.metaKey;
    objectProto.accelKey = object.accelKey;
    objectProto.button = Number(object.button) as MouseButtonProto;
    return objectProto as LeftClickEventProto;
  }

  static __unpackProto__(
    objectProto: LeftClickEventProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): LeftClickEvent {
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    const _Vector2f = STRUCT_CLASS_BY_TYPE[StructType.VECTOR2F] as typeof Vector2f;
    return new LeftClickEvent({
      button: Number(objectProto.button) as MouseButton,
      position: _Vector2f.fromProto(
        objectProto.position!,
        _session,
        _supergraph,
        _graph,
        _connection,
      ),
      pressure: objectProto.pressure,
      shiftKey: objectProto.shiftKey,
      altKey: objectProto.altKey,
      ctrlKey: objectProto.ctrlKey,
      metaKey: objectProto.metaKey,
      accelKey: objectProto.accelKey,
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
    objectProto: LeftClickEventProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): LeftClickEvent {
    return LeftClickEvent.__unpackProto__(objectProto, _session, _supergraph, _graph, _connection);
  }

  static fromProtoString(packedProtoString: string): LeftClickEvent {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = LeftClickEventProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerNodeClass(NodeType.LEFT_CLICK_EVENT, LeftClickEvent);
/* ==== DESTACK_GENERATED_END:NODE:560202 ==== */

/* ==== DESTACK_GENERATED_START:NODE:560203 ==== */
/**
 * A RightClickEvent is a ClickEvent when a pointer is clicked with the right button.
 */
export class RightClickEvent extends ClickEvent {
  static metatype: NodeType = NodeType.RIGHT_CLICK_EVENT;

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
  get createdBy(): (Node & IsSubject) | null {
    const nodePtr: NodeReference | null = this.createdByPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as (Node & IsSubject) | null;
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
   * The Node this Event is about.
   */
  get node(): Node | null {
    const nodePtr: NodeReference | null = this.nodePtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Node | null;
    }
    return null;
  }
  readonly nodePtr: NodeReference | null;

  /**
   * PointerEvent.position
   */
  readonly position: Vector2f;

  /**
   * PointerEvent.pressure
   */
  readonly pressure: number;

  /**
   * PointerEvent.shiftKey
   */
  readonly shiftKey: boolean;

  /**
   * PointerEvent.altKey
   */
  readonly altKey: boolean;

  /**
   * PointerEvent.ctrlKey
   */
  readonly ctrlKey: boolean;

  /**
   * PointerEvent.metaKey
   */
  readonly metaKey: boolean;

  /**
   * PointerEvent.accelKey
   */
  readonly accelKey: boolean;

  /**
   * MouseEvent.button
   */
  readonly button: MouseButton;

  constructor(options: {
    id?: string;
    parent?: Space | NodeReference | null;
    space?: Space | NodeReference | null;
    snapshot?: Snapshot | NodeReference | null;
    createdAt?: Temporal.ZonedDateTime;
    createdBy?: (Node & IsSubject) | NodeReference | null;
    client?: Client | NodeReference | null;
    clientNonce?: string | null;
    status?: EventStatus;
    node?: Node | NodeReference | null;
    position: Vector2f;
    pressure: number;
    shiftKey: boolean;
    altKey: boolean;
    ctrlKey: boolean;
    metaKey: boolean;
    accelKey: boolean;
    button: MouseButton;
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
      throw new Error(`RightClickEvent.status is required`);
    }
    this.status = _status;
    let _node = options.node ?? null;
    if (_node != null && _node.metatype != StructType.NODE_REFERENCE) {
      _node = (_node as Node).toRef();
    }
    this.nodePtr = _node;
    let _position = options.position;
    if (_position === null) {
      throw new Error(`RightClickEvent.position is required`);
    }
    this.position = _position;
    let _pressure = options.pressure;
    if (_pressure === null) {
      throw new Error(`RightClickEvent.pressure is required`);
    }
    this.pressure = _pressure;
    let _shiftKey = options.shiftKey;
    if (_shiftKey === null) {
      throw new Error(`RightClickEvent.shiftKey is required`);
    }
    this.shiftKey = _shiftKey;
    let _altKey = options.altKey;
    if (_altKey === null) {
      throw new Error(`RightClickEvent.altKey is required`);
    }
    this.altKey = _altKey;
    let _ctrlKey = options.ctrlKey;
    if (_ctrlKey === null) {
      throw new Error(`RightClickEvent.ctrlKey is required`);
    }
    this.ctrlKey = _ctrlKey;
    let _metaKey = options.metaKey;
    if (_metaKey === null) {
      throw new Error(`RightClickEvent.metaKey is required`);
    }
    this.metaKey = _metaKey;
    let _accelKey = options.accelKey;
    if (_accelKey === null) {
      throw new Error(`RightClickEvent.accelKey is required`);
    }
    this.accelKey = _accelKey;
    let _button = options.button;
    if (_button === null) {
      throw new Error(`RightClickEvent.button is required`);
    }
    this.button = _button;

    // identity
    if (options.id == null) {
      const now = Temporal.Now.zonedDateTimeISO("UTC");
      this.createdAt = now;
      this.createdByPtr = null;
    } else {
      if (options.createdAt == null) {
        throw new Error(`RightClickEvent.createdAt is required for existing Events`);
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
    if (!(this.button === other.button)) {
      return false;
    }
    if (!this.position.equals(other.position)) {
      return false;
    }
    if (!(this.pressure === other.pressure || Math.abs(this.pressure - other.pressure) < 1e-10)) {
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
    if (!(this.accelKey === other.accelKey)) {
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
    if (!(this.nodePtr?.id === other.nodePtr?.id)) {
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
    h = (h * 31 + this.button) & 0xffffffff;
    h = (h * 31 + this.position.hash()) & 0xffffffff;
    h = (h * 31 + hashFloat(this.pressure)) & 0xffffffff;
    h = (h * 31 + hashBool(this.shiftKey)) & 0xffffffff;
    h = (h * 31 + hashBool(this.altKey)) & 0xffffffff;
    h = (h * 31 + hashBool(this.ctrlKey)) & 0xffffffff;
    h = (h * 31 + hashBool(this.metaKey)) & 0xffffffff;
    h = (h * 31 + hashBool(this.accelKey)) & 0xffffffff;
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
    if (this.nodePtr !== null) {
      h = (h * 31 + hashString(this.nodePtr.id)) & 0xffffffff;
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
      type: NodeType.RIGHT_CLICK_EVENT,
      id: this.id,
      spaceId: this.spacePtr?.id ?? null,
      snapshotId: this.snapshotPtr?.id ?? null,
      _session: this._session,
      _supergraph: this._supergraph,
    });
  }

  get _pathKey(): string {
    return `RightClickEvent[id=${this.id}]`;
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
    propertyReprs.push(`status=${EventStatus[this.status]}`);
    return `<RightClickEvent '${this.path}' ${propertyReprs.join(" ")}>`;
  }

  toValue(): { [key: string]: any } {
    return RightClickEvent.__packValue__(this);
  }

  static __packValue__(object: RightClickEvent): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 560203;
    objectValue["2"] = String(object.id);
    if (object.parentPtr != null) {
      objectValue["3"] = object.parentPtr.toValue();
    }
    if (object.spacePtr != null) {
      objectValue["5"] = object.spacePtr.toValue();
    }
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
    objectValue["110"] = object.position.toValue();
    objectValue["111"] = object.pressure;
    objectValue["120"] = object.shiftKey;
    objectValue["121"] = object.altKey;
    objectValue["122"] = object.ctrlKey;
    objectValue["123"] = object.metaKey;
    objectValue["124"] = object.accelKey;
    objectValue["130"] = object.button;
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): RightClickEvent {
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    const _Vector2f = STRUCT_CLASS_BY_TYPE[StructType.VECTOR2F] as typeof Vector2f;
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
    const nodePtrValue = objectValue["101"];
    const unpackedNodePtr =
      nodePtrValue != undefined
        ? _NodeReference.fromValue(nodePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const spacePtrValue = objectValue["5"];
    const unpackedSpacePtr =
      spacePtrValue != undefined
        ? _NodeReference.fromValue(spacePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    return new RightClickEvent({
      button: Number(objectValue["130"]),
      position: _Vector2f.fromValue(objectValue["110"], _session, _supergraph, _graph, _connection),
      pressure: objectValue["111"],
      shiftKey: objectValue["120"],
      altKey: objectValue["121"],
      ctrlKey: objectValue["122"],
      metaKey: objectValue["123"],
      accelKey: objectValue["124"],
      parent: unpackedParentPtr,
      snapshot: unpackedSnapshotPtr,
      createdAt: Temporal.Instant.from(objectValue["20"]).toZonedDateTimeISO("UTC"),
      createdBy: unpackedCreatedByPtr,
      client: unpackedClientPtr,
      clientNonce: unpackedClientNonce,
      status: Number(objectValue["30"]),
      node: unpackedNodePtr,
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
  ): RightClickEvent {
    return RightClickEvent.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
  }

  toProto(): RightClickEventProto {
    return RightClickEvent.__packProto__(this);
  }

  static __packProto__(object: RightClickEvent): RightClickEventProto {
    const objectProto: Partial<RightClickEventProto> = { metatype: 560203 };
    objectProto.id = String(object.id);
    if (object.parentPtr != null) {
      objectProto.parentPtr = object.parentPtr.toProto();
    }
    if (object.spacePtr != null) {
      objectProto.spacePtr = object.spacePtr.toProto();
    }
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
    objectProto.position = object.position.toProto();
    objectProto.pressure = object.pressure;
    objectProto.shiftKey = object.shiftKey;
    objectProto.altKey = object.altKey;
    objectProto.ctrlKey = object.ctrlKey;
    objectProto.metaKey = object.metaKey;
    objectProto.accelKey = object.accelKey;
    objectProto.button = Number(object.button) as MouseButtonProto;
    return objectProto as RightClickEventProto;
  }

  static __unpackProto__(
    objectProto: RightClickEventProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): RightClickEvent {
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    const _Vector2f = STRUCT_CLASS_BY_TYPE[StructType.VECTOR2F] as typeof Vector2f;
    return new RightClickEvent({
      button: Number(objectProto.button) as MouseButton,
      position: _Vector2f.fromProto(
        objectProto.position!,
        _session,
        _supergraph,
        _graph,
        _connection,
      ),
      pressure: objectProto.pressure,
      shiftKey: objectProto.shiftKey,
      altKey: objectProto.altKey,
      ctrlKey: objectProto.ctrlKey,
      metaKey: objectProto.metaKey,
      accelKey: objectProto.accelKey,
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
    objectProto: RightClickEventProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): RightClickEvent {
    return RightClickEvent.__unpackProto__(objectProto, _session, _supergraph, _graph, _connection);
  }

  static fromProtoString(packedProtoString: string): RightClickEvent {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = RightClickEventProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerNodeClass(NodeType.RIGHT_CLICK_EVENT, RightClickEvent);
/* ==== DESTACK_GENERATED_END:NODE:560203 ==== */

/* ==== DESTACK_GENERATED_START:NODE:560204 ==== */
/**
 * A MiddleClickEvent is a ClickEvent when a pointer is clicked with the middle button.
 */
export class MiddleClickEvent extends ClickEvent {
  static metatype: NodeType = NodeType.MIDDLE_CLICK_EVENT;

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
  get createdBy(): (Node & IsSubject) | null {
    const nodePtr: NodeReference | null = this.createdByPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as (Node & IsSubject) | null;
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
   * The Node this Event is about.
   */
  get node(): Node | null {
    const nodePtr: NodeReference | null = this.nodePtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Node | null;
    }
    return null;
  }
  readonly nodePtr: NodeReference | null;

  /**
   * PointerEvent.position
   */
  readonly position: Vector2f;

  /**
   * PointerEvent.pressure
   */
  readonly pressure: number;

  /**
   * PointerEvent.shiftKey
   */
  readonly shiftKey: boolean;

  /**
   * PointerEvent.altKey
   */
  readonly altKey: boolean;

  /**
   * PointerEvent.ctrlKey
   */
  readonly ctrlKey: boolean;

  /**
   * PointerEvent.metaKey
   */
  readonly metaKey: boolean;

  /**
   * PointerEvent.accelKey
   */
  readonly accelKey: boolean;

  /**
   * MouseEvent.button
   */
  readonly button: MouseButton;

  constructor(options: {
    id?: string;
    parent?: Space | NodeReference | null;
    space?: Space | NodeReference | null;
    snapshot?: Snapshot | NodeReference | null;
    createdAt?: Temporal.ZonedDateTime;
    createdBy?: (Node & IsSubject) | NodeReference | null;
    client?: Client | NodeReference | null;
    clientNonce?: string | null;
    status?: EventStatus;
    node?: Node | NodeReference | null;
    position: Vector2f;
    pressure: number;
    shiftKey: boolean;
    altKey: boolean;
    ctrlKey: boolean;
    metaKey: boolean;
    accelKey: boolean;
    button: MouseButton;
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
      throw new Error(`MiddleClickEvent.status is required`);
    }
    this.status = _status;
    let _node = options.node ?? null;
    if (_node != null && _node.metatype != StructType.NODE_REFERENCE) {
      _node = (_node as Node).toRef();
    }
    this.nodePtr = _node;
    let _position = options.position;
    if (_position === null) {
      throw new Error(`MiddleClickEvent.position is required`);
    }
    this.position = _position;
    let _pressure = options.pressure;
    if (_pressure === null) {
      throw new Error(`MiddleClickEvent.pressure is required`);
    }
    this.pressure = _pressure;
    let _shiftKey = options.shiftKey;
    if (_shiftKey === null) {
      throw new Error(`MiddleClickEvent.shiftKey is required`);
    }
    this.shiftKey = _shiftKey;
    let _altKey = options.altKey;
    if (_altKey === null) {
      throw new Error(`MiddleClickEvent.altKey is required`);
    }
    this.altKey = _altKey;
    let _ctrlKey = options.ctrlKey;
    if (_ctrlKey === null) {
      throw new Error(`MiddleClickEvent.ctrlKey is required`);
    }
    this.ctrlKey = _ctrlKey;
    let _metaKey = options.metaKey;
    if (_metaKey === null) {
      throw new Error(`MiddleClickEvent.metaKey is required`);
    }
    this.metaKey = _metaKey;
    let _accelKey = options.accelKey;
    if (_accelKey === null) {
      throw new Error(`MiddleClickEvent.accelKey is required`);
    }
    this.accelKey = _accelKey;
    let _button = options.button;
    if (_button === null) {
      throw new Error(`MiddleClickEvent.button is required`);
    }
    this.button = _button;

    // identity
    if (options.id == null) {
      const now = Temporal.Now.zonedDateTimeISO("UTC");
      this.createdAt = now;
      this.createdByPtr = null;
    } else {
      if (options.createdAt == null) {
        throw new Error(`MiddleClickEvent.createdAt is required for existing Events`);
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
    if (!(this.button === other.button)) {
      return false;
    }
    if (!this.position.equals(other.position)) {
      return false;
    }
    if (!(this.pressure === other.pressure || Math.abs(this.pressure - other.pressure) < 1e-10)) {
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
    if (!(this.accelKey === other.accelKey)) {
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
    if (!(this.nodePtr?.id === other.nodePtr?.id)) {
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
    h = (h * 31 + this.button) & 0xffffffff;
    h = (h * 31 + this.position.hash()) & 0xffffffff;
    h = (h * 31 + hashFloat(this.pressure)) & 0xffffffff;
    h = (h * 31 + hashBool(this.shiftKey)) & 0xffffffff;
    h = (h * 31 + hashBool(this.altKey)) & 0xffffffff;
    h = (h * 31 + hashBool(this.ctrlKey)) & 0xffffffff;
    h = (h * 31 + hashBool(this.metaKey)) & 0xffffffff;
    h = (h * 31 + hashBool(this.accelKey)) & 0xffffffff;
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
    if (this.nodePtr !== null) {
      h = (h * 31 + hashString(this.nodePtr.id)) & 0xffffffff;
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
      type: NodeType.MIDDLE_CLICK_EVENT,
      id: this.id,
      spaceId: this.spacePtr?.id ?? null,
      snapshotId: this.snapshotPtr?.id ?? null,
      _session: this._session,
      _supergraph: this._supergraph,
    });
  }

  get _pathKey(): string {
    return `MiddleClickEvent[id=${this.id}]`;
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
    propertyReprs.push(`status=${EventStatus[this.status]}`);
    return `<MiddleClickEvent '${this.path}' ${propertyReprs.join(" ")}>`;
  }

  toValue(): { [key: string]: any } {
    return MiddleClickEvent.__packValue__(this);
  }

  static __packValue__(object: MiddleClickEvent): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 560204;
    objectValue["2"] = String(object.id);
    if (object.parentPtr != null) {
      objectValue["3"] = object.parentPtr.toValue();
    }
    if (object.spacePtr != null) {
      objectValue["5"] = object.spacePtr.toValue();
    }
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
    objectValue["110"] = object.position.toValue();
    objectValue["111"] = object.pressure;
    objectValue["120"] = object.shiftKey;
    objectValue["121"] = object.altKey;
    objectValue["122"] = object.ctrlKey;
    objectValue["123"] = object.metaKey;
    objectValue["124"] = object.accelKey;
    objectValue["130"] = object.button;
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): MiddleClickEvent {
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    const _Vector2f = STRUCT_CLASS_BY_TYPE[StructType.VECTOR2F] as typeof Vector2f;
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
    const nodePtrValue = objectValue["101"];
    const unpackedNodePtr =
      nodePtrValue != undefined
        ? _NodeReference.fromValue(nodePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const spacePtrValue = objectValue["5"];
    const unpackedSpacePtr =
      spacePtrValue != undefined
        ? _NodeReference.fromValue(spacePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    return new MiddleClickEvent({
      button: Number(objectValue["130"]),
      position: _Vector2f.fromValue(objectValue["110"], _session, _supergraph, _graph, _connection),
      pressure: objectValue["111"],
      shiftKey: objectValue["120"],
      altKey: objectValue["121"],
      ctrlKey: objectValue["122"],
      metaKey: objectValue["123"],
      accelKey: objectValue["124"],
      parent: unpackedParentPtr,
      snapshot: unpackedSnapshotPtr,
      createdAt: Temporal.Instant.from(objectValue["20"]).toZonedDateTimeISO("UTC"),
      createdBy: unpackedCreatedByPtr,
      client: unpackedClientPtr,
      clientNonce: unpackedClientNonce,
      status: Number(objectValue["30"]),
      node: unpackedNodePtr,
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
  ): MiddleClickEvent {
    return MiddleClickEvent.__unpackValue__(
      objectValue,
      _session,
      _supergraph,
      _graph,
      _connection,
    );
  }

  toProto(): MiddleClickEventProto {
    return MiddleClickEvent.__packProto__(this);
  }

  static __packProto__(object: MiddleClickEvent): MiddleClickEventProto {
    const objectProto: Partial<MiddleClickEventProto> = { metatype: 560204 };
    objectProto.id = String(object.id);
    if (object.parentPtr != null) {
      objectProto.parentPtr = object.parentPtr.toProto();
    }
    if (object.spacePtr != null) {
      objectProto.spacePtr = object.spacePtr.toProto();
    }
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
    objectProto.position = object.position.toProto();
    objectProto.pressure = object.pressure;
    objectProto.shiftKey = object.shiftKey;
    objectProto.altKey = object.altKey;
    objectProto.ctrlKey = object.ctrlKey;
    objectProto.metaKey = object.metaKey;
    objectProto.accelKey = object.accelKey;
    objectProto.button = Number(object.button) as MouseButtonProto;
    return objectProto as MiddleClickEventProto;
  }

  static __unpackProto__(
    objectProto: MiddleClickEventProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): MiddleClickEvent {
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    const _Vector2f = STRUCT_CLASS_BY_TYPE[StructType.VECTOR2F] as typeof Vector2f;
    return new MiddleClickEvent({
      button: Number(objectProto.button) as MouseButton,
      position: _Vector2f.fromProto(
        objectProto.position!,
        _session,
        _supergraph,
        _graph,
        _connection,
      ),
      pressure: objectProto.pressure,
      shiftKey: objectProto.shiftKey,
      altKey: objectProto.altKey,
      ctrlKey: objectProto.ctrlKey,
      metaKey: objectProto.metaKey,
      accelKey: objectProto.accelKey,
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
    objectProto: MiddleClickEventProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): MiddleClickEvent {
    return MiddleClickEvent.__unpackProto__(
      objectProto,
      _session,
      _supergraph,
      _graph,
      _connection,
    );
  }

  static fromProtoString(packedProtoString: string): MiddleClickEvent {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = MiddleClickEventProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerNodeClass(NodeType.MIDDLE_CLICK_EVENT, MiddleClickEvent);
/* ==== DESTACK_GENERATED_END:NODE:560204 ==== */

/* ==== DESTACK_GENERATED_START:NODE:560205 ==== */
/**
 * A DoubleClickEvent is a ClickEvent when a pointer is clicked twice in a short time.
 */
export class DoubleClickEvent extends ClickEvent {
  static metatype: NodeType = NodeType.DOUBLE_CLICK_EVENT;

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
  get createdBy(): (Node & IsSubject) | null {
    const nodePtr: NodeReference | null = this.createdByPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as (Node & IsSubject) | null;
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
   * The Node this Event is about.
   */
  get node(): Node | null {
    const nodePtr: NodeReference | null = this.nodePtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Node | null;
    }
    return null;
  }
  readonly nodePtr: NodeReference | null;

  /**
   * PointerEvent.position
   */
  readonly position: Vector2f;

  /**
   * PointerEvent.pressure
   */
  readonly pressure: number;

  /**
   * PointerEvent.shiftKey
   */
  readonly shiftKey: boolean;

  /**
   * PointerEvent.altKey
   */
  readonly altKey: boolean;

  /**
   * PointerEvent.ctrlKey
   */
  readonly ctrlKey: boolean;

  /**
   * PointerEvent.metaKey
   */
  readonly metaKey: boolean;

  /**
   * PointerEvent.accelKey
   */
  readonly accelKey: boolean;

  /**
   * MouseEvent.button
   */
  readonly button: MouseButton;

  constructor(options: {
    id?: string;
    parent?: Space | NodeReference | null;
    space?: Space | NodeReference | null;
    snapshot?: Snapshot | NodeReference | null;
    createdAt?: Temporal.ZonedDateTime;
    createdBy?: (Node & IsSubject) | NodeReference | null;
    client?: Client | NodeReference | null;
    clientNonce?: string | null;
    status?: EventStatus;
    node?: Node | NodeReference | null;
    position: Vector2f;
    pressure: number;
    shiftKey: boolean;
    altKey: boolean;
    ctrlKey: boolean;
    metaKey: boolean;
    accelKey: boolean;
    button: MouseButton;
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
      throw new Error(`DoubleClickEvent.status is required`);
    }
    this.status = _status;
    let _node = options.node ?? null;
    if (_node != null && _node.metatype != StructType.NODE_REFERENCE) {
      _node = (_node as Node).toRef();
    }
    this.nodePtr = _node;
    let _position = options.position;
    if (_position === null) {
      throw new Error(`DoubleClickEvent.position is required`);
    }
    this.position = _position;
    let _pressure = options.pressure;
    if (_pressure === null) {
      throw new Error(`DoubleClickEvent.pressure is required`);
    }
    this.pressure = _pressure;
    let _shiftKey = options.shiftKey;
    if (_shiftKey === null) {
      throw new Error(`DoubleClickEvent.shiftKey is required`);
    }
    this.shiftKey = _shiftKey;
    let _altKey = options.altKey;
    if (_altKey === null) {
      throw new Error(`DoubleClickEvent.altKey is required`);
    }
    this.altKey = _altKey;
    let _ctrlKey = options.ctrlKey;
    if (_ctrlKey === null) {
      throw new Error(`DoubleClickEvent.ctrlKey is required`);
    }
    this.ctrlKey = _ctrlKey;
    let _metaKey = options.metaKey;
    if (_metaKey === null) {
      throw new Error(`DoubleClickEvent.metaKey is required`);
    }
    this.metaKey = _metaKey;
    let _accelKey = options.accelKey;
    if (_accelKey === null) {
      throw new Error(`DoubleClickEvent.accelKey is required`);
    }
    this.accelKey = _accelKey;
    let _button = options.button;
    if (_button === null) {
      throw new Error(`DoubleClickEvent.button is required`);
    }
    this.button = _button;

    // identity
    if (options.id == null) {
      const now = Temporal.Now.zonedDateTimeISO("UTC");
      this.createdAt = now;
      this.createdByPtr = null;
    } else {
      if (options.createdAt == null) {
        throw new Error(`DoubleClickEvent.createdAt is required for existing Events`);
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
    if (!(this.button === other.button)) {
      return false;
    }
    if (!this.position.equals(other.position)) {
      return false;
    }
    if (!(this.pressure === other.pressure || Math.abs(this.pressure - other.pressure) < 1e-10)) {
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
    if (!(this.accelKey === other.accelKey)) {
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
    if (!(this.nodePtr?.id === other.nodePtr?.id)) {
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
    h = (h * 31 + this.button) & 0xffffffff;
    h = (h * 31 + this.position.hash()) & 0xffffffff;
    h = (h * 31 + hashFloat(this.pressure)) & 0xffffffff;
    h = (h * 31 + hashBool(this.shiftKey)) & 0xffffffff;
    h = (h * 31 + hashBool(this.altKey)) & 0xffffffff;
    h = (h * 31 + hashBool(this.ctrlKey)) & 0xffffffff;
    h = (h * 31 + hashBool(this.metaKey)) & 0xffffffff;
    h = (h * 31 + hashBool(this.accelKey)) & 0xffffffff;
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
    if (this.nodePtr !== null) {
      h = (h * 31 + hashString(this.nodePtr.id)) & 0xffffffff;
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
      type: NodeType.DOUBLE_CLICK_EVENT,
      id: this.id,
      spaceId: this.spacePtr?.id ?? null,
      snapshotId: this.snapshotPtr?.id ?? null,
      _session: this._session,
      _supergraph: this._supergraph,
    });
  }

  get _pathKey(): string {
    return `DoubleClickEvent[id=${this.id}]`;
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
    propertyReprs.push(`status=${EventStatus[this.status]}`);
    return `<DoubleClickEvent '${this.path}' ${propertyReprs.join(" ")}>`;
  }

  toValue(): { [key: string]: any } {
    return DoubleClickEvent.__packValue__(this);
  }

  static __packValue__(object: DoubleClickEvent): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 560205;
    objectValue["2"] = String(object.id);
    if (object.parentPtr != null) {
      objectValue["3"] = object.parentPtr.toValue();
    }
    if (object.spacePtr != null) {
      objectValue["5"] = object.spacePtr.toValue();
    }
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
    objectValue["110"] = object.position.toValue();
    objectValue["111"] = object.pressure;
    objectValue["120"] = object.shiftKey;
    objectValue["121"] = object.altKey;
    objectValue["122"] = object.ctrlKey;
    objectValue["123"] = object.metaKey;
    objectValue["124"] = object.accelKey;
    objectValue["130"] = object.button;
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): DoubleClickEvent {
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    const _Vector2f = STRUCT_CLASS_BY_TYPE[StructType.VECTOR2F] as typeof Vector2f;
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
    const nodePtrValue = objectValue["101"];
    const unpackedNodePtr =
      nodePtrValue != undefined
        ? _NodeReference.fromValue(nodePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const spacePtrValue = objectValue["5"];
    const unpackedSpacePtr =
      spacePtrValue != undefined
        ? _NodeReference.fromValue(spacePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    return new DoubleClickEvent({
      button: Number(objectValue["130"]),
      position: _Vector2f.fromValue(objectValue["110"], _session, _supergraph, _graph, _connection),
      pressure: objectValue["111"],
      shiftKey: objectValue["120"],
      altKey: objectValue["121"],
      ctrlKey: objectValue["122"],
      metaKey: objectValue["123"],
      accelKey: objectValue["124"],
      parent: unpackedParentPtr,
      snapshot: unpackedSnapshotPtr,
      createdAt: Temporal.Instant.from(objectValue["20"]).toZonedDateTimeISO("UTC"),
      createdBy: unpackedCreatedByPtr,
      client: unpackedClientPtr,
      clientNonce: unpackedClientNonce,
      status: Number(objectValue["30"]),
      node: unpackedNodePtr,
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
  ): DoubleClickEvent {
    return DoubleClickEvent.__unpackValue__(
      objectValue,
      _session,
      _supergraph,
      _graph,
      _connection,
    );
  }

  toProto(): DoubleClickEventProto {
    return DoubleClickEvent.__packProto__(this);
  }

  static __packProto__(object: DoubleClickEvent): DoubleClickEventProto {
    const objectProto: Partial<DoubleClickEventProto> = { metatype: 560205 };
    objectProto.id = String(object.id);
    if (object.parentPtr != null) {
      objectProto.parentPtr = object.parentPtr.toProto();
    }
    if (object.spacePtr != null) {
      objectProto.spacePtr = object.spacePtr.toProto();
    }
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
    objectProto.position = object.position.toProto();
    objectProto.pressure = object.pressure;
    objectProto.shiftKey = object.shiftKey;
    objectProto.altKey = object.altKey;
    objectProto.ctrlKey = object.ctrlKey;
    objectProto.metaKey = object.metaKey;
    objectProto.accelKey = object.accelKey;
    objectProto.button = Number(object.button) as MouseButtonProto;
    return objectProto as DoubleClickEventProto;
  }

  static __unpackProto__(
    objectProto: DoubleClickEventProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): DoubleClickEvent {
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    const _Vector2f = STRUCT_CLASS_BY_TYPE[StructType.VECTOR2F] as typeof Vector2f;
    return new DoubleClickEvent({
      button: Number(objectProto.button) as MouseButton,
      position: _Vector2f.fromProto(
        objectProto.position!,
        _session,
        _supergraph,
        _graph,
        _connection,
      ),
      pressure: objectProto.pressure,
      shiftKey: objectProto.shiftKey,
      altKey: objectProto.altKey,
      ctrlKey: objectProto.ctrlKey,
      metaKey: objectProto.metaKey,
      accelKey: objectProto.accelKey,
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
    objectProto: DoubleClickEventProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): DoubleClickEvent {
    return DoubleClickEvent.__unpackProto__(
      objectProto,
      _session,
      _supergraph,
      _graph,
      _connection,
    );
  }

  static fromProtoString(packedProtoString: string): DoubleClickEvent {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = DoubleClickEventProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerNodeClass(NodeType.DOUBLE_CLICK_EVENT, DoubleClickEvent);
/* ==== DESTACK_GENERATED_END:NODE:560205 ==== */

/* ==== DESTACK_GENERATED_START:NODE:560206 ==== */
/**
 * A WheelEvent is a MouseEvent when a wheel is scrolled.
 */
export class WheelEvent extends MouseEvent {
  static metatype: NodeType = NodeType.WHEEL_EVENT;

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
  get createdBy(): (Node & IsSubject) | null {
    const nodePtr: NodeReference | null = this.createdByPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as (Node & IsSubject) | null;
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
   * The Node this Event is about.
   */
  get node(): Node | null {
    const nodePtr: NodeReference | null = this.nodePtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Node | null;
    }
    return null;
  }
  readonly nodePtr: NodeReference | null;

  /**
   * PointerEvent.position
   */
  readonly position: Vector2f;

  /**
   * PointerEvent.pressure
   */
  readonly pressure: number;

  /**
   * PointerEvent.shiftKey
   */
  readonly shiftKey: boolean;

  /**
   * PointerEvent.altKey
   */
  readonly altKey: boolean;

  /**
   * PointerEvent.ctrlKey
   */
  readonly ctrlKey: boolean;

  /**
   * PointerEvent.metaKey
   */
  readonly metaKey: boolean;

  /**
   * PointerEvent.accelKey
   */
  readonly accelKey: boolean;

  /**
   * MouseEvent.button
   */
  readonly button: MouseButton;

  /**
   * WheelEvent.delta
   */
  readonly delta: Vector2f;

  constructor(options: {
    id?: string;
    parent?: Space | NodeReference | null;
    space?: Space | NodeReference | null;
    snapshot?: Snapshot | NodeReference | null;
    createdAt?: Temporal.ZonedDateTime;
    createdBy?: (Node & IsSubject) | NodeReference | null;
    client?: Client | NodeReference | null;
    clientNonce?: string | null;
    status?: EventStatus;
    node?: Node | NodeReference | null;
    position: Vector2f;
    pressure: number;
    shiftKey: boolean;
    altKey: boolean;
    ctrlKey: boolean;
    metaKey: boolean;
    accelKey: boolean;
    button: MouseButton;
    delta: Vector2f;
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
      throw new Error(`WheelEvent.status is required`);
    }
    this.status = _status;
    let _node = options.node ?? null;
    if (_node != null && _node.metatype != StructType.NODE_REFERENCE) {
      _node = (_node as Node).toRef();
    }
    this.nodePtr = _node;
    let _position = options.position;
    if (_position === null) {
      throw new Error(`WheelEvent.position is required`);
    }
    this.position = _position;
    let _pressure = options.pressure;
    if (_pressure === null) {
      throw new Error(`WheelEvent.pressure is required`);
    }
    this.pressure = _pressure;
    let _shiftKey = options.shiftKey;
    if (_shiftKey === null) {
      throw new Error(`WheelEvent.shiftKey is required`);
    }
    this.shiftKey = _shiftKey;
    let _altKey = options.altKey;
    if (_altKey === null) {
      throw new Error(`WheelEvent.altKey is required`);
    }
    this.altKey = _altKey;
    let _ctrlKey = options.ctrlKey;
    if (_ctrlKey === null) {
      throw new Error(`WheelEvent.ctrlKey is required`);
    }
    this.ctrlKey = _ctrlKey;
    let _metaKey = options.metaKey;
    if (_metaKey === null) {
      throw new Error(`WheelEvent.metaKey is required`);
    }
    this.metaKey = _metaKey;
    let _accelKey = options.accelKey;
    if (_accelKey === null) {
      throw new Error(`WheelEvent.accelKey is required`);
    }
    this.accelKey = _accelKey;
    let _button = options.button;
    if (_button === null) {
      throw new Error(`WheelEvent.button is required`);
    }
    this.button = _button;
    let _delta = options.delta;
    if (_delta === null) {
      throw new Error(`WheelEvent.delta is required`);
    }
    this.delta = _delta;

    // identity
    if (options.id == null) {
      const now = Temporal.Now.zonedDateTimeISO("UTC");
      this.createdAt = now;
      this.createdByPtr = null;
    } else {
      if (options.createdAt == null) {
        throw new Error(`WheelEvent.createdAt is required for existing Events`);
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
    if (!this.delta.equals(other.delta)) {
      return false;
    }
    if (!(this.button === other.button)) {
      return false;
    }
    if (!this.position.equals(other.position)) {
      return false;
    }
    if (!(this.pressure === other.pressure || Math.abs(this.pressure - other.pressure) < 1e-10)) {
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
    if (!(this.accelKey === other.accelKey)) {
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
    if (!(this.nodePtr?.id === other.nodePtr?.id)) {
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
    h = (h * 31 + this.delta.hash()) & 0xffffffff;
    h = (h * 31 + this.button) & 0xffffffff;
    h = (h * 31 + this.position.hash()) & 0xffffffff;
    h = (h * 31 + hashFloat(this.pressure)) & 0xffffffff;
    h = (h * 31 + hashBool(this.shiftKey)) & 0xffffffff;
    h = (h * 31 + hashBool(this.altKey)) & 0xffffffff;
    h = (h * 31 + hashBool(this.ctrlKey)) & 0xffffffff;
    h = (h * 31 + hashBool(this.metaKey)) & 0xffffffff;
    h = (h * 31 + hashBool(this.accelKey)) & 0xffffffff;
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
    if (this.nodePtr !== null) {
      h = (h * 31 + hashString(this.nodePtr.id)) & 0xffffffff;
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
      type: NodeType.WHEEL_EVENT,
      id: this.id,
      spaceId: this.spacePtr?.id ?? null,
      snapshotId: this.snapshotPtr?.id ?? null,
      _session: this._session,
      _supergraph: this._supergraph,
    });
  }

  get _pathKey(): string {
    return `WheelEvent[id=${this.id}]`;
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
    propertyReprs.push(`status=${EventStatus[this.status]}`);
    return `<WheelEvent '${this.path}' ${propertyReprs.join(" ")}>`;
  }

  toValue(): { [key: string]: any } {
    return WheelEvent.__packValue__(this);
  }

  static __packValue__(object: WheelEvent): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 560206;
    objectValue["2"] = String(object.id);
    if (object.parentPtr != null) {
      objectValue["3"] = object.parentPtr.toValue();
    }
    if (object.spacePtr != null) {
      objectValue["5"] = object.spacePtr.toValue();
    }
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
    objectValue["110"] = object.position.toValue();
    objectValue["111"] = object.pressure;
    objectValue["120"] = object.shiftKey;
    objectValue["121"] = object.altKey;
    objectValue["122"] = object.ctrlKey;
    objectValue["123"] = object.metaKey;
    objectValue["124"] = object.accelKey;
    objectValue["130"] = object.button;
    objectValue["140"] = object.delta.toValue();
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): WheelEvent {
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    const _Vector2f = STRUCT_CLASS_BY_TYPE[StructType.VECTOR2F] as typeof Vector2f;
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
    const nodePtrValue = objectValue["101"];
    const unpackedNodePtr =
      nodePtrValue != undefined
        ? _NodeReference.fromValue(nodePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const spacePtrValue = objectValue["5"];
    const unpackedSpacePtr =
      spacePtrValue != undefined
        ? _NodeReference.fromValue(spacePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    return new WheelEvent({
      delta: _Vector2f.fromValue(objectValue["140"], _session, _supergraph, _graph, _connection),
      button: Number(objectValue["130"]),
      position: _Vector2f.fromValue(objectValue["110"], _session, _supergraph, _graph, _connection),
      pressure: objectValue["111"],
      shiftKey: objectValue["120"],
      altKey: objectValue["121"],
      ctrlKey: objectValue["122"],
      metaKey: objectValue["123"],
      accelKey: objectValue["124"],
      parent: unpackedParentPtr,
      snapshot: unpackedSnapshotPtr,
      createdAt: Temporal.Instant.from(objectValue["20"]).toZonedDateTimeISO("UTC"),
      createdBy: unpackedCreatedByPtr,
      client: unpackedClientPtr,
      clientNonce: unpackedClientNonce,
      status: Number(objectValue["30"]),
      node: unpackedNodePtr,
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
  ): WheelEvent {
    return WheelEvent.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
  }

  toProto(): WheelEventProto {
    return WheelEvent.__packProto__(this);
  }

  static __packProto__(object: WheelEvent): WheelEventProto {
    const objectProto: Partial<WheelEventProto> = { metatype: 560206 };
    objectProto.id = String(object.id);
    if (object.parentPtr != null) {
      objectProto.parentPtr = object.parentPtr.toProto();
    }
    if (object.spacePtr != null) {
      objectProto.spacePtr = object.spacePtr.toProto();
    }
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
    objectProto.position = object.position.toProto();
    objectProto.pressure = object.pressure;
    objectProto.shiftKey = object.shiftKey;
    objectProto.altKey = object.altKey;
    objectProto.ctrlKey = object.ctrlKey;
    objectProto.metaKey = object.metaKey;
    objectProto.accelKey = object.accelKey;
    objectProto.button = Number(object.button) as MouseButtonProto;
    objectProto.delta = object.delta.toProto();
    return objectProto as WheelEventProto;
  }

  static __unpackProto__(
    objectProto: WheelEventProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): WheelEvent {
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    const _Vector2f = STRUCT_CLASS_BY_TYPE[StructType.VECTOR2F] as typeof Vector2f;
    return new WheelEvent({
      delta: _Vector2f.fromProto(objectProto.delta!, _session, _supergraph, _graph, _connection),
      button: Number(objectProto.button) as MouseButton,
      position: _Vector2f.fromProto(
        objectProto.position!,
        _session,
        _supergraph,
        _graph,
        _connection,
      ),
      pressure: objectProto.pressure,
      shiftKey: objectProto.shiftKey,
      altKey: objectProto.altKey,
      ctrlKey: objectProto.ctrlKey,
      metaKey: objectProto.metaKey,
      accelKey: objectProto.accelKey,
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
    objectProto: WheelEventProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): WheelEvent {
    return WheelEvent.__unpackProto__(objectProto, _session, _supergraph, _graph, _connection);
  }

  static fromProtoString(packedProtoString: string): WheelEvent {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = WheelEventProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerNodeClass(NodeType.WHEEL_EVENT, WheelEvent);
/* ==== DESTACK_GENERATED_END:NODE:560206 ==== */
