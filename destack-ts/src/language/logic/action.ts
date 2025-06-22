import { CustomEntityDefinition, Agent, IsTaggable, BuiltinObject, ACTIVE_SESSION, Session, StructFrozen, IsOrdered, Graph, Service, Struct, activeSession, StructType, NodeType, Supergraph, QueryConnection, NodeReference, MaterializationType, Value, IsRunnable, User, IsExtensible, Spatial, IsSourceable, Space, Text, IsDeletable, Entity, Script, EnumType, Node, IsTracked } from '@/language';
import { Temporal } from 'temporal-polyfill'; // until Temporal ships natively

/* ==== DESTACK_GENERATED_START:ENUM:3020 ==== */
export enum ActionCardinality {
  UNARY = 1,
}
/* ==== DESTACK_GENERATED_END:ENUM:3020 ==== */

/* ==== DESTACK_GENERATED_START:NODE:3020 ==== */
export class Action extends Node implements Spatial, Entity, IsTracked, IsDeletable, IsExtensible, IsOrdered, IsTaggable, IsRunnable, IsSourceable {
  readonly id: string;
  get parent(): CustomEntityDefinition | Service | null | null {
      const nodePtr: NodeReference | null = this.parentPtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as CustomEntityDefinition | Service | null | null;
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
  readonly deletedAt: Temporal.ZonedDateTime | null;
  value: Map<string, Value>;
  readonly orderKey: string;
  name: string;
  cardinality: ActionCardinality;
  text: Text | null;
  get source(): Script | null | null {
      const nodePtr: NodeReference | null = this.sourcePtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Script | null | null;
      }
      return null;
  }
  ;
  sourcePtr: NodeReference | null

  constructor(options: {
    value?: Map<string, Value>,
    name: string,
    cardinality?: ActionCardinality,
    text?: Text | null,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: Graph | null,
    _connection?: QueryConnection | null
  }) {
    const session = options._session ?? ACTIVE_SESSION.get();
    const supergraph = options._supergraph ?? session.supergraph;
    super(options.id, options.parent, session, supergraph, options._graph, options._connection);
    this.value = options.value ?? new Map();
    this.name = options.name;
    this.cardinality = options.cardinality ?? ActionCardinality.UNARY;
    this.text = options.text ?? null;
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
    return new NodeReference(NodeType.ACTION, this.id, this.spacePtr?.id ?? null, null, this._supergraph);
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
/* ==== DESTACK_GENERATED_END:NODE:3020 ==== */