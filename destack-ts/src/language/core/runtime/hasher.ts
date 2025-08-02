import type {
  Boolean,
  Bytes,
  Date,
  Datetime,
  Duration,
  Float16,
  Float32,
  Float64,
  Int8,
  Int16,
  Int32,
  Int64,
  Int128,
  Json,
  String,
  Time,
  UInt8,
  UInt16,
  UInt32,
  UInt64,
  UInt128,
  UUID,
} from "@destack/language/core/builtin/types";
import { Temporal } from "temporal-polyfill";

const _EPOCH_DATE = new Temporal.PlainDate(1, 1, 1);

export class Hasher {
  private buffer: Int32 = 0x811c9dc5;

  digest(): Int32 {
    return this.buffer;
  }

  reset(): void {
    this.buffer = 0x811c9dc5;
  }

  private _mixByte(byte: number): void {
    // xor then multiply by prime
    this.buffer ^= byte & 0xff;
    this.buffer = Math.imul(this.buffer, 0x01000193) >>> 0;
  }

  private _mixBytes(data: Uint8Array): void {
    for (let i = 0; i < data.length; i++) {
      this._mixByte(data[i]);
    }
  }

  private _zigzagEncode(value: number): number {
    if (value >= 0) {
      return value << 1;
    } else {
      return (-value << 1) - 1;
    }
  }

  private _zigzagEncode64(value: bigint): bigint {
    if (value >= 0n) {
      return value << 1n;
    } else {
      return (-value << 1n) - 1n;
    }
  }

  // PrimitiveType.NONE
  hashNone(): void {
    this._mixByte(0);
  }

  // PrimitiveType.BOOLEAN
  hashBool(value: Boolean): void {
    this._mixByte(value ? 1 : 0);
  }

  // PrimitiveType.INT8
  hashInt8(value: Int8): void {
    this._mixByte(value & 0xff);
  }

  // PrimitiveType.INT16
  hashInt16(value: Int16): void {
    let zigzag = this._zigzagEncode(value);
    while (zigzag >= 0x80) {
      this._mixByte((zigzag & 0x7f) | 0x80);
      zigzag >>= 7;
    }
    this._mixByte(zigzag & 0x7f);
  }

  // PrimitiveType.INT32
  hashInt32(value: Int32): void {
    let zigzag = this._zigzagEncode(value);
    while (zigzag >= 0x80) {
      this._mixByte((zigzag & 0x7f) | 0x80);
      zigzag >>= 7;
    }
    this._mixByte(zigzag & 0x7f);
  }

  // PrimitiveType.INT64
  hashInt64(value: Int64): void {
    // javascript number can safely represent up to 53 bits
    // for larger values, we'd need bigint
    let zigzag = this._zigzagEncode(value);
    while (zigzag >= 0x80) {
      this._mixByte((zigzag & 0x7f) | 0x80);
      zigzag = Math.floor(zigzag / 128);
    }
    this._mixByte(zigzag & 0x7f);
  }

  // PrimitiveType.INT128
  hashInt128(value: Int128): void {
    // for int128, we need to use bigint
    const bigValue = BigInt(value);
    let zigzag = this._zigzagEncode64(bigValue);
    while (zigzag >= 0x80n) {
      this._mixByte(Number((zigzag & 0x7fn) | 0x80n));
      zigzag >>= 7n;
    }
    this._mixByte(Number(zigzag & 0x7fn));
  }

  // PrimitiveType.UINT8
  hashUint8(value: UInt8): void {
    this._mixByte(value);
  }

  // PrimitiveType.UINT16
  hashUint16(value: UInt16): void {
    let v = value;
    while (v >= 0x80) {
      this._mixByte((v & 0x7f) | 0x80);
      v >>= 7;
    }
    this._mixByte(v & 0x7f);
  }

  // PrimitiveType.UINT32
  hashUint32(value: UInt32): void {
    let v = value;
    while (v >= 0x80) {
      this._mixByte((v & 0x7f) | 0x80);
      v >>>= 7;
    }
    this._mixByte(v & 0x7f);
  }

  // PrimitiveType.UINT64
  hashUint64(value: UInt64): void {
    let v = value;
    while (v >= 0x80) {
      this._mixByte((v & 0x7f) | 0x80);
      v = Math.floor(v / 128);
    }
    this._mixByte(v & 0x7f);
  }

  // PrimitiveType.UINT128
  hashUint128(value: UInt128): void {
    const bigValue = BigInt(value);
    let v = bigValue;
    while (v >= 0x80n) {
      this._mixByte(Number((v & 0x7fn) | 0x80n));
      v >>= 7n;
    }
    this._mixByte(Number(v & 0x7fn));
  }

  // PrimitiveType.FLOAT16
  hashFloat16(value: Float16): void {
    // encode float16 to bytes
    const bytes = this._encodeFloat16(value);
    this._mixBytes(bytes);
  }

  // PrimitiveType.FLOAT32
  hashFloat32(value: Float32): void {
    const buffer = new ArrayBuffer(4);
    const view = new DataView(buffer);
    view.setFloat32(0, value, true); // little-endian
    this._mixBytes(new Uint8Array(buffer));
  }

  // PrimitiveType.FLOAT64
  hashFloat64(value: Float64): void {
    const buffer = new ArrayBuffer(8);
    const view = new DataView(buffer);
    view.setFloat64(0, value, true); // little-endian
    this._mixBytes(new Uint8Array(buffer));
  }

  // PrimitiveType.DATETIME
  hashDatetime(value: Datetime): void {
    // ensure UTC timezone
    const utcValue = value.withTimeZone("UTC");
    const date = utcValue.toPlainDate();
    const days = _EPOCH_DATE.until(date, { largestUnit: "days" }).days;
    const timeMicros =
      utcValue.hour * 3_600_000_000 +
      utcValue.minute * 60_000_000 +
      utcValue.second * 1_000_000 +
      Math.floor(utcValue.millisecond * 1000) +
      Math.floor(utcValue.microsecond);
    const micros = days * 86_400_000_000 + timeMicros;
    this.hashInt64(micros);
  }

  // PrimitiveType.DATE
  hashDate(value: Date): void {
    const days = _EPOCH_DATE.until(value, { largestUnit: "days" }).days;
    this.hashInt32(days);
  }

  // PrimitiveType.TIME
  hashTime(value: Time): void {
    const micros =
      value.hour * 3_600_000_000 +
      value.minute * 60_000_000 +
      value.second * 1_000_000 +
      Math.floor(value.millisecond * 1000) +
      Math.floor(value.microsecond);
    this.hashUint64(micros);
  }

  // PrimitiveType.DURATION
  hashDuration(value: Duration): void {
    // convert to microseconds
    const totalMicroseconds =
      (value.days ?? 0) * 86_400_000_000 +
      (value.hours ?? 0) * 3_600_000_000 +
      (value.minutes ?? 0) * 60_000_000 +
      (value.seconds ?? 0) * 1_000_000 +
      (value.milliseconds ?? 0) * 1_000 +
      (value.microseconds ?? 0);
    this.hashInt64(totalMicroseconds);
  }

  // PrimitiveType.STRING
  hashString(value: String): void {
    const encoder = new TextEncoder();
    const encoded = encoder.encode(value);
    this.hashBytes(encoded);
  }

  // PrimitiveType.UUID
  hashUuid(value: UUID): void {
    // convert UUID string to 16 bytes
    const hex = value.replace(/-/g, "");
    const bytes = new Uint8Array(16);
    for (let i = 0; i < 16; i++) {
      bytes[i] = parseInt(hex.slice(i * 2, i * 2 + 2), 16);
    }
    this._mixBytes(bytes);
  }

  // PrimitiveType.BYTES
  hashBytes(value: Bytes): void {
    // length-prefixed bytes
    let length = value.length;
    while (length >= 0x80) {
      this._mixByte((length & 0x7f) | 0x80);
      length >>= 7;
    }
    this._mixByte(length & 0x7f);
    this._mixBytes(value);
  }

  // PrimitiveType.JSON
  hashJson(value: Json): void {
    if (value === null) {
      this._mixByte(0);
    } else if (value === false) {
      this._mixByte(1);
    } else if (value === true) {
      this._mixByte(2);
    } else if (typeof value === "number" && Number.isInteger(value)) {
      this._mixByte(3);
      this.hashInt64(value);
    } else if (typeof value === "number") {
      this._mixByte(4);
      this.hashFloat64(value);
    } else if (typeof value === "string") {
      this._mixByte(5);
      this.hashString(value);
    } else if (Array.isArray(value)) {
      this._mixByte(6);
      let length = value.length;
      while (length >= 0x80) {
        this._mixByte((length & 0x7f) | 0x80);
        length >>= 7;
      }
      this._mixByte(length & 0x7f);
      for (const item of value) {
        this.hashJson(item);
      }
    } else if (typeof value === "object") {
      this._mixByte(7);
      const keys = Object.keys(value).sort();
      let length = keys.length;
      while (length >= 0x80) {
        this._mixByte((length & 0x7f) | 0x80);
        length >>= 7;
      }
      this._mixByte(length & 0x7f);
      // sort keys for stable hashing
      for (const key of keys) {
        this.hashString(key);
        this.hashJson(value[key]);
      }
    } else {
      throw new Error(`invalid JSON type '${typeof value}': ${value}`);
    }
  }

  private _encodeFloat16(value: number): Uint8Array {
    // convert float32 to float16 (IEEE 754 half-precision)
    const float32 = new Float32Array([value]);
    const uint32 = new Uint32Array(float32.buffer);
    const bits = uint32[0];

    // extract float32 components
    const sign = (bits >> 31) & 0x1;
    const exponent = (bits >> 23) & 0xff;
    const mantissa = bits & 0x7fffff;

    let float16bits: number;

    if (exponent === 0xff) {
      // infinity or NaN
      if (mantissa === 0) {
        // infinity
        float16bits = (sign << 15) | 0x7c00;
      } else {
        // nan
        float16bits = (sign << 15) | 0x7c00 | (mantissa >> 13);
      }
    } else if (exponent === 0) {
      // zero or subnormal
      float16bits = sign << 15;
    } else {
      // normal number
      const newExponent = exponent - 127 + 15;
      if (newExponent >= 31) {
        // overflow to infinity
        float16bits = (sign << 15) | 0x7c00;
      } else if (newExponent <= 0) {
        // underflow to zero
        float16bits = sign << 15;
      } else {
        // normal float16
        float16bits = (sign << 15) | (newExponent << 10) | (mantissa >> 13);
      }
    }

    // convert to little-endian bytes
    return new Uint8Array([float16bits & 0xff, (float16bits >> 8) & 0xff]);
  }
}
