import { PRIMITIVE_JS_TYPES, PRIMITIVE_TYPE_BY_JS_TYPE } from "@destack/language";
import { NodeReference, Session, Supergraph } from "@destack/language/core";
import {
  DefaultFactory,
  EnumType,
  Node,
  NodeType,
  PrimitiveType,
  ScalarType,
  Struct,
  StructFrozen,
  StructType,
  TraitType,
  TypeCardinality,
} from "@destack/language/core/builtin";
import { CustomEntityDefinition, Value } from "@destack/language/core/common";
import { registerEnumClass, registerStructClass } from "@destack/language/registry";
import {
  CollectionConstraintProto,
  DefaultFactoryProto,
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
} from "@destack/proto";

/* ==== DESTACK_GENERATED_START:ENUM:2570 ==== */
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
/* ==== DESTACK_GENERATED_END:ENUM:2570 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:2571 ==== */
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
/* ==== DESTACK_GENERATED_END:ENUM:2571 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:2503 ==== */
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

  hash(): number {
    throw new Error("not implemented");
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
    objectValue["1"] = 2503;
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
    return StringConstraint.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
  }

  toProto(): StringConstraintProto {
    if (this._proto === null) {
      // @ts-expect-error(readonly)
      this._proto = StringConstraint.__packProto__(this);
    }
    return this._proto as StringConstraintProto;
  }

  static __packProto__(object: StringConstraint): StringConstraintProto {
    const objectProto: Partial<StringConstraintProto> = { metatype: 2503 };
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
    return StringConstraint.__unpackProto__(objectProto, _session, _supergraph, _graph, _connection);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerStructClass(StructType.STRING_CONSTRAINT, StringConstraint);
/* ==== DESTACK_GENERATED_END:STRUCT:2503 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:2502 ==== */
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
      (this.minValue != null && !(this.minValue === other.minValue || Math.abs(this.minValue - other.minValue) < 1e-10))
    ) {
      return false;
    }
    if (
      (this.maxValue == null) !== (other.maxValue == null) ||
      (this.maxValue != null && !(this.maxValue === other.maxValue || Math.abs(this.maxValue - other.maxValue) < 1e-10))
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

  hash(): number {
    throw new Error("not implemented");
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
    objectValue["1"] = 2502;
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
    return NumberConstraint.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
  }

  toProto(): NumberConstraintProto {
    if (this._proto === null) {
      // @ts-expect-error(readonly)
      this._proto = NumberConstraint.__packProto__(this);
    }
    return this._proto as NumberConstraintProto;
  }

  static __packProto__(object: NumberConstraint): NumberConstraintProto {
    const objectProto: Partial<NumberConstraintProto> = { metatype: 2502 };
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
    return NumberConstraint.__unpackProto__(objectProto, _session, _supergraph, _graph, _connection);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerStructClass(StructType.NUMBER_CONSTRAINT, NumberConstraint);
/* ==== DESTACK_GENERATED_END:STRUCT:2502 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:2504 ==== */
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

  hash(): number {
    throw new Error("not implemented");
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
    objectValue["1"] = 2504;
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
    return CollectionConstraint.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
  }

  toProto(): CollectionConstraintProto {
    if (this._proto === null) {
      // @ts-expect-error(readonly)
      this._proto = CollectionConstraint.__packProto__(this);
    }
    return this._proto as CollectionConstraintProto;
  }

  static __packProto__(object: CollectionConstraint): CollectionConstraintProto {
    const objectProto: Partial<CollectionConstraintProto> = { metatype: 2504 };
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
    return CollectionConstraint.__unpackProto__(objectProto, _session, _supergraph, _graph, _connection);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerStructClass(StructType.COLLECTION_CONSTRAINT, CollectionConstraint);
/* ==== DESTACK_GENERATED_END:STRUCT:2504 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:2505 ==== */
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

  hash(): number {
    throw new Error("not implemented");
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
    objectValue["1"] = 2505;
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
    const objectProto: Partial<NodeConstraintProto> = { metatype: 2505 };
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

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerStructClass(StructType.NODE_CONSTRAINT, NodeConstraint);
/* ==== DESTACK_GENERATED_END:STRUCT:2505 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:2501 ==== */
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
   * node_definition
   */
  get nodeDefinition(): CustomEntityDefinition | null {
    const nodePtr: NodeReference | null = this.nodeDefinitionPtr;
    if (nodePtr !== null) {
      if (this._supergraph === null) {
        return null;
      }
      return this._supergraph.get(nodePtr.id) as CustomEntityDefinition | null;
    }
    return null;
  }
  readonly nodeDefinitionPtr: NodeReference | null;

  /**
   * Type.structType
   */
  readonly structType: StructType | null;

  /**
   * base_type
   */
  get baseType(): Node | null {
    const nodePtr: NodeReference | null = this.baseTypePtr;
    if (nodePtr !== null) {
      if (this._supergraph === null) {
        return null;
      }
      return this._supergraph.get(nodePtr.id) as Node | null;
    }
    return null;
  }
  readonly baseTypePtr: NodeReference | null;

  /**
   * Type.keyType
   */
  readonly keyType: Type | null;

  /**
   * Type.isRequired
   */
  readonly isRequired: boolean | null;

  /**
   * Type.isVariable
   */
  readonly isVariable: boolean | null;

  /**
   * Type.defaultValue
   */
  readonly defaultValue: Value | null;

  /**
   * Type.defaultFactory
   */
  readonly defaultFactory: DefaultFactory | null;

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
    nodeDefinition?: CustomEntityDefinition | NodeReference | null;
    structType?: StructType | null;
    baseType?: Node | NodeReference | null;
    keyType?: Type | null;
    isRequired?: boolean | null;
    isVariable?: boolean | null;
    defaultValue?: Value | null;
    defaultFactory?: DefaultFactory | null;
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
      _cardinality = TypeCardinality.SCALAR;
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
    let _nodeDefinition = options.nodeDefinition ?? null;
    if (_nodeDefinition != null && _nodeDefinition instanceof Node) {
      _nodeDefinition = _nodeDefinition.toRef();
    }
    this.nodeDefinitionPtr = _nodeDefinition;
    let _structType = options.structType ?? null;
    this.structType = _structType;
    let _baseType = options.baseType ?? null;
    if (_baseType != null && _baseType instanceof Node) {
      _baseType = _baseType.toRef();
    }
    this.baseTypePtr = _baseType;
    let _keyType = options.keyType ?? null;
    this.keyType = _keyType;
    let _isRequired = options.isRequired ?? null;
    this.isRequired = _isRequired;
    let _isVariable = options.isVariable ?? null;
    this.isVariable = _isVariable;
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
    if (!(this.nodeDefinitionPtr?.id === other.nodeDefinitionPtr?.id)) {
      return false;
    }
    if (!(this.structType === other.structType)) {
      return false;
    }
    if (!(this.baseTypePtr?.id === other.baseTypePtr?.id)) {
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
    if (!(this.isVariable === other.isVariable)) {
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
      (this.collectionConstraint != null && !this.collectionConstraint.equals(other.collectionConstraint))
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

  hash(): number {
    throw new Error("not implemented");
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
    objectValue["1"] = 2501;
    objectValue["40"] = object.cardinality;
    objectValue["41"] = object.scalarType;
    if (object.primitiveType != null) {
      objectValue["42"] = object.primitiveType;
    }
    if (object.enumType != null) {
      objectValue["43"] = object.enumType;
    }
    if (object.nodeType != null) {
      objectValue["44"] = object.nodeType;
    }
    if (object.nodeDefinitionPtr != null) {
      objectValue["45"] = object.nodeDefinitionPtr.toValue();
    }
    if (object.structType != null) {
      objectValue["46"] = object.structType;
    }
    if (object.baseTypePtr != null) {
      objectValue["47"] = object.baseTypePtr.toValue();
    }
    if (object.keyType != null) {
      objectValue["48"] = object.keyType.toValue();
    }
    if (object.isRequired != null) {
      objectValue["50"] = object.isRequired;
    }
    if (object.isVariable != null) {
      objectValue["51"] = object.isVariable;
    }
    if (object.defaultValue != null) {
      objectValue["55"] = object.defaultValue.toValue();
    }
    if (object.defaultFactory != null) {
      objectValue["56"] = object.defaultFactory;
    }
    if (object.collectionConstraint != null) {
      objectValue["60"] = object.collectionConstraint.toValue();
    }
    if (object.stringConstraint != null) {
      objectValue["61"] = object.stringConstraint.toValue();
    }
    if (object.numberConstraint != null) {
      objectValue["62"] = object.numberConstraint.toValue();
    }
    if (object.nodeConstraint != null) {
      objectValue["63"] = object.nodeConstraint.toValue();
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
    const primitiveTypeValue = objectValue["42"];
    const unpackedPrimitiveType = primitiveTypeValue != undefined ? Number(primitiveTypeValue) : null;
    const enumTypeValue = objectValue["43"];
    const unpackedEnumType = enumTypeValue != undefined ? Number(enumTypeValue) : null;
    const nodeTypeValue = objectValue["44"];
    const unpackedNodeType = nodeTypeValue != undefined ? Number(nodeTypeValue) : null;
    const nodeDefinitionPtrValue = objectValue["45"];
    const unpackedNodeDefinitionPtr =
      nodeDefinitionPtrValue != undefined
        ? NodeReference.fromValue(nodeDefinitionPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const structTypeValue = objectValue["46"];
    const unpackedStructType = structTypeValue != undefined ? Number(structTypeValue) : null;
    const baseTypePtrValue = objectValue["47"];
    const unpackedBaseTypePtr =
      baseTypePtrValue != undefined
        ? NodeReference.fromValue(baseTypePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const keyTypeValue = objectValue["48"];
    const unpackedKeyType =
      keyTypeValue != undefined ? Type.fromValue(keyTypeValue, _session, _supergraph, _graph, _connection) : null;
    const isRequiredValue = objectValue["50"];
    const unpackedIsRequired = isRequiredValue != undefined ? isRequiredValue : null;
    const isVariableValue = objectValue["51"];
    const unpackedIsVariable = isVariableValue != undefined ? isVariableValue : null;
    const defaultValueValue = objectValue["55"];
    const unpackedDefaultValue =
      defaultValueValue != undefined
        ? Value.fromValue(defaultValueValue, _session, _supergraph, _graph, _connection)
        : null;
    const defaultFactoryValue = objectValue["56"];
    const unpackedDefaultFactory = defaultFactoryValue != undefined ? Number(defaultFactoryValue) : null;
    const collectionConstraintValue = objectValue["60"];
    const unpackedCollectionConstraint =
      collectionConstraintValue != undefined
        ? CollectionConstraint.fromValue(collectionConstraintValue, _session, _supergraph, _graph, _connection)
        : null;
    const stringConstraintValue = objectValue["61"];
    const unpackedStringConstraint =
      stringConstraintValue != undefined
        ? StringConstraint.fromValue(stringConstraintValue, _session, _supergraph, _graph, _connection)
        : null;
    const numberConstraintValue = objectValue["62"];
    const unpackedNumberConstraint =
      numberConstraintValue != undefined
        ? NumberConstraint.fromValue(numberConstraintValue, _session, _supergraph, _graph, _connection)
        : null;
    const nodeConstraintValue = objectValue["63"];
    const unpackedNodeConstraint =
      nodeConstraintValue != undefined
        ? NodeConstraint.fromValue(nodeConstraintValue, _session, _supergraph, _graph, _connection)
        : null;
    return new Type({
      cardinality: Number(objectValue["40"]),
      scalarType: Number(objectValue["41"]),
      primitiveType: unpackedPrimitiveType,
      enumType: unpackedEnumType,
      nodeType: unpackedNodeType,
      nodeDefinition: unpackedNodeDefinitionPtr,
      structType: unpackedStructType,
      baseType: unpackedBaseTypePtr,
      keyType: unpackedKeyType,
      isRequired: unpackedIsRequired,
      isVariable: unpackedIsVariable,
      defaultValue: unpackedDefaultValue,
      defaultFactory: unpackedDefaultFactory,
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
    const objectProto: Partial<TypeProto> = { metatype: 2501 };
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
    if (object.nodeDefinitionPtr != null) {
      objectProto.nodeDefinitionPtr = object.nodeDefinitionPtr.toProto();
    }
    if (object.structType != null) {
      objectProto.structType = Number(object.structType) as StructTypeProto;
    }
    if (object.baseTypePtr != null) {
      objectProto.baseTypePtr = object.baseTypePtr.toProto();
    }
    if (object.keyType != null) {
      objectProto.keyType = object.keyType.toProto();
    }
    if (object.isRequired != null) {
      objectProto.isRequired = object.isRequired;
    }
    if (object.isVariable != null) {
      objectProto.isVariable = object.isVariable;
    }
    if (object.defaultValue != null) {
      objectProto.defaultValue = object.defaultValue.toProto();
    }
    if (object.defaultFactory != null) {
      objectProto.defaultFactory = Number(object.defaultFactory) as DefaultFactoryProto;
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
    return new Type({
      cardinality: Number(objectProto.cardinality) as TypeCardinality,
      scalarType: Number(objectProto.scalarType) as ScalarType,
      primitiveType:
        objectProto.primitiveType != undefined ? (Number(objectProto.primitiveType) as PrimitiveType) : null,
      enumType: objectProto.enumType != undefined ? (Number(objectProto.enumType) as EnumType) : null,
      nodeType: objectProto.nodeType != undefined ? (Number(objectProto.nodeType) as NodeType) : null,
      nodeDefinition:
        objectProto.nodeDefinitionPtr != undefined
          ? NodeReference.fromProto(objectProto.nodeDefinitionPtr!, _session, _supergraph, _graph, _connection)
          : null,
      structType: objectProto.structType != undefined ? (Number(objectProto.structType) as StructType) : null,
      baseType:
        objectProto.baseTypePtr != undefined
          ? NodeReference.fromProto(objectProto.baseTypePtr!, _session, _supergraph, _graph, _connection)
          : null,
      keyType:
        objectProto.keyType != undefined
          ? Type.fromProto(objectProto.keyType!, _session, _supergraph, _graph, _connection)
          : null,
      isRequired: objectProto.isRequired != undefined ? objectProto.isRequired : null,
      isVariable: objectProto.isVariable != undefined ? objectProto.isVariable : null,
      defaultValue:
        objectProto.defaultValue != undefined
          ? Value.fromProto(objectProto.defaultValue!, _session, _supergraph, _graph, _connection)
          : null,
      defaultFactory:
        objectProto.defaultFactory != undefined ? (Number(objectProto.defaultFactory) as DefaultFactory) : null,
      collectionConstraint:
        objectProto.collectionConstraint != undefined
          ? CollectionConstraint.fromProto(
              objectProto.collectionConstraint!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      stringConstraint:
        objectProto.stringConstraint != undefined
          ? StringConstraint.fromProto(objectProto.stringConstraint!, _session, _supergraph, _graph, _connection)
          : null,
      numberConstraint:
        objectProto.numberConstraint != undefined
          ? NumberConstraint.fromProto(objectProto.numberConstraint!, _session, _supergraph, _graph, _connection)
          : null,
      nodeConstraint:
        objectProto.nodeConstraint != undefined
          ? NodeConstraint.fromProto(objectProto.nodeConstraint!, _session, _supergraph, _graph, _connection)
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

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerStructClass(StructType.TYPE, Type);
/* ==== DESTACK_GENERATED_END:STRUCT:2501 ==== */

/**
 * Guess the type of a value or class.
 */
export function toType(valueOrType: any, nodeAsValue: boolean = false): Type {
  if (valueOrType === null || valueOrType === undefined) {
    throw new Error("null/undefined is not a valid Type");
  }

  // scalar values
  if (valueOrType instanceof NodeReference) {
    return new Type({
      cardinality: TypeCardinality.SCALAR,
      scalarType: ScalarType.NODE_REFERENCE,
      nodeType: valueOrType.nodeType,
    });
  } else if (valueOrType instanceof Node) {
    return new Type({
      cardinality: TypeCardinality.SCALAR,
      scalarType: nodeAsValue ? ScalarType.NODE_VALUE : ScalarType.NODE_REFERENCE,
      nodeType: valueOrType.metatype,
    });
  } else if (valueOrType instanceof Struct) {
    return new Type({
      cardinality: TypeCardinality.SCALAR,
      scalarType: ScalarType.STRUCT,
      structType: valueOrType.metatype,
    });
  } else if (PRIMITIVE_JS_TYPES.has(valueOrType.constructor) && valueOrType.constructor !== Object) {
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
      throw new Error(`expected scalar inside array, got ${elementType.cardinality} for ${valueOrType}`);
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
    if (valueType.cardinality !== TypeCardinality.SCALAR && valueType.cardinality !== TypeCardinality.LIST) {
      throw new Error(`expected scalar or list value in Map, got ${valueType.cardinality} for ${valueOrType}`);
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
      throw new Error(`expected scalar key in object, got ${keyType.cardinality} for ${valueOrType}`);
    }
    const valueType = toType(sampleValue);
    if (valueType.cardinality !== TypeCardinality.SCALAR && valueType.cardinality !== TypeCardinality.LIST) {
      throw new Error(`expected scalar or list value in object, got ${valueType.cardinality} for ${valueOrType}`);
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
