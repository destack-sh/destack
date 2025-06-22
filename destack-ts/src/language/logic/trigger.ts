import { Agent, BuiltinObject, ACTIVE_SESSION, Session, StructFrozen, IsFrozen, Graph, Service, Struct, Event, activeSession, Particle, StructType, NodeType, Action, Indexed, Supergraph, QueryConnection, NodeReference, MaterializationType, Value, User, Analytic, Condition, Spatial, Space, Entity, Script, EnumType, RelationReference, Node, IsTracked } from '@/language';
import { Temporal } from 'temporal-polyfill'; // until Temporal ships natively

/* ==== DESTACK_GENERATED_START:ENUM:3041 ==== */
export enum TriggerEventType {
  STARTED = 1,
  TRIGGERED = 2,
  STOPPED = 3,
}
/* ==== DESTACK_GENERATED_END:ENUM:3041 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:3040 ==== */
export enum TriggerType {
  EVENT = 1,
}
/* ==== DESTACK_GENERATED_END:ENUM:3040 ==== */

/* ==== DESTACK_GENERATED_START:NODE:3041 ==== */
export class TriggerEvent extends Node implements Spatial, Particle, Analytic, Indexed, Event, IsFrozen, IsTracked {
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
  type: TriggerEventType;
  get node(): Trigger | null {
      const nodePtr: NodeReference | null = this.nodePtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Trigger | null;
      }
      return null;
  }

  set node(node: Trigger) {
      this.nodePtr = node.toRef();
  }
  ;
  nodePtr: NodeReference

  constructor(options: {
    type: TriggerEventType,
    node: Trigger | NodeReference,
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
    return new NodeReference(NodeType.TRIGGER_EVENT, this.id, this.spacePtr?.id ?? null, null, this._supergraph);
  }

  get _pathKey(): string {
      return "TriggerEvent[id={this.id}]";
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
/* ==== DESTACK_GENERATED_END:NODE:3041 ==== */

/* ==== DESTACK_GENERATED_START:NODE:3040 ==== */
export class Trigger extends Node implements Spatial, Entity, IsTracked {
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
  readonly materialization: MaterializationType;
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
  type: TriggerType;
  name: string;
  event: RelationReference | null;
  where: Condition | null;
  get target(): Action | Script | Service | null {
      const nodePtr: NodeReference | null = this.targetPtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Action | Script | Service | null;
      }
      return null;
  }

  set target(node: Action | Script | Service) {
      this.targetPtr = node.toRef();
  }
  ;
  targetPtr: NodeReference
  arguments: Map<string, Value>;

  constructor(options: {
    type: TriggerType,
    name: string,
    event?: RelationReference | null,
    where?: Condition | null,
    target: Action | Script | Service | NodeReference,
    arguments?: Map<string, Value>,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: Graph | null,
    _connection?: QueryConnection | null
  }) {
    const session = options._session ?? ACTIVE_SESSION.get();
    const supergraph = options._supergraph ?? session.supergraph;
    super(options.id, options.parent, session, supergraph, options._graph, options._connection);
    this.type = options.type;
    this.name = options.name;
    this.event = options.event ?? null;
    this.where = options.where ?? null;
    this.targetPtr = options.target != null ? (options.target.metatype == StructType.NODE_REFERENCE ? (options.target as NodeReference) : (options.target as Node).toRef()) : null;
    this.arguments = options.arguments ?? new Map();
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
    return new NodeReference(NodeType.TRIGGER, this.id, this.spacePtr?.id ?? null, null, this._supergraph);
  }

  get _pathKey(): string {
      return this.name;
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
/* ==== DESTACK_GENERATED_END:NODE:3040 ==== */