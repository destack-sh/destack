import {
  Analytic,
  Graph,
  Indexed,
  IsExtensible,
  IsRunnable,
  IsSubject,
  IsTracked,
  Message,
  Node,
  NodeReference,
  NodeType,
  Particle,
  QueryConnection,
  Run,
  Session,
  Space,
  Span,
  Spatial,
  StructType,
  Supergraph,
  TraitType,
  Value,
} from "@destack/language";
import { Temporal } from "temporal-polyfill";

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
  static metatype: NodeType = NodeType.INTERRUPTION;
  static __traits__: TraitType[] = [
    TraitType.SPATIAL,
    TraitType.PARTICLE,
    TraitType.ANALYTIC,
    TraitType.INDEXED,
    TraitType.TRACKED,
    TraitType.EXTENSIBLE,
  ];
  static __rootType__: NodeType | null = NodeType.SPACE;
  static __parentTypes__: NodeType[] = [NodeType.RUN];
  static __childTypes__: NodeType[] = [NodeType.FIELD];
  static __ancestorTypes__: NodeType[] = [NodeType.RUN, NodeType.SPACE];
  static __descendantTypes__: NodeType[] = [NodeType.FIELD, NodeType.OPTION, NodeType.TAGGING];

  get parent(): Run | null | null {
    const nodePtr: NodeReference | null = this.parentPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Run | null | null;
    }
    return null;
  }
  readonly parentPtr: NodeReference | null;
  get space(): Space | null | null {
    const nodePtr: NodeReference | null = this.spacePtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Space | null | null;
    }
    return null;
  }
  readonly spacePtr: NodeReference | null;
  readonly createdAt: Temporal.ZonedDateTime;
  get createdBy(): (Node & IsSubject) | null | null {
    const nodePtr: NodeReference | null = this.createdByPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as (Node & IsSubject) | null | null;
    }
    return null;
  }
  readonly createdByPtr: NodeReference | null;
  readonly updatedAt: Temporal.ZonedDateTime;
  get updatedBy(): (Node & IsSubject) | null | null {
    const nodePtr: NodeReference | null = this.updatedByPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as (Node & IsSubject) | null | null;
    }
    return null;
  }
  readonly updatedByPtr: NodeReference | null;
  value: Map<string, Value>;
  type: InterruptionType;
  get runnable(): (Node & IsRunnable) | null | null {
    const nodePtr: NodeReference | null = this.runnablePtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as (Node & IsRunnable) | null | null;
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
  spanPtr: NodeReference | null;
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
  messagePtr: NodeReference | null;

  constructor(options: {
    id?: string;
    parent?: Run | NodeReference | null;
    space?: Space | NodeReference | null;
    createdAt?: Temporal.ZonedDateTime;
    createdBy?: (Node & IsSubject) | NodeReference | null;
    updatedAt?: Temporal.ZonedDateTime;
    updatedBy?: (Node & IsSubject) | NodeReference | null;
    value?: Map<string, Value>;
    type: InterruptionType;
    runnable?: (Node & IsRunnable) | NodeReference | null;
    span?: Span | NodeReference | null;
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
      throw new Error(`Interruption.value is required`);
    }
    this.value = _value;
    let _type = options.type;
    if (_type === null) {
      throw new Error(`Interruption.type is required`);
    }
    this.type = _type;
    let _runnable = options.runnable ?? null;
    if (_runnable != null && _runnable instanceof Node) {
      _runnable = _runnable.toRef();
    }
    this.runnablePtr = _runnable;
    let _span = options.span ?? null;
    if (_span != null && _span instanceof Node) {
      _span = _span.toRef();
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
    if (_message != null && _message instanceof Node) {
      _message = _message.toRef();
    }
    this.messagePtr = _message;

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
}
/* ==== DESTACK_GENERATED_END:NODE:4020 ==== */
