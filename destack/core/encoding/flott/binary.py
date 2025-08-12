import struct
from datetime import UTC, datetime, time, timedelta
from typing import TYPE_CHECKING, override

from ...builtin import (
    Bytes,
    Date,
    DateTime,
    Duration,
    Int16,
    Int32,
    Int64,
    Int128,
    String,
    Time,
    Timestamp,
    UInt16,
    UInt32,
    UInt64,
    UInt128,
)
from ..kompakt import KompaktBinaryReader, KompaktBinaryWriter

if TYPE_CHECKING:
    pass


class BinaryError(ValueError):
    """Base class for binary encoding/decoding errors."""


class FlottBinaryWriter(KompaktBinaryWriter):
    """Write binary primitive values using FLOTT encoding."""

    @override
    def write_int16(self, value: Int16) -> None:
        """
        Write a signed 16-bit integer as fixed 2 bytes (little-endian).
        Range: -2^15 to 2^15-1.
        Size: 2 bytes.
        """
        self.buffer.extend(struct.pack("<h", value))

    @override
    def write_int32(self, value: Int32) -> None:
        """
        Write a signed 32-bit integer as fixed 4 bytes (little-endian).
        Range: -2^31 to 2^31-1.
        Size: 4 bytes.
        """
        self.buffer.extend(struct.pack("<i", value))

    @override
    def write_int64(self, value: Int64) -> None:
        """
        Write a signed 64-bit integer as fixed 8 bytes (little-endian).
        Range: -2^63 to 2^63-1.
        Size: 8 bytes.
        """
        self.buffer.extend(struct.pack("<q", value))

    @override
    def write_int128(self, value: Int128) -> None:
        """
        Write a signed 128-bit integer as fixed 16 bytes (little-endian).
        Range: -2^127 to 2^127-1.
        Size: 16 bytes.
        """
        # split into two 64-bit parts
        if value < 0:
            # handle negative numbers with two's complement
            value = (1 << 128) + value
        low = value & 0xFFFFFFFFFFFFFFFF
        high = (value >> 64) & 0xFFFFFFFFFFFFFFFF
        self.buffer.extend(struct.pack("<QQ", low, high))

    @override
    def write_uint16(self, value: UInt16) -> None:
        """
        Write an unsigned 16-bit integer as fixed 2 bytes (little-endian).
        Range: 0 to 2^16-1.
        Size: 2 bytes.
        """
        self.buffer.extend(struct.pack("<H", value))

    @override
    def write_uint32(self, value: UInt32) -> None:
        """
        Write an unsigned 32-bit integer as fixed 4 bytes (little-endian).
        Range: 0 to 2^32-1.
        Size: 4 bytes.
        """
        self.buffer.extend(struct.pack("<I", value))

    @override
    def write_uint64(self, value: UInt64) -> None:
        """
        Write an unsigned 64-bit integer as fixed 8 bytes (little-endian).
        Range: 0 to 2^64-1.
        Size: 8 bytes.
        """
        self.buffer.extend(struct.pack("<Q", value))

    @override
    def write_uint128(self, value: UInt128) -> None:
        """
        Write an unsigned 128-bit integer as fixed 16 bytes (little-endian).
        Range: 0 to 2^128-1.
        Size: 16 bytes.
        """
        # split into two 64-bit parts
        low = value & 0xFFFFFFFFFFFFFFFF
        high = (value >> 64) & 0xFFFFFFFFFFFFFFFF
        self.buffer.extend(struct.pack("<QQ", low, high))

    @override
    def write_datetime(self, value: DateTime) -> None:
        """
        Write a datetime as fixed 8 bytes of signed 64-bit microseconds since Unix epoch (UTC).
        Range: approximately ±292,277 years.
        Size: 8 bytes.
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
        self.buffer.extend(struct.pack("<q", micros))

    @override
    def write_date(self, value: Date) -> None:
        """
        Write a date as fixed 8 bytes of signed 64-bit days since Unix epoch (1970-01-01).
        Range: full Python date range supported by calculations.
        Size: 8 bytes.
        """
        epoch = datetime(1970, 1, 1, tzinfo=UTC)
        days = (value - epoch.date()).days
        self.buffer.extend(struct.pack("<q", days))

    @override
    def write_time(self, value: Time) -> None:
        """
        Write a time as fixed 8 bytes of unsigned 64-bit nanoseconds since midnight.
        Range: 00:00:00.000000000 to 23:59:59.999999999.
        Size: 8 bytes.
        """
        nanos = (
            value.hour * 3_600_000_000_000
            + value.minute * 60_000_000_000
            + value.second * 1_000_000_000
            + value.microsecond * 1_000
        )
        self.buffer.extend(struct.pack("<Q", nanos))

    @override
    def write_timestamp(self, value: Timestamp) -> None:
        """
        Write a timestamp as fixed 8 bytes of unsigned 64-bit nanoseconds since Unix epoch (UTC).
        Range: 1970-01-01 to 2554-07-21 UTC.
        Size: 8 bytes.
        """
        self.buffer.extend(struct.pack("<Q", value))

    @override
    def write_duration(self, value: Duration) -> None:
        """
        Write a duration as fixed 8 bytes of signed 64-bit nanoseconds.
        Range: approximately ±292,277 years.
        Size: 8 bytes.
        """
        # use integer math to avoid precision issues with large durations
        micros = value.days * 86_400_000_000 + value.seconds * 1_000_000 + value.microseconds
        nanos = micros * 1_000
        self.buffer.extend(struct.pack("<q", nanos))

    @override
    def write_string(self, value: String) -> None:
        """
        Write a UTF-8 string with fixed 4-byte length prefix followed by UTF-8 bytes.
        Range: 0 to 2^32-1 bytes (UTF-8 encoded).
        Size: 4 bytes (length) + string length in bytes.
        """
        encoded = value.encode("utf-8")
        self.write_bytes(encoded)

    @override
    def write_bytes(self, value: Bytes) -> None:
        """
        Write raw bytes with fixed 4-byte length prefix followed by the bytes.
        Range: 0 to 2^32-1 bytes.
        Size: 4 bytes (length) + data length.
        """
        self.buffer.extend(struct.pack("<I", len(value)))
        self.buffer.extend(value)


class FlottBinaryReader(KompaktBinaryReader):
    """Read binary primitive values using FLOTT encoding."""

    @override
    def read_int16(self) -> Int16:
        """
        Read a signed 16-bit integer from fixed 2 bytes (little-endian).
        Range: -2^15 to 2^15-1.
        Size: 2 bytes.
        """
        if self.pos + 2 > len(self.buffer):
            raise BinaryError(f"unexpected end of buffer at {self.pos}")
        value = struct.unpack("<h", self.buffer[self.pos : self.pos + 2])[0]
        self.pos += 2
        return value

    @override
    def read_int32(self) -> Int32:
        """
        Read a signed 32-bit integer from fixed 4 bytes (little-endian).
        Range: -2^31 to 2^31-1.
        Size: 4 bytes.
        """
        if self.pos + 4 > len(self.buffer):
            raise BinaryError(f"unexpected end of buffer at {self.pos}")
        value = struct.unpack("<i", self.buffer[self.pos : self.pos + 4])[0]
        self.pos += 4
        return value

    @override
    def read_int64(self) -> Int64:
        """
        Read a signed 64-bit integer from fixed 8 bytes (little-endian).
        Range: -2^63 to 2^63-1.
        Size: 8 bytes.
        """
        if self.pos + 8 > len(self.buffer):
            raise BinaryError(f"unexpected end of buffer at {self.pos}")
        value = struct.unpack("<q", self.buffer[self.pos : self.pos + 8])[0]
        self.pos += 8
        return value

    @override
    def read_int128(self) -> Int128:
        """
        Read a signed 128-bit integer from fixed 16 bytes (little-endian).
        Range: -2^127 to 2^127-1.
        Size: 16 bytes.
        """
        if self.pos + 16 > len(self.buffer):
            raise BinaryError(f"unexpected end of buffer at {self.pos}")
        low, high = struct.unpack("<QQ", self.buffer[self.pos : self.pos + 16])
        self.pos += 16
        value = (high << 64) | low
        # handle two's complement for negative numbers
        if value >= (1 << 127):
            value -= 1 << 128
        return value

    @override
    def read_uint16(self) -> UInt16:
        """
        Read an unsigned 16-bit integer from fixed 2 bytes (little-endian).
        Range: 0 to 2^16-1.
        Size: 2 bytes.
        """
        if self.pos + 2 > len(self.buffer):
            raise BinaryError(f"unexpected end of buffer at {self.pos}")
        value = struct.unpack("<H", self.buffer[self.pos : self.pos + 2])[0]
        self.pos += 2
        return value

    @override
    def read_uint32(self) -> UInt32:
        """
        Read an unsigned 32-bit integer from fixed 4 bytes (little-endian).
        Range: 0 to 2^32-1.
        Size: 4 bytes.
        """
        if self.pos + 4 > len(self.buffer):
            raise BinaryError(f"unexpected end of buffer at {self.pos}")
        value = struct.unpack("<I", self.buffer[self.pos : self.pos + 4])[0]
        self.pos += 4
        return value

    @override
    def read_uint64(self) -> UInt64:
        """
        Read an unsigned 64-bit integer from fixed 8 bytes (little-endian).
        Range: 0 to 2^64-1.
        Size: 8 bytes.
        """
        if self.pos + 8 > len(self.buffer):
            raise BinaryError(f"unexpected end of buffer at {self.pos}")
        value = struct.unpack("<Q", self.buffer[self.pos : self.pos + 8])[0]
        self.pos += 8
        return value

    @override
    def read_uint128(self) -> UInt128:
        """
        Read an unsigned 128-bit integer from fixed 16 bytes (little-endian).
        Range: 0 to 2^128-1.
        Size: 16 bytes.
        """
        if self.pos + 16 > len(self.buffer):
            raise BinaryError(f"unexpected end of buffer at {self.pos}")
        low, high = struct.unpack("<QQ", self.buffer[self.pos : self.pos + 16])
        self.pos += 16
        return (high << 64) | low

    @override
    def read_datetime(self) -> DateTime:
        """
        Read a datetime from fixed 8 bytes of signed 64-bit microseconds since Unix epoch (UTC).
        Range: approximately ±292,277 years.
        Size: 8 bytes.
        """
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

    @override
    def read_date(self) -> Date:
        """
        Read a date from fixed 8 bytes of signed 64-bit days since Unix epoch (1970-01-01).
        Size: 8 bytes.
        """
        if self.pos + 8 > len(self.buffer):
            raise BinaryError(f"unexpected end of buffer at {self.pos}")
        days = struct.unpack("<q", self.buffer[self.pos : self.pos + 8])[0]
        self.pos += 8
        epoch = datetime(1970, 1, 1, tzinfo=UTC)
        return epoch.date() + timedelta(days=days)

    @override
    def read_time(self) -> Time:
        """
        Read a time from fixed 8 bytes of unsigned 64-bit nanoseconds since midnight.
        Range: 00:00:00.000000000 to 23:59:59.999999999.
        Size: 8 bytes.
        """
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

    @override
    def read_timestamp(self) -> Timestamp:
        """
        Read a timestamp from fixed 8 bytes of unsigned 64-bit nanoseconds since Unix epoch (UTC).
        Range: 1970-01-01 to 2554-07-21 UTC.
        Size: 8 bytes.
        """
        if self.pos + 8 > len(self.buffer):
            raise BinaryError(f"unexpected end of buffer at {self.pos}")
        value = struct.unpack("<Q", self.buffer[self.pos : self.pos + 8])[0]
        self.pos += 8
        return value

    @override
    def read_duration(self) -> Duration:
        """
        Read a duration from fixed 8 bytes of signed 64-bit nanoseconds.
        Range: approximately ±292,277 years.
        Size: 8 bytes.
        """
        if self.pos + 8 > len(self.buffer):
            raise BinaryError(f"unexpected end of buffer at {self.pos}")
        nanos = struct.unpack("<q", self.buffer[self.pos : self.pos + 8])[0]
        self.pos += 8
        micros = nanos // 1_000
        return timedelta(microseconds=micros)

    @override
    def read_string(self) -> String:
        """
        Read a UTF-8 string from fixed 4-byte length prefix + UTF-8 bytes.
        Range: 0 to 2^32-1 bytes (UTF-8 encoded).
        Size: 4 bytes (length) + string length in bytes.
        """
        return self.read_bytes().decode("utf-8")

    @override
    def read_bytes(self) -> Bytes:
        """
        Read raw bytes from fixed 4-byte length prefix + data bytes.
        Range: 0 to 2^32-1 bytes.
        Size: 4 bytes (length) + data length.
        """
        if self.pos + 4 > len(self.buffer):
            raise BinaryError(f"unexpected end of buffer at {self.pos}")
        length = struct.unpack("<I", self.buffer[self.pos : self.pos + 4])[0]
        self.pos += 4
        if self.pos + length > len(self.buffer):
            raise BinaryError(f"unexpected end of buffer at {self.pos}")
        value = self.buffer[self.pos : self.pos + length]
        self.pos += length
        return value
