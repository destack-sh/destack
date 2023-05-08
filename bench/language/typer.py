from __future__ import annotations

from typing import Any, Union

from bench.language.type import PRIMITIVE_TYPES, TypeNode, TypeTag
from bench.utils.utils import to_pyidentifier

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
    value: Any,
    expected: TypeNode,
    eager_error: bool = True,
    on_invalid=on_invalid_raise,
    ignore_array: bool = False,
    is_output: bool = None,
):
    """
    Checks whether the given value has the expected type (recursively).
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

    if expected.is_array and not ignore_array:
        _check(isinstance(value, list), "expected array")
        if isinstance(value, list):  # _check may not be eager
            for item in value:
                check_type(
                    item,
                    expected.type_nodes[0],
                    eager_error=eager_error,
                    on_invalid=_on_invalid_collect,
                    ignore_array=True,
                )
    if expected.tag == TypeTag.STRING:
        _check(isinstance(value, str), "expected string")
    elif expected.tag == TypeTag.NUMBER:
        _check(isinstance(value, (int, float)), "expected number")
    elif expected.tag == TypeTag.BOOLEAN:
        _check(isinstance(value, bool), "expected boolean")
    elif expected.tag == TypeTag.ENUM:
        # assumes literal/value enums
        _check(any(member.value == value for member in expected.type_nodes), "expected enum member")
    elif expected.tag == TypeTag.STRUCT or expected.tag == TypeTag.FUNCTION:
        if expected.tag == TypeTag.FUNCTION and is_output and not expected.outputs:
            value = value or {}  # None is allowed for empty outputs
        _check(isinstance(value, dict), "expected struct")
        if isinstance(value, dict):  # _check may not be eager
            for subtype in expected.type_nodes:
                if is_output is not None and subtype.is_output != is_output:
                    continue
                alt_name = to_pyidentifier(subtype.name)
                subvalue = value.get(subtype.name, value.get(alt_name))
                check_type(
                    subvalue, subtype, eager_error=eager_error, on_invalid=_on_invalid_collect
                )
    elif expected.tag == TypeTag.UNION:
        for subtype in expected.type_nodes:
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


def unkey_value(
    value: Any, type: TypeNode, is_output: bool = None, ignore_array: bool = False
) -> Any:
    """Replaces all name 'keys' with the actual names (recursively)."""
    if type.tag in PRIMITIVE_TYPES:
        return value
    elif type.tag not in (TypeTag.STRUCT, TypeTag.FUNCTION):
        raise TypeError(value, type, "expected struct-like")
    if not isinstance(value, dict):
        return value  # type error, but ignore here
    if type.is_array and not ignore_array:
        return [unkey_value(item, type, ignore_array=True) for item in value]
    unkeyed = {}
    for subtype in type.type_nodes:
        if is_output is not None and subtype.is_output != is_output:
            continue
        if subtype.key not in value:
            return value  # ignore if it doesn't exist
        unkeyed[subtype.name] = unkey_value(value[subtype.key], subtype)
    return unkeyed


def rekey_value(
    value: Any, type: TypeNode, is_output: bool = None, ignore_array: bool = True
) -> Any:
    """Replaces all actual names with the name 'keys' (recursively)."""
    if type.tag in PRIMITIVE_TYPES:
        return value
    elif type.tag not in (TypeTag.STRUCT, TypeTag.FUNCTION):
        raise TypeError(value, type, "expected struct-like")
    if not isinstance(value, dict):
        return value  # type error, but ignore here
    if type.is_array and not ignore_array:
        return [rekey_value(item, type, ignore_array=True) for item in value]
    keyed = {}
    for subtype in type.type_nodes:
        if is_output is not None and subtype.is_output != is_output:
            continue
        if subtype.name not in value:
            return value
        keyed[subtype.key] = rekey_value(value[subtype.name], subtype)
    return keyed
