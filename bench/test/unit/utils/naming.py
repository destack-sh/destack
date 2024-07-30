import pytest
from hypothesis import assume, given
from hypothesis import strategies as st

from bench.utils.string import Casing, to_casing, to_py_name


@given(st.text())
def test_to_py_name(s: str):
    assume(len(s) > 0)
    ident = to_py_name(s)
    assert ident.isidentifier()


@pytest.mark.parametrize(
    argnames=("subject", "casing", "expected"),
    argvalues=(
        # snake
        ("HelloWorld", Casing.SNAKE, "hello_world"),
        ("123 This is aTest", Casing.SNAKE, "_this_is_a_test"),
        ("ALL CAPS ALL DAY", Casing.SNAKE, "all_caps_all_day"),
        ("camelCase", Casing.SNAKE, "camel_case"),
        ("kebab-case", Casing.SNAKE, "kebab_case"),
        # (upper) camel
        ("HelloWorld", Casing.CAMEL, "HelloWorld"),
        ("123 This is aTest", Casing.CAMEL, "ThisIsATest"),
        ("ALL CAPS ALL DAY", Casing.CAMEL, "AllCapsAllDay"),
        ("camelCase", Casing.CAMEL, "CamelCase"),
        ("kebab-case", Casing.CAMEL, "KebabCase"),
        # all caps
        ("HelloWorld", Casing.ALL_CAPS, "HELLO_WORLD"),
        ("123 This is aTest", Casing.ALL_CAPS, "_THIS_IS_A_TEST"),
        ("ALL CAPS ALL DAY", Casing.ALL_CAPS, "ALL_CAPS_ALL_DAY"),
        ("camelCase", Casing.ALL_CAPS, "CAMEL_CASE"),
        ("kebab-case", Casing.ALL_CAPS, "KEBAB_CASE"),
    ),
)
def test_casing(subject: str, casing: Casing, expected: str):
    assert to_casing(subject, casing) == expected


@pytest.mark.parametrize(
    argnames=("subject", "casing", "expected"),
    argvalues=(
        # snake
        ("123 This is aTest", Casing.SNAKE, "this is a test"),
        ("ALL CAPS ALL DAY", Casing.SNAKE, "all caps all day"),
        # (upper) camel
        ("123 This is aTest", Casing.CAMEL, "This Is A Test"),
        ("ALL CAPS ALL DAY", Casing.CAMEL, "All Caps All Day"),
        # all caps
        ("123 This is aTest", Casing.ALL_CAPS, "THIS IS A TEST"),
        ("ALL CAPS ALL DAY", Casing.ALL_CAPS, "ALL CAPS ALL DAY"),
    ),
)
def test_casing_whitespace(subject: str, casing: Casing, expected: str):
    assert to_casing(subject, casing, allow_whitespace=True) == expected
