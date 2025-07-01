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
  QueryConnection,
  Session,
  Supergraph,
} from "@destack/language/core";
import {
  Entity,
  EnumType,
  Node,
  NodeReference,
  NodeType,
  StructType,
} from "@destack/language/core";
import { registerEnumClass, registerNodeClass } from "@destack/language/registry";
import type { Run } from "@destack/language/runtime/run";
import type { SpanEvent } from "@destack/language/runtime/span";
import type { Message } from "@destack/language/social";
import type { Space } from "@destack/language/space";
import {
  InterruptionProto,
  InterruptionResponseProto,
  InterruptionStatusProto,
  InterruptionTypeProto,
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
      _status = InterruptionStatus.OPEN;
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
    return new NodeReference({
      nodeType: NodeType.INTERRUPTION,
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
    objectValue["15"] = object.createdAt.toString({ timeZoneName: "never" });
    if (object.createdByPtr != null) {
      objectValue["16"] = object.createdByPtr.toValue();
    }
    objectValue["17"] = object.updatedAt.toString({ timeZoneName: "never" });
    if (object.updatedByPtr != null) {
      objectValue["18"] = object.updatedByPtr.toValue();
    }
    objectValue["30"] = object.type;
    if (object.runnablePtr != null) {
      objectValue["32"] = object.runnablePtr.toValue();
    }
    if (object.spanPtr != null) {
      objectValue["37"] = object.spanPtr.toValue();
    }
    objectValue["40"] = object.status;
    if (object.duration != null) {
      objectValue["41"] = timedeltaToISOFormat(object.duration);
    }
    if (object.closedAt != null) {
      objectValue["42"] = object.closedAt.toString({ timeZoneName: "never" });
    }
    if (object.response != null) {
      objectValue["54"] = object.response;
    }
    if (object.messagePtr != null) {
      objectValue["55"] = object.messagePtr.toValue();
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
    const parentPtrValue = objectValue["3"];
    const unpackedParentPtr =
      parentPtrValue != undefined
        ? NodeReference.fromValue(parentPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const runnablePtrValue = objectValue["32"];
    const unpackedRunnablePtr =
      runnablePtrValue != undefined
        ? NodeReference.fromValue(runnablePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const spanPtrValue = objectValue["37"];
    const unpackedSpanPtr =
      spanPtrValue != undefined
        ? NodeReference.fromValue(spanPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const durationValue = objectValue["41"];
    const unpackedDuration =
      durationValue != undefined ? timedeltaFromISOFormat(durationValue) : null;
    const closedAtValue = objectValue["42"];
    const unpackedClosedAt =
      closedAtValue != undefined
        ? Temporal.Instant.from(closedAtValue).toZonedDateTimeISO("UTC")
        : null;
    const responseValue = objectValue["54"];
    const unpackedResponse = responseValue != undefined ? Number(responseValue) : null;
    const messagePtrValue = objectValue["55"];
    const unpackedMessagePtr =
      messagePtrValue != undefined
        ? NodeReference.fromValue(messagePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const spacePtrValue = objectValue["5"];
    const unpackedSpacePtr =
      spacePtrValue != undefined
        ? NodeReference.fromValue(spacePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const createdByPtrValue = objectValue["16"];
    const unpackedCreatedByPtr =
      createdByPtrValue != undefined
        ? NodeReference.fromValue(createdByPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const updatedByPtrValue = objectValue["18"];
    const unpackedUpdatedByPtr =
      updatedByPtrValue != undefined
        ? NodeReference.fromValue(updatedByPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    return new Interruption({
      parent: unpackedParentPtr,
      type: Number(objectValue["30"]),
      runnable: unpackedRunnablePtr,
      span: unpackedSpanPtr,
      status: Number(objectValue["40"]),
      duration: unpackedDuration,
      closedAt: unpackedClosedAt,
      response: unpackedResponse,
      message: unpackedMessagePtr,
      space: unpackedSpacePtr,
      createdAt: Temporal.Instant.from(objectValue["15"]).toZonedDateTimeISO("UTC"),
      createdBy: unpackedCreatedByPtr,
      updatedAt: Temporal.Instant.from(objectValue["17"]).toZonedDateTimeISO("UTC"),
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
    return new Interruption({
      parent:
        objectProto.parentPtr != undefined
          ? NodeReference.fromProto(
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
          ? NodeReference.fromProto(
              objectProto.runnablePtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      span:
        objectProto.spanPtr != undefined
          ? NodeReference.fromProto(
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
          ? NodeReference.fromProto(
              objectProto.messagePtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      space:
        objectProto.spacePtr != undefined
          ? NodeReference.fromProto(
              objectProto.spacePtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      createdAt: unpackProtoTimestamp(objectProto.createdAt!),
      createdBy:
        objectProto.createdByPtr != undefined
          ? NodeReference.fromProto(
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
          ? NodeReference.fromProto(
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
