import json
import math
import struct
from datetime import UTC, date, datetime, time, timedelta
from typing import TYPE_CHECKING, Any

from destack.utils.uuid import UUID

if TYPE_CHECKING:
    pass


class BinaryError(ValueError):
    """Base class for binary encoding/decoding errors."""


_EPOCH_DATE = date(1970, 1, 1)
_FLOAT_INF = float("inf")
_FLOAT_NINF = float("-inf")
_FLOAT_NAN = float("nan")


class BinaryWriter:
    """Write binary data in our custom encoding. Little-endian, varint, zigzag, etc."""

    def __init__(self) -> None:
        self.buffer = bytearray()

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

    # PrimitiveType.SINT8
    def write_sint8(self, value: int) -> None:
        """
        Write a signed 8-bit integer as raw byte.
        Size: 1 byte.
        """
        self.buffer.append(value & 0xFF)

    # PrimitiveType.SINT16
    def write_sint16(self, value: int) -> None:
        """
        Write a signed 16-bit integer using zigzag encoding then varint.
        Size: 1-3 bytes.
        """
        self._write_varint(self._write_zigzag(value))

    # PrimitiveType.SINT32
    def write_sint32(self, value: int) -> None:
        """
        Write a signed 32-bit integer using zigzag encoding then varint.
        Size: 1-5 bytes.
        """
        self._write_varint(self._write_zigzag(value))

    # PrimitiveType.SINT64
    def write_sint64(self, value: int) -> None:
        """
        Write a signed 64-bit integer using zigzag encoding then varint.
        Size: 1-10 bytes.
        """
        self._write_varint(self._write_zigzag(value))

    # PrimitiveType.SINT128
    def write_sint128(self, value: int) -> None:
        """
        Write a signed 128-bit integer using zigzag encoding then varint.
        Size: 1-19 bytes.
        """
        self._write_varint(self._write_zigzag(value))

    # PrimitiveType.UINT8
    def write_uint8(self, value: int) -> None:
        """
        Write an unsigned 8-bit integer as raw byte.
        Size: 1 byte.
        """
        self.buffer.append(value & 0xFF)

    # PrimitiveType.UINT16
    def write_uint16(self, value: int) -> None:
        """
        Write an unsigned 16-bit integer using varint encoding.
        Size: 1-3 bytes.
        """
        self._write_varint(value)

    # PrimitiveType.UINT32
    def write_uint32(self, value: int) -> None:
        """
        Write an unsigned 32-bit integer using varint encoding.
        Size: 1-5 bytes.
        """
        self._write_varint(value)

    # PrimitiveType.UINT64
    def write_uint64(self, value: int) -> None:
        """
        Write an unsigned 64-bit integer using varint encoding.
        Size: 1-10 bytes.
        """
        self._write_varint(value)

    # PrimitiveType.UINT128
    def write_uint128(self, value: int) -> None:
        """
        Write an unsigned 128-bit integer using varint encoding.
        Size: 1-19 bytes.
        """
        self._write_varint(value)

    # PrimitiveType.FLOAT16
    def write_float16(self, value: float) -> None:
        """
        Write a 16-bit float with flag byte:
          - 0: zero (1 byte)
          - 1: negative zero (1 byte)
          - 2: positive infinity (1 byte)
          - 3: negative infinity (1 byte)
          - 4: NaN (1 byte)
          - 5: sint8 for integers (2 bytes)
          - 6: little-endian float16 for others (3 bytes)
        """
        if value == 0.0:
            if math.copysign(1.0, value) == -1.0:
                self.buffer.append(1)  # negative zero
            else:
                self.buffer.append(0)  # positive zero
        elif value == _FLOAT_INF:
            self.buffer.append(2)  # positive infinity
        elif value == _FLOAT_NINF:
            self.buffer.append(3)  # negative infinity
        elif value != value:  # NaN check
            self.buffer.append(4)  # NaN
        elif value == float(int(value)) and -(2**7) <= value <= 2**7:
            self.buffer.append(5)
            self.write_sint8(int(value))
        else:
            self.buffer.append(6)
            self.buffer.extend(struct.pack("<e", value))

    # PrimitiveType.FLOAT32
    def write_float32(self, value: float) -> None:
        """
        Write a 32-bit float with flag byte:
          - 0: zero (1 byte)
          - 1: negative zero (1 byte)
          - 2: positive infinity (1 byte)
          - 3: negative infinity (1 byte)
          - 4: NaN (1 byte)
          - 5: sint16 for integers (3 bytes)
          - 6: little-endian float32 for others (5 bytes)
        """
        if value == 0.0:
            if math.copysign(1.0, value) == -1.0:
                self.buffer.append(1)  # negative zero
            else:
                self.buffer.append(0)  # positive zero
        elif value == _FLOAT_INF:
            self.buffer.append(2)  # positive infinity
        elif value == _FLOAT_NINF:
            self.buffer.append(3)  # negative infinity
        elif value != value:  # NaN check
            self.buffer.append(4)  # NaN
        elif value == float(int(value)) and -(2**15) <= value <= 2**15:
            self.buffer.append(5)
            self.write_sint16(int(value))
        else:
            self.buffer.append(6)
            self.buffer.extend(struct.pack("<f", value))

    # PrimitiveType.FLOAT64
    def write_float64(self, value: float) -> None:
        """
        Write a 64-bit float with flag byte:
          - 0: zero (1 byte)
          - 1: negative zero (1 byte)
          - 2: positive infinity (1 byte)
          - 3: negative infinity (1 byte)
          - 4: NaN (1 byte)
          - 5: sint32 for integers (6 bytes)
          - 6: little-endian float64 for others (9 bytes)
        """
        if value == 0.0:
            if math.copysign(1.0, value) == -1.0:
                self.buffer.append(1)  # negative zero
            else:
                self.buffer.append(0)  # positive zero
        elif value == _FLOAT_INF:
            self.buffer.append(2)  # positive infinity
        elif value == _FLOAT_NINF:
            self.buffer.append(3)  # negative infinity
        elif value != value:  # NaN check
            self.buffer.append(4)  # NaN
        elif value == float(int(value)) and -(2**31) <= value <= 2**31:
            # can be represented exactly as an integer
            self.buffer.append(5)
            self.write_sint32(int(value))
        else:
            self.buffer.append(6)
            self.buffer.extend(struct.pack("<d", value))

    # PrimitiveType.DATETIME
    def write_datetime(self, value: "datetime") -> None:
        """
        Write a datetime as zigzag-encoded varint of microseconds since epoch.
        Size: 1-10 bytes.
        """
        # ensure UTC timezone
        if value.tzinfo is None:
            value = value.replace(tzinfo=UTC)
        micros = int(value.timestamp() * 1_000_000)
        self._write_varint(self._write_zigzag(micros))

    # PrimitiveType.DATE
    def write_date(self, value: "date") -> None:
        """
        Write a date as zigzag-encoded varint of days since epoch (1970-01-01).
        Size: 1-5 bytes.
        """
        days = (value - _EPOCH_DATE).days
        self._write_varint(self._write_zigzag(days))

    # PrimitiveType.TIME
    def write_time(self, value: "time") -> None:
        """
        Write a time as varint of microseconds since midnight.
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
        Size: 1-10 bytes.
        """
        micros = round(value.total_seconds() * 1_000_000)
        self._write_varint(self._write_zigzag(micros))

    # PrimitiveType.STRING
    def write_string(self, value: str) -> None:
        """
        Write a UTF-8 string with varint length prefix followed by UTF-8 bytes.
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
        Size: 1-5 bytes (length) + data length.
        """
        self._write_varint(len(value))
        self.buffer.extend(value)

    # PrimitiveType.JSON
    def write_json(self, value: "Any") -> None:
        """
        Write JSON as compact UTF-8 string with varint length prefix.
        Size: 1-5 bytes (length) + JSON string length in bytes.
        """
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
        """
        Read a boolean from 1 byte (0 for false, non-zero for true).
        Size: 1 byte.
        """
        if self.pos >= len(self.buffer):
            raise BinaryError(f"unexpected end of buffer at {self.pos}")
        value = self.buffer[self.pos]
        self.pos += 1
        return value != 0

    # PrimitiveType.SINT8
    def read_sint8(self) -> int:
        """
        Read a signed 8-bit integer from raw byte with sign extension.
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

    # PrimitiveType.SINT16
    def read_sint16(self) -> int:
        """
        Read a signed 16-bit integer from zigzag-decoded varint.
        Size: 1-3 bytes.
        """
        return self._zigzag_decode(self._read_varint())

    # PrimitiveType.SINT32
    def read_sint32(self) -> int:
        """
        Read a signed 32-bit integer from zigzag-decoded varint.
        Size: 1-5 bytes.
        """
        return self._zigzag_decode(self._read_varint())

    # PrimitiveType.SINT64
    def read_sint64(self) -> int:
        """
        Read a signed 64-bit integer from zigzag-decoded varint.
        Size: 1-10 bytes.
        """
        return self._zigzag_decode(self._read_varint())

    # PrimitiveType.SINT128
    def read_sint128(self) -> int:
        """
        Read a signed 128-bit integer from zigzag-decoded varint.
        Size: 1-19 bytes.
        """
        return self._zigzag_decode(self._read_varint())

    # PrimitiveType.UINT8
    def read_uint8(self) -> int:
        """
        Read an unsigned 8-bit integer from raw byte.
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
        Size: 1-3 bytes.
        """
        return self._read_varint()

    # PrimitiveType.UINT32
    def read_uint32(self) -> int:
        """
        Read an unsigned 32-bit integer from varint.
        Size: 1-5 bytes.
        """
        return self._read_varint()

    # PrimitiveType.UINT64
    def read_uint64(self) -> int:
        """
        Read an unsigned 64-bit integer from varint.
        Size: 1-10 bytes.
        """
        return self._read_varint()

    # PrimitiveType.UINT128
    def read_uint128(self) -> int:
        """
        Read an unsigned 128-bit integer from varint.
        Size: 1-19 bytes.
        """
        return self._read_varint()

    # PrimitiveType.FLOAT16
    def read_float16(self) -> float:
        """
        Read a 16-bit float from flag byte + data:
          - 0: zero (1 byte)
          - 1: negative zero (1 byte)
          - 2: positive infinity (1 byte)
          - 3: negative infinity (1 byte)
          - 4: NaN (1 byte)
          - 5: sint8 for integers (2 bytes)
          - 6: little-endian float16 for others (3 bytes)
        """
        if self.pos >= len(self.buffer):
            raise BinaryError(f"unexpected end of buffer at {self.pos}")

        flag = self.buffer[self.pos]
        self.pos += 1

        if flag == 0:
            return 0.0
        elif flag == 1:
            return -0.0
        elif flag == 2:
            return _FLOAT_INF
        elif flag == 3:
            return _FLOAT_NINF
        elif flag == 4:
            return _FLOAT_NAN
        elif flag == 5:
            return float(self.read_sint8())
        elif flag == 6:
            if self.pos + 2 > len(self.buffer):
                raise BinaryError(f"unexpected end of buffer at {self.pos}")
            value = struct.unpack("<e", self.buffer[self.pos : self.pos + 2])[0]
            self.pos += 2
            return value
        else:
            raise BinaryError(f"invalid float16 encoding flag at {self.pos}: {flag}")

    # PrimitiveType.FLOAT32
    def read_float32(self) -> float:
        """
        Read a 32-bit float from flag byte + data:
          - 0: zero (1 byte)
          - 1: negative zero (1 byte)
          - 2: positive infinity (1 byte)
          - 3: negative infinity (1 byte)
          - 4: NaN (1 byte)
          - 5: sint16 for integers (3 bytes)
          - 6: little-endian float32 for others (5 bytes)
        """
        if self.pos >= len(self.buffer):
            raise BinaryError(f"unexpected end of buffer at {self.pos}")

        flag = self.buffer[self.pos]
        self.pos += 1

        if flag == 0:
            return 0.0
        elif flag == 1:
            return -0.0
        elif flag == 2:
            return _FLOAT_INF
        elif flag == 3:
            return _FLOAT_NINF
        elif flag == 4:
            return _FLOAT_NAN
        elif flag == 5:
            return float(self.read_sint16())
        elif flag == 6:
            if self.pos + 4 > len(self.buffer):
                raise BinaryError(f"unexpected end of buffer at {self.pos}")
            value = struct.unpack("<f", self.buffer[self.pos : self.pos + 4])[0]
            self.pos += 4
            return value
        else:
            raise BinaryError(f"invalid float32 encoding flag at {self.pos}: {flag}")

    # PrimitiveType.FLOAT64
    def read_float64(self) -> float:
        """
        Read a 64-bit float from flag byte + data:
          - 0: zero (1 byte)
          - 1: negative zero (1 byte)
          - 2: positive infinity (1 byte)
          - 3: negative infinity (1 byte)
          - 4: NaN (1 byte)
          - 5: sint32 for integers (6 bytes)
          - 6: little-endian float64 for others (9 bytes)
        """
        if self.pos >= len(self.buffer):
            raise BinaryError(f"unexpected end of buffer at {self.pos}")

        flag = self.buffer[self.pos]
        self.pos += 1

        if flag == 0:
            return 0.0
        elif flag == 1:
            return -0.0
        elif flag == 2:
            return _FLOAT_INF
        elif flag == 3:
            return _FLOAT_NINF
        elif flag == 4:
            return _FLOAT_NAN
        elif flag == 5:
            return float(self.read_sint32())
        elif flag == 6:
            if self.pos + 8 > len(self.buffer):
                raise BinaryError(f"unexpected end of buffer at {self.pos}")
            value = struct.unpack("<d", self.buffer[self.pos : self.pos + 8])[0]
            self.pos += 8
            return value
        else:
            raise BinaryError(f"invalid float64 encoding flag at {self.pos}: {flag}")

    # PrimitiveType.DATETIME
    def read_datetime(self) -> datetime:
        """
        Read a datetime from zigzag-decoded varint of microseconds since epoch.
        Size: 1-10 bytes.
        """
        micros = self._zigzag_decode(self._read_varint())
        return datetime.fromtimestamp(micros / 1_000_000, tz=UTC)

    # PrimitiveType.DATE
    def read_date(self) -> date:
        """
        Read a date from zigzag-decoded varint of days since epoch (1970-01-01).
        Size: 1-5 bytes.
        """
        days = self._zigzag_decode(self._read_varint())
        return _EPOCH_DATE + timedelta(days=days)

    # PrimitiveType.TIME
    def read_time(self) -> time:
        """
        Read a time from varint of microseconds since midnight.
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
        Size: 1-10 bytes.
        """
        micros = self._zigzag_decode(self._read_varint())
        return timedelta(microseconds=micros)

    # PrimitiveType.STRING
    def read_string(self) -> str:
        """
        Read a UTF-8 string from varint length prefix + UTF-8 bytes.
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
        Read JSON from UTF-8 string with varint length prefix.
        Size: 1-5 bytes (length) + JSON string length in bytes.
        """
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
