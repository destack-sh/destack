import struct
from datetime import UTC
from typing import Any

from destack.language.core import (
    Bytes,
    Date,
    Datetime,
    Duration,
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
    Universe,
    builtin_handle,
    builtin_method,
    builtin_property_runtime,
)
from destack.utils.uuid import UUID


@builtin_handle(HandleType.HASHER)
class Hasher(Handle):
    """Hash primitive values efficiently with a 32-bit integer."""

    # use fnv-1a 32-bit offset basis for consistent initialization
    buffer: Int32 = builtin_property_runtime(401, default=0x811C9DC5)

    @builtin_method(80)
    def digest(self) -> Int32:
        """Get the hash as a 32-bit integer."""
        return self.buffer

    @builtin_method(81)
    def reset(self) -> None:
        """Reset the hasher."""
        self.buffer = 0x811C9DC5

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

    # PrimitiveType.NONE
    @builtin_method(101)
    def hash_none(self) -> None:
        """
        Hash a None value.
        """
        self._mix_byte(0)

    # PrimitiveType.BOOLEAN
    @builtin_method(102)
    def hash_bool(self, value: bool) -> None:
        """
        Hash a boolean as 1 byte.
        """
        self._mix_byte(1 if value else 0)

    # PrimitiveType.INT8
    @builtin_method(103)
    def hash_int8(self, value: Int8) -> None:
        """
        Hash a signed 8-bit integer as raw byte.
        Range: -2^7 to 2^7-1.
        """
        self._mix_byte(value & 0xFF)

    # PrimitiveType.INT16
    @builtin_method(104)
    def hash_int16(self, value: Int16) -> None:
        """
        Hash a signed 16-bit integer using zigzag encoding then varint.
        Range: -2^15 to 2^15-1.
        """
        zigzag = self._zigzag_encode(value)
        while zigzag >= 0x80:
            self._mix_byte((zigzag & 0x7F) | 0x80)
            zigzag >>= 7
        self._mix_byte(zigzag & 0x7F)

    # PrimitiveType.INT32
    @builtin_method(105)
    def hash_int32(self, value: Int32) -> None:
        """
        Hash a signed 32-bit integer using zigzag encoding then varint.
        Range: -2^31 to 2^31-1.
        """
        zigzag = self._zigzag_encode(value)
        while zigzag >= 0x80:
            self._mix_byte((zigzag & 0x7F) | 0x80)
            zigzag >>= 7
        self._mix_byte(zigzag & 0x7F)

    # PrimitiveType.INT64
    @builtin_method(106)
    def hash_int64(self, value: Int64) -> None:
        """
        Hash a signed 64-bit integer using zigzag encoding then varint.
        Range: -2^63 to 2^63-1.
        """
        zigzag = self._zigzag_encode(value)
        while zigzag >= 0x80:
            self._mix_byte((zigzag & 0x7F) | 0x80)
            zigzag >>= 7
        self._mix_byte(zigzag & 0x7F)

    # PrimitiveType.INT128
    @builtin_method(107)
    def hash_int128(self, value: Int128) -> None:
        """
        Hash a signed 128-bit integer using zigzag encoding then varint.
        Range: -2^127 to 2^127-1.
        """
        zigzag = self._zigzag_encode(value)
        while zigzag >= 0x80:
            self._mix_byte((zigzag & 0x7F) | 0x80)
            zigzag >>= 7
        self._mix_byte(zigzag & 0x7F)

    # PrimitiveType.UINT8
    @builtin_method(110)
    def hash_uint8(self, value: UInt8) -> None:
        """
        Hash an unsigned 8-bit integer as raw byte.
        Range: 0 to 2^8-1.
        """
        self._mix_byte(value)

    # PrimitiveType.UINT16
    @builtin_method(111)
    def hash_uint16(self, value: UInt16) -> None:
        """
        Hash an unsigned 16-bit integer using varint encoding.
        Range: 0 to 2^16-1.
        """
        while value >= 0x80:
            self._mix_byte((value & 0x7F) | 0x80)
            value >>= 7
        self._mix_byte(value & 0x7F)

    # PrimitiveType.UINT32
    @builtin_method(112)
    def hash_uint32(self, value: UInt32) -> None:
        """
        Hash an unsigned 32-bit integer using varint encoding.
        Range: 0 to 2^32-1.
        """
        while value >= 0x80:
            self._mix_byte((value & 0x7F) | 0x80)
            value >>= 7
        self._mix_byte(value & 0x7F)

    # PrimitiveType.UINT64
    @builtin_method(113)
    def hash_uint64(self, value: UInt64) -> None:
        """
        Hash an unsigned 64-bit integer using varint encoding.
        Range: 0 to 2^64-1.
        """
        while value >= 0x80:
            self._mix_byte((value & 0x7F) | 0x80)
            value >>= 7
        self._mix_byte(value & 0x7F)

    # PrimitiveType.UINT128
    @builtin_method(114)
    def hash_uint128(self, value: UInt128) -> None:
        """
        Hash an unsigned 128-bit integer using varint encoding.
        Range: 0 to 2^128-1.
        """
        while value >= 0x80:
            self._mix_byte((value & 0x7F) | 0x80)
            value >>= 7
        self._mix_byte(value & 0x7F)

    # PrimitiveType.FLOAT16
    @builtin_method(122)
    def hash_float16(self, value: float) -> None:
        """
        Hash a fixed-length 16-bit float.
        Range: ±6.55e4 (half precision IEEE 754).
        """
        self._mix_bytes(struct.pack("<e", value))

    # PrimitiveType.FLOAT32
    @builtin_method(123)
    def hash_float32(self, value: float) -> None:
        """
        Hash a fixed-length 32-bit float.
        Range: ±3.4e38 (single precision IEEE 754).
        """
        self._mix_bytes(struct.pack("<f", value))

    # PrimitiveType.FLOAT64
    @builtin_method(124)
    def hash_float64(self, value: float) -> None:
        """
        Hash a fixed-length 64-bit float.
        Range: ±1.8e308 (double precision IEEE 754).
        """
        self._mix_bytes(struct.pack("<d", value))

    # PrimitiveType.DATETIME
    @builtin_method(140)
    def hash_datetime(self, value: Datetime) -> None:
        """
        Hash a datetime as zigzag-encoded varint of microseconds since epoch.
        Range: 0001-01-01 00:00:00 to 9999-12-31 23:59:59.999999 UTC.
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
        self.hash_int64(micros)

    # PrimitiveType.DATE
    @builtin_method(141)
    def hash_date(self, value: Date) -> None:
        """
        Hash a date as zigzag-encoded varint of days since epoch (0001-01-01).
        Range: 0001-01-01 to 9999-12-31.
        """
        days = (value - Universe.BEGINNING_OF_DATETIME.date()).days
        self.hash_int32(days)

    # PrimitiveType.TIME
    @builtin_method(142)
    def hash_time(self, value: Time) -> None:
        """
        Hash a time as varint of microseconds since midnight.
        Range: 00:00:00 to 23:59:59.999999.
        """
        micros = (
            value.hour * 3_600_000_000
            + value.minute * 60_000_000
            + value.second * 1_000_000
            + value.microsecond
        )
        self.hash_uint64(micros)

    # PrimitiveType.DURATION
    @builtin_method(143)
    def hash_duration(self, value: Duration) -> None:
        """
        Hash a duration as zigzag-encoded varint of microseconds.
        Range: -999,999,999 days to 999,999,999 days.
        """
        micros = value.days * 86_400_000_000 + value.seconds * 1_000_000 + value.microseconds
        self.hash_int64(micros)

    # PrimitiveType.STRING
    @builtin_method(150)
    def hash_string(self, value: String) -> None:
        """
        Hash a UTF-8 string with varint length prefix followed by UTF-8 bytes.
        Range: 0 to 2^32-1 bytes (UTF-8 encoded).
        """
        encoded = value.encode("utf-8")
        self.hash_bytes(encoded)

    # PrimitiveType.UUID
    @builtin_method(151)
    def hash_uuid(self, value: UUID) -> None:
        """
        Hash a UUID as 16 raw bytes.
        """
        self._mix_bytes(value.bytes)

    # PrimitiveType.BYTES
    @builtin_method(152)
    def hash_bytes(self, value: Bytes) -> None:
        """
        Hash raw bytes with varint length prefix followed by the bytes.
        Range: 0 to 2^32-1 bytes.
        """
        length = len(value)
        while length >= 0x80:
            self._mix_byte((length & 0x7F) | 0x80)
            length >>= 7
        self._mix_byte(length & 0x7F)
        self._mix_bytes(value)

    # PrimitiveType.JSON
    @builtin_method(155)
    def hash_json(self, value: Any) -> None:
        """
        Hash JSON.
        """
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
