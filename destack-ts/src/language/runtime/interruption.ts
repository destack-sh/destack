import { User, NodeReference, Span, Supergraph, EnumType, Service, StructType, Space, IsTracked, Run, Spatial, Message, Analytic, Script, Session, QueryConnection, Action, Node, BuiltinObject, Value, Graph, Particle, Indexed, Agent, IsExtensible, NodeType, Struct, StructFrozen } from '@/language';
import { Temporal } from 'temporal-polyfill'; // until Temporal ships natively

/* ==== DESTACK_GENERATED_START:ENUM:4020 ==== */
export enum InterruptionType {
  PAUSE = 10,
  YIELD = 20,
  WAIT = 30,
}
/* ==== DESTACK_GENERATED_END:ENUM:4020 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:4021 ==== */
export enum InterruptionStatus {
  OPEN = 10,
  CANGALAXYED = 30,
  COMPLETED = 33,
}
/* ==== DESTACK_GENERATED_END:ENUM:4021 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:4022 ==== */
export enum InterruptionResponse {
  ACCEPT = 10,
  REJECT = 20,
}
/* ==== DESTACK_GENERATED_END:ENUM:4022 ==== */

/* ==== DESTACK_GENERATED_START:NODE:4020 ==== */
export class Interruption extends Node implements Spatial, Particle, Analytic, Indexed, IsTracked, IsExtensible {
  readonly id: string;
  get parent(): Run | null | null {
      const nodePtr: NodeReference | null = this.parentPtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Run | null | null;
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
  type: InterruptionType;
  get runnable(): Action | Script | Service | null | null {
      const nodePtr: NodeReference | null = this.runnablePtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Action | Script | Service | null | null;
      }
      return null;
  }

  set runnable(value: Action | Script | Service | null) {
      if (value === null) {
          this.runnablePtr = null;
      } else {
          this.runnablePtr = value.toRef();
      }
  }
  ;
  runnablePtr: NodeReference | null
  get span(): Span | null | null {
      const nodePtr: NodeReference | null = this.spanPtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Span | null | null;
      }
      return null;
  }

  set span(value: Span | null) {
      if (value === null) {
          this.spanPtr = null;
      } else {
          this.spanPtr = value.toRef();
      }
  }
  ;
  spanPtr: NodeReference | null
  status: InterruptionStatus;
  duration: Temporal.Duration | null;
  closedAt: Temporal.ZonedDateTime | null;
  response: InterruptionResponse | null;
  get message(): Message | null | null {
      const nodePtr: NodeReference | null = this.messagePtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Message | null | null;
      }
      return null;
  }

  set message(value: Message | null) {
      if (value === null) {
          this.messagePtr = null;
      } else {
          this.messagePtr = value.toRef();
      }
  }
  ;
  messagePtr: NodeReference | null

  constructor(
    id: string,
    parentPtr: NodeReference | null,
    spacePtr: NodeReference | null,
    createdAt: Temporal.ZonedDateTime,
    createdByPtr: NodeReference | null,
    updatedAt: Temporal.ZonedDateTime,
    updatedByPtr: NodeReference | null,
    value: Map<string, Value>,
    type: InterruptionType,
    runnablePtr: NodeReference | null,
    spanPtr: NodeReference | null,
    status: InterruptionStatus,
    duration: Temporal.Duration | null,
    closedAt: Temporal.ZonedDateTime | null,
    response: InterruptionResponse | null,
    messagePtr: NodeReference | null,
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
    this.type = type;
    this.runnablePtr = runnablePtr;
    this.spanPtr = spanPtr;
    this.status = status;
    this.duration = duration;
    this.closedAt = closedAt;
    this.response = response;
    this.messagePtr = messagePtr;
  }


  static create(options: {
    value?: Map<string, Value>,
    type: InterruptionType,
    runnable?: Action | Script | Service | NodeReference | null,
    span?: Span | NodeReference | null,
    status?: InterruptionStatus,
    duration?: Temporal.Duration | null,
    closedAt?: Temporal.ZonedDateTime | null,
    response?: InterruptionResponse | null,
    message?: Message | NodeReference | null,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: Graph | null,
    _connection?: QueryConnection | null
  }): Interruption {
    const session = options._session ?? ACTIVE_SESSION.get();
    const supergraph = options._supergraph ?? session.supergraph;
    return new Interruption(
      options.value ?? new Map(),
      options.type,
      options.runnable != null ? (options.runnable.metatype == StructType.NODE_REFERENCE ? options.runnable : options.runnable.toRef()) : null,
      options.span != null ? (options.span.metatype == StructType.NODE_REFERENCE ? options.span : options.span.toRef()) : null,
      options.status ?? InterruptionStatus.OPEN,
      options.duration ?? null,
      options.closedAt ?? null,
      options.response ?? null,
      options.message != null ? (options.message.metatype == StructType.NODE_REFERENCE ? options.message : options.message.toRef()) : null,
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
    return new NodeReference(NodeType.INTERRUPTION, this.id, this.spacePtr?.id ?? null, null, this._supergraph);
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
}
/* ==== DESTACK_GENERATED_END:NODE:4020 ==== */