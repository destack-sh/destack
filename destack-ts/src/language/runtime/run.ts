import { packProtoDuration, packProtoTimestamp, unpackProtoDuration, unpackProtoTimestamp } from "@destack/grpc";
import {
  Analytic,
  EnumType,
  Event,
  Graph,
  Indexed,
  IsExtensible,
  IsRunnable,
  IsSubject,
  Node,
  NodeReference,
  NodeType,
  Particle,
  QueryConnection,
  Session,
  Spatial,
  StructType,
  Supergraph,
  TraitType,
  Value,
} from "@destack/language/core";
import { registerEnumClass, registerNodeClass } from "@destack/language/registry";
import { Interruption } from "@destack/language/runtime";
import { Space } from "@destack/language/space";
import { RunEventProto, RunEventTypeProto, RunProto, RunStatusProto } from "@destack/proto";
import { timedeltaFromISOFormat, timedeltaToISOFormat } from "@destack/utils";
import { Temporal } from "temporal-polyfill";

/* ==== DESTACK_GENERATED_START:ENUM:4000 ==== */
/**
 * RunStatus
 */
export enum RunStatus {
  SCHEDULED = 2,
  RUNNING = 10,
  PAUSED = 21,
  YIELDED = 23,
  CANGALAXYED = 51,
  ABORTED = 52,
  FAILED = 53,
  COMPLETED = 54,

  /* ==== DESTACK_CUSTOM_START ==== */

  // ...

  /* ==== DESTACK_CUSTOM_END ==== */
}
registerEnumClass(EnumType.RUN_STATUS, RunStatus);
/* ==== DESTACK_GENERATED_END:ENUM:4000 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:4001 ==== */
/**
 * RunEventType
 */
export enum RunEventType {
  SCHEDULED = 1,
  RUNNING = 10,
  REQUESTED_PAUSE = 20,
  PAUSED = 21,
  REQUESTED_RESUME = 22,
  RESUMED = 23,
  REQUESTED_CANCEL = 50,
  CANGALAXYED = 51,
  ABORTED = 52,
  FAILED = 53,
  COMPLETED = 54,

  /* ==== DESTACK_CUSTOM_START ==== */

  // ...

  /* ==== DESTACK_CUSTOM_END ==== */
}
registerEnumClass(EnumType.RUN_EVENT_TYPE, RunEventType);
/* ==== DESTACK_GENERATED_END:ENUM:4001 ==== */

/* ==== DESTACK_GENERATED_START:NODE:4001 ==== */
/**
 * A Event regarding a Run.
 */
export class RunEvent extends Node implements Event {
  static metatype: NodeType = NodeType.RUN_EVENT;
  static __traits__: TraitType[] = [
    TraitType.SPATIAL,
    TraitType.PARTICLE,
    TraitType.ANALYTIC,
    TraitType.INDEXED,
    TraitType.FROZEN,
    TraitType.TRACKED,
    TraitType.EVENT,
  ];
  static __rootType__: NodeType | null = NodeType.SPACE;
  static __parentTypes__: NodeType[] = [NodeType.SPACE];
  static __childTypes__: NodeType[] = [];
  static __ancestorTypes__: NodeType[] = [NodeType.SPACE];
  static __descendantTypes__: NodeType[] = [];

  /**
   * Spatial.parent
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
   * IsTracked.createdAt
   */
  readonly createdAt: Temporal.ZonedDateTime;

  /**
   * IsTracked.createdBy
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
   * IsTracked.updatedAt
   */
  readonly updatedAt: Temporal.ZonedDateTime;

  /**
   * IsTracked.updatedBy
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
   * RunEvent.type
   */
  type: RunEventType;

  /**
   * RunEvent.node
   */
  get node(): Run | null {
    const nodePtr: NodeReference | null = this.nodePtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Run | null;
    }
    return null;
  }
  set node(node: Run) {
    this.nodePtr = node.toRef();
  }
  nodePtr: NodeReference;

  /**
   * RunEvent.target
   */
  get target(): (Node & IsRunnable) | null {
    const nodePtr: NodeReference | null = this.targetPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as (Node & IsRunnable) | null;
    }
    return null;
  }
  set target(node: (Node & IsRunnable) | null) {
    if (node === null) {
      this.targetPtr = null;
    } else {
      this.targetPtr = node.toRef();
    }
  }
  targetPtr: NodeReference | null;

  constructor(options: {
    id?: string;
    parent?: Space | NodeReference | null;
    space?: Space | NodeReference | null;
    createdAt?: Temporal.ZonedDateTime;
    createdBy?: (Node & IsSubject) | NodeReference | null;
    updatedAt?: Temporal.ZonedDateTime;
    updatedBy?: (Node & IsSubject) | NodeReference | null;
    type: RunEventType;
    node: Run | NodeReference;
    target?: (Node & IsRunnable) | NodeReference | null;
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
    if (_parent != null && _parent instanceof Node) {
      _parent = _parent.toRef();
    }
    this.parentPtr = _parent;
    let _space = options.space ?? null;
    if (_space != null && _space instanceof Node) {
      _space = _space.toRef();
    }
    this.spacePtr = _space;
    let _type = options.type;
    if (_type === null) {
      throw new Error(`RunEvent.type is required`);
    }
    this.type = _type;
    let _node = options.node;
    if (_node != null && _node instanceof Node) {
      _node = _node.toRef();
    }
    if (_node === null) {
      throw new Error(`RunEvent.node is required`);
    }
    this.nodePtr = _node;
    let _target = options.target ?? null;
    if (_target != null && _target instanceof Node) {
      _target = _target.toRef();
    }
    this.targetPtr = _target;

    // identity
    if (options.id == null) {
      const now = Temporal.Now.zonedDateTimeISO();
      this.createdAt = now;
      this.createdByPtr = null;
      this.updatedAt = now;
      this.updatedByPtr = null;
    } else {
      if (options.createdAt == null || options.updatedAt == null) {
        throw new Error(`{cls.__name__}.createdAt and {cls.__name__}.updatedAt are required for existing Nodes`);
      }
      this.createdAt = options.createdAt;
      this.createdByPtr =
        options.createdBy != null
          ? options.createdBy instanceof Node
            ? options.createdBy.toRef()
            : options.createdBy
          : null;
      this.updatedAt = options.updatedAt;
      this.updatedByPtr =
        options.updatedBy != null
          ? options.updatedBy instanceof Node
            ? options.updatedBy.toRef()
            : options.updatedBy
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
    if (
      (this.targetPtr == null) !== (other.targetPtr == null) ||
      (this.targetPtr != null && !(this.targetPtr.id === other.targetPtr.id))
    ) {
      return false;
    }
    if (
      (this.spacePtr == null) !== (other.spacePtr == null) ||
      (this.spacePtr != null && !(this.spacePtr.id === other.spacePtr.id))
    ) {
      return false;
    }
    return true;
  }

  hash(): number {
    throw new Error("not implemented");
  }

  validate(): void {
    throw new Error("not implemented");
  }

  __toRef__(): NodeReference {
    return new NodeReference({
      nodeType: NodeType.RUN_EVENT,
      id: this.id,
      spaceId: this.spacePtr?.id ?? null,
      _session: this._session,
      _supergraph: this._supergraph,
    });
  }

  get _pathKey(): string {
    return "RunEvent[id={this.id}]";
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

  toValue(): { [key: string]: any } {
    return RunEvent.__packValue__(this);
  }

  static __packValue__(object: RunEvent): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 4001;
    objectValue["2"] = String(object.id);
    if (object.parentPtr != null) {
      objectValue["3"] = object.parentPtr.toValue();
    }
    if (object.spacePtr != null) {
      objectValue["5"] = object.spacePtr.toValue();
    }
    objectValue["15"] = object.createdAt.toString();
    if (object.createdByPtr != null) {
      objectValue["16"] = object.createdByPtr.toValue();
    }
    objectValue["17"] = object.updatedAt.toString();
    if (object.updatedByPtr != null) {
      objectValue["18"] = object.updatedByPtr.toValue();
    }
    objectValue["30"] = object.type;
    objectValue["35"] = object.nodePtr.toValue();
    if (object.targetPtr != null) {
      objectValue["40"] = object.targetPtr.toValue();
    }
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): RunEvent {
    const targetValue = objectValue["40"];
    const unpackedTarget =
      targetValue != undefined
        ? NodeReference.fromValue(targetValue, _session, _supergraph, _graph, _connection)
        : null;
    const parentValue = objectValue["3"];
    const unpackedParent =
      parentValue != undefined
        ? NodeReference.fromValue(parentValue, _session, _supergraph, _graph, _connection)
        : null;
    const spaceValue = objectValue["5"];
    const unpackedSpace =
      spaceValue != undefined ? NodeReference.fromValue(spaceValue, _session, _supergraph, _graph, _connection) : null;
    const createdByValue = objectValue["16"];
    const unpackedCreatedBy =
      createdByValue != undefined
        ? NodeReference.fromValue(createdByValue, _session, _supergraph, _graph, _connection)
        : null;
    const updatedByValue = objectValue["18"];
    const unpackedUpdatedBy =
      updatedByValue != undefined
        ? NodeReference.fromValue(updatedByValue, _session, _supergraph, _graph, _connection)
        : null;
    return new RunEvent({
      type: Number(objectValue["30"]),
      id: String(objectValue["2"]),
      createdAt: Temporal.ZonedDateTime.from(objectValue["15"]),
      updatedAt: Temporal.ZonedDateTime.from(objectValue["17"]),
      node: NodeReference.fromValue(objectValue["35"], _session, _supergraph, _graph, _connection),
      target: unpackedTarget,
      parent: unpackedParent,
      space: unpackedSpace,
      createdBy: unpackedCreatedBy,
      updatedBy: unpackedUpdatedBy,
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
  ): RunEvent {
    return RunEvent.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
  }

  toProto(): RunEventProto {
    return RunEvent.__packProto__(this);
  }

  static __packProto__(object: RunEvent): RunEventProto {
    const objectProto: Partial<RunEventProto> = { metatype: 4001 };
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
    objectProto.type = Number(object.type) as RunEventTypeProto;
    objectProto.nodePtr = object.nodePtr.toProto();
    if (object.targetPtr != null) {
      objectProto.targetPtr = object.targetPtr.toProto();
    }
    return objectProto as RunEventProto;
  }

  static __unpackProto__(
    objectProto: RunEventProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): RunEvent {
    return new RunEvent({
      type: Number(objectProto.type) as RunEventType,
      id: String(objectProto.id),
      createdAt: unpackProtoTimestamp(objectProto.createdAt!),
      updatedAt: unpackProtoTimestamp(objectProto.updatedAt!),
      node: NodeReference.fromProto(objectProto.nodePtr!, _session, _supergraph, _graph, _connection),
      target:
        objectProto.targetPtr != undefined
          ? NodeReference.fromProto(objectProto.targetPtr!, _session, _supergraph, _graph, _connection)
          : null,
      parent:
        objectProto.parentPtr != undefined
          ? NodeReference.fromProto(objectProto.parentPtr!, _session, _supergraph, _graph, _connection)
          : null,
      space:
        objectProto.spacePtr != undefined
          ? NodeReference.fromProto(objectProto.spacePtr!, _session, _supergraph, _graph, _connection)
          : null,
      createdBy:
        objectProto.createdByPtr != undefined
          ? NodeReference.fromProto(objectProto.createdByPtr!, _session, _supergraph, _graph, _connection)
          : null,
      updatedBy:
        objectProto.updatedByPtr != undefined
          ? NodeReference.fromProto(objectProto.updatedByPtr!, _session, _supergraph, _graph, _connection)
          : null,
      _session,
      _graph,
      _connection,
    });
  }

  static fromProto(
    objectProto: RunEventProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): RunEvent {
    return RunEvent.__unpackProto__(objectProto, _session, _supergraph, _graph, _connection);
  }

  /* ==== DESTACK_CUSTOM_START ==== */

  // ...

  /* ==== DESTACK_CUSTOM_END ==== */
}
registerNodeClass(NodeType.RUN_EVENT, RunEvent);
/* ==== DESTACK_GENERATED_END:NODE:4001 ==== */

/* ==== DESTACK_GENERATED_START:NODE:4000 ==== */
/**
 * Run something somewhere, somehow.
 */
export class Run extends Node implements Spatial, Particle, Analytic, Indexed, IsExtensible {
  static metatype: NodeType = NodeType.RUN;
  static __traits__: TraitType[] = [
    TraitType.SPATIAL,
    TraitType.PARTICLE,
    TraitType.ANALYTIC,
    TraitType.INDEXED,
    TraitType.TRACKED,
    TraitType.EXTENSIBLE,
  ];
  static __rootType__: NodeType | null = NodeType.SPACE;
  static __parentTypes__: NodeType[] = [NodeType.SPACE];
  static __childTypes__: NodeType[] = [NodeType.FIELD, NodeType.INTERRUPTION, NodeType.SPAN];
  static __ancestorTypes__: NodeType[] = [NodeType.SPACE];
  static __descendantTypes__: NodeType[] = [
    NodeType.OPTION,
    NodeType.SPAN,
    NodeType.TAGGING,
    NodeType.INTERRUPTION,
    NodeType.FIELD,
  ];

  /**
   * Run.parent
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
   * IsTracked.createdAt
   */
  readonly createdAt: Temporal.ZonedDateTime;

  /**
   * IsTracked.createdBy
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
   * IsTracked.updatedAt
   */
  readonly updatedAt: Temporal.ZonedDateTime;

  /**
   * IsTracked.updatedBy
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
   * IsExtensible.value
   */
  value: Map<string, Value>;

  /**
   * Run.target
   */
  get target(): (Node & IsRunnable) | null {
    const nodePtr: NodeReference | null = this.targetPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as (Node & IsRunnable) | null;
    }
    return null;
  }
  set target(node: (Node & IsRunnable) | null) {
    if (node === null) {
      this.targetPtr = null;
    } else {
      this.targetPtr = node.toRef();
    }
  }
  targetPtr: NodeReference | null;

  /**
   * Run.status
   */
  status: RunStatus;

  /**
   * Duration from first attempt start to last attempt termination.
   */
  duration: Temporal.Duration | null;

  /**
   * When the Run is scheduled to start.
   */
  scheduledAt: Temporal.ZonedDateTime | null;

  /**
   * When the Run first started.
   */
  startedAt: Temporal.ZonedDateTime | null;

  /**
   * When the Run was last active.
   */
  seenAt: Temporal.ZonedDateTime | null;

  /**
   * When the Run was interrupted.
   */
  interruptedAt: Temporal.ZonedDateTime | null;

  /**
   * When the Run was last terminated.
   */
  terminatedAt: Temporal.ZonedDateTime | null;

  /**
   * The latest Interruption.
   */
  get interruption(): Interruption | null {
    const nodePtr: NodeReference | null = this.interruptionPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Interruption | null;
    }
    return null;
  }
  set interruption(node: Interruption | null) {
    if (node === null) {
      this.interruptionPtr = null;
    } else {
      this.interruptionPtr = node.toRef();
    }
  }
  interruptionPtr: NodeReference | null;

  constructor(options: {
    id?: string;
    parent?: Space | NodeReference | null;
    space?: Space | NodeReference | null;
    createdAt?: Temporal.ZonedDateTime;
    createdBy?: (Node & IsSubject) | NodeReference | null;
    updatedAt?: Temporal.ZonedDateTime;
    updatedBy?: (Node & IsSubject) | NodeReference | null;
    value?: Map<string, Value>;
    target?: (Node & IsRunnable) | NodeReference | null;
    status: RunStatus;
    duration?: Temporal.Duration | null;
    scheduledAt?: Temporal.ZonedDateTime | null;
    startedAt?: Temporal.ZonedDateTime | null;
    seenAt?: Temporal.ZonedDateTime | null;
    interruptedAt?: Temporal.ZonedDateTime | null;
    terminatedAt?: Temporal.ZonedDateTime | null;
    interruption?: Interruption | NodeReference | null;
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
    if (_parent != null && _parent instanceof Node) {
      _parent = _parent.toRef();
    }
    this.parentPtr = _parent;
    let _space = options.space ?? null;
    if (_space != null && _space instanceof Node) {
      _space = _space.toRef();
    }
    this.spacePtr = _space;
    let _value = options.value ?? null;
    if (_value === null) {
      _value = new Map();
    }
    this.value = _value;
    let _target = options.target ?? null;
    if (_target != null && _target instanceof Node) {
      _target = _target.toRef();
    }
    this.targetPtr = _target;
    let _status = options.status;
    if (_status === null) {
      throw new Error(`Run.status is required`);
    }
    this.status = _status;
    let _duration = options.duration ?? null;
    this.duration = _duration;
    let _scheduledAt = options.scheduledAt ?? null;
    this.scheduledAt = _scheduledAt;
    let _startedAt = options.startedAt ?? null;
    this.startedAt = _startedAt;
    let _seenAt = options.seenAt ?? null;
    this.seenAt = _seenAt;
    let _interruptedAt = options.interruptedAt ?? null;
    this.interruptedAt = _interruptedAt;
    let _terminatedAt = options.terminatedAt ?? null;
    this.terminatedAt = _terminatedAt;
    let _interruption = options.interruption ?? null;
    if (_interruption != null && _interruption instanceof Node) {
      _interruption = _interruption.toRef();
    }
    this.interruptionPtr = _interruption;

    // identity
    if (options.id == null) {
      const now = Temporal.Now.zonedDateTimeISO();
      this.createdAt = now;
      this.createdByPtr = null;
      this.updatedAt = now;
      this.updatedByPtr = null;
    } else {
      if (options.createdAt == null || options.updatedAt == null) {
        throw new Error(`{cls.__name__}.createdAt and {cls.__name__}.updatedAt are required for existing Nodes`);
      }
      this.createdAt = options.createdAt;
      this.createdByPtr =
        options.createdBy != null
          ? options.createdBy instanceof Node
            ? options.createdBy.toRef()
            : options.createdBy
          : null;
      this.updatedAt = options.updatedAt;
      this.updatedByPtr =
        options.updatedBy != null
          ? options.updatedBy instanceof Node
            ? options.updatedBy.toRef()
            : options.updatedBy
          : null;
    }
  }

  equals(other: any): boolean {
    if (!(this.metatype === other.metatype)) {
      return false;
    }
    if (!(this.status === other.status)) {
      return false;
    }
    if (
      (this.duration == null) !== (other.duration == null) ||
      (this.duration != null && !(this.duration === other.duration))
    ) {
      return false;
    }
    if (
      (this.scheduledAt == null) !== (other.scheduledAt == null) ||
      (this.scheduledAt != null && !(this.scheduledAt === other.scheduledAt))
    ) {
      return false;
    }
    if (
      (this.startedAt == null) !== (other.startedAt == null) ||
      (this.startedAt != null && !(this.startedAt === other.startedAt))
    ) {
      return false;
    }
    if ((this.seenAt == null) !== (other.seenAt == null) || (this.seenAt != null && !(this.seenAt === other.seenAt))) {
      return false;
    }
    if (
      (this.interruptedAt == null) !== (other.interruptedAt == null) ||
      (this.interruptedAt != null && !(this.interruptedAt === other.interruptedAt))
    ) {
      return false;
    }
    if (
      (this.terminatedAt == null) !== (other.terminatedAt == null) ||
      (this.terminatedAt != null && !(this.terminatedAt === other.terminatedAt))
    ) {
      return false;
    }
    if (Object.keys(this.value).length !== Object.keys(other.value).length) {
      return false;
    }
    for (const key in this.value) {
      if (!(key in other.value)) {
        return false;
      }
      if (!this.value[key].equals(other.value[key])) {
        return false;
      }
    }
    if (
      (this.targetPtr == null) !== (other.targetPtr == null) ||
      (this.targetPtr != null && !(this.targetPtr.id === other.targetPtr.id))
    ) {
      return false;
    }
    if (
      (this.interruptionPtr == null) !== (other.interruptionPtr == null) ||
      (this.interruptionPtr != null && !(this.interruptionPtr.id === other.interruptionPtr.id))
    ) {
      return false;
    }
    if (
      (this.spacePtr == null) !== (other.spacePtr == null) ||
      (this.spacePtr != null && !(this.spacePtr.id === other.spacePtr.id))
    ) {
      return false;
    }
    return true;
  }

  hash(): number {
    throw new Error("not implemented");
  }

  validate(): void {
    throw new Error("not implemented");
  }

  __toRef__(): NodeReference {
    return new NodeReference({
      nodeType: NodeType.RUN,
      id: this.id,
      spaceId: this.spacePtr?.id ?? null,
      _session: this._session,
      _supergraph: this._supergraph,
    });
  }

  get _pathKey(): string {
    return "Run[id={this.id}]";
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

  toValue(): { [key: string]: any } {
    return Run.__packValue__(this);
  }

  static __packValue__(object: Run): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 4000;
    objectValue["2"] = String(object.id);
    if (object.parentPtr != null) {
      objectValue["3"] = object.parentPtr.toValue();
    }
    if (object.spacePtr != null) {
      objectValue["5"] = object.spacePtr.toValue();
    }
    objectValue["15"] = object.createdAt.toString();
    if (object.createdByPtr != null) {
      objectValue["16"] = object.createdByPtr.toValue();
    }
    objectValue["17"] = object.updatedAt.toString();
    if (object.updatedByPtr != null) {
      objectValue["18"] = object.updatedByPtr.toValue();
    }
    if (object.value) {
      const packedValue: { [key: string]: any } = {};
      for (const [key, value] of object.value) {
        packedValue[String(String(key))] = value.toValue();
      }
      objectValue["21"] = packedValue;
    }
    if (object.targetPtr != null) {
      objectValue["40"] = object.targetPtr.toValue();
    }
    objectValue["41"] = object.status;
    if (object.duration != null) {
      objectValue["42"] = timedeltaToISOFormat(object.duration);
    }
    if (object.scheduledAt != null) {
      objectValue["45"] = object.scheduledAt.toString();
    }
    if (object.startedAt != null) {
      objectValue["46"] = object.startedAt.toString();
    }
    if (object.seenAt != null) {
      objectValue["47"] = object.seenAt.toString();
    }
    if (object.interruptedAt != null) {
      objectValue["48"] = object.interruptedAt.toString();
    }
    if (object.terminatedAt != null) {
      objectValue["49"] = object.terminatedAt.toString();
    }
    if (object.interruptionPtr != null) {
      objectValue["51"] = object.interruptionPtr.toValue();
    }
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Run {
    const durationValue = objectValue["42"];
    const unpackedDuration = durationValue != undefined ? timedeltaFromISOFormat(durationValue) : null;
    const scheduledAtValue = objectValue["45"];
    const unpackedScheduledAt = scheduledAtValue != undefined ? Temporal.ZonedDateTime.from(scheduledAtValue) : null;
    const startedAtValue = objectValue["46"];
    const unpackedStartedAt = startedAtValue != undefined ? Temporal.ZonedDateTime.from(startedAtValue) : null;
    const seenAtValue = objectValue["47"];
    const unpackedSeenAt = seenAtValue != undefined ? Temporal.ZonedDateTime.from(seenAtValue) : null;
    const interruptedAtValue = objectValue["48"];
    const unpackedInterruptedAt =
      interruptedAtValue != undefined ? Temporal.ZonedDateTime.from(interruptedAtValue) : null;
    const terminatedAtValue = objectValue["49"];
    const unpackedTerminatedAt = terminatedAtValue != undefined ? Temporal.ZonedDateTime.from(terminatedAtValue) : null;
    const unpackedValue = new Map();
    if (objectValue["21"] != undefined) {
      for (const [key, value] of Object.entries(objectValue["21"])) {
        unpackedValue.set(String(key), Value.fromValue(value as any, _session, _supergraph, _graph, _connection));
      }
    }
    const parentValue = objectValue["3"];
    const unpackedParent =
      parentValue != undefined
        ? NodeReference.fromValue(parentValue, _session, _supergraph, _graph, _connection)
        : null;
    const targetValue = objectValue["40"];
    const unpackedTarget =
      targetValue != undefined
        ? NodeReference.fromValue(targetValue, _session, _supergraph, _graph, _connection)
        : null;
    const interruptionValue = objectValue["51"];
    const unpackedInterruption =
      interruptionValue != undefined
        ? NodeReference.fromValue(interruptionValue, _session, _supergraph, _graph, _connection)
        : null;
    const spaceValue = objectValue["5"];
    const unpackedSpace =
      spaceValue != undefined ? NodeReference.fromValue(spaceValue, _session, _supergraph, _graph, _connection) : null;
    const createdByValue = objectValue["16"];
    const unpackedCreatedBy =
      createdByValue != undefined
        ? NodeReference.fromValue(createdByValue, _session, _supergraph, _graph, _connection)
        : null;
    const updatedByValue = objectValue["18"];
    const unpackedUpdatedBy =
      updatedByValue != undefined
        ? NodeReference.fromValue(updatedByValue, _session, _supergraph, _graph, _connection)
        : null;
    return new Run({
      status: Number(objectValue["41"]),
      duration: unpackedDuration,
      scheduledAt: unpackedScheduledAt,
      startedAt: unpackedStartedAt,
      seenAt: unpackedSeenAt,
      interruptedAt: unpackedInterruptedAt,
      terminatedAt: unpackedTerminatedAt,
      id: String(objectValue["2"]),
      createdAt: Temporal.ZonedDateTime.from(objectValue["15"]),
      updatedAt: Temporal.ZonedDateTime.from(objectValue["17"]),
      value: unpackedValue,
      parent: unpackedParent,
      target: unpackedTarget,
      interruption: unpackedInterruption,
      space: unpackedSpace,
      createdBy: unpackedCreatedBy,
      updatedBy: unpackedUpdatedBy,
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
  ): Run {
    return Run.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
  }

  toProto(): RunProto {
    return Run.__packProto__(this);
  }

  static __packProto__(object: Run): RunProto {
    const objectProto: Partial<RunProto> = { metatype: 4000 };
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
    if (object.value) {
      objectProto.value = {};
      for (const [key, value] of object.value) {
        objectProto.value![String(key)] = value.toProto();
      }
    }
    if (object.targetPtr != null) {
      objectProto.targetPtr = object.targetPtr.toProto();
    }
    objectProto.status = Number(object.status) as RunStatusProto;
    if (object.duration != null) {
      objectProto.duration = packProtoDuration(object.duration);
    }
    if (object.scheduledAt != null) {
      objectProto.scheduledAt = packProtoTimestamp(object.scheduledAt);
    }
    if (object.startedAt != null) {
      objectProto.startedAt = packProtoTimestamp(object.startedAt);
    }
    if (object.seenAt != null) {
      objectProto.seenAt = packProtoTimestamp(object.seenAt);
    }
    if (object.interruptedAt != null) {
      objectProto.interruptedAt = packProtoTimestamp(object.interruptedAt);
    }
    if (object.terminatedAt != null) {
      objectProto.terminatedAt = packProtoTimestamp(object.terminatedAt);
    }
    if (object.interruptionPtr != null) {
      objectProto.interruptionPtr = object.interruptionPtr.toProto();
    }
    return objectProto as RunProto;
  }

  static __unpackProto__(
    objectProto: RunProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Run {
    const unpackedValue = new Map();
    if (objectProto.value) {
      for (const [key, value] of Object.entries(objectProto.value)) {
        unpackedValue.set(String(key), Value.fromProto((value as any)!, _session, _supergraph, _graph, _connection));
      }
    }
    return new Run({
      status: Number(objectProto.status) as RunStatus,
      duration: objectProto.duration != undefined ? unpackProtoDuration(objectProto.duration!) : null,
      scheduledAt: objectProto.scheduledAt != undefined ? unpackProtoTimestamp(objectProto.scheduledAt!) : null,
      startedAt: objectProto.startedAt != undefined ? unpackProtoTimestamp(objectProto.startedAt!) : null,
      seenAt: objectProto.seenAt != undefined ? unpackProtoTimestamp(objectProto.seenAt!) : null,
      interruptedAt: objectProto.interruptedAt != undefined ? unpackProtoTimestamp(objectProto.interruptedAt!) : null,
      terminatedAt: objectProto.terminatedAt != undefined ? unpackProtoTimestamp(objectProto.terminatedAt!) : null,
      id: String(objectProto.id),
      createdAt: unpackProtoTimestamp(objectProto.createdAt!),
      updatedAt: unpackProtoTimestamp(objectProto.updatedAt!),
      value: unpackedValue,
      parent:
        objectProto.parentPtr != undefined
          ? NodeReference.fromProto(objectProto.parentPtr!, _session, _supergraph, _graph, _connection)
          : null,
      target:
        objectProto.targetPtr != undefined
          ? NodeReference.fromProto(objectProto.targetPtr!, _session, _supergraph, _graph, _connection)
          : null,
      interruption:
        objectProto.interruptionPtr != undefined
          ? NodeReference.fromProto(objectProto.interruptionPtr!, _session, _supergraph, _graph, _connection)
          : null,
      space:
        objectProto.spacePtr != undefined
          ? NodeReference.fromProto(objectProto.spacePtr!, _session, _supergraph, _graph, _connection)
          : null,
      createdBy:
        objectProto.createdByPtr != undefined
          ? NodeReference.fromProto(objectProto.createdByPtr!, _session, _supergraph, _graph, _connection)
          : null,
      updatedBy:
        objectProto.updatedByPtr != undefined
          ? NodeReference.fromProto(objectProto.updatedByPtr!, _session, _supergraph, _graph, _connection)
          : null,
      _session,
      _graph,
      _connection,
    });
  }

  static fromProto(
    objectProto: RunProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Run {
    return Run.__unpackProto__(objectProto, _session, _supergraph, _graph, _connection);
  }

  /* ==== DESTACK_CUSTOM_START ==== */

  // ...

  /* ==== DESTACK_CUSTOM_END ==== */
}
registerNodeClass(NodeType.RUN, Run);
/* ==== DESTACK_GENERATED_END:NODE:4000 ==== */
