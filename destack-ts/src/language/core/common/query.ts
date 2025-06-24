import { NodeClass, toValue } from "@destack/language";
import {
  AttributeReference,
  CustomEntityDefinition,
  Field,
  NodeType,
  PropertyReference,
  RelationReference,
  Session,
  Struct,
  StructFrozen,
  StructType,
  Supergraph,
  Value,
} from "@destack/language/core";
import { assertNever } from "@destack/utils/functools";
import { v4 as uuid4 } from "uuid";

/* ==== DESTACK_GENERATED_START:ENUM:108 ==== */
/**
 * FunctionType
 */
export enum FunctionType {
  ADD = 1,
  SUBTRACT = 2,
  MULTIPLY = 3,
  DIVIDE = 4,
  MODULO = 5,
  POWER = 6,
}
/* ==== DESTACK_GENERATED_END:ENUM:108 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:103 ==== */
/**
 * ConditionalType
 */
export enum ConditionalType {
  NOT = 1,
  AND = 2,
  OR = 3,
  EQUALS = 10,
  NOT_EQUALS = 11,
  GREATER_THAN = 12,
  GREATER_THAN_OR_EQUALS = 13,
  LESS_THAN = 14,
  LESS_THAN_OR_EQUALS = 15,
  MATCHES = 20,
  STARTS_WITH = 21,
  ENDS_WITH = 22,
  IN = 30,
  NOT_IN = 31,
  EXISTS = 40,
  NOT_EXISTS = 41,
}
/* ==== DESTACK_GENERATED_END:ENUM:103 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:104 ==== */
/**
 * AggregationType
 */
export enum AggregationType {
  EXISTS = 1,
  COUNT = 2,
  SUM = 3,
  MIN = 4,
  MAX = 5,
  AVERAGE = 6,
}
/* ==== DESTACK_GENERATED_END:ENUM:104 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:109 ==== */
/**
 * ExpressionType
 */
export enum ExpressionType {
  LITERAL = 1,
  ATTRIBUTE = 2,
  CONDITION = 3,
  FUNCTION = 4,
  AGGREGATION = 5,
}
/* ==== DESTACK_GENERATED_END:ENUM:109 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:106 ==== */
/**
 * SortType
 */
export enum SortType {
  ASCENDING = 1,
  DESCENDING = 2,
}
/* ==== DESTACK_GENERATED_END:ENUM:106 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:105 ==== */
/**
 * SortMode
 */
export enum SortMode {
  MAX = 1,
  MIN = 2,
  AVERAGE = 3,
  SUM = 4,
  MEDIAN = 5,
}
/* ==== DESTACK_GENERATED_END:ENUM:105 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:107 ==== */
/**
 * JoinType
 */
export enum JoinType {
  LEFT = 1,
  PARENT = 10,
  CHILD = 11,
}
/* ==== DESTACK_GENERATED_END:ENUM:107 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:120 ==== */
/**
 * QueryType
 */
export enum QueryType {
  NODE = 1,
  SCALAR = 2,
  GROUPED_NODE = 10,
  GROUPED_SCALAR = 11,
}
/* ==== DESTACK_GENERATED_END:ENUM:120 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:121 ==== */
/**
 * QueryUpdateType
 */
export enum QueryUpdateType {
  FULL_RESULT = 1,
  PARTIAL_RESULT = 2,
}
/* ==== DESTACK_GENERATED_END:ENUM:121 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:50101 ==== */
/**
 * Function
 */
export class Function extends StructFrozen {
  static metatype: StructType = StructType.FUNCTION;
  static __isFrozen__: boolean = true;

  /**
   * Function.type
   */
  readonly type: FunctionType;

  /**
   * Function.left
   */
  readonly left: Expression;

  /**
   * Function.right
   */
  readonly right: Expression | null;

  constructor(options: {
    type: FunctionType;
    left: Expression;
    right?: Expression | null;
    _session?: Session | null;
    _supergraph?: Supergraph | null;
  }) {
    super(
      // session
      options._session ?? null,
      // supergraph
      options._supergraph ?? null,
    );

    // properties
    let _type = options.type;
    if (_type === null) {
      throw new Error(`Function.type is required`);
    }
    this.type = _type;
    let _left = options.left;
    if (_left === null) {
      throw new Error(`Function.left is required`);
    }
    this.left = _left;
    let _right = options.right ?? null;
    this.right = _right;

    // identity
    // ...
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

  /* ==== DESTACK_CUSTOM_START ==== */

  /** Make a Function from a shorthand expression. */
  static of(type: FunctionType, left: Expression, right?: Expression | null): Function {
    return new Function({ type, left, right: right ?? null });
  }
}

/* ==== DESTACK_GENERATED_END:STRUCT:50101 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:50104 ==== */
/**
 * Boolean predicate (AND, =, <, etc.).
 */
export class Condition extends StructFrozen {
  static metatype: StructType = StructType.CONDITION;
  static __isFrozen__: boolean = true;

  /**
   * Condition.type
   */
  readonly type: ConditionalType;

  /**
   * Condition.left
   */
  readonly left: Expression;

  /**
   * Condition.right
   */
  readonly right: Expression | null;

  constructor(options: {
    type: ConditionalType;
    left: Expression;
    right?: Expression | null;
    _session?: Session | null;
    _supergraph?: Supergraph | null;
  }) {
    super(
      // session
      options._session ?? null,
      // supergraph
      options._supergraph ?? null,
    );

    // properties
    let _type = options.type;
    if (_type === null) {
      throw new Error(`Condition.type is required`);
    }
    this.type = _type;
    let _left = options.left;
    if (_left === null) {
      throw new Error(`Condition.left is required`);
    }
    this.left = _left;
    let _right = options.right ?? null;
    this.right = _right;

    // identity
    // ...
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

  /* ==== DESTACK_CUSTOM_START ==== */

  /** Make a Condition from a shorthand expression. */
  static of(
    attribute: Field | PropertyReference | AttributeReference,
    type: ConditionalType = ConditionalType.EQUALS,
    value: any = null,
  ): Condition {
    const left = Expression.of(attribute);
    const right = Expression.of(toValue(value));
    return new Condition({ type, left, right });
  }
}

/* ==== DESTACK_GENERATED_END:STRUCT:50104 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:50103 ==== */
/**
 * Aggregation.
 */
export class Aggregation extends StructFrozen {
  static metatype: StructType = StructType.AGGREGATION;
  static __isFrozen__: boolean = true;

  /**
   * Aggregation.type
   */
  readonly type: AggregationType;

  /**
   * Aggregation.expression
   */
  readonly expression: Expression | null;

  constructor(options: {
    type: AggregationType;
    expression?: Expression | null;
    _session?: Session | null;
    _supergraph?: Supergraph | null;
  }) {
    super(
      // session
      options._session ?? null,
      // supergraph
      options._supergraph ?? null,
    );

    // properties
    let _type = options.type;
    if (_type === null) {
      throw new Error(`Aggregation.type is required`);
    }
    this.type = _type;
    let _expression = options.expression ?? null;
    this.expression = _expression;

    // identity
    // ...
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

  /* ==== DESTACK_CUSTOM_START ==== */

  /** Make an Aggregation from a shorthand expression. */
  static of(type: AggregationType, operand?: Expression | null): Aggregation {
    return new Aggregation({ type, expression: operand ?? null });
  }
}

/* ==== DESTACK_GENERATED_END:STRUCT:50103 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:50100 ==== */
/**
 * Wrapper to unify any scalar / boolean / aggregate sub-tree.
 */
export class Expression extends StructFrozen {
  static metatype: StructType = StructType.EXPRESSION;
  static __isFrozen__: boolean = true;

  /**
   * Expression.type
   */
  readonly type: ExpressionType;

  /**
   * Expression.literal
   */
  readonly literal: Value | null;

  /**
   * Expression.attribute
   */
  readonly attribute: AttributeReference | null;

  /**
   * Expression.condition
   */
  readonly condition: Condition | null;

  /**
   * Expression.function
   */
  readonly function: Function | null;

  /**
   * Expression.aggregation
   */
  readonly aggregation: Aggregation | null;

  constructor(options: {
    type: ExpressionType;
    literal?: Value | null;
    attribute?: AttributeReference | null;
    condition?: Condition | null;
    function?: Function | null;
    aggregation?: Aggregation | null;
    _session?: Session | null;
    _supergraph?: Supergraph | null;
  }) {
    super(
      // session
      options._session ?? null,
      // supergraph
      options._supergraph ?? null,
    );

    // properties
    let _type = options.type;
    if (_type === null) {
      throw new Error(`Expression.type is required`);
    }
    this.type = _type;
    let _literal = options.literal ?? null;
    this.literal = _literal;
    let _attribute = options.attribute ?? null;
    this.attribute = _attribute;
    let _condition = options.condition ?? null;
    this.condition = _condition;
    let _function = options.function ?? null;
    this.function = _function;
    let _aggregation = options.aggregation ?? null;
    this.aggregation = _aggregation;

    // identity
    // ...
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

  /* ==== DESTACK_CUSTOM_START ==== */

  /** Make an Expression from a shorthand expression. */
  static of(thing: ExpressionIn): Expression {
    if (thing instanceof Value) {
      return new Expression({ type: ExpressionType.LITERAL, literal: thing });
    } else if (thing instanceof AttributeReference) {
      return new Expression({ type: ExpressionType.ATTRIBUTE, attribute: thing });
    } else if (thing instanceof Condition) {
      return new Expression({ type: ExpressionType.CONDITION, condition: thing });
    } else if (thing instanceof Function) {
      return new Expression({ type: ExpressionType.FUNCTION, function: thing });
    } else if (thing instanceof Aggregation) {
      return new Expression({ type: ExpressionType.AGGREGATION, aggregation: thing });
    } else if (thing instanceof Expression) {
      return thing;
    } else if (thing instanceof Field) {
      return new Expression({ type: ExpressionType.ATTRIBUTE, attribute: AttributeReference.of(thing) });
    } else if (thing instanceof PropertyReference) {
      return new Expression({ type: ExpressionType.ATTRIBUTE, attribute: AttributeReference.of(thing) });
    } else {
      assertNever(thing);
    }
  }
}

/* ==== DESTACK_GENERATED_END:STRUCT:50100 ==== */

export type ExpressionIn =
  | Value
  | AttributeReference
  | Field
  | PropertyReference
  | Condition
  | Function
  | Aggregation
  | Expression;

/* ==== DESTACK_GENERATED_START:STRUCT:50105 ==== */
/**
 * ORDER BY specification.
 */
export class Sort extends StructFrozen {
  static metatype: StructType = StructType.SORT;
  static __isFrozen__: boolean = true;

  /**
   * Sort.type
   */
  readonly type: SortType;

  /**
   * Sort.by
   */
  readonly by: Expression;

  /**
   * Sort.mode
   */
  readonly mode: SortMode | null;

  constructor(options: {
    type: SortType;
    by: Expression;
    mode?: SortMode | null;
    _session?: Session | null;
    _supergraph?: Supergraph | null;
  }) {
    super(
      // session
      options._session ?? null,
      // supergraph
      options._supergraph ?? null,
    );

    // properties
    let _type = options.type;
    if (_type === null) {
      throw new Error(`Sort.type is required`);
    }
    this.type = _type;
    let _by = options.by;
    if (_by === null) {
      throw new Error(`Sort.by is required`);
    }
    this.by = _by;
    let _mode = options.mode ?? null;
    this.mode = _mode;

    // identity
    // ...
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

  /* ==== DESTACK_CUSTOM_START ==== */

  /** Make a Sort from a shorthand expression. */
  static of(by: ExpressionIn, mode?: SortMode | null): Sort {
    return new Sort({ type: SortType.ASCENDING, by: Expression.of(by), mode: mode ?? null });
  }
}

/* ==== DESTACK_GENERATED_END:STRUCT:50105 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:50106 ==== */
/**
 * Select specific Attributes.
 */
export class Select extends StructFrozen {
  static metatype: StructType = StructType.SELECT;
  static __isFrozen__: boolean = true;

  /**
   * Select.attributes
   */
  readonly attributes: Array<AttributeReference>;

  constructor(options: {
    attributes?: Array<AttributeReference>;
    _session?: Session | null;
    _supergraph?: Supergraph | null;
  }) {
    super(
      // session
      options._session ?? null,
      // supergraph
      options._supergraph ?? null,
    );

    // properties
    let _attributes = options.attributes ?? null;
    if (_attributes === null) {
      throw new Error(`Select.attributes is required`);
    }
    this.attributes = _attributes;

    // identity
    // ...
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

  /* ==== DESTACK_CUSTOM_START ==== */

  /** Make a Select from a shorthand expression. */
  static of(...attributes: (Field | PropertyReference)[]): Select {
    return new Select({ attributes: attributes.map((attr) => AttributeReference.of(attr)) });
  }
}

/* ==== DESTACK_GENERATED_END:STRUCT:50106 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:50102 ==== */
/**
 * Join a Query with another Query.
 */
export class Join extends StructFrozen {
  static metatype: StructType = StructType.JOIN;
  static __isFrozen__: boolean = true;

  /**
   * Join.type
   */
  readonly type: JoinType;

  /**
   * Join.relation
   */
  readonly relation: RelationReference | null;

  /**
   * Join.recursive
   */
  readonly recursive: boolean;

  /**
   * Join.depth
   */
  readonly depth: number | null;

  /**
   * Join.on
   */
  readonly on: Condition | null;

  constructor(options: {
    type: JoinType;
    relation?: RelationReference | null;
    recursive?: boolean;
    depth?: number | null;
    on?: Condition | null;
    _session?: Session | null;
    _supergraph?: Supergraph | null;
  }) {
    super(
      // session
      options._session ?? null,
      // supergraph
      options._supergraph ?? null,
    );

    // properties
    let _type = options.type;
    if (_type === null) {
      throw new Error(`Join.type is required`);
    }
    this.type = _type;
    let _relation = options.relation ?? null;
    this.relation = _relation;
    let _recursive = options.recursive ?? null;
    if (_recursive === null) {
      _recursive = false;
    }
    if (_recursive === null) {
      throw new Error(`Join.recursive is required`);
    }
    this.recursive = _recursive;
    let _depth = options.depth ?? null;
    this.depth = _depth;
    let _on = options.on ?? null;
    this.on = _on;

    // identity
    // ...
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

  /* ==== DESTACK_CUSTOM_START ==== */

  /** Make a Join from a shorthand expression. */
  static of(
    relation: NodeType | NodeClass | CustomEntityDefinition,
    recursive?: boolean,
    depth?: number | null,
    on?: Condition | null,
  ): Join {
    return new Join({
      type: JoinType.LEFT,
      relation: RelationReference.of(relation),
      recursive: recursive ?? false,
      depth: depth ?? null,
      on: on ?? null,
    });
  }
}

/* ==== DESTACK_GENERATED_END:STRUCT:50102 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:50110 ==== */
/**
 * A GraphQL-inspired Query node (with subqueries).
 */
export class Query extends StructFrozen {
  static metatype: StructType = StructType.QUERY;
  static __isFrozen__: boolean = true;

  /**
   * Query.id
   */
  readonly id: string;

  /**
   * Query.type
   */
  readonly type: QueryType;

  /**
   * Name for this subquery. Must be unique within the parent Query.
   */
  readonly name: string;

  /**
   * Query.relation
   */
  readonly relation: RelationReference;

  /**
   * Relative to parent Query.
   */
  readonly join: Join | null;

  /**
   * Query.select
   */
  readonly select: Select | null;

  /**
   * Query.subqueries
   */
  readonly subqueries: Array<Query>;

  /**
   * Query.where
   */
  readonly where: Condition | null;

  /**
   * Query.having
   */
  readonly having: Condition | null;

  /**
   * Query.groupBy
   */
  readonly groupBy: Array<Expression>;

  /**
   * Query.aggregation
   */
  readonly aggregation: Aggregation | null;

  /**
   * Query.sort
   */
  readonly sort: Array<Sort>;

  /**
   * Query.limit
   */
  readonly limit: number | null;

  /**
   * Query.offset
   */
  readonly offset: number | null;

  constructor(options: {
    id?: string;
    type: QueryType;
    name: string;
    relation: RelationReference;
    join?: Join | null;
    select?: Select | null;
    subqueries?: Array<Query>;
    where?: Condition | null;
    having?: Condition | null;
    groupBy?: Array<Expression>;
    aggregation?: Aggregation | null;
    sort?: Array<Sort>;
    limit?: number | null;
    offset?: number | null;
    _session?: Session | null;
    _supergraph?: Supergraph | null;
  }) {
    super(
      // session
      options._session ?? null,
      // supergraph
      options._supergraph ?? null,
    );

    // properties
    let _id = options.id ?? null;
    if (_id === null) {
      _id = uuid4();
    }
    if (_id === null) {
      throw new Error(`Query.id is required`);
    }
    this.id = _id;
    let _type = options.type;
    if (_type === null) {
      throw new Error(`Query.type is required`);
    }
    this.type = _type;
    let _name = options.name;
    if (_name === null) {
      throw new Error(`Query.name is required`);
    }
    this.name = _name;
    let _relation = options.relation;
    if (_relation === null) {
      throw new Error(`Query.relation is required`);
    }
    this.relation = _relation;
    let _join = options.join ?? null;
    this.join = _join;
    let _select = options.select ?? null;
    this.select = _select;
    let _subqueries = options.subqueries ?? null;
    if (_subqueries === null) {
      throw new Error(`Query.subqueries is required`);
    }
    this.subqueries = _subqueries;
    let _where = options.where ?? null;
    this.where = _where;
    let _having = options.having ?? null;
    this.having = _having;
    let _groupBy = options.groupBy ?? null;
    if (_groupBy === null) {
      throw new Error(`Query.groupBy is required`);
    }
    this.groupBy = _groupBy;
    let _aggregation = options.aggregation ?? null;
    this.aggregation = _aggregation;
    let _sort = options.sort ?? null;
    if (_sort === null) {
      throw new Error(`Query.sort is required`);
    }
    this.sort = _sort;
    let _limit = options.limit ?? null;
    this.limit = _limit;
    let _offset = options.offset ?? null;
    this.offset = _offset;

    // identity
    // ...
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
}
/* ==== DESTACK_GENERATED_END:STRUCT:50110 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:50114 ==== */
/**
 * A histogram.
 */
export class Histogram extends StructFrozen {
  static metatype: StructType = StructType.HISTOGRAM;
  static __isFrozen__: boolean = true;

  /**
   * Histogram.buckets
   */
  readonly buckets: Array<Value>;

  /**
   * Histogram.counts
   */
  readonly counts: Array<number>;

  constructor(options: {
    buckets?: Array<Value>;
    counts?: Array<number>;
    _session?: Session | null;
    _supergraph?: Supergraph | null;
  }) {
    super(
      // session
      options._session ?? null,
      // supergraph
      options._supergraph ?? null,
    );

    // properties
    let _buckets = options.buckets ?? null;
    if (_buckets === null) {
      throw new Error(`Histogram.buckets is required`);
    }
    this.buckets = _buckets;
    let _counts = options.counts ?? null;
    if (_counts === null) {
      throw new Error(`Histogram.counts is required`);
    }
    this.counts = _counts;

    // identity
    // ...
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
}
/* ==== DESTACK_GENERATED_END:STRUCT:50114 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:50111 ==== */
/**
 * The result of a Query.
 * For grouped queries, group results are in Query.groups.
 * The subresults correspond to Query.subqueries.
 * If subresults for a Query clause may be missing if the subquery was deemed empty.
 */
export class QueryResult extends Struct {
  static metatype: StructType = StructType.QUERY_RESULT;
  static __isFrozen__: boolean = false;

  /**
   * QueryResult.id
   */
  id: string;

  /**
   * QueryResultBase.type
   */
  type: QueryType;

  /**
   * QueryResult.groups
   */
  groups: Array<QueryResultGroup>;

  /**
   * QueryResult.subresults
   */
  subresults: Array<QueryResult>;

  /**
   * QueryResultBase.nodes
   */
  nodes: Array<Value>;

  /**
   * QueryResultBase.count
   */
  count: number | null;

  /**
   * QueryResultBase.exists
   */
  exists: boolean | null;

  /**
   * QueryResultBase.scalar
   */
  scalar: Value | null;

  constructor(options: {
    id: string;
    type: QueryType;
    groups?: Array<QueryResultGroup>;
    subresults?: Array<QueryResult>;
    nodes?: Array<Value>;
    count?: number | null;
    exists?: boolean | null;
    scalar?: Value | null;
    _session?: Session | null;
    _supergraph?: Supergraph | null;
  }) {
    super(
      // session
      options._session ?? null,
      // supergraph
      options._supergraph ?? null,
    );

    // properties
    let _id = options.id;
    if (_id === null) {
      throw new Error(`QueryResult.id is required`);
    }
    this.id = _id;
    let _type = options.type;
    if (_type === null) {
      throw new Error(`QueryResult.type is required`);
    }
    this.type = _type;
    let _groups = options.groups ?? null;
    if (_groups === null) {
      throw new Error(`QueryResult.groups is required`);
    }
    this.groups = _groups;
    let _subresults = options.subresults ?? null;
    if (_subresults === null) {
      throw new Error(`QueryResult.subresults is required`);
    }
    this.subresults = _subresults;
    let _nodes = options.nodes ?? null;
    if (_nodes === null) {
      throw new Error(`QueryResult.nodes is required`);
    }
    this.nodes = _nodes;
    let _count = options.count ?? null;
    this.count = _count;
    let _exists = options.exists ?? null;
    this.exists = _exists;
    let _scalar = options.scalar ?? null;
    this.scalar = _scalar;

    // identity
    // ...
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
}
/* ==== DESTACK_GENERATED_END:STRUCT:50111 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:50112 ==== */
/**
 * A group in a QueryResult.
 */
export class QueryResultGroup extends Struct {
  static metatype: StructType = StructType.QUERY_RESULT_GROUP;
  static __isFrozen__: boolean = false;

  /**
   * QueryResultBase.type
   */
  type: QueryType;

  /**
   * QueryResultGroup.discriminator
   */
  discriminator: Value;

  /**
   * QueryResultBase.nodes
   */
  nodes: Array<Value>;

  /**
   * QueryResultBase.count
   */
  count: number | null;

  /**
   * QueryResultBase.exists
   */
  exists: boolean | null;

  /**
   * QueryResultBase.scalar
   */
  scalar: Value | null;

  constructor(options: {
    type: QueryType;
    discriminator: Value;
    nodes?: Array<Value>;
    count?: number | null;
    exists?: boolean | null;
    scalar?: Value | null;
    _session?: Session | null;
    _supergraph?: Supergraph | null;
  }) {
    super(
      // session
      options._session ?? null,
      // supergraph
      options._supergraph ?? null,
    );

    // properties
    let _type = options.type;
    if (_type === null) {
      throw new Error(`QueryResultGroup.type is required`);
    }
    this.type = _type;
    let _discriminator = options.discriminator;
    if (_discriminator === null) {
      throw new Error(`QueryResultGroup.discriminator is required`);
    }
    this.discriminator = _discriminator;
    let _nodes = options.nodes ?? null;
    if (_nodes === null) {
      throw new Error(`QueryResultGroup.nodes is required`);
    }
    this.nodes = _nodes;
    let _count = options.count ?? null;
    this.count = _count;
    let _exists = options.exists ?? null;
    this.exists = _exists;
    let _scalar = options.scalar ?? null;
    this.scalar = _scalar;

    // identity
    // ...
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
}
/* ==== DESTACK_GENERATED_END:STRUCT:50112 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:50113 ==== */
/**
 * An update to a QueryResult.
 */
export class QueryUpdate extends StructFrozen {
  static metatype: StructType = StructType.QUERY_UPDATE;
  static __isFrozen__: boolean = true;

  /**
   * QueryUpdate.type
   */
  readonly type: QueryUpdateType;

  /**
   * QueryUpdate.result
   */
  readonly result: QueryResult | null;

  constructor(options: {
    type: QueryUpdateType;
    result?: QueryResult | null;
    _session?: Session | null;
    _supergraph?: Supergraph | null;
  }) {
    super(
      // session
      options._session ?? null,
      // supergraph
      options._supergraph ?? null,
    );

    // properties
    let _type = options.type;
    if (_type === null) {
      throw new Error(`QueryUpdate.type is required`);
    }
    this.type = _type;
    let _result = options.result ?? null;
    this.result = _result;

    // identity
    // ...
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
}
/* ==== DESTACK_GENERATED_END:STRUCT:50113 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:2571 ==== */
/**
 * A selection of fields from a Node.
 */
export class Selection extends StructFrozen {
  static metatype: StructType = StructType.SELECTION;
  static __isFrozen__: boolean = true;

  constructor(options: { _session?: Session | null; _supergraph?: Supergraph | null }) {
    super(
      // session
      options._session ?? null,
      // supergraph
      options._supergraph ?? null,
    );

    // properties

    // identity
    // ...
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
}
/* ==== DESTACK_GENERATED_END:STRUCT:2571 ==== */
