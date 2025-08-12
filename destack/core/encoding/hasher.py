import struct
from datetime import UTC
from typing import Any, override

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


@declare_handle(HandleType.HASHER)
class Hasher(Handle):
    """Hash primitive values efficiently with a 32-bit integer."""

    # use fnv-1a 32-bit offset basis for consistent initialization
    buffer: Int32 = declare_property_runtime(401, default=0x811C9DC5)

    # PrimitiveType.NONE
    @declare_method(101)
    def hash_none(self) -> None:
        """Hash a None value."""
        raise NotImplementedError

    # PrimitiveType.BOOLEAN
    @declare_method(102)
    def hash_bool(self, value: bool) -> None:
        """Hash a boolean as 1 byte."""
        raise NotImplementedError

    # PrimitiveType.INT8
    @declare_method(103)
    def hash_int8(self, value: Int8) -> None:
        """Hash a signed 8-bit integer as raw byte."""
        raise NotImplementedError

    # PrimitiveType.INT16
    @declare_method(104)
    def hash_int16(self, value: Int16) -> None:
        """Hash a signed 16-bit integer using zigzag encoding then varint."""
        raise NotImplementedError

    # PrimitiveType.INT32
    @declare_method(105)
    def hash_int32(self, value: Int32) -> None:
        """Hash a signed 32-bit integer using zigzag encoding then varint."""
        raise NotImplementedError

    # PrimitiveType.INT64
    @declare_method(106)
    def hash_int64(self, value: Int64) -> None:
        """Hash a signed 64-bit integer using zigzag encoding then varint."""
        raise NotImplementedError

    # PrimitiveType.INT128
    @declare_method(107)
    def hash_int128(self, value: Int128) -> None:
        """Hash a signed 128-bit integer using zigzag encoding then varint."""
        raise NotImplementedError

    # PrimitiveType.UINT8
    @declare_method(110)
    def hash_uint8(self, value: UInt8) -> None:
        """Hash an unsigned 8-bit integer as raw byte."""
        raise NotImplementedError

    # PrimitiveType.UINT16
    @declare_method(111)
    def hash_uint16(self, value: UInt16) -> None:
        """Hash an unsigned 16-bit integer using varint encoding."""
        raise NotImplementedError

    # PrimitiveType.UINT32
    @declare_method(112)
    def hash_uint32(self, value: UInt32) -> None:
        """Hash an unsigned 32-bit integer using varint encoding."""
        raise NotImplementedError

    # PrimitiveType.UINT64
    @declare_method(113)
    def hash_uint64(self, value: UInt64) -> None:
        """Hash an unsigned 64-bit integer using varint encoding."""
        raise NotImplementedError

    # PrimitiveType.UINT128
    @declare_method(114)
    def hash_uint128(self, value: UInt128) -> None:
        """Hash an unsigned 128-bit integer using varint encoding."""
        raise NotImplementedError

    # PrimitiveType.FLOAT16
    @declare_method(122)
    def hash_float16(self, value: Float16) -> None:
        """Hash a fixed-length 16-bit float."""
        raise NotImplementedError

    # PrimitiveType.FLOAT32
    @declare_method(123)
    def hash_float32(self, value: Float32) -> None:
        """Hash a fixed-length 32-bit float."""
        raise NotImplementedError

    # PrimitiveType.FLOAT64
    @declare_method(124)
    def hash_float64(self, value: Float64) -> None:
        """Hash a fixed-length 64-bit float."""
        raise NotImplementedError

    # PrimitiveType.DATETIME
    @declare_method(140)
    def hash_datetime(self, value: DateTime) -> None:
        """Hash a datetime as zigzag-encoded varint of microseconds since epoch."""
        raise NotImplementedError

    # PrimitiveType.DATE
    @declare_method(141)
    def hash_date(self, value: Date) -> None:
        """Hash a date as zigzag-encoded varint of days since epoch."""
        raise NotImplementedError

    # PrimitiveType.TIME
    @declare_method(142)
    def hash_time(self, value: Time) -> None:
        """Hash a time as varint of microseconds since midnight."""
        raise NotImplementedError

    # PrimitiveType.DURATION
    @declare_method(143)
    def hash_duration(self, value: Duration) -> None:
        """Hash a duration as zigzag-encoded varint of microseconds."""
        raise NotImplementedError

    # PrimitiveType.STRING
    @declare_method(150)
    def hash_string(self, value: String) -> None:
        """Hash a UTF-8 string with varint length prefix followed by UTF-8 bytes."""
        raise NotImplementedError

    # PrimitiveType.CHARACTER
    @declare_method(153)
    def hash_character(self, value: Character) -> None:
        """Hash a single Unicode code point as fixed 4 bytes."""
        raise NotImplementedError

    # PrimitiveType.UUID
    @declare_method(151)
    def hash_uuid(self, value: UUID) -> None:
        """Hash a UUID as 16 raw bytes."""
        raise NotImplementedError

    # PrimitiveType.BYTES
    @declare_method(152)
    def hash_bytes(self, value: Bytes) -> None:
        """Hash raw bytes with varint length prefix followed by the bytes."""
        raise NotImplementedError

    # PrimitiveType.JSON
    @declare_method(155)
    def hash_json(self, value: Any) -> None:
        """Hash JSON."""
        raise NotImplementedError

    @declare_method(190)
    def digest(self) -> Int32:
        """Get the hash as a 32-bit integer."""
        raise NotImplementedError

    @declare_method(191)
    def reset(self) -> None:
        """Reset the hasher."""
        raise NotImplementedError


class MemoryHasher(Hasher):
    """Python implementation of the Hasher interface."""

    def _mix_byte(self, byte: int) -> None:
        """Mix a single byte into the hash using FNV-1a algorithm."""
        # xor then multiply by prime
        self.buffer ^= byte & 0xFF
        self.buffer = (self.buffer * 0x01000193) & 0xFFFFFFFF

    def _mix_bytes(self, data: bytes) -> None:
        """Mix multiple bytes into the hash."""
        for byte in data:
            self._mix_byte(byte)

    def _mix_int32(self, value: int) -> None:
        """Mix a 32-bit integer efficiently."""
        # mix in little-endian order for consistency
        self._mix_byte(value & 0xFF)
        self._mix_byte((value >> 8) & 0xFF)
        self._mix_byte((value >> 16) & 0xFF)
        self._mix_byte((value >> 24) & 0xFF)

    def _zigzag_encode(self, value: int) -> int:
        """Encode signed integer to unsigned using zigzag encoding."""
        if value >= 0:
            return value << 1
        else:
            return ((-value) << 1) - 1

    @override
    def hash_none(self) -> None:
        """Hash a None value."""
        self._mix_byte(0)

    @override
    def hash_bool(self, value: bool) -> None:
        """Hash a boolean as 1 byte."""
        self._mix_byte(1 if value else 0)

    @override
    def hash_int8(self, value: Int8) -> None:
        """Hash a signed 8-bit integer as raw byte."""
        self._mix_byte(value & 0xFF)

    @override
    def hash_int16(self, value: Int16) -> None:
        """Hash a signed 16-bit integer using zigzag encoding then varint."""
        zigzag = self._zigzag_encode(value)
        while zigzag >= 0x80:
            self._mix_byte((zigzag & 0x7F) | 0x80)
            zigzag >>= 7
        self._mix_byte(zigzag & 0x7F)

    @override
    def hash_int32(self, value: Int32) -> None:
        """Hash a signed 32-bit integer using zigzag encoding then varint."""
        zigzag = self._zigzag_encode(value)
        while zigzag >= 0x80:
            self._mix_byte((zigzag & 0x7F) | 0x80)
            zigzag >>= 7
        self._mix_byte(zigzag & 0x7F)

    @override
    def hash_int64(self, value: Int64) -> None:
        """Hash a signed 64-bit integer using zigzag encoding then varint."""
        zigzag = self._zigzag_encode(value)
        while zigzag >= 0x80:
            self._mix_byte((zigzag & 0x7F) | 0x80)
            zigzag >>= 7
        self._mix_byte(zigzag & 0x7F)

    @override
    def hash_int128(self, value: Int128) -> None:
        """Hash a signed 128-bit integer using zigzag encoding then varint."""
        zigzag = self._zigzag_encode(value)
        while zigzag >= 0x80:
            self._mix_byte((zigzag & 0x7F) | 0x80)
            zigzag >>= 7
        self._mix_byte(zigzag & 0x7F)

    @override
    def hash_uint8(self, value: UInt8) -> None:
        """Hash an unsigned 8-bit integer as raw byte."""
        self._mix_byte(value)

    @override
    def hash_uint16(self, value: UInt16) -> None:
        """Hash an unsigned 16-bit integer using varint encoding."""
        while value >= 0x80:
            self._mix_byte((value & 0x7F) | 0x80)
            value >>= 7
        self._mix_byte(value & 0x7F)

    @override
    def hash_uint32(self, value: UInt32) -> None:
        """Hash an unsigned 32-bit integer using varint encoding."""
        while value >= 0x80:
            self._mix_byte((value & 0x7F) | 0x80)
            value >>= 7
        self._mix_byte(value & 0x7F)

    @override
    def hash_uint64(self, value: UInt64) -> None:
        """Hash an unsigned 64-bit integer using varint encoding."""
        while value >= 0x80:
            self._mix_byte((value & 0x7F) | 0x80)
            value >>= 7
        self._mix_byte(value & 0x7F)

    @override
    def hash_uint128(self, value: UInt128) -> None:
        """Hash an unsigned 128-bit integer using varint encoding."""
        while value >= 0x80:
            self._mix_byte((value & 0x7F) | 0x80)
            value >>= 7
        self._mix_byte(value & 0x7F)

    @override
    def hash_float16(self, value: Float16) -> None:
        """Hash a fixed-length 16-bit float."""
        self._mix_bytes(struct.pack("<e", value))

    @override
    def hash_float32(self, value: Float32) -> None:
        """Hash a fixed-length 32-bit float."""
        self._mix_bytes(struct.pack("<f", value))

    @override
    def hash_float64(self, value: Float64) -> None:
        """Hash a fixed-length 64-bit float."""
        self._mix_bytes(struct.pack("<d", value))

    @override
    def hash_datetime(self, value: DateTime) -> None:
        """Hash a datetime as zigzag-encoded varint of microseconds since epoch."""
        if value.tzinfo is None:
            value = value.replace(tzinfo=UTC)
        days = (value.date() - Universe.BEGINNING_OF_DATETIME.date()).days
        time_micros = (
            value.hour * 3_600_000_000
            + value.minute * 60_000_000
            + value.second * 1_000_000
            + value.microsecond
        )
        micros = days * 86_400_000_000 + time_micros
        self.hash_int64(micros)

    @override
    def hash_date(self, value: Date) -> None:
        """Hash a date as zigzag-encoded varint of days since epoch."""
        days = (value - Universe.BEGINNING_OF_DATETIME.date()).days
        self.hash_int32(days)

    @override
    def hash_time(self, value: Time) -> None:
        """Hash a time as varint of microseconds since midnight."""
        micros = (
            value.hour * 3_600_000_000
            + value.minute * 60_000_000
            + value.second * 1_000_000
            + value.microsecond
        )
        self.hash_uint64(micros)

    @override
    def hash_duration(self, value: Duration) -> None:
        """Hash a duration as zigzag-encoded varint of microseconds."""
        micros = value.days * 86_400_000_000 + value.seconds * 1_000_000 + value.microseconds
        self.hash_int64(micros)

    @override
    def hash_string(self, value: String) -> None:
        """Hash a UTF-8 string with varint length prefix followed by UTF-8 bytes."""
        encoded = value.encode("utf-8")
        self.hash_bytes(encoded)

    @override
    def hash_character(self, value: Character) -> None:
        """Hash a single Unicode code point as fixed 4 bytes."""
        if len(value) != 1:
            raise ValueError(f"character must be length 1, got {len(value)}: {value!r}")
        self._mix_bytes(struct.pack("<I", ord(value)))

    @override
    def hash_uuid(self, value: UUID) -> None:
        """Hash a UUID as 16 raw bytes."""
        self._mix_bytes(value.bytes)

    @override
    def hash_bytes(self, value: Bytes) -> None:
        """Hash raw bytes with varint length prefix followed by the bytes."""
        length = len(value)
        while length >= 0x80:
            self._mix_byte((length & 0x7F) | 0x80)
            length >>= 7
        self._mix_byte(length & 0x7F)
        self._mix_bytes(value)

    @override
    def hash_json(self, value: Any) -> None:
        """Hash JSON."""
        if value is None:
            self._mix_byte(0)
        elif value is False:
            self._mix_byte(1)
        elif value is True:
            self._mix_byte(2)
        elif isinstance(value, int):
            self._mix_byte(3)
            self.hash_int64(value)
        elif isinstance(value, float):
            self._mix_byte(4)
            self.hash_float64(value)
        elif isinstance(value, str):
            self._mix_byte(5)
            self.hash_string(value)
        elif isinstance(value, list):
            self._mix_byte(6)
            length = len(value)
            while length >= 0x80:
                self._mix_byte((length & 0x7F) | 0x80)
                length >>= 7
            self._mix_byte(length & 0x7F)
            for item in value:
                self.hash_json(item)
        elif isinstance(value, dict):
            self._mix_byte(7)
            length = len(value)
            while length >= 0x80:
                self._mix_byte((length & 0x7F) | 0x80)
                length >>= 7
            self._mix_byte(length & 0x7F)
            # sort keys for stable hashing
            for key in sorted(value.keys()):
                if not isinstance(key, str):
                    raise ValueError(f"invalid JSON key type '{type(key)}': {key!r}")
                self.hash_string(key)
                self.hash_json(value[key])
        else:
            raise ValueError(f"invalid JSON type '{type(value)}': {value!r}")

    @override
    def digest(self) -> Int32:
        """Get the hash as a 32-bit integer."""
        return self.buffer

    @override
    def reset(self) -> None:
        """Reset the hasher."""
        self.buffer = 0x811C9DC5
