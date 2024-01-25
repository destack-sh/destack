from copy import deepcopy
from dataclasses import is_dataclass
from datetime import datetime
from functools import partial
from typing import TYPE_CHECKING, Any, Collection, Optional, Callable, Mapping, Iterable
from uuid import UUID

import structlog

from bench.language.node import NS, UNSET, Node, node_component, struct_property, ScopeNode
from bench.language.text import Text
from bench.language.validation import ValidationHandler
from bench.sql.core import ColumnType
from bench.utils.proxy import proxy_value, unproxy_value

if TYPE_CHECKING:
    from bench.language import HasFields, NodeVisitor, Session
    from bench.language.field import TypeInfo, Field

logger = structlog.get_logger(__name__)


@node_component
class HasValue(Node):
    value: Any | None = struct_property(
        UNSET, default=None, copy=deepcopy, column_type=ColumnType.JSON
    )

    @property
    def _type_of_value(self) -> Optional["HasFields"]:
        return self  # assume this is a statement with fields

    def _validate_inner(self, properties: Collection[str], on_invalid: "ValidationHandler") -> None:
        # type may not be ready if not attached (e.g. Record in a Database)
        if "value" in properties and self._type_of_value is not None:
            try:
                get_k = (
                    lambda f: f.py_ident if self._status == NS.ACTIVE else f._storage_key
                )  # noqa
                check_type(self.value or {}, self._type_of_value, get_k=get_k)
            except TypeError as e:
                on_invalid(self, f"invalid value: {e}", ["value"])

    def _visit_inner(self, visitor: "NodeVisitor") -> None:
        if self.value:  # :VisitValue
            for n in walk_value(self.value, self._type_of_value):  # :VisitValue
                if isinstance(n, Node):
                    visitor.visit_reference(n)
                elif isinstance(n, Text):
                    for mention in n.mentions:
                        if isinstance(mention.reference, Node):
                            visitor.visit_reference(mention.reference)

    def _attached_inner(self) -> None:
        # pack this value if it couldn't be packed in deactivate/detach
        #  (e.g. the type wasn't available on instantiation)
        assert self._type_of_value is not None, f"missing type for {self!r}"
        is_packed = (
            self._type_of_value.fields
            and self.value
            and any(self.value.get(k.py_ident) for k in self._type_of_value.fields)
        )
        if is_packed and self.module:
            check_type(self.value, self._type_of_value)
            value = pack_value(
                self.value, self._type_of_value, ignore_outer=True, none_if_invalid=True
            )
            self._set_untracked("value", value)

    def _activate_inner(self, session: "Session") -> None:
        if self.value is None:
            return
        if not self.attached and not self._type_of_value:
            # don't activate new nodes that don't have a type from yet (e.g., Records)
            return

        def _onwrite_value(key: str) -> None:
            check_type(self.value, self._type_of_value)
            if self.attached:
                self.session.update(self, ["value"])

        assert self._type_of_value is not None, f"missing type for {self!r}"
        if self.attached:
            value = unpack_value(
                self.value, self._type_of_value, session=self._session, ignore_outer=True
            )
        else:
            value = self.value  # don't unpack if not attached (user sets 'unpacked' values)
        value = TypedDict(value, self._type_of_value)
        value = proxy_value(value, onread=lambda *args: None, onwrite=_onwrite_value)
        self._set_untracked("value", value)

    def _deactivate_inner(self) -> None:
        self._set_untracked("value", self._raw_value())

    def _raw_value(self, _force: bool = False) -> dict | None:
        """The raw/stripped value with field keys."""
        if self.value is None:
            return None
        if not _force and (self._status != NS.ACTIVE or not self.attached):
            return unproxy_value(self.value)
        assert self._type_of_value is not None, f"missing type for {self!r}"
        return pack_value(self.value, self._type_of_value, ignore_outer=True, none_if_invalid=True)


def on_invalid_raise(
    value: Any, expected: "TypeInfo", message: str = None, suberrors: list[TypeError] = None
):
    raise TypeError(value, expected, message, suberrors)


def _map_v_noop(value: Any, *args, **kwargs):
    return value


def _map_k_noop(field: Field):
    return field.name, field.name


def map_value(
    value: Any,
    type: "TypeInfo",
    map_v: Callable[[Any, "Field", bool], Any] = None,
    map_k: Callable[["Field"], tuple[str, str]] = None,
    premap_v: Callable[[Any, "Field", bool], Any] = None,
    ignore_array: bool = False,
    ignore_outer: bool = False,
    none_if_invalid: bool = False,
    ignore_empty: bool = True,
) -> Any | None:
    """Walks the value and reassembles with new keys and values."""
    map_v = map_v or _map_v_noop
    map_k = map_k or _map_k_noop

    if premap_v:
        value = premap_v(value, type, ignore_array)

    if type.is_array and not ignore_array:
        if not isinstance(value, Collection) or isinstance(value, str):
            # type error, ignore here
            return None if none_if_invalid else value
        return [
            map_value(
                item,
                type,
                map_v,
                map_k,
                premap_v,
                ignore_array=True,
                none_if_invalid=none_if_invalid,
                ignore_empty=ignore_empty,
            )
            for item in value
        ]
    elif type._effective_tag in PRIMITIVE_TYPES or type._effective_tag == TypeTag.ENUM:
        return map_v(value=value, type=type, ignore_array=ignore_array)
    elif type._effective_tag == TypeTag.JSON:
        return value  # nothing to do ?
    elif type._effective_tag not in (TypeTag.STRUCT, TypeTag.FUNCTION):
        raise RuntimeError(f"expected struct-like {type} at {value}")
    if not isinstance(value, Mapping):
        # type error, ignore here
        return None if none_if_invalid else value
    else:
        # map into a dict
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
                    subtype,
                    map_v,
                    map_k,
                    premap_v,
                    ignore_empty=ignore_empty,
                    none_if_invalid=none_if_invalid,
                )
            mapped[target_k] = target_value
        if not ignore_outer:
            mapped = map_v(value=mapped, type=type, ignore_array=ignore_array)
        return mapped


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
    elif type._effective_tag in PRIMITIVE_TYPES or type._effective_tag == TypeTag.ENUM:
        yield value
        return
    elif type._effective_tag == TypeTag.JSON:
        return  # nothing to do ?
    elif type._effective_tag not in (TypeTag.STRUCT, TypeTag.FUNCTION):
        raise RuntimeError(f"expected struct-like {type} at {value}")
    if not isinstance(value, Mapping) and not is_dataclass(value):
        return  # type error, ignore here

    assert type._status >= NS.INTERP, f"unexpected unresolved type {type}"
    for subtype in type.fields:
        k = get_k(subtype)
        if k not in value:
            continue
        yield from walk_value(value[k], subtype, get_k=get_k)


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
    if type.is_optional and value is None:
        return
    elif type.is_array and not ignore_array:
        if _check(isinstance(value, Collection)):
            for item in value:
                check_type(item, type, get_k=get_k, on_invalid=on_invalid, ignore_array=True)
        return

    # basic instance value check
    mapper = get_type_mapper_by_type(type)
    if not _check(mapper.is_instance_value(type, value)):
        return

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


def is_instance_value_flat(value: Any, type: TypeInfo) -> bool:
    raise NotImplementedError("nocheckin")


def unpack_value_flat(
    value: Any,
    type: "TypeInfo",
    scope: Optional[ScopeNode] = None,
    session: Optional["Session"] = None,
    ignore_array: bool = False,
) -> Any:
    """Maps to the proper Python representation of the given value."""
    if value is None:  # skip null values
        return None  # type checking is done elsewhere
    scope = scope or type.scope
    session = session or type._session
    if scope is None:
        raise ValueError(f"cannot unpack without scope: {type!r}")
    # we longer coerce list/non-list values (doesn't work with real databases) :NoArrayCoercion
    mapping = get_type_mapper_by_type(type)
    try:
        if type.is_array and not ignore_array:  # must be list
            return [mapping.unpack_value(type, scope, session, v) for v in value]
        else:  # must be element
            return mapping.unpack_value(type, scope, session, value)
    except (KeyError, ValueError, TypeError):
        logger.warning(
            "unpack.failed", exc_info=True, value=value, type=type, scope=scope, session=session
        )
        return value  # type checking is done elsewhere


def pack_value_flat(value: Any, type: "TypeInfo", *args, **kwargs) -> Any:
    """Maps back to the raw value from the Python representation."""
    # we don't auto-coerce here since that's only needed for external data
    if value is None:
        return None
    mapping = get_type_mapper_by_type(type)
    if not is_instance_value_flat(type, value):
        return None  # type-checking is done elsewhere
    return mapping.pack_value(type, value)


def render_value_flat(value: Any, type: "TypeInfo", *args, **kwargs) -> str:
    """Renders the given value as a string."""
    raise NotImplementedError


def unpack_value(
    value: Any,
    type: "TypeInfo",
    # TODO @Cleanup: always pass unpacking scope explicitly?
    scope: Optional[ScopeNode] = None,
    session: Optional[Session] = None,
    ignore_array: bool = False,
    ignore_outer: bool = False,
    ignore_empty: bool = True,
    map_k: Callable[[Field], tuple[str, str]] = None,
):
    """Unpacks/deserializes the given value into a Python/Bench representation."""
    scope = scope or type.scope
    if scope is None:
        raise ValueError(f"cannot unpack without scope: {type!r}")
    return map_value(
        value=value,
        type=type,
        map_k=map_k or (lambda f: (f._storage_key, f.py_ident)),
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
    map_k: Callable[[Field], tuple[str, str]] = None,
    filter_v: Callable[[Any], bool] = None,
):
    """Packs/serializes the given value into a JSON-able representation."""
    if filter_v:

        def _filtered_pack_value(value: Any, type: "TypeInfo", *args, **kwargs) -> Any:
            if not filter_v(value):
                return None
            return pack_value_flat(value, type, *args, **kwargs)

        map_v = _filtered_pack_value
    else:
        map_v = pack_value_flat
    return map_value(
        value=value,
        type=type,
        map_k=map_k or (lambda f: (f.py_ident, f._storage_key)),
        map_v=map_v,
        ignore_array=ignore_array,
        ignore_outer=ignore_outer,
        ignore_empty=ignore_empty,
        none_if_invalid=none_if_invalid,
    )


def _render_array(elements: Collection[str]) -> str:
    """Renders the given elements as a Python list."""
    return f"[{', '.join(elements)}]" if elements else "[]"


def _render_dict(elements: Mapping[str, str]) -> str:
    """Renders the given elements as a Python dict."""
    elements_str = ", ".join(f'"{k}": {v}' for k, v in elements.items())
    return f"{{{elements_str}}}" if elements else "{}"


def render_value(
    value: Any,
    type: "TypeInfo",
    get_k: Callable[[Field], str] = None,
    filter_v: Callable[[Any, Field], bool] = None,
    ignore_array: bool = False,
    ignore_empty: bool = True,
) -> str:
    """Renders the given value as a Python string."""
    get_k = get_k or (lambda f: f.py_ident)

    if type.flags & TypeFlag.IS_ARRAY and not ignore_array:
        if not isinstance(value, Collection) or isinstance(value, str):
            return repr(value)  # not sure what to do here?
        elements = [
            render_value(
                item,
                type,
                get_k=get_k,
                filter_v=filter_v,
                ignore_array=True,
                ignore_empty=ignore_empty,
            )
            for item in value
            if filter_v is None or filter_v(item, type)
        ]
        return _render_array(elements)
    elif type.column_type != TypeTag.STRUCT:
        assert filter_v is None or filter_v(value, type), f"unexpected filtered value {value}"
        return render_value_flat(value, type, filter_k=filter_v)
    else:
        # map struct-like types into a dict
        elements = {}
        for subtype in type.fields:
            if filter_v and not filter_v(value, subtype):
                continue
            k = get_k(subtype)
            if k not in value:
                if ignore_empty:
                    continue
                elements[k] = "None"
            else:
                elements[k] = render_value(
                    value[k],
                    subtype,
                    get_k=get_k,
                    filter_v=filter_v,
                    ignore_empty=ignore_empty,
                )
        return _render_dict(elements)


# type tags
_register_mapper(StringMapper(str, TypeTag.STRING), tags=[TypeTag.STRING])
_register_mapper(
    StaticPyTypeMapper(float, TypeTag.NUMBER, alt_py_types=[int]), tags=[TypeTag.NUMBER]
)
_register_mapper(StaticPyTypeMapper(bool, TypeTag.BOOLEAN), tags=[TypeTag.BOOLEAN])
_register_mapper(VectorTypeMapper(), tags=[TypeTag.VECTOR])
_register_mapper(EnumMapper(), tags=[TypeTag.ENUM])
_register_mapper(NodeMapper(), tags=[TypeTag.NODE])
_register_mapper(JsonMapper(), tags=[TypeTag.JSON])
_register_mapper(StringifyMapper(UUID, TypeTag.STRING, hint=TypeHint.UUID), hints=[TypeHint.UUID])
_register_mapper(IsoDtTypeMapper(datetime, TypeTag.STRING), hints=[TypeHint.DATETIME])
_register_mapper(
    StaticPyTypeMapper(int, TypeTag.NUMBER, hint=TypeHint.INTEGER), hints=[TypeHint.INTEGER]
)
