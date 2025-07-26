import { EnumType, NodeType, StructType } from "@destack/language/core/builtin/builtin";
import { GraphDomain, TypeCardinality } from "@destack/language/core/builtin/common";
import type { PropertyDefinition } from "@destack/language/core/builtin/definition";
import type { NodeClass } from "@destack/language/core/builtin/node";
import { isNode, type Node } from "@destack/language/core/builtin/node";
import type { PackedObjectCache } from "@destack/language/core/builtin/object";
import type {
  NodeDefinitionReference,
  NodeReference,
  PropertyReference,
} from "@destack/language/core/builtin/relation";
import { isStruct, StructFrozen } from "@destack/language/core/builtin/struct";
import { Type } from "@destack/language/core/builtin/type";
import type { UInt32, UUID } from "@destack/language/core/builtin/types";
import type { Value } from "@destack/language/core/builtin/value";
import { toValue } from "@destack/language/core/builtin/value";
import type { CustomProperty } from "@destack/language/core/common/property";
import type { Session } from "@destack/language/core/runtime/session";
import {
  registerEnumClass,
  registerStructClass,
  STRUCT_CLASS_BY_TYPE,
} from "@destack/language/registry";
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
    _hash?: number | null;
    _repr?: string | null;
    _PackedObjectCache?: PackedObjectCache[] | null;
  }) {
    /* super */
    super(
      /* session */
      options._session ?? null,
    );

    /* properties */
    let _type = options.type;
    if (_type == null) {
      throw new Error(`Function.type is required`);
    }
    this.type = _type;
    let _left = options.left;
    if (_left == null) {
      throw new Error(`Function.left is required`);
    }
    this.left = _left;
    let _right = options.right ?? null;
    this.right = _right;

    /* identity */
    // @ts-expect-error(readonly)
    this._hash = options._hash ?? null;
    // @ts-expect-error(readonly)
    this._repr = options._repr ?? null;
    // @ts-expect-error(readonly)
    this._PackedObjectCache = options._PackedObjectCache ?? null;
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
      // @ts-expect-error(readonly) */
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
    _hash?: number | null;
    _repr?: string | null;
    _PackedObjectCache?: PackedObjectCache[] | null;
  }) {
    /* super */
    super(
      /* session */
      options._session ?? null,
    );

    /* properties */
    let _type = options.type;
    if (_type == null) {
      throw new Error(`Condition.type is required`);
    }
    this.type = _type;
    let _left = options.left;
    if (_left == null) {
      throw new Error(`Condition.left is required`);
    }
    this.left = _left;
    let _right = options.right ?? null;
    this.right = _right;

    /* identity */
    // @ts-expect-error(readonly)
    this._hash = options._hash ?? null;
    // @ts-expect-error(readonly)
    this._repr = options._repr ?? null;
    // @ts-expect-error(readonly)
    this._PackedObjectCache = options._PackedObjectCache ?? null;
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
      // @ts-expect-error(readonly) */
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
    _hash?: number | null;
    _repr?: string | null;
    _PackedObjectCache?: PackedObjectCache[] | null;
  }) {
    /* super */
    super(
      /* session */
      options._session ?? null,
    );

    /* properties */
    let _type = options.type;
    if (_type == null) {
      throw new Error(`Aggregation.type is required`);
    }
    this.type = _type;
    let _expression = options.expression ?? null;
    this.expression = _expression;

    /* identity */
    // @ts-expect-error(readonly)
    this._hash = options._hash ?? null;
    // @ts-expect-error(readonly)
    this._repr = options._repr ?? null;
    // @ts-expect-error(readonly)
    this._PackedObjectCache = options._PackedObjectCache ?? null;
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
      // @ts-expect-error(readonly) */
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
    _hash?: number | null;
    _repr?: string | null;
    _PackedObjectCache?: PackedObjectCache[] | null;
  }) {
    /* super */
    super(
      /* session */
      options._session ?? null,
    );

    /* properties */
    let _type = options.type;
    if (_type == null) {
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

    /* identity */
    // @ts-expect-error(readonly)
    this._hash = options._hash ?? null;
    // @ts-expect-error(readonly)
    this._repr = options._repr ?? null;
    // @ts-expect-error(readonly)
    this._PackedObjectCache = options._PackedObjectCache ?? null;
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
      // @ts-expect-error(readonly) */
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
    _hash?: number | null;
    _repr?: string | null;
    _PackedObjectCache?: PackedObjectCache[] | null;
  }) {
    /* super */
    super(
      /* session */
      options._session ?? null,
    );

    /* properties */
    let _type = options.type;
    if (_type == null) {
      throw new Error(`Sort.type is required`);
    }
    this.type = _type;
    let _by = options.by;
    if (_by == null) {
      throw new Error(`Sort.by is required`);
    }
    this.by = _by;
    let _mode = options.mode ?? null;
    this.mode = _mode;

    /* identity */
    // @ts-expect-error(readonly)
    this._hash = options._hash ?? null;
    // @ts-expect-error(readonly)
    this._repr = options._repr ?? null;
    // @ts-expect-error(readonly)
    this._PackedObjectCache = options._PackedObjectCache ?? null;
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
      // @ts-expect-error(readonly) */
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
    _hash?: number | null;
    _repr?: string | null;
    _PackedObjectCache?: PackedObjectCache[] | null;
  }) {
    /* super */
    super(
      /* session */
      options._session ?? null,
    );

    /* properties */
    let _attributes = options.attributes ?? null;
    if (_attributes == null) {
      _attributes = [];
    }
    this.attributes = _attributes;

    /* identity */
    // @ts-expect-error(readonly)
    this._hash = options._hash ?? null;
    // @ts-expect-error(readonly)
    this._repr = options._repr ?? null;
    // @ts-expect-error(readonly)
    this._PackedObjectCache = options._PackedObjectCache ?? null;
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
        // @ts-expect-error(readonly) */
        this._repr = `<Select ${propertyReprs.join(" ")}>`;
      } else {
        // @ts-expect-error(readonly) */
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
   * Join.recursive
   */
  readonly recursive: boolean;

  /**
   * Join.on
   */
  readonly on: Condition | null;

  constructor(options: {
    type: JoinType;
    recursive?: boolean;
    on?: Condition | null;
    _session?: Session | null;
    _hash?: number | null;
    _repr?: string | null;
    _PackedObjectCache?: PackedObjectCache[] | null;
  }) {
    /* super */
    super(
      /* session */
      options._session ?? null,
    );

    /* properties */
    let _type = options.type;
    if (_type == null) {
      throw new Error(`Join.type is required`);
    }
    this.type = _type;
    let _recursive = options.recursive ?? null;
    if (_recursive == null) {
      _recursive = false;
    }
    if (_recursive == null) {
      throw new Error(`Join.recursive is required`);
    }
    this.recursive = _recursive;
    let _on = options.on ?? null;
    this.on = _on;

    /* identity */
    // @ts-expect-error(readonly)
    this._hash = options._hash ?? null;
    // @ts-expect-error(readonly)
    this._repr = options._repr ?? null;
    // @ts-expect-error(readonly)
    this._PackedObjectCache = options._PackedObjectCache ?? null;
  }

  equals(other: any): boolean {
    if (!(this.metatype === other.metatype)) {
      return false;
    }
    if (!(this.type === other.type)) {
      return false;
    }
    if (!(this.recursive === other.recursive)) {
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
      propertyReprs.push(`recursive=${this.recursive}`);
      if (this.on != null) {
        propertyReprs.push(`on=${this.on.repr()}`);
      }
      // @ts-expect-error(readonly) */
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
    h = (h * 31 + hashBool(this.recursive)) & 0xffffffff;
    if (this.on != null) {
      h = (h * 31 + this.on.hash()) & 0xffffffff;
    }
    // @ts-expect-error(readonly)
    this._hash = h;
    return h;
  }

  /* ==== DESTACK_CUSTOM_START ==== */

  /** Make a Join from a shorthand expression. */
  static of(
    joinType: JoinType | Join,
    options?: {
      definition?: NodeType | NodeClass | NodeReference;
      recursive?: boolean;
      on?: Condition | null;
    },
  ): Join {
    if (isStruct(joinType, StructType.JOIN)) {
      return joinType;
    }
    return new Join({
      type: joinType,
      recursive: options?.recursive ?? false,
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
  readonly id: UUID;

  /**
   * The type of Query.
   */
  readonly type: QueryType;

  /**
   * The domain of the Query (Entity or Event).
   */
  readonly domain: GraphDomain;

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
  readonly limit: UInt32 | null;

  /**
   * Offset the results.
   */
  readonly offset: UInt32 | null;

  constructor(options: {
    id?: UUID;
    type: QueryType;
    domain: GraphDomain;
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
    limit?: UInt32 | null;
    offset?: UInt32 | null;
    _session?: Session | null;
    _hash?: number | null;
    _repr?: string | null;
    _PackedObjectCache?: PackedObjectCache[] | null;
  }) {
    /* super */
    super(
      /* session */
      options._session ?? null,
    );

    /* properties */
    let _id = options.id ?? null;
    if (_id == null) {
      _id = uuid4();
    }
    if (_id == null) {
      throw new Error(`Query.id is required`);
    }
    this.id = _id;
    let _type = options.type;
    if (_type == null) {
      throw new Error(`Query.type is required`);
    }
    this.type = _type;
    let _domain = options.domain;
    if (_domain == null) {
      throw new Error(`Query.domain is required`);
    }
    this.domain = _domain;
    let _name = options.name;
    if (_name == null) {
      throw new Error(`Query.name is required`);
    }
    this.name = _name;
    let _definition = options.definition;
    if (_definition == null) {
      throw new Error(`Query.definition is required`);
    }
    this.definition = _definition;
    let _subqueries = options.subqueries ?? null;
    if (_subqueries == null) {
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
    if (_groupBy == null) {
      _groupBy = [];
    }
    this.groupBy = _groupBy;
    let _aggregation = options.aggregation ?? null;
    this.aggregation = _aggregation;
    let _sort = options.sort ?? null;
    if (_sort == null) {
      _sort = [];
    }
    this.sort = _sort;
    let _limit = options.limit ?? null;
    this.limit = _limit;
    let _offset = options.offset ?? null;
    this.offset = _offset;

    /* identity */
    // @ts-expect-error(readonly)
    this._hash = options._hash ?? null;
    // @ts-expect-error(readonly)
    this._repr = options._repr ?? null;
    // @ts-expect-error(readonly)
    this._PackedObjectCache = options._PackedObjectCache ?? null;
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
      propertyReprs.push(`domain=${GraphDomain[this.domain]}`);
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
      // @ts-expect-error(readonly) */
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

  /* ==== DESTACK_CUSTOM_START ==== */
  /* ... */
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
