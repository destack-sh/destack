import math

import pytest

from destack import UUID, BinaryDecoder, BinaryEncoder


def test_bool():
    """Test boolean encoding and decoding."""
    encoder = BinaryEncoder()
    encoder.write_bool(True)
    encoder.write_bool(False)

    assert len(encoder.to_bytes()) == 2

    decoder = BinaryDecoder(buffer=encoder.to_bytes())
    assert decoder.read_bool() is True
    assert decoder.read_bool() is False
    assert decoder.remaining == 0


def test_int8():
    """Test signed 8-bit integer encoding and decoding."""
    test_values = [0, 1, -1, 127, -128, 42, -42]

    encoder = BinaryEncoder()
    for value in test_values:
        encoder.write_int8(value)

    assert len(encoder.to_bytes()) == 7

    decoder = BinaryDecoder(buffer=encoder.to_bytes())
    for expected in test_values:
        assert decoder.read_int8() == expected
    assert decoder.remaining == 0


def test_int16():
    """Test signed 16-bit integer encoding and decoding."""
    test_values = [0, 1, -1, 127, -128, 32767, -32768, 42, -42]

    encoder = BinaryEncoder()
    for value in test_values:
        encoder.write_int16(value)

    assert len(encoder.to_bytes()) == 18

    decoder = BinaryDecoder(buffer=encoder.to_bytes())
    for expected in test_values:
        assert decoder.read_int16() == expected
    assert decoder.remaining == 0


def test_int32():
    """Test signed 32-bit integer encoding and decoding."""
    test_values = [0, 1, -1, 127, -128, 32767, -32768, 2147483647, -2147483648, 42, -42]

    encoder = BinaryEncoder()
    for value in test_values:
        encoder.write_int32(value)

    assert len(encoder.to_bytes()) == 44

    decoder = BinaryDecoder(buffer=encoder.to_bytes())
    for expected in test_values:
        assert decoder.read_int32() == expected
    assert decoder.remaining == 0


def test_int64():
    """Test signed 64-bit integer encoding and decoding."""
    test_values = [
        0,
        1,
        -1,
        127,
        -128,
        32767,
        -32768,
        2147483647,
        -2147483648,
        9223372036854775807,
        -9223372036854775808,
        42,
        -42,
    ]

    encoder = BinaryEncoder()
    for value in test_values:
        encoder.write_int64(value)

    assert len(encoder.to_bytes()) == 104

    decoder = BinaryDecoder(buffer=encoder.to_bytes())
    for expected in test_values:
        assert decoder.read_int64() == expected
    assert decoder.remaining == 0


def test_int128():
    """Test signed 128-bit integer encoding and decoding."""
    test_values = [0, 1, -1, 127, -128, 2**63 - 1, -(2**63), 2**127 - 1, -(2**127), 42, -42]

    encoder = BinaryEncoder()
    for value in test_values:
        encoder.write_int128(value)

    assert len(encoder.to_bytes()) == 176

    decoder = BinaryDecoder(buffer=encoder.to_bytes())
    for expected in test_values:
        assert decoder.read_int128() == expected
    assert decoder.remaining == 0


def test_uint8():
    """Test unsigned 8-bit integer encoding and decoding."""
    test_values = [0, 1, 127, 128, 255, 42]

    encoder = BinaryEncoder()
    for value in test_values:
        encoder.write_uint8(value)

    assert len(encoder.to_bytes()) == 6

    decoder = BinaryDecoder(buffer=encoder.to_bytes())
    for expected in test_values:
        assert decoder.read_uint8() == expected
    assert decoder.remaining == 0


def test_uint16():
    """Test unsigned 16-bit integer encoding and decoding."""
    test_values = [0, 1, 127, 128, 255, 256, 32767, 32768, 65535, 42]

    encoder = BinaryEncoder()
    for value in test_values:
        encoder.write_uint16(value)

    assert len(encoder.to_bytes()) == 20

    decoder = BinaryDecoder(buffer=encoder.to_bytes())
    for expected in test_values:
        assert decoder.read_uint16() == expected
    assert decoder.remaining == 0


def test_uint32():
    """Test unsigned 32-bit integer encoding and decoding."""
    test_values = [0, 1, 127, 128, 255, 256, 65535, 65536, 2147483647, 2147483648, 4294967295, 42]

    encoder = BinaryEncoder()
    for value in test_values:
        encoder.write_uint32(value)

    assert len(encoder.to_bytes()) == 48

    decoder = BinaryDecoder(buffer=encoder.to_bytes())
    for expected in test_values:
        assert decoder.read_uint32() == expected
    assert decoder.remaining == 0


def test_uint64():
    """Test unsigned 64-bit integer encoding and decoding."""
    test_values = [
        0,
        1,
        127,
        128,
        255,
        256,
        65535,
        65536,
        4294967295,
        4294967296,
        9223372036854775807,
        9223372036854775808,
        18446744073709551615,
        42,
    ]

    encoder = BinaryEncoder()
    for value in test_values:
        encoder.write_uint64(value)

    assert len(encoder.to_bytes()) == 112

    decoder = BinaryDecoder(buffer=encoder.to_bytes())
    for expected in test_values:
        assert decoder.read_uint64() == expected
    assert decoder.remaining == 0


def test_uint128():
    """Test unsigned 128-bit integer encoding and decoding."""
    test_values = [0, 1, 127, 128, 255, 256, 2**64 - 1, 2**64, 2**100, 2**128 - 1, 42]

    encoder = BinaryEncoder()
    for value in test_values:
        encoder.write_uint128(value)

    assert len(encoder.to_bytes()) == 176

    decoder = BinaryDecoder(buffer=encoder.to_bytes())
    for expected in test_values:
        assert decoder.read_uint128() == expected
    assert decoder.remaining == 0


def test_float32():
    """Test 32-bit float encoding."""
    test_values = [
        0.0,
        -0.0,
        float("inf"),
        float("-inf"),
        float("nan"),
        1.0,
        -1.0,
        math.pi,
        -math.pi,
    ]

    encoder = BinaryEncoder()
    for value in test_values:
        encoder.write_float32(value)

    assert len(encoder.to_bytes()) == 36

    decoder = BinaryDecoder(buffer=encoder.to_bytes())
    for value in test_values:
        result = decoder.read_float32()
        if value != value:  # NaN check
            assert result != result
        else:
            assert result == pytest.approx(value)


def test_float64():
    """Test 64-bit float encoding."""
    test_values = [
        0.0,
        -0.0,
        float("inf"),
        float("-inf"),
        float("nan"),
        1.0,
        -1.0,
        42.0,
        -42.0,
        1000.0,
        -1000.0,
        2**53,
        -(2**53),
        math.pi,
        -math.pi,
        1e100,
        -1e100,
    ]

    encoder = BinaryEncoder()
    for value in test_values:
        encoder.write_float64(value)

    assert len(encoder.to_bytes()) == 136

    decoder = BinaryDecoder(buffer=encoder.to_bytes())
    for value in test_values:
        result = decoder.read_float64()
        if value != value:  # NaN check
            assert result != result
        else:
            assert result == pytest.approx(value)


def test_string():
    """Test string encoding with length prefix."""
    test_strings = [
        "",
        "hello",
        "Hello, 世界!",
        "a" * 1000,
    ]

    encoder = BinaryEncoder()
    for s in test_strings:
        encoder.write_string(s)

    decoder = BinaryDecoder(buffer=encoder.to_bytes())
    for s in test_strings:
        assert decoder.read_string() == s
    assert decoder.remaining == 0


def test_character():
    """Test character encoding and decoding."""
    test_chars = [
        "A",  # ASCII
        "é",  # Latin-1
        "世",  # BMP
        "🌍",  # non-BMP (surrogate pair / astral)
        "\u0000",  # null
    ]

    encoder = BinaryEncoder()
    for ch in test_chars:
        encoder.write_character(ch)

    decoder = BinaryDecoder(buffer=encoder.to_bytes())
    for expected in test_chars:
        assert decoder.read_character() == expected
    assert decoder.remaining == 0


def test_mixed_types():
    """Test encoding and decoding mixed types."""
    encoder = BinaryEncoder()

    # write various types
    encoder.write_bool(True)
    encoder.write_int8(-42)
    encoder.write_uint16(65535)
    encoder.write_int32(-1234567)
    encoder.write_int128(-(2**100))
    encoder.write_uint128(2**100)
    encoder.write_float32(math.pi)
    encoder.write_float64(math.e)
    encoder.write_string("test string")
    encoder.write_character("Z")

    # read them back
    decoder = BinaryDecoder(buffer=encoder.to_bytes())
    assert decoder.read_bool() is True
    assert decoder.read_int8() == -42
    assert decoder.read_uint16() == 65535
    assert decoder.read_int32() == -1234567
    assert decoder.read_int128() == -(2**100)
    assert decoder.read_uint128() == 2**100
    assert decoder.read_float32() == pytest.approx(math.pi, rel=1e-6)
    assert decoder.read_float64() == pytest.approx(math.e)
    assert decoder.read_string() == "test string"
    assert decoder.read_character() == "Z"
    assert decoder.remaining == 0


def test_datetime():
    """Test datetime encoding and decoding."""
    from datetime import UTC, datetime

    test_cases = [
        # unix epoch
        datetime(1970, 1, 1, tzinfo=UTC),
        # current-ish time
        datetime(2024, 1, 15, 14, 30, 45, 123456, tzinfo=UTC),
        # before unix epoch
        datetime(1969, 12, 31, 23, 59, 59, tzinfo=UTC),
        # far future
        datetime(2100, 1, 1, tzinfo=UTC),
        # with microseconds
        datetime(2000, 6, 15, 12, 0, 0, 999999, tzinfo=UTC),
        # range sampling around unix epoch
        datetime(1900, 1, 1, tzinfo=UTC),
        datetime(2000, 1, 1, tzinfo=UTC),
        datetime(2400, 2, 29, tzinfo=UTC),
    ]

    encoder = BinaryEncoder()
    for dt in test_cases:
        encoder.write_datetime(dt)

    decoder = BinaryDecoder(buffer=encoder.to_bytes())
    for expected in test_cases:
        result = decoder.read_datetime()
        assert result == expected
        assert result.tzinfo == UTC
    assert decoder.remaining == 0


def test_datetime_naive():
    """Test that naive datetimes are converted to UTC."""
    from datetime import UTC, datetime

    naive_dt = datetime(2024, 1, 15, 14, 30, 45)  # noqa: DTZ001
    encoder = BinaryEncoder()
    encoder.write_datetime(naive_dt)

    decoder = BinaryDecoder(buffer=encoder.to_bytes())
    result = decoder.read_datetime()
    assert result.tzinfo == UTC
    assert result.replace(tzinfo=None) == naive_dt


def test_date():
    """Test date encoding and decoding."""
    from datetime import date

    test_cases = [
        date(1970, 1, 1),  # unix epoch
        date(2024, 1, 15),  # current-ish
        date(1969, 12, 31),  # before epoch
        date(2100, 12, 31),  # far future
        date(1900, 1, 1),  # old date
        # range sampling
        date(2000, 2, 29),  # leap day
        date(1900, 2, 28),  # non-leap century year
    ]

    encoder = BinaryEncoder()
    for d in test_cases:
        encoder.write_date(d)

    decoder = BinaryDecoder(buffer=encoder.to_bytes())
    for expected in test_cases:
        assert decoder.read_date() == expected
    assert decoder.remaining == 0


def test_time():
    """Test time encoding and decoding."""
    from datetime import time

    test_cases = [
        time(0, 0, 0, 0),  # midnight
        time(12, 0, 0, 0),  # noon
        time(23, 59, 59, 999999),  # almost midnight (microsecond precision)
        time(14, 30, 45, 123456),  # arbitrary time
        time(0, 0, 0, 1),  # 1 microsecond after midnight
    ]

    encoder = BinaryEncoder()
    for t in test_cases:
        encoder.write_time(t)

    decoder = BinaryDecoder(buffer=encoder.to_bytes())
    for expected in test_cases:
        assert decoder.read_time() == expected
    assert decoder.remaining == 0


def test_timestamp():
    """Test timestamp encoding and decoding (nanoseconds since epoch)."""
    test_values = [
        0,
        1,
        999_999_999,
        1_000_000_000,  # 1 second
        1_000_000_001,
        2**32 - 1,
        2**32,
        2**63 - 1,
        2**63,
        2**64 - 1,
    ]

    encoder = BinaryEncoder()
    for value in test_values:
        encoder.write_timestamp(value)

    decoder = BinaryDecoder(buffer=encoder.to_bytes())
    for value in test_values:
        assert decoder.read_timestamp() == value
    assert decoder.remaining == 0


def test_duration():
    """Test duration encoding and decoding."""
    from datetime import timedelta

    test_cases = [
        timedelta(0),  # zero
        timedelta(days=1),  # 1 day
        timedelta(hours=1, minutes=30, seconds=45),  # mixed
        timedelta(microseconds=1),  # tiny
        timedelta(days=-1),  # negative
        timedelta(weeks=52),  # ~1 year
        timedelta(days=365, hours=5, minutes=48, seconds=46),  # approx 1 year
        # weird combinations
        timedelta(days=1, microseconds=999999),  # almost 2 days
        timedelta(seconds=-1),  # negative second
        timedelta(days=1, seconds=-1),  # 1 day minus 1 second
        timedelta(hours=87600),  # 10 years in hours
        # edge cases with microseconds
        timedelta(microseconds=-1),  # negative microsecond
        timedelta(days=1, microseconds=-1),  # 1 day minus 1 microsecond
        timedelta(seconds=1, microseconds=-1),  # 999999 microseconds
    ]

    encoder = BinaryEncoder()
    for td in test_cases:
        encoder.write_duration(td)

    decoder = BinaryDecoder(buffer=encoder.to_bytes())
    for expected in test_cases:
        assert decoder.read_duration() == expected
    assert decoder.remaining == 0


def test_uuid():
    """Test UUID encoding and decoding."""

    test_cases = [
        UUID("00000000-0000-0000-0000-000000000000"),  # nil UUID
        UUID("12345678-1234-5678-1234-567812345678"),  # fixed pattern
        UUID("550e8400-e29b-41d4-a716-446655440000"),  # standard example
        UUID("ffffffff-ffff-ffff-ffff-ffffffffffff"),  # max UUID
    ]

    encoder = BinaryEncoder()
    for uuid in test_cases:
        encoder.write_uuid(uuid)

    # each UUID is exactly 16 bytes
    assert len(encoder.to_bytes()) == 16 * len(test_cases)

    decoder = BinaryDecoder(buffer=encoder.to_bytes())
    for expected in test_cases:
        assert decoder.read_uuid() == expected
    assert decoder.remaining == 0


def test_json():
    """Test JSON encoding and decoding."""
    test_cases = [
        None,
        True,
        False,
        42,
        math.pi,
        "hello",
        [],
        [1, 2, 3],
        {"key": "value"},
        {"nested": {"data": [1, 2, {"more": "stuff"}]}},
        ["mixed", 123, True, None, {"obj": "ect"}],
        # complex nested structures
        {
            "id": 1,
            "name": "Alice",
            "email": "alice@example.com",
            "profile": {
                "age": 30,
                "preferences": {
                    "theme": "dark",
                    "notifications": True,
                    "languages": ["en", "fr", "es"],
                },
                "metadata": None,
            },
            "roles": ["admin", "user"],
        },
        # deeply nested arrays and objects
        {
            "level1": {
                "level2": {
                    "level3": {
                        "level4": {
                            "level5": [
                                {"data": [1, 2, [3, 4, [5, 6]]]},
                                {"more": {"even": {"deeper": {"nesting": True}}}},
                            ]
                        }
                    }
                }
            }
        },
        # unicode and special characters
        {
            "unicode": "Hello 世界 🌍 🚀",
            "special_chars": "!@#$%^&*()_+-=[]{}|;':\",./<>?",
            "escaped": "line1\nline2\ttab\"quote'apostrophe\\backslash",
            "empty_string": "",
            "whitespace": "   \n\t\r   ",
        },
        # edge cases with numbers
        {
            "integers": [0, -1, 1, 2147483647, -2147483648],
            "floats": [0.0, -0.0, 1.5, -1.5, 1e10, 1e-10, math.inf, -math.inf],
            "scientific": [1.23e-4, 5.67e8, -9.87e-6],
        },
        # complex boolean and null patterns
        {
            "matrix": [[True, False, None], [None, True, False], [False, None, True]],
            "flags": {
                "enabled": True,
                "disabled": False,
                "unknown": None,
                "nested_flags": {"a": True, "b": False, "c": None},
            },
        },
    ]

    encoder = BinaryEncoder()
    for value in test_cases:
        encoder.write_json(value)

    decoder = BinaryDecoder(buffer=encoder.to_bytes())
    for expected in test_cases:
        result = decoder.read_json()
        assert result == expected
    assert decoder.remaining == 0
