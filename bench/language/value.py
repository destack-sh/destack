from copy import deepcopy
from functools import partial
from typing import TYPE_CHECKING, Any, Callable, Collection, Iterable, Mapping, Optional

import structlog

from bench.language.issue import IssueHandler
from bench.language.node import (
    NS,
    UNSET,
    Node,
    ScopeNode,
    node_component,
    struct_property,
    struct_runtime,
)
from bench.language.text import Text
from bench.language.validation import ValidationHandler
from bench.sql.core import PrimitiveType
from bench.utils.proxy import proxy_value, unproxy_value

if TYPE_CHECKING:
    from bench.language import NodeVisitor, Session
    from bench.language.field import Field, TypedDict, TypeInfo

logger = structlog.get_logger(__name__)


# TODO @Cleanup: HasValue should somehow be a mixin per property :GeneralizeHasValue
#  e.g. in Run we want typed 'value' behaviour on 'value','inputs','outputs', in Field on 'default'


@node_component
class HasValue(Node):
    value_packed: Any | None = struct_property(
        UNSET, default=None, copy=deepcopy, primitive_type=PrimitiveType.JSON
    )
    # optional secret_value_packed (soon)
    value: Any | None = struct_runtime(default=None)

    @property
    def _type(self) -> Optional["TypeInfo"]:
        raise NotImplementedError(f"{self.__class__.__name__} does not implement HasValue._type")

    def _validate_inner(self, properties: Collection[str], on_invalid: "ValidationHandler") -> None:
        # type may not be ready if not attached (e.g. Record in a Database)
        if "value" in properties and self._type is not None:
            try:
                get_k = lambda f: f.py_ident if self._status == NS.ACTIVE else f.storage_key  # noqa
                check_type(self.value or {}, self._type, get_k=get_k)
            except TypeError as e:
                on_invalid(self, f"invalid value: {e}", ["value"])

    def _visit_inner(self, visitor: "NodeVisitor") -> None:
        if self.value:  # :VisitValue
            for n in walk_value(self.value, self._type):  # :VisitValue
                if isinstance(n, Node):
                    visitor.visit_reference(n)
                elif isinstance(n, Text):
                    for mention in n.mentions:
                        if isinstance(mention.reference, Node):
                            visitor.visit_reference(mention.reference)

    def _clear_inner(self, scope: Optional["ScopeNode"] = None):
        self.value = None

    def _interp_inner(self, scope: "ScopeNode", on_issue: "IssueHandler"):
        if self.value_packed is None:
            return
        if self.value is None:
            self.value = unpack_value(self.value, self._type._typ, scope=scope, ignore_outer=True)

    def _activate_inner(self, session: "Session") -> None:
        if self.value_packed is None:
            return

        def _onwrite_value(key: str) -> None:
            # TODO @Performance: type check only the changed value
            check_type(self.value, self._type)
            if self.attached:
                self.session.update(self, ["value"])

        assert self._type is not None, f"missing type for {self!r}"

        value = unpack_value(self.value, self._type, session=self._session, ignore_outer=True)
        value = TypedDict(value, self._type)
        self.value = proxy_value(value, onread=lambda *args: None, onwrite=_onwrite_value)

    def _deactivate_inner(self) -> None:
        if self.value is not None:
            self.value = unproxy_value(self.value)


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
        assert type._status >= NS.INTERP, f"unexpected unresolved type {type}"
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
    scope: ScopeNode,
    on_issue: "IssueHandler",
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
    scope: ScopeNode,
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
