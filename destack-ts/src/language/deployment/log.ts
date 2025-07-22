import {
  packProtoJson,
  packProtoTimestamp,
  unpackProtoJson,
  unpackProtoTimestamp,
} from "@destack/grpc";
import type {
  Branch,
  Graph,
  GraphConnection,
  IsActor,
  NodeReference,
  Session,
  Snapshot,
  Space,
  Supergraph,
} from "@destack/language/core";
import {
  ACTIVE_BRANCH,
  ACTIVE_SNAPSHOT,
  ACTIVE_SPACE,
  Entity,
  EnumType,
  Event,
  EventStatus,
  Node,
  NodeType,
  StructType,
} from "@destack/language/core";
import {
  STRUCT_CLASS_BY_TYPE,
  registerEnumClass,
  registerNodeClass,
} from "@destack/language/registry";
import type { Client } from "@destack/language/universe";
import { EventStatusProto, LogEventProto, LogLevelProto } from "@destack/proto";
import { base64Decode } from "@destack/utils";
import { hashInt, hashString } from "@destack/utils/hash";
import { Temporal } from "temporal-polyfill";

/* ==== DESTACK_GENERATED_START:ENUM:1110300 ==== */
/**
 * LogLevel
 */
export enum LogLevel {
  TRACE = 1,
  DEBUG = 2,
  INFO = 3,
  WARNING = 4,
  ERROR = 5,
  PANIC = 6,

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerEnumClass(EnumType.LOG_LEVEL, LogLevel);
/* ==== DESTACK_GENERATED_END:ENUM:1110300 ==== */

/* ==== DESTACK_GENERATED_START:NODE:1110011 ==== */
/**
 * A Log message.
 */
export class LogEvent extends Event {
  static metatype: NodeType = NodeType.LOG_EVENT;

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
   * The Node this Event is about.
   */
  get node(): Node | null {
    const nodePtr: NodeReference | null = this.nodePtr;
    if (nodePtr != null) {
      return this._graph.get(nodePtr.id) as Node | null;
    }
    return null;
  }
  readonly nodePtr: NodeReference | null;

  /**
   * LogEvent.content
   */
  readonly content: string;

  /**
   * LogEvent.attributes
   */
  readonly attributes: { readonly [key: string]: any };

  /**
   * LogEvent.level
   */
  readonly level: LogLevel;

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
    content: string;
    attributes?: { readonly [key: string]: any };
    level: LogLevel;
    _session?: Session | null;
    _graph?: Supergraph | null;
    _graph?: Graph | null;
    _connection?: GraphConnection | null;
  }) {
    super(
      // id
      options.id ?? null,
      // parent
      null,
      // session
      options._session ?? null,
      // supergraph
      options._graph ?? null,
      // graph
      options._graph ?? null,
      // connection
      options._connection ?? null,
      // _isNew
      options.id == null,
    );

    // properties
    let _space = options.space ?? null;
    if (_space != null && _space.constructor.name != "NodeReference") {
      _space = (_space as Node).toRef();
    }
    if (_space === null) {
      _space = ACTIVE_SPACE.get();
      if (_space === null) {
        throw new Error(`no active Space for LogEvent`);
      }
      _space = _space.toRef();
    }
    if (_space === null) {
      throw new Error(`LogEvent.space is required`);
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
        throw new Error(`no active Branch for LogEvent`);
      }
      _branch = _branch.toRef();
    }
    if (_branch === null) {
      throw new Error(`LogEvent.branch is required`);
    }
    this.branchPtr = _branch as NodeReference;
    let _snapshot = options.snapshot ?? null;
    if (_snapshot != null && _snapshot.constructor.name != "NodeReference") {
      _snapshot = (_snapshot as Node).toRef();
    }
    if (_snapshot === null) {
      _snapshot = ACTIVE_SNAPSHOT.get();
      if (_snapshot === null) {
        throw new Error(`no active Snapshot for LogEvent`);
      }
      _snapshot = _snapshot.toRef();
    }
    if (_snapshot === null) {
      throw new Error(`LogEvent.snapshot is required`);
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
      throw new Error(`LogEvent.status is required`);
    }
    this.status = _status;
    let _node = options.node ?? null;
    if (_node != null && _node.constructor.name != "NodeReference") {
      _node = (_node as Node).toRef();
    }
    this.nodePtr = _node as NodeReference | null;
    let _content = options.content;
    if (_content === null) {
      throw new Error(`LogEvent.content is required`);
    }
    this.content = _content;
    let _attributes = options.attributes ?? null;
    if (_attributes === null) {
      _attributes = {};
    }
    this.attributes = _attributes;
    let _level = options.level;
    if (_level === null) {
      throw new Error(`LogEvent.level is required`);
    }
    this.level = _level;

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
        throw new Error(`LogEvent.createdAt is required for existing Events`);
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
    if (!(this.content === other.content)) {
      return false;
    }
    if (JSON.stringify(this.attributes) !== JSON.stringify(other.attributes)) {
      return false;
    }
    if (!(this.level === other.level)) {
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
    if (!(this.nodePtr?.id === other.nodePtr?.id)) {
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
    h = (h * 31 + hashString(this.content)) & 0xffffffff;
    if (this.attributes && Object.keys(this.attributes).length > 0) {
      for (const [_key, _value] of Object.entries(this.attributes)) {
        h = (h * 31 + hashString(_key)) & 0xffffffff;
        h = (h * 31 + hashString(JSON.stringify(_value))) & 0xffffffff;
      }
    }
    h = (h * 31 + this.level) & 0xffffffff;
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
    if (this.nodePtr != null) {
      h = (h * 31 + hashString(this.nodePtr.id)) & 0xffffffff;
    }
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
      type: NodeType.LOG_EVENT,
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
    return `LogEvent[id=${this.id}]`;
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
    return `<LogEvent "${this.path}" ${propertyReprs.join(" ")}>`;
  }

  toCson(): { [key: string]: any } {
    return LogEvent.__packCson__(this);
  }

  static __packCson__(object: LogEvent): { [key: string]: any } {
    const objectCson: { [key: string]: any } = {};
    objectCson["1"] = 1110011;
    objectCson["2"] = String(object.id);
    objectCson["5"] = object.spacePtr.toCson();
    if (object.definitionPtr != null) {
      objectCson["11"] = object.definitionPtr.toCson();
    }
    objectCson["12"] = object.branchPtr.toCson();
    objectCson["13"] = object.snapshotPtr.toCson();
    if (object.precededByPtr != null) {
      objectCson["14"] = object.precededByPtr.toCson();
    }
    if (object.causedByPtr != null) {
      objectCson["15"] = object.causedByPtr.toCson();
    }
    objectCson["20"] = object.createdAt.toString({ timeZoneName: "never" });
    objectCson["21"] = object.createdEpoch;
    if (object.createdByPtr != null) {
      objectCson["22"] = object.createdByPtr.toCson();
    }
    if (object.clientPtr != null) {
      objectCson["23"] = object.clientPtr.toCson();
    }
    if (object.clientNonce != null) {
      objectCson["24"] = String(object.clientNonce);
    }
    objectCson["25"] = object.clientCreatedAt.toString({ timeZoneName: "never" });
    objectCson["26"] = object.clientEpoch;
    objectCson["40"] = object.status;
    if (object.nodePtr != null) {
      objectCson["101"] = object.nodePtr.toCson();
    }
    objectCson["110"] = object.content;
    if (Object.keys(object.attributes).length > 0) {
      const packedAttributes: { [key: string]: any } = {} as any;
      for (const [key, value] of Object.entries(object.attributes)) {
        packedAttributes[String(key)] = value;
      }
      objectCson["111"] = packedAttributes;
    }
    objectCson["112"] = object.level;
    return objectCson;
  }

  static __unpackCson__(
    objectCson: { [key: string]: any },
    _session?: Session | null,
    _graph?: Graph | null,
    _connection?: GraphConnection | null,
  ): LogEvent {
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    const unpackedAttributes = {} as any;
    if (objectCson["111"] != undefined) {
      for (const [key, value] of Object.entries(objectCson["111"])) {
        unpackedAttributes[key] = value as any;
      }
    }
    const definitionPtrValue = objectCson["11"];
    const unpackedDefinitionPtr =
      definitionPtrValue != undefined
        ? _NodeReference.fromCson(definitionPtrValue, _session, _graph, _connection)
        : null;
    const precededByPtrValue = objectCson["14"];
    const unpackedPrecededByPtr =
      precededByPtrValue != undefined
        ? _NodeReference.fromCson(precededByPtrValue, _session, _graph, _connection)
        : null;
    const causedByPtrValue = objectCson["15"];
    const unpackedCausedByPtr =
      causedByPtrValue != undefined
        ? _NodeReference.fromCson(causedByPtrValue, _session, _graph, _connection)
        : null;
    const createdByPtrValue = objectCson["22"];
    const unpackedCreatedByPtr =
      createdByPtrValue != undefined
        ? _NodeReference.fromCson(createdByPtrValue, _session, _graph, _connection)
        : null;
    const clientPtrValue = objectCson["23"];
    const unpackedClientPtr =
      clientPtrValue != undefined
        ? _NodeReference.fromCson(clientPtrValue, _session, _graph, _connection)
        : null;
    const clientNonceValue = objectCson["24"];
    const unpackedClientNonce = clientNonceValue != undefined ? String(clientNonceValue) : null;
    const nodePtrValue = objectCson["101"];
    const unpackedNodePtr =
      nodePtrValue != undefined
        ? _NodeReference.fromCson(nodePtrValue, _session, _graph, _connection)
        : null;
    return new LogEvent({
      content: objectCson["110"],
      attributes: unpackedAttributes,
      level: Number(objectCson["112"]),
      definition: unpackedDefinitionPtr,
      branch: _NodeReference.fromCson(objectCson["12"], _session, _graph, _connection),
      snapshot: _NodeReference.fromCson(objectCson["13"], _session, _graph, _connection),
      precededBy: unpackedPrecededByPtr,
      causedBy: unpackedCausedByPtr,
      createdAt: Temporal.Instant.from(objectCson["20"]).toZonedDateTimeISO("UTC"),
      createdEpoch: Number(objectCson["21"]),
      createdBy: unpackedCreatedByPtr,
      client: unpackedClientPtr,
      clientNonce: unpackedClientNonce,
      clientCreatedAt: Temporal.Instant.from(objectCson["25"]).toZonedDateTimeISO("UTC"),
      clientEpoch: Number(objectCson["26"]),
      status: Number(objectCson["40"]),
      node: unpackedNodePtr,
      id: String(objectCson["2"]),
      space: _NodeReference.fromCson(objectCson["5"], _session, _graph, _connection),
      _session,
      _graph,
      _connection,
    });
  }

  static fromCson(
    objectCson: { readonly [key: string]: any },
    _session?: Session | null,
    _graph?: Graph | null,
    _connection?: GraphConnection | null,
  ): LogEvent {
    return LogEvent.__unpackCson__(objectCson, _session, _graph, _connection);
  }

  toProto(): LogEventProto {
    return LogEvent.__packProto__(this);
  }

  static __packProto__(object: LogEvent): LogEventProto {
    const objectProto: Partial<LogEventProto> = { metatype: 1110011 };
    objectProto.id = String(object.id);
    objectProto.spacePtr = object.spacePtr.toProto();
    if (object.definitionPtr != null) {
      objectProto.definitionPtr = object.definitionPtr.toProto();
    }
    objectProto.branchPtr = object.branchPtr.toProto();
    objectProto.snapshotPtr = object.snapshotPtr.toProto();
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
    objectProto.status = Number(object.status) as EventStatusProto;
    if (object.nodePtr != null) {
      objectProto.nodePtr = object.nodePtr.toProto();
    }
    objectProto.content = object.content;
    if (object.attributes) {
      objectProto.attributes = {} as any;
      for (const [key, value] of Object.entries(object.attributes)) {
        objectProto.attributes![key] = packProtoJson(value);
      }
    }
    objectProto.level = Number(object.level) as LogLevelProto;
    return objectProto as LogEventProto;
  }

  static __unpackProto__(
    objectProto: LogEventProto,
    _session?: Session | null,
    _graph?: Supergraph | null,
    _connection?: GraphConnection | null,
  ): LogEvent {
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    const unpackedAttributes = {} as any;
    if (objectProto.attributes) {
      for (const [key, value] of Object.entries(objectProto.attributes)) {
        unpackedAttributes.set(key, unpackProtoJson((value as any)!));
      }
    }
    return new LogEvent({
      content: objectProto.content,
      attributes: unpackedAttributes,
      level: Number(objectProto.level) as LogLevel,
      definition:
        objectProto.definitionPtr != undefined
          ? _NodeReference.fromProto(
              objectProto.definitionPtr!,
              _session,
              _graph,
              _graph,
              _connection,
            )
          : null,
      branch: _NodeReference.fromProto(
        objectProto.branchPtr!,
        _session,
        _graph,
        _graph,
        _connection,
      ),
      snapshot: _NodeReference.fromProto(
        objectProto.snapshotPtr!,
        _session,
        _graph,
        _graph,
        _connection,
      ),
      precededBy:
        objectProto.precededByPtr != undefined
          ? _NodeReference.fromProto(
              objectProto.precededByPtr!,
              _session,
              _graph,
              _graph,
              _connection,
            )
          : null,
      causedBy:
        objectProto.causedByPtr != undefined
          ? _NodeReference.fromProto(
              objectProto.causedByPtr!,
              _session,
              _graph,
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
              _graph,
              _graph,
              _connection,
            )
          : null,
      client:
        objectProto.clientPtr != undefined
          ? _NodeReference.fromProto(objectProto.clientPtr!, _session, _graph, _graph, _connection)
          : null,
      clientNonce: objectProto.clientNonce != undefined ? String(objectProto.clientNonce) : null,
      clientCreatedAt: unpackProtoTimestamp(objectProto.clientCreatedAt!),
      clientEpoch: Number(objectProto.clientEpoch),
      status: Number(objectProto.status) as EventStatus,
      node:
        objectProto.nodePtr != undefined
          ? _NodeReference.fromProto(objectProto.nodePtr!, _session, _graph, _graph, _connection)
          : null,
      id: String(objectProto.id),
      space: _NodeReference.fromProto(objectProto.spacePtr!, _session, _graph, _graph, _connection),
      _session,
      _graph,
      _connection,
    });
  }

  static fromProto(
    objectProto: LogEventProto,
    _session?: Session | null,
    _graph?: Supergraph | null,
    _connection?: GraphConnection | null,
  ): LogEvent {
    return LogEvent.__unpackProto__(objectProto, _session, _graph, _connection);
  }

  static fromProtoString(packedProtoString: string): LogEvent {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = LogEventProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerNodeClass(NodeType.LOG_EVENT, LogEvent);
/* ==== DESTACK_GENERATED_END:NODE:1110011 ==== */
