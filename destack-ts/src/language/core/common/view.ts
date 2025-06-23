import { EnumType, Node, Supergraph, activeSession, Session, NodeType, Struct, QueryConnection, StructFrozen, ACTIVE_SESSION, StructType, NodeReference, BuiltinObject, Graph } from '@/language';

/* ==== DESTACK_GENERATED_START:ENUM:12038 ==== */
export enum Layout {
  STACK = 1,
  GRID = 2,
}
/* ==== DESTACK_GENERATED_END:ENUM:12038 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:12042 ==== */
export enum Overflow {
  HIDDEN = 2,
  VISIBLE = 3,
  SCROLL = 4,
}
/* ==== DESTACK_GENERATED_END:ENUM:12042 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:12041 ==== */
export enum Direction {
  HORIZONTAL = 1,
  VERTICAL = 2,
}
/* ==== DESTACK_GENERATED_END:ENUM:12041 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:12039 ==== */
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
export enum Align {
  START = 1,
  CENTER = 2,
  END = 3,
}
/* ==== DESTACK_GENERATED_END:ENUM:12040 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:12037 ==== */
export enum LengthUnit {
  PIXEL = 1,
  REM = 2,
  PERCENT = 3,
  FR = 4,
}
/* ==== DESTACK_GENERATED_END:ENUM:12037 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:12000 ==== */
export enum PositionType {
  RELATIVE = 1,
  ABSOLUTE = 2,
  FIXED = 3,
  STICKY = 4,
}
/* ==== DESTACK_GENERATED_END:ENUM:12000 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:12045 ==== */
export enum DimensionType {
  FIXED = 2,
  FIT = 3,
  FILL = 4,
}
/* ==== DESTACK_GENERATED_END:ENUM:12045 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:12018 ==== */
export class Length extends StructFrozen {
  readonly unit: LengthUnit;
  readonly value: number;

  constructor(options: {
    unit: LengthUnit,
    value: number,
    _session?: Session | null,
    _supergraph?: Supergraph | null
  }) {
    super(
        // supergraph
        options._supergraph ?? null,
    );

    this.unit = options.unit;
    this.value = options.value;
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
export class Position extends StructFrozen {
  readonly type: PositionType;
  readonly top: Length | null;
  readonly left: Length | null;
  readonly width: Length | null;
  readonly height: Length | null;

  constructor(options: {
    type: PositionType,
    top?: Length | null,
    left?: Length | null,
    width?: Length | null,
    height?: Length | null,
    _session?: Session | null,
    _supergraph?: Supergraph | null
  }) {
    super(
        // supergraph
        options._supergraph ?? null,
    );

    this.type = options.type;
    this.top = options.top ?? null;
    this.left = options.left ?? null;
    this.width = options.width ?? null;
    this.height = options.height ?? null;
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
export class Dimension extends StructFrozen {
  readonly type: DimensionType;
  readonly unit: LengthUnit;
  readonly value: number;

  constructor(options: {
    type: DimensionType,
    unit: LengthUnit,
    value: number,
    _session?: Session | null,
    _supergraph?: Supergraph | null
  }) {
    super(
        // supergraph
        options._supergraph ?? null,
    );

    this.type = options.type;
    this.unit = options.unit;
    this.value = options.value;
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
export class Insets extends StructFrozen {
  readonly base: number | null;
  readonly top: number | null;
  readonly left: number | null;
  readonly right: number | null;
  readonly bottom: number | null;

  constructor(options: {
    base?: number | null,
    top?: number | null,
    left?: number | null,
    right?: number | null,
    bottom?: number | null,
    _session?: Session | null,
    _supergraph?: Supergraph | null
  }) {
    super(
        // supergraph
        options._supergraph ?? null,
    );

    this.base = options.base ?? null;
    this.top = options.top ?? null;
    this.left = options.left ?? null;
    this.right = options.right ?? null;
    this.bottom = options.bottom ?? null;
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
export class Corners extends StructFrozen {
  readonly base: number | null;
  readonly topLeft: number | null;
  readonly topRight: number | null;
  readonly bottomLeft: number | null;
  readonly bottomRight: number | null;

  constructor(options: {
    base?: number | null,
    topLeft?: number | null,
    topRight?: number | null,
    bottomLeft?: number | null,
    bottomRight?: number | null,
    _session?: Session | null,
    _supergraph?: Supergraph | null
  }) {
    super(
        // supergraph
        options._supergraph ?? null,
    );

    this.base = options.base ?? null;
    this.topLeft = options.topLeft ?? null;
    this.topRight = options.topRight ?? null;
    this.bottomLeft = options.bottomLeft ?? null;
    this.bottomRight = options.bottomRight ?? null;
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
export class Axis2 extends StructFrozen {
  readonly base: number | null;
  readonly x: number | null;
  readonly y: number | null;

  constructor(options: {
    base?: number | null,
    x?: number | null,
    y?: number | null,
    _session?: Session | null,
    _supergraph?: Supergraph | null
  }) {
    super(
        // supergraph
        options._supergraph ?? null,
    );

    this.base = options.base ?? null;
    this.x = options.x ?? null;
    this.y = options.y ?? null;
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
export class Axis3 extends StructFrozen {
  readonly base: number | null;
  readonly x: number | null;
  readonly y: number | null;
  readonly z: number | null;

  constructor(options: {
    base?: number | null,
    x?: number | null,
    y?: number | null,
    z?: number | null,
    _session?: Session | null,
    _supergraph?: Supergraph | null
  }) {
    super(
        // supergraph
        options._supergraph ?? null,
    );

    this.base = options.base ?? null;
    this.x = options.x ?? null;
    this.y = options.y ?? null;
    this.z = options.z ?? null;
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
export class Vector2 extends StructFrozen {
  readonly x: number;
  readonly y: number;

  constructor(options: {
    x: number,
    y: number,
    _session?: Session | null,
    _supergraph?: Supergraph | null
  }) {
    super(
        // supergraph
        options._supergraph ?? null,
    );

    this.x = options.x;
    this.y = options.y;
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
export class Vector3 extends StructFrozen {
  readonly x: number;
  readonly y: number;
  readonly z: number;

  constructor(options: {
    x: number,
    y: number,
    z: number,
    _session?: Session | null,
    _supergraph?: Supergraph | null
  }) {
    super(
        // supergraph
        options._supergraph ?? null,
    );

    this.x = options.x;
    this.y = options.y;
    this.z = options.z;
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
export class Vector4 extends StructFrozen {
  readonly x: number;
  readonly y: number;
  readonly z: number;
  readonly w: number;

  constructor(options: {
    x: number,
    y: number,
    z: number,
    w: number,
    _session?: Session | null,
    _supergraph?: Supergraph | null
  }) {
    super(
        // supergraph
        options._supergraph ?? null,
    );

    this.x = options.x;
    this.y = options.y;
    this.z = options.z;
    this.w = options.w;
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
export class Vector2i extends StructFrozen {
  readonly x: number;
  readonly y: number;

  constructor(options: {
    x: number,
    y: number,
    _session?: Session | null,
    _supergraph?: Supergraph | null
  }) {
    super(
        // supergraph
        options._supergraph ?? null,
    );

    this.x = options.x;
    this.y = options.y;
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
export class Vector3i extends StructFrozen {
  readonly x: number;
  readonly y: number;
  readonly z: number;

  constructor(options: {
    x: number,
    y: number,
    z: number,
    _session?: Session | null,
    _supergraph?: Supergraph | null
  }) {
    super(
        // supergraph
        options._supergraph ?? null,
    );

    this.x = options.x;
    this.y = options.y;
    this.z = options.z;
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
export class Vector4i extends StructFrozen {
  readonly x: number;
  readonly y: number;
  readonly z: number;
  readonly w: number;

  constructor(options: {
    x: number,
    y: number,
    z: number,
    w: number,
    _session?: Session | null,
    _supergraph?: Supergraph | null
  }) {
    super(
        // supergraph
        options._supergraph ?? null,
    );

    this.x = options.x;
    this.y = options.y;
    this.z = options.z;
    this.w = options.w;
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
export class Grid extends StructFrozen {
  readonly columns: number;
  readonly rows: number;
  readonly columnWidth: Dimension | null;
  readonly columnMinWidth: Dimension | null;
  readonly rowHeight: Dimension | null;

  constructor(options: {
    columns: number,
    rows: number,
    columnWidth?: Dimension | null,
    columnMinWidth?: Dimension | null,
    rowHeight?: Dimension | null,
    _session?: Session | null,
    _supergraph?: Supergraph | null
  }) {
    super(
        // supergraph
        options._supergraph ?? null,
    );

    this.columns = options.columns;
    this.rows = options.rows;
    this.columnWidth = options.columnWidth ?? null;
    this.columnMinWidth = options.columnMinWidth ?? null;
    this.rowHeight = options.rowHeight ?? null;
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
export class GridSpan extends StructFrozen {
  readonly columns: number;
  readonly rows: number;

  constructor(options: {
    columns: number,
    rows: number,
    _session?: Session | null,
    _supergraph?: Supergraph | null
  }) {
    super(
        // supergraph
        options._supergraph ?? null,
    );

    this.columns = options.columns;
    this.rows = options.rows;
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