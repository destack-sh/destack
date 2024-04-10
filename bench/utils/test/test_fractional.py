import pytest

from bench.utils.fractional import BASE_62_DIGITS, get_order_key


# noinspection Assert
@pytest.mark.parametrize(
    "test_case",
    """| | a0
| a0 Zz
a0 | a1
a0 a1 a0V
a0V a1 a0l
Zz a0 ZzV
Zz a1 a0
| Y00 Xzzz
bzz | c000
a0 a0V a0G
a0 a0G a08
b125 b129 b127
a0 a1V a1
Zz a01 a0
| a0V a0
| b999 b99
| A00000000000000000000000000 !error
| A000000000000000000000000001 A000000000000000000000000000V
zzzzzzzzzzzzzzzzzzzzzzzzzzy | zzzzzzzzzzzzzzzzzzzzzzzzzzz
zzzzzzzzzzzzzzzzzzzzzzzzzzz | zzzzzzzzzzzzzzzzzzzzzzzzzzzV
a00 | !error
a00 a1 !error
0 1 !error
a1 a0 !error""".split(
        "\n"
    ),
)
def test_order_keys(test_case: str) -> None:
    def _map_test_arg(x: str) -> str | None:
        if x == "[":
            return ""
        elif x == "]" or x == "|":
            return None
        else:
            return x

    test_args = [_map_test_arg(c) for c in test_case.split(" ")]
    expected = test_args[-1]
    if expected == "!error":
        with pytest.raises(ValueError):
            get_order_key(*test_args[:-1], BASE_62_DIGITS)
    else:
        actual = get_order_key(*test_args[:-1], BASE_62_DIGITS)
        assert actual == expected
