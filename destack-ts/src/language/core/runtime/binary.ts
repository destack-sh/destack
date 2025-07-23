import { Temporal } from "temporal-polyfill";

export class BinaryError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "BinaryError";
  }
}

/**
 * Write binary data with variable-length encoding for integers.
 */
export class BinaryWriter {
  private buffer: ArrayBuffer;
  private view: DataView;
  private pos: number;
  private textEncoder: TextEncoder;

  constructor() {
    this.buffer = new ArrayBuffer(256); // initial size
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

  // PrimitiveType.INT8
  writeInt8(value: number): void {
    this.ensureCapacity(1);
    this.view.setInt8(this.pos++, value);
  }

  // PrimitiveType.UINT8
  writeUint8(value: number): void {
    this.ensureCapacity(1);
    this.view.setUint8(this.pos++, value);
  }

  // PrimitiveType.INT16
  writeInt16(value: number): void {
    this.writeVarint(this.zigzagEncode(value));
  }

  // PrimitiveType.UINT16
  writeUint16(value: number): void {
    this.writeVarint(value);
  }

  // PrimitiveType.INT32
  writeInt32(value: number): void {
    this.writeVarint(this.zigzagEncode(value));
  }

  // PrimitiveType.UINT32
  writeUint32(value: number): void {
    this.writeVarint(value);
  }

  // PrimitiveType.INT64
  writeInt64(value: bigint): void {
    this.writeVarint64(this.zigzagEncode64(value));
  }

  // PrimitiveType.UINT64
  writeUint64(value: bigint): void {
    this.writeVarint64(value);
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
      if (value > 0x7FFFFFFF || value < -0x80000000) {
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
    const epoch = new Temporal.PlainDate(1970, 1, 1);
    const days = epoch.until(value, { largestUnit: "days" }).days;
    this.writeVarint(this.zigzagEncode(days));
  }

  // PrimitiveType.TIME
  writeTime(value: Temporal.PlainTime): void {
    const micros = BigInt(value.hour) * 3_600_000_000n + 
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
  writeUuid(value: Uint8Array): void {
    if (value.length !== 16) {
      throw new BinaryError("UUID must be exactly 16 bytes");
    }
    this.ensureCapacity(16);
    new Uint8Array(this.buffer, this.pos, 16).set(value);
    this.pos += 16;
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

  private zigzagEncode(value: number): number {
    // handle 32-bit signed integer
    return ((value << 1) ^ (value >> 31)) >>> 0;
  }

  private zigzagEncode64(value: bigint): bigint {
    if (value >= 0n) {
      return value << 1n;
    } else {
      return ((-value) << 1n) - 1n;
    }
  }
}

/**
 * Read binary data with variable-length encoding for integers.
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

  // PrimitiveType.INT8
  readInt8(): number {
    if (this.pos >= this.buffer.length) {
      throw new BinaryError(`unexpected end of buffer at ${this.pos}`);
    }
    return this.view.getInt8(this.pos++);
  }

  // PrimitiveType.UINT8
  readUint8(): number {
    if (this.pos >= this.buffer.length) {
      throw new BinaryError(`unexpected end of buffer at ${this.pos}`);
    }
    return this.buffer[this.pos++];
  }

  // PrimitiveType.INT16
  readInt16(): number {
    return this.zigzagDecode(this.readVarint());
  }

  // PrimitiveType.UINT16
  readUint16(): number {
    return this.readVarint();
  }

  // PrimitiveType.INT32
  readInt32(): number {
    return this.zigzagDecode(this.readVarint());
  }

  // PrimitiveType.UINT32
  readUint32(): number {
    return this.readVarint();
  }

  // PrimitiveType.INT64
  readInt64(): bigint {
    return this.zigzagDecode64(this.readVarint64());
  }

  // PrimitiveType.UINT64
  readUint64(): bigint {
    return this.readVarint64();
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
      // Check first byte to see if it's a large number
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
        // Large number, use 64-bit decoding
        return Number(this.zigzagDecode64(this.readVarint64()));
      } else {
        // Regular 32-bit number
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
    const epoch = new Temporal.PlainDate(1970, 1, 1);
    return epoch.add({ days });
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
  readUuid(): Uint8Array {
    if (this.pos + 16 > this.buffer.length) {
      throw new BinaryError(`unexpected end of buffer at ${this.pos}`);
    }
    const uuid = new Uint8Array(16);
    uuid.set(this.buffer.slice(this.pos, this.pos + 16));
    this.pos += 16;
    return uuid;
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
}
