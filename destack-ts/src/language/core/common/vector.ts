import { StructType } from "@destack/language/core/builtin/common";
import { StructFrozen } from "@destack/language/core/builtin/struct";
import type { Supergraph } from "@destack/language/core/runtime/graph";
import type { Session } from "@destack/language/core/runtime/session";
import { registerStructClass } from "@destack/language/registry";
import {
  Vector2fProto,
  Vector2iProto,
  Vector3fProto,
  Vector3iProto,
  Vector4fProto,
  Vector4iProto,
} from "@destack/proto";
import { base64Decode } from "@destack/utils";
import { hashFloat, hashInt } from "@destack/utils/hash";

/* ==== DESTACK_GENERATED_START:STRUCT:700 ==== */
/**
 * A vector.
 */
export abstract class Vector extends StructFrozen {
  static metatype: StructType = StructType.VECTOR;
  static __isFrozen__: boolean = true;

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerStructClass(StructType.VECTOR, Vector);
/* ==== DESTACK_GENERATED_END:STRUCT:700 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:710 ==== */
/**
 * A floating point vector.
 */
export abstract class Vectorf extends Vector {
  static metatype: StructType = StructType.VECTORF;
  static __isFrozen__: boolean = true;

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerStructClass(StructType.VECTORF, Vectorf);
/* ==== DESTACK_GENERATED_END:STRUCT:710 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:720 ==== */
/**
 * An integer vector.
 */
export abstract class Vectori extends Vector {
  static metatype: StructType = StructType.VECTORI;
  static __isFrozen__: boolean = true;

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerStructClass(StructType.VECTORI, Vectori);
/* ==== DESTACK_GENERATED_END:STRUCT:720 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:711 ==== */
/**
 * A 2D float vector.
 */
export class Vector2f extends Vectorf {
  static metatype: StructType = StructType.VECTOR2F;
  static __isFrozen__: boolean = true;

  /**
   * Vector2f.x
   */
  readonly x: number;

  /**
   * Vector2f.y
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
      throw new Error(`Vector2f.x is required`);
    }
    this.x = _x;
    let _y = options.y;
    if (_y === null) {
      throw new Error(`Vector2f.y is required`);
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
      // @ts-expect-error(readonly)
      this._repr = `<Vector2f ${propertyReprs.join(" ")}>`;
    }
    return this._repr;
  }

  hash(): number {
    if (this._hash !== null) {
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

  toValue(): { readonly [key: string]: any } {
    if (this._value === null) {
      // @ts-expect-error(readonly)
      this._value = Vector2f.__packValue__(this);
    }
    return this._value;
  }

  static __packValue__(object: Vector2f): { readonly [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 711;
    objectValue["101"] = object.x;
    objectValue["102"] = object.y;
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { readonly [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Vector2f {
    return new Vector2f({
      x: objectValue["101"],
      y: objectValue["102"],
      _value: objectValue,
      _supergraph,
    });
  }

  static fromValue(
    objectValue: { readonly [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Vector2f {
    return Vector2f.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
  }

  toProto(): Vector2fProto {
    if (this._proto === null) {
      // @ts-expect-error(readonly)
      this._proto = Vector2f.__packProto__(this);
    }
    return this._proto as Vector2fProto;
  }

  static __packProto__(object: Vector2f): Vector2fProto {
    const objectProto: Partial<Vector2fProto> = { metatype: 711 };
    objectProto.x = object.x;
    objectProto.y = object.y;
    return objectProto as Vector2fProto;
  }

  static __unpackProto__(
    objectProto: Vector2fProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Vector2f {
    return new Vector2f({
      x: objectProto.x,
      y: objectProto.y,
      _proto: objectProto,
      _supergraph,
    });
  }

  static fromProto(
    objectProto: Vector2fProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Vector2f {
    return Vector2f.__unpackProto__(objectProto, _session, _supergraph, _graph, _connection);
  }

  static fromProtoString(packedProtoString: string): Vector2f {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = Vector2fProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */

  /** Add two vectors or a vector and a scalar. */
  add(other: Vector2f | number): Vector2f {
    if (typeof other === "number") {
      return new Vector2f({
        x: this.x + other,
        y: this.y + other,
      });
    } else {
      return new Vector2f({
        x: this.x + other.x,
        y: this.y + other.y,
      });
    }
  }

  /** Subtract two vectors or a vector and a scalar. */
  sub(other: Vector2f | number): Vector2f {
    if (typeof other === "number") {
      return new Vector2f({
        x: this.x - other,
        y: this.y - other,
      });
    } else {
      return new Vector2f({
        x: this.x - other.x,
        y: this.y - other.y,
      });
    }
  }

  /** Multiply two vectors or a vector and a scalar. */
  mul(other: Vector2f | number): Vector2f {
    if (typeof other === "number") {
      return new Vector2f({
        x: this.x * other,
        y: this.y * other,
      });
    } else {
      return new Vector2f({
        x: this.x * other.x,
        y: this.y * other.y,
      });
    }
  }

  /** Divide two vectors or a vector and a scalar. */
  div(other: Vector2f | number): Vector2f {
    if (typeof other === "number") {
      return new Vector2f({
        x: this.x / other,
        y: this.y / other,
      });
    } else {
      return new Vector2f({
        x: this.x / other.x,
        y: this.y / other.y,
      });
    }
  }

  /** Negate a vector. */
  neg(): Vector2f {
    return new Vector2f({
      x: -this.x,
      y: -this.y,
    });
  }

  /**
   * Get the perpendicular vector (rotated 90 degrees counterclockwise).
   */
  per(): Vector2f {
    return new Vector2f({
      x: this.y,
      y: -this.x,
    });
  }

  /** Get the absolute value of a vector. */
  abs(): Vector2f {
    if (this.x >= 0 && this.y >= 0) {
      return this;
    } else {
      return new Vector2f({
        x: Math.abs(this.x),
        y: Math.abs(this.y),
      });
    }
  }

  /**
   * Calculate the dot product with another vector.
   */
  dot(other: Vector2f): number {
    return this.x * other.x + this.y * other.y;
  }

  /**
   * Calculate the linear interpolation between two vectors.
   */
  lerp(other: Vector2f, t: number): Vector2f {
    return new Vector2f({
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
  normalize(): Vector2f {
    const mag = this.magnitude();
    if (mag === 0) {
      return new Vector2f({ x: 0.0, y: 0.0 });
    }
    return new Vector2f({
      x: this.x / mag,
      y: this.y / mag,
    });
  }

  /**
   * Calculate the squared distance to another vector.
   */
  distance2(other: Vector2f): number {
    const dx = this.x - other.x;
    const dy = this.y - other.y;
    return dx * dx + dy * dy;
  }

  /**
   * Calculate the distance to another vector.
   */
  distance(other: Vector2f): number {
    return Math.sqrt(this.distance2(other));
  }

  /**
   * Rotate this vector around another point by the given angle.
   */
  rotWith(center: Vector2f, angle: number): Vector2f {
    const x = this.x - center.x;
    const y = this.y - center.y;
    const s = Math.sin(angle);
    const c = Math.cos(angle);
    return new Vector2f({
      x: center.x + (x * c - y * s),
      y: center.y + (x * s + y * c),
    });
  }

  /* ==== DESTACK_CUSTOM_END ==== */
}
registerStructClass(StructType.VECTOR2F, Vector2f);
/* ==== DESTACK_GENERATED_END:STRUCT:711 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:712 ==== */
/**
 * A 3D float vector.
 */
export class Vector3f extends Vectorf {
  static metatype: StructType = StructType.VECTOR3F;
  static __isFrozen__: boolean = true;

  /**
   * Vector3f.x
   */
  readonly x: number;

  /**
   * Vector3f.y
   */
  readonly y: number;

  /**
   * Vector3f.z
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
      throw new Error(`Vector3f.x is required`);
    }
    this.x = _x;
    let _y = options.y;
    if (_y === null) {
      throw new Error(`Vector3f.y is required`);
    }
    this.y = _y;
    let _z = options.z;
    if (_z === null) {
      throw new Error(`Vector3f.z is required`);
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
      // @ts-expect-error(readonly)
      this._repr = `<Vector3f ${propertyReprs.join(" ")}>`;
    }
    return this._repr;
  }

  hash(): number {
    if (this._hash !== null) {
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

  toValue(): { readonly [key: string]: any } {
    if (this._value === null) {
      // @ts-expect-error(readonly)
      this._value = Vector3f.__packValue__(this);
    }
    return this._value;
  }

  static __packValue__(object: Vector3f): { readonly [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 712;
    objectValue["101"] = object.x;
    objectValue["102"] = object.y;
    objectValue["103"] = object.z;
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { readonly [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Vector3f {
    return new Vector3f({
      x: objectValue["101"],
      y: objectValue["102"],
      z: objectValue["103"],
      _value: objectValue,
      _supergraph,
    });
  }

  static fromValue(
    objectValue: { readonly [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Vector3f {
    return Vector3f.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
  }

  toProto(): Vector3fProto {
    if (this._proto === null) {
      // @ts-expect-error(readonly)
      this._proto = Vector3f.__packProto__(this);
    }
    return this._proto as Vector3fProto;
  }

  static __packProto__(object: Vector3f): Vector3fProto {
    const objectProto: Partial<Vector3fProto> = { metatype: 712 };
    objectProto.x = object.x;
    objectProto.y = object.y;
    objectProto.z = object.z;
    return objectProto as Vector3fProto;
  }

  static __unpackProto__(
    objectProto: Vector3fProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Vector3f {
    return new Vector3f({
      x: objectProto.x,
      y: objectProto.y,
      z: objectProto.z,
      _proto: objectProto,
      _supergraph,
    });
  }

  static fromProto(
    objectProto: Vector3fProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Vector3f {
    return Vector3f.__unpackProto__(objectProto, _session, _supergraph, _graph, _connection);
  }

  static fromProtoString(packedProtoString: string): Vector3f {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = Vector3fProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */

  /** Add two vectors or a vector and a scalar. */
  add(other: Vector3f | number): Vector3f {
    if (typeof other === "number") {
      return new Vector3f({
        x: this.x + other,
        y: this.y + other,
        z: this.z + other,
      });
    } else {
      return new Vector3f({
        x: this.x + other.x,
        y: this.y + other.y,
        z: this.z + other.z,
      });
    }
  }

  /** Subtract two vectors or a vector and a scalar. */
  sub(other: Vector3f | number): Vector3f {
    if (typeof other === "number") {
      return new Vector3f({
        x: this.x - other,
        y: this.y - other,
        z: this.z - other,
      });
    } else {
      return new Vector3f({
        x: this.x - other.x,
        y: this.y - other.y,
        z: this.z - other.z,
      });
    }
  }

  /** Multiply two vectors or a vector and a scalar. */
  mul(other: Vector3f | number): Vector3f {
    if (typeof other === "number") {
      return new Vector3f({
        x: this.x * other,
        y: this.y * other,
        z: this.z * other,
      });
    } else {
      return new Vector3f({
        x: this.x * other.x,
        y: this.y * other.y,
        z: this.z * other.z,
      });
    }
  }

  /** Divide two vectors or a vector and a scalar. */
  div(other: Vector3f | number): Vector3f {
    if (typeof other === "number") {
      return new Vector3f({
        x: this.x / other,
        y: this.y / other,
        z: this.z / other,
      });
    } else {
      return new Vector3f({
        x: this.x / other.x,
        y: this.y / other.y,
        z: this.z / other.z,
      });
    }
  }

  /** Negate a vector. */
  neg(): Vector3f {
    return new Vector3f({
      x: -this.x,
      y: -this.y,
      z: -this.z,
    });
  }

  /**
   * Get the perpendicular vector (rotated 90 degrees counterclockwise).
   */
  per(): Vector3f {
    return new Vector3f({
      x: this.y,
      y: -this.x,
      z: 0,
    });
  }

  /** Get the absolute value of a vector. */
  abs(): Vector3f {
    if (this.x >= 0 && this.y >= 0 && this.z >= 0) {
      return this;
    } else {
      return new Vector3f({
        x: Math.abs(this.x),
        y: Math.abs(this.y),
        z: Math.abs(this.z),
      });
    }
  }

  /**
   * Calculate the dot product with another vector.
   */
  dot(other: Vector3f): number {
    return this.x * other.x + this.y * other.y + this.z * other.z;
  }

  /**
   * Calculate the cross product with another vector.
   */
  cross(other: Vector3f): Vector3f {
    return new Vector3f({
      x: this.y * other.z - this.z * other.y,
      y: this.z * other.x - this.x * other.z,
      z: this.x * other.y - this.y * other.x,
    });
  }

  /**
   * Calculate the linear interpolation between two vectors.
   */
  lerp(other: Vector3f, t: number): Vector3f {
    return new Vector3f({
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
  normalize(): Vector3f {
    const mag = this.magnitude();
    if (mag === 0) {
      return new Vector3f({ x: 0.0, y: 0.0, z: 0.0 });
    }
    return new Vector3f({
      x: this.x / mag,
      y: this.y / mag,
      z: this.z / mag,
    });
  }

  /**
   * Calculate the squared distance to another vector.
   */
  distance2(other: Vector3f): number {
    const dx = this.x - other.x;
    const dy = this.y - other.y;
    const dz = this.z - other.z;
    return dx * dx + dy * dy + dz * dz;
  }

  /**
   * Calculate the distance to another vector.
   */
  distance(other: Vector3f): number {
    return Math.sqrt(this.distance2(other));
  }

  /**
   * Rotate this vector around another point by the given angle.
   */
  rotWith(center: Vector3f, angle: number): Vector3f {
    const x = this.x - center.x;
    const y = this.y - center.y;
    const s = Math.sin(angle);
    const c = Math.cos(angle);
    return new Vector3f({
      x: center.x + (x * c - y * s),
      y: center.y + (x * s + y * c),
      z: this.z,
    });
  }

  /* ==== DESTACK_CUSTOM_END ==== */
}
registerStructClass(StructType.VECTOR3F, Vector3f);
/* ==== DESTACK_GENERATED_END:STRUCT:712 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:713 ==== */
/**
 * A 4D float vector.
 */
export class Vector4f extends Vectorf {
  static metatype: StructType = StructType.VECTOR4F;
  static __isFrozen__: boolean = true;

  /**
   * Vector4f.x
   */
  readonly x: number;

  /**
   * Vector4f.y
   */
  readonly y: number;

  /**
   * Vector4f.z
   */
  readonly z: number;

  /**
   * Vector4f.w
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
      throw new Error(`Vector4f.x is required`);
    }
    this.x = _x;
    let _y = options.y;
    if (_y === null) {
      throw new Error(`Vector4f.y is required`);
    }
    this.y = _y;
    let _z = options.z;
    if (_z === null) {
      throw new Error(`Vector4f.z is required`);
    }
    this.z = _z;
    let _w = options.w;
    if (_w === null) {
      throw new Error(`Vector4f.w is required`);
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
      // @ts-expect-error(readonly)
      this._repr = `<Vector4f ${propertyReprs.join(" ")}>`;
    }
    return this._repr;
  }

  hash(): number {
    if (this._hash !== null) {
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

  toValue(): { readonly [key: string]: any } {
    if (this._value === null) {
      // @ts-expect-error(readonly)
      this._value = Vector4f.__packValue__(this);
    }
    return this._value;
  }

  static __packValue__(object: Vector4f): { readonly [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 713;
    objectValue["101"] = object.x;
    objectValue["102"] = object.y;
    objectValue["103"] = object.z;
    objectValue["104"] = object.w;
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { readonly [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Vector4f {
    return new Vector4f({
      x: objectValue["101"],
      y: objectValue["102"],
      z: objectValue["103"],
      w: objectValue["104"],
      _value: objectValue,
      _supergraph,
    });
  }

  static fromValue(
    objectValue: { readonly [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Vector4f {
    return Vector4f.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
  }

  toProto(): Vector4fProto {
    if (this._proto === null) {
      // @ts-expect-error(readonly)
      this._proto = Vector4f.__packProto__(this);
    }
    return this._proto as Vector4fProto;
  }

  static __packProto__(object: Vector4f): Vector4fProto {
    const objectProto: Partial<Vector4fProto> = { metatype: 713 };
    objectProto.x = object.x;
    objectProto.y = object.y;
    objectProto.z = object.z;
    objectProto.w = object.w;
    return objectProto as Vector4fProto;
  }

  static __unpackProto__(
    objectProto: Vector4fProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Vector4f {
    return new Vector4f({
      x: objectProto.x,
      y: objectProto.y,
      z: objectProto.z,
      w: objectProto.w,
      _proto: objectProto,
      _supergraph,
    });
  }

  static fromProto(
    objectProto: Vector4fProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Vector4f {
    return Vector4f.__unpackProto__(objectProto, _session, _supergraph, _graph, _connection);
  }

  static fromProtoString(packedProtoString: string): Vector4f {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = Vector4fProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */

  /** Add two vectors or a vector and a scalar. */
  add(other: Vector4f | number): Vector4f {
    if (typeof other === "number") {
      return new Vector4f({
        x: this.x + other,
        y: this.y + other,
        z: this.z + other,
        w: this.w + other,
      });
    } else {
      return new Vector4f({
        x: this.x + other.x,
        y: this.y + other.y,
        z: this.z + other.z,
        w: this.w + other.w,
      });
    }
  }

  /** Subtract two vectors or a vector and a scalar. */
  sub(other: Vector4f | number): Vector4f {
    if (typeof other === "number") {
      return new Vector4f({
        x: this.x - other,
        y: this.y - other,
        z: this.z - other,
        w: this.w - other,
      });
    } else {
      return new Vector4f({
        x: this.x - other.x,
        y: this.y - other.y,
        z: this.z - other.z,
        w: this.w - other.w,
      });
    }
  }

  /** Multiply two vectors or a vector and a scalar. */
  mul(other: Vector4f | number): Vector4f {
    if (typeof other === "number") {
      return new Vector4f({
        x: this.x * other,
        y: this.y * other,
        z: this.z * other,
        w: this.w * other,
      });
    } else {
      return new Vector4f({
        x: this.x * other.x,
        y: this.y * other.y,
        z: this.z * other.z,
        w: this.w * other.w,
      });
    }
  }

  /** Divide two vectors or a vector and a scalar. */
  div(other: Vector4f | number): Vector4f {
    if (typeof other === "number") {
      return new Vector4f({
        x: this.x / other,
        y: this.y / other,
        z: this.z / other,
        w: this.w / other,
      });
    } else {
      return new Vector4f({
        x: this.x / other.x,
        y: this.y / other.y,
        z: this.z / other.z,
        w: this.w / other.w,
      });
    }
  }

  /** Negate a vector. */
  neg(): Vector4f {
    return new Vector4f({
      x: -this.x,
      y: -this.y,
      z: -this.z,
      w: -this.w,
    });
  }

  /**
   * Get the perpendicular vector (rotated 90 degrees counterclockwise).
   */
  per(): Vector4f {
    return new Vector4f({
      x: this.y,
      y: -this.x,
      z: 0,
      w: 0,
    });
  }

  /** Get the absolute value of a vector. */
  abs(): Vector4f {
    if (this.x >= 0 && this.y >= 0 && this.z >= 0 && this.w >= 0) {
      return this;
    } else {
      return new Vector4f({
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
  dot(other: Vector4f): number {
    return this.x * other.x + this.y * other.y + this.z * other.z + this.w * other.w;
  }

  /**
   * Calculate the cross product with another vector.
   */
  cross(other: Vector4f): Vector4f {
    return new Vector4f({
      x: this.y * other.z - this.z * other.y,
      y: this.z * other.x - this.x * other.z,
      z: this.x * other.y - this.y * other.x,
      w: 0,
    });
  }

  /**
   * Calculate the linear interpolation between two vectors.
   */
  lerp(other: Vector4f, t: number): Vector4f {
    return new Vector4f({
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
  normalize(): Vector4f {
    const mag = this.magnitude();
    if (mag === 0) {
      return new Vector4f({ x: 0.0, y: 0.0, z: 0.0, w: 0.0 });
    }
    return new Vector4f({
      x: this.x / mag,
      y: this.y / mag,
      z: this.z / mag,
      w: this.w / mag,
    });
  }

  /**
   * Calculate the squared distance to another vector.
   */
  distance2(other: Vector4f): number {
    const dx = this.x - other.x;
    const dy = this.y - other.y;
    const dz = this.z - other.z;
    const dw = this.w - other.w;
    return dx * dx + dy * dy + dz * dz + dw * dw;
  }

  /**
   * Calculate the distance to another vector.
   */
  distance(other: Vector4f): number {
    return Math.sqrt(this.distance2(other));
  }

  /**
   * Rotate this vector around another point by the given angle.
   */
  rotWith(center: Vector4f, angle: number): Vector4f {
    const x = this.x - center.x;
    const y = this.y - center.y;
    const s = Math.sin(angle);
    const c = Math.cos(angle);
    return new Vector4f({
      x: center.x + (x * c - y * s),
      y: center.y + (x * s + y * c),
      z: this.z,
      w: this.w,
    });
  }

  /* ==== DESTACK_CUSTOM_END ==== */
}
registerStructClass(StructType.VECTOR4F, Vector4f);
/* ==== DESTACK_GENERATED_END:STRUCT:713 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:721 ==== */
/**
 * A 2D integer vector.
 */
export class Vector2i extends Vectori {
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
      // @ts-expect-error(readonly)
      this._repr = `<Vector2i ${propertyReprs.join(" ")}>`;
    }
    return this._repr;
  }

  hash(): number {
    if (this._hash !== null) {
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

  toValue(): { readonly [key: string]: any } {
    if (this._value === null) {
      // @ts-expect-error(readonly)
      this._value = Vector2i.__packValue__(this);
    }
    return this._value;
  }

  static __packValue__(object: Vector2i): { readonly [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 721;
    objectValue["101"] = object.x;
    objectValue["102"] = object.y;
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { readonly [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Vector2i {
    return new Vector2i({
      x: Number(objectValue["101"]),
      y: Number(objectValue["102"]),
      _value: objectValue,
      _supergraph,
    });
  }

  static fromValue(
    objectValue: { readonly [key: string]: any },
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
    const objectProto: Partial<Vector2iProto> = { metatype: 721 };
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
/* ==== DESTACK_GENERATED_END:STRUCT:721 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:722 ==== */
/**
 * A 3D integer vector.
 */
export class Vector3i extends Vectori {
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
      // @ts-expect-error(readonly)
      this._repr = `<Vector3i ${propertyReprs.join(" ")}>`;
    }
    return this._repr;
  }

  hash(): number {
    if (this._hash !== null) {
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

  toValue(): { readonly [key: string]: any } {
    if (this._value === null) {
      // @ts-expect-error(readonly)
      this._value = Vector3i.__packValue__(this);
    }
    return this._value;
  }

  static __packValue__(object: Vector3i): { readonly [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 722;
    objectValue["101"] = object.x;
    objectValue["102"] = object.y;
    objectValue["103"] = object.z;
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { readonly [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Vector3i {
    return new Vector3i({
      x: Number(objectValue["101"]),
      y: Number(objectValue["102"]),
      z: Number(objectValue["103"]),
      _value: objectValue,
      _supergraph,
    });
  }

  static fromValue(
    objectValue: { readonly [key: string]: any },
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
    const objectProto: Partial<Vector3iProto> = { metatype: 722 };
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
/* ==== DESTACK_GENERATED_END:STRUCT:722 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:723 ==== */
/**
 * A 4D integer vector.
 */
export class Vector4i extends Vectori {
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
      // @ts-expect-error(readonly)
      this._repr = `<Vector4i ${propertyReprs.join(" ")}>`;
    }
    return this._repr;
  }

  hash(): number {
    if (this._hash !== null) {
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

  toValue(): { readonly [key: string]: any } {
    if (this._value === null) {
      // @ts-expect-error(readonly)
      this._value = Vector4i.__packValue__(this);
    }
    return this._value;
  }

  static __packValue__(object: Vector4i): { readonly [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 723;
    objectValue["101"] = object.x;
    objectValue["102"] = object.y;
    objectValue["103"] = object.z;
    objectValue["104"] = object.w;
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { readonly [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Vector4i {
    return new Vector4i({
      x: Number(objectValue["101"]),
      y: Number(objectValue["102"]),
      z: Number(objectValue["103"]),
      w: Number(objectValue["104"]),
      _value: objectValue,
      _supergraph,
    });
  }

  static fromValue(
    objectValue: { readonly [key: string]: any },
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
    const objectProto: Partial<Vector4iProto> = { metatype: 723 };
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
/* ==== DESTACK_GENERATED_END:STRUCT:723 ==== */
