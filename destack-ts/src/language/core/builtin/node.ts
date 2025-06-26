import {
  activeSession,
  Aggregation,
  AggregationType,
  Condition,
  Expression,
  ExpressionIn,
  Graph,
  INTER_ORDER_TRAITS,
  IsOrdered,
  Join,
  JoinType,
  NodeReference,
  NodeType,
  PropertyDefinition,
  Query,
  QueryConnection,
  QueryType,
  RelationReference,
  Session,
  SingletonGraph,
  Sort,
  Spatial,
  Supergraph,
  TraitType,
} from "@destack/language/core";
import { NodeTypeMapping, TraitTypeMapping } from "@destack/language/mapping";
import { TRAIT_CLASS_BY_TYPE } from "@destack/language/registry";
import { getOrderKey } from "@destack/utils";
import { Casing, toCasing } from "@destack/utils/string";
import { v4 as uuid4 } from "uuid";
import { BuiltinObject, BuiltinObjectClass } from "./object";

export type NodeFilter = {
  includeDeleted?: boolean;
  includeArchived?: boolean;
};

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
    id: string | null,
    parentPtr: NodeReference | null,
    _session: Session | null,
    _supergraph: Supergraph | null,
    _graph: Graph | null,
    _connection: QueryConnection | null,
    _isNew: boolean,
    _isAttached: boolean,
  ) {
    super(_supergraph);
    this.id = id ?? uuid4();
    this.parentPtr = parentPtr;
    this._session = _session ?? activeSession();
    this._supergraph = _supergraph ?? this._session.supergraph;
    if (_graph == null) {
      _graph = new SingletonGraph(this._supergraph, this);
    } else {
      _graph.add(this);
    }
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
  addChild(child: Node, options?: { after?: Node; before?: Node }): this {
    const oldGraph = child._graph;
    const newGraph = this._graph;
    const session = this._session;
    const nodes: Node[] = [child, ...child._graph.getDescendants(child)];

    // validate parent-child relationship
    if (!child.__parentTypes__.includes(this.metatype)) {
      throw new Error(`${this} cannot parent ${child} (allowed: ${child.__parentTypes__})`);
    }
    if (oldGraph === newGraph) {
      throw new Error(`${child} is already in same graph of ${this}`);
    }
    if (oldGraph.supergraph !== this._supergraph) {
      throw new Error(`${child} is not in supergraph of ${this}`);
    }

    // assign order
    if (hasTrait(child, TraitType.ORDERED)) {
      const orderTrait = child.__traits__.find((trait) => trait in INTER_ORDER_TRAITS);
      const peerClass = orderTrait ? TRAIT_CLASS_BY_TYPE[orderTrait] : (child.constructor as NodeClass);
      const existingNodes = this._graph.getChildren(this, peerClass) as (Node & IsOrdered)[];
      if (existingNodes.length > 0) {
        const orderKey = getOrderKey(existingNodes[existingNodes.length - 1].orderKey, null);
        // @ts-expect-error(readonly)
        (child as unknown as Node & IsOrdered).orderKey = orderKey;
      }
    }

    // promote self to polygraph if needed
    if (newGraph instanceof SingletonGraph) {
      const promotedGraph = this._supergraph.promoteToPolygraph(newGraph);
      this._graph = promotedGraph;
    }

    // move to new graph
    if (nodes.length === oldGraph.size) {
      // all nodes were moved
      this._supergraph.removeGraph(oldGraph);
    } else {
      for (const node of nodes) {
        oldGraph.remove(node);
      }
    }

    // set parent reference
    (child as any).parentPtr = this.toRef();
    for (const node of nodes) {
      node._graph = this._graph;
      this._graph.add(node);
    }

    // assign space for spatial nodes
    if (hasTrait(child, TraitType.SPATIAL)) {
      let spacePtr: NodeReference | null = null;
      if (hasTrait(this, TraitType.SPATIAL) && (this as unknown as Node & Spatial).spacePtr != null) {
        spacePtr = (this as unknown as Node & Spatial).spacePtr;
      } else if (this.metatype === NodeType.SPACE) {
        spacePtr = this.toRef();
      }
      if (spacePtr) {
        for (const node of nodes) {
          if (hasTrait(node, TraitType.SPATIAL)) {
            // @ts-expect-error(readonly)
            (node as unknown as Node & Spatial).spacePtr = spacePtr;
          }
        }
      }
    }

    // create new nodes if needed
    if (child._isNew && this._isAttached) {
      for (const node of nodes) {
        node._ref = null; // invalidate cached ref
        session.create(node);
      }
    }

    return this;
  }

  /** Append multiple children to this Node. */
  addChildren(children: Node[], options?: { after?: Node; before?: Node }): this {
    for (const child of children) {
      this.addChild(child, options);
    }
    return this;
  }

  /** Remove a child from this Node. */
  removeChild(child: Node): void {
    throw new Error("not implemented");
  }

  /** Get the children of this Node. */
  getChildren(): Node[];
  getChildren<N extends Node>(classOrTrait: NodeClass<N>, options?: NodeFilter): N[];
  getChildren<T extends TraitType>(
    classOrTrait: TraitClass<any, T>,
    options?: NodeFilter,
  ): (Node & TraitTypeMapping[T])[];
  getChildren(classOrTrait?: NodeClass | TraitClass, options?: NodeFilter): Node[];
  getChildren(classOrTrait?: NodeClass | TraitClass, options?: NodeFilter): Node[] {
    return this._graph.getChildren(this, classOrTrait);
  }

  /** Get a specific child of this Node by name. */
  getChild<N extends Node>(classOrTrait: NodeClass<N>, name: string): N | null;
  getChild<T extends TraitType>(classOrTrait: TraitClass<any, T>, name: string): (Node & TraitTypeMapping[T]) | null;
  getChild(classOrTrait: NodeClass | TraitClass, name: string): Node | null;
  getChild(classOrTrait: NodeClass | TraitClass, name: string, options?: NodeFilter): Node | null {
    const children = this._graph.getChildren(this, classOrTrait);
    for (const child of children) {
      if ((child as any).name === name) {
        return child;
      }
    }
    return null;
  }

  /** Get a specific child of this Node by name, or raises an error if not found. */
  child<N extends Node>(classOrTrait: NodeClass<N>, name: string): N;
  child<T extends TraitType>(classOrTrait: TraitClass<any, T>, name: string): Node & TraitTypeMapping[T];
  child(classOrTrait: NodeClass | TraitClass, name: string): Node;
  child(classOrTrait: NodeClass | TraitClass, name: string): Node {
    const child = this.getChild(classOrTrait, name);
    if (child === null) {
      throw new Error(`no child ${name} of ${this}`);
    }
    return child;
  }

  /** Get the descendants of this Node. */
  getDescendants(): Node[];
  getDescendants<N extends Node>(classOrTrait: NodeClass<N>): N[];
  getDescendants<T extends TraitType>(classOrTrait: TraitClass<any, T>): (Node & TraitTypeMapping[T])[];
  getDescendants(classOrTrait?: NodeClass | TraitClass): Node[];
  getDescendants(classOrTrait?: NodeClass | TraitClass): Node[] {
    return this._graph.getDescendants(this, classOrTrait);
  }

  /** Make a get Query for this Node/Trait type. */
  static get(
    options: WithSubqueries<{
      where?: Condition;
      name?: string;
      join?: Join;
    }>,
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
    options: WithSubqueries<{
      where?: Condition;
      name?: string;
      join?: Join;
      having?: Condition;
      groupBy?: ExpressionIn[];
      sort?: Sort[];
      limit?: number;
      offset?: number;
    }>,
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
    options: WithSubqueries<{
      where?: Condition;
      name?: string;
      join?: Join;
    }>,
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
    options: WithSubqueries<{
      where?: Condition;
      name?: string;
      join?: Join;
      groupBy?: ExpressionIn[];
      having?: Condition;
      sort?: Sort[];
    }>,
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
    options: WithSubqueries<{
      expression: ExpressionIn;
      where?: Condition;
      name?: string;
      join?: Join;
      groupBy?: ExpressionIn[];
      having?: Condition;
      sort?: Sort[];
    }>,
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
    options: WithSubqueries<{
      expression: ExpressionIn;
      where?: Condition;
      name?: string;
      join?: Join;
      groupBy?: ExpressionIn[];
      having?: Condition;
      sort?: Sort[];
    }>,
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
    options: WithSubqueries<{
      expression: ExpressionIn;
      where?: Condition;
      name?: string;
      join?: Join;
      groupBy?: ExpressionIn[];
      having?: Condition;
      sort?: Sort[];
    }>,
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

/** A Node class. */
export type NodeClass<N extends Node = Node> = { new (...args: any[]): N } & BuiltinObjectClass<any, any> & {
    metatype: NodeType;
    __traits__: TraitType[];
    __rootType__: NodeType | null;
    __parentTypes__: NodeType[];
    __childTypes__: NodeType[];
    __ancestorTypes__: NodeType[];
    __descendantTypes__: NodeType[];
  };

/** Internal base class for Trait companion objects.*/
export class TraitClass<N = any, T extends TraitType = TraitType> {
  readonly metatype: T;
  readonly __traits__: TraitType[];
  readonly __properties__: Record<string, PropertyDefinition>;
  readonly __propertiesById__: Record<number, PropertyDefinition>;

  constructor(metatype: any) {
    this.metatype = metatype;
    this.__traits__ = [];
    this.__properties__ = {};
    this.__propertiesById__ = {};
  }

  /** Get a PropertyDefinition or CustomProperty by name. */
  property(name: string): PropertyDefinition {
    const prop = this.__properties__[name];
    if (!prop) {
      throw new Error(`Property ${name} not found on ${this.constructor.name}`);
    }
    return prop;
  }

  /** Make a get Query for this Node/Trait type. */
  get(
    options: WithSubqueries<{
      where?: Condition;
      name?: string;
      join?: Join;
    }>,
  ): Query {
    const { where, name, join, ...subqueries } = options;
    const query = new Query({
      type: QueryType.NODE,
      relation: RelationReference.of(this as unknown as NodeClass),
      name: name ?? toCasing(TraitType[this.metatype], Casing.CAMEL),
      join,
      where,
      subqueries: toSubqueries(subqueries),
    });
    return query;
  }

  /** Make a search Query for this Node/Trait type. */
  search(
    options: WithSubqueries<{
      where?: Condition;
      name?: string;
      join?: Join;
      having?: Condition;
      groupBy?: ExpressionIn[];
      sort?: Sort[];
      limit?: number;
      offset?: number;
    }>,
  ): Query {
    const { where, name, join, having, groupBy, sort, limit, offset, ...subqueries } = options;
    const query = new Query({
      type: groupBy ? QueryType.GROUPED_NODE : QueryType.NODE,
      relation: RelationReference.of(this as unknown as NodeClass),
      name: name ?? toCasing(TraitType[this.metatype], Casing.CAMEL),
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
  exists(
    options: WithSubqueries<{
      where?: Condition;
      name?: string;
      join?: Join;
    }>,
  ): Query {
    const { where, name, join, ...subqueries } = options;
    const query = new Query({
      type: QueryType.SCALAR,
      relation: RelationReference.of(this as unknown as NodeClass),
      name: name ?? toCasing(TraitType[this.metatype], Casing.CAMEL),
      join,
      where,
      aggregation: Aggregation.of(AggregationType.EXISTS),
      subqueries: toSubqueries(subqueries),
    });
    return query;
  }

  /** Make a count Query for this Node/Trait type. */
  count(
    options: WithSubqueries<{
      where?: Condition;
      name?: string;
      join?: Join;
      groupBy?: ExpressionIn[];
      having?: Condition;
      sort?: Sort[];
    }>,
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
  min(
    options: WithSubqueries<{
      expression: ExpressionIn;
      where?: Condition;
      name?: string;
      join?: Join;
      groupBy?: ExpressionIn[];
      having?: Condition;
      sort?: Sort[];
    }>,
  ): Query {
    const { expression, where, name, join, groupBy, having, sort, ...subqueries } = options;
    const query = new Query({
      type: groupBy ? QueryType.GROUPED_SCALAR : QueryType.SCALAR,
      relation: RelationReference.of(this as unknown as NodeClass),
      name: name ?? toCasing(TraitType[this.metatype], Casing.CAMEL),
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
  max(
    options: WithSubqueries<{
      expression: ExpressionIn;
      where?: Condition;
      name?: string;
      join?: Join;
      groupBy?: ExpressionIn[];
      having?: Condition;
      sort?: Sort[];
    }>,
  ): Query {
    const { expression, where, name, join, groupBy, having, sort, ...subqueries } = options;
    const query = new Query({
      type: groupBy ? QueryType.GROUPED_SCALAR : QueryType.SCALAR,
      relation: RelationReference.of(this as unknown as NodeClass),
      name: name ?? toCasing(TraitType[this.metatype], Casing.CAMEL),
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
  sum(
    options: WithSubqueries<{
      expression: ExpressionIn;
      where?: Condition;
      name?: string;
      join?: Join;
      groupBy?: ExpressionIn[];
      having?: Condition;
      sort?: Sort[];
    }>,
  ): Query {
    const { expression, where, name, join, groupBy, having, sort, ...subqueries } = options;
    const query = new Query({
      type: groupBy ? QueryType.GROUPED_SCALAR : QueryType.SCALAR,
      relation: RelationReference.of(this as unknown as NodeClass),
      name: name ?? toCasing(TraitType[this.metatype], Casing.CAMEL),
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

/** Check if a value is a Node of a specific type. */
export function isNode<T extends NodeType>(value: any, nodeType?: T): value is NodeTypeMapping[T] {
  return value instanceof Node && (nodeType === undefined || value.metatype === nodeType);
}

/** Check if a value is a Node with a specific trait. */
export function hasTrait<T extends TraitType>(value: any, traitType: T): value is Node & TraitTypeMapping[T] {
  return value instanceof Node && value.__traits__.includes(traitType);
}

export type WithSubqueries<T, Q = Query> = T & {
  [K: string]: Q | T[keyof T] | undefined;
};

/** Convert a WithSubqueries object to an array of Queries. */
export function toSubqueries(subqueries: WithSubqueries<Record<string, any>>): Query[] {
  const queries: Query[] = [];
  for (const [name, subquery] of Object.entries(subqueries)) {
    if (!(subquery instanceof Query)) {
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
