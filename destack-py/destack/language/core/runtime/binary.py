import struct
from datetime import UTC, date, datetime, time, timedelta
from typing import TYPE_CHECKING, Any

from destack.utils.uuid import UUID

if TYPE_CHECKING:
    pass


class BinaryError(ValueError):
    """Base class for binary encoding/decoding errors."""


_EPOCH_DATE = date(1, 1, 1)  # year 1 AD as epoch for wider date range support


class BinaryWriter:
    """Write binary data in our custom encoding. Little-endian, varint, zigzag, etc."""

    __slots__ = ("buffer",)

    def __init__(self) -> None:
        self.buffer = bytearray()

    def __str__(self) -> str:
        return f"buffer={len(self.buffer)}"

    def __repr__(self) -> str:
        return f"<BinaryWriter buffer={len(self.buffer)}>"

    def __len__(self) -> int:
        return len(self.buffer)

    def to_bytes(self) -> bytes:
        """Get the written bytes."""
        return bytes(self.buffer)

    # PrimitiveType.BOOLEAN
    def write_bool(self, value: bool) -> None:
        """
        Write a boolean as 1 byte (0 for false, 1 for true).
        Size: 1 byte.
        """
        self.buffer.append(1 if value else 0)

    # PrimitiveType.INT8
    def write_int8(self, value: int) -> None:
        """
        Write a signed 8-bit integer as raw byte.
        Range: -2^7 to 2^7-1.
        Size: 1 byte.
        """
        self.buffer.append(value & 0xFF)

    # PrimitiveType.INT16
    def write_int16(self, value: int) -> None:
        """
        Write a signed 16-bit integer using zigzag encoding then varint.
        Range: -2^15 to 2^15-1.
        Size: 1-3 bytes.
        """
        self._write_varint(self._write_zigzag(value))

    # PrimitiveType.INT32
    def write_int32(self, value: int) -> None:
        """
        Write a signed 32-bit integer using zigzag encoding then varint.
        Range: -2^31 to 2^31-1.
        Size: 1-5 bytes.
        """
        self._write_varint(self._write_zigzag(value))

    # PrimitiveType.INT64
    def write_int64(self, value: int) -> None:
        """
        Write a signed 64-bit integer using zigzag encoding then varint.
        Range: -2^63 to 2^63-1.
        Size: 1-10 bytes.
        """
        self._write_varint(self._write_zigzag(value))

    # PrimitiveType.INT128
    def write_int128(self, value: int) -> None:
        """
        Write a signed 128-bit integer using zigzag encoding then varint.
        Range: -2^127 to 2^127-1.
        Size: 1-19 bytes.
        """
        self._write_varint(self._write_zigzag(value))

    # PrimitiveType.UINT8
    def write_uint8(self, value: int) -> None:
        """
        Write an unsigned 8-bit integer as raw byte.
        Range: 0 to 2^8-1.
        Size: 1 byte.
        """
        self.buffer.append(value & 0xFF)

    # PrimitiveType.UINT16
    def write_uint16(self, value: int) -> None:
        """
        Write an unsigned 16-bit integer using varint encoding.
        Range: 0 to 2^16-1.
        Size: 1-3 bytes.
        """
        self._write_varint(value)

    # PrimitiveType.UINT32
    def write_uint32(self, value: int) -> None:
        """
        Write an unsigned 32-bit integer using varint encoding.
        Range: 0 to 2^32-1.
        Size: 1-5 bytes.
        """
        self._write_varint(value)

    # PrimitiveType.UINT64
    def write_uint64(self, value: int) -> None:
        """
        Write an unsigned 64-bit integer using varint encoding.
        Range: 0 to 2^64-1.
        Size: 1-10 bytes.
        """
        self._write_varint(value)

    # PrimitiveType.UINT128
    def write_uint128(self, value: int) -> None:
        """
        Write an unsigned 128-bit integer using varint encoding.
        Range: 0 to 2^128-1.
        Size: 1-19 bytes.
        """
        self._write_varint(value)

    # PrimitiveType.FLOAT16
    def write_float16(self, value: float) -> None:
        """
        Write a fixed-length 16-bit float.
        Range: ±6.55e4 (half precision IEEE 754).
        Size: 2 bytes.
        """
        self.buffer.extend(struct.pack("<e", value))

    # PrimitiveType.FLOAT32
    def write_float32(self, value: float) -> None:
        """
        Write a fixed-length 32-bit float.
        Range: ±3.4e38 (single precision IEEE 754).
        Size: 4 bytes.
        """
        self.buffer.extend(struct.pack("<f", value))

    # PrimitiveType.FLOAT64
    def write_float64(self, value: float) -> None:
        """
        Write a fixed-length 64-bit float.
        Range: ±1.8e308 (double precision IEEE 754).
        Size: 8 bytes.
        """
        self.buffer.extend(struct.pack("<d", value))

    # PrimitiveType.DATETIME
    def write_datetime(self, value: "datetime") -> None:
        """
        Write a datetime as zigzag-encoded varint of microseconds since epoch.
        Range: 0001-01-01 00:00:00 to 9999-12-31 23:59:59.999999 UTC.
        Size: 1-10 bytes.
        """
        # ensure UTC timezone
        if value.tzinfo is None:
            value = value.replace(tzinfo=UTC)
        # calculate microseconds manually to support full date range
        days = (value.date() - _EPOCH_DATE).days
        time_micros = (
            value.hour * 3_600_000_000
            + value.minute * 60_000_000
            + value.second * 1_000_000
            + value.microsecond
        )
        micros = days * 86_400_000_000 + time_micros
        self._write_varint(self._write_zigzag(micros))

    # PrimitiveType.DATE
    def write_date(self, value: "date") -> None:
        """
        Write a date as zigzag-encoded varint of days since epoch (0001-01-01).
        Range: 0001-01-01 to 9999-12-31.
        Size: 1-5 bytes.
        """
        days = (value - _EPOCH_DATE).days
        self._write_varint(self._write_zigzag(days))

    # PrimitiveType.TIME
    def write_time(self, value: "time") -> None:
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
    def write_duration(self, value: "timedelta") -> None:
        """
        Write a duration as zigzag-encoded varint of microseconds.
        Range: -999,999,999 days to 999,999,999 days.
        Size: 1-10 bytes.
        """
        # use integer math to avoid precision issues with large durations
        micros = value.days * 86_400_000_000 + value.seconds * 1_000_000 + value.microseconds
        self._write_varint(self._write_zigzag(micros))

    # PrimitiveType.STRING
    def write_string(self, value: str) -> None:
        """
        Write a UTF-8 string with varint length prefix followed by UTF-8 bytes.
        Range: 0 to 2^32-1 bytes (UTF-8 encoded).
        Size: 1-5 bytes (length) + string length in bytes.
        """
        encoded = value.encode("utf-8")
        self.write_bytes(encoded)

    # PrimitiveType.UUID
    def write_uuid(self, value: "UUID") -> None:
        """
        Write a UUID as 16 raw bytes.
        Size: 16 bytes.
        """
        self.buffer.extend(value.bytes)

    # PrimitiveType.BYTES
    def write_bytes(self, value: bytes) -> None:
        """
        Write raw bytes with varint length prefix followed by the bytes.
        Range: 0 to 2^32-1 bytes.
        Size: 1-5 bytes (length) + data length.
        """
        self._write_varint(len(value))
        self.buffer.extend(value)

    # PrimitiveType.JSON
    def write_json(self, value: "Any") -> None:
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


class BinaryReader:
    """Read binary data in our custom encoding. Little-endian, varint, zigzag, etc."""

    __slots__ = ("buffer", "pos")

    def __init__(self, data: bytes) -> None:
        self.buffer = data
        self.pos = 0

    def __str__(self) -> str:
        return f"pos={self.pos}, remaining={self.remaining}"

    def __repr__(self) -> str:
        return f"<BinaryReader pos={self.pos}, remaining={self.remaining}>"

    @property
    def remaining(self) -> int:
        """Number of bytes remaining."""
        return len(self.buffer) - self.pos

    # PrimitiveType.UINT8
    def peek_uint8(self) -> int:
        """Peek at the next unsigned 8-bit integer without advancing the position."""
        if self.pos >= len(self.buffer):
            raise BinaryError(f"unexpected end of buffer at {self.pos}")
        return self.buffer[self.pos] & 0xFF

    # PrimitiveType.UINT16
    def peek_uint16(self) -> int:
        """Peek at the next unsigned 16-bit integer without advancing the position."""
        if self.pos + 2 > len(self.buffer):
            raise BinaryError(f"unexpected end of buffer at {self.pos}")
        return self._peek_varint()

    # PrimitiveType.UINT32
    def peek_uint32(self) -> int:
        """Peek at the next unsigned 32-bit integer without advancing the position."""
        if self.pos + 4 > len(self.buffer):
            raise BinaryError(f"unexpected end of buffer at {self.pos}")
        return self._peek_varint()

    # PrimitiveType.UINT64
    def peek_uint64(self) -> int:
        """Peek at the next unsigned 64-bit integer without advancing the position."""
        if self.pos + 8 > len(self.buffer):
            raise BinaryError(f"unexpected end of buffer at {self.pos}")
        return self._peek_varint()

    # PrimitiveType.BOOLEAN
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
    def read_int8(self) -> int:
        """
        Read a signed 8-bit integer from raw byte with sign extension.
        Range: -128 to 127
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
    def read_int16(self) -> int:
        """
        Read a signed 16-bit integer from zigzag-decoded varint.
        Range: -32,768 to 32,767
        Size: 1-3 bytes.
        """
        return self._zigzag_decode(self._read_varint())

    # PrimitiveType.INT32
    def read_int32(self) -> int:
        """
        Read a signed 32-bit integer from zigzag-decoded varint.
        Range: -2,147,483,648 to 2,147,483,647
        Size: 1-5 bytes.
        """
        return self._zigzag_decode(self._read_varint())

    # PrimitiveType.INT64
    def read_int64(self) -> int:
        """
        Read a signed 64-bit integer from zigzag-decoded varint.
        Range: -9,223,372,036,854,775,808 to 9,223,372,036,854,775,807
        Size: 1-10 bytes.
        """
        return self._zigzag_decode(self._read_varint())

    # PrimitiveType.INT128
    def read_int128(self) -> int:
        """
        Read a signed 128-bit integer from zigzag-decoded varint.
        Range: -170,141,183,460,469,231,731,687,303,715,884,105,728 to 170,141,183,460,469,231,731,687,303,715,884,105,727
        Size: 1-19 bytes.
        """
        return self._zigzag_decode(self._read_varint())

    # PrimitiveType.UINT8
    def read_uint8(self) -> int:
        """
        Read an unsigned 8-bit integer from raw byte.
        Range: 0 to 255
        Size: 1 byte.
        """
        if self.pos >= len(self.buffer):
            raise BinaryError(f"unexpected end of buffer at {self.pos}")
        value = self.buffer[self.pos]
        self.pos += 1
        return value

    # PrimitiveType.UINT16
    def read_uint16(self) -> int:
        """
        Read an unsigned 16-bit integer from varint.
        Range: 0 to 65,535
        Size: 1-3 bytes.
        """
        return self._read_varint()

    # PrimitiveType.UINT32
    def read_uint32(self) -> int:
        """
        Read an unsigned 32-bit integer from varint.
        Range: 0 to 4,294,967,295
        Size: 1-5 bytes.
        """
        return self._read_varint()

    # PrimitiveType.UINT64
    def read_uint64(self) -> int:
        """
        Read an unsigned 64-bit integer from varint.
        Range: 0 to 18,446,744,073,709,551,615
        Size: 1-10 bytes.
        """
        return self._read_varint()

    # PrimitiveType.UINT128
    def read_uint128(self) -> int:
        """
        Read an unsigned 128-bit integer from varint.
        Range: 0 to 340,282,366,920,938,463,463,374,607,431,768,211,455
        Size: 1-19 bytes.
        """
        return self._read_varint()

    # PrimitiveType.FLOAT16
    def read_float16(self) -> float:
        """
        Read a fixed-length 16-bit float.
        Range: ±6.55e4 (half precision IEEE 754)
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
    def read_float32(self) -> float:
        """
        Read a fixed-length 32-bit float.
        Range: ±3.4e38 (single precision IEEE 754)
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
    def read_float64(self) -> float:
        """
        Read a fixed-length 64-bit float.
        Range: ±1.8e308 (double precision IEEE 754)
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
    def read_datetime(self) -> datetime:
        """
        Read a datetime from zigzag-decoded varint of microseconds since epoch.
        Range: 0001-01-01 00:00:00 to 9999-12-31 23:59:59.999999 UTC
        Size: 1-10 bytes.
        """
        micros = self._zigzag_decode(self._read_varint())
        # calculate datetime manually to support full date range
        days = micros // 86_400_000_000
        time_micros = micros % 86_400_000_000

        date_part = _EPOCH_DATE + timedelta(days=days)
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
    def read_date(self) -> date:
        """
        Read a date from zigzag-decoded varint of days since epoch (0001-01-01).
        Range: 0001-01-01 to 9999-12-31
        Size: 1-5 bytes.
        """
        days = self._zigzag_decode(self._read_varint())
        return _EPOCH_DATE + timedelta(days=days)

    # PrimitiveType.TIME
    def read_time(self) -> time:
        """
        Read a time from varint of microseconds since midnight.
        Range: 00:00:00 to 23:59:59.999999
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
    def read_duration(self) -> timedelta:
        """
        Read a duration from zigzag-decoded varint of microseconds.
        Range: -999,999,999 days to 999,999,999 days
        Size: 1-10 bytes.
        """
        micros = self._zigzag_decode(self._read_varint())
        return timedelta(microseconds=micros)

    # PrimitiveType.STRING
    def read_string(self) -> str:
        """
        Read a UTF-8 string from varint length prefix + UTF-8 bytes.
        Range: 0 to 2^32-1 bytes (UTF-8 encoded)
        Size: 1-5 bytes (length) + string length in bytes.
        """
        return self.read_bytes().decode("utf-8")

    # PrimitiveType.UUID
    def read_uuid(self) -> "UUID":
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
    def read_bytes(self) -> bytes:
        """
        Read raw bytes from varint length prefix + data bytes.
        Range: 0 to 2^32-1 bytes
        Size: 1-5 bytes (length) + data length.
        """
        length = self._read_varint()
        if self.pos + length > len(self.buffer):
            raise BinaryError(f"unexpected end of buffer at {self.pos}")
        value = self.buffer[self.pos : self.pos + length]
        self.pos += length
        return value

    # PrimitiveType.JSON
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

    def _peek_varint(self) -> int:
        """Peek at the next unsigned integer using variable-length encoding."""
        value = 0
        shift = 0
        while True:
            if self.pos >= len(self.buffer):
                raise BinaryError(f"unexpected end of buffer at {self.pos}")
            byte = self.buffer[self.pos]
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
