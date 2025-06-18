import { Session, Spatial, Particle, Analytic, User, NodeReference, BuiltinObject, Value, IsExtensible, Agent, Message, IsTracked, Indexed, Span, Run, Graph, Struct, Supergraph, Service, QueryConnection, NodeType, Script, Node, Space, Action } from '@/language';
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
  get parent(): Run | null {
      const nodePtr: NodeReference | null = this.parentPtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Run | null;
      }
      return null;
  }
  ;
  parentPtr: NodeReference | null
  get space(): Space | null {
      const nodePtr: NodeReference | null = this.spacePtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Space | null;
      }
      return null;
  }

  set space(value: Space | null) {
      if (value === null) {
          this.spacePtr = null;
      } else {
          this.spacePtr = value.toRef();
      }
  }
  ;
  spacePtr: NodeReference | null
  readonly createdAt: Temporal.ZonedDateTime;
  get createdBy(): Agent | User | null {
      const nodePtr: NodeReference | null = this.createdByPtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Agent | User | null;
      }
      return null;
  }
  ;
  createdByPtr: NodeReference | null
  readonly updatedAt: Temporal.ZonedDateTime;
  get updatedBy(): Agent | User | null {
      const nodePtr: NodeReference | null = this.updatedByPtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Agent | User | null;
      }
      return null;
  }
  ;
  updatedByPtr: NodeReference | null
  value: Map<string, Value>;
  type: InterruptionType;
  get runnable(): Action | Script | Service | null {
      const nodePtr: NodeReference | null = this.runnablePtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Action | Script | Service | null;
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
  get span(): Span | null {
      const nodePtr: NodeReference | null = this.spanPtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Span | null;
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
  get message(): Message | null {
      const nodePtr: NodeReference | null = this.messagePtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Message | null;
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


  static create(): Interruption {

    return new Interruption();
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