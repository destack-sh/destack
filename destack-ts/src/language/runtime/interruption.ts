import { Agent, BuiltinObject, ACTIVE_SESSION, Session, StructFrozen, Graph, Service, Struct, activeSession, Particle, StructType, NodeType, Action, Indexed, Span, Supergraph, QueryConnection, NodeReference, Value, User, Analytic, IsExtensible, Spatial, Run, Message, Space, Script, EnumType, Node, IsTracked } from '@/language';
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

  set runnable(node: Action | Script | Service | null) {
      if (node === null) {
          this.runnablePtr = null;
      } else {
          this.runnablePtr = node.toRef();
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

  set span(node: Span | null) {
      if (node === null) {
          this.spanPtr = null;
      } else {
          this.spanPtr = node.toRef();
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

  set message(node: Message | null) {
      if (node === null) {
          this.messagePtr = null;
      } else {
          this.messagePtr = node.toRef();
      }
  }
  ;
  messagePtr: NodeReference | null

  constructor(options: {
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
  }) {
    const session = options._session ?? ACTIVE_SESSION.get();
    const supergraph = options._supergraph ?? session.supergraph;
    super(options.id, options.parent, session, supergraph, options._graph, options._connection);
    this.value = options.value ?? new Map();
    this.type = options.type;
    this.runnablePtr = options.runnable != null ? (options.runnable.metatype == StructType.NODE_REFERENCE ? (options.runnable as NodeReference) : (options.runnable as Node).toRef()) : null;
    this.spanPtr = options.span != null ? (options.span.metatype == StructType.NODE_REFERENCE ? (options.span as NodeReference) : (options.span as Node).toRef()) : null;
    this.status = options.status ?? InterruptionStatus.OPEN;
    this.duration = options.duration ?? null;
    this.closedAt = options.closedAt ?? null;
    this.response = options.response ?? null;
    this.messagePtr = options.message != null ? (options.message.metatype == StructType.NODE_REFERENCE ? (options.message as NodeReference) : (options.message as Node).toRef()) : null;
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