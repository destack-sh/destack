import type { Float32, PackedObjectCache, Session, UInt16 } from "@destack/language/core";
import { EnumType, StructFrozen, StructType } from "@destack/language/core";
import { registerEnumClass, registerStructClass } from "@destack/language/registry";
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

/* ==== DESTACK_GENERATED_START:STRUCT:2400020 ==== */
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
  readonly value: Float32;

  constructor(options: {
    unit: LengthType;
    value: Float32;
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
    let _unit = options.unit;
    if (_unit == null) {
      throw new Error(`Length.unit is required`);
    }
    this.unit = _unit;
    let _value = options.value;
    if (_value == null) {
      throw new Error(`Length.value is required`);
    }
    this.value = _value;

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
      // @ts-expect-error(readonly) */
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

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerStructClass(StructType.LENGTH, Length);
/* ==== DESTACK_GENERATED_END:STRUCT:2400020 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:2400023 ==== */
/**
 * A 2-dimensional grid span value.
 */
export class GridSpan2 extends StructFrozen {
  static metatype: StructType = StructType.GRID_SPAN2;
  static __isFrozen__: boolean = true;

  /**
   * GridSpan2.columns
   */
  readonly columns: UInt16;

  /**
   * GridSpan2.rows
   */
  readonly rows: UInt16;

  constructor(options: {
    columns: UInt16;
    rows: UInt16;
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
    let _columns = options.columns;
    if (_columns == null) {
      throw new Error(`GridSpan2.columns is required`);
    }
    this.columns = _columns;
    let _rows = options.rows;
    if (_rows == null) {
      throw new Error(`GridSpan2.rows is required`);
    }
    this.rows = _rows;

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
      // @ts-expect-error(readonly) */
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

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerStructClass(StructType.GRID_SPAN2, GridSpan2);
/* ==== DESTACK_GENERATED_END:STRUCT:2400023 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:2400024 ==== */
/**
 * A 2-dimensional insets value (base + side overrides).
 */
export class Inset2 extends StructFrozen {
  static metatype: StructType = StructType.INSET2;
  static __isFrozen__: boolean = true;

  /**
   * Inset2.base
   */
  readonly base: UInt16;

  /**
   * Inset2.top
   */
  readonly top: UInt16 | null;

  /**
   * Inset2.left
   */
  readonly left: UInt16 | null;

  /**
   * Inset2.right
   */
  readonly right: UInt16 | null;

  /**
   * Inset2.bottom
   */
  readonly bottom: UInt16 | null;

  constructor(options: {
    base?: UInt16;
    top?: UInt16 | null;
    left?: UInt16 | null;
    right?: UInt16 | null;
    bottom?: UInt16 | null;
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
    let _base = options.base ?? null;
    if (_base == null) {
      _base = 0;
    }
    if (_base == null) {
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
      // @ts-expect-error(readonly) */
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

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerStructClass(StructType.INSET2, Inset2);
/* ==== DESTACK_GENERATED_END:STRUCT:2400024 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:2400025 ==== */
/**
 * A 2-dimensional corners value (base + corner overrides).
 */
export class Corner2 extends StructFrozen {
  static metatype: StructType = StructType.CORNER2;
  static __isFrozen__: boolean = true;

  /**
   * Corner2.base
   */
  readonly base: UInt16;

  /**
   * Corner2.topLeft
   */
  readonly topLeft: UInt16 | null;

  /**
   * Corner2.topRight
   */
  readonly topRight: UInt16 | null;

  /**
   * Corner2.bottomLeft
   */
  readonly bottomLeft: UInt16 | null;

  /**
   * Corner2.bottomRight
   */
  readonly bottomRight: UInt16 | null;

  constructor(options: {
    base?: UInt16;
    topLeft?: UInt16 | null;
    topRight?: UInt16 | null;
    bottomLeft?: UInt16 | null;
    bottomRight?: UInt16 | null;
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
    let _base = options.base ?? null;
    if (_base == null) {
      _base = 0;
    }
    if (_base == null) {
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
      // @ts-expect-error(readonly) */
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

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerStructClass(StructType.CORNER2, Corner2);
/* ==== DESTACK_GENERATED_END:STRUCT:2400025 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:2400026 ==== */
/**
 * A 2-dimensional axis value (base + x/y overrides).
 */
export class Axis2 extends StructFrozen {
  static metatype: StructType = StructType.AXIS2;
  static __isFrozen__: boolean = true;

  /**
   * Axis2.base
   */
  readonly base: Float32;

  /**
   * Axis2.x
   */
  readonly x: Float32 | null;

  /**
   * Axis2.y
   */
  readonly y: Float32 | null;

  constructor(options: {
    base?: Float32;
    x?: Float32 | null;
    y?: Float32 | null;
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
    let _base = options.base ?? null;
    if (_base == null) {
      _base = 0;
    }
    if (_base == null) {
      throw new Error(`Axis2.base is required`);
    }
    this.base = _base;
    let _x = options.x ?? null;
    this.x = _x;
    let _y = options.y ?? null;
    this.y = _y;

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
      // @ts-expect-error(readonly) */
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

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerStructClass(StructType.AXIS2, Axis2);
/* ==== DESTACK_GENERATED_END:STRUCT:2400026 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:2400021 ==== */
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
      // @ts-expect-error(readonly) */
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

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerStructClass(StructType.OFFSET2, Offset2);
/* ==== DESTACK_GENERATED_END:STRUCT:2400021 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:2400022 ==== */
/**
 * A 2-dimensional grid configuration value.
 */
export class Grid2 extends StructFrozen {
  static metatype: StructType = StructType.GRID2;
  static __isFrozen__: boolean = true;

  /**
   * Grid2.columns
   */
  readonly columns: UInt16;

  /**
   * Grid2.rows
   */
  readonly rows: UInt16;

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
    columns: UInt16;
    rows: UInt16;
    columnWidth?: Length | null;
    columnMinWidth?: Length | null;
    rowHeight?: Length | null;
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
    let _columns = options.columns;
    if (_columns == null) {
      throw new Error(`Grid2.columns is required`);
    }
    this.columns = _columns;
    let _rows = options.rows;
    if (_rows == null) {
      throw new Error(`Grid2.rows is required`);
    }
    this.rows = _rows;
    let _columnWidth = options.columnWidth ?? null;
    this.columnWidth = _columnWidth;
    let _columnMinWidth = options.columnMinWidth ?? null;
    this.columnMinWidth = _columnMinWidth;
    let _rowHeight = options.rowHeight ?? null;
    this.rowHeight = _rowHeight;

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
      // @ts-expect-error(readonly) */
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

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerStructClass(StructType.GRID2, Grid2);
/* ==== DESTACK_GENERATED_END:STRUCT:2400022 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:2400027 ==== */
/**
 * A 3-dimensional axis value (base + x/y/z overrides).
 */
export class Axis3 extends StructFrozen {
  static metatype: StructType = StructType.AXIS3;
  static __isFrozen__: boolean = true;

  /**
   * Axis3.base
   */
  readonly base: Float32;

  /**
   * Axis3.x
   */
  readonly x: Float32 | null;

  /**
   * Axis3.y
   */
  readonly y: Float32 | null;

  /**
   * Axis3.z
   */
  readonly z: Float32 | null;

  constructor(options: {
    base?: Float32;
    x?: Float32 | null;
    y?: Float32 | null;
    z?: Float32 | null;
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
    let _base = options.base ?? null;
    if (_base == null) {
      _base = 0;
    }
    if (_base == null) {
      throw new Error(`Axis3.base is required`);
    }
    this.base = _base;
    let _x = options.x ?? null;
    this.x = _x;
    let _y = options.y ?? null;
    this.y = _y;
    let _z = options.z ?? null;
    this.z = _z;

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
      // @ts-expect-error(readonly) */
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

  /* ==== DESTACK_CUSTOM_START ==== */
  /* ... */
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerStructClass(StructType.AXIS3, Axis3);
/* ==== DESTACK_GENERATED_END:STRUCT:2400027 ==== */
