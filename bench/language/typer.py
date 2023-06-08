from __future__ import annotations

from typing import Any, Callable, Collection, Mapping, Union

from bench.language import TypeTag
from bench.language.const import PRIMITIVE_TYPES, TypeFlag
from bench.language.type import RemoteObject, TypeNode
from bench.utils.utils import to_pyidentifier

PyValueType = Union[int, float, bool, str, dict, list]


class TypeError(TypeError):
    def __init__(
        self, value: Any, expected: TypeNode, message: str = None, suberrors: list[TypeError] = None
    ):
        super().__init__(f"{message or 'type mismatch'}: expected {expected}, got {value}")
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
        return valid

    if expected.flags & TypeFlag.IsArray and not ignore_array:
        if _check(isinstance(value, Collection), "expected array"):
            for item in value:
                check_type(
                    item,
                    expected,
                    eager_error=eager_error,
                    on_invalid=on_invalid,
                    ignore_array=True,
                )
    elif expected.tag == TypeTag.STRING:
        _check(isinstance(value, str), "expected string")
    elif expected.tag == TypeTag.NUMBER:
        _check(isinstance(value, (int, float)), "expected number")
    elif expected.tag == TypeTag.BOOLEAN:
        _check(isinstance(value, bool), "expected boolean")
    elif expected.tag == TypeTag.ENUM:
        # assumes literal/value enums
        _check(any(member.name == value for member in expected.fields), "expected enum member")
    elif expected.tag == TypeTag.STRUCT or expected.tag == TypeTag.FUNCTION:
        if expected.tag == TypeTag.FUNCTION and is_output and not expected.outputs:
            value = value or {}  # None is allowed for empty outputs
        if _check(isinstance(value, Mapping), "expected struct"):
            for field in expected.fields:
                if is_output is not None and bool(field.flags & TypeFlag.IsOutput) != is_output:
                    continue
                alt_name = to_pyidentifier(field.name)
                subvalue = value.get(field.name, value.get(alt_name))
                if subvalue is None:
                    _check(bool(field.flags & TypeFlag.IsNullable), "expected non-nullable value")
                else:
                    check_type(subvalue, field, eager_error=eager_error, on_invalid=on_invalid)
    elif expected.tag in (TypeTag.FILE,):
        _check(isinstance(value, RemoteObject), "expected remote object")
    elif expected.tag == TypeTag.UNION:
        for option in expected.fields:
            try:
                check_type(value, option)
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


def _map_v_noop(v, t):
    return v


def _map_k_noop(t):
    return t.name, t.name


def map_value(
    value: Any,
    type: TypeNode,
    map_v: Callable[[Any, TypeNode, bool], Any] = None,
    map_k: Callable[[TypeNode], tuple[str, str]] = None,
    is_output: bool = None,
    ignore_array: bool = False,
    ignore_outer_map: bool = False,
):
    """Walks the value and reassembles with new keys and values."""
    map_v = map_v or _map_v_noop
    map_k = map_k or _map_k_noop
    # communicate via yield/send
    if type.flags & TypeFlag.IsArray and not ignore_array:
        if not isinstance(value, Collection):
            return value  # type error, ignore here
        return [map_value(item, type, map_v, map_k, ignore_array=True) for item in value]
    elif type.tag in PRIMITIVE_TYPES:
        return map_v(value=value, type=type, ignore_array=ignore_array)
    elif type.tag == TypeTag.ENUM:
        return map_v(value=value, type=type, ignore_array=ignore_array)
    elif type.tag not in (TypeTag.STRUCT, TypeTag.FUNCTION):
        raise TypeError(value, type, "expected struct-like")
    if not isinstance(value, Mapping):
        return value  # type error, ignore here
    mapped = {}
    for subtype in type.fields:
        if subtype.flags & TypeFlag.IsUnionWith:  # unresolved union
            raise RuntimeError(f"unexpected union with {type}->{subtype}")
        if is_output is not None and bool(subtype.flags & TypeFlag.IsOutput) != is_output:
            continue
        source_k, target_k = map_k(subtype)
        if source_k not in value:
            continue  # ignore missing keys
        target_value = map_value(value[source_k], subtype, map_v, map_k)
        mapped[target_k] = target_value
    if not ignore_outer_map:
        mapped = map_v(value=mapped, type=type, ignore_array=ignore_array)
    return mapped


def map_unkey_enum(value: Any, type: TypeNode, *args, **kwargs):
    if type.tag == TypeTag.ENUM:
        return type[value].name
    return value


def map_rekey_enum(value: Any, type: TypeNode, *args, **kwargs):
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
    ignore_array: bool = False,
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
