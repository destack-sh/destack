from __future__ import annotations

from typing import Any, Union

from bench.language.type import TypeNode, TypeTag

PyValueType = Union[int, float, bool, str, dict, list]


class TypeError(TypeError):
    def __init__(
        self, value: Any, expected: TypeNode, message: str = None, suberrors: list[TypeError] = None
    ):
        super().__init__(f"expected {expected}, got {value}")
        self.value = value
        self.expected = expected
        self.message = message
        self.suberrors = suberrors or []


def on_invalid_raise(
    value: Any, expected: TypeNode, message: str = None, suberrors: list[TypeError] = None
):
    raise TypeError(value, expected, message, suberrors)


def check_type(
    value: Any, expected: TypeNode, eager_error: bool = True, on_invalid=on_invalid_raise
):
    """
    Checks whether the given value has the expected type (deeply).
    Raises TypeError if not at the first issue.
    """

    _suberrors = []

    def _on_invalid_collect(
        value: Any,
        expected: TypeNode,
        message: str = None,
        suberrors: list[TypeError] = None,
    ):
        _suberrors.append(TypeError(value, expected, message, suberrors))

    def _check(valid: bool, message: str):
        if not valid:
            if eager_error:
                on_invalid(value, expected, message)
            else:
                _on_invalid_collect(value, expected, message)

    if expected.tag == TypeTag.STRING:
        _check(isinstance(value, str), "expected string")
    elif expected.tag == TypeTag.NUMBER:
        _check(isinstance(value, (int, float)), "expected number")
    elif expected.tag == TypeTag.BOOLEAN:
        _check(isinstance(value, bool), "expected boolean")
    elif expected.tag == TypeTag.ARRAY:
        _check(isinstance(value, list), "expected array")
        if isinstance(value, list):  # _check may not be eager
            for item in value:
                check_type(
                    item,
                    expected.children[0],
                    eager_error=eager_error,
                    on_invalid=_on_invalid_collect,
                )
    elif expected.tag == TypeTag.ENUM:
        # assumes literal/value enums
        _check(any(member.value == value for member in expected.members), "expected enum member")
    elif expected.tag == TypeTag.STRUCT:
        _check(isinstance(value, dict), "expected struct")
        if isinstance(value, dict):  # _check may not be eager
            for subtype in expected.children:
                check_type(
                    value.get(subtype.name),
                    subtype,
                    eager_error=eager_error,
                    on_invalid=_on_invalid_collect,
                )
    elif expected.tag == TypeTag.UNION:
        for subtype in expected.children:
            try:
                check_type(value, subtype)
                return
            except TypeError:
                pass
        _check(False, "expected one of the union types")
    elif expected.tag == TypeTag.NULL:
        _check(value is None, "expected null")
    elif expected.tag == TypeTag.ANY:
        pass
    elif expected.tag == TypeTag.LITERAL:
        _check(value == expected.value, "expected literal")
    else:
        raise RuntimeError(f"unexpected type {expected.tag}")

    if not eager_error and _suberrors:
        on_invalid(value, expected, suberrors=_suberrors)
