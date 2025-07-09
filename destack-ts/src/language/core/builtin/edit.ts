import { packProtoTimestamp, unpackProtoTimestamp } from "@destack/grpc";
import { EnumType, NodeType, StructType } from "@destack/language/core/builtin/common";
import type { Snapshot } from "@destack/language/core/builtin/entity";
import { Entity } from "@destack/language/core/builtin/entity";
import { Event, EventStatus } from "@destack/language/core/builtin/event";
import { Node } from "@destack/language/core/builtin/node";
import type { NodeReference, PropertyReference } from "@destack/language/core/builtin/relation";
import type { IsSubject } from "@destack/language/core/builtin/trait";
import type { Value } from "@destack/language/core/common/value";
import type { QueryConnection } from "@destack/language/core/runtime/connection";
import type { Graph, Supergraph } from "@destack/language/core/runtime/graph";
import type { Session } from "@destack/language/core/runtime/session";
import {
  STRUCT_CLASS_BY_TYPE,
  registerEnumClass,
  registerNodeClass,
} from "@destack/language/registry";
import type { Client, Space } from "@destack/language/universe";
import {
  EditEventProto,
  EditOperationProto,
  EditTypeProto,
  EventStatusProto,
} from "@destack/proto";
import { base64Decode } from "@destack/utils";
import { hashString } from "@destack/utils/hash";
import { Temporal } from "temporal-polyfill";

/* ==== DESTACK_GENERATED_START:ENUM:50 ==== */
/**
 * EditType
 */
export enum EditType {
  CREATE = 1,
  UPSERT = 2,
  UPDATE = 3,
  MOVE = 4,
  ARCHIVE = 5,
  UNARCHIVE = 6,
  DELETE = 7,
  RESTORE = 8,
  ERASE = 9,

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerEnumClass(EnumType.EDIT_TYPE, EditType);
/* ==== DESTACK_GENERATED_END:ENUM:50 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:51 ==== */
/**
 * EditOperation
 */
export enum EditOperation {
  SET = 1,
  CLEAR = 2,

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerEnumClass(EnumType.EDIT_OPERATION, EditOperation);
/* ==== DESTACK_GENERATED_END:ENUM:51 ==== */

/* ==== DESTACK_GENERATED_START:NODE:2001 ==== */
/**
 * A recorded Edit of an Entity.
 */
export class EditEvent extends Event {
  static metatype: NodeType = NodeType.EDIT_EVENT;

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
   * The type of Edit.
   */
  readonly type: EditType;

  /**
   * The Entity being edited.
   */
  get node(): Entity | null {
    const nodePtr: NodeReference | null = this.nodePtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Entity | null;
    }
    return null;
  }
  readonly nodePtr: NodeReference;

  /**
   * The specific Edit operation.
   */
  readonly operation: EditOperation | null;

  /**
   * The builtin or custom Property being edited.
   */
  readonly attribute: PropertyReference | null;

  /**
   * The key for map operations.
   */
  readonly key: Value | null;

  /**
   * EditEvent.value
   */
  readonly value: Value | null;

  /**
   * The specific reverse Edit operation.
   */
  readonly reverseOperation: EditOperation | null;

  /**
   * The value of the reverse Edit.
   */
  readonly reverseValue: Value | null;

  constructor(options: {
    id?: string;
    parent?: Space | NodeReference | null;
    space?: Space | NodeReference | null;
    snapshot?: Snapshot | NodeReference | null;
    createdAt?: Temporal.ZonedDateTime;
    createdBy?: (Entity & IsSubject) | NodeReference | null;
    client?: Client | NodeReference | null;
    clientNonce?: string | null;
    status?: EventStatus;
    type: EditType;
    node: Entity | NodeReference;
    operation?: EditOperation | null;
    attribute?: PropertyReference | null;
    key?: Value | null;
    value?: Value | null;
    reverseOperation?: EditOperation | null;
    reverseValue?: Value | null;
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
      throw new Error(`EditEvent.status is required`);
    }
    this.status = _status;
    let _type = options.type;
    if (_type === null) {
      throw new Error(`EditEvent.type is required`);
    }
    this.type = _type;
    let _node = options.node;
    if (_node != null && _node.metatype != StructType.NODE_REFERENCE) {
      _node = (_node as Node).toRef();
    }
    if (_node === null) {
      throw new Error(`EditEvent.node is required`);
    }
    this.nodePtr = _node;
    let _operation = options.operation ?? null;
    this.operation = _operation;
    let _attribute = options.attribute ?? null;
    this.attribute = _attribute;
    let _key = options.key ?? null;
    this.key = _key;
    let _value = options.value ?? null;
    this.value = _value;
    let _reverseOperation = options.reverseOperation ?? null;
    this.reverseOperation = _reverseOperation;
    let _reverseValue = options.reverseValue ?? null;
    this.reverseValue = _reverseValue;

    // identity
    if (options.id == null) {
      const now = Temporal.Now.zonedDateTimeISO("UTC");
      this.createdAt = now;
      this.createdByPtr = null;
    } else {
      if (options.createdAt == null) {
        throw new Error(`EditEvent.createdAt is required for existing Events`);
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
    if (!(this.type === other.type)) {
      return false;
    }
    if (!(this.nodePtr.id === other.nodePtr.id)) {
      return false;
    }
    if (!(this.operation === other.operation)) {
      return false;
    }
    if (
      (this.attribute == null) !== (other.attribute == null) ||
      (this.attribute != null && !this.attribute.equals(other.attribute))
    ) {
      return false;
    }
    if (
      (this.key == null) !== (other.key == null) ||
      (this.key != null && !this.key.equals(other.key))
    ) {
      return false;
    }
    if (
      (this.value == null) !== (other.value == null) ||
      (this.value != null && !this.value.equals(other.value))
    ) {
      return false;
    }
    if (!(this.reverseOperation === other.reverseOperation)) {
      return false;
    }
    if (
      (this.reverseValue == null) !== (other.reverseValue == null) ||
      (this.reverseValue != null && !this.reverseValue.equals(other.reverseValue))
    ) {
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
    if (!(this.spacePtr?.id === other.spacePtr?.id)) {
      return false;
    }
    return true;
  }

  hash(): number {
    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    h = (h * 31 + this.type) & 0xffffffff;
    h = (h * 31 + hashString(this.nodePtr.id)) & 0xffffffff;
    if (this.operation !== null) {
      h = (h * 31 + this.operation) & 0xffffffff;
    }
    if (this.attribute !== null) {
      h = (h * 31 + this.attribute.hash()) & 0xffffffff;
    }
    if (this.key !== null) {
      h = (h * 31 + this.key.hash()) & 0xffffffff;
    }
    if (this.value !== null) {
      h = (h * 31 + this.value.hash()) & 0xffffffff;
    }
    if (this.reverseOperation !== null) {
      h = (h * 31 + this.reverseOperation) & 0xffffffff;
    }
    if (this.reverseValue !== null) {
      h = (h * 31 + this.reverseValue.hash()) & 0xffffffff;
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
      type: NodeType.EDIT_EVENT,
      id: this.id,
      spaceId: this.spacePtr?.id ?? null,
      snapshotId: this.snapshotPtr?.id ?? null,
      _session: this._session,
      _supergraph: this._supergraph,
    });
  }

  get _pathKey(): string {
    return `EditEvent[id=${this.id}]`;
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
    propertyReprs.push(`type=${EditType[this.type]}`);
    propertyReprs.push(`node=${this.node?.repr()}`);
    if (this.operation !== null) {
      propertyReprs.push(`operation=${EditOperation[this.operation]}`);
    }
    if (this.attribute !== null) {
      propertyReprs.push(`attribute=${this.attribute.repr()}`);
    }
    if (this.key !== null) {
      propertyReprs.push(`key=${this.key.repr()}`);
    }
    if (this.reverseOperation !== null) {
      propertyReprs.push(`reverseOperation=${EditOperation[this.reverseOperation]}`);
    }
    if (this.reverseValue !== null) {
      propertyReprs.push(`reverseValue=${this.reverseValue.repr()}`);
    }
    propertyReprs.push(`status=${EventStatus[this.status]}`);
    return `<EditEvent '${this.path}' ${propertyReprs.join(" ")}>`;
  }

  toValue(): { readonly [key: string]: any } {
    return EditEvent.__packValue__(this);
  }

  static __packValue__(object: EditEvent): { readonly [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 2001;
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
    objectValue["100"] = object.type;
    objectValue["101"] = object.nodePtr.toValue();
    if (object.operation != null) {
      objectValue["102"] = object.operation;
    }
    if (object.attribute != null) {
      objectValue["103"] = object.attribute.toValue();
    }
    if (object.key != null) {
      objectValue["105"] = object.key.toValue();
    }
    if (object.value != null) {
      objectValue["110"] = object.value.toValue();
    }
    if (object.reverseOperation != null) {
      objectValue["202"] = object.reverseOperation;
    }
    if (object.reverseValue != null) {
      objectValue["210"] = object.reverseValue.toValue();
    }
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { readonly [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): EditEvent {
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    const _PropertyReference = STRUCT_CLASS_BY_TYPE[
      StructType.PROPERTY_REFERENCE
    ] as typeof PropertyReference;
    const _Value = STRUCT_CLASS_BY_TYPE[StructType.VALUE] as typeof Value;
    const operationValue = objectValue["102"];
    const unpackedOperation = operationValue != undefined ? Number(operationValue) : null;
    const attributeValue = objectValue["103"];
    const unpackedAttribute =
      attributeValue != undefined
        ? _PropertyReference.fromValue(attributeValue, _session, _supergraph, _graph, _connection)
        : null;
    const keyValue = objectValue["105"];
    const unpackedKey =
      keyValue != undefined
        ? _Value.fromValue(keyValue, _session, _supergraph, _graph, _connection)
        : null;
    const valueValue = objectValue["110"];
    const unpackedValue =
      valueValue != undefined
        ? _Value.fromValue(valueValue, _session, _supergraph, _graph, _connection)
        : null;
    const reverseOperationValue = objectValue["202"];
    const unpackedReverseOperation =
      reverseOperationValue != undefined ? Number(reverseOperationValue) : null;
    const reverseValueValue = objectValue["210"];
    const unpackedReverseValue =
      reverseValueValue != undefined
        ? _Value.fromValue(reverseValueValue, _session, _supergraph, _graph, _connection)
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
    const spacePtrValue = objectValue["5"];
    const unpackedSpacePtr =
      spacePtrValue != undefined
        ? _NodeReference.fromValue(spacePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    return new EditEvent({
      type: Number(objectValue["100"]),
      node: _NodeReference.fromValue(
        objectValue["101"],
        _session,
        _supergraph,
        _graph,
        _connection,
      ),
      operation: unpackedOperation,
      attribute: unpackedAttribute,
      key: unpackedKey,
      value: unpackedValue,
      reverseOperation: unpackedReverseOperation,
      reverseValue: unpackedReverseValue,
      parent: unpackedParentPtr,
      snapshot: unpackedSnapshotPtr,
      createdAt: Temporal.Instant.from(objectValue["20"]).toZonedDateTimeISO("UTC"),
      createdBy: unpackedCreatedByPtr,
      client: unpackedClientPtr,
      clientNonce: unpackedClientNonce,
      status: Number(objectValue["30"]),
      space: unpackedSpacePtr,
      id: String(objectValue["2"]),
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
  ): EditEvent {
    return EditEvent.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
  }

  toProto(): EditEventProto {
    return EditEvent.__packProto__(this);
  }

  static __packProto__(object: EditEvent): EditEventProto {
    const objectProto: Partial<EditEventProto> = { metatype: 2001 };
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
    objectProto.type = Number(object.type) as EditTypeProto;
    objectProto.nodePtr = object.nodePtr.toProto();
    if (object.operation != null) {
      objectProto.operation = Number(object.operation) as EditOperationProto;
    }
    if (object.attribute != null) {
      objectProto.attribute = object.attribute.toProto();
    }
    if (object.key != null) {
      objectProto.key = object.key.toProto();
    }
    if (object.value != null) {
      objectProto.value = object.value.toProto();
    }
    if (object.reverseOperation != null) {
      objectProto.reverseOperation = Number(object.reverseOperation) as EditOperationProto;
    }
    if (object.reverseValue != null) {
      objectProto.reverseValue = object.reverseValue.toProto();
    }
    return objectProto as EditEventProto;
  }

  static __unpackProto__(
    objectProto: EditEventProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): EditEvent {
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    const _PropertyReference = STRUCT_CLASS_BY_TYPE[
      StructType.PROPERTY_REFERENCE
    ] as typeof PropertyReference;
    const _Value = STRUCT_CLASS_BY_TYPE[StructType.VALUE] as typeof Value;
    return new EditEvent({
      type: Number(objectProto.type) as EditType,
      node: _NodeReference.fromProto(
        objectProto.nodePtr!,
        _session,
        _supergraph,
        _graph,
        _connection,
      ),
      operation:
        objectProto.operation != undefined
          ? (Number(objectProto.operation) as EditOperation)
          : null,
      attribute:
        objectProto.attribute != undefined
          ? _PropertyReference.fromProto(
              objectProto.attribute!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      key:
        objectProto.key != undefined
          ? _Value.fromProto(objectProto.key!, _session, _supergraph, _graph, _connection)
          : null,
      value:
        objectProto.value != undefined
          ? _Value.fromProto(objectProto.value!, _session, _supergraph, _graph, _connection)
          : null,
      reverseOperation:
        objectProto.reverseOperation != undefined
          ? (Number(objectProto.reverseOperation) as EditOperation)
          : null,
      reverseValue:
        objectProto.reverseValue != undefined
          ? _Value.fromProto(objectProto.reverseValue!, _session, _supergraph, _graph, _connection)
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
    objectProto: EditEventProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): EditEvent {
    return EditEvent.__unpackProto__(objectProto, _session, _supergraph, _graph, _connection);
  }

  static fromProtoString(packedProtoString: string): EditEvent {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = EditEventProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerNodeClass(NodeType.EDIT_EVENT, EditEvent);
/* ==== DESTACK_GENERATED_END:NODE:2001 ==== */

/* ==== DESTACK_GENERATED_START:CONSTANT:CASCADING_EDIT_TYPES ==== */
/**
 * CASCADING_EDIT_TYPES
 */
// prettier-ignore
export const CASCADING_EDIT_TYPES = [
  (5 /* EditType.ARCHIVE */),
  (6 /* EditType.UNARCHIVE */),
  (7 /* EditType.DELETE */),
  (8 /* EditType.RESTORE */),
  (9 /* EditType.ERASE */)
];

/* ==== DESTACK_GENERATED_END:CONSTANT:CASCADING_EDIT_TYPES ==== */
