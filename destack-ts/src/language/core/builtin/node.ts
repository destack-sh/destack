import type {
  Aggregation,
  Condition,
  Expression,
  ExpressionIn,
  Join,
  NodeDefinition,
  NodeDefinitionReference,
  NodeReference,
  Query,
  Sort,
} from "@destack/language";
import {
  GraphDomain,
  NodeType,
  ObjectKind,
  StructType,
  TraitType,
} from "@destack/language/core/builtin/common";
import { activeSession } from "@destack/language/core/builtin/const";
import { BuiltinObject, BuiltinObjectClass } from "@destack/language/core/builtin/object";
import { isStruct } from "@destack/language/core/builtin/struct";
import { AggregationType, JoinType, QueryType } from "@destack/language/core/common/query";
import { Session } from "@destack/language/core/runtime";
import type { NodeTypeMapping, TraitTypeMapping } from "@destack/language/mapping";
import {
  NODE_CLASS_BY_TYPE,
  registerNodeClass,
  STRUCT_CLASS_BY_TYPE,
} from "@destack/language/registry";
import { Casing, toCasing } from "@destack/utils/string";
import { uuid4, uuid7 } from "@destack/utils/uuid";

/** A Node is a collection of properties with an identity. */
export abstract class Node extends BuiltinObject {
  static readonly metatype: NodeType = NodeType.NODE;
  static readonly __kind__: ObjectKind = ObjectKind.NODE;
  static readonly __isNode__: boolean = true;
  static readonly __definition__: NodeDefinition;
  static readonly __definitionReference: NodeDefinitionReference;

  readonly id: string;
  readonly parentPtr: NodeReference | null = null;

  /* The Session this Node is in. */
  _session: Session;
  /* The hash of this Node. */
  _hash: string | null;
  /* The cached reference to this Node. */
  _ref: NodeReference | null;
  /* Whether this Node is new. */
  _isNew: boolean;

  constructor(
    id: string | null,
    parentPtr: NodeReference | null,
    _session: Session | null,
    _isNew: boolean,
  ) {
    super(_session);
    const isEvent = this.__definition__.domain == GraphDomain.EVENT;
    this.id = id ?? (isEvent ? uuid7() : uuid4());
    this.parentPtr = parentPtr; // is assigned again in subclasses but needed for adding to Graph
    this._session = _session ?? activeSession();
    this._hash = this.id;
    this._ref = null;
    this._isNew = _isNew;
  }

  get metatype(): NodeType {
    return (this.constructor as typeof Node).metatype;
  }

  get __definition__(): NodeDefinition {
    return (this.constructor as typeof Node).__definition__;
  }

  get __traits__(): readonly TraitType[] {
    return (this.constructor as typeof Node).__definition__.traits;
  }

  get __inherits__(): readonly NodeType[] {
    return (this.constructor as typeof Node).__definition__.inherits;
  }

  get __extendedBy__(): readonly NodeType[] {
    return (this.constructor as typeof Node).__definition__.extendedBy;
  }

  get __parentTypes__(): readonly NodeType[] {
    return (this.constructor as typeof Node).__definition__.parentTypes;
  }

  get __childTypes__(): readonly NodeType[] {
    return (this.constructor as typeof Node).__definition__.childTypes;
  }

  get __ancestorTypes__(): readonly NodeType[] {
    return (this.constructor as typeof Node).__definition__.ancestorTypes;
  }

  get __descendantTypes__(): readonly NodeType[] {
    return (this.constructor as typeof Node).__definition__.descendantTypes;
  }

  get isRoot(): boolean {
    return this.metatype == NodeType.SPACE;
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

  /** Make a get Query for this Node/Trait type. */
  static get<T extends Node = Node>(
    this: NodeClass<T>,
    options?: WithSubqueries<{
      where?: Condition;
      name?: string;
      join?: Join;
    }>,
  ): Query<T> {
    if (this.__definition__.domain == null) {
      throw new Error(`${this.__definition__.name} has no store domain`);
    }
    const _Query = STRUCT_CLASS_BY_TYPE[StructType.QUERY] as typeof Query;
    const _NodeDefinitionReference = STRUCT_CLASS_BY_TYPE[
      StructType.NODE_DEFINITION_REFERENCE
    ] as typeof NodeDefinitionReference;
    const { where, name, join, ...subqueries } = options ?? {};
    const query = new _Query<T>({
      type: QueryType.NODE,
      domain: this.__definition__.domain,
      definition: _NodeDefinitionReference.of(this as unknown as NodeClass),
      name: name ?? toCasing(NodeType[this.metatype].toLocaleLowerCase(), Casing.CAMEL),
      join,
      where,
      subqueries: toSubqueries(subqueries),
    });
    return query;
  }

  /** Make a search Query for this Node/Trait type. */
  static search<T extends Node = Node>(
    this: NodeClass<T>,
    options?: WithSubqueries<{
      where?: Condition;
      name?: string;
      join?: Join;
      having?: Condition;
      groupBy?: ExpressionIn[];
      sort?: Sort[];
      limit?: number;
      offset?: number;
    }>,
  ): Query<T> {
    if (this.__definition__.domain == null) {
      throw new Error(`${this.__definition__.name} has no store domain`);
    }
    const { where, name, join, having, groupBy, sort, limit, offset, ...subqueries } =
      options ?? {};
    const _Query = STRUCT_CLASS_BY_TYPE[StructType.QUERY] as typeof Query;
    const _NodeDefinitionReference = STRUCT_CLASS_BY_TYPE[
      StructType.NODE_DEFINITION_REFERENCE
    ] as typeof NodeDefinitionReference;
    const _Expression = STRUCT_CLASS_BY_TYPE[StructType.EXPRESSION] as typeof Expression;
    const query = new _Query<T>({
      type: groupBy ? QueryType.GROUPED_NODE : QueryType.NODE,
      domain: this.__definition__.domain,
      definition: _NodeDefinitionReference.of(this as unknown as NodeClass),
      name: name ?? toCasing(NodeType[this.metatype].toLocaleLowerCase(), Casing.CAMEL),
      join,
      where,
      having,
      groupBy: groupBy?.map(_Expression.of),
      sort,
      limit,
      offset,
      subqueries: toSubqueries(subqueries),
    });
    return query;
  }

  /** Make an exists Query for this Node/Trait type. */
  static exists<T extends Node = Node>(
    this: NodeClass<T>,
    options?: WithSubqueries<{
      where?: Condition;
      name?: string;
      join?: Join;
    }>,
  ): Query<T> {
    if (this.__definition__.domain == null) {
      throw new Error(`${this.__definition__.name} has no store domain`);
    }
    const { where, name, join, ...subqueries } = options ?? {};
    const _Query = STRUCT_CLASS_BY_TYPE[StructType.QUERY] as typeof Query;
    const _NodeDefinitionReference = STRUCT_CLASS_BY_TYPE[
      StructType.NODE_DEFINITION_REFERENCE
    ] as typeof NodeDefinitionReference;
    const _Aggregation = STRUCT_CLASS_BY_TYPE[StructType.AGGREGATION] as typeof Aggregation;
    const query = new _Query<T>({
      type: QueryType.SCALAR,
      domain: this.__definition__.domain,
      definition: _NodeDefinitionReference.of(this as unknown as NodeClass),
      name: name ?? toCasing(NodeType[this.metatype].toLocaleLowerCase(), Casing.CAMEL),
      join,
      where,
      aggregation: _Aggregation.of(AggregationType.EXISTS),
      subqueries: toSubqueries(subqueries),
    });
    return query;
  }

  /** Make a count Query for this Node/Trait type. */
  static count<T extends Node = Node>(
    this: NodeClass<T>,
    options?: WithSubqueries<{
      where?: Condition;
      name?: string;
      join?: Join;
      groupBy?: ExpressionIn[];
      having?: Condition;
      sort?: Sort[];
    }>,
  ): Query<T> {
    if (this.__definition__.domain == null) {
      throw new Error(`${this.__definition__.name} has no store domain`);
    }
    const { where, name, join, groupBy, having, sort, ...subqueries } = options ?? {};
    const _Query = STRUCT_CLASS_BY_TYPE[StructType.QUERY] as typeof Query;
    const _NodeDefinitionReference = STRUCT_CLASS_BY_TYPE[
      StructType.NODE_DEFINITION_REFERENCE
    ] as typeof NodeDefinitionReference;
    const _Expression = STRUCT_CLASS_BY_TYPE[StructType.EXPRESSION] as typeof Expression;
    const _Aggregation = STRUCT_CLASS_BY_TYPE[StructType.AGGREGATION] as typeof Aggregation;
    const query = new _Query<T>({
      type: groupBy ? QueryType.GROUPED_SCALAR : QueryType.SCALAR,
      domain: this.__definition__.domain,
      definition: _NodeDefinitionReference.of(this as unknown as NodeClass),
      name: name ?? toCasing(NodeType[this.metatype], Casing.CAMEL),
      join,
      where,
      having,
      groupBy: groupBy?.map(_Expression.of),
      aggregation: _Aggregation.of(AggregationType.COUNT),
      sort,
      subqueries: toSubqueries(subqueries),
    });
    return query;
  }

  /** Make a min Query for this Node/Trait type. */
  static min<T extends Node = Node>(
    this: NodeClass<T>,
    options: WithSubqueries<{
      expression: ExpressionIn;
      where?: Condition;
      name?: string;
      join?: Join;
      groupBy?: ExpressionIn[];
      having?: Condition;
      sort?: Sort[];
    }>,
  ): Query<T> {
    if (this.__definition__.domain == null) {
      throw new Error(`${this.__definition__.name} has no store domain`);
    }
    const _Query = STRUCT_CLASS_BY_TYPE[StructType.QUERY] as typeof Query;
    const _NodeDefinitionReference = STRUCT_CLASS_BY_TYPE[
      StructType.NODE_DEFINITION_REFERENCE
    ] as typeof NodeDefinitionReference;
    const _Expression = STRUCT_CLASS_BY_TYPE[StructType.EXPRESSION] as typeof Expression;
    const _Aggregation = STRUCT_CLASS_BY_TYPE[StructType.AGGREGATION] as typeof Aggregation;
    const { expression, where, name, join, groupBy, having, sort, ...subqueries } = options;
    const query = new _Query<T>({
      type: groupBy ? QueryType.GROUPED_SCALAR : QueryType.SCALAR,
      domain: this.__definition__.domain,
      definition: _NodeDefinitionReference.of(this as unknown as NodeClass),
      name: name ?? toCasing(NodeType[this.metatype].toLocaleLowerCase(), Casing.CAMEL),
      join,
      where,
      having,
      groupBy: groupBy?.map(_Expression.of),
      aggregation: _Aggregation.of(AggregationType.MIN, _Expression.of(expression)),
      sort,
      subqueries: toSubqueries(subqueries),
    });
    return query;
  }

  /** Make a max Query for this Node/Trait type. */
  static max<T extends Node = Node>(
    this: NodeClass<T>,
    options: WithSubqueries<{
      expression: ExpressionIn;
      where?: Condition;
      name?: string;
      join?: Join;
      groupBy?: ExpressionIn[];
      having?: Condition;
      sort?: Sort[];
    }>,
  ): Query<T> {
    if (this.__definition__.domain == null) {
      throw new Error(`${this.__definition__.name} has no store domain`);
    }
    const { expression, where, name, join, groupBy, having, sort, ...subqueries } = options;
    const _Query = STRUCT_CLASS_BY_TYPE[StructType.QUERY] as typeof Query;
    const _NodeDefinitionReference = STRUCT_CLASS_BY_TYPE[
      StructType.NODE_DEFINITION_REFERENCE
    ] as typeof NodeDefinitionReference;
    const _Expression = STRUCT_CLASS_BY_TYPE[StructType.EXPRESSION] as typeof Expression;
    const _Aggregation = STRUCT_CLASS_BY_TYPE[StructType.AGGREGATION] as typeof Aggregation;
    const query = new _Query<T>({
      type: groupBy ? QueryType.GROUPED_SCALAR : QueryType.SCALAR,
      domain: this.__definition__.domain,
      definition: _NodeDefinitionReference.of(this as unknown as NodeClass),
      name: name ?? toCasing(NodeType[this.metatype].toLocaleLowerCase(), Casing.CAMEL),
      join,
      where,
      having,
      groupBy: groupBy?.map(_Expression.of),
      aggregation: _Aggregation.of(AggregationType.MAX, _Expression.of(expression)),
      sort,
      subqueries: toSubqueries(subqueries),
    });
    return query;
  }

  /** Make a sum Query for this Node/Trait type. */
  static sum<T extends Node = Node>(
    this: NodeClass<T>,
    options: WithSubqueries<{
      expression: ExpressionIn;
      where?: Condition;
      name?: string;
      join?: Join;
      groupBy?: ExpressionIn[];
      having?: Condition;
      sort?: Sort[];
    }>,
  ): Query<T> {
    if (this.__definition__.domain == null) {
      throw new Error(`${this.__definition__.name} has no store domain`);
    }
    const { expression, where, name, join, groupBy, having, sort, ...subqueries } = options;
    const _Query = STRUCT_CLASS_BY_TYPE[StructType.QUERY] as typeof Query;
    const _NodeDefinitionReference = STRUCT_CLASS_BY_TYPE[
      StructType.NODE_DEFINITION_REFERENCE
    ] as typeof NodeDefinitionReference;
    const _Expression = STRUCT_CLASS_BY_TYPE[StructType.EXPRESSION] as typeof Expression;
    const _Aggregation = STRUCT_CLASS_BY_TYPE[StructType.AGGREGATION] as typeof Aggregation;
    const query = new _Query<T>({
      type: groupBy ? QueryType.GROUPED_SCALAR : QueryType.SCALAR,
      domain: this.__definition__.domain,
      definition: _NodeDefinitionReference.of(this as unknown as NodeClass),
      name: name ?? toCasing(NodeType[this.metatype].toLocaleLowerCase(), Casing.CAMEL),
      join,
      where,
      having,
      groupBy: groupBy?.map(_Expression.of),
      aggregation: _Aggregation.of(AggregationType.SUM, _Expression.of(expression)),
      sort,
      subqueries: toSubqueries(subqueries),
    });
    return query;
  }
}
registerNodeClass(NodeType.NODE, Node);

/** A Node class. */
type NodeConstructor<N extends Node = Node> = new (...args: any[]) => N;
type AbstractNodeConstructor<N extends Node = Node> = abstract new (...args: any[]) => N;
export type NodeClass<N extends Node = Node> = (NodeConstructor<N> | AbstractNodeConstructor<N>) &
  (BuiltinObjectClass<any> & {
    metatype: NodeType;
    __definition__: NodeDefinition;
    __definitionReference: NodeDefinitionReference;
  });

/** Check if a value is a Node of a specific type. */
export function isNode<T extends NodeType>(value: any, nodeType?: T): value is NodeTypeMapping[T] {
  return (
    value instanceof Node &&
    (nodeType == null ||
      value.metatype === nodeType ||
      value.__definition__.inherits.includes(nodeType))
  );
}

/** Check if a value is a Node with a specific trait. */
export function hasTrait<T extends TraitType>(
  value: any,
  traitType: T,
): value is Node & TraitTypeMapping[T] {
  return value instanceof Node && value.__traits__.includes(traitType);
}

export type WithSubqueries<T, Q = Query> = T & {
  [K: string]: Q | T[keyof T] | undefined;
};

/** Convert a WithSubqueries object to an array of Queries. */
export function toSubqueries(subqueries: WithSubqueries<Record<string, any>>): Query[] {
  const _Join = STRUCT_CLASS_BY_TYPE[StructType.JOIN] as typeof Join;
  const queries: Query[] = [];
  for (const [name, subquery] of Object.entries(subqueries)) {
    if (!isStruct(subquery, StructType.QUERY)) {
      continue;
    }
    if (subquery.join == null) {
      const join = new _Join({ type: JoinType.CHILD });
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

/** Expand a collection of NodeTypes into a flat collection of NodeTypes. */
export function expandNodeInheritance(nodeTypes: NodeType[]): NodeType[] {
  const expanded: NodeType[] = [];
  for (const type of nodeTypes) {
    const nodeDefinition = NODE_CLASS_BY_TYPE[type].__definition__;
    for (const inheritedType of nodeDefinition.inheritedBy) {
      if (!expanded.includes(inheritedType)) {
        expanded.push(inheritedType);
      }
    }
    if (!nodeDefinition.isAbstract && !expanded.includes(type)) {
      expanded.push(type);
    }
  }
  return expanded;
}

/** Resolve the NodeTypes for a NodeType, TraitType, or Node class. */
export function expandNodeTypes(
  nodeType?: NodeType | NodeClass,
  options: { expandInheritance: boolean } = { expandInheritance: true },
): NodeType[] | null {
  if (nodeType == null) {
    return null;
  }
  let nodeTypes: NodeType[] = [];
  if (typeof nodeType == "number") {
    nodeTypes.push(nodeType);
  } else {
    nodeTypes.push(nodeType.metatype);
  }
  if (options.expandInheritance) {
    nodeTypes = expandNodeInheritance(nodeTypes);
  }
  return nodeTypes;
}
