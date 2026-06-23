from __future__ import annotations

import json
import math
import struct
from collections.abc import Callable, Sequence
from typing import Any, TypeVar

U128_VARINT_MAX_BYTES = 19
U128_VARINT_LAST_BYTE_MAX = 0x03
U128_MAX = (1 << 128) - 1
I128_MIN = -(1 << 127)
I128_MAX = (1 << 127) - 1

T = TypeVar("T")


class SerdeError(Exception):
    """Error thrown while encoding or decoding Destack binary serde bytes."""


class Writer:
    """Writer for canonical Destack binary serde bytes."""

    def __init__(self) -> None:
        self._bytes = bytearray()

    def bytes(self) -> bytes:
        """Return the written bytes."""

        return bytes(self._bytes)

    def write_byte(self, value: int) -> None:
        """Write one raw byte."""

        if not isinstance(value, int) or value < 0 or value > 0xFF:
            raise SerdeError(f"byte out of range: {value}")

        self._bytes.append(value)

    def write_bytes(self, value: bytes | bytearray | Sequence[int]) -> None:
        """Write raw bytes."""

        for byte in value:
            self.write_byte(byte)

    def write_bool(self, value: bool) -> None:
        """Write one boolean."""

        self.write_byte(1 if value else 0)

    def write_unsigned(self, value: int) -> None:
        """Write one unsigned integer varint."""

        integer = unsigned_int(value)
        byte_count = 0

        while True:
            if byte_count == U128_VARINT_MAX_BYTES:
                raise SerdeError("varint too large")
            byte_count += 1

            byte = integer & 0x7F
            integer >>= 7
            if integer == 0:
                self.write_byte(byte)
                return

            self.write_byte(byte | 0x80)

    def write_signed(self, value: int) -> None:
        """Write one signed integer zigzag varint."""

        integer = signed_int(value)
        encoded = (integer << 1) ^ (integer >> 127)

        self.write_unsigned(encoded)

    def write_i8(self, value: int) -> None:
        """Write one signed i8 byte."""

        if not isinstance(value, int) or value < -128 or value > 127:
            raise SerdeError(f"i8 out of range: {value}")

        self.write_byte(value + 256 if value < 0 else value)

    def write_f32(self, value: float) -> None:
        """Write one little endian f32."""

        self.write_bytes(struct.pack("<f", value))

    def write_f64(self, value: float) -> None:
        """Write one little endian f64."""

        self.write_bytes(struct.pack("<d", value))

    def write_char(self, value: str) -> None:
        """Write one unicode scalar value."""

        self.write_unsigned(single_code_point(value))

    def write_string(self, value: str) -> None:
        """Write one UTF-8 string."""

        self.write_byte_slice(value.encode("utf-8"))

    def write_json(self, value: Any) -> None:
        """Write one JSON value."""

        text = json.dumps(value, ensure_ascii=False, separators=(",", ":"))
        self.write_string(text)

    def write_byte_slice(self, value: bytes | bytearray | Sequence[int]) -> None:
        """Write one length-prefixed byte slice."""

        self.write_unsigned(len(value))
        self.write_bytes(value)


class Reader:
    """Reader for canonical Destack binary serde bytes."""

    def __init__(self, data: bytes | bytearray | Sequence[int]) -> None:
        self._bytes = bytes(data)
        self._offset = 0

    def finish(self) -> None:
        """Validate that all input bytes were consumed."""

        if self._offset != len(self._bytes):
            raise SerdeError("trailing bytes")

    def read_byte(self) -> int:
        """Read one raw byte."""

        if self._offset >= len(self._bytes):
            raise SerdeError("unexpected end of input")

        byte = self._bytes[self._offset]
        self._offset += 1

        return byte

    def read_bytes(self, length: int) -> bytes:
        """Read exact raw bytes."""

        if not isinstance(length, int) or length < 0:
            raise SerdeError(f"invalid byte length: {length}")

        end = self._offset + length
        if end > len(self._bytes):
            raise SerdeError("unexpected end of input")

        value = self._bytes[self._offset : end]
        self._offset = end

        return value

    def read_bool(self) -> bool:
        """Read one boolean."""

        value = self.read_byte()
        if value == 0:
            return False
        if value == 1:
            return True

        raise SerdeError(f"invalid bool byte: {value}")

    def read_unsigned(self) -> int:
        """Read one unsigned integer varint."""

        value = 0
        shift = 0
        byte_count = 0

        while True:
            if byte_count == U128_VARINT_MAX_BYTES:
                raise SerdeError("varint too large")
            byte_count += 1

            byte = self.read_byte()
            chunk = byte & 0x7F
            if shift == 126 and chunk > U128_VARINT_LAST_BYTE_MAX:
                raise SerdeError("varint too large")

            value |= chunk << shift
            if byte & 0x80 == 0:
                if shift > 0 and chunk == 0:
                    raise SerdeError("non canonical varint")

                return value

            shift += 7

    def read_number(self) -> int:
        """Read one unsigned integer varint as an int."""

        return safe_number(self.read_unsigned())

    def read_signed(self) -> int:
        """Read one signed integer zigzag varint."""

        value = self.read_unsigned()

        return (value >> 1) ^ -(value & 1)

    def read_signed_number(self) -> int:
        """Read one signed integer zigzag varint as an int."""

        return safe_number(self.read_signed())

    def read_i8(self) -> int:
        """Read one signed i8 byte."""

        byte = self.read_byte()

        return byte - 256 if byte > 127 else byte

    def read_f32(self) -> float:
        """Read one little endian f32."""

        return struct.unpack("<f", self.read_bytes(4))[0]

    def read_f64(self) -> float:
        """Read one little endian f64."""

        return struct.unpack("<d", self.read_bytes(8))[0]

    def read_char(self) -> str:
        """Read one unicode scalar value."""

        try:
            return chr(self.read_unsigned())
        except ValueError as error:
            raise SerdeError(str(error)) from error

    def read_string(self) -> str:
        """Read one UTF-8 string."""

        try:
            return self.read_byte_slice().decode("utf-8")
        except UnicodeDecodeError as error:
            raise SerdeError(str(error)) from error

    def read_json(self) -> Any:
        """Read one JSON value."""

        try:
            return json.loads(self.read_string())
        except json.JSONDecodeError as error:
            raise SerdeError(str(error)) from error

    def read_byte_slice(self) -> bytes:
        """Read one length-prefixed byte slice."""

        return self.read_bytes(self.read_number())

    def read_option(self, decode: Callable[[], T]) -> T | None:
        """Read one optional value."""

        tag = self.read_byte()
        if tag == 0:
            return None
        if tag == 1:
            return decode()

        raise SerdeError(f"invalid option tag: {tag}")


def encode_value(encode: Callable[[Writer], None]) -> bytes:
    """Encode one value to canonical bytes."""

    writer = Writer()
    encode(writer)

    return writer.bytes()


def decode_value(
    data: bytes | bytearray | Sequence[int], decode: Callable[[Reader], T]
) -> T:
    """Decode one value from canonical bytes."""

    reader = Reader(data)
    value = decode(reader)
    reader.finish()

    return value


def nested_bytes(encode: Callable[[Writer], None]) -> bytes:
    """Return nested canonical bytes for sorting map keys."""

    return encode_value(encode)


def unsigned_int(value: int) -> int:
    """Return one validated unsigned integer."""

    if not isinstance(value, int) or value < 0 or value > U128_MAX:
        raise SerdeError(f"unsigned integer out of range: {value}")

    return value


def signed_int(value: int) -> int:
    """Return one validated signed integer."""

    if not isinstance(value, int) or value < I128_MIN or value > I128_MAX:
        raise SerdeError(f"signed integer out of range: {value}")

    return value


def safe_number(value: int) -> int:
    """Return one Python int for a protocol number."""

    if not isinstance(value, int) or not math.isfinite(float(value)):
        raise SerdeError(f"invalid number: {value}")

    return value


def single_code_point(value: str) -> int:
    """Return the only code point in one string."""

    if not isinstance(value, str) or len(value) != 1:
        raise SerdeError("char requires a single code point")

    return ord(value)
