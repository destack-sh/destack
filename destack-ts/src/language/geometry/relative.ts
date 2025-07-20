import type { Session, Supergraph } from "@destack/language/core";
import { EnumType, StructFrozen, StructType } from "@destack/language/core";
import {
  STRUCT_CLASS_BY_TYPE,
  registerEnumClass,
  registerStructClass,
} from "@destack/language/registry";
import {
  AnchorProto,
  Axis2Proto,
  Axis3Proto,
  Corner2Proto,
  Grid2Proto,
  GridSpan2Proto,
  Inset2Proto,
  LengthProto,
  LengthTypeProto,
  Offset2Proto,
} from "@destack/proto";
import { base64Decode } from "@destack/utils";
import { hashFloat, hashInt } from "@destack/utils/hash";

/* ==== DESTACK_GENERATED_START:ENUM:2400002 ==== */
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
/* ==== DESTACK_GENERATED_END:ENUM:2400002 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:2400006 ==== */
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
/* ==== DESTACK_GENERATED_END:ENUM:2400006 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:2400005 ==== */
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
/* ==== DESTACK_GENERATED_END:ENUM:2400005 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:2400003 ==== */
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
/* ==== DESTACK_GENERATED_END:ENUM:2400003 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:2400004 ==== */
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
/* ==== DESTACK_GENERATED_END:ENUM:2400004 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:2400000 ==== */
/**
 * Anchor
 */
export enum Anchor {
  RELATIVE = 1,
  ABSOLUTE = 2,
  FIXED = 3,
  STICKY = 4,

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerEnumClass(EnumType.ANCHOR, Anchor);
/* ==== DESTACK_GENERATED_END:ENUM:2400000 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:2400001 ==== */
/**
 * LengthType
 */
export enum LengthType {
  PIXEL = 1,
  REM = 2,
  PERCENT = 3,
  FR = 4,
  FIT = 10,
  FILL = 11,

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerEnumClass(EnumType.LENGTH_TYPE, LengthType);
/* ==== DESTACK_GENERATED_END:ENUM:2400001 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:1800001 ==== */
/**
 * An absolute or relative length value.
 */
export class Length extends StructFrozen {
  static metatype: StructType = StructType.LENGTH;
  static __isFrozen__: boolean = true;

  /**
   * Length.unit
   */
  readonly unit: LengthType;

  /**
   * Length.value
   */
  readonly value: number;

  constructor(options: {
    unit: LengthType;
    value: number;
    _session?: Session | null;
    _supergraph?: Supergraph | null;
    _hash?: number | null;
    _repr?: string | null;
    _proto?: any | null;
    _cson?: any | null;
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
    this._cson = options._cson ?? null;
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

  repr(): string {
    if (this._repr === null) {
      const propertyReprs: string[] = [];
      propertyReprs.push(`unit=${LengthType[this.unit]}`);
      propertyReprs.push(`value=${this.value}`);
      // @ts-expect-error(readonly)
      this._repr = `<Length ${propertyReprs.join(" ")}>`;
    }
    return this._repr;
  }

  hash(): number {
    if (this._hash != null) {
      return this._hash;
    }

    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    h = (h * 31 + this.unit) & 0xffffffff;
    h = (h * 31 + hashFloat(this.value)) & 0xffffffff;

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
      this._cson = Length.__packCson__(this);
    }
    return this._cson;
  }

  static __packCson__(object: Length): { [key: string]: any } {
    const objectCson: { [key: string]: any } = {};
    objectCson["1"] = 1800001;
    objectCson["101"] = object.unit;
    objectCson["102"] = object.value;
    return objectCson;
  }

  static __unpackCson__(
    objectCson: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Length {
    return new Length({
      unit: Number(objectCson["101"]),
      value: objectCson["102"],
      _cson: objectCson,
      _supergraph,
    });
  }

  static fromCson(
    objectCson: { readonly [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Length {
    return Length.__unpackCson__(objectCson, _session, _supergraph, _graph, _connection);
  }

  toProto(): LengthProto {
    if (this._proto === null) {
      // @ts-expect-error(readonly)
      this._proto = Length.__packProto__(this);
    }
    return this._proto as LengthProto;
  }

  static __packProto__(object: Length): LengthProto {
    const objectProto: Partial<LengthProto> = { metatype: 1800001 };
    objectProto.unit = Number(object.unit) as LengthTypeProto;
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
      unit: Number(objectProto.unit) as LengthType,
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

  static fromProtoString(packedProtoString: string): Length {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = LengthProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerStructClass(StructType.LENGTH, Length);
/* ==== DESTACK_GENERATED_END:STRUCT:1800001 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:2400020 ==== */
/**
 * A 2-dimensional position value (relative or absolute).
 */
export class Offset2 extends StructFrozen {
  static metatype: StructType = StructType.OFFSET2;
  static __isFrozen__: boolean = true;

  /**
   * Offset2.type
   */
  readonly type: Anchor;

  /**
   * Offset2.top
   */
  readonly top: Length | null;

  /**
   * Offset2.left
   */
  readonly left: Length | null;

  /**
   * Offset2.width
   */
  readonly width: Length | null;

  /**
   * Offset2.height
   */
  readonly height: Length | null;

  constructor(options: {
    type: Anchor;
    top?: Length | null;
    left?: Length | null;
    width?: Length | null;
    height?: Length | null;
    _session?: Session | null;
    _supergraph?: Supergraph | null;
    _hash?: number | null;
    _repr?: string | null;
    _proto?: any | null;
    _cson?: any | null;
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
      throw new Error(`Offset2.type is required`);
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
      (this.top == null) !== (other.top == null) ||
      (this.top != null && !this.top.equals(other.top))
    ) {
      return false;
    }
    if (
      (this.left == null) !== (other.left == null) ||
      (this.left != null && !this.left.equals(other.left))
    ) {
      return false;
    }
    if (
      (this.width == null) !== (other.width == null) ||
      (this.width != null && !this.width.equals(other.width))
    ) {
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

  repr(): string {
    if (this._repr === null) {
      const propertyReprs: string[] = [];
      propertyReprs.push(`type=${Anchor[this.type]}`);
      if (this.top != null) {
        propertyReprs.push(`top=${this.top.repr()}`);
      }
      if (this.left != null) {
        propertyReprs.push(`left=${this.left.repr()}`);
      }
      if (this.width != null) {
        propertyReprs.push(`width=${this.width.repr()}`);
      }
      if (this.height != null) {
        propertyReprs.push(`height=${this.height.repr()}`);
      }
      // @ts-expect-error(readonly)
      this._repr = `<Offset2 ${propertyReprs.join(" ")}>`;
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
    if (this.top != null) {
      h = (h * 31 + this.top.hash()) & 0xffffffff;
    }
    if (this.left != null) {
      h = (h * 31 + this.left.hash()) & 0xffffffff;
    }
    if (this.width != null) {
      h = (h * 31 + this.width.hash()) & 0xffffffff;
    }
    if (this.height != null) {
      h = (h * 31 + this.height.hash()) & 0xffffffff;
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
      this._cson = Offset2.__packCson__(this);
    }
    return this._cson;
  }

  static __packCson__(object: Offset2): { [key: string]: any } {
    const objectCson: { [key: string]: any } = {};
    objectCson["1"] = 2400020;
    objectCson["100"] = object.type;
    if (object.top != null) {
      objectCson["101"] = object.top.toCson();
    }
    if (object.left != null) {
      objectCson["102"] = object.left.toCson();
    }
    if (object.width != null) {
      objectCson["103"] = object.width.toCson();
    }
    if (object.height != null) {
      objectCson["104"] = object.height.toCson();
    }
    return objectCson;
  }

  static __unpackCson__(
    objectCson: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Offset2 {
    const _Length = STRUCT_CLASS_BY_TYPE[StructType.LENGTH] as typeof Length;
    const topValue = objectCson["101"];
    const unpackedTop =
      topValue != undefined
        ? _Length.fromCson(topValue, _session, _supergraph, _graph, _connection)
        : null;
    const leftValue = objectCson["102"];
    const unpackedLeft =
      leftValue != undefined
        ? _Length.fromCson(leftValue, _session, _supergraph, _graph, _connection)
        : null;
    const widthValue = objectCson["103"];
    const unpackedWidth =
      widthValue != undefined
        ? _Length.fromCson(widthValue, _session, _supergraph, _graph, _connection)
        : null;
    const heightValue = objectCson["104"];
    const unpackedHeight =
      heightValue != undefined
        ? _Length.fromCson(heightValue, _session, _supergraph, _graph, _connection)
        : null;
    return new Offset2({
      type: Number(objectCson["100"]),
      top: unpackedTop,
      left: unpackedLeft,
      width: unpackedWidth,
      height: unpackedHeight,
      _cson: objectCson,
      _supergraph,
    });
  }

  static fromCson(
    objectCson: { readonly [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Offset2 {
    return Offset2.__unpackCson__(objectCson, _session, _supergraph, _graph, _connection);
  }

  toProto(): Offset2Proto {
    if (this._proto === null) {
      // @ts-expect-error(readonly)
      this._proto = Offset2.__packProto__(this);
    }
    return this._proto as Offset2Proto;
  }

  static __packProto__(object: Offset2): Offset2Proto {
    const objectProto: Partial<Offset2Proto> = { metatype: 2400020 };
    objectProto.type = Number(object.type) as AnchorProto;
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
    return objectProto as Offset2Proto;
  }

  static __unpackProto__(
    objectProto: Offset2Proto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Offset2 {
    const _Length = STRUCT_CLASS_BY_TYPE[StructType.LENGTH] as typeof Length;
    return new Offset2({
      type: Number(objectProto.type) as Anchor,
      top:
        objectProto.top != undefined
          ? _Length.fromProto(objectProto.top!, _session, _supergraph, _graph, _connection)
          : null,
      left:
        objectProto.left != undefined
          ? _Length.fromProto(objectProto.left!, _session, _supergraph, _graph, _connection)
          : null,
      width:
        objectProto.width != undefined
          ? _Length.fromProto(objectProto.width!, _session, _supergraph, _graph, _connection)
          : null,
      height:
        objectProto.height != undefined
          ? _Length.fromProto(objectProto.height!, _session, _supergraph, _graph, _connection)
          : null,
      _proto: objectProto,
      _supergraph,
    });
  }

  static fromProto(
    objectProto: Offset2Proto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Offset2 {
    return Offset2.__unpackProto__(objectProto, _session, _supergraph, _graph, _connection);
  }

  static fromProtoString(packedProtoString: string): Offset2 {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = Offset2Proto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerStructClass(StructType.OFFSET2, Offset2);
/* ==== DESTACK_GENERATED_END:STRUCT:2400020 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:2400023 ==== */
/**
 * A 2-dimensional insets value (base + side overrides).
 */
export class Inset2 extends StructFrozen {
  static metatype: StructType = StructType.INSET2;
  static __isFrozen__: boolean = true;

  /**
   * Inset2.base
   */
  readonly base: number;

  /**
   * Inset2.top
   */
  readonly top: number | null;

  /**
   * Inset2.left
   */
  readonly left: number | null;

  /**
   * Inset2.right
   */
  readonly right: number | null;

  /**
   * Inset2.bottom
   */
  readonly bottom: number | null;

  constructor(options: {
    base?: number;
    top?: number | null;
    left?: number | null;
    right?: number | null;
    bottom?: number | null;
    _session?: Session | null;
    _supergraph?: Supergraph | null;
    _hash?: number | null;
    _repr?: string | null;
    _proto?: any | null;
    _cson?: any | null;
  }) {
    super(
      // session
      options._session ?? null,
      // supergraph
      options._supergraph ?? null,
    );

    // properties
    let _base = options.base ?? null;
    if (_base === null) {
      _base = 0;
    }
    if (_base === null) {
      throw new Error(`Inset2.base is required`);
    }
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
    this._cson = options._cson ?? null;
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

  repr(): string {
    if (this._repr === null) {
      const propertyReprs: string[] = [];
      propertyReprs.push(`base=${this.base}`);
      if (this.top != null) {
        propertyReprs.push(`top=${this.top}`);
      }
      if (this.left != null) {
        propertyReprs.push(`left=${this.left}`);
      }
      if (this.right != null) {
        propertyReprs.push(`right=${this.right}`);
      }
      if (this.bottom != null) {
        propertyReprs.push(`bottom=${this.bottom}`);
      }
      // @ts-expect-error(readonly)
      this._repr = `<Inset2 ${propertyReprs.join(" ")}>`;
    }
    return this._repr;
  }

  hash(): number {
    if (this._hash != null) {
      return this._hash;
    }

    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    h = (h * 31 + hashInt(this.base)) & 0xffffffff;
    if (this.top != null) {
      h = (h * 31 + hashInt(this.top)) & 0xffffffff;
    }
    if (this.left != null) {
      h = (h * 31 + hashInt(this.left)) & 0xffffffff;
    }
    if (this.right != null) {
      h = (h * 31 + hashInt(this.right)) & 0xffffffff;
    }
    if (this.bottom != null) {
      h = (h * 31 + hashInt(this.bottom)) & 0xffffffff;
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
      this._cson = Inset2.__packCson__(this);
    }
    return this._cson;
  }

  static __packCson__(object: Inset2): { [key: string]: any } {
    const objectCson: { [key: string]: any } = {};
    objectCson["1"] = 2400023;
    objectCson["101"] = object.base;
    if (object.top != null) {
      objectCson["102"] = object.top;
    }
    if (object.left != null) {
      objectCson["103"] = object.left;
    }
    if (object.right != null) {
      objectCson["104"] = object.right;
    }
    if (object.bottom != null) {
      objectCson["105"] = object.bottom;
    }
    return objectCson;
  }

  static __unpackCson__(
    objectCson: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Inset2 {
    const topValue = objectCson["102"];
    const unpackedTop = topValue != undefined ? Number(topValue) : null;
    const leftValue = objectCson["103"];
    const unpackedLeft = leftValue != undefined ? Number(leftValue) : null;
    const rightValue = objectCson["104"];
    const unpackedRight = rightValue != undefined ? Number(rightValue) : null;
    const bottomValue = objectCson["105"];
    const unpackedBottom = bottomValue != undefined ? Number(bottomValue) : null;
    return new Inset2({
      base: Number(objectCson["101"]),
      top: unpackedTop,
      left: unpackedLeft,
      right: unpackedRight,
      bottom: unpackedBottom,
      _cson: objectCson,
      _supergraph,
    });
  }

  static fromCson(
    objectCson: { readonly [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Inset2 {
    return Inset2.__unpackCson__(objectCson, _session, _supergraph, _graph, _connection);
  }

  toProto(): Inset2Proto {
    if (this._proto === null) {
      // @ts-expect-error(readonly)
      this._proto = Inset2.__packProto__(this);
    }
    return this._proto as Inset2Proto;
  }

  static __packProto__(object: Inset2): Inset2Proto {
    const objectProto: Partial<Inset2Proto> = { metatype: 2400023 };
    objectProto.base = object.base;
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
    return objectProto as Inset2Proto;
  }

  static __unpackProto__(
    objectProto: Inset2Proto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Inset2 {
    return new Inset2({
      base: Number(objectProto.base),
      top: objectProto.top != undefined ? Number(objectProto.top) : null,
      left: objectProto.left != undefined ? Number(objectProto.left) : null,
      right: objectProto.right != undefined ? Number(objectProto.right) : null,
      bottom: objectProto.bottom != undefined ? Number(objectProto.bottom) : null,
      _proto: objectProto,
      _supergraph,
    });
  }

  static fromProto(
    objectProto: Inset2Proto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Inset2 {
    return Inset2.__unpackProto__(objectProto, _session, _supergraph, _graph, _connection);
  }

  static fromProtoString(packedProtoString: string): Inset2 {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = Inset2Proto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerStructClass(StructType.INSET2, Inset2);
/* ==== DESTACK_GENERATED_END:STRUCT:2400023 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:2400024 ==== */
/**
 * A 2-dimensional corners value (base + corner overrides).
 */
export class Corner2 extends StructFrozen {
  static metatype: StructType = StructType.CORNER2;
  static __isFrozen__: boolean = true;

  /**
   * Corner2.base
   */
  readonly base: number;

  /**
   * Corner2.topLeft
   */
  readonly topLeft: number | null;

  /**
   * Corner2.topRight
   */
  readonly topRight: number | null;

  /**
   * Corner2.bottomLeft
   */
  readonly bottomLeft: number | null;

  /**
   * Corner2.bottomRight
   */
  readonly bottomRight: number | null;

  constructor(options: {
    base?: number;
    topLeft?: number | null;
    topRight?: number | null;
    bottomLeft?: number | null;
    bottomRight?: number | null;
    _session?: Session | null;
    _supergraph?: Supergraph | null;
    _hash?: number | null;
    _repr?: string | null;
    _proto?: any | null;
    _cson?: any | null;
  }) {
    super(
      // session
      options._session ?? null,
      // supergraph
      options._supergraph ?? null,
    );

    // properties
    let _base = options.base ?? null;
    if (_base === null) {
      _base = 0;
    }
    if (_base === null) {
      throw new Error(`Corner2.base is required`);
    }
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
    this._cson = options._cson ?? null;
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

  repr(): string {
    if (this._repr === null) {
      const propertyReprs: string[] = [];
      propertyReprs.push(`base=${this.base}`);
      if (this.topLeft != null) {
        propertyReprs.push(`topLeft=${this.topLeft}`);
      }
      if (this.topRight != null) {
        propertyReprs.push(`topRight=${this.topRight}`);
      }
      if (this.bottomLeft != null) {
        propertyReprs.push(`bottomLeft=${this.bottomLeft}`);
      }
      if (this.bottomRight != null) {
        propertyReprs.push(`bottomRight=${this.bottomRight}`);
      }
      // @ts-expect-error(readonly)
      this._repr = `<Corner2 ${propertyReprs.join(" ")}>`;
    }
    return this._repr;
  }

  hash(): number {
    if (this._hash != null) {
      return this._hash;
    }

    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    h = (h * 31 + hashInt(this.base)) & 0xffffffff;
    if (this.topLeft != null) {
      h = (h * 31 + hashInt(this.topLeft)) & 0xffffffff;
    }
    if (this.topRight != null) {
      h = (h * 31 + hashInt(this.topRight)) & 0xffffffff;
    }
    if (this.bottomLeft != null) {
      h = (h * 31 + hashInt(this.bottomLeft)) & 0xffffffff;
    }
    if (this.bottomRight != null) {
      h = (h * 31 + hashInt(this.bottomRight)) & 0xffffffff;
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
      this._cson = Corner2.__packCson__(this);
    }
    return this._cson;
  }

  static __packCson__(object: Corner2): { [key: string]: any } {
    const objectCson: { [key: string]: any } = {};
    objectCson["1"] = 2400024;
    objectCson["101"] = object.base;
    if (object.topLeft != null) {
      objectCson["102"] = object.topLeft;
    }
    if (object.topRight != null) {
      objectCson["103"] = object.topRight;
    }
    if (object.bottomLeft != null) {
      objectCson["104"] = object.bottomLeft;
    }
    if (object.bottomRight != null) {
      objectCson["105"] = object.bottomRight;
    }
    return objectCson;
  }

  static __unpackCson__(
    objectCson: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Corner2 {
    const topLeftValue = objectCson["102"];
    const unpackedTopLeft = topLeftValue != undefined ? Number(topLeftValue) : null;
    const topRightValue = objectCson["103"];
    const unpackedTopRight = topRightValue != undefined ? Number(topRightValue) : null;
    const bottomLeftValue = objectCson["104"];
    const unpackedBottomLeft = bottomLeftValue != undefined ? Number(bottomLeftValue) : null;
    const bottomRightValue = objectCson["105"];
    const unpackedBottomRight = bottomRightValue != undefined ? Number(bottomRightValue) : null;
    return new Corner2({
      base: Number(objectCson["101"]),
      topLeft: unpackedTopLeft,
      topRight: unpackedTopRight,
      bottomLeft: unpackedBottomLeft,
      bottomRight: unpackedBottomRight,
      _cson: objectCson,
      _supergraph,
    });
  }

  static fromCson(
    objectCson: { readonly [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Corner2 {
    return Corner2.__unpackCson__(objectCson, _session, _supergraph, _graph, _connection);
  }

  toProto(): Corner2Proto {
    if (this._proto === null) {
      // @ts-expect-error(readonly)
      this._proto = Corner2.__packProto__(this);
    }
    return this._proto as Corner2Proto;
  }

  static __packProto__(object: Corner2): Corner2Proto {
    const objectProto: Partial<Corner2Proto> = { metatype: 2400024 };
    objectProto.base = object.base;
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
    return objectProto as Corner2Proto;
  }

  static __unpackProto__(
    objectProto: Corner2Proto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Corner2 {
    return new Corner2({
      base: Number(objectProto.base),
      topLeft: objectProto.topLeft != undefined ? Number(objectProto.topLeft) : null,
      topRight: objectProto.topRight != undefined ? Number(objectProto.topRight) : null,
      bottomLeft: objectProto.bottomLeft != undefined ? Number(objectProto.bottomLeft) : null,
      bottomRight: objectProto.bottomRight != undefined ? Number(objectProto.bottomRight) : null,
      _proto: objectProto,
      _supergraph,
    });
  }

  static fromProto(
    objectProto: Corner2Proto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Corner2 {
    return Corner2.__unpackProto__(objectProto, _session, _supergraph, _graph, _connection);
  }

  static fromProtoString(packedProtoString: string): Corner2 {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = Corner2Proto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerStructClass(StructType.CORNER2, Corner2);
/* ==== DESTACK_GENERATED_END:STRUCT:2400024 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:2400025 ==== */
/**
 * A 2-dimensional axis value (base + x/y overrides).
 */
export class Axis2 extends StructFrozen {
  static metatype: StructType = StructType.AXIS2;
  static __isFrozen__: boolean = true;

  /**
   * Axis2.base
   */
  readonly base: number;

  /**
   * Axis2.x
   */
  readonly x: number | null;

  /**
   * Axis2.y
   */
  readonly y: number | null;

  constructor(options: {
    base?: number;
    x?: number | null;
    y?: number | null;
    _session?: Session | null;
    _supergraph?: Supergraph | null;
    _hash?: number | null;
    _repr?: string | null;
    _proto?: any | null;
    _cson?: any | null;
  }) {
    super(
      // session
      options._session ?? null,
      // supergraph
      options._supergraph ?? null,
    );

    // properties
    let _base = options.base ?? null;
    if (_base === null) {
      _base = 0;
    }
    if (_base === null) {
      throw new Error(`Axis2.base is required`);
    }
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
    this._cson = options._cson ?? null;
  }

  equals(other: any): boolean {
    if (!(this.metatype === other.metatype)) {
      return false;
    }
    if (!(this.base === other.base || Math.abs(this.base - other.base) < 1e-10)) {
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

  repr(): string {
    if (this._repr === null) {
      const propertyReprs: string[] = [];
      propertyReprs.push(`base=${this.base}`);
      if (this.x != null) {
        propertyReprs.push(`x=${this.x}`);
      }
      if (this.y != null) {
        propertyReprs.push(`y=${this.y}`);
      }
      // @ts-expect-error(readonly)
      this._repr = `<Axis2 ${propertyReprs.join(" ")}>`;
    }
    return this._repr;
  }

  hash(): number {
    if (this._hash != null) {
      return this._hash;
    }

    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    h = (h * 31 + hashFloat(this.base)) & 0xffffffff;
    if (this.x != null) {
      h = (h * 31 + hashFloat(this.x)) & 0xffffffff;
    }
    if (this.y != null) {
      h = (h * 31 + hashFloat(this.y)) & 0xffffffff;
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
      this._cson = Axis2.__packCson__(this);
    }
    return this._cson;
  }

  static __packCson__(object: Axis2): { [key: string]: any } {
    const objectCson: { [key: string]: any } = {};
    objectCson["1"] = 2400025;
    objectCson["101"] = object.base;
    if (object.x != null) {
      objectCson["102"] = object.x;
    }
    if (object.y != null) {
      objectCson["103"] = object.y;
    }
    return objectCson;
  }

  static __unpackCson__(
    objectCson: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Axis2 {
    const xValue = objectCson["102"];
    const unpackedX = xValue != undefined ? xValue : null;
    const yValue = objectCson["103"];
    const unpackedY = yValue != undefined ? yValue : null;
    return new Axis2({
      base: objectCson["101"],
      x: unpackedX,
      y: unpackedY,
      _cson: objectCson,
      _supergraph,
    });
  }

  static fromCson(
    objectCson: { readonly [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Axis2 {
    return Axis2.__unpackCson__(objectCson, _session, _supergraph, _graph, _connection);
  }

  toProto(): Axis2Proto {
    if (this._proto === null) {
      // @ts-expect-error(readonly)
      this._proto = Axis2.__packProto__(this);
    }
    return this._proto as Axis2Proto;
  }

  static __packProto__(object: Axis2): Axis2Proto {
    const objectProto: Partial<Axis2Proto> = { metatype: 2400025 };
    objectProto.base = object.base;
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
      base: objectProto.base,
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

  static fromProtoString(packedProtoString: string): Axis2 {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = Axis2Proto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerStructClass(StructType.AXIS2, Axis2);
/* ==== DESTACK_GENERATED_END:STRUCT:2400025 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:2400026 ==== */
/**
 * A 3-dimensional axis value (base + x/y/z overrides).
 */
export class Axis3 extends StructFrozen {
  static metatype: StructType = StructType.AXIS3;
  static __isFrozen__: boolean = true;

  /**
   * Axis3.base
   */
  readonly base: number;

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
    base?: number;
    x?: number | null;
    y?: number | null;
    z?: number | null;
    _session?: Session | null;
    _supergraph?: Supergraph | null;
    _hash?: number | null;
    _repr?: string | null;
    _proto?: any | null;
    _cson?: any | null;
  }) {
    super(
      // session
      options._session ?? null,
      // supergraph
      options._supergraph ?? null,
    );

    // properties
    let _base = options.base ?? null;
    if (_base === null) {
      _base = 0;
    }
    if (_base === null) {
      throw new Error(`Axis3.base is required`);
    }
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
    this._cson = options._cson ?? null;
  }

  equals(other: any): boolean {
    if (!(this.metatype === other.metatype)) {
      return false;
    }
    if (!(this.base === other.base || Math.abs(this.base - other.base) < 1e-10)) {
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

  repr(): string {
    if (this._repr === null) {
      const propertyReprs: string[] = [];
      propertyReprs.push(`base=${this.base}`);
      if (this.x != null) {
        propertyReprs.push(`x=${this.x}`);
      }
      if (this.y != null) {
        propertyReprs.push(`y=${this.y}`);
      }
      if (this.z != null) {
        propertyReprs.push(`z=${this.z}`);
      }
      // @ts-expect-error(readonly)
      this._repr = `<Axis3 ${propertyReprs.join(" ")}>`;
    }
    return this._repr;
  }

  hash(): number {
    if (this._hash != null) {
      return this._hash;
    }

    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    h = (h * 31 + hashFloat(this.base)) & 0xffffffff;
    if (this.x != null) {
      h = (h * 31 + hashFloat(this.x)) & 0xffffffff;
    }
    if (this.y != null) {
      h = (h * 31 + hashFloat(this.y)) & 0xffffffff;
    }
    if (this.z != null) {
      h = (h * 31 + hashFloat(this.z)) & 0xffffffff;
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
      this._cson = Axis3.__packCson__(this);
    }
    return this._cson;
  }

  static __packCson__(object: Axis3): { [key: string]: any } {
    const objectCson: { [key: string]: any } = {};
    objectCson["1"] = 2400026;
    objectCson["101"] = object.base;
    if (object.x != null) {
      objectCson["102"] = object.x;
    }
    if (object.y != null) {
      objectCson["103"] = object.y;
    }
    if (object.z != null) {
      objectCson["104"] = object.z;
    }
    return objectCson;
  }

  static __unpackCson__(
    objectCson: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Axis3 {
    const xValue = objectCson["102"];
    const unpackedX = xValue != undefined ? xValue : null;
    const yValue = objectCson["103"];
    const unpackedY = yValue != undefined ? yValue : null;
    const zValue = objectCson["104"];
    const unpackedZ = zValue != undefined ? zValue : null;
    return new Axis3({
      base: objectCson["101"],
      x: unpackedX,
      y: unpackedY,
      z: unpackedZ,
      _cson: objectCson,
      _supergraph,
    });
  }

  static fromCson(
    objectCson: { readonly [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Axis3 {
    return Axis3.__unpackCson__(objectCson, _session, _supergraph, _graph, _connection);
  }

  toProto(): Axis3Proto {
    if (this._proto === null) {
      // @ts-expect-error(readonly)
      this._proto = Axis3.__packProto__(this);
    }
    return this._proto as Axis3Proto;
  }

  static __packProto__(object: Axis3): Axis3Proto {
    const objectProto: Partial<Axis3Proto> = { metatype: 2400026 };
    objectProto.base = object.base;
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
      base: objectProto.base,
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

  static fromProtoString(packedProtoString: string): Axis3 {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = Axis3Proto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerStructClass(StructType.AXIS3, Axis3);
/* ==== DESTACK_GENERATED_END:STRUCT:2400026 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:2400021 ==== */
/**
 * A 2-dimensional grid configuration value.
 */
export class Grid2 extends StructFrozen {
  static metatype: StructType = StructType.GRID2;
  static __isFrozen__: boolean = true;

  /**
   * Grid2.columns
   */
  readonly columns: number;

  /**
   * Grid2.rows
   */
  readonly rows: number;

  /**
   * Grid2.columnWidth
   */
  readonly columnWidth: Length | null;

  /**
   * Grid2.columnMinWidth
   */
  readonly columnMinWidth: Length | null;

  /**
   * Grid2.rowHeight
   */
  readonly rowHeight: Length | null;

  constructor(options: {
    columns: number;
    rows: number;
    columnWidth?: Length | null;
    columnMinWidth?: Length | null;
    rowHeight?: Length | null;
    _session?: Session | null;
    _supergraph?: Supergraph | null;
    _hash?: number | null;
    _repr?: string | null;
    _proto?: any | null;
    _cson?: any | null;
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
      throw new Error(`Grid2.columns is required`);
    }
    this.columns = _columns;
    let _rows = options.rows;
    if (_rows === null) {
      throw new Error(`Grid2.rows is required`);
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
    this._cson = options._cson ?? null;
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

  repr(): string {
    if (this._repr === null) {
      const propertyReprs: string[] = [];
      propertyReprs.push(`columns=${this.columns}`);
      propertyReprs.push(`rows=${this.rows}`);
      if (this.columnWidth != null) {
        propertyReprs.push(`columnWidth=${this.columnWidth.repr()}`);
      }
      if (this.columnMinWidth != null) {
        propertyReprs.push(`columnMinWidth=${this.columnMinWidth.repr()}`);
      }
      if (this.rowHeight != null) {
        propertyReprs.push(`rowHeight=${this.rowHeight.repr()}`);
      }
      // @ts-expect-error(readonly)
      this._repr = `<Grid2 ${propertyReprs.join(" ")}>`;
    }
    return this._repr;
  }

  hash(): number {
    if (this._hash != null) {
      return this._hash;
    }

    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    h = (h * 31 + hashInt(this.columns)) & 0xffffffff;
    h = (h * 31 + hashInt(this.rows)) & 0xffffffff;
    if (this.columnWidth != null) {
      h = (h * 31 + this.columnWidth.hash()) & 0xffffffff;
    }
    if (this.columnMinWidth != null) {
      h = (h * 31 + this.columnMinWidth.hash()) & 0xffffffff;
    }
    if (this.rowHeight != null) {
      h = (h * 31 + this.rowHeight.hash()) & 0xffffffff;
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
      this._cson = Grid2.__packCson__(this);
    }
    return this._cson;
  }

  static __packCson__(object: Grid2): { [key: string]: any } {
    const objectCson: { [key: string]: any } = {};
    objectCson["1"] = 2400021;
    objectCson["101"] = object.columns;
    objectCson["102"] = object.rows;
    if (object.columnWidth != null) {
      objectCson["103"] = object.columnWidth.toCson();
    }
    if (object.columnMinWidth != null) {
      objectCson["104"] = object.columnMinWidth.toCson();
    }
    if (object.rowHeight != null) {
      objectCson["105"] = object.rowHeight.toCson();
    }
    return objectCson;
  }

  static __unpackCson__(
    objectCson: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Grid2 {
    const _Length = STRUCT_CLASS_BY_TYPE[StructType.LENGTH] as typeof Length;
    const columnWidthValue = objectCson["103"];
    const unpackedColumnWidth =
      columnWidthValue != undefined
        ? _Length.fromCson(columnWidthValue, _session, _supergraph, _graph, _connection)
        : null;
    const columnMinWidthValue = objectCson["104"];
    const unpackedColumnMinWidth =
      columnMinWidthValue != undefined
        ? _Length.fromCson(columnMinWidthValue, _session, _supergraph, _graph, _connection)
        : null;
    const rowHeightValue = objectCson["105"];
    const unpackedRowHeight =
      rowHeightValue != undefined
        ? _Length.fromCson(rowHeightValue, _session, _supergraph, _graph, _connection)
        : null;
    return new Grid2({
      columns: Number(objectCson["101"]),
      rows: Number(objectCson["102"]),
      columnWidth: unpackedColumnWidth,
      columnMinWidth: unpackedColumnMinWidth,
      rowHeight: unpackedRowHeight,
      _cson: objectCson,
      _supergraph,
    });
  }

  static fromCson(
    objectCson: { readonly [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Grid2 {
    return Grid2.__unpackCson__(objectCson, _session, _supergraph, _graph, _connection);
  }

  toProto(): Grid2Proto {
    if (this._proto === null) {
      // @ts-expect-error(readonly)
      this._proto = Grid2.__packProto__(this);
    }
    return this._proto as Grid2Proto;
  }

  static __packProto__(object: Grid2): Grid2Proto {
    const objectProto: Partial<Grid2Proto> = { metatype: 2400021 };
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
    return objectProto as Grid2Proto;
  }

  static __unpackProto__(
    objectProto: Grid2Proto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Grid2 {
    const _Length = STRUCT_CLASS_BY_TYPE[StructType.LENGTH] as typeof Length;
    return new Grid2({
      columns: Number(objectProto.columns),
      rows: Number(objectProto.rows),
      columnWidth:
        objectProto.columnWidth != undefined
          ? _Length.fromProto(objectProto.columnWidth!, _session, _supergraph, _graph, _connection)
          : null,
      columnMinWidth:
        objectProto.columnMinWidth != undefined
          ? _Length.fromProto(
              objectProto.columnMinWidth!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      rowHeight:
        objectProto.rowHeight != undefined
          ? _Length.fromProto(objectProto.rowHeight!, _session, _supergraph, _graph, _connection)
          : null,
      _proto: objectProto,
      _supergraph,
    });
  }

  static fromProto(
    objectProto: Grid2Proto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Grid2 {
    return Grid2.__unpackProto__(objectProto, _session, _supergraph, _graph, _connection);
  }

  static fromProtoString(packedProtoString: string): Grid2 {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = Grid2Proto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerStructClass(StructType.GRID2, Grid2);
/* ==== DESTACK_GENERATED_END:STRUCT:2400021 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:2400022 ==== */
/**
 * A 2-dimensional grid span value.
 */
export class GridSpan2 extends StructFrozen {
  static metatype: StructType = StructType.GRID_SPAN2;
  static __isFrozen__: boolean = true;

  /**
   * GridSpan2.columns
   */
  readonly columns: number;

  /**
   * GridSpan2.rows
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
    _cson?: any | null;
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
      throw new Error(`GridSpan2.columns is required`);
    }
    this.columns = _columns;
    let _rows = options.rows;
    if (_rows === null) {
      throw new Error(`GridSpan2.rows is required`);
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
    this._cson = options._cson ?? null;
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

  repr(): string {
    if (this._repr === null) {
      const propertyReprs: string[] = [];
      propertyReprs.push(`columns=${this.columns}`);
      propertyReprs.push(`rows=${this.rows}`);
      // @ts-expect-error(readonly)
      this._repr = `<GridSpan2 ${propertyReprs.join(" ")}>`;
    }
    return this._repr;
  }

  hash(): number {
    if (this._hash != null) {
      return this._hash;
    }

    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    h = (h * 31 + hashInt(this.columns)) & 0xffffffff;
    h = (h * 31 + hashInt(this.rows)) & 0xffffffff;

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
      this._cson = GridSpan2.__packCson__(this);
    }
    return this._cson;
  }

  static __packCson__(object: GridSpan2): { [key: string]: any } {
    const objectCson: { [key: string]: any } = {};
    objectCson["1"] = 2400022;
    objectCson["101"] = object.columns;
    objectCson["102"] = object.rows;
    return objectCson;
  }

  static __unpackCson__(
    objectCson: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): GridSpan2 {
    return new GridSpan2({
      columns: Number(objectCson["101"]),
      rows: Number(objectCson["102"]),
      _cson: objectCson,
      _supergraph,
    });
  }

  static fromCson(
    objectCson: { readonly [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): GridSpan2 {
    return GridSpan2.__unpackCson__(objectCson, _session, _supergraph, _graph, _connection);
  }

  toProto(): GridSpan2Proto {
    if (this._proto === null) {
      // @ts-expect-error(readonly)
      this._proto = GridSpan2.__packProto__(this);
    }
    return this._proto as GridSpan2Proto;
  }

  static __packProto__(object: GridSpan2): GridSpan2Proto {
    const objectProto: Partial<GridSpan2Proto> = { metatype: 2400022 };
    objectProto.columns = object.columns;
    objectProto.rows = object.rows;
    return objectProto as GridSpan2Proto;
  }

  static __unpackProto__(
    objectProto: GridSpan2Proto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): GridSpan2 {
    return new GridSpan2({
      columns: Number(objectProto.columns),
      rows: Number(objectProto.rows),
      _proto: objectProto,
      _supergraph,
    });
  }

  static fromProto(
    objectProto: GridSpan2Proto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): GridSpan2 {
    return GridSpan2.__unpackProto__(objectProto, _session, _supergraph, _graph, _connection);
  }

  static fromProtoString(packedProtoString: string): GridSpan2 {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = GridSpan2Proto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerStructClass(StructType.GRID_SPAN2, GridSpan2);
/* ==== DESTACK_GENERATED_END:STRUCT:2400022 ==== */
