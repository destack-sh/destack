import {
  Agent,
  CustomEntityDefinition,
  Entity,
  Graph,
  IsDeletable,
  IsExtensible,
  IsOrdered,
  IsRunnable,
  IsSourceable,
  IsTaggable,
  IsTracked,
  MaterializationType,
  Node,
  NodeReference,
  NodeType,
  QueryConnection,
  Script,
  Service,
  Session,
  Space,
  Spatial,
  StructType,
  Supergraph,
  Text,
  TraitType,
  User,
  Value,
} from "@/language";
import { Temporal } from "temporal-polyfill";

/* ==== DESTACK_GENERATED_START:ENUM:3020 ==== */
export enum ActionCardinality {
  UNARY = 1,
} /* ==== DESTACK_GENERATED_END:ENUM:3020 ==== */

/* ==== DESTACK_GENERATED_START:NODE:3020 ==== */
export class Action
  extends Node
  implements Spatial, Entity, IsTracked, IsDeletable, IsExtensible, IsOrdered, IsTaggable, IsRunnable, IsSourceable
{
  static metatype: NodeType = NodeType.ACTION;
  static __traits__: TraitType[] = [
    TraitType.SPATIAL,
    TraitType.TAGGABLE,
    TraitType.ENTITY,
    TraitType.TRACKED,
    TraitType.DELETABLE,
    TraitType.EXTENSIBLE,
    TraitType.ORDERED,
    TraitType.RUNNABLE,
    TraitType.SOURCEABLE,
  ];
  static __rootType__: NodeType | null = NodeType.SPACE;
  static __parentTypes__: NodeType[] = [NodeType.CUSTOM_ENTITY_DEFINITION, NodeType.SERVICE];
  static __childTypes__: NodeType[] = [NodeType.FIELD, NodeType.TAGGING];
  static __ancestorTypes__: NodeType[] = [
    NodeType.CUSTOM_ENTITY_DEFINITION,
    NodeType.SPACE,
    NodeType.SERVICE,
    NodeType.FOLDER,
  ];
  static __descendantTypes__: NodeType[] = [NodeType.FIELD, NodeType.OPTION, NodeType.TAGGING];

  readonly id: string;
  get parent(): CustomEntityDefinition | Service | null | null {
    const nodePtr: NodeReference | null = this.parentPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as CustomEntityDefinition | Service | null | null;
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
  readonly materialization: MaterializationType;
  readonly createdAt: Temporal.ZonedDateTime;
  get createdBy(): Agent | User | null | null {
    const nodePtr: NodeReference | null = this.createdByPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Agent | User | null | null;
    }
    return null;
  }
  readonly createdByPtr: NodeReference | null;
  readonly updatedAt: Temporal.ZonedDateTime;
  get updatedBy(): Agent | User | null | null {
    const nodePtr: NodeReference | null = this.updatedByPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Agent | User | null | null;
    }
    return null;
  }
  readonly updatedByPtr: NodeReference | null;
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
  readonly sourcePtr: NodeReference | null;

  constructor(options: {
    id: string;
    parent?: CustomEntityDefinition | Service | NodeReference | null;
    space?: Space | NodeReference | null;
    materialization?: MaterializationType;
    createdAt: Temporal.ZonedDateTime;
    createdBy?: Agent | User | NodeReference | null;
    updatedAt: Temporal.ZonedDateTime;
    updatedBy?: Agent | User | NodeReference | null;
    deletedAt?: Temporal.ZonedDateTime | null;
    value?: Map<string, Value>;
    orderKey?: string;
    name: string;
    cardinality?: ActionCardinality;
    text?: Text | null;
    source?: Script | NodeReference | null;
    _session?: Session | null;
    _supergraph?: Supergraph | null;
    _graph?: Graph | null;
    _connection?: QueryConnection | null;
  }) {
    super(
      // id
      options.id,
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
      options.id != null,
    );

    this.id = options.id;
    this.parentPtr =
      options.parent != null
        ? options.parent.metatype == StructType.NODE_REFERENCE
          ? (options.parent as NodeReference)
          : (options.parent as Node).toRef()
        : null;
    this.spacePtr =
      options.space != null
        ? options.space.metatype == StructType.NODE_REFERENCE
          ? (options.space as NodeReference)
          : (options.space as Node).toRef()
        : null;
    this.materialization = options.materialization ?? MaterializationType.FULL_GRAPH;
    this.createdAt = options.createdAt;
    this.createdByPtr =
      options.createdBy != null
        ? options.createdBy.metatype == StructType.NODE_REFERENCE
          ? (options.createdBy as NodeReference)
          : (options.createdBy as Node).toRef()
        : null;
    this.updatedAt = options.updatedAt;
    this.updatedByPtr =
      options.updatedBy != null
        ? options.updatedBy.metatype == StructType.NODE_REFERENCE
          ? (options.updatedBy as NodeReference)
          : (options.updatedBy as Node).toRef()
        : null;
    this.deletedAt = options.deletedAt ?? null;
    this.value = options.value ?? new Map();
    this.orderKey = options.orderKey ?? "a0";
    this.name = options.name;
    this.cardinality = options.cardinality ?? ActionCardinality.UNARY;
    this.text = options.text ?? null;
    this.sourcePtr =
      options.source != null
        ? options.source.metatype == StructType.NODE_REFERENCE
          ? (options.source as NodeReference)
          : (options.source as Node).toRef()
        : null;
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
      nodeType: NodeType.ACTION,
      id: this.id,
      spaceId: this.spacePtr?.id ?? null,
      _session: this._session,
      _supergraph: this._supergraph,
    });
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
} /* ==== DESTACK_GENERATED_END:NODE:3020 ==== */
