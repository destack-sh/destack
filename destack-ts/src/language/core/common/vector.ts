import { Session, Supergraph } from "@destack/language/core";
import { StructFrozen, StructType } from "@destack/language/core/builtin";
import { registerStructClass } from "@destack/language/registry";
import {
  Vector2Proto,
  Vector2iProto,
  Vector3Proto,
  Vector3iProto,
  Vector4Proto,
  Vector4iProto,
} from "@destack/proto";
import { base64Decode } from "@destack/utils";

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

  repr(): string {
    if (this._repr === null) {
      const propertyReprs: string[] = [];
      propertyReprs.push(`x=${this.x}`);
      propertyReprs.push(`y=${this.y}`);
      this._repr = `<Vector2 ${propertyReprs.join(" ")}>`;
    }
    return this._repr;
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

  static fromProtoString(packedProtoString: string): Vector2 {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = Vector2Proto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
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
    return new Vector2({
      x: Math.abs(this.x),
      y: Math.abs(this.y),
    });
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

  repr(): string {
    if (this._repr === null) {
      const propertyReprs: string[] = [];
      propertyReprs.push(`x=${this.x}`);
      propertyReprs.push(`y=${this.y}`);
      propertyReprs.push(`z=${this.z}`);
      this._repr = `<Vector3 ${propertyReprs.join(" ")}>`;
    }
    return this._repr;
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

  static fromProtoString(packedProtoString: string): Vector3 {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = Vector3Proto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
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
    return new Vector3({
      x: Math.abs(this.x),
      y: Math.abs(this.y),
      z: Math.abs(this.z),
    });
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

  repr(): string {
    if (this._repr === null) {
      const propertyReprs: string[] = [];
      propertyReprs.push(`x=${this.x}`);
      propertyReprs.push(`y=${this.y}`);
      propertyReprs.push(`z=${this.z}`);
      propertyReprs.push(`w=${this.w}`);
      this._repr = `<Vector4 ${propertyReprs.join(" ")}>`;
    }
    return this._repr;
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

  static fromProtoString(packedProtoString: string): Vector4 {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = Vector4Proto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
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
    return new Vector4({
      x: Math.abs(this.x),
      y: Math.abs(this.y),
      z: Math.abs(this.z),
      w: Math.abs(this.w),
    });
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

  repr(): string {
    if (this._repr === null) {
      const propertyReprs: string[] = [];
      propertyReprs.push(`x=${this.x}`);
      propertyReprs.push(`y=${this.y}`);
      this._repr = `<Vector2i ${propertyReprs.join(" ")}>`;
    }
    return this._repr;
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

  static fromProtoString(packedProtoString: string): Vector2i {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = Vector2iProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
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
    return new Vector2i({
      x: Math.abs(this.x),
      y: Math.abs(this.y),
    });
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

  repr(): string {
    if (this._repr === null) {
      const propertyReprs: string[] = [];
      propertyReprs.push(`x=${this.x}`);
      propertyReprs.push(`y=${this.y}`);
      propertyReprs.push(`z=${this.z}`);
      this._repr = `<Vector3i ${propertyReprs.join(" ")}>`;
    }
    return this._repr;
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

  static fromProtoString(packedProtoString: string): Vector3i {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = Vector3iProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
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
    return new Vector3i({
      x: Math.abs(this.x),
      y: Math.abs(this.y),
      z: Math.abs(this.z),
    });
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

  repr(): string {
    if (this._repr === null) {
      const propertyReprs: string[] = [];
      propertyReprs.push(`x=${this.x}`);
      propertyReprs.push(`y=${this.y}`);
      propertyReprs.push(`z=${this.z}`);
      propertyReprs.push(`w=${this.w}`);
      this._repr = `<Vector4i ${propertyReprs.join(" ")}>`;
    }
    return this._repr;
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

  static fromProtoString(packedProtoString: string): Vector4i {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = Vector4iProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
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
    return new Vector4i({
      x: Math.abs(this.x),
      y: Math.abs(this.y),
      z: Math.abs(this.z),
      w: Math.abs(this.w),
    });
  }

  /**
   * Calculate the dot product with another vector.
   */
  dot(other: Vector4i): number {
    return this.x * other.x + this.y * other.y + this.z * other.z + this.w * other.w;
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
/* ==== DESTACK_GENERATED_END:STRUCT:50205 ==== */

export type Vectorf = Vector2 | Vector3 | Vector4;
export type Vectori = Vector2i | Vector3i | Vector4i;
export type Vector = Vectorf | Vectori;
