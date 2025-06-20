import { User, IsFrozen, NodeReference, Interruption, Event, Supergraph, EnumType, Service, Error, StructType, Space, IsTracked, Spatial, Analytic, Script, Session, QueryConnection, Action, Node, BuiltinObject, Value, Graph, Particle, Indexed, Agent, IsExtensible, NodeType, Struct, StructFrozen } from '@/language';
import { Temporal } from 'temporal-polyfill'; // until Temporal ships natively

/* ==== DESTACK_GENERATED_START:ENUM:4000 ==== */
export enum RunStatus {
  SCHEDULED = 2,
  RUNNING = 10,
  PAUSED = 21,
  YIELDED = 23,
  CANGALAXYED = 51,
  ABORTED = 52,
  FAILED = 53,
  COMPLETED = 54,
}
/* ==== DESTACK_GENERATED_END:ENUM:4000 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:4001 ==== */
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
}
/* ==== DESTACK_GENERATED_END:ENUM:4001 ==== */

/* ==== DESTACK_GENERATED_START:NODE:4001 ==== */
export class RunEvent extends Node implements Spatial, Particle, Analytic, Indexed, Event, IsFrozen, IsTracked {
  readonly id: string;
  get parent(): Space | null | null {
      const nodePtr: NodeReference | null = this.parentPtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Space | null | null;
      }
      return null;
  }
  ;
  parentPtr: NodeReference | null
  get space(): Space | null | null {
      const nodePtr: NodeReference | null = this.spacePtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Space | null | null;
      }
      return null;
  }
  ;
  spacePtr: NodeReference | null
  readonly createdAt: Temporal.ZonedDateTime;
  get createdBy(): Agent | User | null | null {
      const nodePtr: NodeReference | null = this.createdByPtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Agent | User | null | null;
      }
      return null;
  }
  ;
  createdByPtr: NodeReference | null
  readonly updatedAt: Temporal.ZonedDateTime;
  get updatedBy(): Agent | User | null | null {
      const nodePtr: NodeReference | null = this.updatedByPtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Agent | User | null | null;
      }
      return null;
  }
  ;
  updatedByPtr: NodeReference | null
  type: RunEventType;
  get node(): Run | null {
      const nodePtr: NodeReference | null = this.nodePtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Run | null;
      }
      return null;
  }

  set node(value: Run) {
      if (value === null) {
          this.nodePtr = null;
      } else {
          this.nodePtr = value.toRef();
      }
  }
  ;
  nodePtr: NodeReference
  get target(): Action | Script | Service | null | null {
      const nodePtr: NodeReference | null = this.targetPtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Action | Script | Service | null | null;
      }
      return null;
  }

  set target(value: Action | Script | Service | null) {
      if (value === null) {
          this.targetPtr = null;
      } else {
          this.targetPtr = value.toRef();
      }
  }
  ;
  targetPtr: NodeReference | null

  constructor(
    id: string,
    parentPtr: NodeReference | null,
    spacePtr: NodeReference | null,
    createdAt: Temporal.ZonedDateTime,
    createdByPtr: NodeReference | null,
    updatedAt: Temporal.ZonedDateTime,
    updatedByPtr: NodeReference | null,
    type: RunEventType,
    nodePtr: NodeReference,
    targetPtr: NodeReference | null,
    _session: Session,
    _supergraph: Supergraph,
    _graph: Graph,
    _connection: QueryConnection | null
  ) {
    super(id, _session, _supergraph, _graph, _connection);
    this.id = id;
    this.parentPtr = parentPtr;
    this.spacePtr = spacePtr;
    this.createdAt = createdAt;
    this.createdByPtr = createdByPtr;
    this.updatedAt = updatedAt;
    this.updatedByPtr = updatedByPtr;
    this.type = type;
    this.nodePtr = nodePtr;
    this.targetPtr = targetPtr;
  }


  static create(options: {
    type: RunEventType,
    node: Run | NodeReference,
    target?: Action | Script | Service | NodeReference | null,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: Graph | null,
    _connection?: QueryConnection | null
  }): RunEvent {
    const session = options._session ?? ACTIVE_SESSION.get();
    const supergraph = options._supergraph ?? session.supergraph;
    return new RunEvent(
      options.type,
      options.node != null ? (options.node.metatype == StructType.NODE_REFERENCE ? options.node : options.node.toRef()) : null,
      options.target != null ? (options.target.metatype == StructType.NODE_REFERENCE ? options.target : options.target.toRef()) : null,
      session,
      supergraph,
      options._graph,
      options._connection
    );
  }

  equals(other: any): boolean {
    throw new Error("Not implemented");
  }

  hash(): number {
    throw new Error("Not implemented");
  }

  validate(): void {
    throw new Error("Not implemented");
  }

  __toRef__(): NodeReference {
    return new NodeReference(NodeType.RUN_EVENT, this.id, this.spacePtr?.id ?? null, null, this._supergraph);
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
}
/* ==== DESTACK_GENERATED_END:NODE:4001 ==== */

/* ==== DESTACK_GENERATED_START:NODE:4000 ==== */
export class Run extends Node implements Spatial, Particle, Analytic, Indexed, IsTracked, IsExtensible {
  readonly id: string;
  get parent(): Space | null | null {
      const nodePtr: NodeReference | null = this.parentPtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Space | null | null;
      }
      return null;
  }
  ;
  parentPtr: NodeReference | null
  get space(): Space | null | null {
      const nodePtr: NodeReference | null = this.spacePtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Space | null | null;
      }
      return null;
  }
  ;
  spacePtr: NodeReference | null
  readonly createdAt: Temporal.ZonedDateTime;
  get createdBy(): Agent | User | null | null {
      const nodePtr: NodeReference | null = this.createdByPtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Agent | User | null | null;
      }
      return null;
  }
  ;
  createdByPtr: NodeReference | null
  readonly updatedAt: Temporal.ZonedDateTime;
  get updatedBy(): Agent | User | null | null {
      const nodePtr: NodeReference | null = this.updatedByPtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Agent | User | null | null;
      }
      return null;
  }
  ;
  updatedByPtr: NodeReference | null
  value: Map<string, Value>;
  get target(): Action | Script | Service | null | null {
      const nodePtr: NodeReference | null = this.targetPtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Action | Script | Service | null | null;
      }
      return null;
  }

  set target(value: Action | Script | Service | null) {
      if (value === null) {
          this.targetPtr = null;
      } else {
          this.targetPtr = value.toRef();
      }
  }
  ;
  targetPtr: NodeReference | null
  status: RunStatus;
  duration: Temporal.Duration | null;
  scheduledAt: Temporal.ZonedDateTime | null;
  startedAt: Temporal.ZonedDateTime | null;
  seenAt: Temporal.ZonedDateTime | null;
  interruptedAt: Temporal.ZonedDateTime | null;
  terminatedAt: Temporal.ZonedDateTime | null;
  error: Error | null;
  get interruption(): Interruption | null | null {
      const nodePtr: NodeReference | null = this.interruptionPtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Interruption | null | null;
      }
      return null;
  }

  set interruption(value: Interruption | null) {
      if (value === null) {
          this.interruptionPtr = null;
      } else {
          this.interruptionPtr = value.toRef();
      }
  }
  ;
  interruptionPtr: NodeReference | null

  constructor(
    id: string,
    parentPtr: NodeReference | null,
    spacePtr: NodeReference | null,
    createdAt: Temporal.ZonedDateTime,
    createdByPtr: NodeReference | null,
    updatedAt: Temporal.ZonedDateTime,
    updatedByPtr: NodeReference | null,
    value: Map<string, Value>,
    targetPtr: NodeReference | null,
    status: RunStatus,
    duration: Temporal.Duration | null,
    scheduledAt: Temporal.ZonedDateTime | null,
    startedAt: Temporal.ZonedDateTime | null,
    seenAt: Temporal.ZonedDateTime | null,
    interruptedAt: Temporal.ZonedDateTime | null,
    terminatedAt: Temporal.ZonedDateTime | null,
    error: Error | null,
    interruptionPtr: NodeReference | null,
    _session: Session,
    _supergraph: Supergraph,
    _graph: Graph,
    _connection: QueryConnection | null
  ) {
    super(id, _session, _supergraph, _graph, _connection);
    this.id = id;
    this.parentPtr = parentPtr;
    this.spacePtr = spacePtr;
    this.createdAt = createdAt;
    this.createdByPtr = createdByPtr;
    this.updatedAt = updatedAt;
    this.updatedByPtr = updatedByPtr;
    this.value = value;
    this.targetPtr = targetPtr;
    this.status = status;
    this.duration = duration;
    this.scheduledAt = scheduledAt;
    this.startedAt = startedAt;
    this.seenAt = seenAt;
    this.interruptedAt = interruptedAt;
    this.terminatedAt = terminatedAt;
    this.error = error;
    this.interruptionPtr = interruptionPtr;
  }


  static create(options: {
    value?: Map<string, Value>,
    target?: Action | Script | Service | NodeReference | null,
    status: RunStatus,
    duration?: Temporal.Duration | null,
    scheduledAt?: Temporal.ZonedDateTime | null,
    startedAt?: Temporal.ZonedDateTime | null,
    seenAt?: Temporal.ZonedDateTime | null,
    interruptedAt?: Temporal.ZonedDateTime | null,
    terminatedAt?: Temporal.ZonedDateTime | null,
    error?: Error | null,
    interruption?: Interruption | NodeReference | null,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: Graph | null,
    _connection?: QueryConnection | null
  }): Run {
    const session = options._session ?? ACTIVE_SESSION.get();
    const supergraph = options._supergraph ?? session.supergraph;
    return new Run(
      options.value ?? new Map(),
      options.target != null ? (options.target.metatype == StructType.NODE_REFERENCE ? options.target : options.target.toRef()) : null,
      options.status,
      options.duration ?? null,
      options.scheduledAt ?? null,
      options.startedAt ?? null,
      options.seenAt ?? null,
      options.interruptedAt ?? null,
      options.terminatedAt ?? null,
      options.error ?? null,
      options.interruption != null ? (options.interruption.metatype == StructType.NODE_REFERENCE ? options.interruption : options.interruption.toRef()) : null,
      session,
      supergraph,
      options._graph,
      options._connection
    );
  }

  equals(other: any): boolean {
    throw new Error("Not implemented");
  }

  hash(): number {
    throw new Error("Not implemented");
  }

  validate(): void {
    throw new Error("Not implemented");
  }

  __toRef__(): NodeReference {
    return new NodeReference(NodeType.RUN, this.id, this.spacePtr?.id ?? null, null, this._supergraph);
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
}
/* ==== DESTACK_GENERATED_END:NODE:4000 ==== */