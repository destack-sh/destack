import struct
from datetime import UTC, datetime, time, timedelta
from typing import TYPE_CHECKING, Any

from ..builtin import (
    UUID,
    Bytes,
    Date,
    Datetime,
    Duration,
    Float16,
    Float32,
    Float64,
    Handle,
    HandleType,
    Int8,
    Int16,
    Int32,
    Int64,
    Int128,
    String,
    Time,
    UInt8,
    UInt16,
    UInt32,
    UInt64,
    UInt128,
    declare_handle,
    declare_method,
    declare_property_runtime,
)
from ..universe import Universe

if TYPE_CHECKING:
    pass


class BinaryError(ValueError):
    """Base class for binary encoding/decoding errors."""


@declare_handle(HandleType.BINARY_WRITER)
class BinaryWriter(Handle):
    """Write binary values in our custom encoding. Little-endian, varint, zigzag, etc."""

    buffer: bytearray = declare_property_runtime(401, default_factory=bytearray)

    def __len__(self) -> int:
        return len(self.buffer)

    @declare_method(80, is_implemented=True)
    def to_bytes(self) -> bytes:
        """Get the written bytes."""
        return bytes(self.buffer)

    # PrimitiveType.BOOLEAN
    @declare_method(102, is_implemented=True)
    def write_bool(self, value: bool) -> None:
        """
        Write a boolean as 1 byte (0 for false, 1 for true).
        Size: 1 byte.
        """
        self.buffer.append(1 if value else 0)

    # PrimitiveType.INT8
    @declare_method(102, is_implemented=True)
    def write_int8(self, value: Int8) -> None:
        """
        Write a signed 8-bit integer as raw byte.
        Range: -2^7 to 2^7-1.
        Size: 1 byte.
        """
        self.buffer.append(value & 0xFF)

    # PrimitiveType.INT16
    @declare_method(103, is_implemented=True)
    def write_int16(self, value: Int16) -> None:
        """
        Write a signed 16-bit integer using zigzag encoding then varint.
        Range: -2^15 to 2^15-1.
        Size: 1-3 bytes.
        """
        self._write_varint(self._write_zigzag(value))

    # PrimitiveType.INT32
    @declare_method(104, is_implemented=True)
    def write_int32(self, value: Int32) -> None:
        """
        Write a signed 32-bit integer using zigzag encoding then varint.
        Range: -2^31 to 2^31-1.
        Size: 1-5 bytes.
        """
        self._write_varint(self._write_zigzag(value))

    # PrimitiveType.INT64
    @declare_method(105, is_implemented=True)
    def write_int64(self, value: Int64) -> None:
        """
        Write a signed 64-bit integer using zigzag encoding then varint.
        Range: -2^63 to 2^63-1.
        Size: 1-10 bytes.
        """
        self._write_varint(self._write_zigzag(value))

    # PrimitiveType.INT128
    @declare_method(107, is_implemented=True)
    def write_int128(self, value: Int128) -> None:
        """
        Write a signed 128-bit integer using zigzag encoding then varint.
        Range: -2^127 to 2^127-1.
        Size: 1-19 bytes.
        """
        self._write_varint(self._write_zigzag(value))

    # PrimitiveType.UINT8
    @declare_method(110, is_implemented=True)
    def write_uint8(self, value: UInt8) -> None:
        """
        Write an unsigned 8-bit integer as raw byte.
        Range: 0 to 2^8-1.
        Size: 1 byte.
        """
        self.buffer.append(value & 0xFF)

    # PrimitiveType.UINT16
    @declare_method(111, is_implemented=True)
    def write_uint16(self, value: UInt16) -> None:
        """
        Write an unsigned 16-bit integer using varint encoding.
        Range: 0 to 2^16-1.
        Size: 1-3 bytes.
        """
        self._write_varint(value)

    # PrimitiveType.UINT32
    @declare_method(112, is_implemented=True)
    def write_uint32(self, value: UInt32) -> None:
        """
        Write an unsigned 32-bit integer using varint encoding.
        Range: 0 to 2^32-1.
        Size: 1-5 bytes.
        """
        self._write_varint(value)

    # PrimitiveType.UINT64
    @declare_method(113, is_implemented=True)
    def write_uint64(self, value: UInt64) -> None:
        """
        Write an unsigned 64-bit integer using varint encoding.
        Range: 0 to 2^64-1.
        Size: 1-10 bytes.
        """
        self._write_varint(value)

    # PrimitiveType.UINT128
    @declare_method(114, is_implemented=True)
    def write_uint128(self, value: UInt128) -> None:
        """
        Write an unsigned 128-bit integer using varint encoding.
        Range: 0 to 2^128-1.
        Size: 1-19 bytes.
        """
        self._write_varint(value)

    # PrimitiveType.FLOAT16
    @declare_method(122, is_implemented=True)
    def write_float16(self, value: Float16) -> None:
        """
        Write a fixed-length 16-bit float.
        Range: ±6.55e4 (half precision IEEE 754).
        Size: 2 bytes.
        """
        self.buffer.extend(struct.pack("<e", value))

    # PrimitiveType.FLOAT32
    @declare_method(123, is_implemented=True)
    def write_float32(self, value: Float32) -> None:
        """
        Write a fixed-length 32-bit float.
        Range: ±3.4e38 (single precision IEEE 754).
        Size: 4 bytes.
        """
        self.buffer.extend(struct.pack("<f", value))

    # PrimitiveType.FLOAT64
    @declare_method(124, is_implemented=True)
    def write_float64(self, value: Float64) -> None:
        """
        Write a fixed-length 64-bit float.
        Range: ±1.8e308 (double precision IEEE 754).
        Size: 8 bytes.
        """
        self.buffer.extend(struct.pack("<d", value))

    # PrimitiveType.DATETIME
    @declare_method(140, is_implemented=True)
    def write_datetime(self, value: Datetime) -> None:
        """
        Write a datetime as zigzag-encoded varint of microseconds since epoch.
        Range: 0001-01-01 00:00:00 to 9999-12-31 23:59:59.999999 UTC.
        Size: 1-10 bytes.
        """
        # ensure UTC timezone
        if value.tzinfo is None:
            value = value.replace(tzinfo=UTC)
        # calculate microseconds manually to support full date range
        days = (value.date() - Universe.BEGINNING_OF_DATETIME.date()).days
        time_micros = (
            value.hour * 3_600_000_000
            + value.minute * 60_000_000
            + value.second * 1_000_000
            + value.microsecond
        )
        micros = days * 86_400_000_000 + time_micros
        self._write_varint(self._write_zigzag(micros))

    # PrimitiveType.DATE
    @declare_method(141, is_implemented=True)
    def write_date(self, value: Date) -> None:
        """
        Write a date as zigzag-encoded varint of days since epoch (0001-01-01).
        Range: 0001-01-01 to 9999-12-31.
        Size: 1-5 bytes.
        """
        days = (value - Universe.BEGINNING_OF_DATETIME.date()).days
        self._write_varint(self._write_zigzag(days))

    # PrimitiveType.TIME
    @declare_method(142, is_implemented=True)
    def write_time(self, value: Time) -> None:
        """
        Write a time as varint of microseconds since midnight.
        Range: 00:00:00 to 23:59:59.999999.
        Size: 1-6 bytes.
        """
        micros = (
            value.hour * 3_600_000_000
            + value.minute * 60_000_000
            + value.second * 1_000_000
            + value.microsecond
        )
        self._write_varint(micros)

    # PrimitiveType.DURATION
    @declare_method(143, is_implemented=True)
    def write_duration(self, value: Duration) -> None:
        """
        Write a duration as zigzag-encoded varint of microseconds.
        Range: -999,999,999 days to 999,999,999 days.
        Size: 1-10 bytes.
        """
        # use integer math to avoid precision issues with large durations
        micros = value.days * 86_400_000_000 + value.seconds * 1_000_000 + value.microseconds
        self._write_varint(self._write_zigzag(micros))

    # PrimitiveType.STRING
    @declare_method(150, is_implemented=True)
    def write_string(self, value: String) -> None:
        """
        Write a UTF-8 string with varint length prefix followed by UTF-8 bytes.
        Range: 0 to 2^32-1 bytes (UTF-8 encoded).
        Size: 1-5 bytes (length) + string length in bytes.
        """
        encoded = value.encode("utf-8")
        self.write_bytes(encoded)

    # PrimitiveType.UUID
    @declare_method(151, is_implemented=True)
    def write_uuid(self, value: UUID) -> None:
        """
        Write a UUID as 16 raw bytes.
        Size: 16 bytes.
        """
        self.buffer.extend(value.bytes)

    # PrimitiveType.BYTES
    @declare_method(152, is_implemented=True)
    def write_bytes(self, value: Bytes) -> None:
        """
        Write raw bytes with varint length prefix followed by the bytes.
        Range: 0 to 2^32-1 bytes.
        Size: 1-5 bytes (length) + data length.
        """
        self._write_varint(len(value))
        self.buffer.extend(value)

    # PrimitiveType.JSON
    @declare_method(155, is_implemented=True)
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


@declare_handle(HandleType.BINARY_READER)
class BinaryReader(Handle):
    """Read binary values in our custom encoding. Little-endian, varint, zigzag, etc."""

    buffer: bytes = declare_property_runtime(401)
    pos: UInt32 = declare_property_runtime(402, default=0)

    def __str__(self) -> str:
        return f"pos={self.pos}, remaining={self.remaining}"

    def __repr__(self) -> str:
        return f"<BinaryReader pos={self.pos}, remaining={self.remaining}>"

    @property
    def remaining(self) -> int:
        """Number of bytes remaining."""
        return len(self.buffer) - self.pos

    # PrimitiveType.BOOLEAN
    @declare_method(102, is_implemented=True)
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

    # PrimitiveType.INT8
    @declare_method(103, is_implemented=True)
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

    # PrimitiveType.INT16
    @declare_method(104, is_implemented=True)
    def read_int16(self) -> Int16:
        """
        Read a signed 16-bit integer from zigzag-decoded varint.
        Range: -2^15 to 2^15-1.
        Size: 1-3 bytes.
        """
        return self._zigzag_decode(self._read_varint())

    # PrimitiveType.INT32
    @declare_method(105, is_implemented=True)
    def read_int32(self) -> Int32:
        """
        Read a signed 32-bit integer from zigzag-decoded varint.
        Range: -2^31 to 2^31-1.
        Size: 1-5 bytes.
        """
        return self._zigzag_decode(self._read_varint())

    # PrimitiveType.INT64
    @declare_method(106, is_implemented=True)
    def read_int64(self) -> Int64:
        """
        Read a signed 64-bit integer from zigzag-decoded varint.
        Range: -2^63 to 2^63-1.
        Size: 1-10 bytes.
        """
        return self._zigzag_decode(self._read_varint())

    # PrimitiveType.INT128
    @declare_method(107, is_implemented=True)
    def read_int128(self) -> Int128:
        """
        Read a signed 128-bit integer from zigzag-decoded varint.
        Range: -2^127 to 2^127-1.
        Size: 1-19 bytes.
        """
        return self._zigzag_decode(self._read_varint())

    # PrimitiveType.UINT8
    @declare_method(110, is_implemented=True)
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

    # PrimitiveType.UINT16
    @declare_method(111, is_implemented=True)
    def read_uint16(self) -> UInt16:
        """
        Read an unsigned 16-bit integer from varint.
        Range: 0 to 2^16-1.
        Size: 1-3 bytes.
        """
        return self._read_varint()

    # PrimitiveType.UINT32
    @declare_method(112, is_implemented=True)
    def read_uint32(self) -> UInt32:
        """
        Read an unsigned 32-bit integer from varint.
        Range: 0 to 2^32-1.
        Size: 1-5 bytes.
        """
        return self._read_varint()

    # PrimitiveType.UINT64
    @declare_method(113, is_implemented=True)
    def read_uint64(self) -> UInt64:
        """
        Read an unsigned 64-bit integer from varint.
        Range: 0 to 2^64-1.
        Size: 1-10 bytes.
        """
        return self._read_varint()

    # PrimitiveType.UINT128
    @declare_method(114, is_implemented=True)
    def read_uint128(self) -> UInt128:
        """
        Read an unsigned 128-bit integer from varint.
        Range: 0 to 2^128-1.
        Size: 1-19 bytes.
        """
        return self._read_varint()

    # PrimitiveType.FLOAT16
    @declare_method(122, is_implemented=True)
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

    # PrimitiveType.FLOAT32
    @declare_method(123, is_implemented=True)
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

    # PrimitiveType.FLOAT64
    @declare_method(124, is_implemented=True)
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

    # PrimitiveType.DATETIME
    @declare_method(140, is_implemented=True)
    def read_datetime(self) -> Datetime:
        """
        Read a datetime from zigzag-decoded varint of microseconds since epoch.
        Range: 0001-01-01 00:00:00 to 9999-12-31 23:59:59.999999 UTC.
        Size: 1-10 bytes.
        """
        micros = self._zigzag_decode(self._read_varint())
        # calculate datetime manually to support full date range
        days = micros // 86_400_000_000
        time_micros = micros % 86_400_000_000

        date_part = Universe.BEGINNING_OF_DATETIME.date() + timedelta(days=days)
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

    # PrimitiveType.DATE
    @declare_method(141, is_implemented=True)
    def read_date(self) -> Date:
        """
        Read a date from zigzag-decoded varint of days since epoch (0001-01-01).
        Range: 0001-01-01 to 9999-12-31.
        Size: 1-5 bytes.
        """
        days = self._zigzag_decode(self._read_varint())
        return Universe.BEGINNING_OF_DATETIME.date() + timedelta(days=days)

    # PrimitiveType.TIME
    @declare_method(142, is_implemented=True)
    def read_time(self) -> Time:
        """
        Read a time from varint of microseconds since midnight.
        Range: 00:00:00 to 23:59:59.999999.
        Size: 1-6 bytes.
        """
        micros = self._read_varint()
        hours = micros // 3_600_000_000
        micros %= 3_600_000_000
        minutes = micros // 60_000_000
        micros %= 60_000_000
        seconds = micros // 1_000_000
        microseconds = micros % 1_000_000
        return time(hours, minutes, seconds, microseconds)

    # PrimitiveType.DURATION
    @declare_method(143, is_implemented=True)
    def read_duration(self) -> Duration:
        """
        Read a duration from zigzag-decoded varint of microseconds.
        Range: -999,999,999 days to 999,999,999 days.
        Size: 1-10 bytes.
        """
        micros = self._zigzag_decode(self._read_varint())
        return timedelta(microseconds=micros)

    # PrimitiveType.STRING
    @declare_method(150, is_implemented=True)
    def read_string(self) -> String:
        """
        Read a UTF-8 string from varint length prefix + UTF-8 bytes.
        Range: 0 to 2^32-1 bytes (UTF-8 encoded).
        Size: 1-5 bytes (length) + string length in bytes.
        """
        return self.read_bytes().decode("utf-8")

    # PrimitiveType.UUID
    @declare_method(151, is_implemented=True)
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

    # PrimitiveType.BYTES
    @declare_method(152, is_implemented=True)
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

    # PrimitiveType.JSON
    @declare_method(155, is_implemented=True)
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
