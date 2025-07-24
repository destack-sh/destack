import struct
from datetime import UTC, date, datetime, time, timedelta
from typing import TYPE_CHECKING, Any

from destack.utils.uuid import UUID

if TYPE_CHECKING:
    pass


class BinaryError(ValueError):
    """Base class for binary encoding/decoding errors."""


_EPOCH_DATE = date(1970, 1, 1)


class BinaryWriter:
    """Write binary data in our custom encoding. Little-endian, varint, zigzag, etc."""

    def __init__(self) -> None:
        self.buffer = bytearray()

    def to_bytes(self) -> bytes:
        """Get the written bytes."""
        return bytes(self.buffer)

    # PrimitiveType.BOOLEAN
    def write_bool(self, value: bool) -> None:
        """Write a boolean as 1 byte."""
        self.buffer.append(1 if value else 0)

    # PrimitiveType.SINT8
    def write_sint8(self, value: int) -> None:
        """Write a signed 8-bit integer."""
        self.buffer.append(value & 0xFF)

    # PrimitiveType.SINT16
    def write_sint16(self, value: int) -> None:
        """Write a signed 16-bit integer with variable-length encoding."""
        self._write_varint(self._write_zigzag(value))

    # PrimitiveType.SINT32
    def write_sint32(self, value: int) -> None:
        """Write a signed 32-bit integer with variable-length encoding."""
        self._write_varint(self._write_zigzag(value))

    # PrimitiveType.SINT64
    def write_sint64(self, value: int) -> None:
        """Write a signed 64-bit integer with variable-length encoding."""
        self._write_varint(self._write_zigzag(value))

    # PrimitiveType.SINT128
    def write_sint128(self, value: int) -> None:
        """Write a signed 128-bit integer with variable-length encoding."""
        self._write_varint(self._write_zigzag(value))

    # PrimitiveType.UINT8
    def write_uint8(self, value: int) -> None:
        """Write an unsigned 8-bit integer."""
        self.buffer.append(value & 0xFF)

    # PrimitiveType.UINT16
    def write_uint16(self, value: int) -> None:
        """Write an unsigned 16-bit integer with variable-length encoding."""
        self._write_varint(value)

    # PrimitiveType.UINT32
    def write_uint32(self, value: int) -> None:
        """Write an unsigned 32-bit integer with variable-length encoding."""
        self._write_varint(value)

    # PrimitiveType.UINT64
    def write_uint64(self, value: int) -> None:
        """Write an unsigned 64-bit integer with variable-length encoding."""
        self._write_varint(value)

    # PrimitiveType.UINT128
    def write_uint128(self, value: int) -> None:
        """Write an unsigned 128-bit integer with variable-length encoding."""
        self._write_varint(value)

    # PrimitiveType.FLOAT16
    def write_float16(self, value: float) -> None:
        """Write a 16-bit float with optimized encoding for zero."""
        if value == 0.0:
            self.buffer.append(0)
        else:
            self.buffer.append(1)
            self.buffer.extend(struct.pack("<e", value))

    # PrimitiveType.FLOAT32
    def write_float32(self, value: float) -> None:
        """Write a 32-bit float with optimized encoding for zero."""
        if value == 0.0:
            self.buffer.append(0)
        else:
            self.buffer.append(1)
            self.buffer.extend(struct.pack("<f", value))

    # PrimitiveType.FLOAT64
    def write_float64(self, value: float) -> None:
        """Write a 64-bit float with optimized encoding."""
        if value == 0.0:
            self.buffer.append(0)
        elif (
            not (value != value or value == float("inf") or value == float("-inf"))
            and value == float(int(value))
            and -(2**53) <= value <= 2**53
        ):
            # can be represented exactly as an integer
            self.buffer.append(1)
            self._write_varint(self._write_zigzag(int(value)))
        else:
            self.buffer.append(2)
            self.buffer.extend(struct.pack("<d", value))

    # PrimitiveType.DATETIME
    def write_datetime(self, value: "datetime") -> None:
        """Write a datetime as microseconds since epoch."""
        # ensure UTC timezone
        if value.tzinfo is None:
            value = value.replace(tzinfo=UTC)
        micros = int(value.timestamp() * 1_000_000)
        self._write_varint(self._write_zigzag(micros))

    # PrimitiveType.DATE
    def write_date(self, value: "date") -> None:
        """Write a date as days since epoch."""
        days = (value - _EPOCH_DATE).days
        self._write_varint(self._write_zigzag(days))

    # PrimitiveType.TIME
    def write_time(self, value: "time") -> None:
        """Write a time as microseconds since midnight."""
        micros = (
            value.hour * 3_600_000_000
            + value.minute * 60_000_000
            + value.second * 1_000_000
            + value.microsecond
        )
        self._write_varint(micros)

    # PrimitiveType.DURATION
    def write_duration(self, value: "timedelta") -> None:
        """Write a duration as microseconds."""
        micros = int(value.total_seconds() * 1_000_000)
        self._write_varint(self._write_zigzag(micros))

    # PrimitiveType.STRING
    def write_string(self, value: str) -> None:
        """Write a UTF-8 string with length prefix."""
        encoded = value.encode("utf-8")
        self.write_bytes(encoded)

    # PrimitiveType.UUID
    def write_uuid(self, value: "UUID") -> None:
        """Write a UUID as 16 bytes."""
        self.buffer.extend(value.bytes)

    # PrimitiveType.BYTES
    def write_bytes(self, value: bytes) -> None:
        """Write raw bytes with length prefix."""
        self._write_varint(len(value))
        self.buffer.extend(value)

    # PrimitiveType.JSON
    def write_json(self, value: "Any") -> None:
        """Write JSON as a string."""
        import json

        self.write_string(json.dumps(value, separators=(",", ":")))

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


class BinaryReader:
    """Read binary data in our custom encoding. Little-endian, varint, zigzag, etc."""

    def __init__(self, data: bytes) -> None:
        self.buffer = data
        self.pos = 0

    @property
    def remaining(self) -> int:
        """Number of bytes remaining."""
        return len(self.buffer) - self.pos

    # PrimitiveType.BOOLEAN
    def read_bool(self) -> bool:
        """Read a boolean from 1 byte."""
        if self.pos >= len(self.buffer):
            raise BinaryError(f"unexpected end of buffer at {self.pos}")
        value = self.buffer[self.pos]
        self.pos += 1
        return value != 0

    # PrimitiveType.SINT8
    def read_sint8(self) -> int:
        """Read a signed 8-bit integer."""
        if self.pos >= len(self.buffer):
            raise BinaryError(f"unexpected end of buffer at {self.pos}")
        value = self.buffer[self.pos]
        self.pos += 1
        # convert to signed
        if value >= 0x80:
            return value - 0x100
        return value

    # PrimitiveType.SINT16
    def read_sint16(self) -> int:
        """Read a signed 16-bit integer with variable-length encoding."""
        return self._zigzag_decode(self._read_varint())

    # PrimitiveType.SINT32
    def read_sint32(self) -> int:
        """Read a signed 32-bit integer with variable-length encoding."""
        return self._zigzag_decode(self._read_varint())

    # PrimitiveType.SINT64
    def read_sint64(self) -> int:
        """Read a signed 64-bit integer with variable-length encoding."""
        return self._zigzag_decode(self._read_varint())

    # PrimitiveType.SINT128
    def read_sint128(self) -> int:
        """Read a signed 128-bit integer with variable-length encoding."""
        return self._zigzag_decode(self._read_varint())

    # PrimitiveType.UINT8
    def read_uint8(self) -> int:
        """Read an unsigned 8-bit integer."""
        if self.pos >= len(self.buffer):
            raise BinaryError(f"unexpected end of buffer at {self.pos}")
        value = self.buffer[self.pos]
        self.pos += 1
        return value

    # PrimitiveType.UINT16
    def read_uint16(self) -> int:
        """Read an unsigned 16-bit integer with variable-length encoding."""
        return self._read_varint()

    # PrimitiveType.UINT32
    def read_uint32(self) -> int:
        """Read an unsigned 32-bit integer with variable-length encoding."""
        return self._read_varint()

    # PrimitiveType.UINT64
    def read_uint64(self) -> int:
        """Read an unsigned 64-bit integer with variable-length encoding."""
        return self._read_varint()

    # PrimitiveType.UINT128
    def read_uint128(self) -> int:
        """Read an unsigned 128-bit integer with variable-length encoding."""
        return self._read_varint()

    # PrimitiveType.FLOAT16
    def read_float16(self) -> float:
        """Read a 16-bit float with optimized encoding for zero."""
        if self.pos >= len(self.buffer):
            raise BinaryError(f"unexpected end of buffer at {self.pos}")

        flag = self.buffer[self.pos]
        self.pos += 1

        if flag == 0:
            return 0.0
        elif flag == 1:
            if self.pos + 2 > len(self.buffer):
                raise BinaryError(f"unexpected end of buffer at {self.pos}")
            value = struct.unpack("<e", self.buffer[self.pos : self.pos + 2])[0]
            self.pos += 2
            return value
        else:
            raise BinaryError(f"invalid float16 encoding flag at {self.pos}: {flag}")

    # PrimitiveType.FLOAT32
    def read_float32(self) -> float:
        """Read a 32-bit float with optimized encoding for zero."""
        if self.pos >= len(self.buffer):
            raise BinaryError(f"unexpected end of buffer at {self.pos}")

        flag = self.buffer[self.pos]
        self.pos += 1

        if flag == 0:
            return 0.0
        elif flag == 1:
            if self.pos + 4 > len(self.buffer):
                raise BinaryError(f"unexpected end of buffer at {self.pos}")
            value = struct.unpack("<f", self.buffer[self.pos : self.pos + 4])[0]
            self.pos += 4
            return value
        else:
            raise BinaryError(f"invalid float32 encoding flag at {self.pos}: {flag}")

    # PrimitiveType.FLOAT64
    def read_float64(self) -> float:
        """Read a 64-bit float with optimized encoding."""
        if self.pos >= len(self.buffer):
            raise BinaryError(f"unexpected end of buffer at {self.pos}")

        flag = self.buffer[self.pos]
        self.pos += 1

        if flag == 0:
            return 0.0
        elif flag == 1:
            # integer representation
            return float(self._zigzag_decode(self._read_varint()))
        elif flag == 2:
            if self.pos + 8 > len(self.buffer):
                raise BinaryError(f"unexpected end of buffer at {self.pos}")
            value = struct.unpack("<d", self.buffer[self.pos : self.pos + 8])[0]
            self.pos += 8
            return value
        else:
            raise BinaryError(f"invalid float64 encoding flag at {self.pos}: {flag}")

    # PrimitiveType.DATETIME
    def read_datetime(self) -> datetime:
        """Read a datetime from microseconds since epoch."""
        micros = self._zigzag_decode(self._read_varint())
        return datetime.fromtimestamp(micros / 1_000_000, tz=UTC)

    # PrimitiveType.DATE
    def read_date(self) -> date:
        """Read a date from days since epoch."""
        days = self._zigzag_decode(self._read_varint())
        return _EPOCH_DATE + timedelta(days=days)

    # PrimitiveType.TIME
    def read_time(self) -> time:
        """Read a time from microseconds since midnight."""
        micros = self._read_varint()
        hours = micros // 3_600_000_000
        micros %= 3_600_000_000
        minutes = micros // 60_000_000
        micros %= 60_000_000
        seconds = micros // 1_000_000
        microseconds = micros % 1_000_000
        return time(hours, minutes, seconds, microseconds)

    # PrimitiveType.DURATION
    def read_duration(self) -> timedelta:
        """Read a duration from microseconds."""
        micros = self._zigzag_decode(self._read_varint())
        return timedelta(microseconds=micros)

    # PrimitiveType.STRING
    def read_string(self) -> str:
        """Read a UTF-8 string with length prefix."""
        return self.read_bytes().decode("utf-8")

    # PrimitiveType.UUID
    def read_uuid(self) -> "UUID":
        """Read a UUID from 16 bytes."""
        if self.pos + 16 > len(self.buffer):
            raise BinaryError(f"unexpected end of buffer at {self.pos}")
        uuid_bytes = self.buffer[self.pos : self.pos + 16]
        self.pos += 16
        return UUID(bytes=uuid_bytes)

    # PrimitiveType.BYTES
    def read_bytes(self) -> bytes:
        """Read raw bytes with length prefix."""
        length = self._read_varint()
        if self.pos + length > len(self.buffer):
            raise BinaryError(f"unexpected end of buffer at {self.pos}")
        value = self.buffer[self.pos : self.pos + length]
        self.pos += length
        return value

    # PrimitiveType.JSON
    def read_json(self) -> Any:
        """Read JSON from a string."""
        import json

        return json.loads(self.read_string())

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
