import type { Float32, PackedCache, Session, SInt32 } from "@destack/language/core";
import { StructFrozen, StructType } from "@destack/language/core";
import { registerStructClass } from "@destack/language/registry";
import { hashFloat, hashInt } from "@destack/utils/hash";

/* ==== DESTACK_GENERATED_START:STRUCT:2400000 ==== */
/**
 * A 2D floating point Vector.
 */
export class Vector2 extends StructFrozen {
  static metatype: StructType = StructType.VECTOR2;
  static __isFrozen__: boolean = true;

  /**
   * The x-coordinate of the Vector2.
   */
  readonly x: Float32;

  /**
   * The y-coordinate of the Vector2.
   */
  readonly y: Float32;

  constructor(options: {
    x: Float32;
    y: Float32;
    _session?: Session | null;
    _hash?: number | null;
    _repr?: string | null;
    _packedCache?: PackedCache[] | null;
  }) {
    /* super */
    super(
      /* session */
      options._session ?? null,
    );

    /* properties */
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

    /* identity */
    // @ts-expect-error(readonly)
    this._hash = options._hash ?? null;
    // @ts-expect-error(readonly)
    this._repr = options._repr ?? null;
    // @ts-expect-error(readonly)
    this._packedCache = options._packedCache ?? null;
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

  repr(): string {
    if (this._repr === null) {
      const propertyReprs: string[] = [];
      propertyReprs.push(`x=${this.x}`);
      propertyReprs.push(`y=${this.y}`);
      // @ts-expect-error(readonly) */
      this._repr = `<Vector2 ${propertyReprs.join(" ")}>`;
    }
    return this._repr;
  }

  hash(): number {
    if (this._hash != null) {
      return this._hash;
    }
    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    h = (h * 31 + hashFloat(this.x)) & 0xffffffff;
    h = (h * 31 + hashFloat(this.y)) & 0xffffffff;
    // @ts-expect-error(readonly)
    this._hash = h;
    return h;
  }

  validate(): void {
    throw new Error("not implemented");
  }

  /* ==== DESTACK_CUSTOM_START ==== */

  /** Add two vectors or a vector and a scalar. */
  add(other: Vector2 | number): Vector2 {
    if (typeof other === "number") {
      return new Vector2({
        x: this.x + other,
        y: this.y + other,
      });
    } else {
      return new Vector2({
        x: this.x + other.x,
        y: this.y + other.y,
      });
    }
  }

  /** Subtract two vectors or a vector and a scalar. */
  sub(other: Vector2 | number): Vector2 {
    if (typeof other === "number") {
      return new Vector2({
        x: this.x - other,
        y: this.y - other,
      });
    } else {
      return new Vector2({
        x: this.x - other.x,
        y: this.y - other.y,
      });
    }
  }

  /** Multiply two vectors or a vector and a scalar. */
  mul(other: Vector2 | number): Vector2 {
    if (typeof other === "number") {
      return new Vector2({
        x: this.x * other,
        y: this.y * other,
      });
    } else {
      return new Vector2({
        x: this.x * other.x,
        y: this.y * other.y,
      });
    }
  }

  /** Divide two vectors or a vector and a scalar. */
  div(other: Vector2 | number): Vector2 {
    if (typeof other === "number") {
      return new Vector2({
        x: this.x / other,
        y: this.y / other,
      });
    } else {
      return new Vector2({
        x: this.x / other.x,
        y: this.y / other.y,
      });
    }
  }

  /** Negate a vector. */
  neg(): Vector2 {
    return new Vector2({
      x: -this.x,
      y: -this.y,
    });
  }

  /**
   * Get the perpendicular vector (rotated 90 degrees counterclockwise).
   */
  per(): Vector2 {
    return new Vector2({
      x: this.y,
      y: -this.x,
    });
  }

  /** Get the absolute value of a vector. */
  abs(): Vector2 {
    if (this.x >= 0 && this.y >= 0) {
      return this;
    } else {
      return new Vector2({
        x: Math.abs(this.x),
        y: Math.abs(this.y),
      });
    }
  }

  /**
   * Calculate the dot product with another vector.
   */
  dot(other: Vector2): number {
    return this.x * other.x + this.y * other.y;
  }

  /**
   * Calculate the linear interpolation between two vectors.
   */
  lerp(other: Vector2, t: number): Vector2 {
    return new Vector2({
      x: this.x + (other.x - this.x) * t,
      y: this.y + (other.y - this.y) * t,
    });
  }

  /**
   * Calculate the magnitude (length) of the vector.
   */
  magnitude(): number {
    return Math.sqrt(this.x * this.x + this.y * this.y);
  }

  /**
   * Return a normalized (unit) vector.
   */
  normalize(): Vector2 {
    const mag = this.magnitude();
    if (mag === 0) {
      return new Vector2({ x: 0.0, y: 0.0 });
    }
    return new Vector2({
      x: this.x / mag,
      y: this.y / mag,
    });
  }

  /**
   * Calculate the squared distance to another vector.
   */
  distance2(other: Vector2): number {
    const dx = this.x - other.x;
    const dy = this.y - other.y;
    return dx * dx + dy * dy;
  }

  /**
   * Calculate the distance to another vector.
   */
  distance(other: Vector2): number {
    return Math.sqrt(this.distance2(other));
  }

  /**
   * Rotate this vector around another point by the given angle.
   */
  rotWith(center: Vector2, angle: number): Vector2 {
    const x = this.x - center.x;
    const y = this.y - center.y;
    const s = Math.sin(angle);
    const c = Math.cos(angle);
    return new Vector2({
      x: center.x + (x * c - y * s),
      y: center.y + (x * s + y * c),
    });
  }

  /* ==== DESTACK_CUSTOM_END ==== */
}
registerStructClass(StructType.VECTOR2, Vector2);
/* ==== DESTACK_GENERATED_END:STRUCT:2400000 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:2400002 ==== */
/**
 * A 3D floating point vector.
 */
export class Vector3 extends StructFrozen {
  static metatype: StructType = StructType.VECTOR3;
  static __isFrozen__: boolean = true;

  /**
   * The x-coordinate of the Vector3.
   */
  readonly x: Float32;

  /**
   * The y-coordinate of the Vector3.
   */
  readonly y: Float32;

  /**
   * The z-coordinate of the Vector3.
   */
  readonly z: Float32;

  constructor(options: {
    x: Float32;
    y: Float32;
    z: Float32;
    _session?: Session | null;
    _hash?: number | null;
    _repr?: string | null;
    _packedCache?: PackedCache[] | null;
  }) {
    /* super */
    super(
      /* session */
      options._session ?? null,
    );

    /* properties */
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

    /* identity */
    // @ts-expect-error(readonly)
    this._hash = options._hash ?? null;
    // @ts-expect-error(readonly)
    this._repr = options._repr ?? null;
    // @ts-expect-error(readonly)
    this._packedCache = options._packedCache ?? null;
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

  repr(): string {
    if (this._repr === null) {
      const propertyReprs: string[] = [];
      propertyReprs.push(`x=${this.x}`);
      propertyReprs.push(`y=${this.y}`);
      propertyReprs.push(`z=${this.z}`);
      // @ts-expect-error(readonly) */
      this._repr = `<Vector3 ${propertyReprs.join(" ")}>`;
    }
    return this._repr;
  }

  hash(): number {
    if (this._hash != null) {
      return this._hash;
    }
    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    h = (h * 31 + hashFloat(this.x)) & 0xffffffff;
    h = (h * 31 + hashFloat(this.y)) & 0xffffffff;
    h = (h * 31 + hashFloat(this.z)) & 0xffffffff;
    // @ts-expect-error(readonly)
    this._hash = h;
    return h;
  }

  validate(): void {
    throw new Error("not implemented");
  }

  /* ==== DESTACK_CUSTOM_START ==== */

  /** Add two vectors or a vector and a scalar. */
  add(other: Vector3 | number): Vector3 {
    if (typeof other === "number") {
      return new Vector3({
        x: this.x + other,
        y: this.y + other,
        z: this.z + other,
      });
    } else {
      return new Vector3({
        x: this.x + other.x,
        y: this.y + other.y,
        z: this.z + other.z,
      });
    }
  }

  /** Subtract two vectors or a vector and a scalar. */
  sub(other: Vector3 | number): Vector3 {
    if (typeof other === "number") {
      return new Vector3({
        x: this.x - other,
        y: this.y - other,
        z: this.z - other,
      });
    } else {
      return new Vector3({
        x: this.x - other.x,
        y: this.y - other.y,
        z: this.z - other.z,
      });
    }
  }

  /** Multiply two vectors or a vector and a scalar. */
  mul(other: Vector3 | number): Vector3 {
    if (typeof other === "number") {
      return new Vector3({
        x: this.x * other,
        y: this.y * other,
        z: this.z * other,
      });
    } else {
      return new Vector3({
        x: this.x * other.x,
        y: this.y * other.y,
        z: this.z * other.z,
      });
    }
  }

  /** Divide two vectors or a vector and a scalar. */
  div(other: Vector3 | number): Vector3 {
    if (typeof other === "number") {
      return new Vector3({
        x: this.x / other,
        y: this.y / other,
        z: this.z / other,
      });
    } else {
      return new Vector3({
        x: this.x / other.x,
        y: this.y / other.y,
        z: this.z / other.z,
      });
    }
  }

  /** Negate a vector. */
  neg(): Vector3 {
    return new Vector3({
      x: -this.x,
      y: -this.y,
      z: -this.z,
    });
  }

  /**
   * Get the perpendicular vector (rotated 90 degrees counterclockwise).
   */
  per(): Vector3 {
    return new Vector3({
      x: this.y,
      y: -this.x,
      z: 0,
    });
  }

  /** Get the absolute value of a vector. */
  abs(): Vector3 {
    if (this.x >= 0 && this.y >= 0 && this.z >= 0) {
      return this;
    } else {
      return new Vector3({
        x: Math.abs(this.x),
        y: Math.abs(this.y),
        z: Math.abs(this.z),
      });
    }
  }

  /**
   * Calculate the dot product with another vector.
   */
  dot(other: Vector3): number {
    return this.x * other.x + this.y * other.y + this.z * other.z;
  }

  /**
   * Calculate the cross product with another vector.
   */
  cross(other: Vector3): Vector3 {
    return new Vector3({
      x: this.y * other.z - this.z * other.y,
      y: this.z * other.x - this.x * other.z,
      z: this.x * other.y - this.y * other.x,
    });
  }

  /**
   * Calculate the linear interpolation between two vectors.
   */
  lerp(other: Vector3, t: number): Vector3 {
    return new Vector3({
      x: this.x + (other.x - this.x) * t,
      y: this.y + (other.y - this.y) * t,
      z: this.z + (other.z - this.z) * t,
    });
  }

  /**
   * Calculate the magnitude (length) of the vector.
   */
  magnitude(): number {
    return Math.sqrt(this.x * this.x + this.y * this.y + this.z * this.z);
  }

  /**
   * Return a normalized (unit) vector.
   */
  normalize(): Vector3 {
    const mag = this.magnitude();
    if (mag === 0) {
      return new Vector3({ x: 0.0, y: 0.0, z: 0.0 });
    }
    return new Vector3({
      x: this.x / mag,
      y: this.y / mag,
      z: this.z / mag,
    });
  }

  /**
   * Calculate the squared distance to another vector.
   */
  distance2(other: Vector3): number {
    const dx = this.x - other.x;
    const dy = this.y - other.y;
    const dz = this.z - other.z;
    return dx * dx + dy * dy + dz * dz;
  }

  /**
   * Calculate the distance to another vector.
   */
  distance(other: Vector3): number {
    return Math.sqrt(this.distance2(other));
  }

  /**
   * Rotate this vector around another point by the given angle.
   */
  rotWith(center: Vector3, angle: number): Vector3 {
    const x = this.x - center.x;
    const y = this.y - center.y;
    const s = Math.sin(angle);
    const c = Math.cos(angle);
    return new Vector3({
      x: center.x + (x * c - y * s),
      y: center.y + (x * s + y * c),
      z: this.z,
    });
  }

  /* ==== DESTACK_CUSTOM_END ==== */
}
registerStructClass(StructType.VECTOR3, Vector3);
/* ==== DESTACK_GENERATED_END:STRUCT:2400002 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:2400004 ==== */
/**
 * A 4D floating point vector.
 */
export class Vector4 extends StructFrozen {
  static metatype: StructType = StructType.VECTOR4;
  static __isFrozen__: boolean = true;

  /**
   * The x-coordinate of the Vector4.
   */
  readonly x: Float32;

  /**
   * The y-coordinate of the Vector4.
   */
  readonly y: Float32;

  /**
   * The z-coordinate of the Vector4.
   */
  readonly z: Float32;

  /**
   * The w-coordinate of the Vector4.
   */
  readonly w: Float32;

  constructor(options: {
    x: Float32;
    y: Float32;
    z: Float32;
    w: Float32;
    _session?: Session | null;
    _hash?: number | null;
    _repr?: string | null;
    _packedCache?: PackedCache[] | null;
  }) {
    /* super */
    super(
      /* session */
      options._session ?? null,
    );

    /* properties */
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

    /* identity */
    // @ts-expect-error(readonly)
    this._hash = options._hash ?? null;
    // @ts-expect-error(readonly)
    this._repr = options._repr ?? null;
    // @ts-expect-error(readonly)
    this._packedCache = options._packedCache ?? null;
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

  repr(): string {
    if (this._repr === null) {
      const propertyReprs: string[] = [];
      propertyReprs.push(`x=${this.x}`);
      propertyReprs.push(`y=${this.y}`);
      propertyReprs.push(`z=${this.z}`);
      propertyReprs.push(`w=${this.w}`);
      // @ts-expect-error(readonly) */
      this._repr = `<Vector4 ${propertyReprs.join(" ")}>`;
    }
    return this._repr;
  }

  hash(): number {
    if (this._hash != null) {
      return this._hash;
    }
    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    h = (h * 31 + hashFloat(this.x)) & 0xffffffff;
    h = (h * 31 + hashFloat(this.y)) & 0xffffffff;
    h = (h * 31 + hashFloat(this.z)) & 0xffffffff;
    h = (h * 31 + hashFloat(this.w)) & 0xffffffff;
    // @ts-expect-error(readonly)
    this._hash = h;
    return h;
  }

  validate(): void {
    throw new Error("not implemented");
  }

  /* ==== DESTACK_CUSTOM_START ==== */

  /** Add two vectors or a vector and a scalar. */
  add(other: Vector4 | number): Vector4 {
    if (typeof other === "number") {
      return new Vector4({
        x: this.x + other,
        y: this.y + other,
        z: this.z + other,
        w: this.w + other,
      });
    } else {
      return new Vector4({
        x: this.x + other.x,
        y: this.y + other.y,
        z: this.z + other.z,
        w: this.w + other.w,
      });
    }
  }

  /** Subtract two vectors or a vector and a scalar. */
  sub(other: Vector4 | number): Vector4 {
    if (typeof other === "number") {
      return new Vector4({
        x: this.x - other,
        y: this.y - other,
        z: this.z - other,
        w: this.w - other,
      });
    } else {
      return new Vector4({
        x: this.x - other.x,
        y: this.y - other.y,
        z: this.z - other.z,
        w: this.w - other.w,
      });
    }
  }

  /** Multiply two vectors or a vector and a scalar. */
  mul(other: Vector4 | number): Vector4 {
    if (typeof other === "number") {
      return new Vector4({
        x: this.x * other,
        y: this.y * other,
        z: this.z * other,
        w: this.w * other,
      });
    } else {
      return new Vector4({
        x: this.x * other.x,
        y: this.y * other.y,
        z: this.z * other.z,
        w: this.w * other.w,
      });
    }
  }

  /** Divide two vectors or a vector and a scalar. */
  div(other: Vector4 | number): Vector4 {
    if (typeof other === "number") {
      return new Vector4({
        x: this.x / other,
        y: this.y / other,
        z: this.z / other,
        w: this.w / other,
      });
    } else {
      return new Vector4({
        x: this.x / other.x,
        y: this.y / other.y,
        z: this.z / other.z,
        w: this.w / other.w,
      });
    }
  }

  /** Negate a vector. */
  neg(): Vector4 {
    return new Vector4({
      x: -this.x,
      y: -this.y,
      z: -this.z,
      w: -this.w,
    });
  }

  /**
   * Get the perpendicular vector (rotated 90 degrees counterclockwise).
   */
  per(): Vector4 {
    return new Vector4({
      x: this.y,
      y: -this.x,
      z: 0,
      w: 0,
    });
  }

  /** Get the absolute value of a vector. */
  abs(): Vector4 {
    if (this.x >= 0 && this.y >= 0 && this.z >= 0 && this.w >= 0) {
      return this;
    } else {
      return new Vector4({
        x: Math.abs(this.x),
        y: Math.abs(this.y),
        z: Math.abs(this.z),
        w: Math.abs(this.w),
      });
    }
  }

  /**
   * Calculate the dot product with another vector.
   */
  dot(other: Vector4): number {
    return this.x * other.x + this.y * other.y + this.z * other.z + this.w * other.w;
  }

  /**
   * Calculate the cross product with another vector.
   */
  cross(other: Vector4): Vector4 {
    return new Vector4({
      x: this.y * other.z - this.z * other.y,
      y: this.z * other.x - this.x * other.z,
      z: this.x * other.y - this.y * other.x,
      w: 0,
    });
  }

  /**
   * Calculate the linear interpolation between two vectors.
   */
  lerp(other: Vector4, t: number): Vector4 {
    return new Vector4({
      x: this.x + (other.x - this.x) * t,
      y: this.y + (other.y - this.y) * t,
      z: this.z + (other.z - this.z) * t,
      w: this.w + (other.w - this.w) * t,
    });
  }

  /**
   * Calculate the magnitude (length) of the vector.
   */
  magnitude(): number {
    return Math.sqrt(this.x * this.x + this.y * this.y + this.z * this.z + this.w * this.w);
  }

  /**
   * Return a normalized (unit) vector.
   */
  normalize(): Vector4 {
    const mag = this.magnitude();
    if (mag === 0) {
      return new Vector4({ x: 0.0, y: 0.0, z: 0.0, w: 0.0 });
    }
    return new Vector4({
      x: this.x / mag,
      y: this.y / mag,
      z: this.z / mag,
      w: this.w / mag,
    });
  }

  /**
   * Calculate the squared distance to another vector.
   */
  distance2(other: Vector4): number {
    const dx = this.x - other.x;
    const dy = this.y - other.y;
    const dz = this.z - other.z;
    const dw = this.w - other.w;
    return dx * dx + dy * dy + dz * dz + dw * dw;
  }

  /**
   * Calculate the distance to another vector.
   */
  distance(other: Vector4): number {
    return Math.sqrt(this.distance2(other));
  }

  /**
   * Rotate this vector around another point by the given angle.
   */
  rotWith(center: Vector4, angle: number): Vector4 {
    const x = this.x - center.x;
    const y = this.y - center.y;
    const s = Math.sin(angle);
    const c = Math.cos(angle);
    return new Vector4({
      x: center.x + (x * c - y * s),
      y: center.y + (x * s + y * c),
      z: this.z,
      w: this.w,
    });
  }

  /* ==== DESTACK_CUSTOM_END ==== */
}
registerStructClass(StructType.VECTOR4, Vector4);
/* ==== DESTACK_GENERATED_END:STRUCT:2400004 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:2400001 ==== */
/**
 * A 2D integer vector.
 */
export class Vector2i extends StructFrozen {
  static metatype: StructType = StructType.VECTOR2I;
  static __isFrozen__: boolean = true;

  /**
   * The x-coordinate of the Vector2i.
   */
  readonly x: SInt32;

  /**
   * The y-coordinate of the Vector2i.
   */
  readonly y: SInt32;

  constructor(options: {
    x: SInt32;
    y: SInt32;
    _session?: Session | null;
    _hash?: number | null;
    _repr?: string | null;
    _packedCache?: PackedCache[] | null;
  }) {
    /* super */
    super(
      /* session */
      options._session ?? null,
    );

    /* properties */
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

    /* identity */
    // @ts-expect-error(readonly)
    this._hash = options._hash ?? null;
    // @ts-expect-error(readonly)
    this._repr = options._repr ?? null;
    // @ts-expect-error(readonly)
    this._packedCache = options._packedCache ?? null;
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

  repr(): string {
    if (this._repr === null) {
      const propertyReprs: string[] = [];
      propertyReprs.push(`x=${this.x}`);
      propertyReprs.push(`y=${this.y}`);
      // @ts-expect-error(readonly) */
      this._repr = `<Vector2i ${propertyReprs.join(" ")}>`;
    }
    return this._repr;
  }

  hash(): number {
    if (this._hash != null) {
      return this._hash;
    }
    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    h = (h * 31 + hashInt(this.x)) & 0xffffffff;
    h = (h * 31 + hashInt(this.y)) & 0xffffffff;
    // @ts-expect-error(readonly)
    this._hash = h;
    return h;
  }

  validate(): void {
    throw new Error("not implemented");
  }

  /* ==== DESTACK_CUSTOM_START ==== */

  /** Add two vectors or a vector and a scalar. */
  add(other: Vector2i | number): Vector2i {
    if (typeof other === "number") {
      return new Vector2i({
        x: this.x + other,
        y: this.y + other,
      });
    } else {
      return new Vector2i({
        x: this.x + other.x,
        y: this.y + other.y,
      });
    }
  }

  /** Subtract two vectors or a vector and a scalar. */
  sub(other: Vector2i | number): Vector2i {
    if (typeof other === "number") {
      return new Vector2i({
        x: this.x - other,
        y: this.y - other,
      });
    } else {
      return new Vector2i({
        x: this.x - other.x,
        y: this.y - other.y,
      });
    }
  }

  /** Multiply two vectors or a vector and a scalar. */
  mul(other: Vector2i | number): Vector2i {
    if (typeof other === "number") {
      return new Vector2i({
        x: this.x * other,
        y: this.y * other,
      });
    } else {
      return new Vector2i({
        x: this.x * other.x,
        y: this.y * other.y,
      });
    }
  }

  /** Divide two vectors or a vector and a scalar. */
  div(other: Vector2i | number): Vector2i {
    if (typeof other === "number") {
      return new Vector2i({
        x: Math.floor(this.x / other),
        y: Math.floor(this.y / other),
      });
    } else {
      return new Vector2i({
        x: Math.floor(this.x / other.x),
        y: Math.floor(this.y / other.y),
      });
    }
  }

  /** Negate a vector. */
  neg(): Vector2i {
    return new Vector2i({
      x: -this.x,
      y: -this.y,
    });
  }

  /**
   * Get the perpendicular vector (rotated 90 degrees counterclockwise).
   */
  per(): Vector2i {
    return new Vector2i({
      x: this.y,
      y: -this.x,
    });
  }

  /** Get the absolute value of a vector. */
  abs(): Vector2i {
    if (this.x >= 0 && this.y >= 0) {
      return this;
    } else {
      return new Vector2i({
        x: Math.abs(this.x),
        y: Math.abs(this.y),
      });
    }
  }

  /**
   * Calculate the dot product with another vector.
   */
  dot(other: Vector2i): number {
    return this.x * other.x + this.y * other.y;
  }

  /**
   * Calculate the linear interpolation between two vectors.
   */
  lerp(other: Vector2i, t: number): Vector2i {
    return new Vector2i({
      x: Math.floor(this.x + (other.x - this.x) * t),
      y: Math.floor(this.y + (other.y - this.y) * t),
    });
  }

  /**
   * Calculate the magnitude (length) of the vector.
   */
  magnitude(): number {
    return Math.sqrt(this.x * this.x + this.y * this.y);
  }

  /**
   * Return a normalized (unit) vector.
   */
  normalize(): Vector2i {
    const mag = this.magnitude();
    if (mag === 0) {
      return new Vector2i({ x: 0, y: 0 });
    }
    return new Vector2i({
      x: Math.floor(this.x / mag),
      y: Math.floor(this.y / mag),
    });
  }

  /**
   * Calculate the squared distance to another vector.
   */
  distance2(other: Vector2i): number {
    const dx = this.x - other.x;
    const dy = this.y - other.y;
    return dx * dx + dy * dy;
  }

  /**
   * Calculate the distance to another vector.
   */
  distance(other: Vector2i): number {
    return Math.sqrt(this.distance2(other));
  }

  /**
   * Rotate this vector around another point by the given angle.
   */
  rotWith(center: Vector2i, angle: number): Vector2i {
    const x = this.x - center.x;
    const y = this.y - center.y;
    const s = Math.sin(angle);
    const c = Math.cos(angle);
    return new Vector2i({
      x: center.x + Math.floor(x * c - y * s),
      y: center.y + Math.floor(x * s + y * c),
    });
  }

  /* ==== DESTACK_CUSTOM_END ==== */
}
registerStructClass(StructType.VECTOR2I, Vector2i);
/* ==== DESTACK_GENERATED_END:STRUCT:2400001 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:2400003 ==== */
/**
 * A 3D integer vector.
 */
export class Vector3i extends StructFrozen {
  static metatype: StructType = StructType.VECTOR3I;
  static __isFrozen__: boolean = true;

  /**
   * The x-coordinate of the Vector3i.
   */
  readonly x: SInt32;

  /**
   * The y-coordinate of the Vector3i.
   */
  readonly y: SInt32;

  /**
   * The z-coordinate of the Vector3i.
   */
  readonly z: SInt32;

  constructor(options: {
    x: SInt32;
    y: SInt32;
    z: SInt32;
    _session?: Session | null;
    _hash?: number | null;
    _repr?: string | null;
    _packedCache?: PackedCache[] | null;
  }) {
    /* super */
    super(
      /* session */
      options._session ?? null,
    );

    /* properties */
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

    /* identity */
    // @ts-expect-error(readonly)
    this._hash = options._hash ?? null;
    // @ts-expect-error(readonly)
    this._repr = options._repr ?? null;
    // @ts-expect-error(readonly)
    this._packedCache = options._packedCache ?? null;
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

  repr(): string {
    if (this._repr === null) {
      const propertyReprs: string[] = [];
      propertyReprs.push(`x=${this.x}`);
      propertyReprs.push(`y=${this.y}`);
      propertyReprs.push(`z=${this.z}`);
      // @ts-expect-error(readonly) */
      this._repr = `<Vector3i ${propertyReprs.join(" ")}>`;
    }
    return this._repr;
  }

  hash(): number {
    if (this._hash != null) {
      return this._hash;
    }
    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    h = (h * 31 + hashInt(this.x)) & 0xffffffff;
    h = (h * 31 + hashInt(this.y)) & 0xffffffff;
    h = (h * 31 + hashInt(this.z)) & 0xffffffff;
    // @ts-expect-error(readonly)
    this._hash = h;
    return h;
  }

  validate(): void {
    throw new Error("not implemented");
  }

  /* ==== DESTACK_CUSTOM_START ==== */

  /** Add two vectors or a vector and a scalar. */
  add(other: Vector3i | number): Vector3i {
    if (typeof other === "number") {
      return new Vector3i({
        x: this.x + other,
        y: this.y + other,
        z: this.z + other,
      });
    } else {
      return new Vector3i({
        x: this.x + other.x,
        y: this.y + other.y,
        z: this.z + other.z,
      });
    }
  }

  /** Subtract two vectors or a vector and a scalar. */
  sub(other: Vector3i | number): Vector3i {
    if (typeof other === "number") {
      return new Vector3i({
        x: this.x - other,
        y: this.y - other,
        z: this.z - other,
      });
    } else {
      return new Vector3i({
        x: this.x - other.x,
        y: this.y - other.y,
        z: this.z - other.z,
      });
    }
  }

  /** Multiply two vectors or a vector and a scalar. */
  mul(other: Vector3i | number): Vector3i {
    if (typeof other === "number") {
      return new Vector3i({
        x: this.x * other,
        y: this.y * other,
        z: this.z * other,
      });
    } else {
      return new Vector3i({
        x: this.x * other.x,
        y: this.y * other.y,
        z: this.z * other.z,
      });
    }
  }

  /** Divide two vectors or a vector and a scalar. */
  div(other: Vector3i | number): Vector3i {
    if (typeof other === "number") {
      return new Vector3i({
        x: Math.floor(this.x / other),
        y: Math.floor(this.y / other),
        z: Math.floor(this.z / other),
      });
    } else {
      return new Vector3i({
        x: Math.floor(this.x / other.x),
        y: Math.floor(this.y / other.y),
        z: Math.floor(this.z / other.z),
      });
    }
  }

  /** Negate a vector. */
  neg(): Vector3i {
    return new Vector3i({
      x: -this.x,
      y: -this.y,
      z: -this.z,
    });
  }

  /** Get the absolute value of a vector. */
  abs(): Vector3i {
    if (this.x >= 0 && this.y >= 0 && this.z >= 0) {
      return this;
    } else {
      return new Vector3i({
        x: Math.abs(this.x),
        y: Math.abs(this.y),
        z: Math.abs(this.z),
      });
    }
  }

  /**
   * Calculate the dot product with another vector.
   */
  dot(other: Vector3i): number {
    return this.x * other.x + this.y * other.y + this.z * other.z;
  }

  /**
   * Calculate the cross product with another vector.
   */
  cross(other: Vector3i): Vector3i {
    return new Vector3i({
      x: this.y * other.z - this.z * other.y,
      y: this.z * other.x - this.x * other.z,
      z: this.x * other.y - this.y * other.x,
    });
  }

  /**
   * Calculate the linear interpolation between two vectors.
   */
  lerp(other: Vector3i, t: number): Vector3i {
    return new Vector3i({
      x: Math.floor(this.x + (other.x - this.x) * t),
      y: Math.floor(this.y + (other.y - this.y) * t),
      z: Math.floor(this.z + (other.z - this.z) * t),
    });
  }

  /**
   * Calculate the magnitude (length) of the vector.
   */
  magnitude(): number {
    return Math.sqrt(this.x * this.x + this.y * this.y + this.z * this.z);
  }

  /**
   * Return a normalized (unit) vector.
   */
  normalize(): Vector3i {
    const mag = this.magnitude();
    if (mag === 0) {
      return new Vector3i({ x: 0, y: 0, z: 0 });
    }
    return new Vector3i({
      x: Math.floor(this.x / mag),
      y: Math.floor(this.y / mag),
      z: Math.floor(this.z / mag),
    });
  }

  /**
   * Calculate the squared distance to another vector.
   */
  distance2(other: Vector3i): number {
    const dx = this.x - other.x;
    const dy = this.y - other.y;
    const dz = this.z - other.z;
    return dx * dx + dy * dy + dz * dz;
  }

  /**
   * Calculate the distance to another vector.
   */
  distance(other: Vector3i): number {
    return Math.sqrt(this.distance2(other));
  }

  /**
   * Rotate this vector around another point by the given angle.
   */
  rotWith(center: Vector3i, angle: number): Vector3i {
    const x = this.x - center.x;
    const y = this.y - center.y;
    const z = this.z - center.z;
    const s = Math.sin(angle);
    const c = Math.cos(angle);
    return new Vector3i({
      x: center.x + Math.floor(x * c - y * s),
      y: center.y + Math.floor(x * s + y * c),
      z: center.z + z,
    });
  }

  /* ==== DESTACK_CUSTOM_END ==== */
}
registerStructClass(StructType.VECTOR3I, Vector3i);
/* ==== DESTACK_GENERATED_END:STRUCT:2400003 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:2400005 ==== */
/**
 * A 4D integer vector.
 */
export class Vector4i extends StructFrozen {
  static metatype: StructType = StructType.VECTOR4I;
  static __isFrozen__: boolean = true;

  /**
   * The x-coordinate of the Vector4i.
   */
  readonly x: SInt32;

  /**
   * The y-coordinate of the Vector4i.
   */
  readonly y: SInt32;

  /**
   * The z-coordinate of the Vector4i.
   */
  readonly z: SInt32;

  /**
   * The w-coordinate of the Vector4i.
   */
  readonly w: SInt32;

  constructor(options: {
    x: SInt32;
    y: SInt32;
    z: SInt32;
    w: SInt32;
    _session?: Session | null;
    _hash?: number | null;
    _repr?: string | null;
    _packedCache?: PackedCache[] | null;
  }) {
    /* super */
    super(
      /* session */
      options._session ?? null,
    );

    /* properties */
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

    /* identity */
    // @ts-expect-error(readonly)
    this._hash = options._hash ?? null;
    // @ts-expect-error(readonly)
    this._repr = options._repr ?? null;
    // @ts-expect-error(readonly)
    this._packedCache = options._packedCache ?? null;
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

  repr(): string {
    if (this._repr === null) {
      const propertyReprs: string[] = [];
      propertyReprs.push(`x=${this.x}`);
      propertyReprs.push(`y=${this.y}`);
      propertyReprs.push(`z=${this.z}`);
      propertyReprs.push(`w=${this.w}`);
      // @ts-expect-error(readonly) */
      this._repr = `<Vector4i ${propertyReprs.join(" ")}>`;
    }
    return this._repr;
  }

  hash(): number {
    if (this._hash != null) {
      return this._hash;
    }
    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    h = (h * 31 + hashInt(this.x)) & 0xffffffff;
    h = (h * 31 + hashInt(this.y)) & 0xffffffff;
    h = (h * 31 + hashInt(this.z)) & 0xffffffff;
    h = (h * 31 + hashInt(this.w)) & 0xffffffff;
    // @ts-expect-error(readonly)
    this._hash = h;
    return h;
  }

  validate(): void {
    throw new Error("not implemented");
  }

  /* ==== DESTACK_CUSTOM_START ==== */

  /** Add two vectors or a vector and a scalar. */
  add(other: Vector4i | number): Vector4i {
    if (typeof other === "number") {
      return new Vector4i({
        x: this.x + other,
        y: this.y + other,
        z: this.z + other,
        w: this.w + other,
      });
    } else {
      return new Vector4i({
        x: this.x + other.x,
        y: this.y + other.y,
        z: this.z + other.z,
        w: this.w + other.w,
      });
    }
  }

  /** Subtract two vectors or a vector and a scalar. */
  sub(other: Vector4i | number): Vector4i {
    if (typeof other === "number") {
      return new Vector4i({
        x: this.x - other,
        y: this.y - other,
        z: this.z - other,
        w: this.w - other,
      });
    } else {
      return new Vector4i({
        x: this.x - other.x,
        y: this.y - other.y,
        z: this.z - other.z,
        w: this.w - other.w,
      });
    }
  }

  /** Multiply two vectors or a vector and a scalar. */
  mul(other: Vector4i | number): Vector4i {
    if (typeof other === "number") {
      return new Vector4i({
        x: this.x * other,
        y: this.y * other,
        z: this.z * other,
        w: this.w * other,
      });
    } else {
      return new Vector4i({
        x: this.x * other.x,
        y: this.y * other.y,
        z: this.z * other.z,
        w: this.w * other.w,
      });
    }
  }

  /** Divide two vectors or a vector and a scalar. */
  div(other: Vector4i | number): Vector4i {
    if (typeof other === "number") {
      return new Vector4i({
        x: Math.floor(this.x / other),
        y: Math.floor(this.y / other),
        z: Math.floor(this.z / other),
        w: Math.floor(this.w / other),
      });
    } else {
      return new Vector4i({
        x: Math.floor(this.x / other.x),
        y: Math.floor(this.y / other.y),
        z: Math.floor(this.z / other.z),
        w: Math.floor(this.w / other.w),
      });
    }
  }

  /** Negate a vector. */
  neg(): Vector4i {
    return new Vector4i({
      x: -this.x,
      y: -this.y,
      z: -this.z,
      w: -this.w,
    });
  }

  /** Get the absolute value of a vector. */
  abs(): Vector4i {
    if (this.x >= 0 && this.y >= 0 && this.z >= 0 && this.w >= 0) {
      return this;
    } else {
      return new Vector4i({
        x: Math.abs(this.x),
        y: Math.abs(this.y),
        z: Math.abs(this.z),
        w: Math.abs(this.w),
      });
    }
  }

  /**
   * Calculate the dot product with another vector.
   */
  dot(other: Vector4i): number {
    return this.x * other.x + this.y * other.y + this.z * other.z + this.w * other.w;
  }

  /**
   * Calculate the cross product with another vector.
   */
  cross(other: Vector4i): Vector4i {
    return new Vector4i({
      x: this.y * other.z - this.z * other.y + this.w * other.x - this.x * other.w,
      y: this.z * other.w - this.w * other.z + this.x * other.y - this.y * other.x,
      z: this.w * other.x - this.x * other.w + this.y * other.z - this.z * other.y,
      w: this.x * other.y - this.y * other.x + this.z * other.w - this.w * other.z,
    });
  }

  /**
   * Calculate the linear interpolation between two vectors.
   */
  lerp(other: Vector4i, t: number): Vector4i {
    return new Vector4i({
      x: Math.floor(this.x + (other.x - this.x) * t),
      y: Math.floor(this.y + (other.y - this.y) * t),
      z: Math.floor(this.z + (other.z - this.z) * t),
      w: Math.floor(this.w + (other.w - this.w) * t),
    });
  }

  /**
   * Calculate the magnitude (length) of the vector.
   */
  magnitude(): number {
    return Math.sqrt(this.x * this.x + this.y * this.y + this.z * this.z + this.w * this.w);
  }

  /**
   * Return a normalized (unit) vector.
   */
  normalize(): Vector4i {
    const mag = this.magnitude();
    if (mag === 0) {
      return new Vector4i({ x: 0, y: 0, z: 0, w: 0 });
    }
    return new Vector4i({
      x: Math.floor(this.x / mag),
      y: Math.floor(this.y / mag),
      z: Math.floor(this.z / mag),
      w: Math.floor(this.w / mag),
    });
  }

  /**
   * Calculate the squared distance to another vector.
   */
  distance2(other: Vector4i): number {
    const dx = this.x - other.x;
    const dy = this.y - other.y;
    const dz = this.z - other.z;
    const dw = this.w - other.w;
    return dx * dx + dy * dy + dz * dz + dw * dw;
  }

  /**
   * Calculate the distance to another vector.
   */
  distance(other: Vector4i): number {
    return Math.sqrt(this.distance2(other));
  }

  /* ==== DESTACK_CUSTOM_END ==== */
}
registerStructClass(StructType.VECTOR4I, Vector4i);
/* ==== DESTACK_GENERATED_END:STRUCT:2400005 ==== */
