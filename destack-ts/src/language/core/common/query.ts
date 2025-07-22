import {
  EnumType,
  NodeType,
  StoreDomain,
  StructType,
  TypeCardinality,
} from "@destack/language/core/builtin/common";
import { activeSession } from "@destack/language/core/builtin/const";
import type { PropertyDefinition } from "@destack/language/core/builtin/definition";
import type { NodeClass } from "@destack/language/core/builtin/node";
import { Node, isNode } from "@destack/language/core/builtin/node";
import type {
  NodeDefinitionReference,
  NodeReference,
  PropertyReference,
} from "@destack/language/core/builtin/relation";
import { StructFrozen, isStruct } from "@destack/language/core/builtin/struct";
import type { CustomProperty } from "@destack/language/core/common/property";
import { Type } from "@destack/language/core/common/type";
import type { Value } from "@destack/language/core/common/value";
import { toValue } from "@destack/language/core/common/value";
import { GraphConnection, QueryConnection } from "@destack/language/core/runtime/connection";
import type { Graph, Supergraph } from "@destack/language/core/runtime/graph";
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
  JoinProto,
  JoinTypeProto,
  QueryProto,
  QueryTypeProto,
  SelectProto,
  SortModeProto,
  SortProto,
  SortTypeProto,
  StoreDomainProto,
} from "@destack/proto";
import { base64Decode } from "@destack/utils";
import { assertNever } from "@destack/utils/functools";
import { hashBool, hashInt, hashString } from "@destack/utils/hash";
import { uuid4 } from "@destack/utils/uuid";

/* ==== DESTACK_GENERATED_START:STRUCT:201 ==== */
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
    _graph?: Supergraph | null;
    _hash?: number | null;
    _repr?: string | null;
    _proto?: any | null;
    _cson?: any | null;
  }) {
    super(
      // session
      options._session ?? null,
      // supergraph
      options._graph ?? null,
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
    this._cson = options._cson ?? null;
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
      if (this.right != null) {
        propertyReprs.push(`right=${this.right.repr()}`);
      }
      // @ts-expect-error(readonly)
      this._repr = `<Function ${propertyReprs.join(" ")}>`;
    }
    return this._repr;
  }

  hash(): number {
    if (this._hash != null) {
      return this._hash;
    }
    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    h = (h * 31 + this.type) & 0xffffffff;
    h = (h * 31 + this.left.hash()) & 0xffffffff;
    if (this.right != null) {
      h = (h * 31 + this.right.hash()) & 0xffffffff;
    }
    // @ts-expect-error(readonly)
    this._hash = h;
    return h;
  }

  validate(): void {
    throw new Error("not implemented");
  }

  toCson(): { [key: string]: any } {
    if (this._cson === null) {
      // @ts-expect-error(readonly)
      this._cson = Function.__packCson__(this);
    }
    return this._cson;
  }

  static __packCson__(object: Function): { [key: string]: any } {
    const objectCson: { [key: string]: any } = {};
    objectCson["1"] = 201;
    objectCson["100"] = object.type;
    objectCson["101"] = object.left.toCson();
    if (object.right != null) {
      objectCson["102"] = object.right.toCson();
    }
    return objectCson;
  }

  static __unpackCson__(
    objectCson: { [key: string]: any },
    _session?: Session | null,
    _graph?: Graph | null,
    _connection?: GraphConnection | null,
  ): Function {
    const _Expression = STRUCT_CLASS_BY_TYPE[StructType.EXPRESSION] as typeof Expression;
    const rightValue = objectCson["102"];
    const unpackedRight =
      rightValue != undefined
        ? _Expression.fromCson(rightValue, _session, _graph, _connection)
        : null;
    return new Function({
      type: Number(objectCson["100"]),
      left: _Expression.fromCson(objectCson["101"], _session, _graph, _connection),
      right: unpackedRight,
      _cson: objectCson,
      _graph,
    });
  }

  static fromCson(
    objectCson: { readonly [key: string]: any },
    _session?: Session | null,
    _graph?: Graph | null,
    _connection?: GraphConnection | null,
  ): Function {
    return Function.__unpackCson__(objectCson, _session, _graph, _connection);
  }

  toProto(): FunctionProto {
    if (this._proto === null) {
      // @ts-expect-error(readonly)
      this._proto = Function.__packProto__(this);
    }
    return this._proto as FunctionProto;
  }

  static __packProto__(object: Function): FunctionProto {
    const objectProto: Partial<FunctionProto> = { metatype: 201 };
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
    _graph?: Supergraph | null,
    _connection?: GraphConnection | null,
  ): Function {
    const _Expression = STRUCT_CLASS_BY_TYPE[StructType.EXPRESSION] as typeof Expression;
    return new Function({
      type: Number(objectProto.type) as FunctionType,
      left: _Expression.fromProto(objectProto.left!, _session, _graph, _graph, _connection),
      right:
        objectProto.right != undefined
          ? _Expression.fromProto(objectProto.right!, _session, _graph, _graph, _connection)
          : null,
      _proto: objectProto,
      _graph,
    });
  }

  static fromProto(
    objectProto: FunctionProto,
    _session?: Session | null,
    _graph?: Supergraph | null,
    _connection?: GraphConnection | null,
  ): Function {
    return Function.__unpackProto__(objectProto, _session, _graph, _connection);
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
/* ==== DESTACK_GENERATED_END:STRUCT:201 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:204 ==== */
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
    _graph?: Supergraph | null;
    _hash?: number | null;
    _repr?: string | null;
    _proto?: any | null;
    _cson?: any | null;
  }) {
    super(
      // session
      options._session ?? null,
      // supergraph
      options._graph ?? null,
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
    this._cson = options._cson ?? null;
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
      if (this.right != null) {
        propertyReprs.push(`right=${this.right.repr()}`);
      }
      // @ts-expect-error(readonly)
      this._repr = `<Condition ${propertyReprs.join(" ")}>`;
    }
    return this._repr;
  }

  hash(): number {
    if (this._hash != null) {
      return this._hash;
    }
    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    h = (h * 31 + this.type) & 0xffffffff;
    h = (h * 31 + this.left.hash()) & 0xffffffff;
    if (this.right != null) {
      h = (h * 31 + this.right.hash()) & 0xffffffff;
    }
    // @ts-expect-error(readonly)
    this._hash = h;
    return h;
  }

  validate(): void {
    throw new Error("not implemented");
  }

  toCson(): { [key: string]: any } {
    if (this._cson === null) {
      // @ts-expect-error(readonly)
      this._cson = Condition.__packCson__(this);
    }
    return this._cson;
  }

  static __packCson__(object: Condition): { [key: string]: any } {
    const objectCson: { [key: string]: any } = {};
    objectCson["1"] = 204;
    objectCson["100"] = object.type;
    objectCson["101"] = object.left.toCson();
    if (object.right != null) {
      objectCson["102"] = object.right.toCson();
    }
    return objectCson;
  }

  static __unpackCson__(
    objectCson: { [key: string]: any },
    _session?: Session | null,
    _graph?: Graph | null,
    _connection?: GraphConnection | null,
  ): Condition {
    const _Expression = STRUCT_CLASS_BY_TYPE[StructType.EXPRESSION] as typeof Expression;
    const rightValue = objectCson["102"];
    const unpackedRight =
      rightValue != undefined
        ? _Expression.fromCson(rightValue, _session, _graph, _connection)
        : null;
    return new Condition({
      type: Number(objectCson["100"]),
      left: _Expression.fromCson(objectCson["101"], _session, _graph, _connection),
      right: unpackedRight,
      _cson: objectCson,
      _graph,
    });
  }

  static fromCson(
    objectCson: { readonly [key: string]: any },
    _session?: Session | null,
    _graph?: Graph | null,
    _connection?: GraphConnection | null,
  ): Condition {
    return Condition.__unpackCson__(objectCson, _session, _graph, _connection);
  }

  toProto(): ConditionProto {
    if (this._proto === null) {
      // @ts-expect-error(readonly)
      this._proto = Condition.__packProto__(this);
    }
    return this._proto as ConditionProto;
  }

  static __packProto__(object: Condition): ConditionProto {
    const objectProto: Partial<ConditionProto> = { metatype: 204 };
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
    _graph?: Supergraph | null,
    _connection?: GraphConnection | null,
  ): Condition {
    const _Expression = STRUCT_CLASS_BY_TYPE[StructType.EXPRESSION] as typeof Expression;
    return new Condition({
      type: Number(objectProto.type) as ConditionalType,
      left: _Expression.fromProto(objectProto.left!, _session, _graph, _graph, _connection),
      right:
        objectProto.right != undefined
          ? _Expression.fromProto(objectProto.right!, _session, _graph, _graph, _connection)
          : null,
      _proto: objectProto,
      _graph,
    });
  }

  static fromProto(
    objectProto: ConditionProto,
    _session?: Session | null,
    _graph?: Supergraph | null,
    _connection?: GraphConnection | null,
  ): Condition {
    return Condition.__unpackProto__(objectProto, _session, _graph, _connection);
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
    type: ConditionalType,
    value: any = null,
  ): Condition {
    const left = Expression.of(attribute);
    if (value != null) {
      let valueType: Type = attribute.toType();
      if (type == ConditionalType.IN || type == ConditionalType.NOT_IN) {
        valueType = new Type({ ...valueType, cardinality: TypeCardinality.LIST });
      }
      const right = Expression.of(toValue(value, valueType));
      return new Condition({ type, left, right });
    } else {
      return new Condition({ type, left });
    }
  }

  /* ==== DESTACK_CUSTOM_END ==== */
}
registerStructClass(StructType.CONDITION, Condition);
/* ==== DESTACK_GENERATED_END:STRUCT:204 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:203 ==== */
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
    _graph?: Supergraph | null;
    _hash?: number | null;
    _repr?: string | null;
    _proto?: any | null;
    _cson?: any | null;
  }) {
    super(
      // session
      options._session ?? null,
      // supergraph
      options._graph ?? null,
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
    this._cson = options._cson ?? null;
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
      if (this.expression != null) {
        propertyReprs.push(`expression=${this.expression.repr()}`);
      }
      // @ts-expect-error(readonly)
      this._repr = `<Aggregation ${propertyReprs.join(" ")}>`;
    }
    return this._repr;
  }

  hash(): number {
    if (this._hash != null) {
      return this._hash;
    }
    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    h = (h * 31 + this.type) & 0xffffffff;
    if (this.expression != null) {
      h = (h * 31 + this.expression.hash()) & 0xffffffff;
    }
    // @ts-expect-error(readonly)
    this._hash = h;
    return h;
  }

  validate(): void {
    throw new Error("not implemented");
  }

  toCson(): { [key: string]: any } {
    if (this._cson === null) {
      // @ts-expect-error(readonly)
      this._cson = Aggregation.__packCson__(this);
    }
    return this._cson;
  }

  static __packCson__(object: Aggregation): { [key: string]: any } {
    const objectCson: { [key: string]: any } = {};
    objectCson["1"] = 203;
    objectCson["100"] = object.type;
    if (object.expression != null) {
      objectCson["101"] = object.expression.toCson();
    }
    return objectCson;
  }

  static __unpackCson__(
    objectCson: { [key: string]: any },
    _session?: Session | null,
    _graph?: Graph | null,
    _connection?: GraphConnection | null,
  ): Aggregation {
    const _Expression = STRUCT_CLASS_BY_TYPE[StructType.EXPRESSION] as typeof Expression;
    const expressionValue = objectCson["101"];
    const unpackedExpression =
      expressionValue != undefined
        ? _Expression.fromCson(expressionValue, _session, _graph, _connection)
        : null;
    return new Aggregation({
      type: Number(objectCson["100"]),
      expression: unpackedExpression,
      _cson: objectCson,
      _graph,
    });
  }

  static fromCson(
    objectCson: { readonly [key: string]: any },
    _session?: Session | null,
    _graph?: Graph | null,
    _connection?: GraphConnection | null,
  ): Aggregation {
    return Aggregation.__unpackCson__(objectCson, _session, _graph, _connection);
  }

  toProto(): AggregationProto {
    if (this._proto === null) {
      // @ts-expect-error(readonly)
      this._proto = Aggregation.__packProto__(this);
    }
    return this._proto as AggregationProto;
  }

  static __packProto__(object: Aggregation): AggregationProto {
    const objectProto: Partial<AggregationProto> = { metatype: 203 };
    objectProto.type = Number(object.type) as AggregationTypeProto;
    if (object.expression != null) {
      objectProto.expression = object.expression.toProto();
    }
    return objectProto as AggregationProto;
  }

  static __unpackProto__(
    objectProto: AggregationProto,
    _session?: Session | null,
    _graph?: Supergraph | null,
    _connection?: GraphConnection | null,
  ): Aggregation {
    const _Expression = STRUCT_CLASS_BY_TYPE[StructType.EXPRESSION] as typeof Expression;
    return new Aggregation({
      type: Number(objectProto.type) as AggregationType,
      expression:
        objectProto.expression != undefined
          ? _Expression.fromProto(objectProto.expression!, _session, _graph, _graph, _connection)
          : null,
      _proto: objectProto,
      _graph,
    });
  }

  static fromProto(
    objectProto: AggregationProto,
    _session?: Session | null,
    _graph?: Supergraph | null,
    _connection?: GraphConnection | null,
  ): Aggregation {
    return Aggregation.__unpackProto__(objectProto, _session, _graph, _connection);
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
/* ==== DESTACK_GENERATED_END:STRUCT:203 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:200 ==== */
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
    _graph?: Supergraph | null;
    _hash?: number | null;
    _repr?: string | null;
    _proto?: any | null;
    _cson?: any | null;
  }) {
    super(
      // session
      options._session ?? null,
      // supergraph
      options._graph ?? null,
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
    this._cson = options._cson ?? null;
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
      if (this.literal != null) {
        propertyReprs.push(`literal=${this.literal.repr()}`);
      }
      if (this.attribute != null) {
        propertyReprs.push(`attribute=${this.attribute.repr()}`);
      }
      if (this.condition != null) {
        propertyReprs.push(`condition=${this.condition.repr()}`);
      }
      if (this.function != null) {
        propertyReprs.push(`function=${this.function.repr()}`);
      }
      if (this.aggregation != null) {
        propertyReprs.push(`aggregation=${this.aggregation.repr()}`);
      }
      // @ts-expect-error(readonly)
      this._repr = `<Expression ${propertyReprs.join(" ")}>`;
    }
    return this._repr;
  }

  hash(): number {
    if (this._hash != null) {
      return this._hash;
    }
    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    h = (h * 31 + this.type) & 0xffffffff;
    if (this.literal != null) {
      h = (h * 31 + this.literal.hash()) & 0xffffffff;
    }
    if (this.attribute != null) {
      h = (h * 31 + this.attribute.hash()) & 0xffffffff;
    }
    if (this.condition != null) {
      h = (h * 31 + this.condition.hash()) & 0xffffffff;
    }
    if (this.function != null) {
      h = (h * 31 + this.function.hash()) & 0xffffffff;
    }
    if (this.aggregation != null) {
      h = (h * 31 + this.aggregation.hash()) & 0xffffffff;
    }
    // @ts-expect-error(readonly)
    this._hash = h;
    return h;
  }

  validate(): void {
    throw new Error("not implemented");
  }

  toCson(): { [key: string]: any } {
    if (this._cson === null) {
      // @ts-expect-error(readonly)
      this._cson = Expression.__packCson__(this);
    }
    return this._cson;
  }

  static __packCson__(object: Expression): { [key: string]: any } {
    const objectCson: { [key: string]: any } = {};
    objectCson["1"] = 200;
    objectCson["100"] = object.type;
    if (object.literal != null) {
      objectCson["101"] = object.literal.toCson();
    }
    if (object.attribute != null) {
      objectCson["102"] = object.attribute.toCson();
    }
    if (object.condition != null) {
      objectCson["103"] = object.condition.toCson();
    }
    if (object.function != null) {
      objectCson["104"] = object.function.toCson();
    }
    if (object.aggregation != null) {
      objectCson["105"] = object.aggregation.toCson();
    }
    return objectCson;
  }

  static __unpackCson__(
    objectCson: { [key: string]: any },
    _session?: Session | null,
    _graph?: Graph | null,
    _connection?: GraphConnection | null,
  ): Expression {
    const _Value = STRUCT_CLASS_BY_TYPE[StructType.VALUE] as typeof Value;
    const _Function = STRUCT_CLASS_BY_TYPE[StructType.FUNCTION] as typeof Function;
    const _Aggregation = STRUCT_CLASS_BY_TYPE[StructType.AGGREGATION] as typeof Aggregation;
    const _Condition = STRUCT_CLASS_BY_TYPE[StructType.CONDITION] as typeof Condition;
    const _PropertyReference = STRUCT_CLASS_BY_TYPE[
      StructType.PROPERTY_REFERENCE
    ] as typeof PropertyReference;
    const literalValue = objectCson["101"];
    const unpackedLiteral =
      literalValue != undefined
        ? _Value.fromCson(literalValue, _session, _graph, _connection)
        : null;
    const attributeValue = objectCson["102"];
    const unpackedAttribute =
      attributeValue != undefined
        ? _PropertyReference.fromCson(attributeValue, _session, _graph, _connection)
        : null;
    const conditionValue = objectCson["103"];
    const unpackedCondition =
      conditionValue != undefined
        ? _Condition.fromCson(conditionValue, _session, _graph, _connection)
        : null;
    const functionValue = objectCson["104"];
    const unpackedFunction =
      functionValue != undefined
        ? _Function.fromCson(functionValue, _session, _graph, _connection)
        : null;
    const aggregationValue = objectCson["105"];
    const unpackedAggregation =
      aggregationValue != undefined
        ? _Aggregation.fromCson(aggregationValue, _session, _graph, _connection)
        : null;
    return new Expression({
      type: Number(objectCson["100"]),
      literal: unpackedLiteral,
      attribute: unpackedAttribute,
      condition: unpackedCondition,
      function: unpackedFunction,
      aggregation: unpackedAggregation,
      _cson: objectCson,
      _graph,
    });
  }

  static fromCson(
    objectCson: { readonly [key: string]: any },
    _session?: Session | null,
    _graph?: Graph | null,
    _connection?: GraphConnection | null,
  ): Expression {
    return Expression.__unpackCson__(objectCson, _session, _graph, _connection);
  }

  toProto(): ExpressionProto {
    if (this._proto === null) {
      // @ts-expect-error(readonly)
      this._proto = Expression.__packProto__(this);
    }
    return this._proto as ExpressionProto;
  }

  static __packProto__(object: Expression): ExpressionProto {
    const objectProto: Partial<ExpressionProto> = { metatype: 200 };
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
    _graph?: Supergraph | null,
    _connection?: GraphConnection | null,
  ): Expression {
    const _Value = STRUCT_CLASS_BY_TYPE[StructType.VALUE] as typeof Value;
    const _Function = STRUCT_CLASS_BY_TYPE[StructType.FUNCTION] as typeof Function;
    const _Aggregation = STRUCT_CLASS_BY_TYPE[StructType.AGGREGATION] as typeof Aggregation;
    const _Condition = STRUCT_CLASS_BY_TYPE[StructType.CONDITION] as typeof Condition;
    const _PropertyReference = STRUCT_CLASS_BY_TYPE[
      StructType.PROPERTY_REFERENCE
    ] as typeof PropertyReference;
    return new Expression({
      type: Number(objectProto.type) as ExpressionType,
      literal:
        objectProto.literal != undefined
          ? _Value.fromProto(objectProto.literal!, _session, _graph, _graph, _connection)
          : null,
      attribute:
        objectProto.attribute != undefined
          ? _PropertyReference.fromProto(
              objectProto.attribute!,
              _session,
              _graph,
              _graph,
              _connection,
            )
          : null,
      condition:
        objectProto.condition != undefined
          ? _Condition.fromProto(objectProto.condition!, _session, _graph, _graph, _connection)
          : null,
      function:
        objectProto.function != undefined
          ? _Function.fromProto(objectProto.function!, _session, _graph, _graph, _connection)
          : null,
      aggregation:
        objectProto.aggregation != undefined
          ? _Aggregation.fromProto(objectProto.aggregation!, _session, _graph, _graph, _connection)
          : null,
      _proto: objectProto,
      _graph,
    });
  }

  static fromProto(
    objectProto: ExpressionProto,
    _session?: Session | null,
    _graph?: Supergraph | null,
    _connection?: GraphConnection | null,
  ): Expression {
    return Expression.__unpackProto__(objectProto, _session, _graph, _connection);
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
/* ==== DESTACK_GENERATED_END:STRUCT:200 ==== */

export type ExpressionIn =
  | Value
  | CustomProperty
  | PropertyReference
  | PropertyDefinition
  | Condition
  | Function
  | Aggregation
  | Expression;

/* ==== DESTACK_GENERATED_START:STRUCT:205 ==== */
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
    _graph?: Supergraph | null;
    _hash?: number | null;
    _repr?: string | null;
    _proto?: any | null;
    _cson?: any | null;
  }) {
    super(
      // session
      options._session ?? null,
      // supergraph
      options._graph ?? null,
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
    this._cson = options._cson ?? null;
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
      if (this.mode != null) {
        propertyReprs.push(`mode=${SortMode[this.mode]}`);
      }
      // @ts-expect-error(readonly)
      this._repr = `<Sort ${propertyReprs.join(" ")}>`;
    }
    return this._repr;
  }

  hash(): number {
    if (this._hash != null) {
      return this._hash;
    }
    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    h = (h * 31 + this.type) & 0xffffffff;
    h = (h * 31 + this.by.hash()) & 0xffffffff;
    if (this.mode != null) {
      h = (h * 31 + this.mode) & 0xffffffff;
    }
    // @ts-expect-error(readonly)
    this._hash = h;
    return h;
  }

  validate(): void {
    throw new Error("not implemented");
  }

  toCson(): { [key: string]: any } {
    if (this._cson === null) {
      // @ts-expect-error(readonly)
      this._cson = Sort.__packCson__(this);
    }
    return this._cson;
  }

  static __packCson__(object: Sort): { [key: string]: any } {
    const objectCson: { [key: string]: any } = {};
    objectCson["1"] = 205;
    objectCson["100"] = object.type;
    objectCson["101"] = object.by.toCson();
    if (object.mode != null) {
      objectCson["102"] = object.mode;
    }
    return objectCson;
  }

  static __unpackCson__(
    objectCson: { [key: string]: any },
    _session?: Session | null,
    _graph?: Graph | null,
    _connection?: GraphConnection | null,
  ): Sort {
    const _Expression = STRUCT_CLASS_BY_TYPE[StructType.EXPRESSION] as typeof Expression;
    const modeValue = objectCson["102"];
    const unpackedMode = modeValue != undefined ? Number(modeValue) : null;
    return new Sort({
      type: Number(objectCson["100"]),
      by: _Expression.fromCson(objectCson["101"], _session, _graph, _connection),
      mode: unpackedMode,
      _cson: objectCson,
      _graph,
    });
  }

  static fromCson(
    objectCson: { readonly [key: string]: any },
    _session?: Session | null,
    _graph?: Graph | null,
    _connection?: GraphConnection | null,
  ): Sort {
    return Sort.__unpackCson__(objectCson, _session, _graph, _connection);
  }

  toProto(): SortProto {
    if (this._proto === null) {
      // @ts-expect-error(readonly)
      this._proto = Sort.__packProto__(this);
    }
    return this._proto as SortProto;
  }

  static __packProto__(object: Sort): SortProto {
    const objectProto: Partial<SortProto> = { metatype: 205 };
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
    _graph?: Supergraph | null,
    _connection?: GraphConnection | null,
  ): Sort {
    const _Expression = STRUCT_CLASS_BY_TYPE[StructType.EXPRESSION] as typeof Expression;
    return new Sort({
      type: Number(objectProto.type) as SortType,
      by: _Expression.fromProto(objectProto.by!, _session, _graph, _graph, _connection),
      mode: objectProto.mode != undefined ? (Number(objectProto.mode) as SortMode) : null,
      _proto: objectProto,
      _graph,
    });
  }

  static fromProto(
    objectProto: SortProto,
    _session?: Session | null,
    _graph?: Supergraph | null,
    _connection?: GraphConnection | null,
  ): Sort {
    return Sort.__unpackProto__(objectProto, _session, _graph, _connection);
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
/* ==== DESTACK_GENERATED_END:STRUCT:205 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:206 ==== */
/**
 * Select specific Attributes.
 */
export class Select extends StructFrozen {
  static metatype: StructType = StructType.SELECT;
  static __isFrozen__: boolean = true;

  /**
   * Select.attributes
   */
  readonly attributes: readonly PropertyReference[];

  constructor(options: {
    attributes?: readonly PropertyReference[];
    _session?: Session | null;
    _graph?: Supergraph | null;
    _hash?: number | null;
    _repr?: string | null;
    _proto?: any | null;
    _cson?: any | null;
  }) {
    super(
      // session
      options._session ?? null,
      // supergraph
      options._graph ?? null,
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
    this._cson = options._cson ?? null;
  }

  equals(other: any): boolean {
    if (!(this.metatype === other.metatype)) {
      return false;
    }
    if (this.attributes.length != other.attributes.length) {
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
    if (this._hash != null) {
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

  toCson(): { [key: string]: any } {
    if (this._cson === null) {
      // @ts-expect-error(readonly)
      this._cson = Select.__packCson__(this);
    }
    return this._cson;
  }

  static __packCson__(object: Select): { [key: string]: any } {
    const objectCson: { [key: string]: any } = {};
    objectCson["1"] = 206;
    if (object.attributes.length > 0) {
      const packedAttributes: any[] = [];
      for (const item of object.attributes) {
        packedAttributes.push(item.toCson());
      }
      objectCson["101"] = packedAttributes;
    }
    return objectCson;
  }

  static __unpackCson__(
    objectCson: { [key: string]: any },
    _session?: Session | null,
    _graph?: Graph | null,
    _connection?: GraphConnection | null,
  ): Select {
    const _PropertyReference = STRUCT_CLASS_BY_TYPE[
      StructType.PROPERTY_REFERENCE
    ] as typeof PropertyReference;
    const unpackedAttributes: any[] = [];
    if (objectCson["101"] != undefined) {
      for (const item of objectCson["101"]) {
        unpackedAttributes.push(_PropertyReference.fromCson(item, _session, _graph, _connection));
      }
    }
    return new Select({
      attributes: unpackedAttributes,
      _cson: objectCson,
      _graph,
    });
  }

  static fromCson(
    objectCson: { readonly [key: string]: any },
    _session?: Session | null,
    _graph?: Graph | null,
    _connection?: GraphConnection | null,
  ): Select {
    return Select.__unpackCson__(objectCson, _session, _graph, _connection);
  }

  toProto(): SelectProto {
    if (this._proto === null) {
      // @ts-expect-error(readonly)
      this._proto = Select.__packProto__(this);
    }
    return this._proto as SelectProto;
  }

  static __packProto__(object: Select): SelectProto {
    const objectProto: Partial<SelectProto> = { metatype: 206 };
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
    _graph?: Supergraph | null,
    _connection?: GraphConnection | null,
  ): Select {
    const _PropertyReference = STRUCT_CLASS_BY_TYPE[
      StructType.PROPERTY_REFERENCE
    ] as typeof PropertyReference;
    const unpackedAttributes: any[] = [];
    if (objectProto.attributes) {
      for (const item of objectProto.attributes) {
        unpackedAttributes.push(
          _PropertyReference.fromProto(item!, _session, _graph, _graph, _connection),
        );
      }
    }
    return new Select({
      attributes: unpackedAttributes,
      _proto: objectProto,
      _graph,
    });
  }

  static fromProto(
    objectProto: SelectProto,
    _session?: Session | null,
    _graph?: Supergraph | null,
    _connection?: GraphConnection | null,
  ): Select {
    return Select.__unpackProto__(objectProto, _session, _graph, _connection);
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
/* ==== DESTACK_GENERATED_END:STRUCT:206 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:202 ==== */
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
   * Join.customDefinition
   */
  readonly customDefinition: NodeDefinitionReference | null;

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
    customDefinition?: NodeDefinitionReference | null;
    recursive?: boolean;
    depth?: number | null;
    on?: Condition | null;
    _session?: Session | null;
    _graph?: Supergraph | null;
    _hash?: number | null;
    _repr?: string | null;
    _proto?: any | null;
    _cson?: any | null;
  }) {
    super(
      // session
      options._session ?? null,
      // supergraph
      options._graph ?? null,
    );

    // properties
    let _type = options.type;
    if (_type === null) {
      throw new Error(`Join.type is required`);
    }
    this.type = _type;
    let _customDefinition = options.customDefinition ?? null;
    this.customDefinition = _customDefinition;
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
    this._cson = options._cson ?? null;
  }

  equals(other: any): boolean {
    if (!(this.metatype === other.metatype)) {
      return false;
    }
    if (!(this.type === other.type)) {
      return false;
    }
    if (
      (this.customDefinition == null) !== (other.customDefinition == null) ||
      (this.customDefinition != null && !this.customDefinition.equals(other.customDefinition))
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
      if (this.customDefinition != null) {
        propertyReprs.push(`customDefinition=${this.customDefinition.repr()}`);
      }
      propertyReprs.push(`recursive=${this.recursive}`);
      if (this.depth != null) {
        propertyReprs.push(`depth=${this.depth}`);
      }
      if (this.on != null) {
        propertyReprs.push(`on=${this.on.repr()}`);
      }
      // @ts-expect-error(readonly)
      this._repr = `<Join ${propertyReprs.join(" ")}>`;
    }
    return this._repr;
  }

  hash(): number {
    if (this._hash != null) {
      return this._hash;
    }
    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    h = (h * 31 + this.type) & 0xffffffff;
    if (this.customDefinition != null) {
      h = (h * 31 + this.customDefinition.hash()) & 0xffffffff;
    }
    h = (h * 31 + hashBool(this.recursive)) & 0xffffffff;
    if (this.depth != null) {
      h = (h * 31 + hashInt(this.depth)) & 0xffffffff;
    }
    if (this.on != null) {
      h = (h * 31 + this.on.hash()) & 0xffffffff;
    }
    // @ts-expect-error(readonly)
    this._hash = h;
    return h;
  }

  validate(): void {
    throw new Error("not implemented");
  }

  toCson(): { [key: string]: any } {
    if (this._cson === null) {
      // @ts-expect-error(readonly)
      this._cson = Join.__packCson__(this);
    }
    return this._cson;
  }

  static __packCson__(object: Join): { [key: string]: any } {
    const objectCson: { [key: string]: any } = {};
    objectCson["1"] = 202;
    objectCson["100"] = object.type;
    if (object.customDefinition != null) {
      objectCson["101"] = object.customDefinition.toCson();
    }
    objectCson["102"] = object.recursive;
    if (object.depth != null) {
      objectCson["103"] = object.depth;
    }
    if (object.on != null) {
      objectCson["104"] = object.on.toCson();
    }
    return objectCson;
  }

  static __unpackCson__(
    objectCson: { [key: string]: any },
    _session?: Session | null,
    _graph?: Graph | null,
    _connection?: GraphConnection | null,
  ): Join {
    const _NodeDefinitionReference = STRUCT_CLASS_BY_TYPE[
      StructType.NODE_DEFINITION_REFERENCE
    ] as typeof NodeDefinitionReference;
    const _Condition = STRUCT_CLASS_BY_TYPE[StructType.CONDITION] as typeof Condition;
    const customDefinitionValue = objectCson["101"];
    const unpackedCustomDefinition =
      customDefinitionValue != undefined
        ? _NodeDefinitionReference.fromCson(customDefinitionValue, _session, _graph, _connection)
        : null;
    const depthValue = objectCson["103"];
    const unpackedDepth = depthValue != undefined ? Number(depthValue) : null;
    const onValue = objectCson["104"];
    const unpackedOn =
      onValue != undefined ? _Condition.fromCson(onValue, _session, _graph, _connection) : null;
    return new Join({
      type: Number(objectCson["100"]),
      customDefinition: unpackedCustomDefinition,
      recursive: objectCson["102"],
      depth: unpackedDepth,
      on: unpackedOn,
      _cson: objectCson,
      _graph,
    });
  }

  static fromCson(
    objectCson: { readonly [key: string]: any },
    _session?: Session | null,
    _graph?: Graph | null,
    _connection?: GraphConnection | null,
  ): Join {
    return Join.__unpackCson__(objectCson, _session, _graph, _connection);
  }

  toProto(): JoinProto {
    if (this._proto === null) {
      // @ts-expect-error(readonly)
      this._proto = Join.__packProto__(this);
    }
    return this._proto as JoinProto;
  }

  static __packProto__(object: Join): JoinProto {
    const objectProto: Partial<JoinProto> = { metatype: 202 };
    objectProto.type = Number(object.type) as JoinTypeProto;
    if (object.customDefinition != null) {
      objectProto.customDefinition = object.customDefinition.toProto();
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
    _graph?: Supergraph | null,
    _connection?: GraphConnection | null,
  ): Join {
    const _NodeDefinitionReference = STRUCT_CLASS_BY_TYPE[
      StructType.NODE_DEFINITION_REFERENCE
    ] as typeof NodeDefinitionReference;
    const _Condition = STRUCT_CLASS_BY_TYPE[StructType.CONDITION] as typeof Condition;
    return new Join({
      type: Number(objectProto.type) as JoinType,
      customDefinition:
        objectProto.customDefinition != undefined
          ? _NodeDefinitionReference.fromProto(
              objectProto.customDefinition!,
              _session,
              _graph,
              _graph,
              _connection,
            )
          : null,
      recursive: objectProto.recursive,
      depth: objectProto.depth != undefined ? Number(objectProto.depth) : null,
      on:
        objectProto.on != undefined
          ? _Condition.fromProto(objectProto.on!, _session, _graph, _graph, _connection)
          : null,
      _proto: objectProto,
      _graph,
    });
  }

  static fromProto(
    objectProto: JoinProto,
    _session?: Session | null,
    _graph?: Supergraph | null,
    _connection?: GraphConnection | null,
  ): Join {
    return Join.__unpackProto__(objectProto, _session, _graph, _connection);
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
      definition?: NodeType | NodeClass | NodeReference;
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
      customDefinition: options?.definition
        ? _NodeDefinitionReference.of(options.definition)
        : null,
      recursive: options?.recursive ?? false,
      depth: options?.depth ?? null,
      on: options?.on ?? null,
    });
  }

  /* ==== DESTACK_CUSTOM_END ==== */
}
registerStructClass(StructType.JOIN, Join);
/* ==== DESTACK_GENERATED_END:STRUCT:202 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:300 ==== */
/**
 * A Query into the supergraph about Nodes (node or scalar and potentially grouped).
 * Queries may either be about Entities or Events.
 */
export class Query<T extends Node = Node> extends StructFrozen {
  static metatype: StructType = StructType.QUERY;
  static __isFrozen__: boolean = true;

  /**
   * Query.id
   */
  readonly id: string;

  /**
   * The type of Query.
   */
  readonly type: QueryType;

  /**
   * The domain of the Query (Entity or Event).
   */
  readonly domain: StoreDomain;

  /**
   * Name for this subquery. Should be unique within the parent Query.
   */
  readonly name: string;

  /**
   * The Node definition this Query is about.
   */
  readonly definition: NodeDefinitionReference;

  /**
   * Subqueries of this Query (if any).
   */
  readonly subqueries: readonly Query[];

  /**
   * How to join this Query to the parent Query (if any).
   */
  readonly join: Join | null;

  /**
   * What to select from the Query.
   */
  readonly select: Select | null;

  /**
   * Filter the Query.
   */
  readonly where: Condition | null;

  /**
   * Filter the Query groups (for grouped Queries).
   */
  readonly having: Condition | null;

  /**
   * Discriminator for grouped Queries.
   */
  readonly groupBy: readonly Expression[];

  /**
   * Aggregate the Query.
   */
  readonly aggregation: Aggregation | null;

  /**
   * How to sort the Query results.
   */
  readonly sort: readonly Sort[];

  /**
   * Limit the number of results.
   */
  readonly limit: number | null;

  /**
   * Offset the results.
   */
  readonly offset: number | null;

  constructor(options: {
    id?: string;
    type: QueryType;
    domain: StoreDomain;
    name: string;
    definition: NodeDefinitionReference;
    subqueries?: readonly Query[];
    join?: Join | null;
    select?: Select | null;
    where?: Condition | null;
    having?: Condition | null;
    groupBy?: readonly Expression[];
    aggregation?: Aggregation | null;
    sort?: readonly Sort[];
    limit?: number | null;
    offset?: number | null;
    _session?: Session | null;
    _graph?: Supergraph | null;
    _hash?: number | null;
    _repr?: string | null;
    _proto?: any | null;
    _cson?: any | null;
  }) {
    super(
      // session
      options._session ?? null,
      // supergraph
      options._graph ?? null,
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
    let _domain = options.domain;
    if (_domain === null) {
      throw new Error(`Query.domain is required`);
    }
    this.domain = _domain;
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

    // identity
    // @ts-expect-error(readonly)
    this._hash = options._hash ?? null;
    // @ts-expect-error(readonly)
    this._repr = options._repr ?? null;
    // @ts-expect-error(readonly)
    this._proto = options._proto ?? null;
    // @ts-expect-error(readonly)
    this._cson = options._cson ?? null;
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
    if (!(this.domain === other.domain)) {
      return false;
    }
    if (!(this.name === other.name)) {
      return false;
    }
    if (!this.definition.equals(other.definition)) {
      return false;
    }
    if (this.subqueries.length != other.subqueries.length) {
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
    if (this.groupBy.length != other.groupBy.length) {
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
    if (this.sort.length != other.sort.length) {
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
    return true;
  }

  repr(): string {
    if (this._repr === null) {
      const propertyReprs: string[] = [];
      propertyReprs.push(`type=${QueryType[this.type]}`);
      propertyReprs.push(`domain=${StoreDomain[this.domain]}`);
      propertyReprs.push(`name=${`"${this.name}"`}`);
      propertyReprs.push(`definition=${this.definition.repr()}`);
      if (this.subqueries.length > 0) {
        propertyReprs.push(`subqueries=${this.subqueries.map((_item) => _item.repr()).join(", ")}`);
      }
      if (this.join != null) {
        propertyReprs.push(`join=${this.join.repr()}`);
      }
      if (this.select != null) {
        propertyReprs.push(`select=${this.select.repr()}`);
      }
      if (this.where != null) {
        propertyReprs.push(`where=${this.where.repr()}`);
      }
      if (this.having != null) {
        propertyReprs.push(`having=${this.having.repr()}`);
      }
      if (this.groupBy.length > 0) {
        propertyReprs.push(`groupBy=${this.groupBy.map((_item) => _item.repr()).join(", ")}`);
      }
      if (this.aggregation != null) {
        propertyReprs.push(`aggregation=${this.aggregation.repr()}`);
      }
      if (this.sort.length > 0) {
        propertyReprs.push(`sort=${this.sort.map((_item) => _item.repr()).join(", ")}`);
      }
      if (this.limit != null) {
        propertyReprs.push(`limit=${this.limit}`);
      }
      if (this.offset != null) {
        propertyReprs.push(`offset=${this.offset}`);
      }
      // @ts-expect-error(readonly)
      this._repr = `<Query ${propertyReprs.join(" ")}>`;
    }
    return this._repr;
  }

  hash(): number {
    if (this._hash != null) {
      return this._hash;
    }
    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    h = (h * 31 + hashString(this.id.toString())) & 0xffffffff;
    h = (h * 31 + this.type) & 0xffffffff;
    h = (h * 31 + this.domain) & 0xffffffff;
    h = (h * 31 + hashString(this.name)) & 0xffffffff;
    h = (h * 31 + this.definition.hash()) & 0xffffffff;
    if (this.subqueries && this.subqueries.length > 0) {
      for (const _item of this.subqueries) {
        h = (h * 31 + _item.hash()) & 0xffffffff;
      }
    }
    if (this.join != null) {
      h = (h * 31 + this.join.hash()) & 0xffffffff;
    }
    if (this.select != null) {
      h = (h * 31 + this.select.hash()) & 0xffffffff;
    }
    if (this.where != null) {
      h = (h * 31 + this.where.hash()) & 0xffffffff;
    }
    if (this.having != null) {
      h = (h * 31 + this.having.hash()) & 0xffffffff;
    }
    if (this.groupBy && this.groupBy.length > 0) {
      for (const _item of this.groupBy) {
        h = (h * 31 + _item.hash()) & 0xffffffff;
      }
    }
    if (this.aggregation != null) {
      h = (h * 31 + this.aggregation.hash()) & 0xffffffff;
    }
    if (this.sort && this.sort.length > 0) {
      for (const _item of this.sort) {
        h = (h * 31 + _item.hash()) & 0xffffffff;
      }
    }
    if (this.limit != null) {
      h = (h * 31 + hashInt(this.limit)) & 0xffffffff;
    }
    if (this.offset != null) {
      h = (h * 31 + hashInt(this.offset)) & 0xffffffff;
    }
    // @ts-expect-error(readonly)
    this._hash = h;
    return h;
  }

  validate(): void {
    throw new Error("not implemented");
  }

  toCson(): { [key: string]: any } {
    if (this._cson === null) {
      // @ts-expect-error(readonly)
      this._cson = Query.__packCson__(this);
    }
    return this._cson;
  }

  static __packCson__(object: Query): { [key: string]: any } {
    const objectCson: { [key: string]: any } = {};
    objectCson["1"] = 300;
    objectCson["2"] = String(object.id);
    objectCson["100"] = object.type;
    objectCson["101"] = object.domain;
    objectCson["105"] = object.name;
    objectCson["106"] = object.definition.toCson();
    if (object.subqueries.length > 0) {
      const packedSubqueries: any[] = [];
      for (const item of object.subqueries) {
        packedSubqueries.push(item.toCson());
      }
      objectCson["109"] = packedSubqueries;
    }
    if (object.join != null) {
      objectCson["110"] = object.join.toCson();
    }
    if (object.select != null) {
      objectCson["111"] = object.select.toCson();
    }
    if (object.where != null) {
      objectCson["112"] = object.where.toCson();
    }
    if (object.having != null) {
      objectCson["113"] = object.having.toCson();
    }
    if (object.groupBy.length > 0) {
      const packedGroupBy: any[] = [];
      for (const item of object.groupBy) {
        packedGroupBy.push(item.toCson());
      }
      objectCson["114"] = packedGroupBy;
    }
    if (object.aggregation != null) {
      objectCson["115"] = object.aggregation.toCson();
    }
    if (object.sort.length > 0) {
      const packedSort: any[] = [];
      for (const item of object.sort) {
        packedSort.push(item.toCson());
      }
      objectCson["116"] = packedSort;
    }
    if (object.limit != null) {
      objectCson["120"] = object.limit;
    }
    if (object.offset != null) {
      objectCson["121"] = object.offset;
    }
    return objectCson;
  }

  static __unpackCson__(
    objectCson: { [key: string]: any },
    _session?: Session | null,
    _graph?: Graph | null,
    _connection?: GraphConnection | null,
  ): Query {
    const _NodeDefinitionReference = STRUCT_CLASS_BY_TYPE[
      StructType.NODE_DEFINITION_REFERENCE
    ] as typeof NodeDefinitionReference;
    const _Expression = STRUCT_CLASS_BY_TYPE[StructType.EXPRESSION] as typeof Expression;
    const _Join = STRUCT_CLASS_BY_TYPE[StructType.JOIN] as typeof Join;
    const _Aggregation = STRUCT_CLASS_BY_TYPE[StructType.AGGREGATION] as typeof Aggregation;
    const _Condition = STRUCT_CLASS_BY_TYPE[StructType.CONDITION] as typeof Condition;
    const _Sort = STRUCT_CLASS_BY_TYPE[StructType.SORT] as typeof Sort;
    const _Select = STRUCT_CLASS_BY_TYPE[StructType.SELECT] as typeof Select;
    const _Query = STRUCT_CLASS_BY_TYPE[StructType.QUERY] as typeof Query;
    const unpackedSubqueries: any[] = [];
    if (objectCson["109"] != undefined) {
      for (const item of objectCson["109"]) {
        unpackedSubqueries.push(_Query.fromCson(item, _session, _graph, _connection));
      }
    }
    const joinValue = objectCson["110"];
    const unpackedJoin =
      joinValue != undefined ? _Join.fromCson(joinValue, _session, _graph, _connection) : null;
    const selectValue = objectCson["111"];
    const unpackedSelect =
      selectValue != undefined
        ? _Select.fromCson(selectValue, _session, _graph, _connection)
        : null;
    const whereValue = objectCson["112"];
    const unpackedWhere =
      whereValue != undefined
        ? _Condition.fromCson(whereValue, _session, _graph, _connection)
        : null;
    const havingValue = objectCson["113"];
    const unpackedHaving =
      havingValue != undefined
        ? _Condition.fromCson(havingValue, _session, _graph, _connection)
        : null;
    const unpackedGroupBy: any[] = [];
    if (objectCson["114"] != undefined) {
      for (const item of objectCson["114"]) {
        unpackedGroupBy.push(_Expression.fromCson(item, _session, _graph, _connection));
      }
    }
    const aggregationValue = objectCson["115"];
    const unpackedAggregation =
      aggregationValue != undefined
        ? _Aggregation.fromCson(aggregationValue, _session, _graph, _connection)
        : null;
    const unpackedSort: any[] = [];
    if (objectCson["116"] != undefined) {
      for (const item of objectCson["116"]) {
        unpackedSort.push(_Sort.fromCson(item, _session, _graph, _connection));
      }
    }
    const limitValue = objectCson["120"];
    const unpackedLimit = limitValue != undefined ? Number(limitValue) : null;
    const offsetValue = objectCson["121"];
    const unpackedOffset = offsetValue != undefined ? Number(offsetValue) : null;
    return new Query({
      id: String(objectCson["2"]),
      type: Number(objectCson["100"]),
      domain: Number(objectCson["101"]),
      name: objectCson["105"],
      definition: _NodeDefinitionReference.fromCson(
        objectCson["106"],
        _session,
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
      _cson: objectCson,
      _graph,
    });
  }

  static fromCson(
    objectCson: { readonly [key: string]: any },
    _session?: Session | null,
    _graph?: Graph | null,
    _connection?: GraphConnection | null,
  ): Query {
    return Query.__unpackCson__(objectCson, _session, _graph, _connection);
  }

  toProto(): QueryProto {
    if (this._proto === null) {
      // @ts-expect-error(readonly)
      this._proto = Query.__packProto__(this);
    }
    return this._proto as QueryProto;
  }

  static __packProto__(object: Query): QueryProto {
    const objectProto: Partial<QueryProto> = { metatype: 300 };
    objectProto.id = String(object.id);
    objectProto.type = Number(object.type) as QueryTypeProto;
    objectProto.domain = Number(object.domain) as StoreDomainProto;
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
    return objectProto as QueryProto;
  }

  static __unpackProto__(
    objectProto: QueryProto,
    _session?: Session | null,
    _graph?: Supergraph | null,
    _connection?: GraphConnection | null,
  ): Query {
    const _NodeDefinitionReference = STRUCT_CLASS_BY_TYPE[
      StructType.NODE_DEFINITION_REFERENCE
    ] as typeof NodeDefinitionReference;
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
        unpackedSubqueries.push(_Query.fromProto(item!, _session, _graph, _graph, _connection));
      }
    }
    const unpackedGroupBy: any[] = [];
    if (objectProto.groupBy) {
      for (const item of objectProto.groupBy) {
        unpackedGroupBy.push(_Expression.fromProto(item!, _session, _graph, _graph, _connection));
      }
    }
    const unpackedSort: any[] = [];
    if (objectProto.sort) {
      for (const item of objectProto.sort) {
        unpackedSort.push(_Sort.fromProto(item!, _session, _graph, _graph, _connection));
      }
    }
    return new Query({
      id: String(objectProto.id),
      type: Number(objectProto.type) as QueryType,
      domain: Number(objectProto.domain) as StoreDomain,
      name: objectProto.name,
      definition: _NodeDefinitionReference.fromProto(
        objectProto.definition!,
        _session,
        _graph,
        _graph,
        _connection,
      ),
      subqueries: unpackedSubqueries,
      join:
        objectProto.join != undefined
          ? _Join.fromProto(objectProto.join!, _session, _graph, _graph, _connection)
          : null,
      select:
        objectProto.select != undefined
          ? _Select.fromProto(objectProto.select!, _session, _graph, _graph, _connection)
          : null,
      where:
        objectProto.where != undefined
          ? _Condition.fromProto(objectProto.where!, _session, _graph, _graph, _connection)
          : null,
      having:
        objectProto.having != undefined
          ? _Condition.fromProto(objectProto.having!, _session, _graph, _graph, _connection)
          : null,
      groupBy: unpackedGroupBy,
      aggregation:
        objectProto.aggregation != undefined
          ? _Aggregation.fromProto(objectProto.aggregation!, _session, _graph, _graph, _connection)
          : null,
      sort: unpackedSort,
      limit: objectProto.limit != undefined ? Number(objectProto.limit) : null,
      offset: objectProto.offset != undefined ? Number(objectProto.offset) : null,
      _proto: objectProto,
      _graph,
    });
  }

  static fromProto(
    objectProto: QueryProto,
    _session?: Session | null,
    _graph?: Supergraph | null,
    _connection?: GraphConnection | null,
  ): Query {
    return Query.__unpackProto__(objectProto, _session, _graph, _connection);
  }

  static fromProtoString(packedProtoString: string): Query {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = QueryProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */

  /** Execute the Query. */
  async execute(options?: { isLive?: boolean }): Promise<QueryConnection<T>> {
    const session = activeSession();
    const store = session.store;
    if (store == null) {
      throw new Error(`no store in ${session.repr()}`);
    }
    const connection = new QueryConnection<T>({
      query: this,
      store,
      session,
      isLive: options?.isLive ?? false,
    });
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
/* ==== DESTACK_GENERATED_END:STRUCT:300 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:305 ==== */
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
/* ==== DESTACK_GENERATED_END:ENUM:305 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:300 ==== */
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
/* ==== DESTACK_GENERATED_END:ENUM:300 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:301 ==== */
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
/* ==== DESTACK_GENERATED_END:ENUM:301 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:306 ==== */
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
/* ==== DESTACK_GENERATED_END:ENUM:306 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:303 ==== */
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
/* ==== DESTACK_GENERATED_END:ENUM:303 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:302 ==== */
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
/* ==== DESTACK_GENERATED_END:ENUM:302 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:304 ==== */
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
/* ==== DESTACK_GENERATED_END:ENUM:304 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:320 ==== */
/**
 * QueryType
 */
export enum QueryType {
  NODE = 1,
  SCALAR = 5,
  GROUPED_NODE = 10,
  GROUPED_SCALAR = 15,

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerEnumClass(EnumType.QUERY_TYPE, QueryType);
/* ==== DESTACK_GENERATED_END:ENUM:320 ==== */
