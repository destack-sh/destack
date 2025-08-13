from typing import TYPE_CHECKING, Any

from ..builtin import (
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
    declare_method,
)

if TYPE_CHECKING:
    pass


class BinaryError(ValueError):
    """Base class for binary encoding/decoding errors."""


class BinaryWriter:
    """Write binary primitive values in some encoding."""

    def __init__(self) -> None:
        self.buffer = bytearray()

    def __len__(self) -> int:
        return len(self.buffer)

    @declare_method(80)
    def to_bytes(self) -> bytes:
        """Get the written bytes."""
        raise NotImplementedError

    # PrimitiveType.BOOLEAN
    @declare_method(102)
    def write_bool(self, value: bool) -> None:
        """Write a boolean."""
        raise NotImplementedError

    # PrimitiveType.INT8
    @declare_method(102)
    def write_int8(self, value: Int8) -> None:
        """Write a signed 8-bit integer."""
        raise NotImplementedError

    # PrimitiveType.INT16
    @declare_method(103)
    def write_int16(self, value: Int16) -> None:
        """Write a signed 16-bit integer."""
        raise NotImplementedError

    # PrimitiveType.INT32
    @declare_method(104)
    def write_int32(self, value: Int32) -> None:
        """Write a signed 32-bit integer."""
        raise NotImplementedError

    # PrimitiveType.INT64
    @declare_method(105)
    def write_int64(self, value: Int64) -> None:
        """Write a signed 64-bit integer."""
        raise NotImplementedError

    # PrimitiveType.INT128
    @declare_method(107)
    def write_int128(self, value: Int128) -> None:
        """Write a signed 128-bit integer."""
        raise NotImplementedError

    # PrimitiveType.UINT8
    @declare_method(110)
    def write_uint8(self, value: UInt8) -> None:
        """Write an unsigned 8-bit integer."""
        raise NotImplementedError

    # PrimitiveType.UINT16
    @declare_method(111)
    def write_uint16(self, value: UInt16) -> None:
        """Write an unsigned 16-bit integer."""
        raise NotImplementedError

    # PrimitiveType.UINT32
    @declare_method(112)
    def write_uint32(self, value: UInt32) -> None:
        """Write an unsigned 32-bit integer."""
        raise NotImplementedError

    # PrimitiveType.UINT64
    @declare_method(113)
    def write_uint64(self, value: UInt64) -> None:
        """Write an unsigned 64-bit integer."""
        raise NotImplementedError

    # PrimitiveType.UINT128
    @declare_method(114)
    def write_uint128(self, value: UInt128) -> None:
        """Write an unsigned 128-bit integer."""
        raise NotImplementedError

    # PrimitiveType.FLOAT16
    @declare_method(122)
    def write_float16(self, value: Float16) -> None:
        """Write a 16-bit float."""
        raise NotImplementedError

    # PrimitiveType.FLOAT32
    @declare_method(123)
    def write_float32(self, value: Float32) -> None:
        """Write a 32-bit float."""
        raise NotImplementedError

    # PrimitiveType.FLOAT64
    @declare_method(124)
    def write_float64(self, value: Float64) -> None:
        """Write a 64-bit float."""
        raise NotImplementedError

    # PrimitiveType.DATETIME
    @declare_method(140)
    def write_datetime(self, value: DateTime) -> None:
        """Write a datetime."""
        raise NotImplementedError

    # PrimitiveType.DATE
    @declare_method(141)
    def write_date(self, value: Date) -> None:
        """Write a date."""
        raise NotImplementedError

    # PrimitiveType.TIME
    @declare_method(142)
    def write_time(self, value: Time) -> None:
        """Write a time."""
        raise NotImplementedError

    # PrimitiveType.TIMESTAMP
    @declare_method(144)
    def write_timestamp(self, value: Timestamp) -> None:
        """Write a timestamp."""
        raise NotImplementedError

    # PrimitiveType.DURATION
    @declare_method(143)
    def write_duration(self, value: Duration) -> None:
        """Write a duration."""
        raise NotImplementedError

    # PrimitiveType.STRING
    @declare_method(150)
    def write_string(self, value: String) -> None:
        """Write a UTF-8 string."""
        raise NotImplementedError

    # PrimitiveType.CHARACTER
    @declare_method(153)
    def write_character(self, value: Character) -> None:
        """Write a single Unicode character."""
        raise NotImplementedError

    # PrimitiveType.UUID
    @declare_method(151)
    def write_uuid(self, value: UUID) -> None:
        """Write a UUID."""
        raise NotImplementedError

    # PrimitiveType.BYTES
    @declare_method(152)
    def write_bytes(self, value: Bytes) -> None:
        """Write raw bytes."""
        raise NotImplementedError

    # PrimitiveType.JSON
    @declare_method(155)
    def write_json(self, value: Any) -> None:
        """Write JSON."""
        raise NotImplementedError


class BinaryReader:
    """Read binary primitive values in some encoding."""

    def __init__(self, buffer: bytes) -> None:
        self.buffer = buffer
        self.pos: UInt32 = 0

    def __str__(self) -> str:
        return f"pos={self.pos}, remaining={self.remaining}"

    def __repr__(self) -> str:
        return f"<BinaryReader pos={self.pos}, remaining={self.remaining}>"

    @property
    def remaining(self) -> int:
        """Number of bytes remaining."""
        return len(self.buffer) - self.pos

    # PrimitiveType.BOOLEAN
    @declare_method(102)
    def read_bool(self) -> bool:
        """Read a boolean."""
        raise NotImplementedError

    # PrimitiveType.INT8
    @declare_method(103)
    def read_int8(self) -> Int8:
        """Read a signed 8-bit integer."""
        raise NotImplementedError

    # PrimitiveType.INT16
    @declare_method(104)
    def read_int16(self) -> Int16:
        """Read a signed 16-bit integer."""
        raise NotImplementedError

    # PrimitiveType.INT32
    @declare_method(105)
    def read_int32(self) -> Int32:
        """Read a signed 32-bit integer."""
        raise NotImplementedError

    # PrimitiveType.INT64
    @declare_method(106)
    def read_int64(self) -> Int64:
        """Read a signed 64-bit integer."""
        raise NotImplementedError

    # PrimitiveType.INT128
    @declare_method(107)
    def read_int128(self) -> Int128:
        """Read a signed 128-bit integer."""
        raise NotImplementedError

    # PrimitiveType.UINT8
    @declare_method(110)
    def read_uint8(self) -> UInt8:
        """Read an unsigned 8-bit integer."""
        raise NotImplementedError

    # PrimitiveType.UINT16
    @declare_method(111)
    def read_uint16(self) -> UInt16:
        """Read an unsigned 16-bit integer."""
        raise NotImplementedError

    # PrimitiveType.UINT32
    @declare_method(112)
    def read_uint32(self) -> UInt32:
        """Read an unsigned 32-bit integer."""
        raise NotImplementedError

    # PrimitiveType.UINT64
    @declare_method(113)
    def read_uint64(self) -> UInt64:
        """Read an unsigned 64-bit integer."""
        raise NotImplementedError

    # PrimitiveType.UINT128
    @declare_method(114)
    def read_uint128(self) -> UInt128:
        """Read an unsigned 128-bit integer."""
        raise NotImplementedError

    # PrimitiveType.FLOAT16
    @declare_method(122)
    def read_float16(self) -> Float16:
        """Read a 16-bit float."""
        raise NotImplementedError

    # PrimitiveType.FLOAT32
    @declare_method(123)
    def read_float32(self) -> Float32:
        """Read a 32-bit float."""
        raise NotImplementedError

    # PrimitiveType.FLOAT64
    @declare_method(124)
    def read_float64(self) -> Float64:
        """Read a 64-bit float."""
        raise NotImplementedError

    # PrimitiveType.DATETIME
    @declare_method(140)
    def read_datetime(self) -> DateTime:
        """Read a datetime."""
        raise NotImplementedError

    # PrimitiveType.DATE
    @declare_method(141)
    def read_date(self) -> Date:
        """Read a date."""
        raise NotImplementedError

    # PrimitiveType.TIME
    @declare_method(142)
    def read_time(self) -> Time:
        """Read a time."""
        raise NotImplementedError

    # PrimitiveType.TIMESTAMP
    @declare_method(144)
    def read_timestamp(self) -> Timestamp:
        """Read a timestamp."""
        raise NotImplementedError

    # PrimitiveType.DURATION
    @declare_method(143)
    def read_duration(self) -> Duration:
        """Read a duration."""
        raise NotImplementedError

    # PrimitiveType.STRING
    @declare_method(150)
    def read_string(self) -> String:
        """Read a UTF-8 string."""
        raise NotImplementedError

    # PrimitiveType.CHARACTER
    @declare_method(153)
    def read_character(self) -> Character:
        """Read a single Unicode character."""
        raise NotImplementedError

    # PrimitiveType.UUID
    @declare_method(151)
    def read_uuid(self) -> UUID:
        """Read a UUID."""
        raise NotImplementedError

    # PrimitiveType.BYTES
    @declare_method(152)
    def read_bytes(self) -> Bytes:
        """Read raw bytes."""
        raise NotImplementedError

    # PrimitiveType.JSON
    @declare_method(155)
    def read_json(self) -> Any:
        """Read JSON."""
        raise NotImplementedError
