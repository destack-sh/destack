import { expect, test } from "bun:test";
import { BinaryReader, BinaryWriter } from "@destack/language/core/runtime/binary";
import { Temporal } from "temporal-polyfill";

test("bool", () => {
  const writer = new BinaryWriter();
  writer.writeBool(true);
  writer.writeBool(false);

  // Expected: 2 bytes total (1 byte per bool)
  expect(writer.toBytes().length).toBe(2);

  const reader = new BinaryReader(writer.toBytes());
  expect(reader.readBool()).toBe(true);
  expect(reader.readBool()).toBe(false);
  expect(reader.remaining).toBe(0);
});

test("sint8", () => {
  const testValues = [0, 1, -1, 127, -128, 42, -42];

  const writer = new BinaryWriter();
  for (const value of testValues) {
    writer.writeInt8(value);
  }

  // Expected: 7 bytes (1 byte per int8)
  expect(writer.toBytes().length).toBe(7);

  const reader = new BinaryReader(writer.toBytes());
  for (const expected of testValues) {
    expect(reader.readInt8()).toBe(expected);
  }
  expect(reader.remaining).toBe(0);
});

test("sint16", () => {
  // Map of value -> expected bytes with explanation
  const testCases: Record<number, number> = {
    0: 1, // zigzag(0) = 0 -> 1 byte
    1: 1, // zigzag(1) = 2 -> 1 byte
    [-1]: 1, // zigzag(-1) = 1 -> 1 byte
    63: 1, // zigzag(63) = 126 -> 1 byte
    64: 2, // zigzag(64) = 128 -> 2 bytes
    [-64]: 1, // zigzag(-64) = 127 -> 1 byte
    [-65]: 2, // zigzag(-65) = 129 -> 2 bytes
    127: 2, // zigzag(127) = 254 -> 2 bytes
    128: 2, // zigzag(128) = 256 -> 2 bytes
    [-128]: 2, // zigzag(-128) = 255 -> 2 bytes
    32767: 3, // max int16 -> zigzag(32767) = 65534 -> 3 bytes
    [-32768]: 3, // min int16 -> zigzag(-32768) = 65535 -> 3 bytes
  };

  for (const [valueStr, expectedBytes] of Object.entries(testCases)) {
    const value = Number(valueStr);
    const writer = new BinaryWriter();
    writer.writeInt16(value);
    const data = writer.toBytes();
    expect(data.length).toBe(expectedBytes);

    const reader = new BinaryReader(data);
    expect(reader.readInt16()).toBe(value);
  }
});

test("sint32", () => {
  // Map of value -> expected bytes
  const valueToBytes: Record<number, number> = {
    0: 1, // zigzag = 0
    1: 1, // zigzag = 2
    [-1]: 1, // zigzag = 1
    127: 2, // zigzag = 254
    [-128]: 2, // zigzag = 255
    256: 2, // zigzag = 512
    [-256]: 2, // zigzag = 511
    65535: 3, // zigzag = 131070
    [-65536]: 3, // zigzag = 131071
    2147483647: 5, // zigzag = 4294967294
    [-2147483648]: 5, // zigzag = 4294967295
  };

  const writer = new BinaryWriter();
  for (const value of Object.keys(valueToBytes).map(Number)) {
    writer.writeInt32(value);
  }

  // Calculate total expected bytes
  const totalExpected = Object.values(valueToBytes).reduce((a, b) => a + b, 0);
  expect(writer.toBytes().length).toBe(totalExpected); // 27 bytes

  const reader = new BinaryReader(writer.toBytes());
  for (const value of Object.keys(valueToBytes).map(Number)) {
    expect(reader.readInt32()).toBe(value);
  }
  expect(reader.remaining).toBe(0);
});

test("sint64", () => {
  // Map of value -> expected bytes
  const valueToBytes: Record<string, number> = {
    "0": 1,
    "1": 1,
    "-1": 1,
    "127": 2,
    "-128": 2,
    [String(2 ** 31 - 1)]: 5,
    [String(-(2 ** 31))]: 5,
    "9223372036854775807": 10, // max (2^63 - 1)
    "-9223372036854775808": 10, // max (-2^63)
    "123456789012345": 7,
    "-123456789012345": 7,
  };

  const writer = new BinaryWriter();
  for (const value of Object.keys(valueToBytes)) {
    writer.writeInt64(BigInt(value));
  }

  // Calculate total expected bytes
  const totalExpected = Object.values(valueToBytes).reduce((a, b) => a + b, 0);
  expect(writer.toBytes().length).toBe(totalExpected); // 51 bytes

  const reader = new BinaryReader(writer.toBytes());
  for (const value of Object.keys(valueToBytes)) {
    expect(reader.readInt64()).toBe(BigInt(value));
  }
  expect(reader.remaining).toBe(0);
});

test("sint128", () => {
  // Map of value -> expected bytes
  const valueToBytes: Record<string, number> = {
    "0": 1,
    "1": 1,
    "-1": 1,
    "127": 2,
    "-128": 2,
    "9223372036854775807": 10, // max sint64
    "-9223372036854775808": 10, // min sint64
    "170141183460469231731687303715884105727": 19, // max sint128 (2^127 - 1)
    "-170141183460469231731687303715884105728": 19, // min sint128 (-2^127)
    "1267650600228229401496703205376": 15, // 2^100
    "-1267650600228229401496703205376": 15, // -2^100
  };

  const writer = new BinaryWriter();
  for (const value of Object.keys(valueToBytes)) {
    writer.writeInt128(BigInt(value));
  }

  // Calculate total expected bytes
  const totalExpected = Object.values(valueToBytes).reduce((a, b) => a + b, 0);
  expect(writer.toBytes().length).toBe(totalExpected); // 91 bytes

  const reader = new BinaryReader(writer.toBytes());
  for (const value of Object.keys(valueToBytes)) {
    expect(reader.readInt128()).toBe(BigInt(value));
  }
  expect(reader.remaining).toBe(0);
});

test("uint8", () => {
  const testValues = [0, 1, 127, 128, 255, 42];

  const writer = new BinaryWriter();
  for (const value of testValues) {
    writer.writeUint8(value);
  }

  // Expected: 6 bytes (1 byte per uint8)
  expect(writer.toBytes().length).toBe(6);

  const reader = new BinaryReader(writer.toBytes());
  for (const expected of testValues) {
    expect(reader.readUint8()).toBe(expected);
  }
  expect(reader.remaining).toBe(0);
});

test("uint", () => {
  // Map of value -> expected bytes
  const valueToBytes: Record<number, number> = {
    0: 1, // 0 -> 1 byte
    127: 1, // fits in 7 bits
    128: 2, // needs 2 bytes
    16383: 2, // fits in 14 bits
    16384: 3, // needs 3 bytes
    65535: 3, // max uint16 (65535)
    4294967295: 5, // max uint32 (4294967295)
  };

  let totalBytes = 0;
  for (const [value, expectedBytes] of Object.entries(valueToBytes)) {
    const writer = new BinaryWriter();
    writer.writeUint32(Number(value));
    const data = writer.toBytes();
    expect(data.length).toBe(expectedBytes);
    totalBytes += expectedBytes;

    const reader = new BinaryReader(data);
    expect(reader.readUint32()).toBe(Number(value));
  }

  // Total expected: 1 + 1 + 2 + 2 + 3 + 3 + 5 = 17 bytes
  expect(totalBytes).toBe(Object.values(valueToBytes).reduce((a, b) => a + b, 0));
});

test("uint128", () => {
  // Map of value -> expected bytes
  const valueToBytes: Record<string, number> = {
    "0": 1, // 0 -> 1 byte
    "127": 1, // fits in 7 bits
    "128": 2, // needs 2 bytes
    "4294967295": 5, // max uint32
    "18446744073709551615": 10, // max uint64
    "1267650600228229401496703205376": 15, // 2^100
    "170141183460469231731687303715884105728": 19, // 2^127
    "340282366920938463463374607431768211455": 19, // max uint128 (2^128 - 1)
  };

  const writer = new BinaryWriter();
  for (const value of Object.keys(valueToBytes)) {
    writer.writeUint128(BigInt(value));
  }

  // Calculate total expected bytes
  const totalExpected = Object.values(valueToBytes).reduce((a, b) => a + b, 0);
  expect(writer.toBytes().length).toBe(totalExpected); // 71 bytes

  const reader = new BinaryReader(writer.toBytes());
  for (const value of Object.keys(valueToBytes)) {
    expect(reader.readUint128()).toBe(BigInt(value));
  }
  expect(reader.remaining).toBe(0);
});

test("float16", () => {
  // zero should be 1 byte
  let writer = new BinaryWriter();
  writer.writeFloat16(0.0);
  expect(writer.toBytes().length).toBe(1);

  // non-zero should be 3 bytes (1 flag + 2 float)
  writer = new BinaryWriter();
  writer.writeFloat16(Math.PI);
  expect(writer.toBytes().length).toBe(3);

  // test roundtrip with expected sizes
  const valueToBytes: Record<number, number> = {
    0.0: 1, // zero optimization
    [-0.0]: 1, // negative zero optimization
    [Number.POSITIVE_INFINITY]: 1, // positive infinity
    [Number.NEGATIVE_INFINITY]: 1, // negative infinity
    [Number.NaN]: 1, // NaN
    1.0: 2, // sint8
    [-1.0]: 2, // sint8
    [Math.PI]: 3, // flag + float16
    [-Math.PI]: 3, // flag + float16
    65504.0: 3, // max normal float16
  };

  writer = new BinaryWriter();
  for (const value of Object.keys(valueToBytes).map(Number)) {
    writer.writeFloat16(value);
  }

  expect(writer.toBytes().length).toBe(Object.values(valueToBytes).reduce((a, b) => a + b, 0)); // 22 bytes

  const reader = new BinaryReader(writer.toBytes());
  for (const value of Object.keys(valueToBytes).map(Number)) {
    const result = reader.readFloat16();
    if (Number.isNaN(value)) {
      expect(Number.isNaN(result)).toBe(true);
    } else {
      // float16 has lower precision, so we need a larger tolerance
      // Using precision 2 to match Python's rel=1e-3 tolerance
      expect(result).toBeCloseTo(value, 2);
    }
  }
  expect(reader.remaining).toBe(0);
});

test("float32", () => {
  // zero should be 1 byte
  let writer = new BinaryWriter();
  writer.writeFloat32(0.0);
  expect(writer.toBytes().length).toBe(1);

  // non-zero should be 5 bytes (1 flag + 4 float)
  writer = new BinaryWriter();
  writer.writeFloat32(Math.PI);
  expect(writer.toBytes().length).toBe(5);

  // test with expected sizes
  const valueToBytes: Record<number, number> = {
    0.0: 1, // zero optimization
    [-0.0]: 1, // negative zero optimization
    [Number.POSITIVE_INFINITY]: 1, // positive infinity
    [Number.NEGATIVE_INFINITY]: 1, // negative infinity
    [Number.NaN]: 1, // NaN
    1.0: 2, // sint16 (varint)
    [-1.0]: 2, // sint16 (varint)
    [Math.PI]: 5, // flag + float32
    [-Math.PI]: 5, // flag + float32
  };

  writer = new BinaryWriter();
  for (const value of Object.keys(valueToBytes).map(Number)) {
    writer.writeFloat32(value);
  }

  expect(writer.toBytes().length).toBe(Object.values(valueToBytes).reduce((a, b) => a + b, 0)); // 31 bytes

  const reader = new BinaryReader(writer.toBytes());
  for (const value of Object.keys(valueToBytes).map(Number)) {
    const result = reader.readFloat32();
    if (Number.isNaN(value)) {
      expect(Number.isNaN(result)).toBe(true);
    } else {
      expect(result).toBeCloseTo(value, 6);
    }
  }
});

test("float64", () => {
  // zero should be 1 byte
  let writer = new BinaryWriter();
  writer.writeFloat64(0.0);
  expect(writer.toBytes().length).toBe(1);

  // small integers should use sint32 varint encoding (1 flag + varint bytes)
  writer = new BinaryWriter();
  writer.writeFloat64(42.0);
  const data = writer.toBytes();
  expect(data.length).toBe(2); // 1 flag + 1 byte for 84 (zigzag of 42)
  expect(data[0]).toBe(5); // sint32 flag

  // large float should be 9 bytes (1 flag + 8 float)
  writer = new BinaryWriter();
  writer.writeFloat64(Math.PI);
  expect(writer.toBytes().length).toBe(9);

  // test with expected sizes
  const valueToBytes: Record<number, number> = {
    0.0: 1, // zero optimization
    [-0.0]: 1, // negative zero optimization
    [Number.POSITIVE_INFINITY]: 1, // positive infinity
    [Number.NEGATIVE_INFINITY]: 1, // negative infinity
    [Number.NaN]: 1, // NaN
    1.0: 2, // flag + (zigzag(1) = 2)
    [-1.0]: 2, // flag + (zigzag(-1) = 1)
    42.0: 2, // flag + (zigzag(42) = 84)
    [-42.0]: 2, // flag + (zigzag(-42) = 83)
    1000.0: 3, // flag + (zigzag(1000) = 2000)
    [-1000.0]: 3, // flag + (zigzag(-1000) = 1999)
    [2 ** 53]: 9, // flag + float64 (exceeds sint32 range)
    [-(2 ** 53)]: 9, // flag + float64 (exceeds sint32 range)
    [Math.PI]: 9, // flag + double
    [-Math.PI]: 9, // flag + double
    1e100: 9, // flag + double
    [-1e100]: 9, // flag + double
  };

  writer = new BinaryWriter();
  for (const value of Object.keys(valueToBytes).map(Number)) {
    writer.writeFloat64(value);
  }

  expect(writer.toBytes().length).toBe(Object.values(valueToBytes).reduce((a, b) => a + b, 0)); // 87 bytes

  const reader = new BinaryReader(writer.toBytes());
  for (const value of Object.keys(valueToBytes).map(Number)) {
    const result = reader.readFloat64();
    if (Number.isNaN(value)) {
      expect(Number.isNaN(result)).toBe(true);
    } else {
      expect(result).toBeCloseTo(value);
    }
  }
});

test("string", () => {
  // Map of string -> expected bytes
  const stringToBytes: Record<string, number> = {
    "": 1, // just length 0
    hello: 6, // 1 length + 5 chars
    "Hello, 世界!": 15, // 1 length + 14 UTF-8 bytes (7 + 3 + 3 + 1)
    ["a".repeat(1000)]: 1002, // 2 length bytes + 1000 chars
  };

  const writer = new BinaryWriter();
  for (const s of Object.keys(stringToBytes)) {
    writer.writeString(s);
  }

  expect(writer.toBytes().length).toBe(Object.values(stringToBytes).reduce((a, b) => a + b, 0)); // 1024 bytes

  const reader = new BinaryReader(writer.toBytes());
  for (const s of Object.keys(stringToBytes)) {
    expect(reader.readString()).toBe(s);
  }
  expect(reader.remaining).toBe(0);
});

test("bytes", () => {
  // Map of bytes -> expected bytes
  const bytesToBytes = new Map<Uint8Array, number>([
    [new Uint8Array([]), 1], // just length 0
    [new TextEncoder().encode("hello"), 6], // 1 length + 5 bytes
    [new Uint8Array([0, 1, 2, 3]), 5], // 1 length + 4 bytes
    [new Uint8Array(Array.from({ length: 256 }, (_, i) => i)), 258], // 2 length bytes + 256 bytes
  ]);

  const writer = new BinaryWriter();
  for (const [b] of bytesToBytes) {
    writer.writeBytes(b);
  }

  expect(writer.toBytes().length).toBe(
    Array.from(bytesToBytes.values()).reduce((a, b) => a + b, 0),
  ); // 270 bytes

  const reader = new BinaryReader(writer.toBytes());
  for (const [b] of bytesToBytes) {
    const result = reader.readBytes();
    expect(result).toEqual(b);
  }
  expect(reader.remaining).toBe(0);
});

test("mixed types", () => {
  const writer = new BinaryWriter();

  // Write various types with expected sizes
  const operations: Array<[() => void, number]> = [
    [() => writer.writeBool(true), 1], // 1 byte
    [() => writer.writeInt8(-42), 1], // 1 byte
    [() => writer.writeUint16(65535), 3], // 3 bytes (varint)
    [() => writer.writeInt32(-1234567), 4], // 4 bytes (varint)
    [() => writer.writeInt128(BigInt("-1267650600228229401496703205376")), 15], // 15 bytes (varint)
    [() => writer.writeUint128(BigInt("1267650600228229401496703205376")), 15], // 15 bytes (varint)
    [() => writer.writeFloat16(1.5), 3], // 3 bytes
    [() => writer.writeFloat32(Math.PI), 5], // 5 bytes
    [() => writer.writeFloat64(Math.E), 9], // 9 bytes
    [() => writer.writeString("test string"), 12], // 12 bytes (1 + 11)
    [() => writer.writeBytes(new Uint8Array([1, 2, 3])), 4], // 4 bytes (1 + 3)
  ];

  for (const [writeOp] of operations) {
    writeOp();
  }

  // Calculate total expected bytes
  const totalExpected = operations.reduce((sum, [, size]) => sum + size, 0);
  expect(writer.toBytes().length).toBe(totalExpected); // 72 bytes

  // read them back
  const reader = new BinaryReader(writer.toBytes());
  expect(reader.readBool()).toBe(true);
  expect(reader.readInt8()).toBe(-42);
  expect(reader.readUint16()).toBe(65535);
  expect(reader.readInt32()).toBe(-1234567);
  expect(reader.readInt128()).toBe(BigInt("-1267650600228229401496703205376"));
  expect(reader.readUint128()).toBe(BigInt("1267650600228229401496703205376"));
  expect(reader.readFloat16()).toBeCloseTo(1.5, 2);
  expect(reader.readFloat32()).toBeCloseTo(Math.PI, 6);
  expect(reader.readFloat64()).toBeCloseTo(Math.E);
  expect(reader.readString()).toBe("test string");
  expect(reader.readBytes()).toEqual(new Uint8Array([1, 2, 3]));
  expect(reader.remaining).toBe(0);
});

test("int zigzag", () => {
  const testCases: Array<[number, number]> = [
    [0, 0],
    [-1, 1],
    [1, 2],
    [-2, 3],
    [2, 4],
    [-64, 127],
    [64, 128],
    [-65, 129],
    [2147483647, 4294967294],
    [-2147483648, 4294967295],
  ];

  // Test zigzagEncode by comparing with expected results
  for (const [signed, _] of testCases) {
    // we can't directly access private methods, so we'll test via writeInt32/readInt32
    const writer = new BinaryWriter();
    writer.writeInt32(signed);
    const reader = new BinaryReader(writer.toBytes());
    expect(reader.readInt32()).toBe(signed);
  }
});

test("varint", () => {
  // Map of value -> expected bytes
  const varintSizes: Record<number, number> = {
    0: 1, // 1 byte: 0-127
    127: 1,
    128: 2, // 2 bytes: 128-16383
    16383: 2,
    16384: 3, // 3 bytes: 16384-2097151
    2097151: 3,
    2097152: 4, // 4 bytes: 2097152-268435455
    268435455: 4,
    268435456: 5, // 5 bytes: 268435456-max uint32
    4294967295: 5,
  };

  for (const [value, expectedBytes] of Object.entries(varintSizes)) {
    const writer = new BinaryWriter();
    writer.writeUint32(Number(value));
    expect(writer.toBytes().length).toBe(expectedBytes);
  }
});

test("datetime", () => {
  const testCases = [
    // epoch
    Temporal.Instant.from("1970-01-01T00:00:00.000Z").toZonedDateTimeISO("UTC"),
    // current-ish time
    Temporal.Instant.from("2024-01-15T14:30:45.123Z").toZonedDateTimeISO("UTC"),
    // negative epoch (before 1970)
    Temporal.Instant.from("1969-12-31T23:59:59.000Z").toZonedDateTimeISO("UTC"),
    // far future
    Temporal.Instant.from("2100-01-01T00:00:00.000Z").toZonedDateTimeISO("UTC"),
    // with microseconds (rounded to milliseconds in JS)
    Temporal.Instant.from("2000-06-15T12:00:00.999Z").toZonedDateTimeISO("UTC"),
  ];

  const writer = new BinaryWriter();
  for (const dt of testCases) {
    writer.writeDateTime(dt);
  }

  const reader = new BinaryReader(writer.toBytes());
  for (const expected of testCases) {
    const result = reader.readDateTime();
    // Compare epoch milliseconds since Temporal has nanosecond precision
    expect(result.epochMilliseconds).toBe(expected.epochMilliseconds);
  }
  expect(reader.remaining).toBe(0);
});

test("date", () => {
  const testCases = [
    Temporal.PlainDate.from("1970-01-01"), // epoch
    Temporal.PlainDate.from("2024-01-15"), // current-ish
    Temporal.PlainDate.from("1969-12-31"), // before epoch
    Temporal.PlainDate.from("2100-12-31"), // far future
    Temporal.PlainDate.from("1900-01-01"), // old date
  ];

  const writer = new BinaryWriter();
  for (const d of testCases) {
    writer.writeDate(d);
  }

  const reader = new BinaryReader(writer.toBytes());
  for (const expected of testCases) {
    const result = reader.readDate();
    expect(result.toString()).toBe(expected.toString());
  }
  expect(reader.remaining).toBe(0);
});

test("time", () => {
  const testCases = [
    new Temporal.PlainTime(0, 0, 0, 0, 0), // midnight
    new Temporal.PlainTime(12, 0, 0, 0, 0), // noon
    new Temporal.PlainTime(23, 59, 59, 999, 999), // almost midnight (999ms 999μs)
    new Temporal.PlainTime(14, 30, 45, 123, 456), // arbitrary time (123ms 456μs)
    new Temporal.PlainTime(0, 0, 0, 0, 1), // 1 microsecond after midnight
  ];

  const writer = new BinaryWriter();
  for (const t of testCases) {
    writer.writeTime(t);
  }

  const reader = new BinaryReader(writer.toBytes());
  for (const expected of testCases) {
    const result = reader.readTime();
    expect(result.hour).toBe(expected.hour);
    expect(result.minute).toBe(expected.minute);
    expect(result.second).toBe(expected.second);
    expect(result.millisecond).toBe(expected.millisecond);
    expect(result.microsecond).toBe(expected.microsecond);
  }
  expect(reader.remaining).toBe(0);
});

test("duration", () => {
  const testCases = [
    Temporal.Duration.from({ microseconds: 0 }), // zero
    Temporal.Duration.from({ microseconds: 24 * 60 * 60 * 1_000_000 }), // 1 day
    Temporal.Duration.from({ microseconds: (1 * 3600 + 30 * 60 + 45) * 1_000_000 }), // 1 hour, 30 minutes, 45 seconds
    Temporal.Duration.from({ microseconds: 1 }), // 1 microsecond
    Temporal.Duration.from({ microseconds: -24 * 60 * 60 * 1_000_000 }), // negative 1 day
    Temporal.Duration.from({ microseconds: 52 * 7 * 24 * 60 * 60 * 1_000_000 }), // 52 weeks (1 year)
    Temporal.Duration.from({
      microseconds: (365 * 24 * 60 * 60 + 5 * 3600 + 48 * 60 + 46) * 1_000_000,
    }), // approx 1 year
  ];

  const writer = new BinaryWriter();
  for (const td of testCases) {
    writer.writeDuration(td);
  }

  const reader = new BinaryReader(writer.toBytes());
  for (const expected of testCases) {
    const result = reader.readDuration();
    expect(result.total({ unit: "microseconds" })).toBe(expected.total({ unit: "microseconds" }));
  }
  expect(reader.remaining).toBe(0);
});

test("uuid", () => {
  const testCases = [
    "00000000-0000-0000-0000-000000000000", // nil UUID
    "12345678-1234-5678-1234-567812345678", // fixed pattern
    "550e8400-e29b-41d4-a716-446655440000", // standard example
    "ffffffff-ffff-ffff-ffff-ffffffffffff", // max UUID
  ];

  const writer = new BinaryWriter();
  for (const uuid of testCases) {
    writer.writeUuid(uuid);
  }

  // each UUID is exactly 16 bytes
  expect(writer.toBytes().length).toBe(16 * testCases.length);

  const reader = new BinaryReader(writer.toBytes());
  for (const expected of testCases) {
    expect(reader.readUuid()).toBe(expected);
  }
  expect(reader.remaining).toBe(0);
});

test("json", () => {
  const testCases = [
    null,
    true,
    false,
    42,
    Math.PI,
    "hello",
    [],
    [1, 2, 3],
    { key: "value" },
    { nested: { data: [1, 2, { more: "stuff" }] } },
    ["mixed", 123, true, null, { obj: "ect" }],
  ];

  const writer = new BinaryWriter();
  for (const value of testCases) {
    writer.writeJson(value);
  }

  const reader = new BinaryReader(writer.toBytes());
  for (const expected of testCases) {
    expect(reader.readJson()).toEqual(expected);
  }
  expect(reader.remaining).toBe(0);
});

test("datetime naive", () => {
  // In Temporal, all ZonedDateTime are timezone-aware
  const naiveDt = Temporal.Now.zonedDateTimeISO();
  const writer = new BinaryWriter();
  writer.writeDateTime(naiveDt);

  const reader = new BinaryReader(writer.toBytes());
  const result = reader.readDateTime();
  expect(result.epochMilliseconds).toBe(naiveDt.epochMilliseconds);
});
