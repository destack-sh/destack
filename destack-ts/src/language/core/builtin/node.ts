import {
  activeSession,
  Aggregation,
  AggregationType,
  Condition,
  Expression,
  ExpressionIn,
  Graph,
  Join,
  JoinType,
  NodeReference,
  NodeType,
  Query,
  QueryConnection,
  QueryType,
  RelationReference,
  Session,
  Sort,
  Supergraph,
  TraitType,
} from "@/language";
import { NodeTypeMapping, TraitTypeMapping } from "@/language/registry";
import { Casing, toCasing } from "@/utils/string";
import { BuiltinObject } from "./object";

/** A Node is a collection of properties with an identity. */
export abstract class Node extends BuiltinObject {
  static readonly __isNode__: boolean = true;
  static readonly metatype: NodeType;
  static readonly __traits__: TraitType[];
  static readonly __rootType__: NodeType | null;
  static readonly __parentTypes__: NodeType[];
  static readonly __childTypes__: NodeType[];
  static readonly __ancestorTypes__: NodeType[];
  static readonly __descendantTypes__: NodeType[];

  readonly id: string;
  get parent(): Node | null {
    if (this.parentPtr === null) {
      return null;
    }
    return this._supergraph.get(this.parentPtr.id);
  }
  readonly parentPtr: NodeReference | null;

  // runtime
  _session: Session;
  _supergraph: Supergraph;
  _graph: Graph;
  _connection: QueryConnection | null;
  _hash: string | null;
  _ref: NodeReference | null;
  _isNew: boolean;
  _isAttached: boolean;
  _dirty: Record<string, any> | null;

  constructor(
    id: string,
    parentPtr: NodeReference | null,
    _session: Session | null,
    _supergraph: Supergraph | null,
    _graph: Graph | null,
    _connection: QueryConnection | null,
    _isNew: boolean,
    _isAttached: boolean,
  ) {
    super(_supergraph);
    this.id = id;
    this.parentPtr = parentPtr;
    this._session = _session ?? activeSession();
    this._supergraph = _supergraph ?? this._session.supergraph;
    this._graph = _graph;
    this._connection = _connection;
    this._hash = this.id;
    this._ref = null;
    this._dirty = null;
    this._isNew = _isNew;
    this._isAttached = _isAttached;
  }

  get metatype(): NodeType {
    return (this.constructor as typeof Node).metatype;
  }

  get __traits__(): TraitType[] {
    return (this.constructor as typeof Node).__traits__;
  }

  get __rootType__(): NodeType | null {
    return (this.constructor as typeof Node).__rootType__;
  }

  get __parentTypes__(): NodeType[] {
    return (this.constructor as typeof Node).__parentTypes__;
  }

  get __childTypes__(): NodeType[] {
    return (this.constructor as typeof Node).__childTypes__;
  }

  get __ancestorTypes__(): NodeType[] {
    return (this.constructor as typeof Node).__ancestorTypes__;
  }

  get __descendantTypes__(): NodeType[] {
    return (this.constructor as typeof Node).__descendantTypes__;
  }

  get _pathKey(): string {
    throw new Error("not implemented");
  }

  get path(): string {
    throw new Error("not implemented");
  }

  __toRef__(): NodeReference {
    throw new Error("not implemented");
  }

  toRef(): NodeReference {
    if (this._ref === null) {
      this._ref = this.__toRef__();
    }
    return this._ref;
  }

  erase(): void {
    this._session.erase(this);
  }

  moveTo(parent: Node): void {
    throw new Error("not implemented");
  }

  /** Append a child to this Node. */
  addChild(child: Node, after?: Node, before?: Node): void {
    throw new Error("not implemented");
  }

  /** Append multiple children to this Node. */
  addChildren(children: Node[], after?: Node, before?: Node): void {
    throw new Error("not implemented");
  }

  /** Remove a child from this Node. */
  removeChild(child: Node): void {
    throw new Error("not implemented");
  }

  /** Get the children of this Node. */
  getChildren(): Node[];
  getChildren<T extends NodeType>(options: { nodeType: T }): NodeTypeMapping[T][];
  getChildren<T extends TraitType>(options: { traitType: T }): (Node & TraitTypeMapping[T])[];
  getChildren<N extends Node>(options: { nodeClass: NodeClass }): N[];
  getChildren<N extends Node = Node>(options?: {
    nodeType?: NodeType;
    traitType?: TraitType;
    nodeClass?: NodeClass;
  }): N[] {
    return this._graph.getChildren(this, options);
  }

  /** Get a specific child of this Node by name. */
  getChild<T extends NodeType>(options: { nodeType: T; name: string }): NodeTypeMapping[T] | null;
  getChild<T extends TraitType>(options: { traitType: T; name: string }): (Node & TraitTypeMapping[T]) | null;
  getChild<N extends Node>(options: { nodeClass: NodeClass; name: string }): N | null;
  getChild<N extends Node>(options: {
    nodeType?: NodeType;
    traitType?: TraitType;
    nodeClass?: NodeClass;
    name?: string;
  }): N | null;
  getChild<N extends Node = Node>(options: {
    nodeType?: NodeType;
    traitType?: TraitType;
    nodeClass?: NodeClass;
    name?: string;
  }): N | null {
    const children = this._graph.getChildren(this, options);
    if (options.name === undefined) {
      return children[0] as N | null;
    }
    for (const child of children) {
      if ((child as any).name === options.name) {
        return child as N;
      }
    }
    return null;
  }

  /** Get a specific child of this Node by name, or raises an error if not found. */
  child<T extends NodeType>(options: { nodeType: T; name: string }): NodeTypeMapping[T];
  child<T extends TraitType>(options: { traitType: T; name: string }): Node & TraitTypeMapping[T];
  child<N extends Node>(options: { nodeClass: NodeClass; name: string }): N;
  child<N extends Node>(options: {
    nodeType?: NodeType;
    traitType?: TraitType;
    nodeClass?: NodeClass;
    name?: string;
  }): N;
  child<N extends Node = Node>(options: {
    nodeType?: NodeType;
    traitType?: TraitType;
    nodeClass?: NodeClass;
    name?: string;
  }): N {
    const child = this.getChild(options);
    if (child === null) {
      throw new Error(`no child ${options.name} of ${this}`);
    }
    return child as N;
  }

  /** Get the descendants of this Node. */
  getDescendants(): Node[];
  getDescendants<T extends NodeType>(options: { nodeType: T }): NodeTypeMapping[T][];
  getDescendants<T extends TraitType>(options: { traitType: T }): (Node & TraitTypeMapping[T])[];
  getDescendants<N extends Node>(options: { nodeClass: NodeClass }): N[];
  getDescendants<N extends Node = Node>(options?: {
    nodeType?: NodeType;
    traitType?: TraitType;
    nodeClass?: NodeClass;
  }): N[] {
    return this._graph.getDescendants(this, options);
  }

  /** Make a get Query for this Node/Trait type. */
  static get(
    options: {
      where?: Condition;
      name?: string;
      join?: Join;
    } & SubqueriesIn,
  ): Query {
    const { where, name, join, ...subqueries } = options;
    const query = new Query({
      type: QueryType.NODE,
      relation: RelationReference.of(this as unknown as NodeClass),
      name: name ?? toCasing(NodeType[this.metatype], Casing.CAMEL),
      join,
      where,
      subqueries: toSubqueries(subqueries),
    });
    return query;
  }

  /** Make a search Query for this Node/Trait type. */
  static search(
    options: {
      where?: Condition;
      name?: string;
      join?: Join;
      having?: Condition;
      groupBy?: ExpressionIn[];
      sort?: Sort[];
      limit?: number;
      offset?: number;
    } & SubqueriesIn,
  ): Query {
    const { where, name, join, having, groupBy, sort, limit, offset, ...subqueries } = options;
    const query = new Query({
      type: groupBy ? QueryType.GROUPED_NODE : QueryType.NODE,
      relation: RelationReference.of(this as unknown as NodeClass),
      name: name ?? toCasing(NodeType[this.metatype], Casing.CAMEL),
      join,
      where,
      having,
      groupBy: groupBy?.map(Expression.of),
      sort,
      limit,
      offset,
      subqueries: toSubqueries(subqueries),
    });
    return query;
  }

  /** Make an exists Query for this Node/Trait type. */
  static exists(
    options: {
      where?: Condition;
      name?: string;
      join?: Join;
    } & SubqueriesIn,
  ): Query {
    const { where, name, join, ...subqueries } = options;
    const query = new Query({
      type: QueryType.SCALAR,
      relation: RelationReference.of(this as unknown as NodeClass),
      name: name ?? toCasing(NodeType[this.metatype], Casing.CAMEL),
      join,
      where,
      aggregation: Aggregation.of(AggregationType.EXISTS),
      subqueries: toSubqueries(subqueries),
    });
    return query;
  }

  /** Make a count Query for this Node/Trait type. */
  static count(
    options: {
      where?: Condition;
      name?: string;
      join?: Join;
      groupBy?: ExpressionIn[];
      having?: Condition;
      sort?: Sort[];
    } & SubqueriesIn,
  ): Query {
    const { where, name, join, groupBy, having, sort, ...subqueries } = options;
    const query = new Query({
      type: groupBy ? QueryType.GROUPED_SCALAR : QueryType.SCALAR,
      relation: RelationReference.of(this as unknown as NodeClass),
      name: name ?? toCasing(NodeType[this.metatype], Casing.CAMEL),
      join,
      where,
      having,
      groupBy: groupBy?.map(Expression.of),
      aggregation: Aggregation.of(AggregationType.COUNT),
      sort,
      subqueries: toSubqueries(subqueries),
    });
    return query;
  }

  /** Make a min Query for this Node/Trait type. */
  static min(
    options: {
      expression: ExpressionIn;
      where?: Condition;
      name?: string;
      join?: Join;
      groupBy?: ExpressionIn[];
      having?: Condition;
      sort?: Sort[];
    } & SubqueriesIn,
  ): Query {
    const { expression, where, name, join, groupBy, having, sort, ...subqueries } = options;
    const query = new Query({
      type: groupBy ? QueryType.GROUPED_SCALAR : QueryType.SCALAR,
      relation: RelationReference.of(this as unknown as NodeClass),
      name: name ?? toCasing(NodeType[this.metatype], Casing.CAMEL),
      join,
      where,
      having,
      groupBy: groupBy?.map(Expression.of),
      aggregation: Aggregation.of(AggregationType.MIN, Expression.of(expression)),
      sort,
      subqueries: toSubqueries(subqueries),
    });
    return query;
  }

  /** Make a max Query for this Node/Trait type. */
  static max(
    options: {
      expression: ExpressionIn;
      where?: Condition;
      name?: string;
      join?: Join;
      groupBy?: ExpressionIn[];
      having?: Condition;
      sort?: Sort[];
    } & SubqueriesIn,
  ): Query {
    const { expression, where, name, join, groupBy, having, sort, ...subqueries } = options;
    const query = new Query({
      type: groupBy ? QueryType.GROUPED_SCALAR : QueryType.SCALAR,
      relation: RelationReference.of(this as unknown as NodeClass),
      name: name ?? toCasing(NodeType[this.metatype], Casing.CAMEL),
      join,
      where,
      having,
      groupBy: groupBy?.map(Expression.of),
      aggregation: Aggregation.of(AggregationType.MAX, Expression.of(expression)),
      sort,
      subqueries: toSubqueries(subqueries),
    });
    return query;
  }

  /** Make a sum Query for this Node/Trait type. */
  static sum(
    options: {
      expression: ExpressionIn;
      where?: Condition;
      name?: string;
      join?: Join;
      groupBy?: ExpressionIn[];
      having?: Condition;
      sort?: Sort[];
    } & SubqueriesIn,
  ): Query {
    const { expression, where, name, join, groupBy, having, sort, ...subqueries } = options;
    const query = new Query({
      type: groupBy ? QueryType.GROUPED_SCALAR : QueryType.SCALAR,
      relation: RelationReference.of(this as unknown as NodeClass),
      name: name ?? toCasing(NodeType[this.metatype], Casing.CAMEL),
      join,
      where,
      having,
      groupBy: groupBy?.map(Expression.of),
      aggregation: Aggregation.of(AggregationType.SUM, Expression.of(expression)),
      sort,
      subqueries: toSubqueries(subqueries),
    });
    return query;
  }
}

/** A Node constructor. */
export type NodeClass = { new (...args: any[]): Node } & {
  metatype: NodeType;
  __traits__: TraitType[];
  __rootType__: NodeType | null;
  __parentTypes__: NodeType[];
  __childTypes__: NodeType[];
  __ancestorTypes__: NodeType[];
  __descendantTypes__: NodeType[];
};

/** Check if a value is a Node of a specific type. */
export function isNode<T extends NodeType>(value: any, nodeType?: T): value is NodeTypeMapping[T] {
  return value instanceof Node && (nodeType === undefined || value.metatype === nodeType);
}

/** Check if a value is a Node with a specific trait. */
export function isNodeWithTrait<T extends TraitType>(value: any, traitType: T): value is Node & TraitTypeMapping[T] {
  return value instanceof Node && value.__traits__.includes(traitType);
}

type SubqueriesIn = Record<string, Query | undefined>;

function toSubqueries(subqueries: SubqueriesIn): Query[] {
  const queries: Query[] = [];
  for (const [name, subquery] of Object.entries(subqueries)) {
    if (subquery === undefined) {
      continue;
    }
    if (subquery.join === undefined) {
      const join = new Join({ type: JoinType.CHILD });
      // @ts-expect-error(readonly)
      subquery.join = join;
    }
    // @ts-expect-error(readonly)
    subquery.name = name;
    subquery._invalidateFrozenCache();
    queries.push(subquery);
  }
  return queries;
}
