import {
  EnumType,
  NodeType,
  PRIMITIVE_JS_TYPES,
  PRIMITIVE_TYPE_BY_JS_TYPE,
  PrimitiveType,
  ScalarType,
  StructType,
  TraitType,
  TypeCardinality,
  ValueFactory,
} from "@destack/language/core/builtin/common";
import type {
  CustomEntityDefinition,
  CustomTraitDefinition,
} from "@destack/language/core/builtin/entity";
import type { CustomEventDefinition } from "@destack/language/core/builtin/event";
import { Node, isNode } from "@destack/language/core/builtin/node";
import type { NodeReference } from "@destack/language/core/builtin/relation";
import { StructFrozen, isStruct } from "@destack/language/core/builtin/struct";
import type { CustomEnumDefinition } from "@destack/language/core/common/enum";
import type { CustomStructDefinition } from "@destack/language/core/common/struct";
import type { Value } from "@destack/language/core/common/value";
import type { Supergraph } from "@destack/language/core/runtime/graph";
import type { Session } from "@destack/language/core/runtime/session";
import {
  STRUCT_CLASS_BY_TYPE,
  registerEnumClass,
  registerStructClass,
} from "@destack/language/registry";
import {
  CollectionConstraintProto,
  EnumTypeProto,
  NodeConstraintProto,
  NodeTypeProto,
  NumberConstraintProto,
  NumberFormatProto,
  PrimitiveTypeProto,
  ScalarTypeProto,
  StringConstraintProto,
  StringFormatProto,
  StructTypeProto,
  TraitTypeProto,
  TypeCardinalityProto,
  TypeProto,
  ValueFactoryProto,
} from "@destack/proto";
import { base64Decode } from "@destack/utils";
import { hashBool, hashFloat, hashInt, hashString } from "@destack/utils/hash";

/**
 * Guess the type of a value or class.
 */
export function toType(valueOrType: any, nodeAsValue: boolean = false): Type {
  if (valueOrType === null || valueOrType === undefined) {
    throw new Error("null/undefined is not a valid Type");
  }

  // scalar values
  if (isStruct(valueOrType, StructType.NODE_REFERENCE)) {
    return new Type({
      cardinality: TypeCardinality.SCALAR,
      scalarType: ScalarType.NODE_REFERENCE,
      nodeType: valueOrType.type,
    });
  } else if (isNode(valueOrType)) {
    return new Type({
      cardinality: TypeCardinality.SCALAR,
      scalarType: nodeAsValue ? ScalarType.NODE_VALUE : ScalarType.NODE_REFERENCE,
      nodeType: valueOrType.metatype,
    });
  } else if (isStruct(valueOrType)) {
    return new Type({
      cardinality: TypeCardinality.SCALAR,
      scalarType: ScalarType.STRUCT,
      structType: valueOrType.metatype,
    });
  } else if (
    PRIMITIVE_JS_TYPES.has(valueOrType.constructor) &&
    valueOrType.constructor !== Object
  ) {
    return new Type({
      cardinality: TypeCardinality.SCALAR,
      scalarType: ScalarType.PRIMITIVE,
      primitiveType: PRIMITIVE_TYPE_BY_JS_TYPE.get(valueOrType.constructor) || null,
    });
  }

  // collections
  if (Array.isArray(valueOrType)) {
    if (valueOrType.length === 0) {
      throw new Error(`cannot infer type of empty array: ${valueOrType}`);
    }
    const elementType = toType(valueOrType[0]);
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
      nodeType: elementType.nodeType,
      structType: elementType.structType,
      nodeConstraint: elementType.nodeConstraint,
    });
  } else if (valueOrType instanceof Map) {
    if (valueOrType.size === 0) {
      throw new Error(`cannot infer type of empty Map: ${valueOrType}`);
    }
    const [sampleKey, sampleValue] = Array.from(valueOrType.entries()).at(0)!;
    const keyType = toType(sampleKey);
    if (keyType.cardinality !== TypeCardinality.SCALAR) {
      throw new Error(`expected scalar key in Map, got ${keyType.cardinality} for ${valueOrType}`);
    }
    const valueType = toType(sampleValue);
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
      nodeType: valueType.nodeType,
      structType: valueType.structType,
      nodeConstraint: valueType.nodeConstraint,
      keyType: keyType,
    });
  } else if (typeof valueOrType === "object" && valueOrType.constructor === Object) {
    const keys = Object.keys(valueOrType);
    if (keys.length === 0) {
      throw new Error(`cannot infer type of empty object: ${valueOrType}`);
    }
    const sampleKey = keys[0];
    const sampleValue = valueOrType[sampleKey];
    const keyType = toType(sampleKey);
    if (keyType.cardinality !== TypeCardinality.SCALAR) {
      throw new Error(
        `expected scalar key in object, got ${keyType.cardinality} for ${valueOrType}`,
      );
    }
    const valueType = toType(sampleValue);
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
      nodeType: valueType.nodeType,
      structType: valueType.structType,
      nodeConstraint: valueType.nodeConstraint,
      keyType: keyType,
    });
  }

  throw new Error(`cannot infer type of ${valueOrType}`);
}

/* ==== DESTACK_GENERATED_START:ENUM:64 ==== */
/**
 * StringFormat
 */
export enum StringFormat {
  NAME = 1,
  SLUG = 2,
  EMAIL = 3,
  UUID = 10,
  URL = 11,
  EMOJI = 12,
  MIME = 13,
  BASE64 = 20,

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerEnumClass(EnumType.STRING_FORMAT, StringFormat);
/* ==== DESTACK_GENERATED_END:ENUM:64 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:65 ==== */
/**
 * NumberFormat
 */
export enum NumberFormat {
  PERCENTAGE = 1,
  ANGLE = 2,
  CURRENCY = 3,

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerEnumClass(EnumType.NUMBER_FORMAT, NumberFormat);
/* ==== DESTACK_GENERATED_END:ENUM:65 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:651 ==== */
/**
 * The constraint of a string.
 */
export class StringConstraint extends StructFrozen {
  static metatype: StructType = StructType.STRING_CONSTRAINT;
  static __isFrozen__: boolean = true;

  /**
   * StringConstraint.format
   */
  readonly format: StringFormat | null;

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
    format?: StringFormat | null;
    regex?: string | null;
    startsWith?: string | null;
    endsWith?: string | null;
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
    let _format = options.format ?? null;
    this.format = _format;
    let _regex = options.regex ?? null;
    this.regex = _regex;
    let _startsWith = options.startsWith ?? null;
    this.startsWith = _startsWith;
    let _endsWith = options.endsWith ?? null;
    this.endsWith = _endsWith;

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
    if (!(this.format === other.format)) {
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
    if (this._hash !== null) {
      return this._hash;
    }

    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    if (this.format !== null) {
      h = (h * 31 + this.format) & 0xffffffff;
    }
    if (this.regex !== null) {
      h = (h * 31 + hashString(this.regex)) & 0xffffffff;
    }
    if (this.startsWith !== null) {
      h = (h * 31 + hashString(this.startsWith)) & 0xffffffff;
    }
    if (this.endsWith !== null) {
      h = (h * 31 + hashString(this.endsWith)) & 0xffffffff;
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
      this._value = StringConstraint.__packValue__(this);
    }
    return this._value;
  }

  static __packValue__(object: StringConstraint): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 651;
    if (object.format != null) {
      objectValue["40"] = object.format;
    }
    if (object.regex != null) {
      objectValue["41"] = object.regex;
    }
    if (object.startsWith != null) {
      objectValue["42"] = object.startsWith;
    }
    if (object.endsWith != null) {
      objectValue["43"] = object.endsWith;
    }
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): StringConstraint {
    const formatValue = objectValue["40"];
    const unpackedFormat = formatValue != undefined ? Number(formatValue) : null;
    const regexValue = objectValue["41"];
    const unpackedRegex = regexValue != undefined ? regexValue : null;
    const startsWithValue = objectValue["42"];
    const unpackedStartsWith = startsWithValue != undefined ? startsWithValue : null;
    const endsWithValue = objectValue["43"];
    const unpackedEndsWith = endsWithValue != undefined ? endsWithValue : null;
    return new StringConstraint({
      format: unpackedFormat,
      regex: unpackedRegex,
      startsWith: unpackedStartsWith,
      endsWith: unpackedEndsWith,
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
  ): StringConstraint {
    return StringConstraint.__unpackValue__(
      objectValue,
      _session,
      _supergraph,
      _graph,
      _connection,
    );
  }

  toProto(): StringConstraintProto {
    if (this._proto === null) {
      // @ts-expect-error(readonly)
      this._proto = StringConstraint.__packProto__(this);
    }
    return this._proto as StringConstraintProto;
  }

  static __packProto__(object: StringConstraint): StringConstraintProto {
    const objectProto: Partial<StringConstraintProto> = { metatype: 651 };
    if (object.format != null) {
      objectProto.format = Number(object.format) as StringFormatProto;
    }
    if (object.regex != null) {
      objectProto.regex = object.regex;
    }
    if (object.startsWith != null) {
      objectProto.startsWith = object.startsWith;
    }
    if (object.endsWith != null) {
      objectProto.endsWith = object.endsWith;
    }
    return objectProto as StringConstraintProto;
  }

  static __unpackProto__(
    objectProto: StringConstraintProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): StringConstraint {
    return new StringConstraint({
      format: objectProto.format != undefined ? (Number(objectProto.format) as StringFormat) : null,
      regex: objectProto.regex != undefined ? objectProto.regex : null,
      startsWith: objectProto.startsWith != undefined ? objectProto.startsWith : null,
      endsWith: objectProto.endsWith != undefined ? objectProto.endsWith : null,
      _proto: objectProto,
      _supergraph,
    });
  }

  static fromProto(
    objectProto: StringConstraintProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): StringConstraint {
    return StringConstraint.__unpackProto__(
      objectProto,
      _session,
      _supergraph,
      _graph,
      _connection,
    );
  }

  static fromProtoString(packedProtoString: string): StringConstraint {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = StringConstraintProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerStructClass(StructType.STRING_CONSTRAINT, StringConstraint);
/* ==== DESTACK_GENERATED_END:STRUCT:651 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:650 ==== */
/**
 * The constraint of a number.
 */
export class NumberConstraint extends StructFrozen {
  static metatype: StructType = StructType.NUMBER_CONSTRAINT;
  static __isFrozen__: boolean = true;

  /**
   * NumberConstraint.format
   */
  readonly format: NumberFormat | null;

  /**
   * NumberConstraint.minValue
   */
  readonly minValue: number | null;

  /**
   * NumberConstraint.maxValue
   */
  readonly maxValue: number | null;

  /**
   * NumberConstraint.stepValue
   */
  readonly stepValue: number | null;

  /**
   * NumberConstraint.precision
   */
  readonly precision: number | null;

  /**
   * NumberConstraint.scale
   */
  readonly scale: number | null;

  constructor(options: {
    format?: NumberFormat | null;
    minValue?: number | null;
    maxValue?: number | null;
    stepValue?: number | null;
    precision?: number | null;
    scale?: number | null;
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
    let _format = options.format ?? null;
    this.format = _format;
    let _minValue = options.minValue ?? null;
    this.minValue = _minValue;
    let _maxValue = options.maxValue ?? null;
    this.maxValue = _maxValue;
    let _stepValue = options.stepValue ?? null;
    this.stepValue = _stepValue;
    let _precision = options.precision ?? null;
    this.precision = _precision;
    let _scale = options.scale ?? null;
    this.scale = _scale;

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
    if (!(this.format === other.format)) {
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
    if (!(this.precision === other.precision)) {
      return false;
    }
    if (!(this.scale === other.scale)) {
      return false;
    }
    return true;
  }

  repr(): string {
    return `<NumberConstraint>`;
  }

  hash(): number {
    if (this._hash !== null) {
      return this._hash;
    }

    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    if (this.format !== null) {
      h = (h * 31 + this.format) & 0xffffffff;
    }
    if (this.minValue !== null) {
      h = (h * 31 + hashFloat(this.minValue)) & 0xffffffff;
    }
    if (this.maxValue !== null) {
      h = (h * 31 + hashFloat(this.maxValue)) & 0xffffffff;
    }
    if (this.stepValue !== null) {
      h = (h * 31 + hashFloat(this.stepValue)) & 0xffffffff;
    }
    if (this.precision !== null) {
      h = (h * 31 + hashInt(this.precision)) & 0xffffffff;
    }
    if (this.scale !== null) {
      h = (h * 31 + hashInt(this.scale)) & 0xffffffff;
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
      this._value = NumberConstraint.__packValue__(this);
    }
    return this._value;
  }

  static __packValue__(object: NumberConstraint): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 650;
    if (object.format != null) {
      objectValue["40"] = object.format;
    }
    if (object.minValue != null) {
      objectValue["41"] = object.minValue;
    }
    if (object.maxValue != null) {
      objectValue["42"] = object.maxValue;
    }
    if (object.stepValue != null) {
      objectValue["43"] = object.stepValue;
    }
    if (object.precision != null) {
      objectValue["44"] = object.precision;
    }
    if (object.scale != null) {
      objectValue["45"] = object.scale;
    }
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): NumberConstraint {
    const formatValue = objectValue["40"];
    const unpackedFormat = formatValue != undefined ? Number(formatValue) : null;
    const minValueValue = objectValue["41"];
    const unpackedMinValue = minValueValue != undefined ? minValueValue : null;
    const maxValueValue = objectValue["42"];
    const unpackedMaxValue = maxValueValue != undefined ? maxValueValue : null;
    const stepValueValue = objectValue["43"];
    const unpackedStepValue = stepValueValue != undefined ? stepValueValue : null;
    const precisionValue = objectValue["44"];
    const unpackedPrecision = precisionValue != undefined ? Number(precisionValue) : null;
    const scaleValue = objectValue["45"];
    const unpackedScale = scaleValue != undefined ? Number(scaleValue) : null;
    return new NumberConstraint({
      format: unpackedFormat,
      minValue: unpackedMinValue,
      maxValue: unpackedMaxValue,
      stepValue: unpackedStepValue,
      precision: unpackedPrecision,
      scale: unpackedScale,
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
  ): NumberConstraint {
    return NumberConstraint.__unpackValue__(
      objectValue,
      _session,
      _supergraph,
      _graph,
      _connection,
    );
  }

  toProto(): NumberConstraintProto {
    if (this._proto === null) {
      // @ts-expect-error(readonly)
      this._proto = NumberConstraint.__packProto__(this);
    }
    return this._proto as NumberConstraintProto;
  }

  static __packProto__(object: NumberConstraint): NumberConstraintProto {
    const objectProto: Partial<NumberConstraintProto> = { metatype: 650 };
    if (object.format != null) {
      objectProto.format = Number(object.format) as NumberFormatProto;
    }
    if (object.minValue != null) {
      objectProto.minValue = object.minValue;
    }
    if (object.maxValue != null) {
      objectProto.maxValue = object.maxValue;
    }
    if (object.stepValue != null) {
      objectProto.stepValue = object.stepValue;
    }
    if (object.precision != null) {
      objectProto.precision = object.precision;
    }
    if (object.scale != null) {
      objectProto.scale = object.scale;
    }
    return objectProto as NumberConstraintProto;
  }

  static __unpackProto__(
    objectProto: NumberConstraintProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): NumberConstraint {
    return new NumberConstraint({
      format: objectProto.format != undefined ? (Number(objectProto.format) as NumberFormat) : null,
      minValue: objectProto.minValue != undefined ? objectProto.minValue : null,
      maxValue: objectProto.maxValue != undefined ? objectProto.maxValue : null,
      stepValue: objectProto.stepValue != undefined ? objectProto.stepValue : null,
      precision: objectProto.precision != undefined ? Number(objectProto.precision) : null,
      scale: objectProto.scale != undefined ? Number(objectProto.scale) : null,
      _proto: objectProto,
      _supergraph,
    });
  }

  static fromProto(
    objectProto: NumberConstraintProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): NumberConstraint {
    return NumberConstraint.__unpackProto__(
      objectProto,
      _session,
      _supergraph,
      _graph,
      _connection,
    );
  }

  static fromProtoString(packedProtoString: string): NumberConstraint {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = NumberConstraintProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerStructClass(StructType.NUMBER_CONSTRAINT, NumberConstraint);
/* ==== DESTACK_GENERATED_END:STRUCT:650 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:652 ==== */
/**
 * The constraint of a collection.
 */
export class CollectionConstraint extends StructFrozen {
  static metatype: StructType = StructType.COLLECTION_CONSTRAINT;
  static __isFrozen__: boolean = true;

  /**
   * CollectionConstraint.minLength
   */
  readonly minLength: number | null;

  /**
   * CollectionConstraint.maxLength
   */
  readonly maxLength: number | null;

  constructor(options: {
    minLength?: number | null;
    maxLength?: number | null;
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
    let _minLength = options.minLength ?? null;
    this.minLength = _minLength;
    let _maxLength = options.maxLength ?? null;
    this.maxLength = _maxLength;

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
    if (this._hash !== null) {
      return this._hash;
    }

    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    if (this.minLength !== null) {
      h = (h * 31 + hashInt(this.minLength)) & 0xffffffff;
    }
    if (this.maxLength !== null) {
      h = (h * 31 + hashInt(this.maxLength)) & 0xffffffff;
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
      this._value = CollectionConstraint.__packValue__(this);
    }
    return this._value;
  }

  static __packValue__(object: CollectionConstraint): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 652;
    if (object.minLength != null) {
      objectValue["41"] = object.minLength;
    }
    if (object.maxLength != null) {
      objectValue["42"] = object.maxLength;
    }
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): CollectionConstraint {
    const minLengthValue = objectValue["41"];
    const unpackedMinLength = minLengthValue != undefined ? Number(minLengthValue) : null;
    const maxLengthValue = objectValue["42"];
    const unpackedMaxLength = maxLengthValue != undefined ? Number(maxLengthValue) : null;
    return new CollectionConstraint({
      minLength: unpackedMinLength,
      maxLength: unpackedMaxLength,
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
  ): CollectionConstraint {
    return CollectionConstraint.__unpackValue__(
      objectValue,
      _session,
      _supergraph,
      _graph,
      _connection,
    );
  }

  toProto(): CollectionConstraintProto {
    if (this._proto === null) {
      // @ts-expect-error(readonly)
      this._proto = CollectionConstraint.__packProto__(this);
    }
    return this._proto as CollectionConstraintProto;
  }

  static __packProto__(object: CollectionConstraint): CollectionConstraintProto {
    const objectProto: Partial<CollectionConstraintProto> = { metatype: 652 };
    if (object.minLength != null) {
      objectProto.minLength = object.minLength;
    }
    if (object.maxLength != null) {
      objectProto.maxLength = object.maxLength;
    }
    return objectProto as CollectionConstraintProto;
  }

  static __unpackProto__(
    objectProto: CollectionConstraintProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): CollectionConstraint {
    return new CollectionConstraint({
      minLength: objectProto.minLength != undefined ? Number(objectProto.minLength) : null,
      maxLength: objectProto.maxLength != undefined ? Number(objectProto.maxLength) : null,
      _proto: objectProto,
      _supergraph,
    });
  }

  static fromProto(
    objectProto: CollectionConstraintProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): CollectionConstraint {
    return CollectionConstraint.__unpackProto__(
      objectProto,
      _session,
      _supergraph,
      _graph,
      _connection,
    );
  }

  static fromProtoString(packedProtoString: string): CollectionConstraint {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = CollectionConstraintProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerStructClass(StructType.COLLECTION_CONSTRAINT, CollectionConstraint);
/* ==== DESTACK_GENERATED_END:STRUCT:652 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:653 ==== */
/**
 * The constraint of a node.
 */
export class NodeConstraint extends StructFrozen {
  static metatype: StructType = StructType.NODE_CONSTRAINT;
  static __isFrozen__: boolean = true;

  /**
   * NodeConstraint.nodeTypes
   */
  readonly nodeTypes: Array<NodeType>;

  /**
   * NodeConstraint.nodeTraits
   */
  readonly nodeTraits: Array<TraitType>;

  constructor(options: {
    nodeTypes?: Array<NodeType>;
    nodeTraits?: Array<TraitType>;
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
    let _nodeTypes = options.nodeTypes ?? null;
    if (_nodeTypes === null) {
      _nodeTypes = [];
    }
    this.nodeTypes = _nodeTypes;
    let _nodeTraits = options.nodeTraits ?? null;
    if (_nodeTraits === null) {
      _nodeTraits = [];
    }
    this.nodeTraits = _nodeTraits;

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
    if (this.nodeTypes.length !== other.nodeTypes.length) {
      return false;
    }
    for (let i = 0; i < this.nodeTypes.length; i++) {
      if (!(this.nodeTypes[i] === other.nodeTypes[i])) {
        return false;
      }
    }
    if (this.nodeTraits.length !== other.nodeTraits.length) {
      return false;
    }
    for (let i = 0; i < this.nodeTraits.length; i++) {
      if (!(this.nodeTraits[i] === other.nodeTraits[i])) {
        return false;
      }
    }
    return true;
  }

  repr(): string {
    return `<NodeConstraint>`;
  }

  hash(): number {
    if (this._hash !== null) {
      return this._hash;
    }

    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    if (this.nodeTypes && this.nodeTypes.length > 0) {
      for (const _item of this.nodeTypes) {
        h = (h * 31 + _item) & 0xffffffff;
      }
    }
    if (this.nodeTraits && this.nodeTraits.length > 0) {
      for (const _item of this.nodeTraits) {
        h = (h * 31 + _item) & 0xffffffff;
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
      this._value = NodeConstraint.__packValue__(this);
    }
    return this._value;
  }

  static __packValue__(object: NodeConstraint): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 653;
    if (object.nodeTypes.length > 0) {
      const packedNodeTypes: any[] = [];
      for (const item of object.nodeTypes) {
        packedNodeTypes.push(item);
      }
      objectValue["41"] = packedNodeTypes;
    }
    if (object.nodeTraits.length > 0) {
      const packedNodeTraits: any[] = [];
      for (const item of object.nodeTraits) {
        packedNodeTraits.push(item);
      }
      objectValue["42"] = packedNodeTraits;
    }
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): NodeConstraint {
    const unpackedNodeTypes: any[] = [];
    if (objectValue["41"] != undefined) {
      for (const item of objectValue["41"]) {
        unpackedNodeTypes.push(Number(item));
      }
    }
    const unpackedNodeTraits: any[] = [];
    if (objectValue["42"] != undefined) {
      for (const item of objectValue["42"]) {
        unpackedNodeTraits.push(Number(item));
      }
    }
    return new NodeConstraint({
      nodeTypes: unpackedNodeTypes,
      nodeTraits: unpackedNodeTraits,
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
  ): NodeConstraint {
    return NodeConstraint.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
  }

  toProto(): NodeConstraintProto {
    if (this._proto === null) {
      // @ts-expect-error(readonly)
      this._proto = NodeConstraint.__packProto__(this);
    }
    return this._proto as NodeConstraintProto;
  }

  static __packProto__(object: NodeConstraint): NodeConstraintProto {
    const objectProto: Partial<NodeConstraintProto> = { metatype: 653 };
    if (object.nodeTypes) {
      const packedNodeTypes: any[] = [];
      for (const item of object.nodeTypes) {
        packedNodeTypes.push(Number(item) as NodeTypeProto);
      }
      objectProto.nodeTypes = packedNodeTypes;
    }
    if (object.nodeTraits) {
      const packedNodeTraits: any[] = [];
      for (const item of object.nodeTraits) {
        packedNodeTraits.push(Number(item) as TraitTypeProto);
      }
      objectProto.nodeTraits = packedNodeTraits;
    }
    return objectProto as NodeConstraintProto;
  }

  static __unpackProto__(
    objectProto: NodeConstraintProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): NodeConstraint {
    const unpackedNodeTypes: any[] = [];
    if (objectProto.nodeTypes) {
      for (const item of objectProto.nodeTypes) {
        unpackedNodeTypes.push(Number(item) as NodeType);
      }
    }
    const unpackedNodeTraits: any[] = [];
    if (objectProto.nodeTraits) {
      for (const item of objectProto.nodeTraits) {
        unpackedNodeTraits.push(Number(item) as TraitType);
      }
    }
    return new NodeConstraint({
      nodeTypes: unpackedNodeTypes,
      nodeTraits: unpackedNodeTraits,
      _proto: objectProto,
      _supergraph,
    });
  }

  static fromProto(
    objectProto: NodeConstraintProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): NodeConstraint {
    return NodeConstraint.__unpackProto__(objectProto, _session, _supergraph, _graph, _connection);
  }

  static fromProtoString(packedProtoString: string): NodeConstraint {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = NodeConstraintProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerStructClass(StructType.NODE_CONSTRAINT, NodeConstraint);
/* ==== DESTACK_GENERATED_END:STRUCT:653 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:601 ==== */
/**
 * A Type in the type system.
 */
export class Type extends StructFrozen {
  static metatype: StructType = StructType.TYPE;
  static __isFrozen__: boolean = true;

  /**
   * Type.cardinality
   */
  readonly cardinality: TypeCardinality;

  /**
   * Type.scalarType
   */
  readonly scalarType: ScalarType;

  /**
   * Type.primitiveType
   */
  readonly primitiveType: PrimitiveType | null;

  /**
   * Type.enumType
   */
  readonly enumType: EnumType | null;

  /**
   * Type.nodeType
   */
  readonly nodeType: NodeType | null;

  /**
   * Type.structType
   */
  readonly structType: StructType | null;

  /**
   * Type.definition
   */
  get definition():
    | CustomEntityDefinition
    | CustomEventDefinition
    | CustomEnumDefinition
    | CustomStructDefinition
    | CustomTraitDefinition
    | null {
    const nodePtr: NodeReference | null = this.definitionPtr;
    if (nodePtr !== null) {
      if (this._supergraph === null) {
        return null;
      }
      return this._supergraph.get(nodePtr.id) as
        | CustomEntityDefinition
        | CustomEventDefinition
        | CustomEnumDefinition
        | CustomStructDefinition
        | CustomTraitDefinition
        | null;
    }
    return null;
  }
  readonly definitionPtr: NodeReference | null;

  /**
   * Type.keyType
   */
  readonly keyType: Type | null;

  /**
   * Type.isRequired
   */
  readonly isRequired: boolean | null;

  /**
   * Type.value
   */
  readonly value: Value | null;

  /**
   * Type.valueFactory
   */
  readonly valueFactory: ValueFactory | null;

  /**
   * Type.collectionConstraint
   */
  readonly collectionConstraint: CollectionConstraint | null;

  /**
   * Type.stringConstraint
   */
  readonly stringConstraint: StringConstraint | null;

  /**
   * Type.numberConstraint
   */
  readonly numberConstraint: NumberConstraint | null;

  /**
   * Type.nodeConstraint
   */
  readonly nodeConstraint: NodeConstraint | null;

  constructor(options: {
    cardinality?: TypeCardinality;
    scalarType: ScalarType;
    primitiveType?: PrimitiveType | null;
    enumType?: EnumType | null;
    nodeType?: NodeType | null;
    structType?: StructType | null;
    definition?:
      | CustomEntityDefinition
      | CustomEventDefinition
      | CustomEnumDefinition
      | CustomStructDefinition
      | CustomTraitDefinition
      | NodeReference
      | null;
    keyType?: Type | null;
    isRequired?: boolean | null;
    value?: Value | null;
    valueFactory?: ValueFactory | null;
    collectionConstraint?: CollectionConstraint | null;
    stringConstraint?: StringConstraint | null;
    numberConstraint?: NumberConstraint | null;
    nodeConstraint?: NodeConstraint | null;
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
    let _cardinality = options.cardinality ?? null;
    if (_cardinality === null) {
      _cardinality = 1 /* TypeCardinality.SCALAR */;
    }
    if (_cardinality === null) {
      throw new Error(`Type.cardinality is required`);
    }
    this.cardinality = _cardinality;
    let _scalarType = options.scalarType;
    if (_scalarType === null) {
      throw new Error(`Type.scalarType is required`);
    }
    this.scalarType = _scalarType;
    let _primitiveType = options.primitiveType ?? null;
    this.primitiveType = _primitiveType;
    let _enumType = options.enumType ?? null;
    this.enumType = _enumType;
    let _nodeType = options.nodeType ?? null;
    this.nodeType = _nodeType;
    let _structType = options.structType ?? null;
    this.structType = _structType;
    let _definition = options.definition ?? null;
    if (_definition != null && _definition.metatype != StructType.NODE_REFERENCE) {
      _definition = (_definition as Node).toRef();
    }
    this.definitionPtr = _definition;
    let _keyType = options.keyType ?? null;
    this.keyType = _keyType;
    let _isRequired = options.isRequired ?? null;
    this.isRequired = _isRequired;
    let _value = options.value ?? null;
    this.value = _value;
    let _valueFactory = options.valueFactory ?? null;
    this.valueFactory = _valueFactory;
    let _collectionConstraint = options.collectionConstraint ?? null;
    this.collectionConstraint = _collectionConstraint;
    let _stringConstraint = options.stringConstraint ?? null;
    this.stringConstraint = _stringConstraint;
    let _numberConstraint = options.numberConstraint ?? null;
    this.numberConstraint = _numberConstraint;
    let _nodeConstraint = options.nodeConstraint ?? null;
    this.nodeConstraint = _nodeConstraint;

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
    if (!(this.cardinality === other.cardinality)) {
      return false;
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
    if (!(this.nodeType === other.nodeType)) {
      return false;
    }
    if (!(this.structType === other.structType)) {
      return false;
    }
    if (!(this.definitionPtr?.id === other.definitionPtr?.id)) {
      return false;
    }
    if (
      (this.keyType == null) !== (other.keyType == null) ||
      (this.keyType != null && !this.keyType.equals(other.keyType))
    ) {
      return false;
    }
    if (!(this.isRequired === other.isRequired)) {
      return false;
    }
    if (
      (this.value == null) !== (other.value == null) ||
      (this.value != null && !this.value.equals(other.value))
    ) {
      return false;
    }
    if (!(this.valueFactory === other.valueFactory)) {
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
    if (
      (this.nodeConstraint == null) !== (other.nodeConstraint == null) ||
      (this.nodeConstraint != null && !this.nodeConstraint.equals(other.nodeConstraint))
    ) {
      return false;
    }
    return true;
  }

  repr(): string {
    if (this._repr === null) {
      const propertyReprs: string[] = [];
      propertyReprs.push(`cardinality=${TypeCardinality[this.cardinality]}`);
      propertyReprs.push(`scalarType=${ScalarType[this.scalarType]}`);
      if (this.primitiveType !== null) {
        propertyReprs.push(`primitiveType=${PrimitiveType[this.primitiveType]}`);
      }
      if (this.enumType !== null) {
        propertyReprs.push(`enumType=${EnumType[this.enumType]}`);
      }
      if (this.nodeType !== null) {
        propertyReprs.push(`nodeType=${NodeType[this.nodeType]}`);
      }
      if (this.structType !== null) {
        propertyReprs.push(`structType=${StructType[this.structType]}`);
      }
      if (this.definition !== null) {
        propertyReprs.push(`definition=${this.definition?.repr()}`);
      }
      if (this.keyType !== null) {
        propertyReprs.push(`keyType=${this.keyType.repr()}`);
      }
      // @ts-expect-error(readonly)
      this._repr = `<Type ${propertyReprs.join(" ")}>`;
    }
    return this._repr;
  }

  hash(): number {
    if (this._hash !== null) {
      return this._hash;
    }

    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    h = (h * 31 + this.cardinality) & 0xffffffff;
    h = (h * 31 + this.scalarType) & 0xffffffff;
    if (this.primitiveType !== null) {
      h = (h * 31 + this.primitiveType) & 0xffffffff;
    }
    if (this.enumType !== null) {
      h = (h * 31 + this.enumType) & 0xffffffff;
    }
    if (this.nodeType !== null) {
      h = (h * 31 + this.nodeType) & 0xffffffff;
    }
    if (this.structType !== null) {
      h = (h * 31 + this.structType) & 0xffffffff;
    }
    if (this.definitionPtr !== null) {
      h = (h * 31 + hashString(this.definitionPtr.id)) & 0xffffffff;
    }
    if (this.keyType !== null) {
      h = (h * 31 + this.keyType.hash()) & 0xffffffff;
    }
    if (this.isRequired !== null) {
      h = (h * 31 + hashBool(this.isRequired)) & 0xffffffff;
    }
    if (this.value !== null) {
      h = (h * 31 + this.value.hash()) & 0xffffffff;
    }
    if (this.valueFactory !== null) {
      h = (h * 31 + this.valueFactory) & 0xffffffff;
    }
    if (this.collectionConstraint !== null) {
      h = (h * 31 + this.collectionConstraint.hash()) & 0xffffffff;
    }
    if (this.stringConstraint !== null) {
      h = (h * 31 + this.stringConstraint.hash()) & 0xffffffff;
    }
    if (this.numberConstraint !== null) {
      h = (h * 31 + this.numberConstraint.hash()) & 0xffffffff;
    }
    if (this.nodeConstraint !== null) {
      h = (h * 31 + this.nodeConstraint.hash()) & 0xffffffff;
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
      this._value = Type.__packValue__(this);
    }
    return this._value;
  }

  static __packValue__(object: Type): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 601;
    objectValue["110"] = object.cardinality;
    objectValue["111"] = object.scalarType;
    if (object.primitiveType != null) {
      objectValue["112"] = object.primitiveType;
    }
    if (object.enumType != null) {
      objectValue["113"] = object.enumType;
    }
    if (object.nodeType != null) {
      objectValue["114"] = object.nodeType;
    }
    if (object.structType != null) {
      objectValue["115"] = object.structType;
    }
    if (object.definitionPtr != null) {
      objectValue["116"] = object.definitionPtr.toValue();
    }
    if (object.keyType != null) {
      objectValue["117"] = object.keyType.toValue();
    }
    if (object.isRequired != null) {
      objectValue["118"] = object.isRequired;
    }
    if (object.value != null) {
      objectValue["130"] = object.value.toValue();
    }
    if (object.valueFactory != null) {
      objectValue["131"] = object.valueFactory;
    }
    if (object.collectionConstraint != null) {
      objectValue["140"] = object.collectionConstraint.toValue();
    }
    if (object.stringConstraint != null) {
      objectValue["141"] = object.stringConstraint.toValue();
    }
    if (object.numberConstraint != null) {
      objectValue["142"] = object.numberConstraint.toValue();
    }
    if (object.nodeConstraint != null) {
      objectValue["143"] = object.nodeConstraint.toValue();
    }
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Type {
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    const _Value = STRUCT_CLASS_BY_TYPE[StructType.VALUE] as typeof Value;
    const _Type = STRUCT_CLASS_BY_TYPE[StructType.TYPE] as typeof Type;
    const _NumberConstraint = STRUCT_CLASS_BY_TYPE[
      StructType.NUMBER_CONSTRAINT
    ] as typeof NumberConstraint;
    const _StringConstraint = STRUCT_CLASS_BY_TYPE[
      StructType.STRING_CONSTRAINT
    ] as typeof StringConstraint;
    const _CollectionConstraint = STRUCT_CLASS_BY_TYPE[
      StructType.COLLECTION_CONSTRAINT
    ] as typeof CollectionConstraint;
    const _NodeConstraint = STRUCT_CLASS_BY_TYPE[
      StructType.NODE_CONSTRAINT
    ] as typeof NodeConstraint;
    const primitiveTypeValue = objectValue["112"];
    const unpackedPrimitiveType =
      primitiveTypeValue != undefined ? Number(primitiveTypeValue) : null;
    const enumTypeValue = objectValue["113"];
    const unpackedEnumType = enumTypeValue != undefined ? Number(enumTypeValue) : null;
    const nodeTypeValue = objectValue["114"];
    const unpackedNodeType = nodeTypeValue != undefined ? Number(nodeTypeValue) : null;
    const structTypeValue = objectValue["115"];
    const unpackedStructType = structTypeValue != undefined ? Number(structTypeValue) : null;
    const definitionPtrValue = objectValue["116"];
    const unpackedDefinitionPtr =
      definitionPtrValue != undefined
        ? _NodeReference.fromValue(definitionPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const keyTypeValue = objectValue["117"];
    const unpackedKeyType =
      keyTypeValue != undefined
        ? _Type.fromValue(keyTypeValue, _session, _supergraph, _graph, _connection)
        : null;
    const isRequiredValue = objectValue["118"];
    const unpackedIsRequired = isRequiredValue != undefined ? isRequiredValue : null;
    const valueValue = objectValue["130"];
    const unpackedValue =
      valueValue != undefined
        ? _Value.fromValue(valueValue, _session, _supergraph, _graph, _connection)
        : null;
    const valueFactoryValue = objectValue["131"];
    const unpackedValueFactory = valueFactoryValue != undefined ? Number(valueFactoryValue) : null;
    const collectionConstraintValue = objectValue["140"];
    const unpackedCollectionConstraint =
      collectionConstraintValue != undefined
        ? _CollectionConstraint.fromValue(
            collectionConstraintValue,
            _session,
            _supergraph,
            _graph,
            _connection,
          )
        : null;
    const stringConstraintValue = objectValue["141"];
    const unpackedStringConstraint =
      stringConstraintValue != undefined
        ? _StringConstraint.fromValue(
            stringConstraintValue,
            _session,
            _supergraph,
            _graph,
            _connection,
          )
        : null;
    const numberConstraintValue = objectValue["142"];
    const unpackedNumberConstraint =
      numberConstraintValue != undefined
        ? _NumberConstraint.fromValue(
            numberConstraintValue,
            _session,
            _supergraph,
            _graph,
            _connection,
          )
        : null;
    const nodeConstraintValue = objectValue["143"];
    const unpackedNodeConstraint =
      nodeConstraintValue != undefined
        ? _NodeConstraint.fromValue(nodeConstraintValue, _session, _supergraph, _graph, _connection)
        : null;
    return new Type({
      cardinality: Number(objectValue["110"]),
      scalarType: Number(objectValue["111"]),
      primitiveType: unpackedPrimitiveType,
      enumType: unpackedEnumType,
      nodeType: unpackedNodeType,
      structType: unpackedStructType,
      definition: unpackedDefinitionPtr,
      keyType: unpackedKeyType,
      isRequired: unpackedIsRequired,
      value: unpackedValue,
      valueFactory: unpackedValueFactory,
      collectionConstraint: unpackedCollectionConstraint,
      stringConstraint: unpackedStringConstraint,
      numberConstraint: unpackedNumberConstraint,
      nodeConstraint: unpackedNodeConstraint,
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
  ): Type {
    return Type.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
  }

  toProto(): TypeProto {
    if (this._proto === null) {
      // @ts-expect-error(readonly)
      this._proto = Type.__packProto__(this);
    }
    return this._proto as TypeProto;
  }

  static __packProto__(object: Type): TypeProto {
    const objectProto: Partial<TypeProto> = { metatype: 601 };
    objectProto.cardinality = Number(object.cardinality) as TypeCardinalityProto;
    objectProto.scalarType = Number(object.scalarType) as ScalarTypeProto;
    if (object.primitiveType != null) {
      objectProto.primitiveType = Number(object.primitiveType) as PrimitiveTypeProto;
    }
    if (object.enumType != null) {
      objectProto.enumType = Number(object.enumType) as EnumTypeProto;
    }
    if (object.nodeType != null) {
      objectProto.nodeType = Number(object.nodeType) as NodeTypeProto;
    }
    if (object.structType != null) {
      objectProto.structType = Number(object.structType) as StructTypeProto;
    }
    if (object.definitionPtr != null) {
      objectProto.definitionPtr = object.definitionPtr.toProto();
    }
    if (object.keyType != null) {
      objectProto.keyType = object.keyType.toProto();
    }
    if (object.isRequired != null) {
      objectProto.isRequired = object.isRequired;
    }
    if (object.value != null) {
      objectProto.value = object.value.toProto();
    }
    if (object.valueFactory != null) {
      objectProto.valueFactory = Number(object.valueFactory) as ValueFactoryProto;
    }
    if (object.collectionConstraint != null) {
      objectProto.collectionConstraint = object.collectionConstraint.toProto();
    }
    if (object.stringConstraint != null) {
      objectProto.stringConstraint = object.stringConstraint.toProto();
    }
    if (object.numberConstraint != null) {
      objectProto.numberConstraint = object.numberConstraint.toProto();
    }
    if (object.nodeConstraint != null) {
      objectProto.nodeConstraint = object.nodeConstraint.toProto();
    }
    return objectProto as TypeProto;
  }

  static __unpackProto__(
    objectProto: TypeProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Type {
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    const _Value = STRUCT_CLASS_BY_TYPE[StructType.VALUE] as typeof Value;
    const _Type = STRUCT_CLASS_BY_TYPE[StructType.TYPE] as typeof Type;
    const _NumberConstraint = STRUCT_CLASS_BY_TYPE[
      StructType.NUMBER_CONSTRAINT
    ] as typeof NumberConstraint;
    const _StringConstraint = STRUCT_CLASS_BY_TYPE[
      StructType.STRING_CONSTRAINT
    ] as typeof StringConstraint;
    const _CollectionConstraint = STRUCT_CLASS_BY_TYPE[
      StructType.COLLECTION_CONSTRAINT
    ] as typeof CollectionConstraint;
    const _NodeConstraint = STRUCT_CLASS_BY_TYPE[
      StructType.NODE_CONSTRAINT
    ] as typeof NodeConstraint;
    return new Type({
      cardinality: Number(objectProto.cardinality) as TypeCardinality,
      scalarType: Number(objectProto.scalarType) as ScalarType,
      primitiveType:
        objectProto.primitiveType != undefined
          ? (Number(objectProto.primitiveType) as PrimitiveType)
          : null,
      enumType:
        objectProto.enumType != undefined ? (Number(objectProto.enumType) as EnumType) : null,
      nodeType:
        objectProto.nodeType != undefined ? (Number(objectProto.nodeType) as NodeType) : null,
      structType:
        objectProto.structType != undefined ? (Number(objectProto.structType) as StructType) : null,
      definition:
        objectProto.definitionPtr != undefined
          ? _NodeReference.fromProto(
              objectProto.definitionPtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      keyType:
        objectProto.keyType != undefined
          ? _Type.fromProto(objectProto.keyType!, _session, _supergraph, _graph, _connection)
          : null,
      isRequired: objectProto.isRequired != undefined ? objectProto.isRequired : null,
      value:
        objectProto.value != undefined
          ? _Value.fromProto(objectProto.value!, _session, _supergraph, _graph, _connection)
          : null,
      valueFactory:
        objectProto.valueFactory != undefined
          ? (Number(objectProto.valueFactory) as ValueFactory)
          : null,
      collectionConstraint:
        objectProto.collectionConstraint != undefined
          ? _CollectionConstraint.fromProto(
              objectProto.collectionConstraint!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      stringConstraint:
        objectProto.stringConstraint != undefined
          ? _StringConstraint.fromProto(
              objectProto.stringConstraint!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      numberConstraint:
        objectProto.numberConstraint != undefined
          ? _NumberConstraint.fromProto(
              objectProto.numberConstraint!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      nodeConstraint:
        objectProto.nodeConstraint != undefined
          ? _NodeConstraint.fromProto(
              objectProto.nodeConstraint!,
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
    objectProto: TypeProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Type {
    return Type.__unpackProto__(objectProto, _session, _supergraph, _graph, _connection);
  }

  static fromProtoString(packedProtoString: string): Type {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = TypeProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerStructClass(StructType.TYPE, Type);
/* ==== DESTACK_GENERATED_END:STRUCT:601 ==== */
