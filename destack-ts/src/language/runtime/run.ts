import { Indexed, NodeType, QueryConnection, StructType, Space, NodeReference, Interruption, Action, Graph, Analytic, Script, Value, User, Struct, Service, StructFrozen, Event, Spatial, BuiltinObject, EnumType, Agent, Particle, activeSession, Node, IsTracked, Supergraph, IsExtensible, IsFrozen, ACTIVE_SESSION, Session } from '@/language';
import { Temporal } from 'temporal-polyfill';

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
  readonly parentPtr: NodeReference | null
  get space(): Space | null | null {
      const nodePtr: NodeReference | null = this.spacePtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Space | null | null;
      }
      return null;
  }
  ;
  readonly spacePtr: NodeReference | null
  readonly createdAt: Temporal.ZonedDateTime;
  get createdBy(): Agent | User | null | null {
      const nodePtr: NodeReference | null = this.createdByPtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Agent | User | null | null;
      }
      return null;
  }
  ;
  readonly createdByPtr: NodeReference | null
  readonly updatedAt: Temporal.ZonedDateTime;
  get updatedBy(): Agent | User | null | null {
      const nodePtr: NodeReference | null = this.updatedByPtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Agent | User | null | null;
      }
      return null;
  }
  ;
  readonly updatedByPtr: NodeReference | null
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
    id: string,
    parent?: Space | NodeReference | null,
    space?: Space | NodeReference | null,
    createdAt: Temporal.ZonedDateTime,
    createdBy?: Agent | User | NodeReference | null,
    updatedAt: Temporal.ZonedDateTime,
    updatedBy?: Agent | User | NodeReference | null,
    type: RunEventType,
    node: Run | NodeReference,
    target?: Action | Script | Service | NodeReference | null,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: Graph | null,
    _connection?: QueryConnection | null
  }) {
    super(
        // id
        options.id,
        // parent
        options.parent != null ? (options.parent.metatype == StructType.NODE_REFERENCE ? (options.parent as NodeReference) : (options.parent as Node).toRef()) : null,
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
        options.id != null,
    );

    this.id = options.id;
    this.parentPtr = options.parent != null ? (options.parent.metatype == StructType.NODE_REFERENCE ? (options.parent as NodeReference) : (options.parent as Node).toRef()) : null;
    this.spacePtr = options.space != null ? (options.space.metatype == StructType.NODE_REFERENCE ? (options.space as NodeReference) : (options.space as Node).toRef()) : null;
    this.createdAt = options.createdAt;
    this.createdByPtr = options.createdBy != null ? (options.createdBy.metatype == StructType.NODE_REFERENCE ? (options.createdBy as NodeReference) : (options.createdBy as Node).toRef()) : null;
    this.updatedAt = options.updatedAt;
    this.updatedByPtr = options.updatedBy != null ? (options.updatedBy.metatype == StructType.NODE_REFERENCE ? (options.updatedBy as NodeReference) : (options.updatedBy as Node).toRef()) : null;
    this.type = options.type;
    this.nodePtr = options.node != null ? (options.node.metatype == StructType.NODE_REFERENCE ? (options.node as NodeReference) : (options.node as Node).toRef()) : null;
    this.targetPtr = options.target != null ? (options.target.metatype == StructType.NODE_REFERENCE ? (options.target as NodeReference) : (options.target as Node).toRef()) : null;
  }

  equals(other: any): boolean {
    throw new Error("not implemented");
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
  readonly parentPtr: NodeReference | null
  get space(): Space | null | null {
      const nodePtr: NodeReference | null = this.spacePtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Space | null | null;
      }
      return null;
  }
  ;
  readonly spacePtr: NodeReference | null
  readonly createdAt: Temporal.ZonedDateTime;
  get createdBy(): Agent | User | null | null {
      const nodePtr: NodeReference | null = this.createdByPtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Agent | User | null | null;
      }
      return null;
  }
  ;
  readonly createdByPtr: NodeReference | null
  readonly updatedAt: Temporal.ZonedDateTime;
  get updatedBy(): Agent | User | null | null {
      const nodePtr: NodeReference | null = this.updatedByPtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Agent | User | null | null;
      }
      return null;
  }
  ;
  readonly updatedByPtr: NodeReference | null
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
    id: string,
    parent?: Space | NodeReference | null,
    space?: Space | NodeReference | null,
    createdAt: Temporal.ZonedDateTime,
    createdBy?: Agent | User | NodeReference | null,
    updatedAt: Temporal.ZonedDateTime,
    updatedBy?: Agent | User | NodeReference | null,
    value?: Map<string, Value>,
    target?: Action | Script | Service | NodeReference | null,
    status: RunStatus,
    duration?: Temporal.Duration | null,
    scheduledAt?: Temporal.ZonedDateTime | null,
    startedAt?: Temporal.ZonedDateTime | null,
    seenAt?: Temporal.ZonedDateTime | null,
    interruptedAt?: Temporal.ZonedDateTime | null,
    terminatedAt?: Temporal.ZonedDateTime | null,
    interruption?: Interruption | NodeReference | null,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: Graph | null,
    _connection?: QueryConnection | null
  }) {
    super(
        // id
        options.id,
        // parent
        options.parent != null ? (options.parent.metatype == StructType.NODE_REFERENCE ? (options.parent as NodeReference) : (options.parent as Node).toRef()) : null,
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
        options.id != null,
    );

    this.id = options.id;
    this.parentPtr = options.parent != null ? (options.parent.metatype == StructType.NODE_REFERENCE ? (options.parent as NodeReference) : (options.parent as Node).toRef()) : null;
    this.spacePtr = options.space != null ? (options.space.metatype == StructType.NODE_REFERENCE ? (options.space as NodeReference) : (options.space as Node).toRef()) : null;
    this.createdAt = options.createdAt;
    this.createdByPtr = options.createdBy != null ? (options.createdBy.metatype == StructType.NODE_REFERENCE ? (options.createdBy as NodeReference) : (options.createdBy as Node).toRef()) : null;
    this.updatedAt = options.updatedAt;
    this.updatedByPtr = options.updatedBy != null ? (options.updatedBy.metatype == StructType.NODE_REFERENCE ? (options.updatedBy as NodeReference) : (options.updatedBy as Node).toRef()) : null;
    this.value = options.value ?? new Map();
    this.targetPtr = options.target != null ? (options.target.metatype == StructType.NODE_REFERENCE ? (options.target as NodeReference) : (options.target as Node).toRef()) : null;
    this.status = options.status;
    this.duration = options.duration ?? null;
    this.scheduledAt = options.scheduledAt ?? null;
    this.startedAt = options.startedAt ?? null;
    this.seenAt = options.seenAt ?? null;
    this.interruptedAt = options.interruptedAt ?? null;
    this.terminatedAt = options.terminatedAt ?? null;
    this.interruptionPtr = options.interruption != null ? (options.interruption.metatype == StructType.NODE_REFERENCE ? (options.interruption as NodeReference) : (options.interruption as Node).toRef()) : null;
  }

  equals(other: any): boolean {
    throw new Error("not implemented");
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
}
/* ==== DESTACK_GENERATED_END:NODE:4000 ==== */