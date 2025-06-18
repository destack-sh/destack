import { Graph, IsTaggable, IsDeletable, MaterializationType, Spatial, StructFrozen, Struct, EnumType, Field, Script, IsTracked, IsSourceable, Entity, QueryConnection, NodeReference, Node, IsOrdered, NodeType, Session, CustomStructDefinition, Agent, Space, User, StructType, Icon, Supergraph, BuiltinObject } from '@/language';
import { Temporal } from 'temporal-polyfill'; // until Temporal ships natively

/* ==== DESTACK_GENERATED_START:NODE:2530 ==== */
export class Option extends Node implements Spatial, Entity, IsTracked, IsDeletable, IsOrdered, IsTaggable, IsSourceable {
  readonly id: string;
  get parent(): CustomStructDefinition | Field | null | null {
      const nodePtr: NodeReference | null = this.parentPtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as CustomStructDefinition | Field | null | null;
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
  readonly orderKey: string;
  name: string;
  icon: Icon | null;
  get source(): Script | null | null {
      const nodePtr: NodeReference | null = this.sourcePtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Script | null | null;
      }
      return null;
  }
  ;
  sourcePtr: NodeReference | null

  constructor(
    id: string,
    parentPtr: NodeReference | null,
    spacePtr: NodeReference | null,
    materialization: MaterializationType,
    createdAt: Temporal.ZonedDateTime,
    createdByPtr: NodeReference | null,
    updatedAt: Temporal.ZonedDateTime,
    updatedByPtr: NodeReference | null,
    deletedAt: Temporal.ZonedDateTime | null,
    orderKey: string,
    name: string,
    icon: Icon | null,
    sourcePtr: NodeReference | null,
    _session: Session,
    _supergraph: Supergraph,
    _graph: Graph,
    _connection: QueryConnection | null
  ) {
    super(id, _session, _supergraph, _graph, _connection);
    this.id = id;
    this.parentPtr = parentPtr;
    this.spacePtr = spacePtr;
    this.materialization = materialization;
    this.createdAt = createdAt;
    this.createdByPtr = createdByPtr;
    this.updatedAt = updatedAt;
    this.updatedByPtr = updatedByPtr;
    this.deletedAt = deletedAt;
    this.orderKey = orderKey;
    this.name = name;
    this.icon = icon;
    this.sourcePtr = sourcePtr;
  }


  static create(options: {
    name: string,
    icon?: Icon | null
  }): Option {

    return new Option(

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
    return new NodeReference(NodeType.OPTION, this.id, this.spacePtr?.id ?? null, null, this._supergraph);
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
/* ==== DESTACK_GENERATED_END:NODE:2530 ==== */