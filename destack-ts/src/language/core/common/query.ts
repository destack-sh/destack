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
    _hash?: number | null;
    _repr?: string | null;
    _proto?: any | null;
    _value?: { [key: string]: any } | null;
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
    // @ts-expect-error(readonly)
    this._hash = options._hash ?? null;
    // @ts-expect-error(readonly)
    this._repr = options._repr ?? null;
    // @ts-expect-error(readonly)
    this._proto = options._proto ?? null;
    // @ts-expect-error(readonly)
    this._value = options._value ?? null;
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

  toValue(): { [key: string]: any } {
    if (this._value === null) {
      // @ts-expect-error(readonly)
      this._value = Function.__packValue__(this);
    }
    return this._value;
  }

  static __packValue__(object: Function): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 50101;
    objectValue["30"] = object.type;
    objectValue["31"] = object.left.toValue();
    if (object.right !== null) {
      objectValue["32"] = object.right.toValue();
    }
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Function {
    const rightValue = objectValue["32"];
    const unpackedRight =
      rightValue !== undefined ? Expression.fromValue(rightValue, _session, _supergraph, _graph, _connection) : null;
    return new Function({
      type: Number(objectValue["30"]),
      left: Expression.fromValue(objectValue["31"], _session, _supergraph, _graph, _connection),
      right: unpackedRight,
      _value: objectValue,
      _supergraph,
    });
  }

  static fromValue(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Function {
    return Function.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
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
    _hash?: number | null;
    _repr?: string | null;
    _proto?: any | null;
    _value?: { [key: string]: any } | null;
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
    // @ts-expect-error(readonly)
    this._hash = options._hash ?? null;
    // @ts-expect-error(readonly)
    this._repr = options._repr ?? null;
    // @ts-expect-error(readonly)
    this._proto = options._proto ?? null;
    // @ts-expect-error(readonly)
    this._value = options._value ?? null;
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

  toValue(): { [key: string]: any } {
    if (this._value === null) {
      // @ts-expect-error(readonly)
      this._value = Condition.__packValue__(this);
    }
    return this._value;
  }

  static __packValue__(object: Condition): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 50104;
    objectValue["30"] = object.type;
    objectValue["31"] = object.left.toValue();
    if (object.right !== null) {
      objectValue["32"] = object.right.toValue();
    }
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Condition {
    const rightValue = objectValue["32"];
    const unpackedRight =
      rightValue !== undefined ? Expression.fromValue(rightValue, _session, _supergraph, _graph, _connection) : null;
    return new Condition({
      type: Number(objectValue["30"]),
      left: Expression.fromValue(objectValue["31"], _session, _supergraph, _graph, _connection),
      right: unpackedRight,
      _value: objectValue,
      _supergraph,
    });
  }

  static fromValue(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Condition {
    return Condition.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
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
    _hash?: number | null;
    _repr?: string | null;
    _proto?: any | null;
    _value?: { [key: string]: any } | null;
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
    // @ts-expect-error(readonly)
    this._hash = options._hash ?? null;
    // @ts-expect-error(readonly)
    this._repr = options._repr ?? null;
    // @ts-expect-error(readonly)
    this._proto = options._proto ?? null;
    // @ts-expect-error(readonly)
    this._value = options._value ?? null;
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

  toValue(): { [key: string]: any } {
    if (this._value === null) {
      // @ts-expect-error(readonly)
      this._value = Aggregation.__packValue__(this);
    }
    return this._value;
  }

  static __packValue__(object: Aggregation): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 50103;
    objectValue["30"] = object.type;
    if (object.expression !== null) {
      objectValue["31"] = object.expression.toValue();
    }
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Aggregation {
    const expressionValue = objectValue["31"];
    const unpackedExpression =
      expressionValue !== undefined
        ? Expression.fromValue(expressionValue, _session, _supergraph, _graph, _connection)
        : null;
    return new Aggregation({
      type: Number(objectValue["30"]),
      expression: unpackedExpression,
      _value: objectValue,
      _supergraph,
    });
  }

  static fromValue(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Aggregation {
    return Aggregation.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
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
    _hash?: number | null;
    _repr?: string | null;
    _proto?: any | null;
    _value?: { [key: string]: any } | null;
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
    // @ts-expect-error(readonly)
    this._hash = options._hash ?? null;
    // @ts-expect-error(readonly)
    this._repr = options._repr ?? null;
    // @ts-expect-error(readonly)
    this._proto = options._proto ?? null;
    // @ts-expect-error(readonly)
    this._value = options._value ?? null;
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

  toValue(): { [key: string]: any } {
    if (this._value === null) {
      // @ts-expect-error(readonly)
      this._value = Expression.__packValue__(this);
    }
    return this._value;
  }

  static __packValue__(object: Expression): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 50100;
    objectValue["30"] = object.type;
    if (object.literal !== null) {
      objectValue["31"] = object.literal.toValue();
    }
    if (object.attribute !== null) {
      objectValue["32"] = object.attribute.toValue();
    }
    if (object.condition !== null) {
      objectValue["33"] = object.condition.toValue();
    }
    if (object.function !== null) {
      objectValue["34"] = object.function.toValue();
    }
    if (object.aggregation !== null) {
      objectValue["35"] = object.aggregation.toValue();
    }
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Expression {
    const literalValue = objectValue["31"];
    const unpackedLiteral =
      literalValue !== undefined ? Value.fromValue(literalValue, _session, _supergraph, _graph, _connection) : null;
    const attributeValue = objectValue["32"];
    const unpackedAttribute =
      attributeValue !== undefined
        ? AttributeReference.fromValue(attributeValue, _session, _supergraph, _graph, _connection)
        : null;
    const conditionValue = objectValue["33"];
    const unpackedCondition =
      conditionValue !== undefined
        ? Condition.fromValue(conditionValue, _session, _supergraph, _graph, _connection)
        : null;
    const functionValue = objectValue["34"];
    const unpackedFunction =
      functionValue !== undefined
        ? Function.fromValue(functionValue, _session, _supergraph, _graph, _connection)
        : null;
    const aggregationValue = objectValue["35"];
    const unpackedAggregation =
      aggregationValue !== undefined
        ? Aggregation.fromValue(aggregationValue, _session, _supergraph, _graph, _connection)
        : null;
    return new Expression({
      type: Number(objectValue["30"]),
      literal: unpackedLiteral,
      attribute: unpackedAttribute,
      condition: unpackedCondition,
      function: unpackedFunction,
      aggregation: unpackedAggregation,
      _value: objectValue,
      _supergraph,
    });
  }

  static fromValue(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Expression {
    return Expression.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
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
    _hash?: number | null;
    _repr?: string | null;
    _proto?: any | null;
    _value?: { [key: string]: any } | null;
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
    // @ts-expect-error(readonly)
    this._hash = options._hash ?? null;
    // @ts-expect-error(readonly)
    this._repr = options._repr ?? null;
    // @ts-expect-error(readonly)
    this._proto = options._proto ?? null;
    // @ts-expect-error(readonly)
    this._value = options._value ?? null;
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

  toValue(): { [key: string]: any } {
    if (this._value === null) {
      // @ts-expect-error(readonly)
      this._value = Sort.__packValue__(this);
    }
    return this._value;
  }

  static __packValue__(object: Sort): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 50105;
    objectValue["30"] = object.type;
    objectValue["31"] = object.by.toValue();
    if (object.mode !== null) {
      objectValue["32"] = object.mode;
    }
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Sort {
    const modeValue = objectValue["32"];
    const unpackedMode = modeValue !== undefined ? Number(modeValue) : null;
    return new Sort({
      type: Number(objectValue["30"]),
      by: Expression.fromValue(objectValue["31"], _session, _supergraph, _graph, _connection),
      mode: unpackedMode,
      _value: objectValue,
      _supergraph,
    });
  }

  static fromValue(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Sort {
    return Sort.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
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
    _hash?: number | null;
    _repr?: string | null;
    _proto?: any | null;
    _value?: { [key: string]: any } | null;
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
    // @ts-expect-error(readonly)
    this._hash = options._hash ?? null;
    // @ts-expect-error(readonly)
    this._repr = options._repr ?? null;
    // @ts-expect-error(readonly)
    this._proto = options._proto ?? null;
    // @ts-expect-error(readonly)
    this._value = options._value ?? null;
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

  toValue(): { [key: string]: any } {
    if (this._value === null) {
      // @ts-expect-error(readonly)
      this._value = Select.__packValue__(this);
    }
    return this._value;
  }

  static __packValue__(object: Select): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 50106;
    if (object.attributes) {
      const packedAttributes: any[] = [];
      for (const item of object.attributes) {
        packedAttributes.push(item.toValue());
      }
      objectValue["31"] = packedAttributes;
    }
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Select {
    const unpackedAttributes: any[] = [];
    if (objectValue["31"] !== undefined) {
      for (const item of objectValue["31"]) {
        unpackedAttributes.push(AttributeReference.fromValue(item, _session, _supergraph, _graph, _connection));
      }
    }
    return new Select({
      attributes: unpackedAttributes,
      _value: objectValue,
      _supergraph,
    });
  }

  static fromValue(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Select {
    return Select.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
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
    _hash?: number | null;
    _repr?: string | null;
    _proto?: any | null;
    _value?: { [key: string]: any } | null;
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
    // @ts-expect-error(readonly)
    this._hash = options._hash ?? null;
    // @ts-expect-error(readonly)
    this._repr = options._repr ?? null;
    // @ts-expect-error(readonly)
    this._proto = options._proto ?? null;
    // @ts-expect-error(readonly)
    this._value = options._value ?? null;
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

  toValue(): { [key: string]: any } {
    if (this._value === null) {
      // @ts-expect-error(readonly)
      this._value = Join.__packValue__(this);
    }
    return this._value;
  }

  static __packValue__(object: Join): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 50102;
    objectValue["30"] = object.type;
    if (object.relation !== null) {
      objectValue["31"] = object.relation.toValue();
    }
    objectValue["33"] = object.recursive;
    if (object.depth !== null) {
      objectValue["34"] = object.depth;
    }
    if (object.on !== null) {
      objectValue["35"] = object.on.toValue();
    }
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Join {
    const relationValue = objectValue["31"];
    const unpackedRelation =
      relationValue !== undefined
        ? RelationReference.fromValue(relationValue, _session, _supergraph, _graph, _connection)
        : null;
    const depthValue = objectValue["34"];
    const unpackedDepth = depthValue !== undefined ? Number(depthValue) : null;
    const onValue = objectValue["35"];
    const unpackedOn =
      onValue !== undefined ? Condition.fromValue(onValue, _session, _supergraph, _graph, _connection) : null;
    return new Join({
      type: Number(objectValue["30"]),
      relation: unpackedRelation,
      recursive: objectValue["33"],
      depth: unpackedDepth,
      on: unpackedOn,
      _value: objectValue,
      _supergraph,
    });
  }

  static fromValue(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Join {
    return Join.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
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
    _hash?: number | null;
    _repr?: string | null;
    _proto?: any | null;
    _value?: { [key: string]: any } | null;
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
    // @ts-expect-error(readonly)
    this._hash = options._hash ?? null;
    // @ts-expect-error(readonly)
    this._repr = options._repr ?? null;
    // @ts-expect-error(readonly)
    this._proto = options._proto ?? null;
    // @ts-expect-error(readonly)
    this._value = options._value ?? null;
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

  toValue(): { [key: string]: any } {
    if (this._value === null) {
      // @ts-expect-error(readonly)
      this._value = Query.__packValue__(this);
    }
    return this._value;
  }

  static __packValue__(object: Query): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 50110;
    objectValue["2"] = String(object.id);
    objectValue["30"] = object.type;
    objectValue["31"] = object.name;
    objectValue["32"] = object.relation.toValue();
    if (object.join !== null) {
      objectValue["33"] = object.join.toValue();
    }
    if (object.select !== null) {
      objectValue["34"] = object.select.toValue();
    }
    if (object.subqueries) {
      const packedSubqueries: any[] = [];
      for (const item of object.subqueries) {
        packedSubqueries.push(item.toValue());
      }
      objectValue["35"] = packedSubqueries;
    }
    if (object.where !== null) {
      objectValue["40"] = object.where.toValue();
    }
    if (object.having !== null) {
      objectValue["41"] = object.having.toValue();
    }
    if (object.groupBy) {
      const packedGroupBy: any[] = [];
      for (const item of object.groupBy) {
        packedGroupBy.push(item.toValue());
      }
      objectValue["42"] = packedGroupBy;
    }
    if (object.aggregation !== null) {
      objectValue["43"] = object.aggregation.toValue();
    }
    if (object.sort) {
      const packedSort: any[] = [];
      for (const item of object.sort) {
        packedSort.push(item.toValue());
      }
      objectValue["44"] = packedSort;
    }
    if (object.limit !== null) {
      objectValue["50"] = object.limit;
    }
    if (object.offset !== null) {
      objectValue["51"] = object.offset;
    }
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Query {
    const joinValue = objectValue["33"];
    const unpackedJoin =
      joinValue !== undefined ? Join.fromValue(joinValue, _session, _supergraph, _graph, _connection) : null;
    const selectValue = objectValue["34"];
    const unpackedSelect =
      selectValue !== undefined ? Select.fromValue(selectValue, _session, _supergraph, _graph, _connection) : null;
    const unpackedSubqueries: any[] = [];
    if (objectValue["35"] !== undefined) {
      for (const item of objectValue["35"]) {
        unpackedSubqueries.push(Query.fromValue(item, _session, _supergraph, _graph, _connection));
      }
    }
    const whereValue = objectValue["40"];
    const unpackedWhere =
      whereValue !== undefined ? Condition.fromValue(whereValue, _session, _supergraph, _graph, _connection) : null;
    const havingValue = objectValue["41"];
    const unpackedHaving =
      havingValue !== undefined ? Condition.fromValue(havingValue, _session, _supergraph, _graph, _connection) : null;
    const unpackedGroupBy: any[] = [];
    if (objectValue["42"] !== undefined) {
      for (const item of objectValue["42"]) {
        unpackedGroupBy.push(Expression.fromValue(item, _session, _supergraph, _graph, _connection));
      }
    }
    const aggregationValue = objectValue["43"];
    const unpackedAggregation =
      aggregationValue !== undefined
        ? Aggregation.fromValue(aggregationValue, _session, _supergraph, _graph, _connection)
        : null;
    const unpackedSort: any[] = [];
    if (objectValue["44"] !== undefined) {
      for (const item of objectValue["44"]) {
        unpackedSort.push(Sort.fromValue(item, _session, _supergraph, _graph, _connection));
      }
    }
    const limitValue = objectValue["50"];
    const unpackedLimit = limitValue !== undefined ? Number(limitValue) : null;
    const offsetValue = objectValue["51"];
    const unpackedOffset = offsetValue !== undefined ? Number(offsetValue) : null;
    return new Query({
      id: String(objectValue["2"]),
      type: Number(objectValue["30"]),
      name: objectValue["31"],
      relation: RelationReference.fromValue(objectValue["32"], _session, _supergraph, _graph, _connection),
      join: unpackedJoin,
      select: unpackedSelect,
      subqueries: unpackedSubqueries,
      where: unpackedWhere,
      having: unpackedHaving,
      groupBy: unpackedGroupBy,
      aggregation: unpackedAggregation,
      sort: unpackedSort,
      limit: unpackedLimit,
      offset: unpackedOffset,
      _value: objectValue,
      _supergraph,
    });
  }

  static fromValue(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Query {
    return Query.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
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
    _hash?: number | null;
    _repr?: string | null;
    _proto?: any | null;
    _value?: { [key: string]: any } | null;
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
    // @ts-expect-error(readonly)
    this._hash = options._hash ?? null;
    // @ts-expect-error(readonly)
    this._repr = options._repr ?? null;
    // @ts-expect-error(readonly)
    this._proto = options._proto ?? null;
    // @ts-expect-error(readonly)
    this._value = options._value ?? null;
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

  toValue(): { [key: string]: any } {
    if (this._value === null) {
      // @ts-expect-error(readonly)
      this._value = Histogram.__packValue__(this);
    }
    return this._value;
  }

  static __packValue__(object: Histogram): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 50114;
    if (object.buckets) {
      const packedBuckets: any[] = [];
      for (const item of object.buckets) {
        packedBuckets.push(item.toValue());
      }
      objectValue["40"] = packedBuckets;
    }
    if (object.counts) {
      const packedCounts: any[] = [];
      for (const item of object.counts) {
        packedCounts.push(item);
      }
      objectValue["41"] = packedCounts;
    }
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Histogram {
    const unpackedBuckets: any[] = [];
    if (objectValue["40"] !== undefined) {
      for (const item of objectValue["40"]) {
        unpackedBuckets.push(Value.fromValue(item, _session, _supergraph, _graph, _connection));
      }
    }
    const unpackedCounts: any[] = [];
    if (objectValue["41"] !== undefined) {
      for (const item of objectValue["41"]) {
        unpackedCounts.push(Number(item));
      }
    }
    return new Histogram({
      buckets: unpackedBuckets,
      counts: unpackedCounts,
      _value: objectValue,
      _supergraph,
    });
  }

  static fromValue(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Histogram {
    return Histogram.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
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

  toValue(): { [key: string]: any } {
    return QueryResult.__packValue__(this);
  }

  static __packValue__(object: QueryResult): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 50111;
    objectValue["2"] = String(object.id);
    objectValue["30"] = object.type;
    if (object.groups) {
      const packedGroups: any[] = [];
      for (const item of object.groups) {
        packedGroups.push(item.toValue());
      }
      objectValue["35"] = packedGroups;
    }
    if (object.subresults) {
      const packedSubresults: any[] = [];
      for (const item of object.subresults) {
        packedSubresults.push(item.toValue());
      }
      objectValue["36"] = packedSubresults;
    }
    if (object.nodes) {
      const packedNodes: any[] = [];
      for (const item of object.nodes) {
        packedNodes.push(item.toValue());
      }
      objectValue["40"] = packedNodes;
    }
    if (object.count !== null) {
      objectValue["41"] = object.count;
    }
    if (object.exists !== null) {
      objectValue["42"] = object.exists;
    }
    if (object.scalar !== null) {
      objectValue["43"] = object.scalar.toValue();
    }
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): QueryResult {
    const unpackedGroups: any[] = [];
    if (objectValue["35"] !== undefined) {
      for (const item of objectValue["35"]) {
        unpackedGroups.push(QueryResultGroup.fromValue(item, _session, _supergraph, _graph, _connection));
      }
    }
    const unpackedSubresults: any[] = [];
    if (objectValue["36"] !== undefined) {
      for (const item of objectValue["36"]) {
        unpackedSubresults.push(QueryResult.fromValue(item, _session, _supergraph, _graph, _connection));
      }
    }
    const unpackedNodes: any[] = [];
    if (objectValue["40"] !== undefined) {
      for (const item of objectValue["40"]) {
        unpackedNodes.push(Value.fromValue(item, _session, _supergraph, _graph, _connection));
      }
    }
    const countValue = objectValue["41"];
    const unpackedCount = countValue !== undefined ? Number(countValue) : null;
    const existsValue = objectValue["42"];
    const unpackedExists = existsValue !== undefined ? existsValue : null;
    const scalarValue = objectValue["43"];
    const unpackedScalar =
      scalarValue !== undefined ? Value.fromValue(scalarValue, _session, _supergraph, _graph, _connection) : null;
    return new QueryResult({
      id: String(objectValue["2"]),
      groups: unpackedGroups,
      subresults: unpackedSubresults,
      type: Number(objectValue["30"]),
      nodes: unpackedNodes,
      count: unpackedCount,
      exists: unpackedExists,
      scalar: unpackedScalar,
      _supergraph,
    });
  }

  static fromValue(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): QueryResult {
    return QueryResult.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
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

  toValue(): { [key: string]: any } {
    return QueryResultGroup.__packValue__(this);
  }

  static __packValue__(object: QueryResultGroup): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 50112;
    objectValue["30"] = object.type;
    objectValue["31"] = object.discriminator.toValue();
    if (object.nodes) {
      const packedNodes: any[] = [];
      for (const item of object.nodes) {
        packedNodes.push(item.toValue());
      }
      objectValue["40"] = packedNodes;
    }
    if (object.count !== null) {
      objectValue["41"] = object.count;
    }
    if (object.exists !== null) {
      objectValue["42"] = object.exists;
    }
    if (object.scalar !== null) {
      objectValue["43"] = object.scalar.toValue();
    }
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): QueryResultGroup {
    const unpackedNodes: any[] = [];
    if (objectValue["40"] !== undefined) {
      for (const item of objectValue["40"]) {
        unpackedNodes.push(Value.fromValue(item, _session, _supergraph, _graph, _connection));
      }
    }
    const countValue = objectValue["41"];
    const unpackedCount = countValue !== undefined ? Number(countValue) : null;
    const existsValue = objectValue["42"];
    const unpackedExists = existsValue !== undefined ? existsValue : null;
    const scalarValue = objectValue["43"];
    const unpackedScalar =
      scalarValue !== undefined ? Value.fromValue(scalarValue, _session, _supergraph, _graph, _connection) : null;
    return new QueryResultGroup({
      discriminator: Value.fromValue(objectValue["31"], _session, _supergraph, _graph, _connection),
      type: Number(objectValue["30"]),
      nodes: unpackedNodes,
      count: unpackedCount,
      exists: unpackedExists,
      scalar: unpackedScalar,
      _supergraph,
    });
  }

  static fromValue(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): QueryResultGroup {
    return QueryResultGroup.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
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
    _hash?: number | null;
    _repr?: string | null;
    _proto?: any | null;
    _value?: { [key: string]: any } | null;
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
    // @ts-expect-error(readonly)
    this._hash = options._hash ?? null;
    // @ts-expect-error(readonly)
    this._repr = options._repr ?? null;
    // @ts-expect-error(readonly)
    this._proto = options._proto ?? null;
    // @ts-expect-error(readonly)
    this._value = options._value ?? null;
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

  toValue(): { [key: string]: any } {
    if (this._value === null) {
      // @ts-expect-error(readonly)
      this._value = QueryUpdate.__packValue__(this);
    }
    return this._value;
  }

  static __packValue__(object: QueryUpdate): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 50113;
    objectValue["30"] = object.type;
    if (object.result !== null) {
      objectValue["40"] = object.result.toValue();
    }
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): QueryUpdate {
    const resultValue = objectValue["40"];
    const unpackedResult =
      resultValue !== undefined ? QueryResult.fromValue(resultValue, _session, _supergraph, _graph, _connection) : null;
    return new QueryUpdate({
      type: Number(objectValue["30"]),
      result: unpackedResult,
      _value: objectValue,
      _supergraph,
    });
  }

  static fromValue(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): QueryUpdate {
    return QueryUpdate.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
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

  constructor(options: {
    _session?: Session | null;
    _supergraph?: Supergraph | null;
    _hash?: number | null;
    _repr?: string | null;
    _proto?: any | null;
    _value?: { [key: string]: any } | null;
  }) {
    super(
      // session
      options._session ?? null,
      // supergraph
      options._supergraph ?? null,
    );

    // properties

    // identity
    // @ts-expect-error(readonly)
    this._hash = options._hash ?? null;
    // @ts-expect-error(readonly)
    this._repr = options._repr ?? null;
    // @ts-expect-error(readonly)
    this._proto = options._proto ?? null;
    // @ts-expect-error(readonly)
    this._value = options._value ?? null;
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

  toValue(): { [key: string]: any } {
    if (this._value === null) {
      // @ts-expect-error(readonly)
      this._value = Selection.__packValue__(this);
    }
    return this._value;
  }

  static __packValue__(object: Selection): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 2571;
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Selection {
    return new Selection({
      _value: objectValue,
      _supergraph,
    });
  }

  static fromValue(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Selection {
    return Selection.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
  }
}
/* ==== DESTACK_GENERATED_END:STRUCT:2571 ==== */
