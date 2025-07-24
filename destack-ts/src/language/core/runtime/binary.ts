import { Temporal } from "temporal-polyfill";

export class BinaryError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "BinaryError";
  }
}

const _EPOCH_DATE = new Temporal.PlainDate(1970, 1, 1);

/**
 * Write binary data in our custom encoding. Little-endian, varint, zigzag, etc.
 */
export class BinaryWriter {
  private buffer: ArrayBuffer;
  private view: DataView;
  private pos: number;
  private textEncoder: TextEncoder;

  constructor(initialSize: number = 256) {
    this.buffer = new ArrayBuffer(initialSize);
    this.view = new DataView(this.buffer);
    this.pos = 0;
    this.textEncoder = new TextEncoder();
  }

  toBytes(): Uint8Array {
    return new Uint8Array(this.buffer, 0, this.pos);
  }

  private ensureCapacity(additional: number): void {
    const needed = this.pos + additional;
    if (needed > this.buffer.byteLength) {
      // grow buffer by at least 50% or to needed size
      const newSize = Math.max(needed, Math.floor(this.buffer.byteLength * 1.5));
      const newBuffer = new ArrayBuffer(newSize);
      const newView = new DataView(newBuffer);
      // copy existing data
      const oldBytes = new Uint8Array(this.buffer, 0, this.pos);
      new Uint8Array(newBuffer).set(oldBytes);
      this.buffer = newBuffer;
      this.view = newView;
    }
  }

  // PrimitiveType.BOOLEAN
  writeBool(value: boolean): void {
    this.ensureCapacity(1);
    this.view.setUint8(this.pos++, value ? 1 : 0);
  }

  // PrimitiveType.SINT8
  writeSInt8(value: number): void {
    this.ensureCapacity(1);
    this.view.setInt8(this.pos++, value);
  }

  // PrimitiveType.SINT16
  writeSInt16(value: number): void {
    this.writeVarint(this.zigzagEncode(value));
  }

  // PrimitiveType.SINT32
  writeSInt32(value: number): void {
    this.writeVarint(this.zigzagEncode(value));
  }

  // PrimitiveType.SINT64
  writeSInt64(value: bigint): void {
    this.writeVarint64(this.zigzagEncode64(value));
  }

  // PrimitiveType.SINT128
  writeSInt128(value: bigint): void {
    this.writeVarint128(this.zigzagEncode128(value));
  }

  // PrimitiveType.UINT8
  writeUint8(value: number): void {
    this.ensureCapacity(1);
    this.view.setUint8(this.pos++, value);
  }

  // PrimitiveType.UINT16
  writeUint16(value: number): void {
    this.writeVarint(value);
  }

  // PrimitiveType.UINT32
  writeUint32(value: number): void {
    this.writeVarint(value);
  }

  // PrimitiveType.UINT64
  writeUint64(value: bigint): void {
    this.writeVarint64(value);
  }

  // PrimitiveType.UINT128
  writeUint128(value: bigint): void {
    this.writeVarint128(value);
  }

  // PrimitiveType.FLOAT16
  writeFloat16(value: number): void {
    this.ensureCapacity(3);
    if (value === 0.0) {
      this.view.setUint8(this.pos++, 0);
    } else {
      this.view.setUint8(this.pos++, 1);
      // Encode float16
      const float16Bytes = this.encodeFloat16(value);
      this.view.setUint8(this.pos++, float16Bytes[0]);
      this.view.setUint8(this.pos++, float16Bytes[1]);
    }
  }

  // PrimitiveType.FLOAT32
  writeFloat32(value: number): void {
    this.ensureCapacity(5);
    if (value === 0.0) {
      this.view.setUint8(this.pos++, 0);
    } else {
      this.view.setUint8(this.pos++, 1);
      this.view.setFloat32(this.pos, value, true); // little-endian
      this.pos += 4;
    }
  }

  // PrimitiveType.FLOAT64
  writeFloat64(value: number): void {
    this.ensureCapacity(9);
    if (value === 0.0) {
      this.view.setUint8(this.pos++, 0);
    } else if (
      !Number.isNaN(value) &&
      Number.isFinite(value) &&
      value === Math.floor(value) &&
      value >= -(2 ** 53) &&
      value <= 2 ** 53
    ) {
      // can be represented exactly as an integer
      this.view.setUint8(this.pos++, 1);
      // For large numbers beyond 32-bit range, use 64-bit encoding
      if (value > 0x7fffffff || value < -0x80000000) {
        this.writeVarint64(this.zigzagEncode64(BigInt(value)));
      } else {
        this.writeVarint(this.zigzagEncode(value));
      }
    } else {
      this.view.setUint8(this.pos++, 2);
      this.view.setFloat64(this.pos, value, true); // little-endian
      this.pos += 8;
    }
  }

  // PrimitiveType.DATETIME
  writeDateTime(value: Temporal.ZonedDateTime): void {
    // convert to microseconds since epoch
    const micros = BigInt(value.epochMilliseconds) * 1000n + BigInt(value.nanosecond / 1000);
    this.writeVarint64(this.zigzagEncode64(micros));
  }

  // PrimitiveType.DATE
  writeDate(value: Temporal.PlainDate): void {
    // convert to days since epoch
    const days = _EPOCH_DATE.until(value, { largestUnit: "days" }).days;
    this.writeVarint(this.zigzagEncode(days));
  }

  // PrimitiveType.TIME
  writeTime(value: Temporal.PlainTime): void {
    const micros =
      BigInt(value.hour) * 3_600_000_000n +
      BigInt(value.minute) * 60_000_000n +
      BigInt(value.second) * 1_000_000n +
      BigInt(value.millisecond) * 1_000n +
      BigInt(value.microsecond);
    this.writeVarint64(micros);
  }

  // PrimitiveType.DURATION
  writeDuration(value: Temporal.Duration): void {
    // convert to total microseconds
    const totalMicroseconds = value.total({ unit: "microseconds" });
    this.writeVarint64(this.zigzagEncode64(BigInt(Math.floor(totalMicroseconds))));
  }

  // PrimitiveType.STRING
  writeString(value: string): void {
    const encoded = this.textEncoder.encode(value);
    this.writeBytes(encoded);
  }

  // PrimitiveType.UUID
  writeUuid(value: string): void {
    // Convert UUID string to 16 bytes
    const hex = value.replace(/-/g, "");
    if (hex.length !== 32) {
      throw new BinaryError("UUID must be a valid 32-character hex string");
    }
    this.ensureCapacity(16);
    for (let i = 0; i < 16; i++) {
      this.view.setUint8(this.pos++, parseInt(hex.slice(i * 2, i * 2 + 2), 16));
    }
  }

  // PrimitiveType.BYTES
  writeBytes(value: Uint8Array): void {
    this.writeVarint(value.length);
    this.ensureCapacity(value.length);
    new Uint8Array(this.buffer, this.pos, value.length).set(value);
    this.pos += value.length;
  }

  // PrimitiveType.JSON
  writeJson(value: any): void {
    this.writeString(JSON.stringify(value));
  }

  private writeVarint(value: number): void {
    this.ensureCapacity(5); // max 5 bytes for 32-bit
    // convert to unsigned 32-bit
    value = value >>> 0;
    while (value >= 0x80) {
      this.view.setUint8(this.pos++, (value & 0x7f) | 0x80);
      value = value >>> 7;
    }
    this.view.setUint8(this.pos++, value & 0x7f);
  }

  private writeVarint64(value: bigint): void {
    this.ensureCapacity(10); // max 10 bytes for 64-bit
    while (value >= 0x80n) {
      this.view.setUint8(this.pos++, Number(value & 0x7fn) | 0x80);
      value = value >> 7n;
    }
    this.view.setUint8(this.pos++, Number(value & 0x7fn));
  }

  private writeVarint128(value: bigint): void {
    this.ensureCapacity(19); // max 19 bytes for 128-bit
    while (value >= 0x80n) {
      this.view.setUint8(this.pos++, Number(value & 0x7fn) | 0x80);
      value = value >> 7n;
    }
    this.view.setUint8(this.pos++, Number(value & 0x7fn));
  }

  private zigzagEncode(value: number): number {
    // handle 32-bit signed integer
    return ((value << 1) ^ (value >> 31)) >>> 0;
  }

  private zigzagEncode64(value: bigint): bigint {
    if (value >= 0n) {
      return value << 1n;
    } else {
      return (-value << 1n) - 1n;
    }
  }

  private zigzagEncode128(value: bigint): bigint {
    if (value >= 0n) {
      return value << 1n;
    } else {
      return (-value << 1n) - 1n;
    }
  }

  private encodeFloat16(value: number): Uint8Array {
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
        // NaN
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

/**
 * Read binary data in our custom encoding. Little-endian, varint, zigzag, etc.
 */
export class BinaryReader {
  private buffer: Uint8Array;
  private view: DataView;
  private pos: number;
  private textDecoder: TextDecoder;

  constructor(data: Uint8Array) {
    this.buffer = data;
    this.view = new DataView(data.buffer, data.byteOffset, data.byteLength);
    this.pos = 0;
    this.textDecoder = new TextDecoder();
  }

  get remaining(): number {
    return this.buffer.length - this.pos;
  }

  // PrimitiveType.BOOLEAN
  readBool(): boolean {
    if (this.pos >= this.buffer.length) {
      throw new BinaryError(`unexpected end of buffer at ${this.pos}`);
    }
    return this.buffer[this.pos++] !== 0;
  }

  // PrimitiveType.SINT8
  readSInt8(): number {
    if (this.pos >= this.buffer.length) {
      throw new BinaryError(`unexpected end of buffer at ${this.pos}`);
    }
    return this.view.getInt8(this.pos++);
  }

  // PrimitiveType.SINT16
  readSInt16(): number {
    return this.zigzagDecode(this.readVarint());
  }

  // PrimitiveType.SINT32
  readSInt32(): number {
    return this.zigzagDecode(this.readVarint());
  }

  // PrimitiveType.SINT64
  readSInt64(): bigint {
    return this.zigzagDecode64(this.readVarint64());
  }

  // PrimitiveType.SINT128
  readSInt128(): bigint {
    return this.zigzagDecode128(this.readVarint128());
  }

  // PrimitiveType.UINT8
  readUint8(): number {
    if (this.pos >= this.buffer.length) {
      throw new BinaryError(`unexpected end of buffer at ${this.pos}`);
    }
    return this.buffer[this.pos++];
  }

  // PrimitiveType.UINT16
  readUint16(): number {
    return this.readVarint();
  }

  // PrimitiveType.UINT32
  readUint32(): number {
    return this.readVarint();
  }

  // PrimitiveType.UINT64
  readUint64(): bigint {
    return this.readVarint64();
  }

  // PrimitiveType.UINT128
  readUint128(): bigint {
    return this.readVarint128();
  }

  // PrimitiveType.FLOAT16
  readFloat16(): number {
    if (this.pos >= this.buffer.length) {
      throw new BinaryError(`unexpected end of buffer at ${this.pos}`);
    }

    const flag = this.buffer[this.pos++];
    if (flag === 0) {
      return 0.0;
    } else if (flag === 1) {
      if (this.pos + 2 > this.buffer.length) {
        throw new BinaryError(`unexpected end of buffer at ${this.pos}`);
      }
      const float16Bytes = new Uint8Array(this.buffer.slice(this.pos, this.pos + 2));
      this.pos += 2;
      return this.decodeFloat16(float16Bytes);
    } else {
      throw new BinaryError(`invalid float16 encoding flag at ${this.pos - 1}: ${flag}`);
    }
  }

  // PrimitiveType.FLOAT32
  readFloat32(): number {
    if (this.pos >= this.buffer.length) {
      throw new BinaryError(`unexpected end of buffer at ${this.pos}`);
    }

    const flag = this.buffer[this.pos++];
    if (flag === 0) {
      return 0.0;
    } else if (flag === 1) {
      if (this.pos + 4 > this.buffer.length) {
        throw new BinaryError(`unexpected end of buffer at ${this.pos}`);
      }
      const value = this.view.getFloat32(this.pos, true); // little-endian
      this.pos += 4;
      return value;
    } else {
      throw new BinaryError(`invalid float32 encoding flag at ${this.pos - 1}: ${flag}`);
    }
  }

  // PrimitiveType.FLOAT64
  readFloat64(): number {
    if (this.pos >= this.buffer.length) {
      throw new BinaryError(`unexpected end of buffer at ${this.pos}`);
    }

    const flag = this.buffer[this.pos++];
    if (flag === 0) {
      return 0.0;
    } else if (flag === 1) {
      // integer representation
      // check first byte to see if it's a large number
      const peekPos = this.pos;
      let shift = 0;
      let hasMoreBytes = true;
      while (hasMoreBytes && this.pos < this.buffer.length) {
        const byte = this.buffer[this.pos++];
        hasMoreBytes = (byte & 0x80) !== 0;
        shift += 7;
      }
      this.pos = peekPos; // reset position

      if (shift > 35) {
        // large number, use 64-bit decoding
        return Number(this.zigzagDecode64(this.readVarint64()));
      } else {
        // regular 32-bit number
        return this.zigzagDecode(this.readVarint());
      }
    } else if (flag === 2) {
      if (this.pos + 8 > this.buffer.length) {
        throw new BinaryError(`unexpected end of buffer at ${this.pos}`);
      }
      const value = this.view.getFloat64(this.pos, true); // little-endian
      this.pos += 8;
      return value;
    } else {
      throw new BinaryError(`invalid float64 encoding flag at ${this.pos - 1}: ${flag}`);
    }
  }

  // PrimitiveType.DATETIME
  readDateTime(): Temporal.ZonedDateTime {
    const micros = this.zigzagDecode64(this.readVarint64());
    return Temporal.Instant.fromEpochNanoseconds(micros * 1000n).toZonedDateTimeISO("UTC");
  }

  // PrimitiveType.DATE
  readDate(): Temporal.PlainDate {
    const days = this.zigzagDecode(this.readVarint());
    return _EPOCH_DATE.add({ days });
  }

  // PrimitiveType.TIME
  readTime(): Temporal.PlainTime {
    const micros = this.readVarint64();
    const hours = Number(micros / 3_600_000_000n);
    const remaining1 = micros % 3_600_000_000n;
    const minutes = Number(remaining1 / 60_000_000n);
    const remaining2 = remaining1 % 60_000_000n;
    const seconds = Number(remaining2 / 1_000_000n);
    const remaining3 = remaining2 % 1_000_000n;
    const milliseconds = Number(remaining3 / 1_000n);
    const microseconds = Number(remaining3 % 1_000n);
    return new Temporal.PlainTime(hours, minutes, seconds, milliseconds, microseconds);
  }

  // PrimitiveType.DURATION
  readDuration(): Temporal.Duration {
    const micros = Number(this.zigzagDecode64(this.readVarint64()));
    return Temporal.Duration.from({ microseconds: micros });
  }

  // PrimitiveType.STRING
  readString(): string {
    const bytes = this.readBytes();
    return this.textDecoder.decode(bytes);
  }

  // PrimitiveType.UUID
  readUuid(): string {
    if (this.pos + 16 > this.buffer.length) {
      throw new BinaryError(`unexpected end of buffer at ${this.pos}`);
    }
    // read 16 bytes and convert to UUID string
    const bytes = new Uint8Array(16);
    for (let i = 0; i < 16; i++) {
      bytes[i] = this.buffer[this.pos++];
    }
    // convert bytes to hex string
    const hex = Array.from(bytes)
      .map((b) => b.toString(16).padStart(2, "0"))
      .join("");

    // format as UUID
    return `${hex.slice(0, 8)}-${hex.slice(8, 12)}-${hex.slice(12, 16)}-${hex.slice(16, 20)}-${hex.slice(20, 32)}`;
  }

  // PrimitiveType.BYTES
  readBytes(): Uint8Array {
    const length = this.readVarint();
    if (this.pos + length > this.buffer.length) {
      throw new BinaryError(`unexpected end of buffer at ${this.pos}`);
    }
    const value = this.buffer.slice(this.pos, this.pos + length);
    this.pos += length;
    return value;
  }

  // PrimitiveType.JSON
  readJson(): any {
    return JSON.parse(this.readString());
  }

  private readVarint(): number {
    let value = 0;
    let shift = 0;

    while (true) {
      if (this.pos >= this.buffer.length) {
        throw new BinaryError(`unexpected end of buffer at ${this.pos}`);
      }

      const byte = this.buffer[this.pos++];
      value |= (byte & 0x7f) << shift;

      if ((byte & 0x80) === 0) {
        return value >>> 0; // ensure unsigned
      }

      shift += 7;
      if (shift >= 35) {
        throw new BinaryError(`varint too long at ${this.pos}`);
      }
    }
  }

  private readVarint64(): bigint {
    let value = 0n;
    let shift = 0n;

    while (true) {
      if (this.pos >= this.buffer.length) {
        throw new BinaryError(`unexpected end of buffer at ${this.pos}`);
      }

      const byte = this.buffer[this.pos++];
      value |= BigInt(byte & 0x7f) << shift;

      if ((byte & 0x80) === 0) {
        return value;
      }

      shift += 7n;
      if (shift >= 70n) {
        throw new BinaryError(`varint too long at ${this.pos}`);
      }
    }
  }

  private readVarint128(): bigint {
    let value = 0n;
    let shift = 0n;

    while (true) {
      if (this.pos >= this.buffer.length) {
        throw new BinaryError(`unexpected end of buffer at ${this.pos}`);
      }

      const byte = this.buffer[this.pos++];
      value |= BigInt(byte & 0x7f) << shift;

      if ((byte & 0x80) === 0) {
        return value;
      }

      shift += 7n;
      if (shift >= 140n) {
        throw new BinaryError(`varint too long at ${this.pos}`);
      }
    }
  }

  private zigzagDecode(value: number): number {
    // properly handle 32-bit unsigned to signed conversion
    return ((value >>> 1) ^ -(value & 1)) | 0;
  }

  private zigzagDecode64(value: bigint): bigint {
    if (value & 1n) {
      return -((value + 1n) >> 1n);
    } else {
      return value >> 1n;
    }
  }

  private zigzagDecode128(value: bigint): bigint {
    if (value & 1n) {
      return -((value + 1n) >> 1n);
    } else {
      return value >> 1n;
    }
  }

  private decodeFloat16(bytes: Uint8Array): number {
    // convert little-endian bytes to float16 bits
    const float16bits = bytes[0] | (bytes[1] << 8);
    // extract components
    const sign = (float16bits >> 15) & 0x1;
    const exponent = (float16bits >> 10) & 0x1f;
    const mantissa = float16bits & 0x3ff;

    if (exponent === 0x1f) {
      // infinity or NaN
      if (mantissa === 0) {
        // infinity
        return sign ? Number.NEGATIVE_INFINITY : Number.POSITIVE_INFINITY;
      } else {
        // NaN
        return Number.NaN;
      }
    } else if (exponent === 0) {
      // zero or subnormal
      if (mantissa === 0) {
        // zero
        return sign ? -0 : 0;
      } else {
        // subnormal number
        const value = 2 ** -14 * (mantissa / 1024);
        return sign ? -value : value;
      }
    } else {
      // normal number
      const newExponent = exponent - 15 + 127;
      const float32bits = (sign << 31) | (newExponent << 23) | (mantissa << 13);
      const float32 = new Uint32Array([float32bits]);
      return new Float32Array(float32.buffer)[0];
    }
  }
}
