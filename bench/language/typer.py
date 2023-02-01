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

    if expected.tag == TypeTag.STRING:
        _check(isinstance(value, str), "expected string")
    elif expected.tag == TypeTag.NUMBER:
        _check(isinstance(value, (int, float)), "expected number")
    elif expected.tag == TypeTag.BOOLEAN:
        _check(isinstance(value, bool), "expected boolean")
    elif expected.tag == TypeTag.ARRAY:
        _check(isinstance(value, list), "expected array")
        for item in value:
            check_type(item, expected.children[0])
    elif expected.tag == TypeTag.ENUM:
        # assumes literal/value enums
        _check(any(member.value == value for member in expected.members), "expected enum member")
    elif expected.tag == TypeTag.STRUCT:
        _check(isinstance(value, dict), "expected struct")
        for subtype in expected.children:
            check_type(value[subtype.name], subtype)
    elif expected.tag == TypeTag.UNION:
        for subtype in expected.children:
            try:
                check_type(value, subtype)
                return
            except TypeError:
                pass
        raise TypeError(value, expected, "expected one of the union types")
    elif expected.tag == TypeTag.NULL:
        _check(value is None, "expected null")
    elif expected.tag == TypeTag.ANY:
        pass
    elif expected.tag == TypeTag.LITERAL:
        _check(value == expected.value, "expected literal")
    else:
        raise RuntimeError(f"unexpected type {expected.tag}")
