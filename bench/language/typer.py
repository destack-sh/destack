from __future__ import annotations

from typing import Any, Union

from bench.language.type import TypeNode, TypeTag

PyValueType = Union[int, float, bool, str, dict, list]


class TypeError(TypeError):
    def __init__(self, value: Any, expected: TypeNode, message: str = None):
        super().__init__(f"expected {expected}, got {value}")
        self.value = value
        self.expected = expected
        self.message = message


def check_type(value: Any, expected: TypeNode):
    def _check(valid: bool, message: str):
        if not valid:
            raise TypeError(value, expected, message)

    if expected.type == TypeTag.STRING:
        _check(isinstance(value, str), "expected string")
    elif expected.type == TypeTag.NUMBER:
        _check(isinstance(value, (int, float)), "expected number")
    elif expected.type == TypeTag.BOOLEAN:
        _check(isinstance(value, bool), "expected boolean")
    elif expected.type == TypeTag.ARRAY:
        _check(isinstance(value, list), "expected array")
        for item in value:
            check_type(item, expected.children[0])
    elif expected.type == TypeTag.ENUM:
        # assumes literal/value enums
        _check(any(member.value == value for member in expected.members), "expected enum member")
    elif expected.type == TypeTag.STRUCT:
        _check(isinstance(value, dict), "expected struct")
        for subtype in expected.children:
            check_type(value[subtype.name], subtype)
    elif expected.type == TypeTag.UNION:
        for subtype in expected.children:
            try:
                check_type(value, subtype)
                return
            except TypeError:
                pass
        raise TypeError(value, expected, "expected one of the union types")
    elif expected.type == TypeTag.NULL:
        _check(value is None, "expected null")
    elif expected.type == TypeTag.ANY:
        pass
    else:
        raise RuntimeError(f"unexpected type {expected.type}")
