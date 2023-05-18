from __future__ import annotations

from typing import Any, Callable, Collection, Mapping, Union

from bench.language.type import PRIMITIVE_TYPES, RemoteObject, TypeFlag, TypeNode, TypeTag
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

    if expected.flags & TypeFlag.IsArray and not ignore_array:
        _check(isinstance(value, Collection), "expected array")
        if isinstance(value, Collection):  # _check may not be eager
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
        _check(isinstance(value, Mapping), "expected struct")
        if isinstance(value, Mapping):  # _check may not be eager
            for subtype in expected.type_nodes:
                if is_output is not None and bool(subtype.flags & TypeFlag.IsOutput) != is_output:
                    continue
                alt_name = to_pyidentifier(subtype.name)
                subvalue = value.get(subtype.name, value.get(alt_name))
                check_type(
                    subvalue, subtype, eager_error=eager_error, on_invalid=_on_invalid_collect
                )
    elif expected.tag in (TypeTag.FILE, TypeTag.IMAGE, TypeTag.AUDIO, TypeTag.VIDEO):
        _check(isinstance(value, RemoteObject), "expected remote object")
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


def map_value(
    value: Any,
    type: TypeNode,
    map_v: Callable[[Any, TypeNode], Any] = None,
    map_k: Callable[[TypeNode], tuple[str, str]] = None,
    is_output: bool = None,
    ignore_array: bool = False,
):
    """Walks the value and reassembles with new keys and values."""
    map_v = map_v or (lambda v, t: v)
    map_k = map_k or (lambda t: (t.name, t.name))
    # communicate via yield/send
    if type.tag in PRIMITIVE_TYPES:
        return map_v(value, type)
    elif type.tag == TypeTag.ENUM:
        return map_v(value, type)
    elif type.tag not in (TypeTag.STRUCT, TypeTag.FUNCTION):
        raise TypeError(value, type, "expected struct-like")
    if not isinstance(value, Mapping):
        return value  # type error, ignore here
    if type.flags & TypeFlag.IsArray and not ignore_array:
        return [map_value(item, type, map_v, map_k, ignore_array=True) for item in value]
    mapped = {}
    for subtype in type.type_nodes:
        if is_output is not None and bool(subtype.flags & TypeFlag.IsOutput) != is_output:
            continue
        source_k, target_k = map_k(subtype)
        if source_k not in value:
            continue  # ignore missing keys
        new_value = map_value(value[source_k], subtype, map_v, map_k)
        mapped[target_k] = new_value
    return mapped


def map_unkey_enum(value: Any, type: TypeNode):
    if type.tag == TypeTag.ENUM:
        return type[value].name
    return value


def map_rekey_enum(value: Any, type: TypeNode):
    if type.tag == TypeTag.ENUM:
        return type[value].key
    return value


def unkey_value(
    value: Any,
    type: TypeNode,
    is_output: bool = None,
    ignore_array: bool = False,
    to_ident: bool = False,
) -> Any:
    """Replaces keys with actual values."""
    if to_ident:

        def map_k(t: TypeNode):
            return t.key, t.ident

    else:

        def map_k(t: TypeNode):
            return t.key, t.name

    return map_value(
        value,
        type,
        map_v=map_unkey_enum,
        map_k=map_k,
        is_output=is_output,
        ignore_array=ignore_array,
    )


def rekey_value(
    value: Any,
    type: TypeNode,
    is_output: bool = None,
    ignore_array: bool = True,
    from_ident: bool = False,
) -> Any:
    """Replaces names with keys."""
    if from_ident:

        def map_k(t: TypeNode):
            return t.ident, t.key

    else:

        def map_k(t: TypeNode):
            return t.name, t.key

    return map_value(
        value,
        type,
        map_v=map_rekey_enum,
        map_k=map_k,
        is_output=is_output,
        ignore_array=ignore_array,
    )
