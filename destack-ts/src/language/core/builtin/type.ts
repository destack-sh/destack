import { EnumType, NodeType, StructType } from "@destack/language/core/builtin/builtin";
import {
  PRIMITIVE_TYPE_BY_JS_TYPE_NAME,
  PrimitiveType,
  ScalarType,
  TypeCardinality,
  type ValueFactory,
} from "@destack/language/core/builtin/common";
import { isNode } from "@destack/language/core/builtin/node";
import type { PackedObjectCache } from "@destack/language/core/builtin/object";
import { isStruct, StructFrozen } from "@destack/language/core/builtin/struct";
import type { Float32, UInt32 } from "@destack/language/core/builtin/types";
import type { Value } from "@destack/language/core/builtin/value";
import type { Session } from "@destack/language/core/runtime/session";
import { registerStructClass } from "@destack/language/registry";
import { hashBool, hashFloat, hashInt, hashString } from "@destack/utils/hash";

/**
 * Guess the type of a value or class.
 */
export function toType(valueOrType: any, options?: { nodeAsValue: boolean }): Type {
  if (valueOrType === null || valueOrType === undefined) {
    throw new Error("null/undefined is not a valid Type");
  }

  // scalar values
  if (isStruct(valueOrType, StructType.NODE_REFERENCE)) {
    return new Type({
      cardinality: TypeCardinality.SCALAR,
      scalarType: ScalarType.NODE_REFERENCE,
      nodeTypes: [valueOrType.type],
    });
  } else if (isNode(valueOrType)) {
    return new Type({
      cardinality: TypeCardinality.SCALAR,
      scalarType: options?.nodeAsValue ? ScalarType.NODE_VALUE : ScalarType.NODE_REFERENCE,
      nodeTypes: [valueOrType.metatype],
    });
  } else if (isStruct(valueOrType)) {
    return new Type({
      cardinality: TypeCardinality.SCALAR,
      scalarType: ScalarType.STRUCT,
      structType: valueOrType.metatype,
    });
  } else if (
    PRIMITIVE_TYPE_BY_JS_TYPE_NAME.has(valueOrType.constructor.name) &&
    valueOrType.constructor !== Object
  ) {
    return new Type({
      cardinality: TypeCardinality.SCALAR,
      scalarType: ScalarType.PRIMITIVE,
      primitiveType: PRIMITIVE_TYPE_BY_JS_TYPE_NAME.get(valueOrType.constructor.name) || null,
    });
  }

  // collections
  if (Array.isArray(valueOrType)) {
    if (valueOrType.length === 0) {
      throw new Error(`cannot infer type of empty array: ${valueOrType}`);
    }
    const elementType = toType(valueOrType[0], options);
    if (elementType.cardinality !== TypeCardinality.SCALAR) {
      throw new Error(
        `expected scalar inside array, got ${elementType.cardinality} for ${valueOrType}`,
      );
    }
    return new Type({
      cardinality: TypeCardinality.LIST,
      scalarType: elementType.scalarType,
      primitiveType: elementType.primitiveType,
      enumType: elementType.enumType,
      nodeTypes: elementType.nodeTypes,
      structType: elementType.structType,
    });
  } else if (valueOrType instanceof Map) {
    if (valueOrType.size === 0) {
      throw new Error(`cannot infer type of empty Map: ${valueOrType}`);
    }
    const [sampleKey, sampleValue] = Array.from(valueOrType.entries()).at(0)!;
    const keyType = toType(sampleKey, options);
    if (keyType.cardinality !== TypeCardinality.SCALAR) {
      throw new Error(`expected scalar key in Map, got ${keyType.cardinality} for ${valueOrType}`);
    }
    const valueType = toType(sampleValue, options);
    if (
      valueType.cardinality !== TypeCardinality.SCALAR &&
      valueType.cardinality !== TypeCardinality.LIST
    ) {
      throw new Error(
        `expected scalar or list value in Map, got ${valueType.cardinality} for ${valueOrType}`,
      );
    }
    return new Type({
      cardinality: TypeCardinality.MAP,
      scalarType: valueType.scalarType,
      primitiveType: valueType.primitiveType,
      enumType: valueType.enumType,
      nodeTypes: valueType.nodeTypes,
      structType: valueType.structType,
      keyType: keyType,
    });
  } else if (typeof valueOrType === "object" && valueOrType.constructor === Object) {
    const keys = Object.keys(valueOrType);
    if (keys.length === 0) {
      throw new Error(`cannot infer type of empty object: ${valueOrType}`);
    }
    const sampleKey = keys[0];
    const sampleValue = valueOrType[sampleKey];
    const keyType = toType(sampleKey, options);
    if (keyType.cardinality !== TypeCardinality.SCALAR) {
      throw new Error(
        `expected scalar key in object, got ${keyType.cardinality} for ${valueOrType}`,
      );
    }
    const valueType = toType(sampleValue, options);
    if (
      valueType.cardinality !== TypeCardinality.SCALAR &&
      valueType.cardinality !== TypeCardinality.LIST
    ) {
      throw new Error(
        `expected scalar or list value in object, got ${valueType.cardinality} for ${valueOrType}`,
      );
    }
    return new Type({
      cardinality: TypeCardinality.MAP,
      scalarType: valueType.scalarType,
      primitiveType: valueType.primitiveType,
      enumType: valueType.enumType,
      nodeTypes: valueType.nodeTypes,
      structType: valueType.structType,
      keyType: keyType,
    });
  }

  throw new Error(`cannot infer type of ${valueOrType}`);
}

/* ==== DESTACK_GENERATED_START:STRUCT:111 ==== */
/**
 * The constraint of a string.
 */
export class StringConstraint extends StructFrozen {
  static metatype: StructType = StructType.STRING_CONSTRAINT;
  static __isFrozen__: boolean = true;

  /**
   * StringConstraint.regex
   */
  readonly regex: string | null;

  /**
   * StringConstraint.startsWith
   */
  readonly startsWith: string | null;

  /**
   * StringConstraint.endsWith
   */
  readonly endsWith: string | null;

  constructor(options: {
    regex?: string | null;
    startsWith?: string | null;
    endsWith?: string | null;
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
    let _regex = options.regex ?? null;
    this.regex = _regex;
    let _startsWith = options.startsWith ?? null;
    this.startsWith = _startsWith;
    let _endsWith = options.endsWith ?? null;
    this.endsWith = _endsWith;

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
    if (!(this.regex === other.regex)) {
      return false;
    }
    if (!(this.startsWith === other.startsWith)) {
      return false;
    }
    if (!(this.endsWith === other.endsWith)) {
      return false;
    }
    return true;
  }

  repr(): string {
    return `<StringConstraint>`;
  }

  hash(): number {
    if (this._hash != null) {
      return this._hash;
    }
    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    if (this.regex != null) {
      h = (h * 31 + hashString(this.regex)) & 0xffffffff;
    }
    if (this.startsWith != null) {
      h = (h * 31 + hashString(this.startsWith)) & 0xffffffff;
    }
    if (this.endsWith != null) {
      h = (h * 31 + hashString(this.endsWith)) & 0xffffffff;
    }
    // @ts-expect-error(readonly)
    this._hash = h;
    return h;
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerStructClass(StructType.STRING_CONSTRAINT, StringConstraint);
/* ==== DESTACK_GENERATED_END:STRUCT:111 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:110 ==== */
/**
 * The constraint of a number.
 */
export class NumberConstraint extends StructFrozen {
  static metatype: StructType = StructType.NUMBER_CONSTRAINT;
  static __isFrozen__: boolean = true;

  /**
   * NumberConstraint.minValue
   */
  readonly minValue: Float32 | null;

  /**
   * NumberConstraint.maxValue
   */
  readonly maxValue: Float32 | null;

  /**
   * NumberConstraint.stepValue
   */
  readonly stepValue: Float32 | null;

  constructor(options: {
    minValue?: Float32 | null;
    maxValue?: Float32 | null;
    stepValue?: Float32 | null;
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
    let _minValue = options.minValue ?? null;
    this.minValue = _minValue;
    let _maxValue = options.maxValue ?? null;
    this.maxValue = _maxValue;
    let _stepValue = options.stepValue ?? null;
    this.stepValue = _stepValue;

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
    if (
      (this.minValue == null) !== (other.minValue == null) ||
      (this.minValue != null &&
        !(this.minValue === other.minValue || Math.abs(this.minValue - other.minValue) < 1e-10))
    ) {
      return false;
    }
    if (
      (this.maxValue == null) !== (other.maxValue == null) ||
      (this.maxValue != null &&
        !(this.maxValue === other.maxValue || Math.abs(this.maxValue - other.maxValue) < 1e-10))
    ) {
      return false;
    }
    if (
      (this.stepValue == null) !== (other.stepValue == null) ||
      (this.stepValue != null &&
        !(this.stepValue === other.stepValue || Math.abs(this.stepValue - other.stepValue) < 1e-10))
    ) {
      return false;
    }
    return true;
  }

  repr(): string {
    return `<NumberConstraint>`;
  }

  hash(): number {
    if (this._hash != null) {
      return this._hash;
    }
    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    if (this.minValue != null) {
      h = (h * 31 + hashFloat(this.minValue)) & 0xffffffff;
    }
    if (this.maxValue != null) {
      h = (h * 31 + hashFloat(this.maxValue)) & 0xffffffff;
    }
    if (this.stepValue != null) {
      h = (h * 31 + hashFloat(this.stepValue)) & 0xffffffff;
    }
    // @ts-expect-error(readonly)
    this._hash = h;
    return h;
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerStructClass(StructType.NUMBER_CONSTRAINT, NumberConstraint);
/* ==== DESTACK_GENERATED_END:STRUCT:110 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:112 ==== */
/**
 * The constraint of a collection.
 */
export class CollectionConstraint extends StructFrozen {
  static metatype: StructType = StructType.COLLECTION_CONSTRAINT;
  static __isFrozen__: boolean = true;

  /**
   * CollectionConstraint.minLength
   */
  readonly minLength: UInt32 | null;

  /**
   * CollectionConstraint.maxLength
   */
  readonly maxLength: UInt32 | null;

  constructor(options: {
    minLength?: UInt32 | null;
    maxLength?: UInt32 | null;
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
    let _minLength = options.minLength ?? null;
    this.minLength = _minLength;
    let _maxLength = options.maxLength ?? null;
    this.maxLength = _maxLength;

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
    if (!(this.minLength === other.minLength)) {
      return false;
    }
    if (!(this.maxLength === other.maxLength)) {
      return false;
    }
    return true;
  }

  repr(): string {
    return `<CollectionConstraint>`;
  }

  hash(): number {
    if (this._hash != null) {
      return this._hash;
    }
    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    if (this.minLength != null) {
      h = (h * 31 + hashInt(this.minLength)) & 0xffffffff;
    }
    if (this.maxLength != null) {
      h = (h * 31 + hashInt(this.maxLength)) & 0xffffffff;
    }
    // @ts-expect-error(readonly)
    this._hash = h;
    return h;
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerStructClass(StructType.COLLECTION_CONSTRAINT, CollectionConstraint);
/* ==== DESTACK_GENERATED_END:STRUCT:112 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:101 ==== */
/**
 * A basic Type in the type system. Types compose like a tree (with scalars at the leaves):
 *  - Scalar: a single value (primitive, enum, node, struct, etc.)
 *  - List: a dynamic-length sequence of homogeneous values
 *  - Tuple: a fixed-length sequence of heterogeneous values
 *  - Map: a mapping of homogenous keys to homogeneous values
 *  - Literal: a constant value
 *  - Union: a tagged union of heterogeneous values (declaration only)
 */
export class Type extends StructFrozen {
  static metatype: StructType = StructType.TYPE;
  static __isFrozen__: boolean = true;

  /**
   * Cardinality of this Type (scalar, list, map, etc.)
   */
  readonly cardinality: TypeCardinality;

  /**
   * Key type of this Type (if it's a map).
   */
  readonly keyType: Type | null;

  /**
   * Value type of this Type (if it's a list, map, etc.).
   */
  readonly valueType: Type | null;

  /**
   * Element types of this Type (if it's a tuple).
   */
  readonly elementTypes: readonly Type[] | null;

  /**
   * Scalar value type of this Type (primitive, enum, node, struct, etc..).
   */
  readonly scalarType: ScalarType | null;

  /**
   * Primitive type of this Type (if it's a primitive scalar).
   */
  readonly primitiveType: PrimitiveType | null;

  /**
   * Enum type of this Type (if it's an enum scalar).
   */
  readonly enumType: EnumType | null;

  /**
   * Node types of this Type (if it's a node reference or node value scalar).
   */
  readonly nodeTypes: readonly NodeType[] | null;

  /**
   * Struct type of this Type (if it's a struct scalar).
   */
  readonly structType: StructType | null;

  /**
   * Literal value of this Type (if it's a literal scalar).
   */
  readonly literalValue: Value | null;

  /**
   * Union types of this Type (if it's a union scalar).
   */
  readonly unionTypes: readonly Type[] | null;

  /**
   * Type.isRequired
   */
  readonly isRequired: boolean;

  constructor(options: {
    cardinality?: TypeCardinality;
    keyType?: Type | null;
    valueType?: Type | null;
    elementTypes?: readonly Type[] | null;
    scalarType?: ScalarType | null;
    primitiveType?: PrimitiveType | null;
    enumType?: EnumType | null;
    nodeTypes?: readonly NodeType[] | null;
    structType?: StructType | null;
    literalValue?: Value | null;
    unionTypes?: readonly Type[] | null;
    isRequired?: boolean;
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
    let _cardinality = options.cardinality ?? null;
    if (_cardinality == null) {
      _cardinality = 1 /* TypeCardinality.SCALAR */;
    }
    if (_cardinality == null) {
      throw new Error(`Type.cardinality is required`);
    }
    this.cardinality = _cardinality;
    let _keyType = options.keyType ?? null;
    this.keyType = _keyType;
    let _valueType = options.valueType ?? null;
    this.valueType = _valueType;
    let _elementTypes = options.elementTypes ?? null;
    this.elementTypes = _elementTypes;
    let _scalarType = options.scalarType ?? null;
    this.scalarType = _scalarType;
    let _primitiveType = options.primitiveType ?? null;
    this.primitiveType = _primitiveType;
    let _enumType = options.enumType ?? null;
    this.enumType = _enumType;
    let _nodeTypes = options.nodeTypes ?? null;
    this.nodeTypes = _nodeTypes;
    let _structType = options.structType ?? null;
    this.structType = _structType;
    let _literalValue = options.literalValue ?? null;
    this.literalValue = _literalValue;
    let _unionTypes = options.unionTypes ?? null;
    this.unionTypes = _unionTypes;
    let _isRequired = options.isRequired ?? null;
    if (_isRequired == null) {
      _isRequired = true;
    }
    if (_isRequired == null) {
      throw new Error(`Type.isRequired is required`);
    }
    this.isRequired = _isRequired;

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
    if (!(this.cardinality === other.cardinality)) {
      return false;
    }
    if (
      (this.keyType == null) !== (other.keyType == null) ||
      (this.keyType != null && !this.keyType.equals(other.keyType))
    ) {
      return false;
    }
    if (
      (this.valueType == null) !== (other.valueType == null) ||
      (this.valueType != null && !this.valueType.equals(other.valueType))
    ) {
      return false;
    }
    if (this.elementTypes == null) {
      return other.elementTypes == null;
    }
    if (this.elementTypes.length != other.elementTypes.length) {
      return false;
    }
    for (let i = 0; i < this.elementTypes.length; i++) {
      if (!this.elementTypes[i].equals(other.elementTypes[i])) {
        return false;
      }
    }

    if (!(this.scalarType === other.scalarType)) {
      return false;
    }
    if (!(this.primitiveType === other.primitiveType)) {
      return false;
    }
    if (!(this.enumType === other.enumType)) {
      return false;
    }
    if (this.nodeTypes == null) {
      return other.nodeTypes == null;
    }
    if (this.nodeTypes.length != other.nodeTypes.length) {
      return false;
    }
    for (let i = 0; i < this.nodeTypes.length; i++) {
      if (!(this.nodeTypes[i] === other.nodeTypes[i])) {
        return false;
      }
    }

    if (!(this.structType === other.structType)) {
      return false;
    }
    if (
      (this.literalValue == null) !== (other.literalValue == null) ||
      (this.literalValue != null && !this.literalValue.equals(other.literalValue))
    ) {
      return false;
    }
    if (this.unionTypes == null) {
      return other.unionTypes == null;
    }
    if (this.unionTypes.length != other.unionTypes.length) {
      return false;
    }
    for (let i = 0; i < this.unionTypes.length; i++) {
      if (!this.unionTypes[i].equals(other.unionTypes[i])) {
        return false;
      }
    }

    if (!(this.isRequired === other.isRequired)) {
      return false;
    }
    return true;
  }

  repr(): string {
    if (this._repr === null) {
      const propertyReprs: string[] = [];
      propertyReprs.push(`cardinality=${TypeCardinality[this.cardinality]}`);
      if (this.keyType != null) {
        propertyReprs.push(`keyType=${this.keyType.repr()}`);
      }
      if (this.valueType != null) {
        propertyReprs.push(`valueType=${this.valueType.repr()}`);
      }
      if (this.elementTypes != null && this.elementTypes.length > 0) {
        propertyReprs.push(
          `elementTypes=${this.elementTypes.map((_item) => _item.repr()).join(", ")}`,
        );
      }
      if (this.scalarType != null) {
        propertyReprs.push(`scalarType=${ScalarType[this.scalarType]}`);
      }
      if (this.primitiveType != null) {
        propertyReprs.push(`primitiveType=${PrimitiveType[this.primitiveType]}`);
      }
      if (this.enumType != null) {
        propertyReprs.push(`enumType=${EnumType[this.enumType]}`);
      }
      if (this.nodeTypes != null && this.nodeTypes.length > 0) {
        propertyReprs.push(
          `nodeTypes=${this.nodeTypes.map((_item) => NodeType[_item]).join(", ")}`,
        );
      }
      if (this.structType != null) {
        propertyReprs.push(`structType=${StructType[this.structType]}`);
      }
      if (this.literalValue != null) {
        propertyReprs.push(`literalValue=${this.literalValue.repr()}`);
      }
      if (this.unionTypes != null && this.unionTypes.length > 0) {
        propertyReprs.push(`unionTypes=${this.unionTypes.map((_item) => _item.repr()).join(", ")}`);
      }
      // @ts-expect-error(readonly) */
      this._repr = `<Type ${propertyReprs.join(" ")}>`;
    }
    return this._repr;
  }

  hash(): number {
    if (this._hash != null) {
      return this._hash;
    }
    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    h = (h * 31 + this.cardinality) & 0xffffffff;
    if (this.keyType != null) {
      h = (h * 31 + this.keyType.hash()) & 0xffffffff;
    }
    if (this.valueType != null) {
      h = (h * 31 + this.valueType.hash()) & 0xffffffff;
    }
    if (this.elementTypes && this.elementTypes.length > 0) {
      for (const _item of this.elementTypes) {
        h = (h * 31 + _item.hash()) & 0xffffffff;
      }
    }
    if (this.scalarType != null) {
      h = (h * 31 + this.scalarType) & 0xffffffff;
    }
    if (this.primitiveType != null) {
      h = (h * 31 + this.primitiveType) & 0xffffffff;
    }
    if (this.enumType != null) {
      h = (h * 31 + this.enumType) & 0xffffffff;
    }
    if (this.nodeTypes && this.nodeTypes.length > 0) {
      for (const _item of this.nodeTypes) {
        h = (h * 31 + _item) & 0xffffffff;
      }
    }
    if (this.structType != null) {
      h = (h * 31 + this.structType) & 0xffffffff;
    }
    if (this.literalValue != null) {
      h = (h * 31 + this.literalValue.hash()) & 0xffffffff;
    }
    if (this.unionTypes && this.unionTypes.length > 0) {
      for (const _item of this.unionTypes) {
        h = (h * 31 + _item.hash()) & 0xffffffff;
      }
    }
    h = (h * 31 + hashBool(this.isRequired)) & 0xffffffff;
    // @ts-expect-error(readonly)
    this._hash = h;
    return h;
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerStructClass(StructType.TYPE, Type);
/* ==== DESTACK_GENERATED_END:STRUCT:101 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:102 ==== */
/**
 * A full Type in the type system.
 * Extends Type with defaults, constraints, and supporting flags.
 */
export class CheckedType extends Type {
  static metatype: StructType = StructType.CHECKED_TYPE;
  static __isFrozen__: boolean = true;

  /**
   * CheckedType.defaultValue
   */
  readonly defaultValue: Value | null;

  /**
   * CheckedType.defaultFactory
   */
  readonly defaultFactory: ValueFactory | null;

  /**
   * CheckedType.collectionConstraint
   */
  readonly collectionConstraint: CollectionConstraint | null;

  /**
   * CheckedType.stringConstraint
   */
  readonly stringConstraint: StringConstraint | null;

  /**
   * CheckedType.numberConstraint
   */
  readonly numberConstraint: NumberConstraint | null;

  constructor(options: {
    cardinality?: TypeCardinality;
    keyType?: Type | null;
    valueType?: Type | null;
    elementTypes?: readonly Type[] | null;
    scalarType?: ScalarType | null;
    primitiveType?: PrimitiveType | null;
    enumType?: EnumType | null;
    nodeTypes?: readonly NodeType[] | null;
    structType?: StructType | null;
    literalValue?: Value | null;
    unionTypes?: readonly Type[] | null;
    isRequired?: boolean;
    defaultValue?: Value | null;
    defaultFactory?: ValueFactory | null;
    collectionConstraint?: CollectionConstraint | null;
    stringConstraint?: StringConstraint | null;
    numberConstraint?: NumberConstraint | null;
    _session?: Session | null;
    _hash?: number | null;
    _repr?: string | null;
    _PackedObjectCache?: PackedObjectCache[] | null;
  }) {
    /* super */
    super(options);

    /* properties */
    let _defaultValue = options.defaultValue ?? null;
    this.defaultValue = _defaultValue;
    let _defaultFactory = options.defaultFactory ?? null;
    this.defaultFactory = _defaultFactory;
    let _collectionConstraint = options.collectionConstraint ?? null;
    this.collectionConstraint = _collectionConstraint;
    let _stringConstraint = options.stringConstraint ?? null;
    this.stringConstraint = _stringConstraint;
    let _numberConstraint = options.numberConstraint ?? null;
    this.numberConstraint = _numberConstraint;

    /* identity */
    /* ... (already set in parent) */
  }

  equals(other: any): boolean {
    if (!(this.metatype === other.metatype)) {
      return false;
    }
    if (
      (this.defaultValue == null) !== (other.defaultValue == null) ||
      (this.defaultValue != null && !this.defaultValue.equals(other.defaultValue))
    ) {
      return false;
    }
    if (!(this.defaultFactory === other.defaultFactory)) {
      return false;
    }
    if (
      (this.collectionConstraint == null) !== (other.collectionConstraint == null) ||
      (this.collectionConstraint != null &&
        !this.collectionConstraint.equals(other.collectionConstraint))
    ) {
      return false;
    }
    if (
      (this.stringConstraint == null) !== (other.stringConstraint == null) ||
      (this.stringConstraint != null && !this.stringConstraint.equals(other.stringConstraint))
    ) {
      return false;
    }
    if (
      (this.numberConstraint == null) !== (other.numberConstraint == null) ||
      (this.numberConstraint != null && !this.numberConstraint.equals(other.numberConstraint))
    ) {
      return false;
    }
    if (!(this.cardinality === other.cardinality)) {
      return false;
    }
    if (
      (this.keyType == null) !== (other.keyType == null) ||
      (this.keyType != null && !this.keyType.equals(other.keyType))
    ) {
      return false;
    }
    if (
      (this.valueType == null) !== (other.valueType == null) ||
      (this.valueType != null && !this.valueType.equals(other.valueType))
    ) {
      return false;
    }
    if (this.elementTypes == null) {
      return other.elementTypes == null;
    }
    if (this.elementTypes.length != other.elementTypes.length) {
      return false;
    }
    for (let i = 0; i < this.elementTypes.length; i++) {
      if (!this.elementTypes[i].equals(other.elementTypes[i])) {
        return false;
      }
    }

    if (!(this.scalarType === other.scalarType)) {
      return false;
    }
    if (!(this.primitiveType === other.primitiveType)) {
      return false;
    }
    if (!(this.enumType === other.enumType)) {
      return false;
    }
    if (this.nodeTypes == null) {
      return other.nodeTypes == null;
    }
    if (this.nodeTypes.length != other.nodeTypes.length) {
      return false;
    }
    for (let i = 0; i < this.nodeTypes.length; i++) {
      if (!(this.nodeTypes[i] === other.nodeTypes[i])) {
        return false;
      }
    }

    if (!(this.structType === other.structType)) {
      return false;
    }
    if (
      (this.literalValue == null) !== (other.literalValue == null) ||
      (this.literalValue != null && !this.literalValue.equals(other.literalValue))
    ) {
      return false;
    }
    if (this.unionTypes == null) {
      return other.unionTypes == null;
    }
    if (this.unionTypes.length != other.unionTypes.length) {
      return false;
    }
    for (let i = 0; i < this.unionTypes.length; i++) {
      if (!this.unionTypes[i].equals(other.unionTypes[i])) {
        return false;
      }
    }

    if (!(this.isRequired === other.isRequired)) {
      return false;
    }
    return true;
  }

  repr(): string {
    if (this._repr === null) {
      const propertyReprs: string[] = [];
      propertyReprs.push(`cardinality=${TypeCardinality[this.cardinality]}`);
      if (this.keyType != null) {
        propertyReprs.push(`keyType=${this.keyType.repr()}`);
      }
      if (this.valueType != null) {
        propertyReprs.push(`valueType=${this.valueType.repr()}`);
      }
      if (this.elementTypes != null && this.elementTypes.length > 0) {
        propertyReprs.push(
          `elementTypes=${this.elementTypes.map((_item) => _item.repr()).join(", ")}`,
        );
      }
      if (this.scalarType != null) {
        propertyReprs.push(`scalarType=${ScalarType[this.scalarType]}`);
      }
      if (this.primitiveType != null) {
        propertyReprs.push(`primitiveType=${PrimitiveType[this.primitiveType]}`);
      }
      if (this.enumType != null) {
        propertyReprs.push(`enumType=${EnumType[this.enumType]}`);
      }
      if (this.nodeTypes != null && this.nodeTypes.length > 0) {
        propertyReprs.push(
          `nodeTypes=${this.nodeTypes.map((_item) => NodeType[_item]).join(", ")}`,
        );
      }
      if (this.structType != null) {
        propertyReprs.push(`structType=${StructType[this.structType]}`);
      }
      if (this.literalValue != null) {
        propertyReprs.push(`literalValue=${this.literalValue.repr()}`);
      }
      if (this.unionTypes != null && this.unionTypes.length > 0) {
        propertyReprs.push(`unionTypes=${this.unionTypes.map((_item) => _item.repr()).join(", ")}`);
      }
      // @ts-expect-error(readonly) */
      this._repr = `<CheckedType ${propertyReprs.join(" ")}>`;
    }
    return this._repr;
  }

  hash(): number {
    if (this._hash != null) {
      return this._hash;
    }
    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    if (this.defaultValue != null) {
      h = (h * 31 + this.defaultValue.hash()) & 0xffffffff;
    }
    if (this.defaultFactory != null) {
      h = (h * 31 + this.defaultFactory) & 0xffffffff;
    }
    if (this.collectionConstraint != null) {
      h = (h * 31 + this.collectionConstraint.hash()) & 0xffffffff;
    }
    if (this.stringConstraint != null) {
      h = (h * 31 + this.stringConstraint.hash()) & 0xffffffff;
    }
    if (this.numberConstraint != null) {
      h = (h * 31 + this.numberConstraint.hash()) & 0xffffffff;
    }
    h = (h * 31 + this.cardinality) & 0xffffffff;
    if (this.keyType != null) {
      h = (h * 31 + this.keyType.hash()) & 0xffffffff;
    }
    if (this.valueType != null) {
      h = (h * 31 + this.valueType.hash()) & 0xffffffff;
    }
    if (this.elementTypes && this.elementTypes.length > 0) {
      for (const _item of this.elementTypes) {
        h = (h * 31 + _item.hash()) & 0xffffffff;
      }
    }
    if (this.scalarType != null) {
      h = (h * 31 + this.scalarType) & 0xffffffff;
    }
    if (this.primitiveType != null) {
      h = (h * 31 + this.primitiveType) & 0xffffffff;
    }
    if (this.enumType != null) {
      h = (h * 31 + this.enumType) & 0xffffffff;
    }
    if (this.nodeTypes && this.nodeTypes.length > 0) {
      for (const _item of this.nodeTypes) {
        h = (h * 31 + _item) & 0xffffffff;
      }
    }
    if (this.structType != null) {
      h = (h * 31 + this.structType) & 0xffffffff;
    }
    if (this.literalValue != null) {
      h = (h * 31 + this.literalValue.hash()) & 0xffffffff;
    }
    if (this.unionTypes && this.unionTypes.length > 0) {
      for (const _item of this.unionTypes) {
        h = (h * 31 + _item.hash()) & 0xffffffff;
      }
    }
    h = (h * 31 + hashBool(this.isRequired)) & 0xffffffff;
    // @ts-expect-error(readonly)
    this._hash = h;
    return h;
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  /* ... */
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerStructClass(StructType.CHECKED_TYPE, CheckedType);
/* ==== DESTACK_GENERATED_END:STRUCT:102 ==== */
