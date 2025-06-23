import { IsTracked, IsExtensible, ACTIVE_SESSION, Spatial, Value, Message, EnumType, Space, StructType, Struct, Particle, Run, NodeReference, NodeType, Graph, Action, User, Indexed, Agent, Node, QueryConnection, Service, StructFrozen, Script, Supergraph, Span, Analytic, BuiltinObject, Session, activeSession } from '@/language';
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
    id: string,
    parent?: Run | NodeReference | null,
    space?: Space | NodeReference | null,
    createdAt: Temporal.ZonedDateTime,
    createdBy?: Agent | User | NodeReference | null,
    updatedAt: Temporal.ZonedDateTime,
    updatedBy?: Agent | User | NodeReference | null,
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
    throw new Error("not implemented");
  }

  hash(): number {
    throw new Error("not implemented");
  }

  validate(): void {
    throw new Error("not implemented");
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