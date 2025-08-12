import struct
from datetime import UTC, datetime, time, timedelta
from typing import TYPE_CHECKING, Any, override

from ...builtin import (
    UUID,
    Bytes,
    Character,
    Date,
    DateTime,
    Duration,
    Float16,
    Float32,
    Float64,
    Int8,
    Int16,
    Int32,
    Int64,
    Int128,
    String,
    Time,
    Timestamp,
    UInt8,
    UInt16,
    UInt32,
    UInt64,
    UInt128,
)
from ..binary import BinaryReader, BinaryWriter

if TYPE_CHECKING:
    pass


class BinaryError(ValueError):
    """Base class for binary encoding/decoding errors."""


class KompaktBinaryWriter(BinaryWriter):
    """Write binary primitive values using KOMPAKT encoding."""

    @override
    def to_bytes(self) -> Bytes:
        return bytes(self.buffer)

    @override
    def write_bool(self, value: bool) -> None:
        """
        Write a boolean as 1 byte (0 for false, 1 for true).
        Size: 1 byte.
        """
        self.buffer.append(1 if value else 0)

    @override
    def write_int8(self, value: Int8) -> None:
        """
        Write a signed 8-bit integer as raw byte.
        Range: -2^7 to 2^7-1.
        Size: 1 byte.
        """
        self.buffer.append(value & 0xFF)

    @override
    def write_int16(self, value: Int16) -> None:
        """
        Write a signed 16-bit integer using zigzag encoding then varint.
        Range: -2^15 to 2^15-1.
        Size: 1-3 bytes.
        """
        self._write_varint(self._write_zigzag(value))

    @override
    def write_int32(self, value: Int32) -> None:
        """
        Write a signed 32-bit integer using zigzag encoding then varint.
        Range: -2^31 to 2^31-1.
        Size: 1-5 bytes.
        """
        self._write_varint(self._write_zigzag(value))

    @override
    def write_int64(self, value: Int64) -> None:
        """
        Write a signed 64-bit integer using zigzag encoding then varint.
        Range: -2^63 to 2^63-1.
        Size: 1-10 bytes.
        """
        self._write_varint(self._write_zigzag(value))

    @override
    def write_int128(self, value: Int128) -> None:
        """
        Write a signed 128-bit integer using zigzag encoding then varint.
        Range: -2^127 to 2^127-1.
        Size: 1-19 bytes.
        """
        self._write_varint(self._write_zigzag(value))

    @override
    def write_uint8(self, value: UInt8) -> None:
        """
        Write an unsigned 8-bit integer as raw byte.
        Range: 0 to 2^8-1.
        Size: 1 byte.
        """
        self.buffer.append(value & 0xFF)

    @override
    def write_uint16(self, value: UInt16) -> None:
        """
        Write an unsigned 16-bit integer using varint encoding.
        Range: 0 to 2^16-1.
        Size: 1-3 bytes.
        """
        self._write_varint(value)

    @override
    def write_uint32(self, value: UInt32) -> None:
        """
        Write an unsigned 32-bit integer using varint encoding.
        Range: 0 to 2^32-1.
        Size: 1-5 bytes.
        """
        self._write_varint(value)

    @override
    def write_uint64(self, value: UInt64) -> None:
        """
        Write an unsigned 64-bit integer using varint encoding.
        Range: 0 to 2^64-1.
        Size: 1-10 bytes.
        """
        self._write_varint(value)

    @override
    def write_uint128(self, value: UInt128) -> None:
        """
        Write an unsigned 128-bit integer using varint encoding.
        Range: 0 to 2^128-1.
        Size: 1-19 bytes.
        """
        self._write_varint(value)

    @override
    def write_float16(self, value: Float16) -> None:
        """
        Write a fixed-length 16-bit float.
        Range: ±6.55e4 (half precision IEEE 754).
        Size: 2 bytes.
        """
        self.buffer.extend(struct.pack("<e", value))

    @override
    def write_float32(self, value: Float32) -> None:
        """
        Write a fixed-length 32-bit float.
        Range: ±3.4e38 (single precision IEEE 754).
        Size: 4 bytes.
        """
        self.buffer.extend(struct.pack("<f", value))

    @override
    def write_float64(self, value: Float64) -> None:
        """
        Write a fixed-length 64-bit float.
        Range: ±1.8e308 (double precision IEEE 754).
        Size: 8 bytes.
        """
        self.buffer.extend(struct.pack("<d", value))

    @override
    def write_datetime(self, value: DateTime) -> None:
        """
        Write a datetime as zigzag-encoded varint of signed 64-bit microseconds since Unix epoch (UTC).
        Range: approximately ±292,277 years.
        Size: 1-10 bytes.
        """
        # ensure UTC timezone
        if value.tzinfo is None:
            value = value.replace(tzinfo=UTC)
        epoch = datetime(1970, 1, 1, tzinfo=UTC)
        days = (value.date() - epoch.date()).days
        time_micros = (
            value.hour * 3_600_000_000
            + value.minute * 60_000_000
            + value.second * 1_000_000
            + value.microsecond
        )
        micros = days * 86_400_000_000 + time_micros
        self._write_varint(self._write_zigzag(micros))

    @override
    def write_date(self, value: Date) -> None:
        """
        Write a date as zigzag-encoded varint of signed 64-bit days since Unix epoch (1970-01-01).
        Range: full Python date range supported by calculations.
        Size: 1-10 bytes.
        """
        epoch = datetime(1970, 1, 1, tzinfo=UTC)
        days = (value - epoch.date()).days
        self._write_varint(self._write_zigzag(days))

    @override
    def write_time(self, value: Time) -> None:
        """
        Write a time as varint of unsigned 64-bit nanoseconds since midnight.
        Range: 00:00:00.000000000 to 23:59:59.999999999.
        Size: 1-10 bytes.
        """
        nanos = (
            value.hour * 3_600_000_000_000
            + value.minute * 60_000_000_000
            + value.second * 1_000_000_000
            + value.microsecond * 1_000
        )
        self._write_varint(nanos)

    @override
    def write_timestamp(self, value: Timestamp) -> None:
        """
        Write a timestamp as varint of unsigned 64-bit nanoseconds since Unix epoch (UTC).
        Range: 1970-01-01 to 2554-07-21 UTC.
        Size: 1-10 bytes.
        """
        self._write_varint(value)

    @override
    def write_duration(self, value: Duration) -> None:
        """
        Write a duration as zigzag-encoded varint of signed 64-bit nanoseconds.
        Range: approximately ±292,277 years.
        Size: 1-10 bytes.
        """
        # use integer math to avoid precision issues with large durations
        micros = value.days * 86_400_000_000 + value.seconds * 1_000_000 + value.microseconds
        nanos = micros * 1_000
        self._write_varint(self._write_zigzag(nanos))

    @override
    def write_string(self, value: String) -> None:
        """
        Write a UTF-8 string with varint length prefix followed by UTF-8 bytes.
        Range: 0 to 2^32-1 bytes (UTF-8 encoded).
        Size: 1-5 bytes (length) + string length in bytes.
        """
        encoded = value.encode("utf-8")
        self.write_bytes(encoded)

    @override
    def write_character(self, value: Character) -> None:
        """
        Write a single Unicode code point as fixed 4 bytes (UTF-32 LE).
        Size: 4 bytes.
        """
        if len(value) != 1:
            raise BinaryError(f"character must be length 1, got {len(value)}: {value!r}")
        self.buffer.extend(struct.pack("<I", ord(value)))

    @override
    def write_uuid(self, value: UUID) -> None:
        """
        Write a UUID as 16 raw bytes.
        Size: 16 bytes.
        """
        self.buffer.extend(value.bytes)

    @override
    def write_bytes(self, value: Bytes) -> None:
        """
        Write raw bytes with varint length prefix followed by the bytes.
        Range: 0 to 2^32-1 bytes.
        Size: 1-5 bytes (length) + data length.
        """
        self._write_varint(len(value))
        self.buffer.extend(value)

    @override
    def write_json(self, value: Any) -> None:
        """
        Write JSON as compact binary format.
        - 0: null
        - 1: false
        - 2: true
        - 3: int64 (zigzag varint)
        - 4: float64
        - 5: string (length-prefixed UTF-8)
        - 6: array (length + elements)
        - 7: object (length + key-value pairs)
        """
        if value is None:
            self.buffer.append(0)
        elif value is False:
            self.buffer.append(1)
        elif value is True:
            self.buffer.append(2)
        elif isinstance(value, int):
            # encode as int64
            self.buffer.append(3)
            self.write_int64(value)
        elif isinstance(value, float):
            # encode as float64 (handles zero, infinity, NaN, etc.)
            self.buffer.append(4)
            self.write_float64(value)
        elif isinstance(value, str):
            self.buffer.append(5)
            self.write_string(value)
        elif isinstance(value, list):
            self.buffer.append(6)
            self._write_varint(len(value))
            for item in value:
                self.write_json(item)
        elif isinstance(value, dict):
            self.buffer.append(7)
            self._write_varint(len(value))
            for key, val in value.items():
                if not isinstance(key, str):
                    raise BinaryError(f"invalid JSON key type '{type(key)}': {key!r}")
                self.write_string(key)
                self.write_json(val)
        else:
            raise BinaryError(f"invalid JSON type '{type(value)}': {value!r}")

    def _write_varint(self, value: int) -> None:
        """Write an unsigned integer using variable-length encoding."""
        while value >= 0x80:
            self.buffer.append((value & 0x7F) | 0x80)
            value >>= 7
        self.buffer.append(value & 0x7F)

    def _write_zigzag(self, value: int) -> int:
        """Encode signed integer to unsigned using zigzag encoding."""
        if value >= 0:
            return value << 1
        else:
            return ((-value) << 1) - 1


class KompaktBinaryReader(BinaryReader):
    """Read binary primitive values using KOMPAKT encoding."""

    @override
    def read_bool(self) -> bool:
        """
        Read a boolean from 1 byte (0 for false, non-zero for true).
        Size: 1 byte.
        """
        if self.pos >= len(self.buffer):
            raise BinaryError(f"unexpected end of buffer at {self.pos}")
        value = self.buffer[self.pos]
        self.pos += 1
        return value != 0

    @override
    def read_int8(self) -> Int8:
        """
        Read a signed 8-bit integer from raw byte with sign extension.
        Range: -2^7 to 2^7-1.
        Size: 1 byte.
        """
        if self.pos >= len(self.buffer):
            raise BinaryError(f"unexpected end of buffer at {self.pos}")
        value = self.buffer[self.pos]
        self.pos += 1
        # convert to signed
        if value >= 0x80:
            return value - 0x100
        return value

    @override
    def read_int16(self) -> Int16:
        """
        Read a signed 16-bit integer from zigzag-decoded varint.
        Range: -2^15 to 2^15-1.
        Size: 1-3 bytes.
        """
        return self._zigzag_decode(self._read_varint())

    @override
    def read_int32(self) -> Int32:
        """
        Read a signed 32-bit integer from zigzag-decoded varint.
        Range: -2^31 to 2^31-1.
        Size: 1-5 bytes.
        """
        return self._zigzag_decode(self._read_varint())

    @override
    def read_int64(self) -> Int64:
        """
        Read a signed 64-bit integer from zigzag-decoded varint.
        Range: -2^63 to 2^63-1.
        Size: 1-10 bytes.
        """
        return self._zigzag_decode(self._read_varint())

    @override
    def read_int128(self) -> Int128:
        """
        Read a signed 128-bit integer from zigzag-decoded varint.
        Range: -2^127 to 2^127-1.
        Size: 1-19 bytes.
        """
        return self._zigzag_decode(self._read_varint())

    @override
    def read_uint8(self) -> UInt8:
        """
        Read an unsigned 8-bit integer from raw byte.
        Range: 0 to 2^8-1.
        Size: 1 byte.
        """
        if self.pos >= len(self.buffer):
            raise BinaryError(f"unexpected end of buffer at {self.pos}")
        value = self.buffer[self.pos]
        self.pos += 1
        return value

    @override
    def read_uint16(self) -> UInt16:
        """
        Read an unsigned 16-bit integer from varint.
        Range: 0 to 2^16-1.
        Size: 1-3 bytes.
        """
        return self._read_varint()

    @override
    def read_uint32(self) -> UInt32:
        """
        Read an unsigned 32-bit integer from varint.
        Range: 0 to 2^32-1.
        Size: 1-5 bytes.
        """
        return self._read_varint()

    @override
    def read_uint64(self) -> UInt64:
        """
        Read an unsigned 64-bit integer from varint.
        Range: 0 to 2^64-1.
        Size: 1-10 bytes.
        """
        return self._read_varint()

    @override
    def read_uint128(self) -> UInt128:
        """
        Read an unsigned 128-bit integer from varint.
        Range: 0 to 2^128-1.
        Size: 1-19 bytes.
        """
        return self._read_varint()

    @override
    def read_float16(self) -> Float16:
        """
        Read a fixed-length 16-bit float.
        Range: ±6.55e4 (half precision IEEE 754).
        Size: 2 bytes.
        """
        if self.pos >= len(self.buffer):
            raise BinaryError(f"unexpected end of buffer at {self.pos}")

        if self.pos + 2 > len(self.buffer):
            raise BinaryError(f"unexpected end of buffer at {self.pos}")
        value = struct.unpack("<e", self.buffer[self.pos : self.pos + 2])[0]
        self.pos += 2
        return value

    @override
    def read_float32(self) -> Float32:
        """
        Read a fixed-length 32-bit float.
        Range: ±3.4e38 (single precision IEEE 754).
        Size: 4 bytes.
        """
        if self.pos >= len(self.buffer):
            raise BinaryError(f"unexpected end of buffer at {self.pos}")
        if self.pos + 4 > len(self.buffer):
            raise BinaryError(f"unexpected end of buffer at {self.pos}")
        value = struct.unpack("<f", self.buffer[self.pos : self.pos + 4])[0]
        self.pos += 4
        return value

    @override
    def read_float64(self) -> Float64:
        """
        Read a fixed-length 64-bit float.
        Range: ±1.8e308 (double precision IEEE 754).
        Size: 8 bytes.
        """
        if self.pos >= len(self.buffer):
            raise BinaryError(f"unexpected end of buffer at {self.pos}")
        if self.pos + 8 > len(self.buffer):
            raise BinaryError(f"unexpected end of buffer at {self.pos}")
        value = struct.unpack("<d", self.buffer[self.pos : self.pos + 8])[0]
        self.pos += 8
        return value

    @override
    def read_datetime(self) -> DateTime:
        """
        Read a datetime from zigzag-decoded varint of signed 64-bit microseconds since Unix epoch (UTC).
        Range: approximately ±292,277 years.
        Size: 1-10 bytes.
        """
        micros = self._zigzag_decode(self._read_varint())
        # calculate datetime manually to support full date range
        days = micros // 86_400_000_000
        time_micros = micros % 86_400_000_000

        epoch = datetime(1970, 1, 1, tzinfo=UTC)
        date_part = epoch.date() + timedelta(days=days)
        hours = time_micros // 3_600_000_000
        time_micros %= 3_600_000_000
        minutes = time_micros // 60_000_000
        time_micros %= 60_000_000
        seconds = time_micros // 1_000_000
        microseconds = time_micros % 1_000_000

        return datetime(
            date_part.year,
            date_part.month,
            date_part.day,
            hours,
            minutes,
            seconds,
            microseconds,
            tzinfo=UTC,
        )

    @override
    def read_date(self) -> Date:
        """
        Read a date from zigzag-decoded varint of signed 64-bit days since Unix epoch (1970-01-01).
        Size: 1-10 bytes.
        """
        days = self._zigzag_decode(self._read_varint())
        epoch = datetime(1970, 1, 1, tzinfo=UTC)
        return epoch.date() + timedelta(days=days)

    @override
    def read_time(self) -> Time:
        """
        Read a time from varint of unsigned 64-bit nanoseconds since midnight.
        Range: 00:00:00.000000000 to 23:59:59.999999999.
        Size: 1-10 bytes.
        """
        nanos = self._read_varint()
        hours = nanos // 3_600_000_000_000
        nanos %= 3_600_000_000_000
        minutes = nanos // 60_000_000_000
        nanos %= 60_000_000_000
        seconds = nanos // 1_000_000_000
        microseconds = (nanos % 1_000_000_000) // 1_000
        return time(hours, minutes, seconds, microseconds)

    @override
    def read_timestamp(self) -> Timestamp:
        """
        Read a timestamp from varint of unsigned 64-bit nanoseconds since Unix epoch (UTC).
        Range: 1970-01-01 to 2554-07-21 UTC.
        Size: 1-10 bytes.
        """
        return self._read_varint()

    @override
    def read_duration(self) -> Duration:
        """
        Read a duration from zigzag-decoded varint of signed 64-bit nanoseconds.
        Range: approximately ±292,277 years.
        Size: 1-10 bytes.
        """
        nanos = self._zigzag_decode(self._read_varint())
        micros = nanos // 1_000
        return timedelta(microseconds=micros)

    @override
    def read_string(self) -> String:
        """
        Read a UTF-8 string from varint length prefix + UTF-8 bytes.
        Range: 0 to 2^32-1 bytes (UTF-8 encoded).
        Size: 1-5 bytes (length) + string length in bytes.
        """
        return self.read_bytes().decode("utf-8")

    @override
    def read_character(self) -> Character:
        """
        Read a single Unicode code point encoded as fixed 4 bytes (UTF-32 LE).
        Size: 4 bytes.
        """
        if self.pos + 4 > len(self.buffer):
            raise BinaryError(f"unexpected end of buffer at {self.pos}")
        codepoint = struct.unpack("<I", self.buffer[self.pos : self.pos + 4])[0]
        self.pos += 4
        return chr(codepoint)

    @override
    def read_uuid(self) -> UUID:
        """
        Read a UUID from 16 raw bytes.
        Size: 16 bytes.
        """
        if self.pos + 16 > len(self.buffer):
            raise BinaryError(f"unexpected end of buffer at {self.pos}")
        uuid_bytes = self.buffer[self.pos : self.pos + 16]
        self.pos += 16
        return UUID(bytes=uuid_bytes)

    @override
    def read_bytes(self) -> Bytes:
        """
        Read raw bytes from varint length prefix + data bytes.
        Range: 0 to 2^32-1 bytes.
        Size: 1-5 bytes (length) + data length.
        """
        length = self._read_varint()
        if self.pos + length > len(self.buffer):
            raise BinaryError(f"unexpected end of buffer at {self.pos}")
        value = self.buffer[self.pos : self.pos + length]
        self.pos += length
        return value

    @override
    def read_json(self) -> Any:
        """
        Read JSON from compact binary format.
        - 0: null
        - 1: false
        - 2: true
        - 3: int64 (zigzag varint)
        - 4: float64
        - 5: string (length-prefixed UTF-8)
        - 6: array (length + elements)
        - 7: object (length + key-value pairs)
        """
        if self.pos >= len(self.buffer):
            raise BinaryError(f"unexpected end of buffer at {self.pos}")

        tag = self.buffer[self.pos]
        self.pos += 1

        if tag == 0:
            return None
        elif tag == 1:
            return False
        elif tag == 2:
            return True
        elif tag == 3:
            return self.read_int64()
        elif tag == 4:
            return self.read_float64()
        elif tag == 5:
            return self.read_string()
        elif tag == 6:
            length = self._read_varint()
            return [self.read_json() for _ in range(length)]
        elif tag == 7:
            length = self._read_varint()
            result = {}
            for _ in range(length):
                key = self.read_string()
                value = self.read_json()
                result[key] = value
            return result
        else:
            raise BinaryError(f"invalid JSON type tag at {self.pos - 1}: {tag}")

    def _read_varint(self) -> int:
        """Read an unsigned integer using variable-length encoding."""
        value = 0
        shift = 0
        while True:
            if self.pos >= len(self.buffer):
                raise BinaryError(f"unexpected end of buffer at {self.pos}")
            byte = self.buffer[self.pos]
            self.pos += 1
            value |= (byte & 0x7F) << shift
            if (byte & 0x80) == 0:
                return value
            shift += 7
            if shift >= 140:
                raise BinaryError(f"varint too long at {self.pos}")

    def _zigzag_decode(self, value: int) -> int:
        """Decode unsigned integer to signed using zigzag decoding."""
        if value & 1:
            return -((value + 1) >> 1)
        else:
            return value >> 1
