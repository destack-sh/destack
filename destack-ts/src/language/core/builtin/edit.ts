import { EnumType, NodeType, StructType } from "@destack/language/core/builtin/common";
import { ACTIVE_BRANCH, ACTIVE_SNAPSHOT, ACTIVE_SPACE } from "@destack/language/core/builtin/const";
import { Entity } from "@destack/language/core/builtin/entity";
import { Event, EventStatus } from "@destack/language/core/builtin/event";
import { Node } from "@destack/language/core/builtin/node";
import type { NodeReference } from "@destack/language/core/builtin/relation";
import type { CustomProperty } from "@destack/language/core/common/property";
import type { Space } from "@destack/language/core/common/space";
import type { Branch, Snapshot } from "@destack/language/core/common/time";
import type { Value } from "@destack/language/core/common/value";
import type { GraphConnection } from "@destack/language/core/runtime/connection";
import type { Graph } from "@destack/language/core/runtime/graph";
import type { Session } from "@destack/language/core/runtime/session";
import {
  STRUCT_CLASS_BY_TYPE,
  registerEnumClass,
  registerNodeClass,
} from "@destack/language/registry";
import type { Client } from "@destack/language/universe";
import { hashInt, hashString } from "@destack/utils/hash";
import { Temporal } from "temporal-polyfill";

/* ==== DESTACK_GENERATED_START:ENUM:200 ==== */
/**
 * EditType
 */
export enum EditType {
  CREATE = 1,
  UPSERT = 2,
  UPDATE = 10,
  MOVE = 11,
  DELETE = 20,
  RESTORE = 21,

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerEnumClass(EnumType.EDIT_TYPE, EditType);
/* ==== DESTACK_GENERATED_END:ENUM:200 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:201 ==== */
/**
 * EditOperation
 */
export enum EditOperation {
  SET = 1,

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerEnumClass(EnumType.EDIT_OPERATION, EditOperation);
/* ==== DESTACK_GENERATED_END:ENUM:201 ==== */

/* ==== DESTACK_GENERATED_START:NODE:90100 ==== */
/**
 * A recorded Edit of an Entity.
 */
export class EditEvent extends Event {
  static metatype: NodeType = NodeType.EDIT_EVENT;

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
  get createdBy(): Entity | null {
    const nodePtr: NodeReference | null = this.createdByPtr;
    if (nodePtr != null) {
      return this._graph.get(nodePtr.id) as Entity | null;
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
   * The type of Edit.
   */
  readonly type: EditType;

  /**
   * The Entity being edited.
   */
  get node(): Entity | null {
    const nodePtr: NodeReference | null = this.nodePtr;
    if (nodePtr != null) {
      return this._graph.get(nodePtr.id) as Entity | null;
    }
    return null;
  }
  readonly nodePtr: NodeReference;

  /**
   * The specific Edit operation.
   */
  readonly operation: EditOperation | null;

  /**
   * The id of the builtin Property being edited (if not a custom Property).
   */
  readonly propertyId: number | null;

  /**
   * The custom Property being edited (if not a builtin).
   */
  get customProperty(): CustomProperty | null {
    const nodePtr: NodeReference | null = this.customPropertyPtr;
    if (nodePtr != null) {
      return this._graph.get(nodePtr.id) as CustomProperty | null;
    }
    return null;
  }
  readonly customPropertyPtr: NodeReference | null;

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
    space?: Space | NodeReference;
    definition?: Entity | NodeReference | null;
    branch?: Branch | NodeReference;
    snapshot?: Snapshot | NodeReference;
    precededBy?: Event | NodeReference | null;
    causedBy?: Event | NodeReference | null;
    createdAt?: Temporal.ZonedDateTime;
    createdEpoch?: number;
    createdBy?: Entity | NodeReference | null;
    client?: Client | NodeReference | null;
    clientNonce?: string | null;
    clientCreatedAt?: Temporal.ZonedDateTime;
    clientEpoch?: number;
    status?: EventStatus;
    type: EditType;
    node: Entity | NodeReference;
    operation?: EditOperation | null;
    propertyId?: number | null;
    customProperty?: CustomProperty | NodeReference | null;
    key?: Value | null;
    value?: Value | null;
    reverseOperation?: EditOperation | null;
    reverseValue?: Value | null;
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
        throw new Error(`no active Space for EditEvent`);
      }
      _space = _space.toRef();
    }
    if (_space === null) {
      throw new Error(`EditEvent.space is required`);
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
        throw new Error(`no active Branch for EditEvent`);
      }
      _branch = _branch.toRef();
    }
    if (_branch === null) {
      throw new Error(`EditEvent.branch is required`);
    }
    this.branchPtr = _branch as NodeReference;
    let _snapshot = options.snapshot ?? null;
    if (_snapshot != null && _snapshot.constructor.name != "NodeReference") {
      _snapshot = (_snapshot as Node).toRef();
    }
    if (_snapshot === null) {
      _snapshot = ACTIVE_SNAPSHOT.get();
      if (_snapshot === null) {
        throw new Error(`no active Snapshot for EditEvent`);
      }
      _snapshot = _snapshot.toRef();
    }
    if (_snapshot === null) {
      throw new Error(`EditEvent.snapshot is required`);
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
      throw new Error(`EditEvent.status is required`);
    }
    this.status = _status;
    let _type = options.type;
    if (_type === null) {
      throw new Error(`EditEvent.type is required`);
    }
    this.type = _type;
    let _node = options.node;
    if (_node != null && _node.constructor.name != "NodeReference") {
      _node = (_node as Node).toRef();
    }
    if (_node === null) {
      throw new Error(`EditEvent.node is required`);
    }
    this.nodePtr = _node as NodeReference;
    let _operation = options.operation ?? null;
    this.operation = _operation;
    let _propertyId = options.propertyId ?? null;
    this.propertyId = _propertyId;
    let _customProperty = options.customProperty ?? null;
    if (_customProperty != null && _customProperty.constructor.name != "NodeReference") {
      _customProperty = (_customProperty as Node).toRef();
    }
    this.customPropertyPtr = _customProperty as NodeReference | null;
    let _key = options.key ?? null;
    this.key = _key;
    let _value = options.value ?? null;
    this.value = _value;
    let _reverseOperation = options.reverseOperation ?? null;
    this.reverseOperation = _reverseOperation;
    let _reverseValue = options.reverseValue ?? null;
    this.reverseValue = _reverseValue;

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
        throw new Error(`EditEvent.createdAt is required for existing Events`);
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
    if (!(this.type === other.type)) {
      return false;
    }
    if (!(this.nodePtr.id === other.nodePtr.id)) {
      return false;
    }
    if (!(this.operation === other.operation)) {
      return false;
    }
    if (!(this.propertyId === other.propertyId)) {
      return false;
    }
    if (!(this.customPropertyPtr?.id === other.customPropertyPtr?.id)) {
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
    h = (h * 31 + this.type) & 0xffffffff;
    h = (h * 31 + hashString(this.nodePtr.id)) & 0xffffffff;
    if (this.operation != null) {
      h = (h * 31 + this.operation) & 0xffffffff;
    }
    if (this.propertyId != null) {
      h = (h * 31 + hashInt(this.propertyId)) & 0xffffffff;
    }
    if (this.customPropertyPtr != null) {
      h = (h * 31 + hashString(this.customPropertyPtr.id)) & 0xffffffff;
    }
    if (this.key != null) {
      h = (h * 31 + this.key.hash()) & 0xffffffff;
    }
    if (this.value != null) {
      h = (h * 31 + this.value.hash()) & 0xffffffff;
    }
    if (this.reverseOperation != null) {
      h = (h * 31 + this.reverseOperation) & 0xffffffff;
    }
    if (this.reverseValue != null) {
      h = (h * 31 + this.reverseValue.hash()) & 0xffffffff;
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
      type: NodeType.EDIT_EVENT,
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
    return `EditEvent[id=${this.id}]`;
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
    propertyReprs.push(`type=${EditType[this.type]}`);
    propertyReprs.push(`node=${this.node?.repr()}`);
    if (this.operation != null) {
      propertyReprs.push(`operation=${EditOperation[this.operation]}`);
    }
    if (this.propertyId != null) {
      propertyReprs.push(`propertyId=${this.propertyId}`);
    }
    if (this.customProperty != null) {
      propertyReprs.push(`customProperty=${this.customProperty?.repr()}`);
    }
    if (this.key != null) {
      propertyReprs.push(`key=${this.key.repr()}`);
    }
    propertyReprs.push(`createdEpoch=${this.createdEpoch}`);
    propertyReprs.push(`status=${EventStatus[this.status]}`);
    return `<EditEvent "${this.path}" ${propertyReprs.join(" ")}>`;
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerNodeClass(NodeType.EDIT_EVENT, EditEvent);
/* ==== DESTACK_GENERATED_END:NODE:90100 ==== */
