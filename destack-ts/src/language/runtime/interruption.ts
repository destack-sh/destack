import {
  packProtoDuration,
  packProtoTimestamp,
  unpackProtoDuration,
  unpackProtoTimestamp,
} from "@destack/grpc";
import type {
  Graph,
  IsRunnable,
  IsSpatial,
  IsSubject,
  NodeReference,
  QueryConnection,
  Session,
  Snapshot,
  Supergraph,
} from "@destack/language/core";
import {
  Entity,
  EnumType,
  Materialization,
  Node,
  NodeType,
  StructType,
} from "@destack/language/core";
import {
  STRUCT_CLASS_BY_TYPE,
  registerEnumClass,
  registerNodeClass,
} from "@destack/language/registry";
import type { Run } from "@destack/language/runtime/run";
import type { SpanEvent } from "@destack/language/runtime/span";
import type { Message } from "@destack/language/social";
import type { Space } from "@destack/language/space";
import {
  InterruptionProto,
  InterruptionResponseProto,
  InterruptionStatusProto,
  InterruptionTypeProto,
  MaterializationProto,
} from "@destack/proto";
import { base64Decode, timedeltaFromISOFormat, timedeltaToISOFormat } from "@destack/utils";
import { hashFloat, hashString } from "@destack/utils/hash";
import { Temporal } from "temporal-polyfill";

/* ==== DESTACK_GENERATED_START:ENUM:90200 ==== */
/**
 * InterruptionType
 */
export enum InterruptionType {
  PAUSE = 10,
  YIELD = 20,
  WAIT = 30,

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerEnumClass(EnumType.INTERRUPTION_TYPE, InterruptionType);
/* ==== DESTACK_GENERATED_END:ENUM:90200 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:90201 ==== */
/**
 * InterruptionStatus
 */
export enum InterruptionStatus {
  OPEN = 10,
  CANCELLED = 30,
  COMPLETED = 33,

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerEnumClass(EnumType.INTERRUPTION_STATUS, InterruptionStatus);
/* ==== DESTACK_GENERATED_END:ENUM:90201 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:90202 ==== */
/**
 * InterruptionResponse
 */
export enum InterruptionResponse {
  ACCEPT = 10,
  REJECT = 20,

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerEnumClass(EnumType.INTERRUPTION_RESPONSE, InterruptionResponse);
/* ==== DESTACK_GENERATED_END:ENUM:90202 ==== */

/* ==== DESTACK_GENERATED_START:NODE:90400 ==== */
/**
 * An Interruption in run of something.
 */
export class Interruption extends Entity implements IsSpatial {
  static metatype: NodeType = NodeType.INTERRUPTION;

  /**
   * Interruption.parent
   */
  get parent(): Run | null {
    const nodePtr: NodeReference | null = this.parentPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Run | null;
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
   * Entity.materialization
   */
  readonly materialization: Materialization;

  /**
   * The Snapshot this Entity is part of.
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
   * The previous Entity this Entity is based on (from another Snapshot).
   */
  get predecessor(): Interruption | null {
    const nodePtr: NodeReference | null = this.predecessorPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Interruption | null;
    }
    return null;
  }
  readonly predecessorPtr: NodeReference | null;

  /**
   * The template this Entity instance is based on (from the template tree).
   */
  get template(): Interruption | null {
    const nodePtr: NodeReference | null = this.templatePtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Interruption | null;
    }
    return null;
  }
  readonly templatePtr: NodeReference | null;

  /**
   * The (root) Entity in this Entity's instance tree (not the template tree).
   */
  get instanceRoot(): Entity | null {
    const nodePtr: NodeReference | null = this.instanceRootPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Entity | null;
    }
    return null;
  }
  readonly instanceRootPtr: NodeReference | null;

  /**
   * Entity.createdAt
   */
  readonly createdAt: Temporal.ZonedDateTime;

  /**
   * Entity.createdBy
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
   * Entity.updatedAt
   */
  readonly updatedAt: Temporal.ZonedDateTime;

  /**
   * Entity.updatedBy
   */
  get updatedBy(): (Node & IsSubject) | null {
    const nodePtr: NodeReference | null = this.updatedByPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as (Node & IsSubject) | null;
    }
    return null;
  }
  readonly updatedByPtr: NodeReference | null;

  /**
   * Interruption.type
   */
  type: InterruptionType;

  /**
   * Interruption.runnable
   */
  get runnable(): (Node & IsRunnable) | null {
    const nodePtr: NodeReference | null = this.runnablePtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as (Node & IsRunnable) | null;
    }
    return null;
  }
  set runnable(node: (Node & IsRunnable) | null) {
    if (node === null) {
      this.runnablePtr = null;
    } else {
      this.runnablePtr = node.toRef();
    }
  }
  runnablePtr: NodeReference | null;

  /**
   * Interruption.span
   */
  get span(): SpanEvent | null {
    const nodePtr: NodeReference | null = this.spanPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as SpanEvent | null;
    }
    return null;
  }
  set span(node: SpanEvent | null) {
    if (node === null) {
      this.spanPtr = null;
    } else {
      this.spanPtr = node.toRef();
    }
  }
  spanPtr: NodeReference | null;

  /**
   * Interruption.status
   */
  status: InterruptionStatus;

  /**
   * Interruption.duration
   */
  duration: Temporal.Duration | null;

  /**
   * Interruption.closedAt
   */
  closedAt: Temporal.ZonedDateTime | null;

  /**
   * Interruption.response
   */
  response: InterruptionResponse | null;

  /**
   * The Message that was created for this Interruption.
   */
  get message(): Message | null {
    const nodePtr: NodeReference | null = this.messagePtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Message | null;
    }
    return null;
  }
  set message(node: Message | null) {
    if (node === null) {
      this.messagePtr = null;
    } else {
      this.messagePtr = node.toRef();
    }
  }
  messagePtr: NodeReference | null;

  constructor(options: {
    id?: string;
    parent?: Run | NodeReference | null;
    space?: Space | NodeReference | null;
    materialization?: Materialization;
    snapshot?: Snapshot | NodeReference | null;
    predecessor?: Interruption | NodeReference | null;
    template?: Interruption | NodeReference | null;
    instanceRoot?: Entity | NodeReference | null;
    createdAt?: Temporal.ZonedDateTime;
    createdBy?: (Node & IsSubject) | NodeReference | null;
    updatedAt?: Temporal.ZonedDateTime;
    updatedBy?: (Node & IsSubject) | NodeReference | null;
    type: InterruptionType;
    runnable?: (Node & IsRunnable) | NodeReference | null;
    span?: SpanEvent | NodeReference | null;
    status?: InterruptionStatus;
    duration?: Temporal.Duration | null;
    closedAt?: Temporal.ZonedDateTime | null;
    response?: InterruptionResponse | null;
    message?: Message | NodeReference | null;
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
    let _materialization = options.materialization ?? null;
    if (_materialization === null) {
      _materialization = 3 /* Materialization.FULL_GRAPH */;
    }
    if (_materialization === null) {
      throw new Error(`Interruption.materialization is required`);
    }
    this.materialization = _materialization;
    let _snapshot = options.snapshot ?? null;
    if (_snapshot != null && _snapshot.metatype != StructType.NODE_REFERENCE) {
      _snapshot = (_snapshot as Node).toRef();
    }
    this.snapshotPtr = _snapshot;
    let _predecessor = options.predecessor ?? null;
    if (_predecessor != null && _predecessor.metatype != StructType.NODE_REFERENCE) {
      _predecessor = (_predecessor as Node).toRef();
    }
    this.predecessorPtr = _predecessor;
    let _template = options.template ?? null;
    if (_template != null && _template.metatype != StructType.NODE_REFERENCE) {
      _template = (_template as Node).toRef();
    }
    this.templatePtr = _template;
    let _instanceRoot = options.instanceRoot ?? null;
    if (_instanceRoot != null && _instanceRoot.metatype != StructType.NODE_REFERENCE) {
      _instanceRoot = (_instanceRoot as Node).toRef();
    }
    this.instanceRootPtr = _instanceRoot;
    let _type = options.type;
    if (_type === null) {
      throw new Error(`Interruption.type is required`);
    }
    this.type = _type;
    let _runnable = options.runnable ?? null;
    if (_runnable != null && _runnable.metatype != StructType.NODE_REFERENCE) {
      _runnable = (_runnable as Node).toRef();
    }
    this.runnablePtr = _runnable;
    let _span = options.span ?? null;
    if (_span != null && _span.metatype != StructType.NODE_REFERENCE) {
      _span = (_span as Node).toRef();
    }
    this.spanPtr = _span;
    let _status = options.status ?? null;
    if (_status === null) {
      _status = 10 /* InterruptionStatus.OPEN */;
    }
    if (_status === null) {
      throw new Error(`Interruption.status is required`);
    }
    this.status = _status;
    let _duration = options.duration ?? null;
    this.duration = _duration;
    let _closedAt = options.closedAt ?? null;
    this.closedAt = _closedAt;
    let _response = options.response ?? null;
    this.response = _response;
    let _message = options.message ?? null;
    if (_message != null && _message.metatype != StructType.NODE_REFERENCE) {
      _message = (_message as Node).toRef();
    }
    this.messagePtr = _message;

    // identity
    if (options.id == null) {
      const now = Temporal.Now.zonedDateTimeISO("UTC");
      this.createdAt = now;
      this.createdByPtr = null;
      this.updatedAt = now;
      this.updatedByPtr = null;
    } else {
      if (options.createdAt == null || options.updatedAt == null) {
        throw new Error(
          `{cls.__name__}.createdAt and {cls.__name__}.updatedAt are required for existing Nodes`,
        );
      }
      this.createdAt = options.createdAt;
      this.createdByPtr =
        options.createdBy != null
          ? options.createdBy.metatype == StructType.NODE_REFERENCE
            ? (options.createdBy as NodeReference)
            : (options.createdBy as Node).toRef()
          : null;
      this.updatedAt = options.updatedAt;
      this.updatedByPtr =
        options.updatedBy != null
          ? options.updatedBy.metatype == StructType.NODE_REFERENCE
            ? (options.updatedBy as NodeReference)
            : (options.updatedBy as Node).toRef()
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
    if (!(this.runnablePtr?.id === other.runnablePtr?.id)) {
      return false;
    }
    if (!(this.spanPtr?.id === other.spanPtr?.id)) {
      return false;
    }
    if (!(this.status === other.status)) {
      return false;
    }
    if (!(this.duration === other.duration)) {
      return false;
    }
    if (!(this.closedAt === other.closedAt)) {
      return false;
    }
    if (!(this.response === other.response)) {
      return false;
    }
    if (!(this.messagePtr?.id === other.messagePtr?.id)) {
      return false;
    }
    if (!(this.spacePtr?.id === other.spacePtr?.id)) {
      return false;
    }
    if (!(this.snapshotPtr?.id === other.snapshotPtr?.id)) {
      return false;
    }
    if (!(this.predecessorPtr?.id === other.predecessorPtr?.id)) {
      return false;
    }
    if (!(this.templatePtr?.id === other.templatePtr?.id)) {
      return false;
    }
    if (!(this.instanceRootPtr?.id === other.instanceRootPtr?.id)) {
      return false;
    }
    return true;
  }

  hash(): number {
    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    if (this.parentPtr !== null) {
      h = (h * 31 + hashString(this.parentPtr.id)) & 0xffffffff;
    }
    h = (h * 31 + this.type) & 0xffffffff;
    if (this.runnablePtr !== null) {
      h = (h * 31 + hashString(this.runnablePtr.id)) & 0xffffffff;
    }
    if (this.spanPtr !== null) {
      h = (h * 31 + hashString(this.spanPtr.id)) & 0xffffffff;
    }
    h = (h * 31 + this.status) & 0xffffffff;
    if (this.duration !== null) {
      h = (h * 31 + hashFloat(this.duration.total("seconds"))) & 0xffffffff;
    }
    if (this.closedAt !== null) {
      h = (h * 31 + hashString(this.closedAt.toString({ timeZoneName: "never" }))) & 0xffffffff;
    }
    if (this.response !== null) {
      h = (h * 31 + this.response) & 0xffffffff;
    }
    if (this.messagePtr !== null) {
      h = (h * 31 + hashString(this.messagePtr.id)) & 0xffffffff;
    }
    if (this.spacePtr !== null) {
      h = (h * 31 + hashString(this.spacePtr.id)) & 0xffffffff;
    }
    if (this.snapshotPtr !== null) {
      h = (h * 31 + hashString(this.snapshotPtr.id)) & 0xffffffff;
    }
    if (this.predecessorPtr !== null) {
      h = (h * 31 + hashString(this.predecessorPtr.id)) & 0xffffffff;
    }
    if (this.templatePtr !== null) {
      h = (h * 31 + hashString(this.templatePtr.id)) & 0xffffffff;
    }
    if (this.instanceRootPtr !== null) {
      h = (h * 31 + hashString(this.instanceRootPtr.id)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.createdAt.toString({ timeZoneName: "never" }))) & 0xffffffff;
    if (this.createdByPtr !== null) {
      h = (h * 31 + hashString(this.createdByPtr.id)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.updatedAt.toString({ timeZoneName: "never" }))) & 0xffffffff;
    if (this.updatedByPtr !== null) {
      h = (h * 31 + hashString(this.updatedByPtr.id)) & 0xffffffff;
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
      type: NodeType.INTERRUPTION,
      id: this.id,
      spaceId: this.spacePtr?.id ?? null,
      _session: this._session,
      _supergraph: this._supergraph,
    });
  }

  get _pathKey(): string {
    return "Interruption[id={this.id}]";
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
    return `<Interruption '${this.path}'>`;
  }

  toValue(): { [key: string]: any } {
    return Interruption.__packValue__(this);
  }

  static __packValue__(object: Interruption): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 90400;
    objectValue["2"] = String(object.id);
    if (object.parentPtr != null) {
      objectValue["3"] = object.parentPtr.toValue();
    }
    if (object.spacePtr != null) {
      objectValue["5"] = object.spacePtr.toValue();
    }
    objectValue["10"] = object.materialization;
    if (object.snapshotPtr != null) {
      objectValue["11"] = object.snapshotPtr.toValue();
    }
    if (object.predecessorPtr != null) {
      objectValue["12"] = object.predecessorPtr.toValue();
    }
    if (object.templatePtr != null) {
      objectValue["13"] = object.templatePtr.toValue();
    }
    if (object.instanceRootPtr != null) {
      objectValue["14"] = object.instanceRootPtr.toValue();
    }
    objectValue["20"] = object.createdAt.toString({ timeZoneName: "never" });
    if (object.createdByPtr != null) {
      objectValue["21"] = object.createdByPtr.toValue();
    }
    objectValue["22"] = object.updatedAt.toString({ timeZoneName: "never" });
    if (object.updatedByPtr != null) {
      objectValue["23"] = object.updatedByPtr.toValue();
    }
    objectValue["100"] = object.type;
    if (object.runnablePtr != null) {
      objectValue["110"] = object.runnablePtr.toValue();
    }
    if (object.spanPtr != null) {
      objectValue["111"] = object.spanPtr.toValue();
    }
    objectValue["120"] = object.status;
    if (object.duration != null) {
      objectValue["121"] = timedeltaToISOFormat(object.duration);
    }
    if (object.closedAt != null) {
      objectValue["122"] = object.closedAt.toString({ timeZoneName: "never" });
    }
    if (object.response != null) {
      objectValue["130"] = object.response;
    }
    if (object.messagePtr != null) {
      objectValue["131"] = object.messagePtr.toValue();
    }
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Interruption {
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    const parentPtrValue = objectValue["3"];
    const unpackedParentPtr =
      parentPtrValue != undefined
        ? _NodeReference.fromValue(parentPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const runnablePtrValue = objectValue["110"];
    const unpackedRunnablePtr =
      runnablePtrValue != undefined
        ? _NodeReference.fromValue(runnablePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const spanPtrValue = objectValue["111"];
    const unpackedSpanPtr =
      spanPtrValue != undefined
        ? _NodeReference.fromValue(spanPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const durationValue = objectValue["121"];
    const unpackedDuration =
      durationValue != undefined ? timedeltaFromISOFormat(durationValue) : null;
    const closedAtValue = objectValue["122"];
    const unpackedClosedAt =
      closedAtValue != undefined
        ? Temporal.Instant.from(closedAtValue).toZonedDateTimeISO("UTC")
        : null;
    const responseValue = objectValue["130"];
    const unpackedResponse = responseValue != undefined ? Number(responseValue) : null;
    const messagePtrValue = objectValue["131"];
    const unpackedMessagePtr =
      messagePtrValue != undefined
        ? _NodeReference.fromValue(messagePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const spacePtrValue = objectValue["5"];
    const unpackedSpacePtr =
      spacePtrValue != undefined
        ? _NodeReference.fromValue(spacePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const snapshotPtrValue = objectValue["11"];
    const unpackedSnapshotPtr =
      snapshotPtrValue != undefined
        ? _NodeReference.fromValue(snapshotPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const predecessorPtrValue = objectValue["12"];
    const unpackedPredecessorPtr =
      predecessorPtrValue != undefined
        ? _NodeReference.fromValue(predecessorPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const templatePtrValue = objectValue["13"];
    const unpackedTemplatePtr =
      templatePtrValue != undefined
        ? _NodeReference.fromValue(templatePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const instanceRootPtrValue = objectValue["14"];
    const unpackedInstanceRootPtr =
      instanceRootPtrValue != undefined
        ? _NodeReference.fromValue(instanceRootPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const createdByPtrValue = objectValue["21"];
    const unpackedCreatedByPtr =
      createdByPtrValue != undefined
        ? _NodeReference.fromValue(createdByPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const updatedByPtrValue = objectValue["23"];
    const unpackedUpdatedByPtr =
      updatedByPtrValue != undefined
        ? _NodeReference.fromValue(updatedByPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    return new Interruption({
      parent: unpackedParentPtr,
      type: Number(objectValue["100"]),
      runnable: unpackedRunnablePtr,
      span: unpackedSpanPtr,
      status: Number(objectValue["120"]),
      duration: unpackedDuration,
      closedAt: unpackedClosedAt,
      response: unpackedResponse,
      message: unpackedMessagePtr,
      space: unpackedSpacePtr,
      materialization: Number(objectValue["10"]),
      snapshot: unpackedSnapshotPtr,
      predecessor: unpackedPredecessorPtr,
      template: unpackedTemplatePtr,
      instanceRoot: unpackedInstanceRootPtr,
      createdAt: Temporal.Instant.from(objectValue["20"]).toZonedDateTimeISO("UTC"),
      createdBy: unpackedCreatedByPtr,
      updatedAt: Temporal.Instant.from(objectValue["22"]).toZonedDateTimeISO("UTC"),
      updatedBy: unpackedUpdatedByPtr,
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
  ): Interruption {
    return Interruption.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
  }

  toProto(): InterruptionProto {
    return Interruption.__packProto__(this);
  }

  static __packProto__(object: Interruption): InterruptionProto {
    const objectProto: Partial<InterruptionProto> = { metatype: 90400 };
    objectProto.id = String(object.id);
    if (object.parentPtr != null) {
      objectProto.parentPtr = object.parentPtr.toProto();
    }
    if (object.spacePtr != null) {
      objectProto.spacePtr = object.spacePtr.toProto();
    }
    objectProto.materialization = Number(object.materialization) as MaterializationProto;
    if (object.snapshotPtr != null) {
      objectProto.snapshotPtr = object.snapshotPtr.toProto();
    }
    if (object.predecessorPtr != null) {
      objectProto.predecessorPtr = object.predecessorPtr.toProto();
    }
    if (object.templatePtr != null) {
      objectProto.templatePtr = object.templatePtr.toProto();
    }
    if (object.instanceRootPtr != null) {
      objectProto.instanceRootPtr = object.instanceRootPtr.toProto();
    }
    objectProto.createdAt = packProtoTimestamp(object.createdAt);
    if (object.createdByPtr != null) {
      objectProto.createdByPtr = object.createdByPtr.toProto();
    }
    objectProto.updatedAt = packProtoTimestamp(object.updatedAt);
    if (object.updatedByPtr != null) {
      objectProto.updatedByPtr = object.updatedByPtr.toProto();
    }
    objectProto.type = Number(object.type) as InterruptionTypeProto;
    if (object.runnablePtr != null) {
      objectProto.runnablePtr = object.runnablePtr.toProto();
    }
    if (object.spanPtr != null) {
      objectProto.spanPtr = object.spanPtr.toProto();
    }
    objectProto.status = Number(object.status) as InterruptionStatusProto;
    if (object.duration != null) {
      objectProto.duration = packProtoDuration(object.duration);
    }
    if (object.closedAt != null) {
      objectProto.closedAt = packProtoTimestamp(object.closedAt);
    }
    if (object.response != null) {
      objectProto.response = Number(object.response) as InterruptionResponseProto;
    }
    if (object.messagePtr != null) {
      objectProto.messagePtr = object.messagePtr.toProto();
    }
    return objectProto as InterruptionProto;
  }

  static __unpackProto__(
    objectProto: InterruptionProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Interruption {
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    return new Interruption({
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
      type: Number(objectProto.type) as InterruptionType,
      runnable:
        objectProto.runnablePtr != undefined
          ? _NodeReference.fromProto(
              objectProto.runnablePtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      span:
        objectProto.spanPtr != undefined
          ? _NodeReference.fromProto(
              objectProto.spanPtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      status: Number(objectProto.status) as InterruptionStatus,
      duration:
        objectProto.duration != undefined ? unpackProtoDuration(objectProto.duration!) : null,
      closedAt:
        objectProto.closedAt != undefined ? unpackProtoTimestamp(objectProto.closedAt!) : null,
      response:
        objectProto.response != undefined
          ? (Number(objectProto.response) as InterruptionResponse)
          : null,
      message:
        objectProto.messagePtr != undefined
          ? _NodeReference.fromProto(
              objectProto.messagePtr!,
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
      materialization: Number(objectProto.materialization) as Materialization,
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
      predecessor:
        objectProto.predecessorPtr != undefined
          ? _NodeReference.fromProto(
              objectProto.predecessorPtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      template:
        objectProto.templatePtr != undefined
          ? _NodeReference.fromProto(
              objectProto.templatePtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      instanceRoot:
        objectProto.instanceRootPtr != undefined
          ? _NodeReference.fromProto(
              objectProto.instanceRootPtr!,
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
      updatedAt: unpackProtoTimestamp(objectProto.updatedAt!),
      updatedBy:
        objectProto.updatedByPtr != undefined
          ? _NodeReference.fromProto(
              objectProto.updatedByPtr!,
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
    objectProto: InterruptionProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Interruption {
    return Interruption.__unpackProto__(objectProto, _session, _supergraph, _graph, _connection);
  }

  static fromProtoString(packedProtoString: string): Interruption {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = InterruptionProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerNodeClass(NodeType.INTERRUPTION, Interruption);
/* ==== DESTACK_GENERATED_END:NODE:90400 ==== */
