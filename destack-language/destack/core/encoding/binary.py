import struct
from datetime import UTC, datetime, time, timedelta
from typing import TYPE_CHECKING, Any

from ..builtin import (
    UUID,
    Bytes,
    Character,
    Date,
    DateTime,
    Duration,
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

if TYPE_CHECKING:
    pass


class BinaryError(ValueError):
    """Base class for binary encoding/decoding errors."""


class BinaryEncoder:
    """Write binary primitive values in some encoding."""

    def __init__(self) -> None:
        self.buffer = bytearray()

    def __len__(self) -> int:
        return len(self.buffer)

    def to_bytes(self) -> bytes:
        """Get the written bytes."""
        return bytes(self.buffer)

    # PrimitiveType.BOOLEAN
    def write_bool(self, value: bool) -> None:
        """Write a boolean."""
        self.buffer.append(1 if value else 0)

    # PrimitiveType.INT8
    def write_int8(self, value: Int8) -> None:
        """Write a signed 8-bit integer."""
        self.buffer.extend(struct.pack("<b", value))

    # PrimitiveType.INT16
    def write_int16(self, value: Int16) -> None:
        """Write a signed 16-bit integer."""
        self.buffer.extend(struct.pack("<h", value))

    # PrimitiveType.INT32
    def write_int32(self, value: Int32) -> None:
        """Write a signed 32-bit integer."""
        self.buffer.extend(struct.pack("<i", value))

    # PrimitiveType.INT64
    def write_int64(self, value: Int64) -> None:
        """Write a signed 64-bit integer."""
        self.buffer.extend(struct.pack("<q", value))

    # PrimitiveType.INT128
    def write_int128(self, value: Int128) -> None:
        """Write a signed 128-bit integer."""
        # split into two 64-bit parts
        low = value & 0xFFFFFFFFFFFFFFFF
        high = (value >> 64) & 0xFFFFFFFFFFFFFFFF
        self.buffer.extend(struct.pack("<QQ", low, high))

    # PrimitiveType.UINT8
    def write_uint8(self, value: UInt8) -> None:
        """Write an unsigned 8-bit integer."""
        self.buffer.extend(struct.pack("<B", value))

    # PrimitiveType.UINT16
    def write_uint16(self, value: UInt16) -> None:
        """Write an unsigned 16-bit integer."""
        self.buffer.extend(struct.pack("<H", value))

    # PrimitiveType.UINT32
    def write_uint32(self, value: UInt32) -> None:
        """Write an unsigned 32-bit integer."""
        self.buffer.extend(struct.pack("<I", value))

    # PrimitiveType.UINT64
    def write_uint64(self, value: UInt64) -> None:
        """Write an unsigned 64-bit integer."""
        self.buffer.extend(struct.pack("<Q", value))

    # PrimitiveType.UINT128
    def write_uint128(self, value: UInt128) -> None:
        """Write an unsigned 128-bit integer."""
        # split into two 64-bit parts
        low = value & 0xFFFFFFFFFFFFFFFF
        high = (value >> 64) & 0xFFFFFFFFFFFFFFFF
        self.buffer.extend(struct.pack("<QQ", low, high))

    # PrimitiveType.FLOAT32
    def write_float32(self, value: Float32) -> None:
        """Write a 32-bit float."""
        self.buffer.extend(struct.pack("<f", value))

    # PrimitiveType.FLOAT64
    def write_float64(self, value: Float64) -> None:
        """Write a 64-bit float."""
        self.buffer.extend(struct.pack("<d", value))

    # PrimitiveType.DATETIME
    def write_datetime(self, value: DateTime) -> None:
        """Write a datetime."""
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
        self.buffer.extend(struct.pack("<q", micros))

    # PrimitiveType.DATE
    def write_date(self, value: Date) -> None:
        """Write a date."""
        epoch = datetime(1970, 1, 1, tzinfo=UTC)
        days = (value - epoch.date()).days
        self.buffer.extend(struct.pack("<q", days))

    # PrimitiveType.TIME
    def write_time(self, value: Time) -> None:
        """Write a time."""
        nanos = (
            value.hour * 3_600_000_000_000
            + value.minute * 60_000_000_000
            + value.second * 1_000_000_000
            + value.microsecond * 1_000
        )
        self.buffer.extend(struct.pack("<Q", nanos))

    # PrimitiveType.TIMESTAMP
    def write_timestamp(self, value: Timestamp) -> None:
        """Write a timestamp."""
        self.buffer.extend(struct.pack("<Q", value))

    # PrimitiveType.DURATION
    def write_duration(self, value: Duration) -> None:
        """Write a duration."""
        nanos = int(value.total_seconds() * 1_000_000_000)
        self.buffer.extend(struct.pack("<q", nanos))

    # PrimitiveType.STRING
    def write_string(self, value: String) -> None:
        """Write a UTF-8 string."""
        data = value.encode("utf-8")
        self.buffer.extend(struct.pack("<I", len(data)))
        self.buffer.extend(data)

    # PrimitiveType.CHARACTER
    def write_character(self, value: Character) -> None:
        """Write a single Unicode character."""
        data = value.encode("utf-8")
        self.buffer.extend(struct.pack("<I", len(data)))
        self.buffer.extend(data)

    # PrimitiveType.UUID
    def write_uuid(self, value: UUID) -> None:
        """Write a UUID."""
        self.buffer.extend(value.bytes)

    # PrimitiveType.BYTES
    def write_bytes(self, value: Bytes) -> None:
        """Write raw bytes."""
        self.buffer.extend(struct.pack("<I", len(value)))
        self.buffer.extend(value)

    # PrimitiveType.JSON
    def write_json(self, value: Any) -> None:
        """Write JSON."""
        if value is None:
            self.buffer.append(0)
        elif value is False:
            self.buffer.append(1)
        elif value is True:
            self.buffer.append(2)
        elif isinstance(value, int):
            self.buffer.append(3)
            self.write_int64(value)
        elif isinstance(value, float):
            self.buffer.append(4)
            self.write_float64(value)
        elif isinstance(value, str):
            self.buffer.append(5)
            self.write_string(value)
        elif isinstance(value, list):
            self.buffer.append(6)
            self.buffer.extend(struct.pack("<I", len(value)))
            for item in value:
                self.write_json(item)
        elif isinstance(value, dict):
            self.buffer.append(7)
            self.buffer.extend(struct.pack("<I", len(value)))
            for key, val in value.items():
                if not isinstance(key, str):
                    raise BinaryError(f"invalid JSON key type '{type(key)}': {key!r}")
                self.write_string(key)
                self.write_json(val)
        else:
            raise BinaryError(f"invalid JSON type '{type(value)}': {value!r}")


class BinaryDecoder:
    """Read binary primitive values in some encoding."""

    def __init__(self, buffer: bytes) -> None:
        self.buffer = buffer
        self.pos: UInt32 = 0

    def __str__(self) -> str:
        return f"pos={self.pos}, remaining={self.remaining}"

    def __repr__(self) -> str:
        return f"<BinaryDecoder pos={self.pos}, remaining={self.remaining}>"

    @property
    def remaining(self) -> int:
        """Number of bytes remaining."""
        return len(self.buffer) - self.pos

    # PrimitiveType.BOOLEAN
    def read_bool(self) -> bool:
        """Read a boolean."""
        if self.pos >= len(self.buffer):
            raise BinaryError(f"unexpected end of buffer at {self.pos}")
        value = self.buffer[self.pos]
        self.pos += 1
        return value != 0

    # PrimitiveType.INT8
    def read_int8(self) -> Int8:
        """Read a signed 8-bit integer."""
        if self.pos + 1 > len(self.buffer):
            raise BinaryError(f"unexpected end of buffer at {self.pos}")
        value = struct.unpack("<b", self.buffer[self.pos : self.pos + 1])[0]
        self.pos += 1
        return value

    # PrimitiveType.INT16
    def read_int16(self) -> Int16:
        """Read a signed 16-bit integer."""
        if self.pos + 2 > len(self.buffer):
            raise BinaryError(f"unexpected end of buffer at {self.pos}")
        value = struct.unpack("<h", self.buffer[self.pos : self.pos + 2])[0]
        self.pos += 2
        return value

    # PrimitiveType.INT32
    def read_int32(self) -> Int32:
        """Read a signed 32-bit integer."""
        if self.pos + 4 > len(self.buffer):
            raise BinaryError(f"unexpected end of buffer at {self.pos}")
        value = struct.unpack("<i", self.buffer[self.pos : self.pos + 4])[0]
        self.pos += 4
        return value

    # PrimitiveType.INT64
    def read_int64(self) -> Int64:
        """Read a signed 64-bit integer."""
        if self.pos + 8 > len(self.buffer):
            raise BinaryError(f"unexpected end of buffer at {self.pos}")
        value = struct.unpack("<q", self.buffer[self.pos : self.pos + 8])[0]
        self.pos += 8
        return value

    # PrimitiveType.INT128
    def read_int128(self) -> Int128:
        """Read a signed 128-bit integer."""
        if self.pos + 16 > len(self.buffer):
            raise BinaryError(f"unexpected end of buffer at {self.pos}")
        low, high = struct.unpack("<QQ", self.buffer[self.pos : self.pos + 16])
        self.pos += 16
        value = (high << 64) | low
        # handle sign extension for 128-bit
        if high & 0x8000000000000000:
            value -= 1 << 128
        return value

    # PrimitiveType.UINT8
    def read_uint8(self) -> UInt8:
        """Read an unsigned 8-bit integer."""
        if self.pos + 1 > len(self.buffer):
            raise BinaryError(f"unexpected end of buffer at {self.pos}")
        value = struct.unpack("<B", self.buffer[self.pos : self.pos + 1])[0]
        self.pos += 1
        return value

    # PrimitiveType.UINT16
    def read_uint16(self) -> UInt16:
        """Read an unsigned 16-bit integer."""
        if self.pos + 2 > len(self.buffer):
            raise BinaryError(f"unexpected end of buffer at {self.pos}")
        value = struct.unpack("<H", self.buffer[self.pos : self.pos + 2])[0]
        self.pos += 2
        return value

    # PrimitiveType.UINT32
    def read_uint32(self) -> UInt32:
        """Read an unsigned 32-bit integer."""
        if self.pos + 4 > len(self.buffer):
            raise BinaryError(f"unexpected end of buffer at {self.pos}")
        value = struct.unpack("<I", self.buffer[self.pos : self.pos + 4])[0]
        self.pos += 4
        return value

    # PrimitiveType.UINT64
    def read_uint64(self) -> UInt64:
        """Read an unsigned 64-bit integer."""
        if self.pos + 8 > len(self.buffer):
            raise BinaryError(f"unexpected end of buffer at {self.pos}")
        value = struct.unpack("<Q", self.buffer[self.pos : self.pos + 8])[0]
        self.pos += 8
        return value

    # PrimitiveType.UINT128
    def read_uint128(self) -> UInt128:
        """Read an unsigned 128-bit integer."""
        if self.pos + 16 > len(self.buffer):
            raise BinaryError(f"unexpected end of buffer at {self.pos}")
        low, high = struct.unpack("<QQ", self.buffer[self.pos : self.pos + 16])
        self.pos += 16
        return (high << 64) | low

    # PrimitiveType.FLOAT32
    def read_float32(self) -> Float32:
        """Read a 32-bit float."""
        if self.pos + 4 > len(self.buffer):
            raise BinaryError(f"unexpected end of buffer at {self.pos}")
        value = struct.unpack("<f", self.buffer[self.pos : self.pos + 4])[0]
        self.pos += 4
        return value

    # PrimitiveType.FLOAT64
    def read_float64(self) -> Float64:
        """Read a 64-bit float."""
        if self.pos + 8 > len(self.buffer):
            raise BinaryError(f"unexpected end of buffer at {self.pos}")
        value = struct.unpack("<d", self.buffer[self.pos : self.pos + 8])[0]
        self.pos += 8
        return value

    # PrimitiveType.DATETIME
    def read_datetime(self) -> DateTime:
        """Read a datetime."""
        if self.pos + 8 > len(self.buffer):
            raise BinaryError(f"unexpected end of buffer at {self.pos}")
        micros = struct.unpack("<q", self.buffer[self.pos : self.pos + 8])[0]
        self.pos += 8

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

    # PrimitiveType.DATE
    def read_date(self) -> Date:
        """Read a date."""
        if self.pos + 8 > len(self.buffer):
            raise BinaryError(f"unexpected end of buffer at {self.pos}")
        days = struct.unpack("<q", self.buffer[self.pos : self.pos + 8])[0]
        self.pos += 8
        epoch = datetime(1970, 1, 1, tzinfo=UTC)
        return epoch.date() + timedelta(days=days)

    # PrimitiveType.TIME
    def read_time(self) -> Time:
        """Read a time."""
        if self.pos + 8 > len(self.buffer):
            raise BinaryError(f"unexpected end of buffer at {self.pos}")
        nanos = struct.unpack("<Q", self.buffer[self.pos : self.pos + 8])[0]
        self.pos += 8
        hours = nanos // 3_600_000_000_000
        nanos %= 3_600_000_000_000
        minutes = nanos // 60_000_000_000
        nanos %= 60_000_000_000
        seconds = nanos // 1_000_000_000
        microseconds = (nanos % 1_000_000_000) // 1_000
        return time(hours, minutes, seconds, microseconds)

    # PrimitiveType.TIMESTAMP
    def read_timestamp(self) -> Timestamp:
        """Read a timestamp."""
        if self.pos + 8 > len(self.buffer):
            raise BinaryError(f"unexpected end of buffer at {self.pos}")
        value = struct.unpack("<Q", self.buffer[self.pos : self.pos + 8])[0]
        self.pos += 8
        return value

    # PrimitiveType.DURATION
    def read_duration(self) -> Duration:
        """Read a duration."""
        if self.pos + 8 > len(self.buffer):
            raise BinaryError(f"unexpected end of buffer at {self.pos}")
        nanos = struct.unpack("<q", self.buffer[self.pos : self.pos + 8])[0]
        self.pos += 8
        return timedelta(microseconds=nanos // 1_000)

    # PrimitiveType.STRING
    def read_string(self) -> String:
        """Read a UTF-8 string."""
        if self.pos + 4 > len(self.buffer):
            raise BinaryError(f"unexpected end of buffer at {self.pos}")
        length = struct.unpack("<I", self.buffer[self.pos : self.pos + 4])[0]
        self.pos += 4
        if self.pos + length > len(self.buffer):
            raise BinaryError(f"unexpected end of buffer at {self.pos}")
        value = self.buffer[self.pos : self.pos + length].decode("utf-8")
        self.pos += length
        return value

    # PrimitiveType.CHARACTER
    def read_character(self) -> Character:
        """Read a single Unicode character."""
        if self.pos + 4 > len(self.buffer):
            raise BinaryError(f"unexpected end of buffer at {self.pos}")
        length = struct.unpack("<I", self.buffer[self.pos : self.pos + 4])[0]
        self.pos += 4
        if self.pos + length > len(self.buffer):
            raise BinaryError(f"unexpected end of buffer at {self.pos}")
        value = self.buffer[self.pos : self.pos + length].decode("utf-8")
        self.pos += length
        return value

    # PrimitiveType.UUID
    def read_uuid(self) -> UUID:
        """Read a UUID."""
        if self.pos + 16 > len(self.buffer):
            raise BinaryError(f"unexpected end of buffer at {self.pos}")
        uuid_bytes = self.buffer[self.pos : self.pos + 16]
        self.pos += 16
        return UUID(bytes=uuid_bytes)

    # PrimitiveType.BYTES
    def read_bytes(self) -> Bytes:
        """Read raw bytes."""
        if self.pos + 4 > len(self.buffer):
            raise BinaryError(f"unexpected end of buffer at {self.pos}")
        length = struct.unpack("<I", self.buffer[self.pos : self.pos + 4])[0]
        self.pos += 4
        if self.pos + length > len(self.buffer):
            raise BinaryError(f"unexpected end of buffer at {self.pos}")
        value = self.buffer[self.pos : self.pos + length]
        self.pos += length
        return value

    # PrimitiveType.JSON
    def read_json(self) -> Any:
        """Read JSON."""
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
            if self.pos + 4 > len(self.buffer):
                raise BinaryError(f"unexpected end of buffer at {self.pos}")
            length = struct.unpack("<I", self.buffer[self.pos : self.pos + 4])[0]
            self.pos += 4
            return [self.read_json() for _ in range(length)]
        elif tag == 7:
            if self.pos + 4 > len(self.buffer):
                raise BinaryError(f"unexpected end of buffer at {self.pos}")
            length = struct.unpack("<I", self.buffer[self.pos : self.pos + 4])[0]
            self.pos += 4
            result = {}
            for _ in range(length):
                key = self.read_string()
                value = self.read_json()
                result[key] = value
            return result
        else:
            raise BinaryError(f"invalid JSON type tag at {self.pos - 1}: {tag}")
