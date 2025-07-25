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
  /**
   * Write a boolean as 1 byte (0 for false, 1 for true).
   * Size: 1 byte.
   */
  writeBool(value: boolean): void {
    this.ensureCapacity(1);
    this.view.setUint8(this.pos++, value ? 1 : 0);
  }

  // PrimitiveType.INT8
  /**
   * Write a signed 8-bit integer as raw byte.
   * Size: 1 byte.
   */
  writeInt8(value: number): void {
    this.ensureCapacity(1);
    this.view.setInt8(this.pos++, value);
  }

  // PrimitiveType.INT16
  /**
   * Write a signed 16-bit integer using zigzag encoding then varint.
   * Size: 1-3 bytes.
   */
  writeInt16(value: number): void {
    this.writeVarint(this.zigzagEncode(value));
  }

  // PrimitiveType.INT32
  /**
   * Write a signed 32-bit integer using zigzag encoding then varint.
   * Size: 1-5 bytes.
   */
  writeInt32(value: number): void {
    this.writeVarint(this.zigzagEncode(value));
  }

  // PrimitiveType.INT64
  /**
   * Write a signed 64-bit integer using zigzag encoding then varint.
   * Size: 1-10 bytes.
   */
  writeInt64(value: bigint): void {
    this.writeVarint64(this.zigzagEncode64(value));
  }

  // PrimitiveType.INT128
  /**
   * Write a signed 128-bit integer using zigzag encoding then varint.
   * Size: 1-19 bytes.
   */
  writeInt128(value: bigint): void {
    this.writeVarint128(this.zigzagEncode128(value));
  }

  // PrimitiveType.UINT8
  /**
   * Write an unsigned 8-bit integer as raw byte.
   * Size: 1 byte.
   */
  writeUint8(value: number): void {
    this.ensureCapacity(1);
    this.view.setUint8(this.pos++, value);
  }

  // PrimitiveType.UINT16
  /**
   * Write an unsigned 16-bit integer using varint encoding.
   * Size: 1-3 bytes.
   */
  writeUint16(value: number): void {
    this.writeVarint(value);
  }

  // PrimitiveType.UINT32
  /**
   * Write an unsigned 32-bit integer using varint encoding.
   * Size: 1-5 bytes.
   */
  writeUint32(value: number): void {
    this.writeVarint(value);
  }

  // PrimitiveType.UINT64
  /**
   * Write an unsigned 64-bit integer using varint encoding.
   * Size: 1-10 bytes.
   */
  writeUint64(value: bigint): void {
    this.writeVarint64(value);
  }

  // PrimitiveType.UINT128
  /**
   * Write an unsigned 128-bit integer using varint encoding.
   * Size: 1-19 bytes.
   */
  writeUint128(value: bigint): void {
    this.writeVarint128(value);
  }

  // PrimitiveType.FLOAT16
  /**
   * Write a 16-bit float with flag byte:
   *   - 0: zero (1 byte)
   *   - 1: negative zero (1 byte)
   *   - 2: positive infinity (1 byte)
   *   - 3: negative infinity (1 byte)
   *   - 4: NaN (1 byte)
   *   - 5: sint8 for integers (2 bytes)
   *   - 6: little-endian float16 for others (3 bytes)
   */
  writeFloat16(value: number): void {
    this.ensureCapacity(3);
    if (value === 0.0) {
      if (Object.is(value, -0)) {
        this.view.setUint8(this.pos++, 1); // negative zero
      } else {
        this.view.setUint8(this.pos++, 0); // positive zero
      }
    } else if (value === Number.POSITIVE_INFINITY) {
      this.view.setUint8(this.pos++, 2); // positive infinity
    } else if (value === Number.NEGATIVE_INFINITY) {
      this.view.setUint8(this.pos++, 3); // negative infinity
    } else if (Number.isNaN(value)) {
      this.view.setUint8(this.pos++, 4); // NaN
    } else if (Number.isInteger(value) && value >= -(2 ** 7) && value <= 2 ** 7) {
      // can be represented as sint8
      this.view.setUint8(this.pos++, 5);
      this.writeInt8(value);
    } else {
      this.view.setUint8(this.pos++, 6);
      // Encode float16
      const float16Bytes = this.encodeFloat16(value);
      this.view.setUint8(this.pos++, float16Bytes[0]);
      this.view.setUint8(this.pos++, float16Bytes[1]);
    }
  }

  // PrimitiveType.FLOAT32
  /**
   * Write a 32-bit float with flag byte:
   *   - 0: zero (1 byte)
   *   - 1: negative zero (1 byte)
   *   - 2: positive infinity (1 byte)
   *   - 3: negative infinity (1 byte)
   *   - 4: NaN (1 byte)
   *   - 5: sint16 for integers (3 bytes)
   *   - 6: little-endian float32 for others (5 bytes)
   */
  writeFloat32(value: number): void {
    this.ensureCapacity(5);
    if (value === 0.0) {
      if (Object.is(value, -0)) {
        this.view.setUint8(this.pos++, 1); // negative zero
      } else {
        this.view.setUint8(this.pos++, 0); // positive zero
      }
    } else if (value === Number.POSITIVE_INFINITY) {
      this.view.setUint8(this.pos++, 2); // positive infinity
    } else if (value === Number.NEGATIVE_INFINITY) {
      this.view.setUint8(this.pos++, 3); // negative infinity
    } else if (Number.isNaN(value)) {
      this.view.setUint8(this.pos++, 4); // NaN
    } else if (Number.isInteger(value) && value >= -(2 ** 15) && value <= 2 ** 15) {
      // can be represented as sint16
      this.view.setUint8(this.pos++, 5);
      this.writeInt16(value);
    } else {
      this.view.setUint8(this.pos++, 6);
      this.view.setFloat32(this.pos, value, true); // little-endian
      this.pos += 4;
    }
  }

  // PrimitiveType.FLOAT64
  /**
   * Write a 64-bit float with flag byte:
   *   - 0: zero (1 byte)
   *   - 1: negative zero (1 byte)
   *   - 2: positive infinity (1 byte)
   *   - 3: negative infinity (1 byte)
   *   - 4: NaN (1 byte)
   *   - 5: sint32 for integers (6 bytes)
   *   - 6: little-endian float64 for others (9 bytes)
   */
  writeFloat64(value: number): void {
    this.ensureCapacity(9);
    if (value === 0.0) {
      if (Object.is(value, -0)) {
        this.view.setUint8(this.pos++, 1); // negative zero
      } else {
        this.view.setUint8(this.pos++, 0); // positive zero
      }
    } else if (value === Number.POSITIVE_INFINITY) {
      this.view.setUint8(this.pos++, 2); // positive infinity
    } else if (value === Number.NEGATIVE_INFINITY) {
      this.view.setUint8(this.pos++, 3); // negative infinity
    } else if (Number.isNaN(value)) {
      this.view.setUint8(this.pos++, 4); // NaN
    } else if (Number.isInteger(value) && value >= -(2 ** 31) && value <= 2 ** 31) {
      // can be represented as sint32
      this.view.setUint8(this.pos++, 5);
      this.writeInt32(value);
    } else {
      this.view.setUint8(this.pos++, 6);
      this.view.setFloat64(this.pos, value, true); // little-endian
      this.pos += 8;
    }
  }

  // PrimitiveType.DATETIME
  /**
   * Write a datetime as zigzag-encoded varint of microseconds since epoch.
   * Size: 1-10 bytes.
   */
  writeDateTime(value: Temporal.ZonedDateTime): void {
    // convert to microseconds since epoch (64-bit)
    const micros = BigInt(value.epochMilliseconds) * 1000n + BigInt(value.nanosecond / 1000);
    this.writeVarint64(this.zigzagEncode64(micros));
  }

  // PrimitiveType.DATE
  /**
   * Write a date as zigzag-encoded varint of days since epoch (1970-01-01).
   * Size: 1-5 bytes.
   */
  writeDate(value: Temporal.PlainDate): void {
    // convert to days since epoch
    const days = _EPOCH_DATE.until(value, { largestUnit: "days" }).days;
    this.writeVarint(this.zigzagEncode(days));
  }

  // PrimitiveType.TIME
  /**
   * Write a time as varint of microseconds since midnight.
   * Size: 1-6 bytes.
   */
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
  /**
   * Write a duration as zigzag-encoded varint of microseconds.
   * Size: 1-10 bytes.
   */
  writeDuration(value: Temporal.Duration): void {
    // convert to total microseconds
    const totalMicroseconds = value.total({ unit: "microseconds" });
    this.writeVarint64(this.zigzagEncode64(BigInt(Math.round(totalMicroseconds))));
  }

  // PrimitiveType.STRING
  /**
   * Write a UTF-8 string with varint length prefix.
   * Size: 1-5 bytes (length) + string length in bytes.
   */
  writeString(value: string): void {
    const encoded = this.textEncoder.encode(value);
    this.writeBytes(encoded);
  }

  // PrimitiveType.UUID
  /**
   * Write a UUID as 16 raw bytes.
   * Size: 16 bytes.
   */
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
  /**
   * Write raw bytes with varint length prefix.
   * Size: 1-5 bytes (length) + data length.
   */
  writeBytes(value: Uint8Array): void {
    this.writeVarint(value.length);
    this.ensureCapacity(value.length);
    new Uint8Array(this.buffer, this.pos, value.length).set(value);
    this.pos += value.length;
  }

  // PrimitiveType.JSON
  /**
   * Write JSON as UTF-8 string with varint length prefix.
   * Size: 1-5 bytes (length) + JSON string length in bytes.
   */
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
  /**
   * Read a boolean from 1 byte (0 for false, non-zero for true).
   * Size: 1 byte.
   */
  readBool(): boolean {
    if (this.pos >= this.buffer.length) {
      throw new BinaryError(`unexpected end of buffer at ${this.pos}`);
    }
    return this.buffer[this.pos++] !== 0;
  }

  // PrimitiveType.INT8
  /**
   * Read a signed 8-bit integer from raw byte.
   * Size: 1 byte.
   */
  readInt8(): number {
    if (this.pos >= this.buffer.length) {
      throw new BinaryError(`unexpected end of buffer at ${this.pos}`);
    }
    return this.view.getInt8(this.pos++);
  }

  // PrimitiveType.INT16
  /**
   * Read a signed 16-bit integer from zigzag-decoded varint.
   * Size: 1-3 bytes.
   */
  readInt16(): number {
    return this.zigzagDecode(this.readVarint());
  }

  // PrimitiveType.INT32
  /**
   * Read a signed 32-bit integer from zigzag-decoded varint.
   * Size: 1-5 bytes.
   */
  readInt32(): number {
    return this.zigzagDecode(this.readVarint());
  }

  // PrimitiveType.INT64
  /**
   * Read a signed 64-bit integer from zigzag-decoded varint.
   * Size: 1-10 bytes.
   */
  readInt64(): bigint {
    return this.zigzagDecode64(this.readVarint64());
  }

  // PrimitiveType.INT128
  /**
   * Read a signed 128-bit integer from zigzag-decoded varint.
   * Size: 1-19 bytes.
   */
  readInt128(): bigint {
    return this.zigzagDecode128(this.readVarint128());
  }

  // PrimitiveType.UINT8
  /**
   * Read an unsigned 8-bit integer from raw byte.
   * Size: 1 byte.
   */
  readUint8(): number {
    if (this.pos >= this.buffer.length) {
      throw new BinaryError(`unexpected end of buffer at ${this.pos}`);
    }
    return this.buffer[this.pos++];
  }

  // PrimitiveType.UINT16
  /**
   * Read an unsigned 16-bit integer from varint.
   * Size: 1-3 bytes.
   */
  readUint16(): number {
    return this.readVarint();
  }

  // PrimitiveType.UINT32
  /**
   * Read an unsigned 32-bit integer from varint.
   * Size: 1-5 bytes.
   */
  readUint32(): number {
    return this.readVarint();
  }

  // PrimitiveType.UINT64
  /**
   * Read an unsigned 64-bit integer from varint.
   * Size: 1-10 bytes.
   */
  readUint64(): bigint {
    return this.readVarint64();
  }

  // PrimitiveType.UINT128
  /**
   * Read an unsigned 128-bit integer from varint.
   * Size: 1-19 bytes.
   */
  readUint128(): bigint {
    return this.readVarint128();
  }

  // PrimitiveType.FLOAT16
  /**
   * Read a 16-bit float from flag byte + data:
   *   - 0: zero (1 byte)
   *   - 1: negative zero (1 byte)
   *   - 2: positive infinity (1 byte)
   *   - 3: negative infinity (1 byte)
   *   - 4: NaN (1 byte)
   *   - 5: sint8 for integers (2 bytes)
   *   - 6: little-endian float16 for others (3 bytes)
   */
  readFloat16(): number {
    if (this.pos >= this.buffer.length) {
      throw new BinaryError(`unexpected end of buffer at ${this.pos}`);
    }

    const flag = this.buffer[this.pos++];
    if (flag === 0) {
      return 0.0;
    } else if (flag === 1) {
      return -0.0;
    } else if (flag === 2) {
      return Number.POSITIVE_INFINITY;
    } else if (flag === 3) {
      return Number.NEGATIVE_INFINITY;
    } else if (flag === 4) {
      return Number.NaN;
    } else if (flag === 5) {
      return this.readInt8();
    } else if (flag === 6) {
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
  /**
   * Read a 32-bit float from flag byte + data:
   *   - 0: zero (1 byte)
   *   - 1: negative zero (1 byte)
   *   - 2: positive infinity (1 byte)
   *   - 3: negative infinity (1 byte)
   *   - 4: NaN (1 byte)
   *   - 5: sint16 for integers (3 bytes)
   *   - 6: little-endian float32 for others (5 bytes)
   */
  readFloat32(): number {
    if (this.pos >= this.buffer.length) {
      throw new BinaryError(`unexpected end of buffer at ${this.pos}`);
    }

    const flag = this.buffer[this.pos++];
    if (flag === 0) {
      return 0.0;
    } else if (flag === 1) {
      return -0.0;
    } else if (flag === 2) {
      return Number.POSITIVE_INFINITY;
    } else if (flag === 3) {
      return Number.NEGATIVE_INFINITY;
    } else if (flag === 4) {
      return Number.NaN;
    } else if (flag === 5) {
      return this.readInt16();
    } else if (flag === 6) {
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
  /**
   * Read a 64-bit float from flag byte + data:
   *   - 0: zero (1 byte)
   *   - 1: negative zero (1 byte)
   *   - 2: positive infinity (1 byte)
   *   - 3: negative infinity (1 byte)
   *   - 4: NaN (1 byte)
   *   - 5: sint32 for integers (6 bytes)
   *   - 6: little-endian float64 for others (9 bytes)
   */
  readFloat64(): number {
    if (this.pos >= this.buffer.length) {
      throw new BinaryError(`unexpected end of buffer at ${this.pos}`);
    }

    const flag = this.buffer[this.pos++];
    if (flag === 0) {
      return 0.0;
    } else if (flag === 1) {
      return -0.0;
    } else if (flag === 2) {
      return Number.POSITIVE_INFINITY;
    } else if (flag === 3) {
      return Number.NEGATIVE_INFINITY;
    } else if (flag === 4) {
      return Number.NaN;
    } else if (flag === 5) {
      return this.readInt32();
    } else if (flag === 6) {
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
  /**
   * Read a datetime from zigzag-decoded varint of microseconds since epoch.
   * Size: 1-10 bytes.
   */
  readDateTime(): Temporal.ZonedDateTime {
    const micros = this.zigzagDecode64(this.readVarint64());
    return Temporal.Instant.fromEpochNanoseconds(micros * 1000n).toZonedDateTimeISO("UTC");
  }

  // PrimitiveType.DATE
  /**
   * Read a date from zigzag-decoded varint of days since epoch (1970-01-01).
   * Size: 1-5 bytes.
   */
  readDate(): Temporal.PlainDate {
    const days = this.zigzagDecode(this.readVarint());
    return _EPOCH_DATE.add({ days });
  }

  // PrimitiveType.TIME
  /**
   * Read a time from varint of microseconds since midnight.
   * Size: 1-6 bytes.
   */
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
  /**
   * Read a duration from zigzag-decoded varint of microseconds.
   * Size: 1-10 bytes.
   */
  readDuration(): Temporal.Duration {
    const micros = Number(this.zigzagDecode64(this.readVarint64()));
    return Temporal.Duration.from({ microseconds: micros });
  }

  // PrimitiveType.STRING
  /**
   * Read a UTF-8 string from varint length prefix + UTF-8 bytes.
   * Size: 1-5 bytes (length) + string length in bytes.
   */
  readString(): string {
    const bytes = this.readBytes();
    return this.textDecoder.decode(bytes);
  }

  // PrimitiveType.UUID
  /**
   * Read a UUID from 16 raw bytes.
   * Size: 16 bytes.
   */
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
  /**
   * Read raw bytes from varint length prefix + data bytes.
   * Size: 1-5 bytes (length) + data length.
   */
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
  /**
   * Read JSON from UTF-8 string with varint length prefix.
   * Size: 1-5 bytes (length) + JSON string length in bytes.
   */
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
