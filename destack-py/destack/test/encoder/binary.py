import math

import pytest

from destack.language.core.runtime.binary import BinaryReader, BinaryWriter


def test_bool_roundtrip():
    """Test boolean encoding and decoding."""
    writer = BinaryWriter()
    writer.write_bool(True)
    writer.write_bool(False)

    # Expected: 2 bytes total (1 byte per bool)
    assert len(writer.to_bytes()) == 2

    reader = BinaryReader(writer.to_bytes())
    assert reader.read_bool() is True
    assert reader.read_bool() is False
    assert reader.remaining == 0


def test_int8_roundtrip():
    """Test signed 8-bit integer encoding and decoding."""
    test_values = [0, 1, -1, 127, -128, 42, -42]

    writer = BinaryWriter()
    for value in test_values:
        writer.write_int8(value)

    # Expected: 7 bytes (1 byte per int8)
    assert len(writer.to_bytes()) == 7

    reader = BinaryReader(writer.to_bytes())
    for expected in test_values:
        assert reader.read_int8() == expected
    assert reader.remaining == 0


def test_uint8_roundtrip():
    """Test unsigned 8-bit integer encoding and decoding."""
    test_values = [0, 1, 127, 128, 255, 42]

    writer = BinaryWriter()
    for value in test_values:
        writer.write_uint8(value)

    # Expected: 6 bytes (1 byte per uint8)
    assert len(writer.to_bytes()) == 6

    reader = BinaryReader(writer.to_bytes())
    for expected in test_values:
        assert reader.read_uint8() == expected
    assert reader.remaining == 0


def test_int16_varint():
    """Test signed 16-bit integer variable-length encoding."""
    # Map of value -> expected bytes with explanation
    test_cases = {
        0: 1,  # zigzag(0) = 0 -> 1 byte
        1: 1,  # zigzag(1) = 2 -> 1 byte
        -1: 1,  # zigzag(-1) = 1 -> 1 byte
        63: 1,  # zigzag(63) = 126 -> 1 byte
        64: 2,  # zigzag(64) = 128 -> 2 bytes
        -64: 1,  # zigzag(-64) = 127 -> 1 byte
        -65: 2,  # zigzag(-65) = 129 -> 2 bytes
        127: 2,  # zigzag(127) = 254 -> 2 bytes
        128: 2,  # zigzag(128) = 256 -> 2 bytes
        -128: 2,  # zigzag(-128) = 255 -> 2 bytes
        32767: 3,  # max int16 -> zigzag(32767) = 65534 -> 3 bytes
        -32768: 3,  # min int16 -> zigzag(-32768) = 65535 -> 3 bytes
    }

    for value, expected_bytes in test_cases.items():
        writer = BinaryWriter()
        writer.write_int16(value)
        data = writer.to_bytes()
        assert len(data) == expected_bytes, (
            f"Value {value} should encode to {expected_bytes} bytes, got {len(data)}"
        )

        reader = BinaryReader(data)
        assert reader.read_int16() == value


def test_int32_varint():
    """Test signed 32-bit integer variable-length encoding."""
    # Map of value -> expected bytes
    value_to_bytes = {
        0: 1,  # zigzag = 0
        1: 1,  # zigzag = 2
        -1: 1,  # zigzag = 1
        127: 2,  # zigzag = 254
        -128: 2,  # zigzag = 255
        256: 2,  # zigzag = 512
        -256: 2,  # zigzag = 511
        65535: 3,  # zigzag = 131070
        -65536: 3,  # zigzag = 131071
        2147483647: 5,  # zigzag = 4294967294
        -2147483648: 5,  # zigzag = 4294967295
    }

    writer = BinaryWriter()
    for value in value_to_bytes:
        writer.write_int32(value)

    # Calculate total expected bytes
    total_expected = sum(value_to_bytes.values())
    assert len(writer.to_bytes()) == total_expected  # 27 bytes

    reader = BinaryReader(writer.to_bytes())
    for value in value_to_bytes:
        assert reader.read_int32() == value
    assert reader.remaining == 0


def test_int64_varint():
    """Test signed 64-bit integer variable-length encoding."""
    # Map of value -> expected bytes
    value_to_bytes = {
        0: 1,
        1: 1,
        -1: 1,
        127: 2,
        -128: 2,
        2**31 - 1: 5,
        -(2**31): 5,
        2**63 - 1: 10,  # max varint
        -(2**63): 10,  # max varint
        123456789012345: 7,
        -123456789012345: 7,
    }

    writer = BinaryWriter()
    for value in value_to_bytes:
        writer.write_int64(value)

    # Calculate total expected bytes
    total_expected = sum(value_to_bytes.values())
    assert len(writer.to_bytes()) == total_expected  # 51 bytes

    reader = BinaryReader(writer.to_bytes())
    for value in value_to_bytes:
        assert reader.read_int64() == value
    assert reader.remaining == 0


def test_uint_varint():
    """Test unsigned integer variable-length encoding."""
    # Map of value -> expected bytes
    value_to_bytes = {
        0: 1,  # 0 -> 1 byte
        127: 1,  # fits in 7 bits
        128: 2,  # needs 2 bytes
        16383: 2,  # fits in 14 bits
        16384: 3,  # needs 3 bytes
        0xFFFF: 3,  # max uint16 (65535)
        0xFFFFFFFF: 5,  # max uint32 (4294967295)
    }

    total_bytes = 0
    for value, expected_bytes in value_to_bytes.items():
        writer = BinaryWriter()
        writer.write_uint32(value)
        data = writer.to_bytes()
        assert len(data) == expected_bytes, (
            f"Value {value} should encode to {expected_bytes} bytes, got {len(data)}"
        )
        total_bytes += expected_bytes

        reader = BinaryReader(data)
        assert reader.read_uint32() == value

    # Total expected: 1 + 1 + 2 + 2 + 3 + 3 + 5 = 17 bytes
    assert total_bytes == sum(value_to_bytes.values())


def test_float32_encoding():
    """Test 32-bit float encoding with zero optimization."""
    # zero should be 1 byte
    writer = BinaryWriter()
    writer.write_float32(0.0)
    assert len(writer.to_bytes()) == 1

    # non-zero should be 5 bytes (1 flag + 4 float)
    writer = BinaryWriter()
    writer.write_float32(math.pi)
    assert len(writer.to_bytes()) == 5

    # test roundtrip with expected sizes
    value_to_bytes = {
        0.0: 1,  # zero optimization
        1.0: 5,  # flag + float32
        -1.0: 5,
        math.pi: 5,
        -math.pi: 5,
        float("inf"): 5,
        float("-inf"): 5,
    }

    writer = BinaryWriter()
    for value in value_to_bytes:
        writer.write_float32(value)

    assert len(writer.to_bytes()) == sum(value_to_bytes.values())  # 31 bytes

    reader = BinaryReader(writer.to_bytes())
    for value in value_to_bytes:
        result = reader.read_float32()
        if value != value:  # NaN check
            assert result != result
        else:
            assert result == pytest.approx(value)


def test_float64_encoding():
    """Test 64-bit float encoding with optimizations."""
    # zero should be 1 byte
    writer = BinaryWriter()
    writer.write_float64(0.0)
    assert len(writer.to_bytes()) == 1

    # small integers should use varint encoding (1 flag + varint bytes)
    writer = BinaryWriter()
    writer.write_float64(42.0)
    data = writer.to_bytes()
    assert len(data) == 2  # 1 flag + 1 varint byte for 84 (zigzag of 42)
    assert data[0] == 1  # integer flag

    # large float should be 9 bytes (1 flag + 8 float)
    writer = BinaryWriter()
    writer.write_float64(math.pi)
    assert len(writer.to_bytes()) == 9

    # test roundtrip with expected sizes
    value_to_bytes = {
        0.0: 1,  # zero optimization
        1.0: 2,  # flag + varint (zigzag(1) = 2)
        -1.0: 2,  # flag + varint (zigzag(-1) = 1)
        42.0: 2,  # flag + varint (zigzag(42) = 84)
        -42.0: 2,  # flag + varint (zigzag(-42) = 83)
        1000.0: 3,  # flag + varint (zigzag(1000) = 2000)
        -1000.0: 3,  # flag + varint (zigzag(-1000) = 1999)
        2**53: 9,  # flag + varint for large number
        -(2**53): 9,  # flag + varint for large number
        math.pi: 9,  # flag + double
        -math.pi: 9,  # flag + double
        1e100: 9,  # flag + double
        -1e100: 9,  # flag + double
        float("inf"): 9,  # flag + double
        float("-inf"): 9,  # flag + double
    }

    writer = BinaryWriter()
    for value in value_to_bytes:
        writer.write_float64(value)

    assert len(writer.to_bytes()) == sum(value_to_bytes.values())  # 87 bytes

    reader = BinaryReader(writer.to_bytes())
    for value in value_to_bytes:
        result = reader.read_float64()
        assert result == pytest.approx(value)


def test_string_encoding():
    """Test string encoding with length prefix."""
    # Map of string -> expected bytes
    string_to_bytes = {
        "": 1,  # just length 0
        "hello": 6,  # 1 length + 5 chars
        "Hello, 世界!": 15,  # 1 length + 14 UTF-8 bytes (7 + 3 + 3 + 1)
        "a" * 1000: 1002,  # 2 length bytes + 1000 chars
    }

    writer = BinaryWriter()
    for s in string_to_bytes:
        writer.write_string(s)

    assert len(writer.to_bytes()) == sum(string_to_bytes.values())  # 1024 bytes

    reader = BinaryReader(writer.to_bytes())
    for s in string_to_bytes:
        assert reader.read_string() == s
    assert reader.remaining == 0


def test_bytes_encoding():
    """Test bytes encoding with length prefix."""
    # Map of bytes -> expected bytes
    bytes_to_bytes = {
        b"": 1,  # just length 0
        b"hello": 6,  # 1 length + 5 bytes
        b"\x00\x01\x02\x03": 5,  # 1 length + 4 bytes
        bytes(range(256)): 258,  # 2 length bytes + 256 bytes
    }

    writer = BinaryWriter()
    for b in bytes_to_bytes:
        writer.write_bytes(b)

    assert len(writer.to_bytes()) == sum(bytes_to_bytes.values())  # 270 bytes

    reader = BinaryReader(writer.to_bytes())
    for b in bytes_to_bytes:
        assert reader.read_bytes() == b
    assert reader.remaining == 0


def test_mixed_types():
    """Test encoding and decoding mixed types."""
    writer = BinaryWriter()

    # Write various types with expected sizes
    operations = [
        (lambda: writer.write_bool(True), 1),  # 1 byte
        (lambda: writer.write_int8(-42), 1),  # 1 byte
        (lambda: writer.write_uint16(65535), 3),  # 3 bytes (varint)
        (lambda: writer.write_int32(-1234567), 4),  # 4 bytes (varint)
        (lambda: writer.write_float32(math.pi), 5),  # 5 bytes
        (lambda: writer.write_float64(math.e), 9),  # 9 bytes
        (lambda: writer.write_string("test string"), 12),  # 12 bytes (1 + 11)
        (lambda: writer.write_bytes(b"\x01\x02\x03"), 4),  # 4 bytes (1 + 3)
    ]

    for write_op, _ in operations:
        write_op()

    # Calculate total expected bytes
    total_expected = sum(size for _, size in operations)
    assert len(writer.to_bytes()) == total_expected  # 39 bytes

    # read them back
    reader = BinaryReader(writer.to_bytes())
    assert reader.read_bool() is True
    assert reader.read_int8() == -42
    assert reader.read_uint16() == 65535
    assert reader.read_int32() == -1234567
    assert reader.read_float32() == pytest.approx(math.pi, rel=1e-6)
    assert reader.read_float64() == pytest.approx(math.e)
    assert reader.read_string() == "test string"
    assert reader.read_bytes() == b"\x01\x02\x03"
    assert reader.remaining == 0


def test_int_zigzag_roundtrip():
    """Test zigzag encoding/decoding correctness."""
    test_cases = [
        (0, 0),
        (-1, 1),
        (1, 2),
        (-2, 3),
        (2, 4),
        (-64, 127),
        (64, 128),
        (-65, 129),
        (2147483647, 4294967294),
        (-2147483648, 4294967295),
    ]

    writer = BinaryWriter()
    for signed, unsigned in test_cases:
        assert writer._write_zigzag(signed) == unsigned

    reader = BinaryReader(b"")
    for signed, unsigned in test_cases:
        assert reader._zigzag_decode(unsigned) == signed


def test_varint_roundtrip():
    """Test varint encoding produces expected sizes for various ranges."""
    # Map of value -> expected bytes
    varint_sizes = {
        0: 1,  # 1 byte: 0-127
        127: 1,
        128: 2,  # 2 bytes: 128-16383
        16383: 2,
        16384: 3,  # 3 bytes: 16384-2097151
        2097151: 3,
        2097152: 4,  # 4 bytes: 2097152-268435455
        268435455: 4,
        268435456: 5,  # 5 bytes: 268435456-max uint32
        0xFFFFFFFF: 5,
    }

    for value, expected_bytes in varint_sizes.items():
        writer = BinaryWriter()
        writer._write_varint(value)
        assert len(writer.to_bytes()) == expected_bytes, (
            f"Varint {value} should encode to {expected_bytes} bytes"
        )
