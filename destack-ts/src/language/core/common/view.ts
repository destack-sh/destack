import { Session, StructFrozen, StructType, Supergraph } from "@destack/language/core";

/* ==== DESTACK_GENERATED_START:ENUM:12038 ==== */
/**
 * Layout
 */
export enum Layout {
  STACK = 1,
  GRID = 2,
}
/* ==== DESTACK_GENERATED_END:ENUM:12038 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:12042 ==== */
/**
 * Overflow
 */
export enum Overflow {
  HIDDEN = 2,
  VISIBLE = 3,
  SCROLL = 4,
}
/* ==== DESTACK_GENERATED_END:ENUM:12042 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:12041 ==== */
/**
 * Direction
 */
export enum Direction {
  HORIZONTAL = 1,
  VERTICAL = 2,
}
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
}
/* ==== DESTACK_GENERATED_END:ENUM:12039 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:12040 ==== */
/**
 * Align
 */
export enum Align {
  START = 1,
  CENTER = 2,
  END = 3,
}
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
}
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
}
/* ==== DESTACK_GENERATED_END:ENUM:12000 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:12045 ==== */
/**
 * DimensionType
 */
export enum DimensionType {
  FIXED = 2,
  FIT = 3,
  FILL = 4,
}
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
}
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
}
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
}
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
}
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
}
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
}
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
}
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

  constructor(options: { x: number; y: number; _session?: Session | null; _supergraph?: Supergraph | null }) {
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
}
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
}
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
}
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

  constructor(options: { x: number; y: number; _session?: Session | null; _supergraph?: Supergraph | null }) {
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
}
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
}
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
}
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
}
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

  constructor(options: { columns: number; rows: number; _session?: Session | null; _supergraph?: Supergraph | null }) {
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
}
/* ==== DESTACK_GENERATED_END:STRUCT:12028 ==== */
