import { EnumType, NodeType, StructType } from "@destack/language/core/builtin/common";
import { activeSession } from "@destack/language/core/builtin/const";
import type { CustomEntityDefinition, Snapshot } from "@destack/language/core/builtin/entity";
import type { NodeClass } from "@destack/language/core/builtin/node";
import { Node, isNode } from "@destack/language/core/builtin/node";
import type {
  NodeDefinitionReference,
  NodeReference,
  PropertyReference,
} from "@destack/language/core/builtin/relation";
import { Struct, StructFrozen, isStruct } from "@destack/language/core/builtin/struct";
import type { PropertyDefinition } from "@destack/language/core/common/definition";
import type { CustomProperty } from "@destack/language/core/common/property";
import type { Value } from "@destack/language/core/common/value";
import { toValue } from "@destack/language/core/common/value";
import { QueryConnection } from "@destack/language/core/runtime/connection";
import type { Supergraph } from "@destack/language/core/runtime/graph";
import type { Session } from "@destack/language/core/runtime/session";
import {
  STRUCT_CLASS_BY_TYPE,
  registerEnumClass,
  registerStructClass,
} from "@destack/language/registry";
import {
  AggregationProto,
  AggregationTypeProto,
  ConditionProto,
  ConditionalTypeProto,
  ExpressionProto,
  ExpressionTypeProto,
  FunctionProto,
  FunctionTypeProto,
  HistogramProto,
  JoinProto,
  JoinTypeProto,
  QueryProto,
  QueryResultGroupProto,
  QueryResultProto,
  QueryTypeProto,
  QueryUpdateProto,
  QueryUpdateTypeProto,
  SelectProto,
  SelectionProto,
  SortModeProto,
  SortProto,
  SortTypeProto,
} from "@destack/proto";
import { base64Decode } from "@destack/utils";
import { assertNever } from "@destack/utils/functools";
import { hashBool, hashInt, hashString } from "@destack/utils/hash";
import { v4 as uuid4 } from "uuid";

/* ==== DESTACK_GENERATED_START:STRUCT:501 ==== */
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
    if (!(this.metatype === other.metatype)) {
      return false;
    }
    if (!(this.type === other.type)) {
      return false;
    }
    if (!this.left.equals(other.left)) {
      return false;
    }
    if (
      (this.right == null) !== (other.right == null) ||
      (this.right != null && !this.right.equals(other.right))
    ) {
      return false;
    }
    return true;
  }

  repr(): string {
    if (this._repr === null) {
      const propertyReprs: string[] = [];
      propertyReprs.push(`type=${FunctionType[this.type]}`);
      propertyReprs.push(`left=${this.left.repr()}`);
      if (this.right !== null) {
        propertyReprs.push(`right=${this.right.repr()}`);
      }
      // @ts-expect-error(readonly)
      this._repr = `<Function ${propertyReprs.join(" ")}>`;
    }
    return this._repr;
  }

  hash(): number {
    if (this._hash !== null) {
      return this._hash;
    }

    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    h = (h * 31 + this.type) & 0xffffffff;
    h = (h * 31 + this.left.hash()) & 0xffffffff;
    if (this.right !== null) {
      h = (h * 31 + this.right.hash()) & 0xffffffff;
    }

    // @ts-expect-error(readonly)
    this._hash = h;
    return h;
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
    objectValue["1"] = 501;
    objectValue["100"] = object.type;
    objectValue["101"] = object.left.toValue();
    if (object.right != null) {
      objectValue["102"] = object.right.toValue();
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
    const _Expression = STRUCT_CLASS_BY_TYPE[StructType.EXPRESSION] as typeof Expression;
    const rightValue = objectValue["102"];
    const unpackedRight =
      rightValue != undefined
        ? _Expression.fromValue(rightValue, _session, _supergraph, _graph, _connection)
        : null;
    return new Function({
      type: Number(objectValue["100"]),
      left: _Expression.fromValue(objectValue["101"], _session, _supergraph, _graph, _connection),
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

  toProto(): FunctionProto {
    if (this._proto === null) {
      // @ts-expect-error(readonly)
      this._proto = Function.__packProto__(this);
    }
    return this._proto as FunctionProto;
  }

  static __packProto__(object: Function): FunctionProto {
    const objectProto: Partial<FunctionProto> = { metatype: 501 };
    objectProto.type = Number(object.type) as FunctionTypeProto;
    objectProto.left = object.left.toProto();
    if (object.right != null) {
      objectProto.right = object.right.toProto();
    }
    return objectProto as FunctionProto;
  }

  static __unpackProto__(
    objectProto: FunctionProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Function {
    const _Expression = STRUCT_CLASS_BY_TYPE[StructType.EXPRESSION] as typeof Expression;
    return new Function({
      type: Number(objectProto.type) as FunctionType,
      left: _Expression.fromProto(objectProto.left!, _session, _supergraph, _graph, _connection),
      right:
        objectProto.right != undefined
          ? _Expression.fromProto(objectProto.right!, _session, _supergraph, _graph, _connection)
          : null,
      _proto: objectProto,
      _supergraph,
    });
  }

  static fromProto(
    objectProto: FunctionProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Function {
    return Function.__unpackProto__(objectProto, _session, _supergraph, _graph, _connection);
  }

  static fromProtoString(packedProtoString: string): Function {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = FunctionProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */

  /** Make a Function from a shorthand expression. */
  static of(type: FunctionType, left: Expression, right?: Expression | null): Function {
    return new Function({ type, left, right: right ?? null });
  }

  /* ==== DESTACK_CUSTOM_END ==== */
}
registerStructClass(StructType.FUNCTION, Function);
/* ==== DESTACK_GENERATED_END:STRUCT:501 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:504 ==== */
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
    if (!(this.metatype === other.metatype)) {
      return false;
    }
    if (!(this.type === other.type)) {
      return false;
    }
    if (!this.left.equals(other.left)) {
      return false;
    }
    if (
      (this.right == null) !== (other.right == null) ||
      (this.right != null && !this.right.equals(other.right))
    ) {
      return false;
    }
    return true;
  }

  repr(): string {
    if (this._repr === null) {
      const propertyReprs: string[] = [];
      propertyReprs.push(`type=${ConditionalType[this.type]}`);
      propertyReprs.push(`left=${this.left.repr()}`);
      if (this.right !== null) {
        propertyReprs.push(`right=${this.right.repr()}`);
      }
      // @ts-expect-error(readonly)
      this._repr = `<Condition ${propertyReprs.join(" ")}>`;
    }
    return this._repr;
  }

  hash(): number {
    if (this._hash !== null) {
      return this._hash;
    }

    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    h = (h * 31 + this.type) & 0xffffffff;
    h = (h * 31 + this.left.hash()) & 0xffffffff;
    if (this.right !== null) {
      h = (h * 31 + this.right.hash()) & 0xffffffff;
    }

    // @ts-expect-error(readonly)
    this._hash = h;
    return h;
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
    objectValue["1"] = 504;
    objectValue["100"] = object.type;
    objectValue["101"] = object.left.toValue();
    if (object.right != null) {
      objectValue["102"] = object.right.toValue();
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
    const _Expression = STRUCT_CLASS_BY_TYPE[StructType.EXPRESSION] as typeof Expression;
    const rightValue = objectValue["102"];
    const unpackedRight =
      rightValue != undefined
        ? _Expression.fromValue(rightValue, _session, _supergraph, _graph, _connection)
        : null;
    return new Condition({
      type: Number(objectValue["100"]),
      left: _Expression.fromValue(objectValue["101"], _session, _supergraph, _graph, _connection),
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

  toProto(): ConditionProto {
    if (this._proto === null) {
      // @ts-expect-error(readonly)
      this._proto = Condition.__packProto__(this);
    }
    return this._proto as ConditionProto;
  }

  static __packProto__(object: Condition): ConditionProto {
    const objectProto: Partial<ConditionProto> = { metatype: 504 };
    objectProto.type = Number(object.type) as ConditionalTypeProto;
    objectProto.left = object.left.toProto();
    if (object.right != null) {
      objectProto.right = object.right.toProto();
    }
    return objectProto as ConditionProto;
  }

  static __unpackProto__(
    objectProto: ConditionProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Condition {
    const _Expression = STRUCT_CLASS_BY_TYPE[StructType.EXPRESSION] as typeof Expression;
    return new Condition({
      type: Number(objectProto.type) as ConditionalType,
      left: _Expression.fromProto(objectProto.left!, _session, _supergraph, _graph, _connection),
      right:
        objectProto.right != undefined
          ? _Expression.fromProto(objectProto.right!, _session, _supergraph, _graph, _connection)
          : null,
      _proto: objectProto,
      _supergraph,
    });
  }

  static fromProto(
    objectProto: ConditionProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Condition {
    return Condition.__unpackProto__(objectProto, _session, _supergraph, _graph, _connection);
  }

  static fromProtoString(packedProtoString: string): Condition {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = ConditionProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */

  /** OR two Conditions. */
  or(right: Condition): Condition {
    return new Condition({
      type: ConditionalType.OR,
      left: Expression.of(this),
      right: Expression.of(right),
    });
  }

  /** AND two Conditions. */
  and(right: Condition): Condition {
    return new Condition({
      type: ConditionalType.AND,
      left: Expression.of(this),
      right: Expression.of(right),
    });
  }

  /** Make a Condition from a shorthand expression. */
  static of(
    attribute: CustomProperty | PropertyReference | PropertyDefinition,
    type: ConditionalType = ConditionalType.EQUALS,
    value: any = null,
  ): Condition {
    const left = Expression.of(attribute);
    const right = Expression.of(toValue(value));
    return new Condition({ type, left, right });
  }

  /* ==== DESTACK_CUSTOM_END ==== */
}
registerStructClass(StructType.CONDITION, Condition);
/* ==== DESTACK_GENERATED_END:STRUCT:504 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:503 ==== */
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
    if (!(this.metatype === other.metatype)) {
      return false;
    }
    if (!(this.type === other.type)) {
      return false;
    }
    if (
      (this.expression == null) !== (other.expression == null) ||
      (this.expression != null && !this.expression.equals(other.expression))
    ) {
      return false;
    }
    return true;
  }

  repr(): string {
    if (this._repr === null) {
      const propertyReprs: string[] = [];
      propertyReprs.push(`type=${AggregationType[this.type]}`);
      if (this.expression !== null) {
        propertyReprs.push(`expression=${this.expression.repr()}`);
      }
      // @ts-expect-error(readonly)
      this._repr = `<Aggregation ${propertyReprs.join(" ")}>`;
    }
    return this._repr;
  }

  hash(): number {
    if (this._hash !== null) {
      return this._hash;
    }

    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    h = (h * 31 + this.type) & 0xffffffff;
    if (this.expression !== null) {
      h = (h * 31 + this.expression.hash()) & 0xffffffff;
    }

    // @ts-expect-error(readonly)
    this._hash = h;
    return h;
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
    objectValue["1"] = 503;
    objectValue["100"] = object.type;
    if (object.expression != null) {
      objectValue["101"] = object.expression.toValue();
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
    const _Expression = STRUCT_CLASS_BY_TYPE[StructType.EXPRESSION] as typeof Expression;
    const expressionValue = objectValue["101"];
    const unpackedExpression =
      expressionValue != undefined
        ? _Expression.fromValue(expressionValue, _session, _supergraph, _graph, _connection)
        : null;
    return new Aggregation({
      type: Number(objectValue["100"]),
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

  toProto(): AggregationProto {
    if (this._proto === null) {
      // @ts-expect-error(readonly)
      this._proto = Aggregation.__packProto__(this);
    }
    return this._proto as AggregationProto;
  }

  static __packProto__(object: Aggregation): AggregationProto {
    const objectProto: Partial<AggregationProto> = { metatype: 503 };
    objectProto.type = Number(object.type) as AggregationTypeProto;
    if (object.expression != null) {
      objectProto.expression = object.expression.toProto();
    }
    return objectProto as AggregationProto;
  }

  static __unpackProto__(
    objectProto: AggregationProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Aggregation {
    const _Expression = STRUCT_CLASS_BY_TYPE[StructType.EXPRESSION] as typeof Expression;
    return new Aggregation({
      type: Number(objectProto.type) as AggregationType,
      expression:
        objectProto.expression != undefined
          ? _Expression.fromProto(
              objectProto.expression!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      _proto: objectProto,
      _supergraph,
    });
  }

  static fromProto(
    objectProto: AggregationProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Aggregation {
    return Aggregation.__unpackProto__(objectProto, _session, _supergraph, _graph, _connection);
  }

  static fromProtoString(packedProtoString: string): Aggregation {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = AggregationProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */

  /** Make an Aggregation from a shorthand expression. */
  static of(type: AggregationType, operand?: Expression | null): Aggregation {
    return new Aggregation({ type, expression: operand ?? null });
  }

  /* ==== DESTACK_CUSTOM_END ==== */
}
registerStructClass(StructType.AGGREGATION, Aggregation);
/* ==== DESTACK_GENERATED_END:STRUCT:503 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:500 ==== */
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
  readonly attribute: PropertyReference | null;

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
    attribute?: PropertyReference | null;
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
    if (!(this.metatype === other.metatype)) {
      return false;
    }
    if (!(this.type === other.type)) {
      return false;
    }
    if (
      (this.literal == null) !== (other.literal == null) ||
      (this.literal != null && !this.literal.equals(other.literal))
    ) {
      return false;
    }
    if (
      (this.attribute == null) !== (other.attribute == null) ||
      (this.attribute != null && !this.attribute.equals(other.attribute))
    ) {
      return false;
    }
    if (
      (this.condition == null) !== (other.condition == null) ||
      (this.condition != null && !this.condition.equals(other.condition))
    ) {
      return false;
    }
    if (
      (this.function == null) !== (other.function == null) ||
      (this.function != null && !this.function.equals(other.function))
    ) {
      return false;
    }
    if (
      (this.aggregation == null) !== (other.aggregation == null) ||
      (this.aggregation != null && !this.aggregation.equals(other.aggregation))
    ) {
      return false;
    }
    return true;
  }

  repr(): string {
    if (this._repr === null) {
      const propertyReprs: string[] = [];
      propertyReprs.push(`type=${ExpressionType[this.type]}`);
      if (this.literal !== null) {
        propertyReprs.push(`literal=${this.literal.repr()}`);
      }
      if (this.attribute !== null) {
        propertyReprs.push(`attribute=${this.attribute.repr()}`);
      }
      if (this.condition !== null) {
        propertyReprs.push(`condition=${this.condition.repr()}`);
      }
      if (this.function !== null) {
        propertyReprs.push(`function=${this.function.repr()}`);
      }
      if (this.aggregation !== null) {
        propertyReprs.push(`aggregation=${this.aggregation.repr()}`);
      }
      // @ts-expect-error(readonly)
      this._repr = `<Expression ${propertyReprs.join(" ")}>`;
    }
    return this._repr;
  }

  hash(): number {
    if (this._hash !== null) {
      return this._hash;
    }

    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    h = (h * 31 + this.type) & 0xffffffff;
    if (this.literal !== null) {
      h = (h * 31 + this.literal.hash()) & 0xffffffff;
    }
    if (this.attribute !== null) {
      h = (h * 31 + this.attribute.hash()) & 0xffffffff;
    }
    if (this.condition !== null) {
      h = (h * 31 + this.condition.hash()) & 0xffffffff;
    }
    if (this.function !== null) {
      h = (h * 31 + this.function.hash()) & 0xffffffff;
    }
    if (this.aggregation !== null) {
      h = (h * 31 + this.aggregation.hash()) & 0xffffffff;
    }

    // @ts-expect-error(readonly)
    this._hash = h;
    return h;
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
    objectValue["1"] = 500;
    objectValue["100"] = object.type;
    if (object.literal != null) {
      objectValue["101"] = object.literal.toValue();
    }
    if (object.attribute != null) {
      objectValue["102"] = object.attribute.toValue();
    }
    if (object.condition != null) {
      objectValue["103"] = object.condition.toValue();
    }
    if (object.function != null) {
      objectValue["104"] = object.function.toValue();
    }
    if (object.aggregation != null) {
      objectValue["105"] = object.aggregation.toValue();
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
    const _PropertyReference = STRUCT_CLASS_BY_TYPE[
      StructType.PROPERTY_REFERENCE
    ] as typeof PropertyReference;
    const _Function = STRUCT_CLASS_BY_TYPE[StructType.FUNCTION] as typeof Function;
    const _Aggregation = STRUCT_CLASS_BY_TYPE[StructType.AGGREGATION] as typeof Aggregation;
    const _Condition = STRUCT_CLASS_BY_TYPE[StructType.CONDITION] as typeof Condition;
    const _Value = STRUCT_CLASS_BY_TYPE[StructType.VALUE] as typeof Value;
    const literalValue = objectValue["101"];
    const unpackedLiteral =
      literalValue != undefined
        ? _Value.fromValue(literalValue, _session, _supergraph, _graph, _connection)
        : null;
    const attributeValue = objectValue["102"];
    const unpackedAttribute =
      attributeValue != undefined
        ? _PropertyReference.fromValue(attributeValue, _session, _supergraph, _graph, _connection)
        : null;
    const conditionValue = objectValue["103"];
    const unpackedCondition =
      conditionValue != undefined
        ? _Condition.fromValue(conditionValue, _session, _supergraph, _graph, _connection)
        : null;
    const functionValue = objectValue["104"];
    const unpackedFunction =
      functionValue != undefined
        ? _Function.fromValue(functionValue, _session, _supergraph, _graph, _connection)
        : null;
    const aggregationValue = objectValue["105"];
    const unpackedAggregation =
      aggregationValue != undefined
        ? _Aggregation.fromValue(aggregationValue, _session, _supergraph, _graph, _connection)
        : null;
    return new Expression({
      type: Number(objectValue["100"]),
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

  toProto(): ExpressionProto {
    if (this._proto === null) {
      // @ts-expect-error(readonly)
      this._proto = Expression.__packProto__(this);
    }
    return this._proto as ExpressionProto;
  }

  static __packProto__(object: Expression): ExpressionProto {
    const objectProto: Partial<ExpressionProto> = { metatype: 500 };
    objectProto.type = Number(object.type) as ExpressionTypeProto;
    if (object.literal != null) {
      objectProto.literal = object.literal.toProto();
    }
    if (object.attribute != null) {
      objectProto.attribute = object.attribute.toProto();
    }
    if (object.condition != null) {
      objectProto.condition = object.condition.toProto();
    }
    if (object.function != null) {
      objectProto.function = object.function.toProto();
    }
    if (object.aggregation != null) {
      objectProto.aggregation = object.aggregation.toProto();
    }
    return objectProto as ExpressionProto;
  }

  static __unpackProto__(
    objectProto: ExpressionProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Expression {
    const _PropertyReference = STRUCT_CLASS_BY_TYPE[
      StructType.PROPERTY_REFERENCE
    ] as typeof PropertyReference;
    const _Function = STRUCT_CLASS_BY_TYPE[StructType.FUNCTION] as typeof Function;
    const _Aggregation = STRUCT_CLASS_BY_TYPE[StructType.AGGREGATION] as typeof Aggregation;
    const _Condition = STRUCT_CLASS_BY_TYPE[StructType.CONDITION] as typeof Condition;
    const _Value = STRUCT_CLASS_BY_TYPE[StructType.VALUE] as typeof Value;
    return new Expression({
      type: Number(objectProto.type) as ExpressionType,
      literal:
        objectProto.literal != undefined
          ? _Value.fromProto(objectProto.literal!, _session, _supergraph, _graph, _connection)
          : null,
      attribute:
        objectProto.attribute != undefined
          ? _PropertyReference.fromProto(
              objectProto.attribute!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      condition:
        objectProto.condition != undefined
          ? _Condition.fromProto(objectProto.condition!, _session, _supergraph, _graph, _connection)
          : null,
      function:
        objectProto.function != undefined
          ? _Function.fromProto(objectProto.function!, _session, _supergraph, _graph, _connection)
          : null,
      aggregation:
        objectProto.aggregation != undefined
          ? _Aggregation.fromProto(
              objectProto.aggregation!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      _proto: objectProto,
      _supergraph,
    });
  }

  static fromProto(
    objectProto: ExpressionProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Expression {
    return Expression.__unpackProto__(objectProto, _session, _supergraph, _graph, _connection);
  }

  static fromProtoString(packedProtoString: string): Expression {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = ExpressionProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */

  /** Make an Expression from a shorthand expression. */
  static of(thing: ExpressionIn): Expression {
    if (isStruct(thing, StructType.VALUE)) {
      return new Expression({ type: ExpressionType.LITERAL, literal: thing });
    } else if (isStruct(thing, StructType.PROPERTY_REFERENCE)) {
      return new Expression({ type: ExpressionType.ATTRIBUTE, attribute: thing });
    } else if (isStruct(thing, StructType.PROPERTY_DEFINITION)) {
      return new Expression({ type: ExpressionType.ATTRIBUTE, attribute: thing.toRef() });
    } else if (isStruct(thing, StructType.CONDITION)) {
      return new Expression({ type: ExpressionType.CONDITION, condition: thing });
    } else if (isStruct(thing, StructType.FUNCTION)) {
      return new Expression({ type: ExpressionType.FUNCTION, function: thing });
    } else if (isStruct(thing, StructType.AGGREGATION)) {
      return new Expression({ type: ExpressionType.AGGREGATION, aggregation: thing });
    } else if (isStruct(thing, StructType.EXPRESSION)) {
      return thing;
    } else if (isNode(thing, NodeType.CUSTOM_PROPERTY)) {
      const _PropertyReference = STRUCT_CLASS_BY_TYPE[
        StructType.PROPERTY_REFERENCE
      ] as typeof PropertyReference;
      return new Expression({
        type: ExpressionType.ATTRIBUTE,
        attribute: _PropertyReference.of(thing),
      });
    } else {
      assertNever(thing);
    }
  }

  /* ==== DESTACK_CUSTOM_END ==== */
}
registerStructClass(StructType.EXPRESSION, Expression);
/* ==== DESTACK_GENERATED_END:STRUCT:500 ==== */

export type ExpressionIn =
  | Value
  | CustomProperty
  | PropertyReference
  | PropertyDefinition
  | Condition
  | Function
  | Aggregation
  | Expression;

/* ==== DESTACK_GENERATED_START:STRUCT:505 ==== */
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
    if (!(this.metatype === other.metatype)) {
      return false;
    }
    if (!(this.type === other.type)) {
      return false;
    }
    if (!this.by.equals(other.by)) {
      return false;
    }
    if (!(this.mode === other.mode)) {
      return false;
    }
    return true;
  }

  repr(): string {
    if (this._repr === null) {
      const propertyReprs: string[] = [];
      propertyReprs.push(`type=${SortType[this.type]}`);
      propertyReprs.push(`by=${this.by.repr()}`);
      if (this.mode !== null) {
        propertyReprs.push(`mode=${SortMode[this.mode]}`);
      }
      // @ts-expect-error(readonly)
      this._repr = `<Sort ${propertyReprs.join(" ")}>`;
    }
    return this._repr;
  }

  hash(): number {
    if (this._hash !== null) {
      return this._hash;
    }

    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    h = (h * 31 + this.type) & 0xffffffff;
    h = (h * 31 + this.by.hash()) & 0xffffffff;
    if (this.mode !== null) {
      h = (h * 31 + this.mode) & 0xffffffff;
    }

    // @ts-expect-error(readonly)
    this._hash = h;
    return h;
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
    objectValue["1"] = 505;
    objectValue["100"] = object.type;
    objectValue["101"] = object.by.toValue();
    if (object.mode != null) {
      objectValue["102"] = object.mode;
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
    const _Expression = STRUCT_CLASS_BY_TYPE[StructType.EXPRESSION] as typeof Expression;
    const modeValue = objectValue["102"];
    const unpackedMode = modeValue != undefined ? Number(modeValue) : null;
    return new Sort({
      type: Number(objectValue["100"]),
      by: _Expression.fromValue(objectValue["101"], _session, _supergraph, _graph, _connection),
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

  toProto(): SortProto {
    if (this._proto === null) {
      // @ts-expect-error(readonly)
      this._proto = Sort.__packProto__(this);
    }
    return this._proto as SortProto;
  }

  static __packProto__(object: Sort): SortProto {
    const objectProto: Partial<SortProto> = { metatype: 505 };
    objectProto.type = Number(object.type) as SortTypeProto;
    objectProto.by = object.by.toProto();
    if (object.mode != null) {
      objectProto.mode = Number(object.mode) as SortModeProto;
    }
    return objectProto as SortProto;
  }

  static __unpackProto__(
    objectProto: SortProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Sort {
    const _Expression = STRUCT_CLASS_BY_TYPE[StructType.EXPRESSION] as typeof Expression;
    return new Sort({
      type: Number(objectProto.type) as SortType,
      by: _Expression.fromProto(objectProto.by!, _session, _supergraph, _graph, _connection),
      mode: objectProto.mode != undefined ? (Number(objectProto.mode) as SortMode) : null,
      _proto: objectProto,
      _supergraph,
    });
  }

  static fromProto(
    objectProto: SortProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Sort {
    return Sort.__unpackProto__(objectProto, _session, _supergraph, _graph, _connection);
  }

  static fromProtoString(packedProtoString: string): Sort {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = SortProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */

  /** Make a Sort from a shorthand expression. */
  static of(by: ExpressionIn, type: SortType = SortType.ASCENDING, mode?: SortMode | null): Sort {
    return new Sort({ type, by: Expression.of(by), mode: mode ?? null });
  }

  /* ==== DESTACK_CUSTOM_END ==== */
}
registerStructClass(StructType.SORT, Sort);
/* ==== DESTACK_GENERATED_END:STRUCT:505 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:506 ==== */
/**
 * Select specific Attributes.
 */
export class Select extends StructFrozen {
  static metatype: StructType = StructType.SELECT;
  static __isFrozen__: boolean = true;

  /**
   * Select.attributes
   */
  readonly attributes: Array<PropertyReference>;

  constructor(options: {
    attributes?: Array<PropertyReference>;
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
      _attributes = [];
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
    if (!(this.metatype === other.metatype)) {
      return false;
    }
    if (this.attributes.length !== other.attributes.length) {
      return false;
    }
    for (let i = 0; i < this.attributes.length; i++) {
      if (!this.attributes[i].equals(other.attributes[i])) {
        return false;
      }
    }
    return true;
  }

  repr(): string {
    if (this._repr === null) {
      const propertyReprs: string[] = [];
      if (this.attributes.length > 0) {
        propertyReprs.push(`attributes=${this.attributes.map((_item) => _item.repr()).join(", ")}`);
      }
      if (propertyReprs.length > 0) {
        // @ts-expect-error(readonly)
        this._repr = `<Select ${propertyReprs.join(" ")}>`;
      } else {
        // @ts-expect-error(readonly)
        this._repr = `<Select>`;
      }
    }
    return this._repr;
  }

  hash(): number {
    if (this._hash !== null) {
      return this._hash;
    }

    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    if (this.attributes && this.attributes.length > 0) {
      for (const _item of this.attributes) {
        h = (h * 31 + _item.hash()) & 0xffffffff;
      }
    }

    // @ts-expect-error(readonly)
    this._hash = h;
    return h;
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
    objectValue["1"] = 506;
    if (object.attributes.length > 0) {
      const packedAttributes: any[] = [];
      for (const item of object.attributes) {
        packedAttributes.push(item.toValue());
      }
      objectValue["101"] = packedAttributes;
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
    const _PropertyReference = STRUCT_CLASS_BY_TYPE[
      StructType.PROPERTY_REFERENCE
    ] as typeof PropertyReference;
    const unpackedAttributes: any[] = [];
    if (objectValue["101"] != undefined) {
      for (const item of objectValue["101"]) {
        unpackedAttributes.push(
          _PropertyReference.fromValue(item, _session, _supergraph, _graph, _connection),
        );
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

  toProto(): SelectProto {
    if (this._proto === null) {
      // @ts-expect-error(readonly)
      this._proto = Select.__packProto__(this);
    }
    return this._proto as SelectProto;
  }

  static __packProto__(object: Select): SelectProto {
    const objectProto: Partial<SelectProto> = { metatype: 506 };
    if (object.attributes) {
      const packedAttributes: any[] = [];
      for (const item of object.attributes) {
        packedAttributes.push(item.toProto());
      }
      objectProto.attributes = packedAttributes;
    }
    return objectProto as SelectProto;
  }

  static __unpackProto__(
    objectProto: SelectProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Select {
    const _PropertyReference = STRUCT_CLASS_BY_TYPE[
      StructType.PROPERTY_REFERENCE
    ] as typeof PropertyReference;
    const unpackedAttributes: any[] = [];
    if (objectProto.attributes) {
      for (const item of objectProto.attributes) {
        unpackedAttributes.push(
          _PropertyReference.fromProto(item!, _session, _supergraph, _graph, _connection),
        );
      }
    }
    return new Select({
      attributes: unpackedAttributes,
      _proto: objectProto,
      _supergraph,
    });
  }

  static fromProto(
    objectProto: SelectProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Select {
    return Select.__unpackProto__(objectProto, _session, _supergraph, _graph, _connection);
  }

  static fromProtoString(packedProtoString: string): Select {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = SelectProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */

  /** Make a Select from a shorthand expression. */
  static of(...attributes: (CustomProperty | PropertyReference)[]): Select {
    const _PropertyReference = STRUCT_CLASS_BY_TYPE[
      StructType.PROPERTY_REFERENCE
    ] as typeof PropertyReference;
    return new Select({
      attributes: attributes.map((attr) => _PropertyReference.of(attr)),
    });
  }

  /* ==== DESTACK_CUSTOM_END ==== */
}
registerStructClass(StructType.SELECT, Select);
/* ==== DESTACK_GENERATED_END:STRUCT:506 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:502 ==== */
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
   * Join.definition
   */
  readonly definition: NodeDefinitionReference | null;

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
    definition?: NodeDefinitionReference | null;
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
    let _definition = options.definition ?? null;
    this.definition = _definition;
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
    if (!(this.metatype === other.metatype)) {
      return false;
    }
    if (!(this.type === other.type)) {
      return false;
    }
    if (
      (this.definition == null) !== (other.definition == null) ||
      (this.definition != null && !this.definition.equals(other.definition))
    ) {
      return false;
    }
    if (!(this.recursive === other.recursive)) {
      return false;
    }
    if (!(this.depth === other.depth)) {
      return false;
    }
    if (
      (this.on == null) !== (other.on == null) ||
      (this.on != null && !this.on.equals(other.on))
    ) {
      return false;
    }
    return true;
  }

  repr(): string {
    if (this._repr === null) {
      const propertyReprs: string[] = [];
      propertyReprs.push(`type=${JoinType[this.type]}`);
      if (this.definition !== null) {
        propertyReprs.push(`definition=${this.definition.repr()}`);
      }
      propertyReprs.push(`recursive=${this.recursive}`);
      if (this.depth !== null) {
        propertyReprs.push(`depth=${this.depth}`);
      }
      if (this.on !== null) {
        propertyReprs.push(`on=${this.on.repr()}`);
      }
      // @ts-expect-error(readonly)
      this._repr = `<Join ${propertyReprs.join(" ")}>`;
    }
    return this._repr;
  }

  hash(): number {
    if (this._hash !== null) {
      return this._hash;
    }

    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    h = (h * 31 + this.type) & 0xffffffff;
    if (this.definition !== null) {
      h = (h * 31 + this.definition.hash()) & 0xffffffff;
    }
    h = (h * 31 + hashBool(this.recursive)) & 0xffffffff;
    if (this.depth !== null) {
      h = (h * 31 + hashInt(this.depth)) & 0xffffffff;
    }
    if (this.on !== null) {
      h = (h * 31 + this.on.hash()) & 0xffffffff;
    }

    // @ts-expect-error(readonly)
    this._hash = h;
    return h;
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
    objectValue["1"] = 502;
    objectValue["100"] = object.type;
    if (object.definition != null) {
      objectValue["101"] = object.definition.toValue();
    }
    objectValue["102"] = object.recursive;
    if (object.depth != null) {
      objectValue["103"] = object.depth;
    }
    if (object.on != null) {
      objectValue["104"] = object.on.toValue();
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
    const _NodeDefinitionReference = STRUCT_CLASS_BY_TYPE[
      StructType.NODE_DEFINITION_REFERENCE
    ] as typeof NodeDefinitionReference;
    const _Condition = STRUCT_CLASS_BY_TYPE[StructType.CONDITION] as typeof Condition;
    const definitionValue = objectValue["101"];
    const unpackedDefinition =
      definitionValue != undefined
        ? _NodeDefinitionReference.fromValue(
            definitionValue,
            _session,
            _supergraph,
            _graph,
            _connection,
          )
        : null;
    const depthValue = objectValue["103"];
    const unpackedDepth = depthValue != undefined ? Number(depthValue) : null;
    const onValue = objectValue["104"];
    const unpackedOn =
      onValue != undefined
        ? _Condition.fromValue(onValue, _session, _supergraph, _graph, _connection)
        : null;
    return new Join({
      type: Number(objectValue["100"]),
      definition: unpackedDefinition,
      recursive: objectValue["102"],
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

  toProto(): JoinProto {
    if (this._proto === null) {
      // @ts-expect-error(readonly)
      this._proto = Join.__packProto__(this);
    }
    return this._proto as JoinProto;
  }

  static __packProto__(object: Join): JoinProto {
    const objectProto: Partial<JoinProto> = { metatype: 502 };
    objectProto.type = Number(object.type) as JoinTypeProto;
    if (object.definition != null) {
      objectProto.definition = object.definition.toProto();
    }
    objectProto.recursive = object.recursive;
    if (object.depth != null) {
      objectProto.depth = object.depth;
    }
    if (object.on != null) {
      objectProto.on = object.on.toProto();
    }
    return objectProto as JoinProto;
  }

  static __unpackProto__(
    objectProto: JoinProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Join {
    const _NodeDefinitionReference = STRUCT_CLASS_BY_TYPE[
      StructType.NODE_DEFINITION_REFERENCE
    ] as typeof NodeDefinitionReference;
    const _Condition = STRUCT_CLASS_BY_TYPE[StructType.CONDITION] as typeof Condition;
    return new Join({
      type: Number(objectProto.type) as JoinType,
      definition:
        objectProto.definition != undefined
          ? _NodeDefinitionReference.fromProto(
              objectProto.definition!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      recursive: objectProto.recursive,
      depth: objectProto.depth != undefined ? Number(objectProto.depth) : null,
      on:
        objectProto.on != undefined
          ? _Condition.fromProto(objectProto.on!, _session, _supergraph, _graph, _connection)
          : null,
      _proto: objectProto,
      _supergraph,
    });
  }

  static fromProto(
    objectProto: JoinProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Join {
    return Join.__unpackProto__(objectProto, _session, _supergraph, _graph, _connection);
  }

  static fromProtoString(packedProtoString: string): Join {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = JoinProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */

  /** Make a Join from a shorthand expression. */
  static of(
    joinType: JoinType | Join,
    options?: {
      definition?: NodeType | NodeClass | CustomEntityDefinition;
      recursive?: boolean;
      depth?: number | null;
      on?: Condition | null;
    },
  ): Join {
    if (isStruct(joinType, StructType.JOIN)) {
      return joinType;
    }
    const _NodeDefinitionReference = STRUCT_CLASS_BY_TYPE[
      StructType.NODE_DEFINITION_REFERENCE
    ] as typeof NodeDefinitionReference;
    return new Join({
      type: joinType,
      definition: options?.definition ? _NodeDefinitionReference.of(options.definition) : null,
      recursive: options?.recursive ?? false,
      depth: options?.depth ?? null,
      on: options?.on ?? null,
    });
  }

  /* ==== DESTACK_CUSTOM_END ==== */
}
registerStructClass(StructType.JOIN, Join);
/* ==== DESTACK_GENERATED_END:STRUCT:502 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:553 ==== */
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
    if (!(this.metatype === other.metatype)) {
      return false;
    }
    if (!(this.type === other.type)) {
      return false;
    }
    if (
      (this.result == null) !== (other.result == null) ||
      (this.result != null && !this.result.equals(other.result))
    ) {
      return false;
    }
    return true;
  }

  repr(): string {
    if (this._repr === null) {
      const propertyReprs: string[] = [];
      propertyReprs.push(`type=${QueryUpdateType[this.type]}`);
      if (this.result !== null) {
        propertyReprs.push(`result=${this.result.repr()}`);
      }
      // @ts-expect-error(readonly)
      this._repr = `<QueryUpdate ${propertyReprs.join(" ")}>`;
    }
    return this._repr;
  }

  hash(): number {
    if (this._hash !== null) {
      return this._hash;
    }

    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    h = (h * 31 + this.type) & 0xffffffff;
    if (this.result !== null) {
      h = (h * 31 + this.result.hash()) & 0xffffffff;
    }

    // @ts-expect-error(readonly)
    this._hash = h;
    return h;
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
    objectValue["1"] = 553;
    objectValue["100"] = object.type;
    if (object.result != null) {
      objectValue["101"] = object.result.toValue();
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
    const _QueryResult = STRUCT_CLASS_BY_TYPE[StructType.QUERY_RESULT] as typeof QueryResult;
    const resultValue = objectValue["101"];
    const unpackedResult =
      resultValue != undefined
        ? _QueryResult.fromValue(resultValue, _session, _supergraph, _graph, _connection)
        : null;
    return new QueryUpdate({
      type: Number(objectValue["100"]),
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

  toProto(): QueryUpdateProto {
    if (this._proto === null) {
      // @ts-expect-error(readonly)
      this._proto = QueryUpdate.__packProto__(this);
    }
    return this._proto as QueryUpdateProto;
  }

  static __packProto__(object: QueryUpdate): QueryUpdateProto {
    const objectProto: Partial<QueryUpdateProto> = { metatype: 553 };
    objectProto.type = Number(object.type) as QueryUpdateTypeProto;
    if (object.result != null) {
      objectProto.result = object.result.toProto();
    }
    return objectProto as QueryUpdateProto;
  }

  static __unpackProto__(
    objectProto: QueryUpdateProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): QueryUpdate {
    const _QueryResult = STRUCT_CLASS_BY_TYPE[StructType.QUERY_RESULT] as typeof QueryResult;
    return new QueryUpdate({
      type: Number(objectProto.type) as QueryUpdateType,
      result:
        objectProto.result != undefined
          ? _QueryResult.fromProto(objectProto.result!, _session, _supergraph, _graph, _connection)
          : null,
      _proto: objectProto,
      _supergraph,
    });
  }

  static fromProto(
    objectProto: QueryUpdateProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): QueryUpdate {
    return QueryUpdate.__unpackProto__(objectProto, _session, _supergraph, _graph, _connection);
  }

  static fromProtoString(packedProtoString: string): QueryUpdate {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = QueryUpdateProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerStructClass(StructType.QUERY_UPDATE, QueryUpdate);
/* ==== DESTACK_GENERATED_END:STRUCT:553 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:550 ==== */
/**
 * A GraphQL-inspired Query node (with subqueries).
 */
export class Query<T extends Node = Node> extends StructFrozen {
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
   * Query.definition
   */
  readonly definition: NodeDefinitionReference;

  /**
   * Query.subqueries
   */
  readonly subqueries: Array<Query>;

  /**
   * Relative to parent Query.
   */
  readonly join: Join | null;

  /**
   * Query.select
   */
  readonly select: Select | null;

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

  /**
   * The Snapshot this Query is for.
   */
  get snapshot(): Snapshot | null {
    const nodePtr: NodeReference | null = this.snapshotPtr;
    if (nodePtr !== null) {
      if (this._supergraph === null) {
        return null;
      }
      return this._supergraph.get(nodePtr.id) as Snapshot | null;
    }
    return null;
  }
  readonly snapshotPtr: NodeReference | null;

  /**
   * The path of Snapshots from the given Snapshot to to a full Snapshot (inclusive).
   * If Query.snapshot is set, this must contain at least one element.
   */
  readonly snapshotPath: Array<string>;

  constructor(options: {
    id?: string;
    type: QueryType;
    name: string;
    definition: NodeDefinitionReference;
    subqueries?: Array<Query>;
    join?: Join | null;
    select?: Select | null;
    where?: Condition | null;
    having?: Condition | null;
    groupBy?: Array<Expression>;
    aggregation?: Aggregation | null;
    sort?: Array<Sort>;
    limit?: number | null;
    offset?: number | null;
    snapshot?: Snapshot | NodeReference | null;
    snapshotPath?: Array<string>;
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
    let _definition = options.definition;
    if (_definition === null) {
      throw new Error(`Query.definition is required`);
    }
    this.definition = _definition;
    let _subqueries = options.subqueries ?? null;
    if (_subqueries === null) {
      _subqueries = [];
    }
    this.subqueries = _subqueries;
    let _join = options.join ?? null;
    this.join = _join;
    let _select = options.select ?? null;
    this.select = _select;
    let _where = options.where ?? null;
    this.where = _where;
    let _having = options.having ?? null;
    this.having = _having;
    let _groupBy = options.groupBy ?? null;
    if (_groupBy === null) {
      _groupBy = [];
    }
    this.groupBy = _groupBy;
    let _aggregation = options.aggregation ?? null;
    this.aggregation = _aggregation;
    let _sort = options.sort ?? null;
    if (_sort === null) {
      _sort = [];
    }
    this.sort = _sort;
    let _limit = options.limit ?? null;
    this.limit = _limit;
    let _offset = options.offset ?? null;
    this.offset = _offset;
    let _snapshot = options.snapshot ?? null;
    if (_snapshot != null && _snapshot.metatype != StructType.NODE_REFERENCE) {
      _snapshot = (_snapshot as Node).toRef();
    }
    this.snapshotPtr = _snapshot;
    let _snapshotPath = options.snapshotPath ?? null;
    if (_snapshotPath === null) {
      _snapshotPath = [];
    }
    this.snapshotPath = _snapshotPath;

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
    if (!(this.metatype === other.metatype)) {
      return false;
    }
    if (!(this.id === other.id)) {
      return false;
    }
    if (!(this.type === other.type)) {
      return false;
    }
    if (!(this.name === other.name)) {
      return false;
    }
    if (!this.definition.equals(other.definition)) {
      return false;
    }
    if (this.subqueries.length !== other.subqueries.length) {
      return false;
    }
    for (let i = 0; i < this.subqueries.length; i++) {
      if (!this.subqueries[i].equals(other.subqueries[i])) {
        return false;
      }
    }
    if (
      (this.join == null) !== (other.join == null) ||
      (this.join != null && !this.join.equals(other.join))
    ) {
      return false;
    }
    if (
      (this.select == null) !== (other.select == null) ||
      (this.select != null && !this.select.equals(other.select))
    ) {
      return false;
    }
    if (
      (this.where == null) !== (other.where == null) ||
      (this.where != null && !this.where.equals(other.where))
    ) {
      return false;
    }
    if (
      (this.having == null) !== (other.having == null) ||
      (this.having != null && !this.having.equals(other.having))
    ) {
      return false;
    }
    if (this.groupBy.length !== other.groupBy.length) {
      return false;
    }
    for (let i = 0; i < this.groupBy.length; i++) {
      if (!this.groupBy[i].equals(other.groupBy[i])) {
        return false;
      }
    }
    if (
      (this.aggregation == null) !== (other.aggregation == null) ||
      (this.aggregation != null && !this.aggregation.equals(other.aggregation))
    ) {
      return false;
    }
    if (this.sort.length !== other.sort.length) {
      return false;
    }
    for (let i = 0; i < this.sort.length; i++) {
      if (!this.sort[i].equals(other.sort[i])) {
        return false;
      }
    }
    if (!(this.limit === other.limit)) {
      return false;
    }
    if (!(this.offset === other.offset)) {
      return false;
    }
    if (!(this.snapshotPtr?.id === other.snapshotPtr?.id)) {
      return false;
    }
    if (this.snapshotPath.length !== other.snapshotPath.length) {
      return false;
    }
    for (let i = 0; i < this.snapshotPath.length; i++) {
      if (!(this.snapshotPath[i] === other.snapshotPath[i])) {
        return false;
      }
    }
    return true;
  }

  repr(): string {
    if (this._repr === null) {
      const propertyReprs: string[] = [];
      propertyReprs.push(`type=${QueryType[this.type]}`);
      propertyReprs.push(`name=${this.name}`);
      propertyReprs.push(`definition=${this.definition.repr()}`);
      if (this.subqueries.length > 0) {
        propertyReprs.push(`subqueries=${this.subqueries.map((_item) => _item.repr()).join(", ")}`);
      }
      if (this.join !== null) {
        propertyReprs.push(`join=${this.join.repr()}`);
      }
      if (this.select !== null) {
        propertyReprs.push(`select=${this.select.repr()}`);
      }
      if (this.where !== null) {
        propertyReprs.push(`where=${this.where.repr()}`);
      }
      if (this.having !== null) {
        propertyReprs.push(`having=${this.having.repr()}`);
      }
      if (this.groupBy.length > 0) {
        propertyReprs.push(`groupBy=${this.groupBy.map((_item) => _item.repr()).join(", ")}`);
      }
      if (this.aggregation !== null) {
        propertyReprs.push(`aggregation=${this.aggregation.repr()}`);
      }
      if (this.sort.length > 0) {
        propertyReprs.push(`sort=${this.sort.map((_item) => _item.repr()).join(", ")}`);
      }
      if (this.limit !== null) {
        propertyReprs.push(`limit=${this.limit}`);
      }
      if (this.offset !== null) {
        propertyReprs.push(`offset=${this.offset}`);
      }
      if (this.snapshot !== null) {
        propertyReprs.push(`snapshot=${this.snapshot?.repr()}`);
      }
      if (this.snapshotPath.length > 0) {
        propertyReprs.push(`snapshotPath=${this.snapshotPath.map((_item) => _item).join(", ")}`);
      }
      // @ts-expect-error(readonly)
      this._repr = `<Query ${propertyReprs.join(" ")}>`;
    }
    return this._repr;
  }

  hash(): number {
    if (this._hash !== null) {
      return this._hash;
    }

    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    h = (h * 31 + hashString(this.id.toString())) & 0xffffffff;
    h = (h * 31 + this.type) & 0xffffffff;
    h = (h * 31 + hashString(this.name)) & 0xffffffff;
    h = (h * 31 + this.definition.hash()) & 0xffffffff;
    if (this.subqueries && this.subqueries.length > 0) {
      for (const _item of this.subqueries) {
        h = (h * 31 + _item.hash()) & 0xffffffff;
      }
    }
    if (this.join !== null) {
      h = (h * 31 + this.join.hash()) & 0xffffffff;
    }
    if (this.select !== null) {
      h = (h * 31 + this.select.hash()) & 0xffffffff;
    }
    if (this.where !== null) {
      h = (h * 31 + this.where.hash()) & 0xffffffff;
    }
    if (this.having !== null) {
      h = (h * 31 + this.having.hash()) & 0xffffffff;
    }
    if (this.groupBy && this.groupBy.length > 0) {
      for (const _item of this.groupBy) {
        h = (h * 31 + _item.hash()) & 0xffffffff;
      }
    }
    if (this.aggregation !== null) {
      h = (h * 31 + this.aggregation.hash()) & 0xffffffff;
    }
    if (this.sort && this.sort.length > 0) {
      for (const _item of this.sort) {
        h = (h * 31 + _item.hash()) & 0xffffffff;
      }
    }
    if (this.limit !== null) {
      h = (h * 31 + hashInt(this.limit)) & 0xffffffff;
    }
    if (this.offset !== null) {
      h = (h * 31 + hashInt(this.offset)) & 0xffffffff;
    }
    if (this.snapshotPtr !== null) {
      h = (h * 31 + hashString(this.snapshotPtr.id)) & 0xffffffff;
    }
    if (this.snapshotPath && this.snapshotPath.length > 0) {
      for (const _item of this.snapshotPath) {
        h = (h * 31 + hashString(_item.toString())) & 0xffffffff;
      }
    }

    // @ts-expect-error(readonly)
    this._hash = h;
    return h;
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
    objectValue["1"] = 550;
    objectValue["2"] = String(object.id);
    objectValue["100"] = object.type;
    objectValue["101"] = object.name;
    objectValue["102"] = object.definition.toValue();
    if (object.subqueries.length > 0) {
      const packedSubqueries: any[] = [];
      for (const item of object.subqueries) {
        packedSubqueries.push(item.toValue());
      }
      objectValue["109"] = packedSubqueries;
    }
    if (object.join != null) {
      objectValue["110"] = object.join.toValue();
    }
    if (object.select != null) {
      objectValue["111"] = object.select.toValue();
    }
    if (object.where != null) {
      objectValue["112"] = object.where.toValue();
    }
    if (object.having != null) {
      objectValue["113"] = object.having.toValue();
    }
    if (object.groupBy.length > 0) {
      const packedGroupBy: any[] = [];
      for (const item of object.groupBy) {
        packedGroupBy.push(item.toValue());
      }
      objectValue["114"] = packedGroupBy;
    }
    if (object.aggregation != null) {
      objectValue["115"] = object.aggregation.toValue();
    }
    if (object.sort.length > 0) {
      const packedSort: any[] = [];
      for (const item of object.sort) {
        packedSort.push(item.toValue());
      }
      objectValue["116"] = packedSort;
    }
    if (object.limit != null) {
      objectValue["120"] = object.limit;
    }
    if (object.offset != null) {
      objectValue["121"] = object.offset;
    }
    if (object.snapshotPtr != null) {
      objectValue["130"] = object.snapshotPtr.toValue();
    }
    if (object.snapshotPath.length > 0) {
      const packedSnapshotPath: any[] = [];
      for (const item of object.snapshotPath) {
        packedSnapshotPath.push(String(item));
      }
      objectValue["131"] = packedSnapshotPath;
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
    const _NodeDefinitionReference = STRUCT_CLASS_BY_TYPE[
      StructType.NODE_DEFINITION_REFERENCE
    ] as typeof NodeDefinitionReference;
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    const _Expression = STRUCT_CLASS_BY_TYPE[StructType.EXPRESSION] as typeof Expression;
    const _Join = STRUCT_CLASS_BY_TYPE[StructType.JOIN] as typeof Join;
    const _Aggregation = STRUCT_CLASS_BY_TYPE[StructType.AGGREGATION] as typeof Aggregation;
    const _Condition = STRUCT_CLASS_BY_TYPE[StructType.CONDITION] as typeof Condition;
    const _Sort = STRUCT_CLASS_BY_TYPE[StructType.SORT] as typeof Sort;
    const _Select = STRUCT_CLASS_BY_TYPE[StructType.SELECT] as typeof Select;
    const _Query = STRUCT_CLASS_BY_TYPE[StructType.QUERY] as typeof Query;
    const unpackedSubqueries: any[] = [];
    if (objectValue["109"] != undefined) {
      for (const item of objectValue["109"]) {
        unpackedSubqueries.push(_Query.fromValue(item, _session, _supergraph, _graph, _connection));
      }
    }
    const joinValue = objectValue["110"];
    const unpackedJoin =
      joinValue != undefined
        ? _Join.fromValue(joinValue, _session, _supergraph, _graph, _connection)
        : null;
    const selectValue = objectValue["111"];
    const unpackedSelect =
      selectValue != undefined
        ? _Select.fromValue(selectValue, _session, _supergraph, _graph, _connection)
        : null;
    const whereValue = objectValue["112"];
    const unpackedWhere =
      whereValue != undefined
        ? _Condition.fromValue(whereValue, _session, _supergraph, _graph, _connection)
        : null;
    const havingValue = objectValue["113"];
    const unpackedHaving =
      havingValue != undefined
        ? _Condition.fromValue(havingValue, _session, _supergraph, _graph, _connection)
        : null;
    const unpackedGroupBy: any[] = [];
    if (objectValue["114"] != undefined) {
      for (const item of objectValue["114"]) {
        unpackedGroupBy.push(
          _Expression.fromValue(item, _session, _supergraph, _graph, _connection),
        );
      }
    }
    const aggregationValue = objectValue["115"];
    const unpackedAggregation =
      aggregationValue != undefined
        ? _Aggregation.fromValue(aggregationValue, _session, _supergraph, _graph, _connection)
        : null;
    const unpackedSort: any[] = [];
    if (objectValue["116"] != undefined) {
      for (const item of objectValue["116"]) {
        unpackedSort.push(_Sort.fromValue(item, _session, _supergraph, _graph, _connection));
      }
    }
    const limitValue = objectValue["120"];
    const unpackedLimit = limitValue != undefined ? Number(limitValue) : null;
    const offsetValue = objectValue["121"];
    const unpackedOffset = offsetValue != undefined ? Number(offsetValue) : null;
    const snapshotPtrValue = objectValue["130"];
    const unpackedSnapshotPtr =
      snapshotPtrValue != undefined
        ? _NodeReference.fromValue(snapshotPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const unpackedSnapshotPath: any[] = [];
    if (objectValue["131"] != undefined) {
      for (const item of objectValue["131"]) {
        unpackedSnapshotPath.push(String(item));
      }
    }
    return new Query({
      id: String(objectValue["2"]),
      type: Number(objectValue["100"]),
      name: objectValue["101"],
      definition: _NodeDefinitionReference.fromValue(
        objectValue["102"],
        _session,
        _supergraph,
        _graph,
        _connection,
      ),
      subqueries: unpackedSubqueries,
      join: unpackedJoin,
      select: unpackedSelect,
      where: unpackedWhere,
      having: unpackedHaving,
      groupBy: unpackedGroupBy,
      aggregation: unpackedAggregation,
      sort: unpackedSort,
      limit: unpackedLimit,
      offset: unpackedOffset,
      snapshot: unpackedSnapshotPtr,
      snapshotPath: unpackedSnapshotPath,
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

  toProto(): QueryProto {
    if (this._proto === null) {
      // @ts-expect-error(readonly)
      this._proto = Query.__packProto__(this);
    }
    return this._proto as QueryProto;
  }

  static __packProto__(object: Query): QueryProto {
    const objectProto: Partial<QueryProto> = { metatype: 550 };
    objectProto.id = String(object.id);
    objectProto.type = Number(object.type) as QueryTypeProto;
    objectProto.name = object.name;
    objectProto.definition = object.definition.toProto();
    if (object.subqueries) {
      const packedSubqueries: any[] = [];
      for (const item of object.subqueries) {
        packedSubqueries.push(item.toProto());
      }
      objectProto.subqueries = packedSubqueries;
    }
    if (object.join != null) {
      objectProto.join = object.join.toProto();
    }
    if (object.select != null) {
      objectProto.select = object.select.toProto();
    }
    if (object.where != null) {
      objectProto.where = object.where.toProto();
    }
    if (object.having != null) {
      objectProto.having = object.having.toProto();
    }
    if (object.groupBy) {
      const packedGroupBy: any[] = [];
      for (const item of object.groupBy) {
        packedGroupBy.push(item.toProto());
      }
      objectProto.groupBy = packedGroupBy;
    }
    if (object.aggregation != null) {
      objectProto.aggregation = object.aggregation.toProto();
    }
    if (object.sort) {
      const packedSort: any[] = [];
      for (const item of object.sort) {
        packedSort.push(item.toProto());
      }
      objectProto.sort = packedSort;
    }
    if (object.limit != null) {
      objectProto.limit = object.limit;
    }
    if (object.offset != null) {
      objectProto.offset = object.offset;
    }
    if (object.snapshotPtr != null) {
      objectProto.snapshotPtr = object.snapshotPtr.toProto();
    }
    if (object.snapshotPath) {
      const packedSnapshotPath: any[] = [];
      for (const item of object.snapshotPath) {
        packedSnapshotPath.push(String(item));
      }
      objectProto.snapshotPath = packedSnapshotPath;
    }
    return objectProto as QueryProto;
  }

  static __unpackProto__(
    objectProto: QueryProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Query {
    const _NodeDefinitionReference = STRUCT_CLASS_BY_TYPE[
      StructType.NODE_DEFINITION_REFERENCE
    ] as typeof NodeDefinitionReference;
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    const _Expression = STRUCT_CLASS_BY_TYPE[StructType.EXPRESSION] as typeof Expression;
    const _Join = STRUCT_CLASS_BY_TYPE[StructType.JOIN] as typeof Join;
    const _Aggregation = STRUCT_CLASS_BY_TYPE[StructType.AGGREGATION] as typeof Aggregation;
    const _Condition = STRUCT_CLASS_BY_TYPE[StructType.CONDITION] as typeof Condition;
    const _Sort = STRUCT_CLASS_BY_TYPE[StructType.SORT] as typeof Sort;
    const _Select = STRUCT_CLASS_BY_TYPE[StructType.SELECT] as typeof Select;
    const _Query = STRUCT_CLASS_BY_TYPE[StructType.QUERY] as typeof Query;
    const unpackedSubqueries: any[] = [];
    if (objectProto.subqueries) {
      for (const item of objectProto.subqueries) {
        unpackedSubqueries.push(
          _Query.fromProto(item!, _session, _supergraph, _graph, _connection),
        );
      }
    }
    const unpackedGroupBy: any[] = [];
    if (objectProto.groupBy) {
      for (const item of objectProto.groupBy) {
        unpackedGroupBy.push(
          _Expression.fromProto(item!, _session, _supergraph, _graph, _connection),
        );
      }
    }
    const unpackedSort: any[] = [];
    if (objectProto.sort) {
      for (const item of objectProto.sort) {
        unpackedSort.push(_Sort.fromProto(item!, _session, _supergraph, _graph, _connection));
      }
    }
    const unpackedSnapshotPath: any[] = [];
    if (objectProto.snapshotPath) {
      for (const item of objectProto.snapshotPath) {
        unpackedSnapshotPath.push(String(item));
      }
    }
    return new Query({
      id: String(objectProto.id),
      type: Number(objectProto.type) as QueryType,
      name: objectProto.name,
      definition: _NodeDefinitionReference.fromProto(
        objectProto.definition!,
        _session,
        _supergraph,
        _graph,
        _connection,
      ),
      subqueries: unpackedSubqueries,
      join:
        objectProto.join != undefined
          ? _Join.fromProto(objectProto.join!, _session, _supergraph, _graph, _connection)
          : null,
      select:
        objectProto.select != undefined
          ? _Select.fromProto(objectProto.select!, _session, _supergraph, _graph, _connection)
          : null,
      where:
        objectProto.where != undefined
          ? _Condition.fromProto(objectProto.where!, _session, _supergraph, _graph, _connection)
          : null,
      having:
        objectProto.having != undefined
          ? _Condition.fromProto(objectProto.having!, _session, _supergraph, _graph, _connection)
          : null,
      groupBy: unpackedGroupBy,
      aggregation:
        objectProto.aggregation != undefined
          ? _Aggregation.fromProto(
              objectProto.aggregation!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      sort: unpackedSort,
      limit: objectProto.limit != undefined ? Number(objectProto.limit) : null,
      offset: objectProto.offset != undefined ? Number(objectProto.offset) : null,
      snapshot:
        objectProto.snapshotPtr != undefined
          ? _NodeReference.fromProto(
              objectProto.snapshotPtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      snapshotPath: unpackedSnapshotPath,
      _proto: objectProto,
      _supergraph,
    });
  }

  static fromProto(
    objectProto: QueryProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Query {
    return Query.__unpackProto__(objectProto, _session, _supergraph, _graph, _connection);
  }

  static fromProtoString(packedProtoString: string): Query {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = QueryProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */

  /** Execute the Query. */
  async execute(): Promise<QueryConnection<T>> {
    const session = activeSession();
    const store = session.store;
    if (store == null) {
      throw new Error(`no store in ${session.repr()}`);
    }
    const connection = new QueryConnection<T>({ query: this, store, session });
    session.connections.push(connection);
    await connection.execute();
    return connection;
  }

  /** Execute the Query and return the root (if any). */
  async executeOneOrNone(): Promise<T | null> {
    if (!(this.type === QueryType.NODE || this.type === QueryType.GROUPED_NODE)) {
      throw new Error(`cannot get node of ${this.repr()}`);
    }
    const connection = await this.execute();
    return connection.toOneOrNone();
  }

  /** Execute the Query and return the root (error if none). */
  async executeOne(): Promise<T> {
    if (!(this.type === QueryType.NODE || this.type === QueryType.GROUPED_NODE)) {
      throw new Error(`cannot get node of ${this.repr()}`);
    }
    const connection = await this.execute();
    return connection.toOne();
  }

  /** Execute the Query and return the list of roots. */
  async executeList(): Promise<Array<T>> {
    if (!(this.type === QueryType.NODE || this.type === QueryType.GROUPED_NODE)) {
      throw new Error(`cannot get nodes of ${this.repr()}`);
    }
    const connection = await this.execute();
    return connection.toList();
  }

  /** Execute the Query and return whether any results exist. */
  async executeExists(): Promise<boolean> {
    if (this.type !== QueryType.SCALAR) {
      throw new Error(`cannot get exists of ${this.repr()}`);
    }
    const connection = await this.execute();
    return connection.toExists();
  }

  /** Execute the Query and return the count. */
  async executeCount(): Promise<number> {
    if (!(this.type === QueryType.SCALAR || this.type === QueryType.GROUPED_SCALAR)) {
      throw new Error(`cannot get count of ${this.repr()}`);
    }
    const connection = await this.execute();
    return connection.toCount();
  }

  /** Execute the Query and return the scalar value. */
  async executeScalar(): Promise<any> {
    if (!(this.type === QueryType.SCALAR || this.type === QueryType.GROUPED_SCALAR)) {
      throw new Error(`cannot get scalar of ${this.repr()}`);
    }
    const connection = await this.execute();
    return connection.toScalar();
  }

  /* ==== DESTACK_CUSTOM_END ==== */
}
registerStructClass(StructType.QUERY, Query);
/* ==== DESTACK_GENERATED_END:STRUCT:550 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:551 ==== */
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
   * QueryResult.type
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
   * QueryResult.nodes
   */
  nodes: Array<Value>;

  /**
   * QueryResult.count
   */
  count: number | null;

  /**
   * QueryResult.exists
   */
  exists: boolean | null;

  /**
   * QueryResult.scalar
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
      _groups = [];
    }
    this.groups = _groups;
    let _subresults = options.subresults ?? null;
    if (_subresults === null) {
      _subresults = [];
    }
    this.subresults = _subresults;
    let _nodes = options.nodes ?? null;
    if (_nodes === null) {
      _nodes = [];
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
    if (!(this.metatype === other.metatype)) {
      return false;
    }
    if (!(this.id === other.id)) {
      return false;
    }
    if (!(this.type === other.type)) {
      return false;
    }
    if (this.groups.length !== other.groups.length) {
      return false;
    }
    for (let i = 0; i < this.groups.length; i++) {
      if (!this.groups[i].equals(other.groups[i])) {
        return false;
      }
    }
    if (this.subresults.length !== other.subresults.length) {
      return false;
    }
    for (let i = 0; i < this.subresults.length; i++) {
      if (!this.subresults[i].equals(other.subresults[i])) {
        return false;
      }
    }
    if (this.nodes.length !== other.nodes.length) {
      return false;
    }
    for (let i = 0; i < this.nodes.length; i++) {
      if (!this.nodes[i].equals(other.nodes[i])) {
        return false;
      }
    }
    if (!(this.count === other.count)) {
      return false;
    }
    if (!(this.exists === other.exists)) {
      return false;
    }
    if (
      (this.scalar == null) !== (other.scalar == null) ||
      (this.scalar != null && !this.scalar.equals(other.scalar))
    ) {
      return false;
    }
    return true;
  }

  repr(): string {
    const propertyReprs: string[] = [];
    propertyReprs.push(`id=${this.id}`);
    propertyReprs.push(`type=${QueryType[this.type]}`);
    if (this.groups.length > 0) {
      propertyReprs.push(`groups=${this.groups.map((_item) => _item.repr()).join(", ")}`);
    }
    if (this.subresults.length > 0) {
      propertyReprs.push(`subresults=${this.subresults.map((_item) => _item.repr()).join(", ")}`);
    }
    if (this.count !== null) {
      propertyReprs.push(`count=${this.count}`);
    }
    if (this.exists !== null) {
      propertyReprs.push(`exists=${this.exists}`);
    }
    if (this.scalar !== null) {
      propertyReprs.push(`scalar=${this.scalar.repr()}`);
    }
    return `<QueryResult ${propertyReprs.join(" ")}>`;
  }

  hash(): number {
    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    h = (h * 31 + hashString(this.id.toString())) & 0xffffffff;
    h = (h * 31 + this.type) & 0xffffffff;
    if (this.groups && this.groups.length > 0) {
      for (const _item of this.groups) {
        h = (h * 31 + _item.hash()) & 0xffffffff;
      }
    }
    if (this.subresults && this.subresults.length > 0) {
      for (const _item of this.subresults) {
        h = (h * 31 + _item.hash()) & 0xffffffff;
      }
    }
    if (this.nodes && this.nodes.length > 0) {
      for (const _item of this.nodes) {
        h = (h * 31 + _item.hash()) & 0xffffffff;
      }
    }
    if (this.count !== null) {
      h = (h * 31 + hashInt(this.count)) & 0xffffffff;
    }
    if (this.exists !== null) {
      h = (h * 31 + hashBool(this.exists)) & 0xffffffff;
    }
    if (this.scalar !== null) {
      h = (h * 31 + this.scalar.hash()) & 0xffffffff;
    }

    return h;
  }

  validate(): void {
    throw new Error("not implemented");
  }

  toValue(): { [key: string]: any } {
    return QueryResult.__packValue__(this);
  }

  static __packValue__(object: QueryResult): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 551;
    objectValue["2"] = String(object.id);
    objectValue["100"] = object.type;
    if (object.groups.length > 0) {
      const packedGroups: any[] = [];
      for (const item of object.groups) {
        packedGroups.push(item.toValue());
      }
      objectValue["101"] = packedGroups;
    }
    if (object.subresults.length > 0) {
      const packedSubresults: any[] = [];
      for (const item of object.subresults) {
        packedSubresults.push(item.toValue());
      }
      objectValue["102"] = packedSubresults;
    }
    if (object.nodes.length > 0) {
      const packedNodes: any[] = [];
      for (const item of object.nodes) {
        packedNodes.push(item.toValue());
      }
      objectValue["110"] = packedNodes;
    }
    if (object.count != null) {
      objectValue["111"] = object.count;
    }
    if (object.exists != null) {
      objectValue["112"] = object.exists;
    }
    if (object.scalar != null) {
      objectValue["113"] = object.scalar.toValue();
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
    const _QueryResult = STRUCT_CLASS_BY_TYPE[StructType.QUERY_RESULT] as typeof QueryResult;
    const _QueryResultGroup = STRUCT_CLASS_BY_TYPE[
      StructType.QUERY_RESULT_GROUP
    ] as typeof QueryResultGroup;
    const _Value = STRUCT_CLASS_BY_TYPE[StructType.VALUE] as typeof Value;
    const unpackedGroups: any[] = [];
    if (objectValue["101"] != undefined) {
      for (const item of objectValue["101"]) {
        unpackedGroups.push(
          _QueryResultGroup.fromValue(item, _session, _supergraph, _graph, _connection),
        );
      }
    }
    const unpackedSubresults: any[] = [];
    if (objectValue["102"] != undefined) {
      for (const item of objectValue["102"]) {
        unpackedSubresults.push(
          _QueryResult.fromValue(item, _session, _supergraph, _graph, _connection),
        );
      }
    }
    const unpackedNodes: any[] = [];
    if (objectValue["110"] != undefined) {
      for (const item of objectValue["110"]) {
        unpackedNodes.push(_Value.fromValue(item, _session, _supergraph, _graph, _connection));
      }
    }
    const countValue = objectValue["111"];
    const unpackedCount = countValue != undefined ? Number(countValue) : null;
    const existsValue = objectValue["112"];
    const unpackedExists = existsValue != undefined ? existsValue : null;
    const scalarValue = objectValue["113"];
    const unpackedScalar =
      scalarValue != undefined
        ? _Value.fromValue(scalarValue, _session, _supergraph, _graph, _connection)
        : null;
    return new QueryResult({
      id: String(objectValue["2"]),
      type: Number(objectValue["100"]),
      groups: unpackedGroups,
      subresults: unpackedSubresults,
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

  toProto(): QueryResultProto {
    return QueryResult.__packProto__(this);
  }

  static __packProto__(object: QueryResult): QueryResultProto {
    const objectProto: Partial<QueryResultProto> = { metatype: 551 };
    objectProto.id = String(object.id);
    objectProto.type = Number(object.type) as QueryTypeProto;
    if (object.groups) {
      const packedGroups: any[] = [];
      for (const item of object.groups) {
        packedGroups.push(item.toProto());
      }
      objectProto.groups = packedGroups;
    }
    if (object.subresults) {
      const packedSubresults: any[] = [];
      for (const item of object.subresults) {
        packedSubresults.push(item.toProto());
      }
      objectProto.subresults = packedSubresults;
    }
    if (object.nodes) {
      const packedNodes: any[] = [];
      for (const item of object.nodes) {
        packedNodes.push(item.toProto());
      }
      objectProto.nodes = packedNodes;
    }
    if (object.count != null) {
      objectProto.count = object.count;
    }
    if (object.exists != null) {
      objectProto.exists = object.exists;
    }
    if (object.scalar != null) {
      objectProto.scalar = object.scalar.toProto();
    }
    return objectProto as QueryResultProto;
  }

  static __unpackProto__(
    objectProto: QueryResultProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): QueryResult {
    const _QueryResult = STRUCT_CLASS_BY_TYPE[StructType.QUERY_RESULT] as typeof QueryResult;
    const _QueryResultGroup = STRUCT_CLASS_BY_TYPE[
      StructType.QUERY_RESULT_GROUP
    ] as typeof QueryResultGroup;
    const _Value = STRUCT_CLASS_BY_TYPE[StructType.VALUE] as typeof Value;
    const unpackedGroups: any[] = [];
    if (objectProto.groups) {
      for (const item of objectProto.groups) {
        unpackedGroups.push(
          _QueryResultGroup.fromProto(item!, _session, _supergraph, _graph, _connection),
        );
      }
    }
    const unpackedSubresults: any[] = [];
    if (objectProto.subresults) {
      for (const item of objectProto.subresults) {
        unpackedSubresults.push(
          _QueryResult.fromProto(item!, _session, _supergraph, _graph, _connection),
        );
      }
    }
    const unpackedNodes: any[] = [];
    if (objectProto.nodes) {
      for (const item of objectProto.nodes) {
        unpackedNodes.push(_Value.fromProto(item!, _session, _supergraph, _graph, _connection));
      }
    }
    return new QueryResult({
      id: String(objectProto.id),
      type: Number(objectProto.type) as QueryType,
      groups: unpackedGroups,
      subresults: unpackedSubresults,
      nodes: unpackedNodes,
      count: objectProto.count != undefined ? Number(objectProto.count) : null,
      exists: objectProto.exists != undefined ? objectProto.exists : null,
      scalar:
        objectProto.scalar != undefined
          ? _Value.fromProto(objectProto.scalar!, _session, _supergraph, _graph, _connection)
          : null,
      _supergraph,
    });
  }

  static fromProto(
    objectProto: QueryResultProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): QueryResult {
    return QueryResult.__unpackProto__(objectProto, _session, _supergraph, _graph, _connection);
  }

  static fromProtoString(packedProtoString: string): QueryResult {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = QueryResultProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerStructClass(StructType.QUERY_RESULT, QueryResult);
/* ==== DESTACK_GENERATED_END:STRUCT:551 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:552 ==== */
/**
 * A group in a QueryResult.
 */
export class QueryResultGroup extends Struct {
  static metatype: StructType = StructType.QUERY_RESULT_GROUP;
  static __isFrozen__: boolean = false;

  /**
   * QueryResultGroup.type
   */
  type: QueryType;

  /**
   * QueryResultGroup.discriminator
   */
  discriminator: Value;

  /**
   * QueryResultGroup.nodes
   */
  nodes: Array<Value>;

  /**
   * QueryResultGroup.count
   */
  count: number | null;

  /**
   * QueryResultGroup.exists
   */
  exists: boolean | null;

  /**
   * QueryResultGroup.scalar
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
      _nodes = [];
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
    if (!(this.metatype === other.metatype)) {
      return false;
    }
    if (!(this.type === other.type)) {
      return false;
    }
    if (!this.discriminator.equals(other.discriminator)) {
      return false;
    }
    if (this.nodes.length !== other.nodes.length) {
      return false;
    }
    for (let i = 0; i < this.nodes.length; i++) {
      if (!this.nodes[i].equals(other.nodes[i])) {
        return false;
      }
    }
    if (!(this.count === other.count)) {
      return false;
    }
    if (!(this.exists === other.exists)) {
      return false;
    }
    if (
      (this.scalar == null) !== (other.scalar == null) ||
      (this.scalar != null && !this.scalar.equals(other.scalar))
    ) {
      return false;
    }
    return true;
  }

  repr(): string {
    const propertyReprs: string[] = [];
    propertyReprs.push(`type=${QueryType[this.type]}`);
    propertyReprs.push(`discriminator=${this.discriminator.repr()}`);
    if (this.count !== null) {
      propertyReprs.push(`count=${this.count}`);
    }
    if (this.exists !== null) {
      propertyReprs.push(`exists=${this.exists}`);
    }
    if (this.scalar !== null) {
      propertyReprs.push(`scalar=${this.scalar.repr()}`);
    }
    return `<QueryResultGroup ${propertyReprs.join(" ")}>`;
  }

  hash(): number {
    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    h = (h * 31 + this.type) & 0xffffffff;
    h = (h * 31 + this.discriminator.hash()) & 0xffffffff;
    if (this.nodes && this.nodes.length > 0) {
      for (const _item of this.nodes) {
        h = (h * 31 + _item.hash()) & 0xffffffff;
      }
    }
    if (this.count !== null) {
      h = (h * 31 + hashInt(this.count)) & 0xffffffff;
    }
    if (this.exists !== null) {
      h = (h * 31 + hashBool(this.exists)) & 0xffffffff;
    }
    if (this.scalar !== null) {
      h = (h * 31 + this.scalar.hash()) & 0xffffffff;
    }

    return h;
  }

  validate(): void {
    throw new Error("not implemented");
  }

  toValue(): { [key: string]: any } {
    return QueryResultGroup.__packValue__(this);
  }

  static __packValue__(object: QueryResultGroup): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 552;
    objectValue["100"] = object.type;
    objectValue["101"] = object.discriminator.toValue();
    if (object.nodes.length > 0) {
      const packedNodes: any[] = [];
      for (const item of object.nodes) {
        packedNodes.push(item.toValue());
      }
      objectValue["110"] = packedNodes;
    }
    if (object.count != null) {
      objectValue["111"] = object.count;
    }
    if (object.exists != null) {
      objectValue["112"] = object.exists;
    }
    if (object.scalar != null) {
      objectValue["113"] = object.scalar.toValue();
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
    const _Value = STRUCT_CLASS_BY_TYPE[StructType.VALUE] as typeof Value;
    const unpackedNodes: any[] = [];
    if (objectValue["110"] != undefined) {
      for (const item of objectValue["110"]) {
        unpackedNodes.push(_Value.fromValue(item, _session, _supergraph, _graph, _connection));
      }
    }
    const countValue = objectValue["111"];
    const unpackedCount = countValue != undefined ? Number(countValue) : null;
    const existsValue = objectValue["112"];
    const unpackedExists = existsValue != undefined ? existsValue : null;
    const scalarValue = objectValue["113"];
    const unpackedScalar =
      scalarValue != undefined
        ? _Value.fromValue(scalarValue, _session, _supergraph, _graph, _connection)
        : null;
    return new QueryResultGroup({
      type: Number(objectValue["100"]),
      discriminator: _Value.fromValue(
        objectValue["101"],
        _session,
        _supergraph,
        _graph,
        _connection,
      ),
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
    return QueryResultGroup.__unpackValue__(
      objectValue,
      _session,
      _supergraph,
      _graph,
      _connection,
    );
  }

  toProto(): QueryResultGroupProto {
    return QueryResultGroup.__packProto__(this);
  }

  static __packProto__(object: QueryResultGroup): QueryResultGroupProto {
    const objectProto: Partial<QueryResultGroupProto> = { metatype: 552 };
    objectProto.type = Number(object.type) as QueryTypeProto;
    objectProto.discriminator = object.discriminator.toProto();
    if (object.nodes) {
      const packedNodes: any[] = [];
      for (const item of object.nodes) {
        packedNodes.push(item.toProto());
      }
      objectProto.nodes = packedNodes;
    }
    if (object.count != null) {
      objectProto.count = object.count;
    }
    if (object.exists != null) {
      objectProto.exists = object.exists;
    }
    if (object.scalar != null) {
      objectProto.scalar = object.scalar.toProto();
    }
    return objectProto as QueryResultGroupProto;
  }

  static __unpackProto__(
    objectProto: QueryResultGroupProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): QueryResultGroup {
    const _Value = STRUCT_CLASS_BY_TYPE[StructType.VALUE] as typeof Value;
    const unpackedNodes: any[] = [];
    if (objectProto.nodes) {
      for (const item of objectProto.nodes) {
        unpackedNodes.push(_Value.fromProto(item!, _session, _supergraph, _graph, _connection));
      }
    }
    return new QueryResultGroup({
      type: Number(objectProto.type) as QueryType,
      discriminator: _Value.fromProto(
        objectProto.discriminator!,
        _session,
        _supergraph,
        _graph,
        _connection,
      ),
      nodes: unpackedNodes,
      count: objectProto.count != undefined ? Number(objectProto.count) : null,
      exists: objectProto.exists != undefined ? objectProto.exists : null,
      scalar:
        objectProto.scalar != undefined
          ? _Value.fromProto(objectProto.scalar!, _session, _supergraph, _graph, _connection)
          : null,
      _supergraph,
    });
  }

  static fromProto(
    objectProto: QueryResultGroupProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): QueryResultGroup {
    return QueryResultGroup.__unpackProto__(
      objectProto,
      _session,
      _supergraph,
      _graph,
      _connection,
    );
  }

  static fromProtoString(packedProtoString: string): QueryResultGroup {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = QueryResultGroupProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerStructClass(StructType.QUERY_RESULT_GROUP, QueryResultGroup);
/* ==== DESTACK_GENERATED_END:STRUCT:552 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:555 ==== */
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
    if (!(this.metatype === other.metatype)) {
      return false;
    }
    return true;
  }

  repr(): string {
    return `<Selection>`;
  }

  hash(): number {
    if (this._hash !== null) {
      return this._hash;
    }

    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;

    // @ts-expect-error(readonly)
    this._hash = h;
    return h;
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
    objectValue["1"] = 555;
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

  toProto(): SelectionProto {
    if (this._proto === null) {
      // @ts-expect-error(readonly)
      this._proto = Selection.__packProto__(this);
    }
    return this._proto as SelectionProto;
  }

  static __packProto__(object: Selection): SelectionProto {
    const objectProto: Partial<SelectionProto> = { metatype: 555 };
    return objectProto as SelectionProto;
  }

  static __unpackProto__(
    objectProto: SelectionProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Selection {
    return new Selection({
      _proto: objectProto,
      _supergraph,
    });
  }

  static fromProto(
    objectProto: SelectionProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Selection {
    return Selection.__unpackProto__(objectProto, _session, _supergraph, _graph, _connection);
  }

  static fromProtoString(packedProtoString: string): Selection {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = SelectionProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerStructClass(StructType.SELECTION, Selection);
/* ==== DESTACK_GENERATED_END:STRUCT:555 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:554 ==== */
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
      _buckets = [];
    }
    this.buckets = _buckets;
    let _counts = options.counts ?? null;
    if (_counts === null) {
      _counts = [];
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
    if (!(this.metatype === other.metatype)) {
      return false;
    }
    if (this.buckets.length !== other.buckets.length) {
      return false;
    }
    for (let i = 0; i < this.buckets.length; i++) {
      if (!this.buckets[i].equals(other.buckets[i])) {
        return false;
      }
    }
    if (this.counts.length !== other.counts.length) {
      return false;
    }
    for (let i = 0; i < this.counts.length; i++) {
      if (!(this.counts[i] === other.counts[i])) {
        return false;
      }
    }
    return true;
  }

  repr(): string {
    if (this._repr === null) {
      const propertyReprs: string[] = [];
      if (this.buckets.length > 0) {
        propertyReprs.push(`buckets=${this.buckets.map((_item) => _item.repr()).join(", ")}`);
      }
      if (this.counts.length > 0) {
        propertyReprs.push(`counts=${this.counts.map((_item) => _item).join(", ")}`);
      }
      if (propertyReprs.length > 0) {
        // @ts-expect-error(readonly)
        this._repr = `<Histogram ${propertyReprs.join(" ")}>`;
      } else {
        // @ts-expect-error(readonly)
        this._repr = `<Histogram>`;
      }
    }
    return this._repr;
  }

  hash(): number {
    if (this._hash !== null) {
      return this._hash;
    }

    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    if (this.buckets && this.buckets.length > 0) {
      for (const _item of this.buckets) {
        h = (h * 31 + _item.hash()) & 0xffffffff;
      }
    }
    if (this.counts && this.counts.length > 0) {
      for (const _item of this.counts) {
        h = (h * 31 + hashInt(_item)) & 0xffffffff;
      }
    }

    // @ts-expect-error(readonly)
    this._hash = h;
    return h;
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
    objectValue["1"] = 554;
    if (object.buckets.length > 0) {
      const packedBuckets: any[] = [];
      for (const item of object.buckets) {
        packedBuckets.push(item.toValue());
      }
      objectValue["101"] = packedBuckets;
    }
    if (object.counts.length > 0) {
      const packedCounts: any[] = [];
      for (const item of object.counts) {
        packedCounts.push(item);
      }
      objectValue["102"] = packedCounts;
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
    const _Value = STRUCT_CLASS_BY_TYPE[StructType.VALUE] as typeof Value;
    const unpackedBuckets: any[] = [];
    if (objectValue["101"] != undefined) {
      for (const item of objectValue["101"]) {
        unpackedBuckets.push(_Value.fromValue(item, _session, _supergraph, _graph, _connection));
      }
    }
    const unpackedCounts: any[] = [];
    if (objectValue["102"] != undefined) {
      for (const item of objectValue["102"]) {
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

  toProto(): HistogramProto {
    if (this._proto === null) {
      // @ts-expect-error(readonly)
      this._proto = Histogram.__packProto__(this);
    }
    return this._proto as HistogramProto;
  }

  static __packProto__(object: Histogram): HistogramProto {
    const objectProto: Partial<HistogramProto> = { metatype: 554 };
    if (object.buckets) {
      const packedBuckets: any[] = [];
      for (const item of object.buckets) {
        packedBuckets.push(item.toProto());
      }
      objectProto.buckets = packedBuckets;
    }
    if (object.counts) {
      const packedCounts: any[] = [];
      for (const item of object.counts) {
        packedCounts.push(item);
      }
      objectProto.counts = packedCounts;
    }
    return objectProto as HistogramProto;
  }

  static __unpackProto__(
    objectProto: HistogramProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Histogram {
    const _Value = STRUCT_CLASS_BY_TYPE[StructType.VALUE] as typeof Value;
    const unpackedBuckets: any[] = [];
    if (objectProto.buckets) {
      for (const item of objectProto.buckets) {
        unpackedBuckets.push(_Value.fromProto(item!, _session, _supergraph, _graph, _connection));
      }
    }
    const unpackedCounts: any[] = [];
    if (objectProto.counts) {
      for (const item of objectProto.counts) {
        unpackedCounts.push(Number(item));
      }
    }
    return new Histogram({
      buckets: unpackedBuckets,
      counts: unpackedCounts,
      _proto: objectProto,
      _supergraph,
    });
  }

  static fromProto(
    objectProto: HistogramProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Histogram {
    return Histogram.__unpackProto__(objectProto, _session, _supergraph, _graph, _connection);
  }

  static fromProtoString(packedProtoString: string): Histogram {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = HistogramProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerStructClass(StructType.HISTOGRAM, Histogram);
/* ==== DESTACK_GENERATED_END:STRUCT:554 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:10108 ==== */
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

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerEnumClass(EnumType.FUNCTION_TYPE, FunctionType);
/* ==== DESTACK_GENERATED_END:ENUM:10108 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:10103 ==== */
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

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerEnumClass(EnumType.CONDITIONAL_TYPE, ConditionalType);
/* ==== DESTACK_GENERATED_END:ENUM:10103 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:10104 ==== */
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

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerEnumClass(EnumType.AGGREGATION_TYPE, AggregationType);
/* ==== DESTACK_GENERATED_END:ENUM:10104 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:10109 ==== */
/**
 * ExpressionType
 */
export enum ExpressionType {
  LITERAL = 1,
  ATTRIBUTE = 2,
  CONDITION = 3,
  FUNCTION = 4,
  AGGREGATION = 5,

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerEnumClass(EnumType.EXPRESSION_TYPE, ExpressionType);
/* ==== DESTACK_GENERATED_END:ENUM:10109 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:10106 ==== */
/**
 * SortType
 */
export enum SortType {
  ASCENDING = 1,
  DESCENDING = 2,

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerEnumClass(EnumType.SORT_TYPE, SortType);
/* ==== DESTACK_GENERATED_END:ENUM:10106 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:10105 ==== */
/**
 * SortMode
 */
export enum SortMode {
  MAX = 1,
  MIN = 2,
  AVERAGE = 3,
  SUM = 4,
  MEDIAN = 5,

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerEnumClass(EnumType.SORT_MODE, SortMode);
/* ==== DESTACK_GENERATED_END:ENUM:10105 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:10107 ==== */
/**
 * JoinType
 */
export enum JoinType {
  LEFT = 1,
  PARENT = 10,
  CHILD = 11,

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerEnumClass(EnumType.JOIN_TYPE, JoinType);
/* ==== DESTACK_GENERATED_END:ENUM:10107 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:10120 ==== */
/**
 * QueryType
 */
export enum QueryType {
  NODE = 1,
  SCALAR = 2,
  GROUPED_NODE = 10,
  GROUPED_SCALAR = 11,

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerEnumClass(EnumType.QUERY_TYPE, QueryType);
/* ==== DESTACK_GENERATED_END:ENUM:10120 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:10121 ==== */
/**
 * QueryUpdateType
 */
export enum QueryUpdateType {
  FULL_RESULT = 1,
  PARTIAL_RESULT = 2,

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerEnumClass(EnumType.QUERY_UPDATE_TYPE, QueryUpdateType);
/* ==== DESTACK_GENERATED_END:ENUM:10121 ==== */
