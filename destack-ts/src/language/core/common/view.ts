import { EnumType, Session, StructType, Supergraph } from "@destack/language/core";
import { StructFrozen } from "@destack/language/core/builtin";
import { registerEnumClass, registerStructClass } from "@destack/language/registry";
import {
  Axis2Proto,
  Axis3Proto,
  CornersProto,
  DimensionProto,
  DimensionTypeProto,
  GridProto,
  GridSpanProto,
  InsetsProto,
  LengthProto,
  LengthUnitProto,
  PositionProto,
  PositionTypeProto,
  Vector2Proto,
  Vector2iProto,
  Vector3Proto,
  Vector3iProto,
  Vector4Proto,
  Vector4iProto,
} from "@destack/proto";

/* ==== DESTACK_GENERATED_START:ENUM:12038 ==== */
/**
 * Layout
 */
export enum Layout {
  STACK = 1,
  GRID = 2,

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerEnumClass(EnumType.LAYOUT, Layout);
/* ==== DESTACK_GENERATED_END:ENUM:12038 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:12042 ==== */
/**
 * Overflow
 */
export enum Overflow {
  HIDDEN = 2,
  VISIBLE = 3,
  SCROLL = 4,

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerEnumClass(EnumType.OVERFLOW, Overflow);
/* ==== DESTACK_GENERATED_END:ENUM:12042 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:12041 ==== */
/**
 * Direction
 */
export enum Direction {
  HORIZONTAL = 1,
  VERTICAL = 2,

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerEnumClass(EnumType.DIRECTION, Direction);
/* ==== DESTACK_GENERATED_END:ENUM:12041 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:12039 ==== */
/**
 * Distribute
 */
export enum Distribute {
  START = 1,
  CENTER = 2,
  END = 3,
  SPACE_BETWEEN = 4,
  SPACE_AROUND = 5,
  SPACE_EVENLY = 6,

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerEnumClass(EnumType.DISTRIBUTE, Distribute);
/* ==== DESTACK_GENERATED_END:ENUM:12039 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:12040 ==== */
/**
 * Align
 */
export enum Align {
  START = 1,
  CENTER = 2,
  END = 3,

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerEnumClass(EnumType.ALIGN, Align);
/* ==== DESTACK_GENERATED_END:ENUM:12040 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:12037 ==== */
/**
 * LengthUnit
 */
export enum LengthUnit {
  PIXEL = 1,
  REM = 2,
  PERCENT = 3,
  FR = 4,

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerEnumClass(EnumType.LENGTH_UNIT, LengthUnit);
/* ==== DESTACK_GENERATED_END:ENUM:12037 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:12000 ==== */
/**
 * PositionType
 */
export enum PositionType {
  RELATIVE = 1,
  ABSOLUTE = 2,
  FIXED = 3,
  STICKY = 4,

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerEnumClass(EnumType.POSITION_TYPE, PositionType);
/* ==== DESTACK_GENERATED_END:ENUM:12000 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:12045 ==== */
/**
 * DimensionType
 */
export enum DimensionType {
  FIXED = 2,
  FIT = 3,
  FILL = 4,

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerEnumClass(EnumType.DIMENSION_TYPE, DimensionType);
/* ==== DESTACK_GENERATED_END:ENUM:12045 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:12018 ==== */
/**
 * A length value.
 */
export class Length extends StructFrozen {
  static metatype: StructType = StructType.LENGTH;
  static __isFrozen__: boolean = true;

  /**
   * Length.unit
   */
  readonly unit: LengthUnit;

  /**
   * Length.value
   */
  readonly value: number;

  constructor(options: {
    unit: LengthUnit;
    value: number;
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
    let _unit = options.unit;
    if (_unit === null) {
      throw new Error(`Length.unit is required`);
    }
    this.unit = _unit;
    let _value = options.value;
    if (_value === null) {
      throw new Error(`Length.value is required`);
    }
    this.value = _value;

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
    if (!(this.unit === other.unit)) {
      return false;
    }
    if (!(this.value === other.value || Math.abs(this.value - other.value) < 1e-10)) {
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
      this._value = Length.__packValue__(this);
    }
    return this._value;
  }

  static __packValue__(object: Length): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 12018;
    objectValue["50"] = object.unit;
    objectValue["51"] = object.value;
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Length {
    return new Length({
      unit: Number(objectValue["50"]),
      value: objectValue["51"],
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
  ): Length {
    return Length.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
  }

  toProto(): LengthProto {
    if (this._proto === null) {
      // @ts-expect-error(readonly)
      this._proto = Length.__packProto__(this);
    }
    return this._proto as LengthProto;
  }

  static __packProto__(object: Length): LengthProto {
    const objectProto: Partial<LengthProto> = { metatype: 12018 };
    objectProto.unit = Number(object.unit) as LengthUnitProto;
    objectProto.value = object.value;
    return objectProto as LengthProto;
  }

  static __unpackProto__(
    objectProto: LengthProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Length {
    return new Length({
      unit: Number(objectProto.unit) as LengthUnit,
      value: objectProto.value,
      _proto: objectProto,
      _supergraph,
    });
  }

  static fromProto(
    objectProto: LengthProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Length {
    return Length.__unpackProto__(objectProto, _session, _supergraph, _graph, _connection);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerStructClass(StructType.LENGTH, Length);
/* ==== DESTACK_GENERATED_END:STRUCT:12018 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:12020 ==== */
/**
 * A position value.
 */
export class Position extends StructFrozen {
  static metatype: StructType = StructType.POSITION;
  static __isFrozen__: boolean = true;

  /**
   * Position.type
   */
  readonly type: PositionType;

  /**
   * Position.top
   */
  readonly top: Length | null;

  /**
   * Position.left
   */
  readonly left: Length | null;

  /**
   * Position.width
   */
  readonly width: Length | null;

  /**
   * Position.height
   */
  readonly height: Length | null;

  constructor(options: {
    type: PositionType;
    top?: Length | null;
    left?: Length | null;
    width?: Length | null;
    height?: Length | null;
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
      throw new Error(`Position.type is required`);
    }
    this.type = _type;
    let _top = options.top ?? null;
    this.top = _top;
    let _left = options.left ?? null;
    this.left = _left;
    let _width = options.width ?? null;
    this.width = _width;
    let _height = options.height ?? null;
    this.height = _height;

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
    if ((this.top == null) !== (other.top == null) || (this.top != null && !this.top.equals(other.top))) {
      return false;
    }
    if ((this.left == null) !== (other.left == null) || (this.left != null && !this.left.equals(other.left))) {
      return false;
    }
    if ((this.width == null) !== (other.width == null) || (this.width != null && !this.width.equals(other.width))) {
      return false;
    }
    if (
      (this.height == null) !== (other.height == null) ||
      (this.height != null && !this.height.equals(other.height))
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
      this._value = Position.__packValue__(this);
    }
    return this._value;
  }

  static __packValue__(object: Position): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 12020;
    objectValue["30"] = object.type;
    if (object.top != null) {
      objectValue["50"] = object.top.toValue();
    }
    if (object.left != null) {
      objectValue["51"] = object.left.toValue();
    }
    if (object.width != null) {
      objectValue["52"] = object.width.toValue();
    }
    if (object.height != null) {
      objectValue["53"] = object.height.toValue();
    }
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Position {
    const topValue = objectValue["50"];
    const unpackedTop =
      topValue != undefined ? Length.fromValue(topValue, _session, _supergraph, _graph, _connection) : null;
    const leftValue = objectValue["51"];
    const unpackedLeft =
      leftValue != undefined ? Length.fromValue(leftValue, _session, _supergraph, _graph, _connection) : null;
    const widthValue = objectValue["52"];
    const unpackedWidth =
      widthValue != undefined ? Length.fromValue(widthValue, _session, _supergraph, _graph, _connection) : null;
    const heightValue = objectValue["53"];
    const unpackedHeight =
      heightValue != undefined ? Length.fromValue(heightValue, _session, _supergraph, _graph, _connection) : null;
    return new Position({
      type: Number(objectValue["30"]),
      top: unpackedTop,
      left: unpackedLeft,
      width: unpackedWidth,
      height: unpackedHeight,
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
  ): Position {
    return Position.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
  }

  toProto(): PositionProto {
    if (this._proto === null) {
      // @ts-expect-error(readonly)
      this._proto = Position.__packProto__(this);
    }
    return this._proto as PositionProto;
  }

  static __packProto__(object: Position): PositionProto {
    const objectProto: Partial<PositionProto> = { metatype: 12020 };
    objectProto.type = Number(object.type) as PositionTypeProto;
    if (object.top != null) {
      objectProto.top = object.top.toProto();
    }
    if (object.left != null) {
      objectProto.left = object.left.toProto();
    }
    if (object.width != null) {
      objectProto.width = object.width.toProto();
    }
    if (object.height != null) {
      objectProto.height = object.height.toProto();
    }
    return objectProto as PositionProto;
  }

  static __unpackProto__(
    objectProto: PositionProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Position {
    return new Position({
      type: Number(objectProto.type) as PositionType,
      top:
        objectProto.top != undefined
          ? Length.fromProto(objectProto.top!, _session, _supergraph, _graph, _connection)
          : null,
      left:
        objectProto.left != undefined
          ? Length.fromProto(objectProto.left!, _session, _supergraph, _graph, _connection)
          : null,
      width:
        objectProto.width != undefined
          ? Length.fromProto(objectProto.width!, _session, _supergraph, _graph, _connection)
          : null,
      height:
        objectProto.height != undefined
          ? Length.fromProto(objectProto.height!, _session, _supergraph, _graph, _connection)
          : null,
      _proto: objectProto,
      _supergraph,
    });
  }

  static fromProto(
    objectProto: PositionProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Position {
    return Position.__unpackProto__(objectProto, _session, _supergraph, _graph, _connection);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerStructClass(StructType.POSITION, Position);
/* ==== DESTACK_GENERATED_END:STRUCT:12020 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:12022 ==== */
/**
 * A dimension value (like Length but can fit or fill container).
 */
export class Dimension extends StructFrozen {
  static metatype: StructType = StructType.DIMENSION;
  static __isFrozen__: boolean = true;

  /**
   * Dimension.type
   */
  readonly type: DimensionType;

  /**
   * Dimension.unit
   */
  readonly unit: LengthUnit;

  /**
   * Dimension.value
   */
  readonly value: number;

  constructor(options: {
    type: DimensionType;
    unit: LengthUnit;
    value: number;
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
      throw new Error(`Dimension.type is required`);
    }
    this.type = _type;
    let _unit = options.unit;
    if (_unit === null) {
      throw new Error(`Dimension.unit is required`);
    }
    this.unit = _unit;
    let _value = options.value;
    if (_value === null) {
      throw new Error(`Dimension.value is required`);
    }
    this.value = _value;

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
    if (!(this.unit === other.unit)) {
      return false;
    }
    if (!(this.value === other.value || Math.abs(this.value - other.value) < 1e-10)) {
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
      this._value = Dimension.__packValue__(this);
    }
    return this._value;
  }

  static __packValue__(object: Dimension): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 12022;
    objectValue["30"] = object.type;
    objectValue["50"] = object.unit;
    objectValue["51"] = object.value;
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Dimension {
    return new Dimension({
      type: Number(objectValue["30"]),
      unit: Number(objectValue["50"]),
      value: objectValue["51"],
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
  ): Dimension {
    return Dimension.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
  }

  toProto(): DimensionProto {
    if (this._proto === null) {
      // @ts-expect-error(readonly)
      this._proto = Dimension.__packProto__(this);
    }
    return this._proto as DimensionProto;
  }

  static __packProto__(object: Dimension): DimensionProto {
    const objectProto: Partial<DimensionProto> = { metatype: 12022 };
    objectProto.type = Number(object.type) as DimensionTypeProto;
    objectProto.unit = Number(object.unit) as LengthUnitProto;
    objectProto.value = object.value;
    return objectProto as DimensionProto;
  }

  static __unpackProto__(
    objectProto: DimensionProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Dimension {
    return new Dimension({
      type: Number(objectProto.type) as DimensionType,
      unit: Number(objectProto.unit) as LengthUnit,
      value: objectProto.value,
      _proto: objectProto,
      _supergraph,
    });
  }

  static fromProto(
    objectProto: DimensionProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Dimension {
    return Dimension.__unpackProto__(objectProto, _session, _supergraph, _graph, _connection);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerStructClass(StructType.DIMENSION, Dimension);
/* ==== DESTACK_GENERATED_END:STRUCT:12022 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:12030 ==== */
/**
 * An insets value (base + side overrides).
 */
export class Insets extends StructFrozen {
  static metatype: StructType = StructType.INSETS;
  static __isFrozen__: boolean = true;

  /**
   * Insets.base
   */
  readonly base: number | null;

  /**
   * Insets.top
   */
  readonly top: number | null;

  /**
   * Insets.left
   */
  readonly left: number | null;

  /**
   * Insets.right
   */
  readonly right: number | null;

  /**
   * Insets.bottom
   */
  readonly bottom: number | null;

  constructor(options: {
    base?: number | null;
    top?: number | null;
    left?: number | null;
    right?: number | null;
    bottom?: number | null;
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
    let _base = options.base ?? null;
    this.base = _base;
    let _top = options.top ?? null;
    this.top = _top;
    let _left = options.left ?? null;
    this.left = _left;
    let _right = options.right ?? null;
    this.right = _right;
    let _bottom = options.bottom ?? null;
    this.bottom = _bottom;

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
    if (!(this.base === other.base)) {
      return false;
    }
    if (!(this.top === other.top)) {
      return false;
    }
    if (!(this.left === other.left)) {
      return false;
    }
    if (!(this.right === other.right)) {
      return false;
    }
    if (!(this.bottom === other.bottom)) {
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
      this._value = Insets.__packValue__(this);
    }
    return this._value;
  }

  static __packValue__(object: Insets): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 12030;
    if (object.base != null) {
      objectValue["50"] = object.base;
    }
    if (object.top != null) {
      objectValue["51"] = object.top;
    }
    if (object.left != null) {
      objectValue["52"] = object.left;
    }
    if (object.right != null) {
      objectValue["53"] = object.right;
    }
    if (object.bottom != null) {
      objectValue["54"] = object.bottom;
    }
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Insets {
    const baseValue = objectValue["50"];
    const unpackedBase = baseValue != undefined ? Number(baseValue) : null;
    const topValue = objectValue["51"];
    const unpackedTop = topValue != undefined ? Number(topValue) : null;
    const leftValue = objectValue["52"];
    const unpackedLeft = leftValue != undefined ? Number(leftValue) : null;
    const rightValue = objectValue["53"];
    const unpackedRight = rightValue != undefined ? Number(rightValue) : null;
    const bottomValue = objectValue["54"];
    const unpackedBottom = bottomValue != undefined ? Number(bottomValue) : null;
    return new Insets({
      base: unpackedBase,
      top: unpackedTop,
      left: unpackedLeft,
      right: unpackedRight,
      bottom: unpackedBottom,
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
  ): Insets {
    return Insets.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
  }

  toProto(): InsetsProto {
    if (this._proto === null) {
      // @ts-expect-error(readonly)
      this._proto = Insets.__packProto__(this);
    }
    return this._proto as InsetsProto;
  }

  static __packProto__(object: Insets): InsetsProto {
    const objectProto: Partial<InsetsProto> = { metatype: 12030 };
    if (object.base != null) {
      objectProto.base = object.base;
    }
    if (object.top != null) {
      objectProto.top = object.top;
    }
    if (object.left != null) {
      objectProto.left = object.left;
    }
    if (object.right != null) {
      objectProto.right = object.right;
    }
    if (object.bottom != null) {
      objectProto.bottom = object.bottom;
    }
    return objectProto as InsetsProto;
  }

  static __unpackProto__(
    objectProto: InsetsProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Insets {
    return new Insets({
      base: objectProto.base != undefined ? Number(objectProto.base) : null,
      top: objectProto.top != undefined ? Number(objectProto.top) : null,
      left: objectProto.left != undefined ? Number(objectProto.left) : null,
      right: objectProto.right != undefined ? Number(objectProto.right) : null,
      bottom: objectProto.bottom != undefined ? Number(objectProto.bottom) : null,
      _proto: objectProto,
      _supergraph,
    });
  }

  static fromProto(
    objectProto: InsetsProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Insets {
    return Insets.__unpackProto__(objectProto, _session, _supergraph, _graph, _connection);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerStructClass(StructType.INSETS, Insets);
/* ==== DESTACK_GENERATED_END:STRUCT:12030 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:12032 ==== */
/**
 * A corners value (base + corner overrides).
 */
export class Corners extends StructFrozen {
  static metatype: StructType = StructType.CORNERS;
  static __isFrozen__: boolean = true;

  /**
   * Corners.base
   */
  readonly base: number | null;

  /**
   * Corners.topLeft
   */
  readonly topLeft: number | null;

  /**
   * Corners.topRight
   */
  readonly topRight: number | null;

  /**
   * Corners.bottomLeft
   */
  readonly bottomLeft: number | null;

  /**
   * Corners.bottomRight
   */
  readonly bottomRight: number | null;

  constructor(options: {
    base?: number | null;
    topLeft?: number | null;
    topRight?: number | null;
    bottomLeft?: number | null;
    bottomRight?: number | null;
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
    let _base = options.base ?? null;
    this.base = _base;
    let _topLeft = options.topLeft ?? null;
    this.topLeft = _topLeft;
    let _topRight = options.topRight ?? null;
    this.topRight = _topRight;
    let _bottomLeft = options.bottomLeft ?? null;
    this.bottomLeft = _bottomLeft;
    let _bottomRight = options.bottomRight ?? null;
    this.bottomRight = _bottomRight;

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
    if (!(this.base === other.base)) {
      return false;
    }
    if (!(this.topLeft === other.topLeft)) {
      return false;
    }
    if (!(this.topRight === other.topRight)) {
      return false;
    }
    if (!(this.bottomLeft === other.bottomLeft)) {
      return false;
    }
    if (!(this.bottomRight === other.bottomRight)) {
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
      this._value = Corners.__packValue__(this);
    }
    return this._value;
  }

  static __packValue__(object: Corners): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 12032;
    if (object.base != null) {
      objectValue["50"] = object.base;
    }
    if (object.topLeft != null) {
      objectValue["51"] = object.topLeft;
    }
    if (object.topRight != null) {
      objectValue["52"] = object.topRight;
    }
    if (object.bottomLeft != null) {
      objectValue["53"] = object.bottomLeft;
    }
    if (object.bottomRight != null) {
      objectValue["54"] = object.bottomRight;
    }
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Corners {
    const baseValue = objectValue["50"];
    const unpackedBase = baseValue != undefined ? Number(baseValue) : null;
    const topLeftValue = objectValue["51"];
    const unpackedTopLeft = topLeftValue != undefined ? Number(topLeftValue) : null;
    const topRightValue = objectValue["52"];
    const unpackedTopRight = topRightValue != undefined ? Number(topRightValue) : null;
    const bottomLeftValue = objectValue["53"];
    const unpackedBottomLeft = bottomLeftValue != undefined ? Number(bottomLeftValue) : null;
    const bottomRightValue = objectValue["54"];
    const unpackedBottomRight = bottomRightValue != undefined ? Number(bottomRightValue) : null;
    return new Corners({
      base: unpackedBase,
      topLeft: unpackedTopLeft,
      topRight: unpackedTopRight,
      bottomLeft: unpackedBottomLeft,
      bottomRight: unpackedBottomRight,
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
  ): Corners {
    return Corners.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
  }

  toProto(): CornersProto {
    if (this._proto === null) {
      // @ts-expect-error(readonly)
      this._proto = Corners.__packProto__(this);
    }
    return this._proto as CornersProto;
  }

  static __packProto__(object: Corners): CornersProto {
    const objectProto: Partial<CornersProto> = { metatype: 12032 };
    if (object.base != null) {
      objectProto.base = object.base;
    }
    if (object.topLeft != null) {
      objectProto.topLeft = object.topLeft;
    }
    if (object.topRight != null) {
      objectProto.topRight = object.topRight;
    }
    if (object.bottomLeft != null) {
      objectProto.bottomLeft = object.bottomLeft;
    }
    if (object.bottomRight != null) {
      objectProto.bottomRight = object.bottomRight;
    }
    return objectProto as CornersProto;
  }

  static __unpackProto__(
    objectProto: CornersProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Corners {
    return new Corners({
      base: objectProto.base != undefined ? Number(objectProto.base) : null,
      topLeft: objectProto.topLeft != undefined ? Number(objectProto.topLeft) : null,
      topRight: objectProto.topRight != undefined ? Number(objectProto.topRight) : null,
      bottomLeft: objectProto.bottomLeft != undefined ? Number(objectProto.bottomLeft) : null,
      bottomRight: objectProto.bottomRight != undefined ? Number(objectProto.bottomRight) : null,
      _proto: objectProto,
      _supergraph,
    });
  }

  static fromProto(
    objectProto: CornersProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Corners {
    return Corners.__unpackProto__(objectProto, _session, _supergraph, _graph, _connection);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerStructClass(StructType.CORNERS, Corners);
/* ==== DESTACK_GENERATED_END:STRUCT:12032 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:50207 ==== */
/**
 * A gap value (base + x/y overrides).
 */
export class Axis2 extends StructFrozen {
  static metatype: StructType = StructType.AXIS2;
  static __isFrozen__: boolean = true;

  /**
   * Axis2.base
   */
  readonly base: number | null;

  /**
   * Axis2.x
   */
  readonly x: number | null;

  /**
   * Axis2.y
   */
  readonly y: number | null;

  constructor(options: {
    base?: number | null;
    x?: number | null;
    y?: number | null;
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
    let _base = options.base ?? null;
    this.base = _base;
    let _x = options.x ?? null;
    this.x = _x;
    let _y = options.y ?? null;
    this.y = _y;

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
    if (
      (this.base == null) !== (other.base == null) ||
      (this.base != null && !(this.base === other.base || Math.abs(this.base - other.base) < 1e-10))
    ) {
      return false;
    }
    if (
      (this.x == null) !== (other.x == null) ||
      (this.x != null && !(this.x === other.x || Math.abs(this.x - other.x) < 1e-10))
    ) {
      return false;
    }
    if (
      (this.y == null) !== (other.y == null) ||
      (this.y != null && !(this.y === other.y || Math.abs(this.y - other.y) < 1e-10))
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
      this._value = Axis2.__packValue__(this);
    }
    return this._value;
  }

  static __packValue__(object: Axis2): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 50207;
    if (object.base != null) {
      objectValue["50"] = object.base;
    }
    if (object.x != null) {
      objectValue["51"] = object.x;
    }
    if (object.y != null) {
      objectValue["52"] = object.y;
    }
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Axis2 {
    const baseValue = objectValue["50"];
    const unpackedBase = baseValue != undefined ? baseValue : null;
    const xValue = objectValue["51"];
    const unpackedX = xValue != undefined ? xValue : null;
    const yValue = objectValue["52"];
    const unpackedY = yValue != undefined ? yValue : null;
    return new Axis2({
      base: unpackedBase,
      x: unpackedX,
      y: unpackedY,
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
  ): Axis2 {
    return Axis2.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
  }

  toProto(): Axis2Proto {
    if (this._proto === null) {
      // @ts-expect-error(readonly)
      this._proto = Axis2.__packProto__(this);
    }
    return this._proto as Axis2Proto;
  }

  static __packProto__(object: Axis2): Axis2Proto {
    const objectProto: Partial<Axis2Proto> = { metatype: 50207 };
    if (object.base != null) {
      objectProto.base = object.base;
    }
    if (object.x != null) {
      objectProto.x = object.x;
    }
    if (object.y != null) {
      objectProto.y = object.y;
    }
    return objectProto as Axis2Proto;
  }

  static __unpackProto__(
    objectProto: Axis2Proto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Axis2 {
    return new Axis2({
      base: objectProto.base != undefined ? objectProto.base : null,
      x: objectProto.x != undefined ? objectProto.x : null,
      y: objectProto.y != undefined ? objectProto.y : null,
      _proto: objectProto,
      _supergraph,
    });
  }

  static fromProto(
    objectProto: Axis2Proto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Axis2 {
    return Axis2.__unpackProto__(objectProto, _session, _supergraph, _graph, _connection);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerStructClass(StructType.AXIS2, Axis2);
/* ==== DESTACK_GENERATED_END:STRUCT:50207 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:50209 ==== */
/**
 * A rotation value (base + x/y/z overrides).
 */
export class Axis3 extends StructFrozen {
  static metatype: StructType = StructType.AXIS3;
  static __isFrozen__: boolean = true;

  /**
   * Axis3.base
   */
  readonly base: number | null;

  /**
   * Axis3.x
   */
  readonly x: number | null;

  /**
   * Axis3.y
   */
  readonly y: number | null;

  /**
   * Axis3.z
   */
  readonly z: number | null;

  constructor(options: {
    base?: number | null;
    x?: number | null;
    y?: number | null;
    z?: number | null;
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
    let _base = options.base ?? null;
    this.base = _base;
    let _x = options.x ?? null;
    this.x = _x;
    let _y = options.y ?? null;
    this.y = _y;
    let _z = options.z ?? null;
    this.z = _z;

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
    if (
      (this.base == null) !== (other.base == null) ||
      (this.base != null && !(this.base === other.base || Math.abs(this.base - other.base) < 1e-10))
    ) {
      return false;
    }
    if (
      (this.x == null) !== (other.x == null) ||
      (this.x != null && !(this.x === other.x || Math.abs(this.x - other.x) < 1e-10))
    ) {
      return false;
    }
    if (
      (this.y == null) !== (other.y == null) ||
      (this.y != null && !(this.y === other.y || Math.abs(this.y - other.y) < 1e-10))
    ) {
      return false;
    }
    if (
      (this.z == null) !== (other.z == null) ||
      (this.z != null && !(this.z === other.z || Math.abs(this.z - other.z) < 1e-10))
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
      this._value = Axis3.__packValue__(this);
    }
    return this._value;
  }

  static __packValue__(object: Axis3): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 50209;
    if (object.base != null) {
      objectValue["50"] = object.base;
    }
    if (object.x != null) {
      objectValue["51"] = object.x;
    }
    if (object.y != null) {
      objectValue["52"] = object.y;
    }
    if (object.z != null) {
      objectValue["53"] = object.z;
    }
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Axis3 {
    const baseValue = objectValue["50"];
    const unpackedBase = baseValue != undefined ? baseValue : null;
    const xValue = objectValue["51"];
    const unpackedX = xValue != undefined ? xValue : null;
    const yValue = objectValue["52"];
    const unpackedY = yValue != undefined ? yValue : null;
    const zValue = objectValue["53"];
    const unpackedZ = zValue != undefined ? zValue : null;
    return new Axis3({
      base: unpackedBase,
      x: unpackedX,
      y: unpackedY,
      z: unpackedZ,
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
  ): Axis3 {
    return Axis3.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
  }

  toProto(): Axis3Proto {
    if (this._proto === null) {
      // @ts-expect-error(readonly)
      this._proto = Axis3.__packProto__(this);
    }
    return this._proto as Axis3Proto;
  }

  static __packProto__(object: Axis3): Axis3Proto {
    const objectProto: Partial<Axis3Proto> = { metatype: 50209 };
    if (object.base != null) {
      objectProto.base = object.base;
    }
    if (object.x != null) {
      objectProto.x = object.x;
    }
    if (object.y != null) {
      objectProto.y = object.y;
    }
    if (object.z != null) {
      objectProto.z = object.z;
    }
    return objectProto as Axis3Proto;
  }

  static __unpackProto__(
    objectProto: Axis3Proto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Axis3 {
    return new Axis3({
      base: objectProto.base != undefined ? objectProto.base : null,
      x: objectProto.x != undefined ? objectProto.x : null,
      y: objectProto.y != undefined ? objectProto.y : null,
      z: objectProto.z != undefined ? objectProto.z : null,
      _proto: objectProto,
      _supergraph,
    });
  }

  static fromProto(
    objectProto: Axis3Proto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Axis3 {
    return Axis3.__unpackProto__(objectProto, _session, _supergraph, _graph, _connection);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerStructClass(StructType.AXIS3, Axis3);
/* ==== DESTACK_GENERATED_END:STRUCT:50209 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:50200 ==== */
/**
 * A 2D float vector.
 */
export class Vector2 extends StructFrozen {
  static metatype: StructType = StructType.VECTOR2;
  static __isFrozen__: boolean = true;

  /**
   * Vector2.x
   */
  readonly x: number;

  /**
   * Vector2.y
   */
  readonly y: number;

  constructor(options: {
    x: number;
    y: number;
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
    let _x = options.x;
    if (_x === null) {
      throw new Error(`Vector2.x is required`);
    }
    this.x = _x;
    let _y = options.y;
    if (_y === null) {
      throw new Error(`Vector2.y is required`);
    }
    this.y = _y;

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
    if (!(this.x === other.x || Math.abs(this.x - other.x) < 1e-10)) {
      return false;
    }
    if (!(this.y === other.y || Math.abs(this.y - other.y) < 1e-10)) {
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
      this._value = Vector2.__packValue__(this);
    }
    return this._value;
  }

  static __packValue__(object: Vector2): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 50200;
    objectValue["50"] = object.x;
    objectValue["51"] = object.y;
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Vector2 {
    return new Vector2({
      x: objectValue["50"],
      y: objectValue["51"],
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
  ): Vector2 {
    return Vector2.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
  }

  toProto(): Vector2Proto {
    if (this._proto === null) {
      // @ts-expect-error(readonly)
      this._proto = Vector2.__packProto__(this);
    }
    return this._proto as Vector2Proto;
  }

  static __packProto__(object: Vector2): Vector2Proto {
    const objectProto: Partial<Vector2Proto> = { metatype: 50200 };
    objectProto.x = object.x;
    objectProto.y = object.y;
    return objectProto as Vector2Proto;
  }

  static __unpackProto__(
    objectProto: Vector2Proto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Vector2 {
    return new Vector2({
      x: objectProto.x,
      y: objectProto.y,
      _proto: objectProto,
      _supergraph,
    });
  }

  static fromProto(
    objectProto: Vector2Proto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Vector2 {
    return Vector2.__unpackProto__(objectProto, _session, _supergraph, _graph, _connection);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerStructClass(StructType.VECTOR2, Vector2);
/* ==== DESTACK_GENERATED_END:STRUCT:50200 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:50201 ==== */
/**
 * A 3D float vector.
 */
export class Vector3 extends StructFrozen {
  static metatype: StructType = StructType.VECTOR3;
  static __isFrozen__: boolean = true;

  /**
   * Vector3.x
   */
  readonly x: number;

  /**
   * Vector3.y
   */
  readonly y: number;

  /**
   * Vector3.z
   */
  readonly z: number;

  constructor(options: {
    x: number;
    y: number;
    z: number;
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
    let _x = options.x;
    if (_x === null) {
      throw new Error(`Vector3.x is required`);
    }
    this.x = _x;
    let _y = options.y;
    if (_y === null) {
      throw new Error(`Vector3.y is required`);
    }
    this.y = _y;
    let _z = options.z;
    if (_z === null) {
      throw new Error(`Vector3.z is required`);
    }
    this.z = _z;

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
    if (!(this.x === other.x || Math.abs(this.x - other.x) < 1e-10)) {
      return false;
    }
    if (!(this.y === other.y || Math.abs(this.y - other.y) < 1e-10)) {
      return false;
    }
    if (!(this.z === other.z || Math.abs(this.z - other.z) < 1e-10)) {
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
      this._value = Vector3.__packValue__(this);
    }
    return this._value;
  }

  static __packValue__(object: Vector3): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 50201;
    objectValue["50"] = object.x;
    objectValue["51"] = object.y;
    objectValue["52"] = object.z;
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Vector3 {
    return new Vector3({
      x: objectValue["50"],
      y: objectValue["51"],
      z: objectValue["52"],
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
  ): Vector3 {
    return Vector3.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
  }

  toProto(): Vector3Proto {
    if (this._proto === null) {
      // @ts-expect-error(readonly)
      this._proto = Vector3.__packProto__(this);
    }
    return this._proto as Vector3Proto;
  }

  static __packProto__(object: Vector3): Vector3Proto {
    const objectProto: Partial<Vector3Proto> = { metatype: 50201 };
    objectProto.x = object.x;
    objectProto.y = object.y;
    objectProto.z = object.z;
    return objectProto as Vector3Proto;
  }

  static __unpackProto__(
    objectProto: Vector3Proto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Vector3 {
    return new Vector3({
      x: objectProto.x,
      y: objectProto.y,
      z: objectProto.z,
      _proto: objectProto,
      _supergraph,
    });
  }

  static fromProto(
    objectProto: Vector3Proto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Vector3 {
    return Vector3.__unpackProto__(objectProto, _session, _supergraph, _graph, _connection);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerStructClass(StructType.VECTOR3, Vector3);
/* ==== DESTACK_GENERATED_END:STRUCT:50201 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:50202 ==== */
/**
 * A 4D float vector.
 */
export class Vector4 extends StructFrozen {
  static metatype: StructType = StructType.VECTOR4;
  static __isFrozen__: boolean = true;

  /**
   * Vector4.x
   */
  readonly x: number;

  /**
   * Vector4.y
   */
  readonly y: number;

  /**
   * Vector4.z
   */
  readonly z: number;

  /**
   * Vector4.w
   */
  readonly w: number;

  constructor(options: {
    x: number;
    y: number;
    z: number;
    w: number;
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
    let _x = options.x;
    if (_x === null) {
      throw new Error(`Vector4.x is required`);
    }
    this.x = _x;
    let _y = options.y;
    if (_y === null) {
      throw new Error(`Vector4.y is required`);
    }
    this.y = _y;
    let _z = options.z;
    if (_z === null) {
      throw new Error(`Vector4.z is required`);
    }
    this.z = _z;
    let _w = options.w;
    if (_w === null) {
      throw new Error(`Vector4.w is required`);
    }
    this.w = _w;

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
    if (!(this.x === other.x || Math.abs(this.x - other.x) < 1e-10)) {
      return false;
    }
    if (!(this.y === other.y || Math.abs(this.y - other.y) < 1e-10)) {
      return false;
    }
    if (!(this.z === other.z || Math.abs(this.z - other.z) < 1e-10)) {
      return false;
    }
    if (!(this.w === other.w || Math.abs(this.w - other.w) < 1e-10)) {
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
      this._value = Vector4.__packValue__(this);
    }
    return this._value;
  }

  static __packValue__(object: Vector4): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 50202;
    objectValue["50"] = object.x;
    objectValue["51"] = object.y;
    objectValue["52"] = object.z;
    objectValue["53"] = object.w;
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Vector4 {
    return new Vector4({
      x: objectValue["50"],
      y: objectValue["51"],
      z: objectValue["52"],
      w: objectValue["53"],
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
  ): Vector4 {
    return Vector4.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
  }

  toProto(): Vector4Proto {
    if (this._proto === null) {
      // @ts-expect-error(readonly)
      this._proto = Vector4.__packProto__(this);
    }
    return this._proto as Vector4Proto;
  }

  static __packProto__(object: Vector4): Vector4Proto {
    const objectProto: Partial<Vector4Proto> = { metatype: 50202 };
    objectProto.x = object.x;
    objectProto.y = object.y;
    objectProto.z = object.z;
    objectProto.w = object.w;
    return objectProto as Vector4Proto;
  }

  static __unpackProto__(
    objectProto: Vector4Proto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Vector4 {
    return new Vector4({
      x: objectProto.x,
      y: objectProto.y,
      z: objectProto.z,
      w: objectProto.w,
      _proto: objectProto,
      _supergraph,
    });
  }

  static fromProto(
    objectProto: Vector4Proto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Vector4 {
    return Vector4.__unpackProto__(objectProto, _session, _supergraph, _graph, _connection);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerStructClass(StructType.VECTOR4, Vector4);
/* ==== DESTACK_GENERATED_END:STRUCT:50202 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:50203 ==== */
/**
 * A 2D integer vector.
 */
export class Vector2i extends StructFrozen {
  static metatype: StructType = StructType.VECTOR2I;
  static __isFrozen__: boolean = true;

  /**
   * Vector2i.x
   */
  readonly x: number;

  /**
   * Vector2i.y
   */
  readonly y: number;

  constructor(options: {
    x: number;
    y: number;
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
    let _x = options.x;
    if (_x === null) {
      throw new Error(`Vector2i.x is required`);
    }
    this.x = _x;
    let _y = options.y;
    if (_y === null) {
      throw new Error(`Vector2i.y is required`);
    }
    this.y = _y;

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
    if (!(this.x === other.x)) {
      return false;
    }
    if (!(this.y === other.y)) {
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
      this._value = Vector2i.__packValue__(this);
    }
    return this._value;
  }

  static __packValue__(object: Vector2i): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 50203;
    objectValue["50"] = object.x;
    objectValue["51"] = object.y;
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Vector2i {
    return new Vector2i({
      x: Number(objectValue["50"]),
      y: Number(objectValue["51"]),
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
  ): Vector2i {
    return Vector2i.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
  }

  toProto(): Vector2iProto {
    if (this._proto === null) {
      // @ts-expect-error(readonly)
      this._proto = Vector2i.__packProto__(this);
    }
    return this._proto as Vector2iProto;
  }

  static __packProto__(object: Vector2i): Vector2iProto {
    const objectProto: Partial<Vector2iProto> = { metatype: 50203 };
    objectProto.x = object.x;
    objectProto.y = object.y;
    return objectProto as Vector2iProto;
  }

  static __unpackProto__(
    objectProto: Vector2iProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Vector2i {
    return new Vector2i({
      x: Number(objectProto.x),
      y: Number(objectProto.y),
      _proto: objectProto,
      _supergraph,
    });
  }

  static fromProto(
    objectProto: Vector2iProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Vector2i {
    return Vector2i.__unpackProto__(objectProto, _session, _supergraph, _graph, _connection);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerStructClass(StructType.VECTOR2I, Vector2i);
/* ==== DESTACK_GENERATED_END:STRUCT:50203 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:50204 ==== */
/**
 * A 3D integer vector.
 */
export class Vector3i extends StructFrozen {
  static metatype: StructType = StructType.VECTOR3I;
  static __isFrozen__: boolean = true;

  /**
   * Vector3i.x
   */
  readonly x: number;

  /**
   * Vector3i.y
   */
  readonly y: number;

  /**
   * Vector3i.z
   */
  readonly z: number;

  constructor(options: {
    x: number;
    y: number;
    z: number;
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
    let _x = options.x;
    if (_x === null) {
      throw new Error(`Vector3i.x is required`);
    }
    this.x = _x;
    let _y = options.y;
    if (_y === null) {
      throw new Error(`Vector3i.y is required`);
    }
    this.y = _y;
    let _z = options.z;
    if (_z === null) {
      throw new Error(`Vector3i.z is required`);
    }
    this.z = _z;

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
    if (!(this.x === other.x)) {
      return false;
    }
    if (!(this.y === other.y)) {
      return false;
    }
    if (!(this.z === other.z)) {
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
      this._value = Vector3i.__packValue__(this);
    }
    return this._value;
  }

  static __packValue__(object: Vector3i): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 50204;
    objectValue["50"] = object.x;
    objectValue["51"] = object.y;
    objectValue["52"] = object.z;
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Vector3i {
    return new Vector3i({
      x: Number(objectValue["50"]),
      y: Number(objectValue["51"]),
      z: Number(objectValue["52"]),
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
  ): Vector3i {
    return Vector3i.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
  }

  toProto(): Vector3iProto {
    if (this._proto === null) {
      // @ts-expect-error(readonly)
      this._proto = Vector3i.__packProto__(this);
    }
    return this._proto as Vector3iProto;
  }

  static __packProto__(object: Vector3i): Vector3iProto {
    const objectProto: Partial<Vector3iProto> = { metatype: 50204 };
    objectProto.x = object.x;
    objectProto.y = object.y;
    objectProto.z = object.z;
    return objectProto as Vector3iProto;
  }

  static __unpackProto__(
    objectProto: Vector3iProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Vector3i {
    return new Vector3i({
      x: Number(objectProto.x),
      y: Number(objectProto.y),
      z: Number(objectProto.z),
      _proto: objectProto,
      _supergraph,
    });
  }

  static fromProto(
    objectProto: Vector3iProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Vector3i {
    return Vector3i.__unpackProto__(objectProto, _session, _supergraph, _graph, _connection);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerStructClass(StructType.VECTOR3I, Vector3i);
/* ==== DESTACK_GENERATED_END:STRUCT:50204 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:50205 ==== */
/**
 * A 4D integer vector.
 */
export class Vector4i extends StructFrozen {
  static metatype: StructType = StructType.VECTOR4I;
  static __isFrozen__: boolean = true;

  /**
   * Vector4i.x
   */
  readonly x: number;

  /**
   * Vector4i.y
   */
  readonly y: number;

  /**
   * Vector4i.z
   */
  readonly z: number;

  /**
   * Vector4i.w
   */
  readonly w: number;

  constructor(options: {
    x: number;
    y: number;
    z: number;
    w: number;
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
    let _x = options.x;
    if (_x === null) {
      throw new Error(`Vector4i.x is required`);
    }
    this.x = _x;
    let _y = options.y;
    if (_y === null) {
      throw new Error(`Vector4i.y is required`);
    }
    this.y = _y;
    let _z = options.z;
    if (_z === null) {
      throw new Error(`Vector4i.z is required`);
    }
    this.z = _z;
    let _w = options.w;
    if (_w === null) {
      throw new Error(`Vector4i.w is required`);
    }
    this.w = _w;

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
    if (!(this.x === other.x)) {
      return false;
    }
    if (!(this.y === other.y)) {
      return false;
    }
    if (!(this.z === other.z)) {
      return false;
    }
    if (!(this.w === other.w)) {
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
      this._value = Vector4i.__packValue__(this);
    }
    return this._value;
  }

  static __packValue__(object: Vector4i): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 50205;
    objectValue["50"] = object.x;
    objectValue["51"] = object.y;
    objectValue["52"] = object.z;
    objectValue["53"] = object.w;
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Vector4i {
    return new Vector4i({
      x: Number(objectValue["50"]),
      y: Number(objectValue["51"]),
      z: Number(objectValue["52"]),
      w: Number(objectValue["53"]),
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
  ): Vector4i {
    return Vector4i.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
  }

  toProto(): Vector4iProto {
    if (this._proto === null) {
      // @ts-expect-error(readonly)
      this._proto = Vector4i.__packProto__(this);
    }
    return this._proto as Vector4iProto;
  }

  static __packProto__(object: Vector4i): Vector4iProto {
    const objectProto: Partial<Vector4iProto> = { metatype: 50205 };
    objectProto.x = object.x;
    objectProto.y = object.y;
    objectProto.z = object.z;
    objectProto.w = object.w;
    return objectProto as Vector4iProto;
  }

  static __unpackProto__(
    objectProto: Vector4iProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Vector4i {
    return new Vector4i({
      x: Number(objectProto.x),
      y: Number(objectProto.y),
      z: Number(objectProto.z),
      w: Number(objectProto.w),
      _proto: objectProto,
      _supergraph,
    });
  }

  static fromProto(
    objectProto: Vector4iProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Vector4i {
    return Vector4i.__unpackProto__(objectProto, _session, _supergraph, _graph, _connection);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerStructClass(StructType.VECTOR4I, Vector4i);
/* ==== DESTACK_GENERATED_END:STRUCT:50205 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:12026 ==== */
/**
 * A grid configuration value.
 */
export class Grid extends StructFrozen {
  static metatype: StructType = StructType.GRID;
  static __isFrozen__: boolean = true;

  /**
   * Grid.columns
   */
  readonly columns: number;

  /**
   * Grid.rows
   */
  readonly rows: number;

  /**
   * Grid.columnWidth
   */
  readonly columnWidth: Dimension | null;

  /**
   * Grid.columnMinWidth
   */
  readonly columnMinWidth: Dimension | null;

  /**
   * Grid.rowHeight
   */
  readonly rowHeight: Dimension | null;

  constructor(options: {
    columns: number;
    rows: number;
    columnWidth?: Dimension | null;
    columnMinWidth?: Dimension | null;
    rowHeight?: Dimension | null;
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
    let _columns = options.columns;
    if (_columns === null) {
      throw new Error(`Grid.columns is required`);
    }
    this.columns = _columns;
    let _rows = options.rows;
    if (_rows === null) {
      throw new Error(`Grid.rows is required`);
    }
    this.rows = _rows;
    let _columnWidth = options.columnWidth ?? null;
    this.columnWidth = _columnWidth;
    let _columnMinWidth = options.columnMinWidth ?? null;
    this.columnMinWidth = _columnMinWidth;
    let _rowHeight = options.rowHeight ?? null;
    this.rowHeight = _rowHeight;

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
    if (!(this.columns === other.columns)) {
      return false;
    }
    if (!(this.rows === other.rows)) {
      return false;
    }
    if (
      (this.columnWidth == null) !== (other.columnWidth == null) ||
      (this.columnWidth != null && !this.columnWidth.equals(other.columnWidth))
    ) {
      return false;
    }
    if (
      (this.columnMinWidth == null) !== (other.columnMinWidth == null) ||
      (this.columnMinWidth != null && !this.columnMinWidth.equals(other.columnMinWidth))
    ) {
      return false;
    }
    if (
      (this.rowHeight == null) !== (other.rowHeight == null) ||
      (this.rowHeight != null && !this.rowHeight.equals(other.rowHeight))
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
      this._value = Grid.__packValue__(this);
    }
    return this._value;
  }

  static __packValue__(object: Grid): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 12026;
    objectValue["50"] = object.columns;
    objectValue["51"] = object.rows;
    if (object.columnWidth != null) {
      objectValue["52"] = object.columnWidth.toValue();
    }
    if (object.columnMinWidth != null) {
      objectValue["53"] = object.columnMinWidth.toValue();
    }
    if (object.rowHeight != null) {
      objectValue["54"] = object.rowHeight.toValue();
    }
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Grid {
    const columnWidthValue = objectValue["52"];
    const unpackedColumnWidth =
      columnWidthValue != undefined
        ? Dimension.fromValue(columnWidthValue, _session, _supergraph, _graph, _connection)
        : null;
    const columnMinWidthValue = objectValue["53"];
    const unpackedColumnMinWidth =
      columnMinWidthValue != undefined
        ? Dimension.fromValue(columnMinWidthValue, _session, _supergraph, _graph, _connection)
        : null;
    const rowHeightValue = objectValue["54"];
    const unpackedRowHeight =
      rowHeightValue != undefined
        ? Dimension.fromValue(rowHeightValue, _session, _supergraph, _graph, _connection)
        : null;
    return new Grid({
      columns: Number(objectValue["50"]),
      rows: Number(objectValue["51"]),
      columnWidth: unpackedColumnWidth,
      columnMinWidth: unpackedColumnMinWidth,
      rowHeight: unpackedRowHeight,
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
  ): Grid {
    return Grid.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
  }

  toProto(): GridProto {
    if (this._proto === null) {
      // @ts-expect-error(readonly)
      this._proto = Grid.__packProto__(this);
    }
    return this._proto as GridProto;
  }

  static __packProto__(object: Grid): GridProto {
    const objectProto: Partial<GridProto> = { metatype: 12026 };
    objectProto.columns = object.columns;
    objectProto.rows = object.rows;
    if (object.columnWidth != null) {
      objectProto.columnWidth = object.columnWidth.toProto();
    }
    if (object.columnMinWidth != null) {
      objectProto.columnMinWidth = object.columnMinWidth.toProto();
    }
    if (object.rowHeight != null) {
      objectProto.rowHeight = object.rowHeight.toProto();
    }
    return objectProto as GridProto;
  }

  static __unpackProto__(
    objectProto: GridProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Grid {
    return new Grid({
      columns: Number(objectProto.columns),
      rows: Number(objectProto.rows),
      columnWidth:
        objectProto.columnWidth != undefined
          ? Dimension.fromProto(objectProto.columnWidth!, _session, _supergraph, _graph, _connection)
          : null,
      columnMinWidth:
        objectProto.columnMinWidth != undefined
          ? Dimension.fromProto(objectProto.columnMinWidth!, _session, _supergraph, _graph, _connection)
          : null,
      rowHeight:
        objectProto.rowHeight != undefined
          ? Dimension.fromProto(objectProto.rowHeight!, _session, _supergraph, _graph, _connection)
          : null,
      _proto: objectProto,
      _supergraph,
    });
  }

  static fromProto(
    objectProto: GridProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Grid {
    return Grid.__unpackProto__(objectProto, _session, _supergraph, _graph, _connection);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerStructClass(StructType.GRID, Grid);
/* ==== DESTACK_GENERATED_END:STRUCT:12026 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:12028 ==== */
/**
 * A grid span value.
 */
export class GridSpan extends StructFrozen {
  static metatype: StructType = StructType.GRID_SPAN;
  static __isFrozen__: boolean = true;

  /**
   * GridSpan.columns
   */
  readonly columns: number;

  /**
   * GridSpan.rows
   */
  readonly rows: number;

  constructor(options: {
    columns: number;
    rows: number;
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
    let _columns = options.columns;
    if (_columns === null) {
      throw new Error(`GridSpan.columns is required`);
    }
    this.columns = _columns;
    let _rows = options.rows;
    if (_rows === null) {
      throw new Error(`GridSpan.rows is required`);
    }
    this.rows = _rows;

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
    if (!(this.columns === other.columns)) {
      return false;
    }
    if (!(this.rows === other.rows)) {
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
      this._value = GridSpan.__packValue__(this);
    }
    return this._value;
  }

  static __packValue__(object: GridSpan): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 12028;
    objectValue["50"] = object.columns;
    objectValue["51"] = object.rows;
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): GridSpan {
    return new GridSpan({
      columns: Number(objectValue["50"]),
      rows: Number(objectValue["51"]),
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
  ): GridSpan {
    return GridSpan.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
  }

  toProto(): GridSpanProto {
    if (this._proto === null) {
      // @ts-expect-error(readonly)
      this._proto = GridSpan.__packProto__(this);
    }
    return this._proto as GridSpanProto;
  }

  static __packProto__(object: GridSpan): GridSpanProto {
    const objectProto: Partial<GridSpanProto> = { metatype: 12028 };
    objectProto.columns = object.columns;
    objectProto.rows = object.rows;
    return objectProto as GridSpanProto;
  }

  static __unpackProto__(
    objectProto: GridSpanProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): GridSpan {
    return new GridSpan({
      columns: Number(objectProto.columns),
      rows: Number(objectProto.rows),
      _proto: objectProto,
      _supergraph,
    });
  }

  static fromProto(
    objectProto: GridSpanProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): GridSpan {
    return GridSpan.__unpackProto__(objectProto, _session, _supergraph, _graph, _connection);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerStructClass(StructType.GRID_SPAN, GridSpan);
/* ==== DESTACK_GENERATED_END:STRUCT:12028 ==== */
