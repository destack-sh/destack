import struct


class BinaryError(ValueError):
    """Base class for binary encoding/decoding errors."""


class BinaryWriter:
    """Write binary data with variable-length encoding for integers."""

    def __init__(self) -> None:
        self.buffer = bytearray()

    def to_bytes(self) -> bytes:
        """Get the written bytes."""
        return bytes(self.buffer)

    def write_bool(self, value: bool) -> None:
        """Write a boolean as 1 byte."""
        self.buffer.append(1 if value else 0)

    def write_int8(self, value: int) -> None:
        """Write a signed 8-bit integer."""
        self.buffer.append(value & 0xFF)

    def write_uint8(self, value: int) -> None:
        """Write an unsigned 8-bit integer."""
        self.buffer.append(value & 0xFF)

    def write_int16(self, value: int) -> None:
        """Write a signed 16-bit integer with variable-length encoding."""
        self._write_varint(self._write_zigzag(value))

    def write_uint16(self, value: int) -> None:
        """Write an unsigned 16-bit integer with variable-length encoding."""
        self._write_varint(value)

    def write_int32(self, value: int) -> None:
        """Write a signed 32-bit integer with variable-length encoding."""
        self._write_varint(self._write_zigzag(value))

    def write_uint32(self, value: int) -> None:
        """Write an unsigned 32-bit integer with variable-length encoding."""
        self._write_varint(value)

    def write_int64(self, value: int) -> None:
        """Write a signed 64-bit integer with variable-length encoding."""
        self._write_varint(self._write_zigzag(value))

    def write_uint64(self, value: int) -> None:
        """Write an unsigned 64-bit integer with variable-length encoding."""
        self._write_varint(value)

    def write_float32(self, value: float) -> None:
        """Write a 32-bit float with optimized encoding for zero."""
        if value == 0.0:
            self.buffer.append(0)
        else:
            self.buffer.append(1)
            self.buffer.extend(struct.pack("<f", value))

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

    def write_bytes(self, value: bytes) -> None:
        """Write raw bytes with length prefix."""
        self._write_varint(len(value))
        self.buffer.extend(value)

    def write_string(self, value: str) -> None:
        """Write a UTF-8 string with length prefix."""
        encoded = value.encode("utf-8")
        self.write_bytes(encoded)

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
    """Read binary data with variable-length encoding for integers."""

    def __init__(self, data: bytes) -> None:
        self.buffer = data
        self.pos = 0

    @property
    def remaining(self) -> int:
        """Number of bytes remaining."""
        return len(self.buffer) - self.pos

    def read_bool(self) -> bool:
        """Read a boolean from 1 byte."""
        if self.pos >= len(self.buffer):
            raise BinaryError(f"unexpected end of buffer at {self.pos}")
        value = self.buffer[self.pos]
        self.pos += 1
        return value != 0

    def read_int8(self) -> int:
        """Read a signed 8-bit integer."""
        if self.pos >= len(self.buffer):
            raise BinaryError(f"unexpected end of buffer at {self.pos}")
        value = self.buffer[self.pos]
        self.pos += 1
        # convert to signed
        if value >= 0x80:
            return value - 0x100
        return value

    def read_uint8(self) -> int:
        """Read an unsigned 8-bit integer."""
        if self.pos >= len(self.buffer):
            raise BinaryError(f"unexpected end of buffer at {self.pos}")
        value = self.buffer[self.pos]
        self.pos += 1
        return value

    def read_int16(self) -> int:
        """Read a signed 16-bit integer with variable-length encoding."""
        return self._zigzag_decode(self._read_varint())

    def read_uint16(self) -> int:
        """Read an unsigned 16-bit integer with variable-length encoding."""
        return self._read_varint()

    def read_int32(self) -> int:
        """Read a signed 32-bit integer with variable-length encoding."""
        return self._zigzag_decode(self._read_varint())

    def read_uint32(self) -> int:
        """Read an unsigned 32-bit integer with variable-length encoding."""
        return self._read_varint()

    def read_int64(self) -> int:
        """Read a signed 64-bit integer with variable-length encoding."""
        return self._zigzag_decode(self._read_varint())

    def read_uint64(self) -> int:
        """Read an unsigned 64-bit integer with variable-length encoding."""
        return self._read_varint()

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

    def read_bytes(self) -> bytes:
        """Read raw bytes with length prefix."""
        length = self._read_varint()
        if self.pos + length > len(self.buffer):
            raise BinaryError(f"unexpected end of buffer at {self.pos}")
        value = self.buffer[self.pos : self.pos + length]
        self.pos += length
        return value

    def read_string(self) -> str:
        """Read a UTF-8 string with length prefix."""
        return self.read_bytes().decode("utf-8")

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
            if shift >= 64:
                raise BinaryError(f"varint too long at {self.pos}")

    def _zigzag_decode(self, value: int) -> int:
        """Decode unsigned integer to signed using zigzag decoding."""
        if value & 1:
            return -((value + 1) >> 1)
        else:
            return value >> 1
