import { Struct, NodeType, Session, NodeReference, Supergraph, Node, StructType, StructFrozen, QueryConnection, BuiltinObject, EnumType, Graph } from '@/language';
import { Temporal } from 'temporal-polyfill'; // until Temporal ships natively

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

  constructor(
    unit: LengthUnit,
    value: number,
    _supergraph: Supergraph | null
  ) {
    super(_supergraph);
    this.unit = unit;
    this.value = value;
  }


  static create(options: {
    unit: LengthUnit,
    value: number,
    _session?: Session | null,
    _supergraph?: Supergraph | null
  }): Length {
    const session = options._session ?? ACTIVE_SESSION.get();
    const supergraph = options._supergraph ?? session.supergraph;
    return new Length(
      options.unit,
      options.value,
      supergraph
    );
  }

  equals(other: any): boolean {
    throw new Error("Not implemented");
  }

  hash(): number {
    throw new Error("Not implemented");
  }

  validate(): void {
    throw new Error("Not implemented");
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

  constructor(
    type: PositionType,
    top: Length | null,
    left: Length | null,
    width: Length | null,
    height: Length | null,
    _supergraph: Supergraph | null
  ) {
    super(_supergraph);
    this.type = type;
    this.top = top;
    this.left = left;
    this.width = width;
    this.height = height;
  }


  static create(options: {
    type: PositionType,
    top?: Length | null,
    left?: Length | null,
    width?: Length | null,
    height?: Length | null,
    _session?: Session | null,
    _supergraph?: Supergraph | null
  }): Position {
    const session = options._session ?? ACTIVE_SESSION.get();
    const supergraph = options._supergraph ?? session.supergraph;
    return new Position(
      options.type,
      options.top ?? null,
      options.left ?? null,
      options.width ?? null,
      options.height ?? null,
      supergraph
    );
  }

  equals(other: any): boolean {
    throw new Error("Not implemented");
  }

  hash(): number {
    throw new Error("Not implemented");
  }

  validate(): void {
    throw new Error("Not implemented");
  }
}
/* ==== DESTACK_GENERATED_END:STRUCT:12020 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:12022 ==== */
export class Dimension extends StructFrozen {
  readonly type: DimensionType;
  readonly unit: LengthUnit;
  readonly value: number;

  constructor(
    type: DimensionType,
    unit: LengthUnit,
    value: number,
    _supergraph: Supergraph | null
  ) {
    super(_supergraph);
    this.type = type;
    this.unit = unit;
    this.value = value;
  }


  static create(options: {
    type: DimensionType,
    unit: LengthUnit,
    value: number,
    _session?: Session | null,
    _supergraph?: Supergraph | null
  }): Dimension {
    const session = options._session ?? ACTIVE_SESSION.get();
    const supergraph = options._supergraph ?? session.supergraph;
    return new Dimension(
      options.type,
      options.unit,
      options.value,
      supergraph
    );
  }

  equals(other: any): boolean {
    throw new Error("Not implemented");
  }

  hash(): number {
    throw new Error("Not implemented");
  }

  validate(): void {
    throw new Error("Not implemented");
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

  constructor(
    base: number | null,
    top: number | null,
    left: number | null,
    right: number | null,
    bottom: number | null,
    _supergraph: Supergraph | null
  ) {
    super(_supergraph);
    this.base = base;
    this.top = top;
    this.left = left;
    this.right = right;
    this.bottom = bottom;
  }


  static create(options: {
    base?: number | null,
    top?: number | null,
    left?: number | null,
    right?: number | null,
    bottom?: number | null,
    _session?: Session | null,
    _supergraph?: Supergraph | null
  }): Insets {
    const session = options._session ?? ACTIVE_SESSION.get();
    const supergraph = options._supergraph ?? session.supergraph;
    return new Insets(
      options.base ?? null,
      options.top ?? null,
      options.left ?? null,
      options.right ?? null,
      options.bottom ?? null,
      supergraph
    );
  }

  equals(other: any): boolean {
    throw new Error("Not implemented");
  }

  hash(): number {
    throw new Error("Not implemented");
  }

  validate(): void {
    throw new Error("Not implemented");
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

  constructor(
    base: number | null,
    topLeft: number | null,
    topRight: number | null,
    bottomLeft: number | null,
    bottomRight: number | null,
    _supergraph: Supergraph | null
  ) {
    super(_supergraph);
    this.base = base;
    this.topLeft = topLeft;
    this.topRight = topRight;
    this.bottomLeft = bottomLeft;
    this.bottomRight = bottomRight;
  }


  static create(options: {
    base?: number | null,
    topLeft?: number | null,
    topRight?: number | null,
    bottomLeft?: number | null,
    bottomRight?: number | null,
    _session?: Session | null,
    _supergraph?: Supergraph | null
  }): Corners {
    const session = options._session ?? ACTIVE_SESSION.get();
    const supergraph = options._supergraph ?? session.supergraph;
    return new Corners(
      options.base ?? null,
      options.topLeft ?? null,
      options.topRight ?? null,
      options.bottomLeft ?? null,
      options.bottomRight ?? null,
      supergraph
    );
  }

  equals(other: any): boolean {
    throw new Error("Not implemented");
  }

  hash(): number {
    throw new Error("Not implemented");
  }

  validate(): void {
    throw new Error("Not implemented");
  }
}
/* ==== DESTACK_GENERATED_END:STRUCT:12032 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:50207 ==== */
export class Axis2 extends StructFrozen {
  readonly base: number | null;
  readonly x: number | null;
  readonly y: number | null;

  constructor(
    base: number | null,
    x: number | null,
    y: number | null,
    _supergraph: Supergraph | null
  ) {
    super(_supergraph);
    this.base = base;
    this.x = x;
    this.y = y;
  }


  static create(options: {
    base?: number | null,
    x?: number | null,
    y?: number | null,
    _session?: Session | null,
    _supergraph?: Supergraph | null
  }): Axis2 {
    const session = options._session ?? ACTIVE_SESSION.get();
    const supergraph = options._supergraph ?? session.supergraph;
    return new Axis2(
      options.base ?? null,
      options.x ?? null,
      options.y ?? null,
      supergraph
    );
  }

  equals(other: any): boolean {
    throw new Error("Not implemented");
  }

  hash(): number {
    throw new Error("Not implemented");
  }

  validate(): void {
    throw new Error("Not implemented");
  }
}
/* ==== DESTACK_GENERATED_END:STRUCT:50207 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:50209 ==== */
export class Axis3 extends StructFrozen {
  readonly base: number | null;
  readonly x: number | null;
  readonly y: number | null;
  readonly z: number | null;

  constructor(
    base: number | null,
    x: number | null,
    y: number | null,
    z: number | null,
    _supergraph: Supergraph | null
  ) {
    super(_supergraph);
    this.base = base;
    this.x = x;
    this.y = y;
    this.z = z;
  }


  static create(options: {
    base?: number | null,
    x?: number | null,
    y?: number | null,
    z?: number | null,
    _session?: Session | null,
    _supergraph?: Supergraph | null
  }): Axis3 {
    const session = options._session ?? ACTIVE_SESSION.get();
    const supergraph = options._supergraph ?? session.supergraph;
    return new Axis3(
      options.base ?? null,
      options.x ?? null,
      options.y ?? null,
      options.z ?? null,
      supergraph
    );
  }

  equals(other: any): boolean {
    throw new Error("Not implemented");
  }

  hash(): number {
    throw new Error("Not implemented");
  }

  validate(): void {
    throw new Error("Not implemented");
  }
}
/* ==== DESTACK_GENERATED_END:STRUCT:50209 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:50200 ==== */
export class Vector2 extends StructFrozen {
  readonly x: number;
  readonly y: number;

  constructor(
    x: number,
    y: number,
    _supergraph: Supergraph | null
  ) {
    super(_supergraph);
    this.x = x;
    this.y = y;
  }


  static create(options: {
    x: number,
    y: number,
    _session?: Session | null,
    _supergraph?: Supergraph | null
  }): Vector2 {
    const session = options._session ?? ACTIVE_SESSION.get();
    const supergraph = options._supergraph ?? session.supergraph;
    return new Vector2(
      options.x,
      options.y,
      supergraph
    );
  }

  equals(other: any): boolean {
    throw new Error("Not implemented");
  }

  hash(): number {
    throw new Error("Not implemented");
  }

  validate(): void {
    throw new Error("Not implemented");
  }
}
/* ==== DESTACK_GENERATED_END:STRUCT:50200 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:50201 ==== */
export class Vector3 extends StructFrozen {
  readonly x: number;
  readonly y: number;
  readonly z: number;

  constructor(
    x: number,
    y: number,
    z: number,
    _supergraph: Supergraph | null
  ) {
    super(_supergraph);
    this.x = x;
    this.y = y;
    this.z = z;
  }


  static create(options: {
    x: number,
    y: number,
    z: number,
    _session?: Session | null,
    _supergraph?: Supergraph | null
  }): Vector3 {
    const session = options._session ?? ACTIVE_SESSION.get();
    const supergraph = options._supergraph ?? session.supergraph;
    return new Vector3(
      options.x,
      options.y,
      options.z,
      supergraph
    );
  }

  equals(other: any): boolean {
    throw new Error("Not implemented");
  }

  hash(): number {
    throw new Error("Not implemented");
  }

  validate(): void {
    throw new Error("Not implemented");
  }
}
/* ==== DESTACK_GENERATED_END:STRUCT:50201 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:50202 ==== */
export class Vector4 extends StructFrozen {
  readonly x: number;
  readonly y: number;
  readonly z: number;
  readonly w: number;

  constructor(
    x: number,
    y: number,
    z: number,
    w: number,
    _supergraph: Supergraph | null
  ) {
    super(_supergraph);
    this.x = x;
    this.y = y;
    this.z = z;
    this.w = w;
  }


  static create(options: {
    x: number,
    y: number,
    z: number,
    w: number,
    _session?: Session | null,
    _supergraph?: Supergraph | null
  }): Vector4 {
    const session = options._session ?? ACTIVE_SESSION.get();
    const supergraph = options._supergraph ?? session.supergraph;
    return new Vector4(
      options.x,
      options.y,
      options.z,
      options.w,
      supergraph
    );
  }

  equals(other: any): boolean {
    throw new Error("Not implemented");
  }

  hash(): number {
    throw new Error("Not implemented");
  }

  validate(): void {
    throw new Error("Not implemented");
  }
}
/* ==== DESTACK_GENERATED_END:STRUCT:50202 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:50203 ==== */
export class Vector2i extends StructFrozen {
  readonly x: number;
  readonly y: number;

  constructor(
    x: number,
    y: number,
    _supergraph: Supergraph | null
  ) {
    super(_supergraph);
    this.x = x;
    this.y = y;
  }


  static create(options: {
    x: number,
    y: number,
    _session?: Session | null,
    _supergraph?: Supergraph | null
  }): Vector2i {
    const session = options._session ?? ACTIVE_SESSION.get();
    const supergraph = options._supergraph ?? session.supergraph;
    return new Vector2i(
      options.x,
      options.y,
      supergraph
    );
  }

  equals(other: any): boolean {
    throw new Error("Not implemented");
  }

  hash(): number {
    throw new Error("Not implemented");
  }

  validate(): void {
    throw new Error("Not implemented");
  }
}
/* ==== DESTACK_GENERATED_END:STRUCT:50203 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:50204 ==== */
export class Vector3i extends StructFrozen {
  readonly x: number;
  readonly y: number;
  readonly z: number;

  constructor(
    x: number,
    y: number,
    z: number,
    _supergraph: Supergraph | null
  ) {
    super(_supergraph);
    this.x = x;
    this.y = y;
    this.z = z;
  }


  static create(options: {
    x: number,
    y: number,
    z: number,
    _session?: Session | null,
    _supergraph?: Supergraph | null
  }): Vector3i {
    const session = options._session ?? ACTIVE_SESSION.get();
    const supergraph = options._supergraph ?? session.supergraph;
    return new Vector3i(
      options.x,
      options.y,
      options.z,
      supergraph
    );
  }

  equals(other: any): boolean {
    throw new Error("Not implemented");
  }

  hash(): number {
    throw new Error("Not implemented");
  }

  validate(): void {
    throw new Error("Not implemented");
  }
}
/* ==== DESTACK_GENERATED_END:STRUCT:50204 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:50205 ==== */
export class Vector4i extends StructFrozen {
  readonly x: number;
  readonly y: number;
  readonly z: number;
  readonly w: number;

  constructor(
    x: number,
    y: number,
    z: number,
    w: number,
    _supergraph: Supergraph | null
  ) {
    super(_supergraph);
    this.x = x;
    this.y = y;
    this.z = z;
    this.w = w;
  }


  static create(options: {
    x: number,
    y: number,
    z: number,
    w: number,
    _session?: Session | null,
    _supergraph?: Supergraph | null
  }): Vector4i {
    const session = options._session ?? ACTIVE_SESSION.get();
    const supergraph = options._supergraph ?? session.supergraph;
    return new Vector4i(
      options.x,
      options.y,
      options.z,
      options.w,
      supergraph
    );
  }

  equals(other: any): boolean {
    throw new Error("Not implemented");
  }

  hash(): number {
    throw new Error("Not implemented");
  }

  validate(): void {
    throw new Error("Not implemented");
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

  constructor(
    columns: number,
    rows: number,
    columnWidth: Dimension | null,
    columnMinWidth: Dimension | null,
    rowHeight: Dimension | null,
    _supergraph: Supergraph | null
  ) {
    super(_supergraph);
    this.columns = columns;
    this.rows = rows;
    this.columnWidth = columnWidth;
    this.columnMinWidth = columnMinWidth;
    this.rowHeight = rowHeight;
  }


  static create(options: {
    columns: number,
    rows: number,
    columnWidth?: Dimension | null,
    columnMinWidth?: Dimension | null,
    rowHeight?: Dimension | null,
    _session?: Session | null,
    _supergraph?: Supergraph | null
  }): Grid {
    const session = options._session ?? ACTIVE_SESSION.get();
    const supergraph = options._supergraph ?? session.supergraph;
    return new Grid(
      options.columns,
      options.rows,
      options.columnWidth ?? null,
      options.columnMinWidth ?? null,
      options.rowHeight ?? null,
      supergraph
    );
  }

  equals(other: any): boolean {
    throw new Error("Not implemented");
  }

  hash(): number {
    throw new Error("Not implemented");
  }

  validate(): void {
    throw new Error("Not implemented");
  }
}
/* ==== DESTACK_GENERATED_END:STRUCT:12026 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:12028 ==== */
export class GridSpan extends StructFrozen {
  readonly columns: number;
  readonly rows: number;

  constructor(
    columns: number,
    rows: number,
    _supergraph: Supergraph | null
  ) {
    super(_supergraph);
    this.columns = columns;
    this.rows = rows;
  }


  static create(options: {
    columns: number,
    rows: number,
    _session?: Session | null,
    _supergraph?: Supergraph | null
  }): GridSpan {
    const session = options._session ?? ACTIVE_SESSION.get();
    const supergraph = options._supergraph ?? session.supergraph;
    return new GridSpan(
      options.columns,
      options.rows,
      supergraph
    );
  }

  equals(other: any): boolean {
    throw new Error("Not implemented");
  }

  hash(): number {
    throw new Error("Not implemented");
  }

  validate(): void {
    throw new Error("Not implemented");
  }
}
/* ==== DESTACK_GENERATED_END:STRUCT:12028 ==== */