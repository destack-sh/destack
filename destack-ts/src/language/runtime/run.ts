import { Agent, BuiltinObject, ACTIVE_SESSION, Session, StructFrozen, IsFrozen, Graph, Service, Struct, Event, activeSession, Particle, Interruption, StructType, NodeType, Action, Indexed, Supergraph, QueryConnection, Error, NodeReference, Value, User, Analytic, IsExtensible, Spatial, Space, Script, EnumType, Node, IsTracked } from '@/language';
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

  set node(node: Run) {
      this.nodePtr = node.toRef();
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

  set target(node: Action | Script | Service | null) {
      if (node === null) {
          this.targetPtr = null;
      } else {
          this.targetPtr = node.toRef();
      }
  }
  ;
  targetPtr: NodeReference | null

  constructor(options: {
    type: RunEventType,
    node: Run | NodeReference,
    target?: Action | Script | Service | NodeReference | null,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: Graph | null,
    _connection?: QueryConnection | null
  }) {
    const session = options._session ?? ACTIVE_SESSION.get();
    const supergraph = options._supergraph ?? session.supergraph;
    super(options.id, options.parent, session, supergraph, options._graph, options._connection);
    this.type = options.type;
    this.nodePtr = options.node != null ? (options.node.metatype == StructType.NODE_REFERENCE ? (options.node as NodeReference) : (options.node as Node).toRef()) : null;
    this.targetPtr = options.target != null ? (options.target.metatype == StructType.NODE_REFERENCE ? (options.target as NodeReference) : (options.target as Node).toRef()) : null;
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

  set target(node: Action | Script | Service | null) {
      if (node === null) {
          this.targetPtr = null;
      } else {
          this.targetPtr = node.toRef();
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

  set interruption(node: Interruption | null) {
      if (node === null) {
          this.interruptionPtr = null;
      } else {
          this.interruptionPtr = node.toRef();
      }
  }
  ;
  interruptionPtr: NodeReference | null

  constructor(options: {
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
  }) {
    const session = options._session ?? ACTIVE_SESSION.get();
    const supergraph = options._supergraph ?? session.supergraph;
    super(options.id, options.parent, session, supergraph, options._graph, options._connection);
    this.value = options.value ?? new Map();
    this.targetPtr = options.target != null ? (options.target.metatype == StructType.NODE_REFERENCE ? (options.target as NodeReference) : (options.target as Node).toRef()) : null;
    this.status = options.status;
    this.duration = options.duration ?? null;
    this.scheduledAt = options.scheduledAt ?? null;
    this.startedAt = options.startedAt ?? null;
    this.seenAt = options.seenAt ?? null;
    this.interruptedAt = options.interruptedAt ?? null;
    this.terminatedAt = options.terminatedAt ?? null;
    this.error = options.error ?? null;
    this.interruptionPtr = options.interruption != null ? (options.interruption.metatype == StructType.NODE_REFERENCE ? (options.interruption as NodeReference) : (options.interruption as Node).toRef()) : null;
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