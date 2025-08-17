import struct
from datetime import UTC
from typing import Any

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
    UInt8,
    UInt16,
    UInt32,
    UInt64,
    UInt128,
)
from ..universe import Universe


class Hasher:
    """Python Hasher."""

    def __init__(self) -> None:
        self.buffer: Int32 = 0x811C9DC5

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

    def hash_none(self) -> None:
        """Hash a None value."""
        self._mix_byte(0)

    def hash_bool(self, value: bool) -> None:
        """Hash a boolean as 1 byte."""
        self._mix_byte(1 if value else 0)

    def hash_int8(self, value: Int8) -> None:
        """Hash a signed 8-bit integer as raw byte."""
        self._mix_byte(value & 0xFF)

    def hash_int16(self, value: Int16) -> None:
        """Hash a signed 16-bit integer using zigzag encoding then varint."""
        zigzag = self._zigzag_encode(value)
        while zigzag >= 0x80:
            self._mix_byte((zigzag & 0x7F) | 0x80)
            zigzag >>= 7
        self._mix_byte(zigzag & 0x7F)

    def hash_int32(self, value: Int32) -> None:
        """Hash a signed 32-bit integer using zigzag encoding then varint."""
        zigzag = self._zigzag_encode(value)
        while zigzag >= 0x80:
            self._mix_byte((zigzag & 0x7F) | 0x80)
            zigzag >>= 7
        self._mix_byte(zigzag & 0x7F)

    def hash_int64(self, value: Int64) -> None:
        """Hash a signed 64-bit integer using zigzag encoding then varint."""
        zigzag = self._zigzag_encode(value)
        while zigzag >= 0x80:
            self._mix_byte((zigzag & 0x7F) | 0x80)
            zigzag >>= 7
        self._mix_byte(zigzag & 0x7F)

    def hash_int128(self, value: Int128) -> None:
        """Hash a signed 128-bit integer using zigzag encoding then varint."""
        zigzag = self._zigzag_encode(value)
        while zigzag >= 0x80:
            self._mix_byte((zigzag & 0x7F) | 0x80)
            zigzag >>= 7
        self._mix_byte(zigzag & 0x7F)

    def hash_uint8(self, value: UInt8) -> None:
        """Hash an unsigned 8-bit integer as raw byte."""
        self._mix_byte(value)

    def hash_uint16(self, value: UInt16) -> None:
        """Hash an unsigned 16-bit integer using varint encoding."""
        while value >= 0x80:
            self._mix_byte((value & 0x7F) | 0x80)
            value >>= 7
        self._mix_byte(value & 0x7F)

    def hash_uint32(self, value: UInt32) -> None:
        """Hash an unsigned 32-bit integer using varint encoding."""
        while value >= 0x80:
            self._mix_byte((value & 0x7F) | 0x80)
            value >>= 7
        self._mix_byte(value & 0x7F)

    def hash_uint64(self, value: UInt64) -> None:
        """Hash an unsigned 64-bit integer using varint encoding."""
        while value >= 0x80:
            self._mix_byte((value & 0x7F) | 0x80)
            value >>= 7
        self._mix_byte(value & 0x7F)

    def hash_uint128(self, value: UInt128) -> None:
        """Hash an unsigned 128-bit integer using varint encoding."""
        while value >= 0x80:
            self._mix_byte((value & 0x7F) | 0x80)
            value >>= 7
        self._mix_byte(value & 0x7F)

    def hash_float32(self, value: Float32) -> None:
        """Hash a fixed-length 32-bit float."""
        self._mix_bytes(struct.pack("<f", value))

    def hash_float64(self, value: Float64) -> None:
        """Hash a fixed-length 64-bit float."""
        self._mix_bytes(struct.pack("<d", value))

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

    def hash_date(self, value: Date) -> None:
        """Hash a date as zigzag-encoded varint of days since epoch."""
        days = (value - Universe.BEGINNING_OF_DATETIME.date()).days
        self.hash_int32(days)

    def hash_time(self, value: Time) -> None:
        """Hash a time as varint of microseconds since midnight."""
        micros = (
            value.hour * 3_600_000_000
            + value.minute * 60_000_000
            + value.second * 1_000_000
            + value.microsecond
        )
        self.hash_uint64(micros)

    def hash_duration(self, value: Duration) -> None:
        """Hash a duration as zigzag-encoded varint of microseconds."""
        micros = value.days * 86_400_000_000 + value.seconds * 1_000_000 + value.microseconds
        self.hash_int64(micros)

    def hash_string(self, value: String) -> None:
        """Hash a UTF-8 string with varint length prefix followed by UTF-8 bytes."""
        encoded = value.encode("utf-8")
        self.hash_bytes(encoded)

    def hash_character(self, value: Character) -> None:
        """Hash a single Unicode code point as fixed 4 bytes."""
        if len(value) != 1:
            raise ValueError(f"character must be length 1, got {len(value)}: {value!r}")
        self._mix_bytes(struct.pack("<I", ord(value)))

    def hash_uuid(self, value: UUID) -> None:
        """Hash a UUID as 16 raw bytes."""
        self._mix_bytes(value.bytes)

    def hash_bytes(self, value: Bytes) -> None:
        """Hash raw bytes with varint length prefix followed by the bytes."""
        length = len(value)
        while length >= 0x80:
            self._mix_byte((length & 0x7F) | 0x80)
            length >>= 7
        self._mix_byte(length & 0x7F)
        self._mix_bytes(value)

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

    def digest(self) -> Int32:
        """Get the hash as a 32-bit integer."""
        return self.buffer

    def reset(self) -> None:
        """Reset the hasher."""
        self.buffer = 0x811C9DC5
