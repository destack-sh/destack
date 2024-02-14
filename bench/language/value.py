from functools import partial
from typing import TYPE_CHECKING, Any, Callable, Collection, Iterable, Mapping, Optional

import structlog

from bench.language.const import StructType, NodeType
from bench.language.node import Node, NodeStatus, node_component, Property, struct, Struct
from bench.language.notice import NoticeHandler
from bench.language.property import p_internal
from bench.language.validation import ValidationHandler
from bench.sql.core import PrimitiveType

if TYPE_CHECKING:
    from bench.language import (
        NodeVisitor,
        Session,
        TypeInfo,
        Bench,
        Environment,
        Branch,
        Package,
        Block,
    )
    from bench.language.field import Field, TypeInfo

logger = structlog.get_logger(__name__)


# nocheckin: implement new value system


@node_component
class HasValues(Node):
    def _validate_inner(
        self, properties: Collection[Property], on_invalid: "ValidationHandler"
    ) -> None:
        pass

    def _visit_inner(self, visitor: "NodeVisitor") -> None:
        pass

    def _clear_inner(self, scope: Optional["Node"] = None):
        pass

    def _interp_inner(self, scope: "Node", on_notice: "NoticeHandler"):
        pass

    def _track_inner(self, session: "Session") -> None:
        pass

    def _untrack_inner(self) -> None:
        pass


@struct(StructType.CONTEXT)
class Context(Struct):
    """A semi-magical value that accumulates context down the graph (starting with system context)."""

    # system
    bench: Optional["Bench"] = p_internal(30, require=False, array=False, references=NodeType.BENCH)
    environment: Optional["Environment"] = p_internal(
        31, require=False, array=False, references=NodeType.ENVIRONMENT
    )
    branch: Optional["Branch"] = p_internal(
        32, require=False, array=False, references=NodeType.BRANCH
    )
    package: Optional["Package"] = p_internal(
        33, require=False, array=False, references=NodeType.PACKAGE
    )
    module: Optional["Block"] = p_internal(
        34, require=False, array=False, references=NodeType.BLOCK
    )
    page: Optional["Block"] = p_internal(35, require=False, array=False, references=NodeType.BLOCK)
    # log: ...

    # custom
    # value_packed: Any = p_value_packed(40)
    # secret_value_packed: Any = p_secret_value_packed(41)
    # value: Any = p_value_runtime(40, 41)


def on_invalid_raise(
    value: Any, expected: "TypeInfo", message: str = None, suberrors: list[TypeError] = None
):
    raise TypeError(value, expected, message, suberrors)


def _map_v_noop(value: Any, *args, **kwargs):
    return value


def _map_k_noop(field: "Field"):
    return field.py_ident, field.py_ident


def map_value(
    value: Any,
    type: "TypeInfo",
    map_v: Callable[[Any, "Field", bool], Any] = _map_v_noop,
    map_k: Callable[["Field"], tuple[str, str]] = _map_k_noop,
    premap_v: Callable[[Any, "Field", bool], Any] = None,
    ignore_array: bool = False,
    ignore_outer: bool = False,
    none_if_invalid: bool = False,
    ignore_empty: bool = True,
) -> Any | None:
    """Walks the value and reassembles with new keys and values."""

    if premap_v:
        value = premap_v(value, type, ignore_array)

    if type.is_array and not ignore_array:
        if not isinstance(value, Collection) or isinstance(value, str):
            # type error, ignore here
            return None if none_if_invalid else value
        return [
            map_value(
                item,
                type=type,
                map_v=map_v,
                map_k=map_k,
                premap_v=premap_v,
                ignore_array=True,
                none_if_invalid=none_if_invalid,
                ignore_empty=ignore_empty,
            )
            for item in value
        ]
    elif type.is_nested:
        if not isinstance(value, Mapping):
            return None if none_if_invalid else value
        mapped = {}
        assert type._status >= NodeStatus.INTERP, f"unexpected unresolved type {type}"
        for subtype in type.fields:
            source_k, target_k = map_k(subtype)
            if source_k not in value:
                if ignore_empty:
                    continue
                target_value = None
            else:
                target_value = map_value(
                    value[source_k],
                    type=subtype,
                    map_v=map_v,
                    map_k=map_k,
                    premap_v=premap_v,
                    ignore_empty=ignore_empty,
                    none_if_invalid=none_if_invalid,
                )
            mapped[target_k] = target_value
        if not ignore_outer:
            mapped = map_v(value=mapped, type=type, ignore_array=ignore_array)
        return mapped
    elif type.primitive_type == PrimitiveType.JSON:
        return value  # nothing to do ?
    else:  # scalar
        return map_v(value=value, type=type, ignore_array=ignore_array)


def walk_value(
    value: Any,
    type: "TypeInfo",
    get_k: Callable[["Field"], str] = None,
    ignore_array: bool = False,
) -> Iterable[Any]:
    """Yields all flat values in the instantiated value recursively."""
    get_k = get_k or (lambda f: f.py_ident)

    if type.is_array and not ignore_array:
        if not isinstance(value, Collection) or isinstance(value, str):
            return  # type error, ignore here
        for item in value:
            yield from walk_value(item, type, get_k=get_k, ignore_array=True)
        return
    elif type.is_nested:
        if not isinstance(value, Mapping):
            return
        for subtype in type.fields:
            k = get_k(subtype)
            if k not in value:
                continue
            yield from walk_value(value[k], subtype, get_k=get_k)
    else:
        yield value


def check_type(
    value: Any,
    type: "TypeInfo",
    get_k: Callable[["Field"], str] = None,
    on_invalid=on_invalid_raise,
    ignore_array: bool = False,
) -> None:
    """
    Checks whether the given value has the expected type (recursively).
    Raises TypeError if not.
    """

    get_k = get_k or (lambda f: f.py_ident)

    def _check(valid: bool, message: str = None):
        if not valid:
            on_invalid(value, type, message)
        return valid

    # optional / list types
    if not type.is_required and value is None:
        return
    elif type.is_array and not ignore_array:
        if _check(isinstance(value, Collection)):
            for item in value:
                check_type(item, type, get_k=get_k, on_invalid=on_invalid, ignore_array=True)
        return

    # basic instance value check
    if not is_instance_value_flat(value, type):
        on_invalid(value, type)

    # walk nested types
    fields = type.fields
    if fields:
        for f in fields:
            k = get_k(f)
            subvalue = getattr(value, k)
            check_type(subvalue, f, get_k=get_k, on_invalid=on_invalid)
        if hasattr(value, "keys"):
            for key in value.keys():
                exists = any(get_k(f) == key for f in fields)
                _check(exists, f"extraneous field '{key}'")


def is_instance_value_flat(value: Any, type: "TypeInfo") -> bool:
    raise NotImplementedError


def unpack_value_flat(
    value: Any,
    type: "TypeInfo",
    scope: Node,
    on_notice: "NoticeHandler",
    ignore_array: bool = False,
) -> Any:
    """Unpacks into the proper Python representation of the given packed value."""
    if value is None:
        return None
    raise NotImplementedError


def pack_value_flat(value: Any, type: "TypeInfo") -> Any:
    """Packs the value into a robust JSON value from the Python representation."""
    if value is None:
        return None
    raise NotImplementedError


def unpack_value(
    value: Any,
    type: "TypeInfo",
    scope: Node,
    session: Optional["Session"] = None,
    ignore_array: bool = False,
    ignore_outer: bool = False,
    ignore_empty: bool = True,
    map_k: Callable[["Field"], tuple[str, str]] = None,
):
    """Unpacks/deserializes the given value into a Python/Bench representation."""
    if scope is None:
        raise ValueError(f"cannot unpack without scope: {type!r}")
    return map_value(
        value=value,
        type=type,
        map_k=map_k or (lambda f: (f.storage_key, f.py_ident)),
        map_v=partial(unpack_value_flat, scope=scope, session=session),
        ignore_array=ignore_array,
        ignore_outer=ignore_outer,
        ignore_empty=ignore_empty,
    )


def pack_value(
    value: Any,
    type: "TypeInfo",
    ignore_array: bool = False,
    ignore_outer: bool = False,
    ignore_empty: bool = True,
    none_if_invalid: bool = False,
    map_k: Callable[["Field"], tuple[str, str]] = None,
    filter_v: Callable[[Any], bool] = None,
):
    """Packs/serializes the given value into a JSON-able representation."""
    if filter_v:

        def _filtered_pack_value(value: Any, type: "TypeInfo") -> Any:
            if not filter_v(value):
                return None
            return pack_value_flat(value, type, check=False)

        map_v = _filtered_pack_value
    else:
        map_v = pack_value_flat
    return map_value(
        value=value,
        type=type,
        map_k=map_k or (lambda f: (f.py_ident, f.storage_key)),
        map_v=map_v,
        ignore_array=ignore_array,
        ignore_outer=ignore_outer,
        ignore_empty=ignore_empty,
        none_if_invalid=none_if_invalid,
    )


class TypedDict(dict):
    """
    A dot dict based on a type.
    Errors on attribute access if the field doesn't exist, otherwise returns the value (or None).
    """

    _PROPS = ("_type", "_is_output")

    def __init__(self, d: dict, type: "TypeInfo", is_output: bool = None):
        super().__init__(**d)
        self._type = type
        self._is_output = is_output

    def __str__(self):
        return super().__str__()

    def __repr__(self):
        kwargs_str = ", ".join(f"{k}={v!r}" for k, v in self.items())
        return f"{self._type.base_type.py_ident}({kwargs_str})"

    def __getitem__(self, item):
        try:
            return dict.__getitem__(self, item)
        except KeyError:
            field = self._type.fields.get(item)
            if field and (self._is_output is None or field.is_output == self._is_output):
                return None
        raise KeyError(f"no key {item!r} on {self._type!r}")

    def __getattr__(self, item):
        if item in TypedDict._PROPS:
            return super().__getattr__(item)
        try:
            return dict.__getitem__(self, item)
        except KeyError:
            field = self._type.fields.get(item)
            if field and (self._is_output is None or field.is_output == self._is_output):
                return None
        raise AttributeError(f"no attribute {item!r} on {self._type!r}")

    def __setattr__(self, name, value):
        if name in TypedDict._PROPS:
            return super().__setattr__(name, value)

        field = self._type.fields.get(name)
        if field and (self._is_output is None or field.is_output == self._is_output):
            return dict.__setitem__(self, name, value)
        raise AttributeError(f"cannot set attribute {name!r} on {self._type!r}")


def _curry_path(onfn: Callable[[str], None], key: str) -> Callable[[str], Any]:
    return lambda path: onfn(f"{key}.{path}")


def proxy_value(
    value: Any,
    onwrite: Callable[[str], None],
    default_none: bool = False,
) -> Any:
    """Recursively proxy the given value, calling onread/onwrite when a key is accessed."""
    if isinstance(value, dict):
        return ProxyDict(value, onwrite, default_none=default_none)
    elif isinstance(value, list):
        return ProxyList(value)
    else:
        return value


def unproxy_value(value: Any) -> Any:
    """Recursively unproxy the given value."""
    if isinstance(value, ProxyDict):
        return value._inner
    elif isinstance(value, ProxyList):
        return value._inner
    else:
        return value


class ProxyDict(Mapping):
    """Proxy a dict, behave as a type dict, calling onread/onwrite when a key is accessed."""

    def __init__(
        self,
        inner: dict,
        onwrite: Callable[[str], None],
    ):
        self._inner = inner
        self._onwrite = onwrite

    def __str__(self):
        return str(self._inner)

    def __repr__(self):
        return f"<ProxyDict {self._inner}>"

    def items(self):
        return self._inner.items()

    def keys(self):
        return self._inner.keys()

    def values(self):
        return self._inner.values()

    def __getitem__(self, key: str) -> Any:
        return self._inner[key]

    def update(self, other: dict) -> None:
        for key, value in other.items():
            self[key] = value

    def __setitem__(self, key: str, value: Any) -> None:
        value = proxy_value(value, _curry_path(self._onwrite, key))
        self._inner[key] = value
        self._onwrite(key)

    def __delitem__(self, key: str) -> None:
        del self._inner[key]
        self._onwrite(key)

    def __contains__(self, key: str) -> bool:
        return key in self._inner

    def __len__(self):
        return len(self._inner)

    def __iter__(self):
        return iter(self._inner)

    # dot dict

    def __setattr__(self, item, value):
        if item in ("_inner", "_onwrite"):
            return super().__setattr__(item, value)
        self._inner[item] = value
        self._onwrite(item)

    def __getattr__(self, name):
        try:
            return self[name]
        except KeyError:
            raise AttributeError(name)


class ProxyList(Collection):
    """Proxy a list, calling onread and onwrite when a key is accessed."""

    def __init__(self, inner: list, onwrite: Callable[[int], None]):
        self._inner = inner
        self._onwrite = onwrite

    def __str__(self):
        return str(self._inner)

    def __repr__(self):
        return f"<ProxyList {self._inner}>"

    def __delitem__(self, key: int) -> None:
        self._onwrite(key)
        del self._inner[key]

    def __contains__(self, key: Any) -> bool:
        return key in self._inner

    def __getitem__(self, item: int | slice) -> Any:
        return self._inner[item]

    def __len__(self):
        return len(self._inner)

    def __iter__(self):
        return iter(self._inner)

    def append(self, value: Any) -> None:
        idx = len(self._inner)
        self._inner.append(value)
        self._onwrite(idx)

    def extend(self, value: Any) -> None:
        for v in value:
            len(self._inner)
            self._inner.append(v)
        self._onwrite("")
