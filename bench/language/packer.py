import json
from dataclasses import dataclass, field, is_dataclass
from datetime import date, datetime, time
from functools import partial
from typing import (
    Any,
    Callable,
    Collection,
    Iterable,
    Mapping,
    NamedTuple,
    Optional,
    Union,
)
from uuid import UUID

import pytz
import structlog

from bench.language.const import TypeFlag, TypeHint, TypeTag
from bench.language.expression import TYPE_DISCRIMINATOR_KEY
from bench.language.field import (
    PRIMITIVE_TYPES,
    TYPE_TAG_BY_TYPE_HINT,
    Field,
    HasFields,
    HasType,
    Key,
    TypedDict,
    TypeError,
    Vector,
)
from bench.language.module import NS, Node, ScopeNode
from bench.language.session import Session
from bench.language.statement import Statement
from bench.language.text import Text, parse_text_multi, render_text_html, render_text_simple
from bench.utils.func import strip_py_type, try_to_uuid

logger = structlog.get_logger(__name__)


def on_invalid_raise(
    value: Any, expected: HasType, message: str = None, suberrors: list[TypeError] = None
):
    raise TypeError(value, expected, message, suberrors)


def _map_v_noop(value: Any, *args, **kwargs):
    return value


def _map_k_noop(field: Field):
    return field.name, field.name


def _is_arrayable_not_an_array(type: Field, value: Any) -> bool:
    return (
        not isinstance(value, Collection)
        or isinstance(value, (str, bytes))
        or (
            type._effective_tag == TypeTag.VECTOR
            and isinstance(value, list)
            and (not value or isinstance(value[0], (float, int)))
        )
    )


def map_value(
    value: Any,
    type: "Statement",
    map_v: Callable[[Any, Field, bool], Any] = None,
    map_k: Callable[[Field], tuple[str, str]] = None,
    premap_v: Callable[[Any, Field, bool], Any] = None,
    is_output: bool = None,
    ignore_array: bool = False,
    ignore_outer: bool = False,
    none_if_invalid: bool = False,
    ignore_empty: bool = True,
):
    """Walks the value and reassembles with new keys and values."""
    map_v = map_v or _map_v_noop
    map_k = map_k or _map_k_noop

    if premap_v:
        value = premap_v(value, type, ignore_array)

    if type.flags & TypeFlag.IS_ARRAY and not ignore_array:
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
    elif type.flags & TypeFlag.IS_ARRAYABLE and not ignore_array:
        if _is_arrayable_not_an_array(type, value):
            return map_value(
                value,
                type,
                map_v,
                map_k,
                ignore_array=True,
                ignore_empty=ignore_empty,
                none_if_invalid=none_if_invalid,
            )
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
    if not isinstance(value, Mapping) and not is_dataclass(value):
        # type error, ignore here
        return None if none_if_invalid else value

    # map into a dict
    mapped = {}
    assert type._status >= NS.INTERP, f"unexpected unresolved type {type}"
    for subtype in type.resolved_fields:
        assert (
            not subtype.flags & TypeFlag.IS_UNION_WITH
        ), f"unexpected union with {type}->{subtype}"
        if is_output is not None and bool(subtype.flags & TypeFlag.IS_OUTPUT) != is_output:
            continue
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
    type: "Statement",
    is_output: bool = None,
    get_k: Callable[[Field], str] = None,
    ignore_array: bool = False,
) -> Iterable[Any]:
    """Yields all flat values in the value recursively."""
    get_k = get_k or (lambda f: f.py_ident)

    if type.flags & TypeFlag.IS_ARRAY and not ignore_array:
        if not isinstance(value, Collection) or isinstance(value, str):
            return  # type error, ignore here
        for item in value:
            yield from walk_value(item, type, get_k=get_k, ignore_array=True)
        return
    elif type.flags & TypeFlag.IS_ARRAYABLE and not ignore_array:
        if _is_arrayable_not_an_array(type, value):
            yield from walk_value(value, type, get_k=get_k, ignore_array=True)
            return
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
    for subtype in type.resolved_fields:
        assert (
            not subtype.flags & TypeFlag.IS_UNION_WITH
        ), f"unexpected union with {type}->{subtype}"
        if is_output is not None and bool(subtype.flags & TypeFlag.IS_OUTPUT) != is_output:
            continue
        k = get_k(subtype)
        if k not in value:
            continue
        yield from walk_value(value[k], subtype, get_k=get_k)


OMITTED_SENTINEL = "__omitted"  # :OmittedSentinel
BLOB_TYPENAME = "Blob"
SECRET_TYPENAME = "Secret"
TypeSignature = NamedTuple(
    "TypeSignature", [("tag", TypeTag), ("hint", Optional[TypeHint]), ("flags", TypeFlag)]
)


def check_type(
    value: Any,
    type: HasType,
    get_k: Callable[[Field], str] = None,
    on_invalid=on_invalid_raise,
    is_output: bool = None,
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
    if type.flags & TypeFlag.IS_OPTIONAL and value is None:
        return
    elif type.flags & TypeFlag.IS_ARRAY and not ignore_array:
        if _check(isinstance(value, Collection)):
            for item in value:
                check_type(item, type, get_k=get_k, on_invalid=on_invalid, ignore_array=True)
        return
    elif (
        type.flags & TypeFlag.IS_ARRAYABLE
        and not ignore_array
        and not _is_arrayable_not_an_array(type, value)
    ):
        for item in value:
            check_type(item, type, get_k=get_k, on_invalid=on_invalid, ignore_array=True)
        return

    # basic instance value check
    mapper = get_type_mapper_by_type(type)
    if not _check(mapper.is_instance_value(type, value)):
        return

    # walk struct-like types
    if type._effective_tag == TypeTag.STRUCT or type._effective_tag == TypeTag.FUNCTION:
        if type.tag == TypeTag.FUNCTION and is_output:
            value = value or {}  # None is allowed for empty outputs
        is_dc = is_dataclass(value)
        for f in type.resolved_fields:
            if is_output is not None and bool(f.flags & TypeFlag.IS_OUTPUT) != is_output:
                continue
            k = get_k(f)
            if is_dc:
                subvalue = getattr(value, k)
            else:
                subvalue = value.get(k)
            check_type(subvalue, f, get_k=get_k, on_invalid=on_invalid)
        if hasattr(value, "keys"):
            for key in value.keys():
                exists = any(get_k(f) == key for f in type.resolved_fields)
                _check(exists, f"extraneous field '{key}'")


StatementOrField = Union["Statement", "Field"]


class TypeMapper:
    """
    Maps specific types (and values) into and from Python.
    Values and types are flattened for mapping, so ignore lists/optionals/etc.
    """

    def __str__(self):
        return self.__class__.__name__

    def is_instance_type(self, py_type: type) -> bool:
        """Whether this mapper can represent the given Python instance type."""
        raise NotImplementedError(f"{self!r} does not support this for {py_type!r}")

    def is_instance_value(self, type: HasType, value: Any) -> bool:
        """
        Whether this mapper can represent the given Python instance value.
        For nested types (like structs) this only checks the top-level value (no walking).
        """
        raise NotImplementedError(f"{self!r} does not support this for {type!r}")

    def unpack_value(
        self, type: HasType, scope: ScopeNode, session: Optional[Session], value: Any
    ) -> Any:
        """Converts and coerces a raw flat value of the type into an instance value."""
        return value

    def pack_value(self, type: HasType, value: Any) -> Any:
        """Converts a value of the given instance type back into a flat value."""
        return value

    def render_python(self, type: HasType, value: Any) -> str:
        """Renders an unpacked Python value as a string to reconstruct that value."""
        raise NotImplementedError(f"{self!r} does not support this for {type!r}")


type_mappers: dict[TypeSignature, TypeMapper] = {}


def register_mapper(
    mapping: TypeMapper,
    *,
    tags: list[TypeTag] = None,
    hints: list[TypeHint] = None,
    flags: TypeFlag = None,
):
    if not tags and not hints:
        raise ValueError("at least one tag or hint must be specified")

    def _register(signature: TypeSignature):
        if signature in type_mappers:
            raise ValueError(
                f"mapper for {signature} already registered: {type_mappers[signature]}"
            )
        type_mappers[signature] = mapping

    tags = tags or []
    hints = hints or []
    flags = flags or TypeFlag.ZERO
    for tag in tags:
        _register(TypeSignature(tag, None, flags))
    for hint in hints:
        tag = TYPE_TAG_BY_TYPE_HINT[hint]
        _register(TypeSignature(tag, hint, flags))


def get_type_mapper_by_type(type: HasType) -> TypeMapper:
    """
    Gets the most appropriate mapping for the given type.
    (flat because we ignore list and optional types).
    """
    # strip to only relevant flags for mapping
    stripped_flags = type.flags & TypeFlag.IS_SECRET
    exact_signature = TypeSignature(type._effective_tag, type._effective_hint, stripped_flags)
    mapping = type_mappers.get(exact_signature)
    if mapping is not None:
        return mapping
    # no exact match, try generic without hint
    stripped_signature = TypeSignature(type._effective_tag, None, stripped_flags)
    mapping = type_mappers.get(stripped_signature)
    if mapping is not None:
        return mapping
    raise LookupError(f"no mapping found for {type}")


def _strip_py_type(py_type: type) -> tuple[type, TypeFlag]:
    py_type, info = strip_py_type(py_type)
    flags = TypeFlag.ZERO
    if info.is_optional:
        flags |= TypeFlag.IS_OPTIONAL
    if info.is_arrayable:
        flags |= TypeFlag.IS_ARRAYABLE
    if info.is_array:
        flags |= TypeFlag.IS_ARRAY
    return py_type, flags


@dataclass
class StaticPyTypeMapper(TypeMapper):
    py_type: type
    py_type_raw: type = field(init=False)
    tag: TypeTag
    alt_py_types: list[type] = field(default_factory=list)
    hint: Optional[TypeHint] = None

    def __post_init__(self):
        self.py_type_raw = self.py_type
        # strip newtype
        if hasattr(self.py_type, "__supertype__"):
            self.py_type_raw = self.py_type.__supertype__
        self._all_py_types = (self.py_type_raw,) + tuple(self.alt_py_types or [])

    def is_instance_value(self, type: HasType, value: Any) -> bool:
        return isinstance(value, self._all_py_types)

    def unpack_value(
        self, type: HasType, scope: ScopeNode, session: Optional[Session], value: Any
    ) -> Any:
        return self.py_type(value)

    def render_python(self, type: HasType, value: Any) -> str:
        return repr(value)


@dataclass
class StringTypeMapper(StaticPyTypeMapper):
    py_type: type = str
    tag: TypeTag = TypeTag.STRING

    def pack_value(self, type: HasType, value: Any) -> str:
        # sanitize null character
        return str(value).replace("\x00", "")

    def render_python(self, type: HasType, value: Any) -> str:
        # if it contains newlines transform into multiline string
        # and escape any multiline strings inside
        if "\n" in value:
            value = value.replace('"""', '\\"\\"\\"')
            return f'"""\\\n{value}"""'
        else:
            value = value.replace('"', '\\"')
            return repr(value)


class RichTextMapper(TypeMapper):
    def is_instance_value(self, type: HasType, value: Any) -> bool:
        return isinstance(value, (str, Text))

    def unpack_value(
        self, type: HasType, scope: ScopeNode, session: Optional[Session], value: Any
    ) -> Any:
        spans = parse_text_multi(value)
        text = Text(spans=spans, _raw_text=value)
        text._resolve(scope)
        return text

    def pack_value(self, type: HasType, value: Any) -> Any:
        if not isinstance(value, Text):
            return value
        return render_text_html(value.spans)

    def render_python(self, type: HasType, value: Any) -> str:
        return repr(render_text_simple(value.spans))


@dataclass
class StringifyTypeMapping(StaticPyTypeMapper):
    def unpack_value(
        self, type: HasType, scope: ScopeNode, session: Optional[Session], value: Any
    ) -> Any:
        return self.py_type(value)

    def pack_value(self, type: HasType, value: Any) -> str:
        return str(value)

    def render_python(self, type: HasType, value: Any) -> str:
        return repr(value)


@dataclass
class IsoDtTypeMapping(StaticPyTypeMapper):
    HINT_BY_PY_TYPE = {
        date: TypeHint.DATE,
        datetime: TypeHint.DATETIME,
        time: TypeHint.TIME,
    }

    def unpack_value(
        self, type: HasType, scope: ScopeNode, session: Optional[Session], value: Any
    ) -> Any:
        if not isinstance(value, self.py_type):
            value = self.py_type.fromisoformat(value)
        # add UTC if no timezone is specified
        if isinstance(value, datetime) and value.tzinfo is None:
            value = value.replace(tzinfo=pytz.utc)
        return value

    def pack_value(self, type: HasType, value: Any) -> str:
        # convert to UTC if no timezone is specified
        if isinstance(value, datetime) and value.tzinfo is None:
            value = value.replace(tzinfo=pytz.utc)
        return value.isoformat()

    def render_python(self, type: HasType, value: Any) -> str:
        return repr(value)


@dataclass
class EnumMapper(TypeMapper):
    def is_instance_value(self, type: HasType, value: Any) -> bool:
        if isinstance(value, str):
            # allow string values for built-in enums
            # (that also function as regular enums in code)
            return value in type.resolved_fields
        return isinstance(value, Field) and value.key in type.resolved_fields

    def unpack_value(
        self, type: HasFields, scope: ScopeNode, session: Optional[Session], value: Any
    ) -> Any:
        field_ = type.resolved_fields.get(value)
        return field_.field if field_ else value

    def pack_value(self, type: HasFields, value: Any) -> Any:
        field_ = type.resolved_fields.get(value) if not isinstance(value, Field) else value
        return field_.key if field_ else value

    def render_python(self, type: HasType, value: Any) -> str:
        field_ = type.resolved_fields.get(value) if not isinstance(value, Field) else value
        return f"{type._effective_type.py_ident}.{field_.py_ident}"


@dataclass
class VectorTypeMapper(StaticPyTypeMapper):
    py_type: type = Vector
    tag: TypeTag = TypeTag.VECTOR

    def is_instance_value(self, type: HasType, value: Any) -> bool:
        # not quite right but good enough for now
        return (
            isinstance(value, bytes)
            or isinstance(value, Collection)
            and len(value) > 0
            and isinstance(value[0], (float, int))
        )

    def render_python(self, type: HasType, value: Any) -> str:
        return "<vector>"  # not sure how to render this


@dataclass
class NodeMapper(TypeMapper):
    def is_instance_value(self, type: HasType, value: Any) -> bool:
        return isinstance(value, Node)

    def unpack_value(
        self, type: HasType, scope: ScopeNode, session: Optional[Session], value: Any
    ) -> Any:
        # TODO @Architecture: unify values with errors or silent ignore on error (e.g. missing reference)
        return scope.lookup(try_to_uuid(value)) or value

    def pack_value(self, type: HasType, value: Any) -> Any:
        return str(value.ck) if isinstance(value, Node) else value

    def render_python(self, type: HasType, value: Any) -> str:
        return value.py_ident


@dataclass
class StructMapper(TypeMapper):
    def is_instance_value(self, type: HasType, value: Any) -> bool:
        return isinstance(value, Mapping) or is_dataclass(value)

    def unpack_value(
        self, type: HasType, scope: ScopeNode, session: Optional[Session], value: Any
    ) -> Any:
        return TypedDict(value, type) if not isinstance(value, TypedDict) else value

    def pack_value(self, type: HasType, value: Any) -> Any:
        return {TYPE_DISCRIMINATOR_KEY: type.key, **value}

    def render_python(self, type: HasType, value: Any) -> str:
        # render parts as python
        parts_strs = []
        for field_ in type.resolved_fields:
            if field_.flags & TypeFlag.IS_OUTPUT:
                continue
            field_value = value[field_.key]
            if field_value is None:
                continue
            field_str = render_value(field_, field_value)
            parts_strs.append(f"{field_.py_ident}={field_str}")
        return f"{type.py_ident}({', '.join(parts_strs)})"


@dataclass
class JsonMapper(TypeMapper):
    def is_instance_value(self, type: HasType, value: Any) -> bool:
        return True  # not sure how to check this

    def render_python(self, type: HasType, value: Any) -> str:
        return json.dumps(value, indent=2)


@dataclass
class FunctionMapper(TypeMapper):
    def is_instance_value(self, type: HasType, value: Any) -> bool:
        return value is None or isinstance(value, dict)


_TYPE_MAP: dict[Any, HasFields] = {}


def unpack_value_flat(
    value: Any,
    type: HasType,
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
        if type.flags & TypeFlag.IS_ARRAYABLE:  # keep as is
            if not isinstance(value, list):
                return mapping.unpack_value(type, scope, session, value)
            else:
                return [mapping.unpack_value(type, scope, session, v) for v in value]
        elif type.flags & TypeFlag.IS_ARRAY and not ignore_array:  # must be list
            return [mapping.unpack_value(type, scope, session, v) for v in value]
        else:  # must be element
            return mapping.unpack_value(type, scope, session, value)
    except (KeyError, ValueError, TypeError):
        logger.warning(
            "unpack.failed", exc_info=True, value=value, type=type, scope=scope, session=session
        )
        return value  # type checking is done elsewhere


def pack_value_flat(value: Any, type: HasType, *args, **kwargs) -> Any:
    """Maps back to the raw value from the Python representation."""
    # we don't auto-coerce here since that's only needed for external data
    if value is None:
        return None
    mapping = get_type_mapper_by_type(type)
    if not mapping.is_instance_value(type, value):
        return None  # type-checking is done elsewhere
    return mapping.pack_value(type, value)


def render_value_flat(value: Any, type: HasType, *args, **kwargs) -> str:
    """Renders the given value as a string."""
    mapping = get_type_mapper_by_type(type)
    return mapping.render_python(type, value)


def unpack_value(
    value: Any,
    type: HasFields,
    # TODO @Cleanup: always pass unpacking scope explicitly?
    scope: Optional[ScopeNode] = None,
    session: Optional[Session] = None,
    ignore_array: bool = False,
    ignore_outer: bool = False,
    ignore_empty: bool = True,
    is_output: bool = None,
    map_k: Callable[[Field], tuple[str, str]] = None,
):
    """Unpacks/deserializes the given value into a Python/Bench representation."""
    scope = scope or type.scope
    if scope is None:
        raise ValueError(f"cannot unpack without scope: {type!r}")
    return map_value(
        value=value,
        type=type,
        map_k=map_k or (lambda f: (f._typed_key, f.py_ident)),
        map_v=partial(unpack_value_flat, scope=scope, session=session),
        ignore_array=ignore_array,
        ignore_outer=ignore_outer,
        ignore_empty=ignore_empty,
        is_output=is_output,
    )


def pack_value(
    value: Any,
    type: HasFields,
    ignore_array: bool = False,
    ignore_outer: bool = False,
    ignore_empty: bool = True,
    none_if_invalid: bool = False,
    is_output: bool = None,
    map_k: Callable[[Field], tuple[str, str]] = None,
    filter_v: Callable[[Any], bool] = None,
):
    """Packs/serializes the given value into a JSON-able representation."""
    if filter_v:

        def _filtered_pack_value(value: Any, type: HasType, *args, **kwargs) -> Any:
            if not filter_v(value):
                return None
            return pack_value_flat(value, type, *args, **kwargs)

        map_v = _filtered_pack_value
    else:
        map_v = pack_value_flat
    return map_value(
        value=value,
        type=type,
        map_k=map_k or (lambda f: (f.py_ident, f._typed_key)),
        map_v=map_v,
        ignore_array=ignore_array,
        ignore_outer=ignore_outer,
        ignore_empty=ignore_empty,
        none_if_invalid=none_if_invalid,
        is_output=is_output,
    )


def key_value(value: Any, type: "Statement", is_output: bool = None) -> Any:
    def map_v(value: Any, type: HasType, *args, **kwargs) -> Any:
        if type._effective_tag == TypeTag.ENUM:
            return value.key
        else:
            return value

    return map_value(
        value, type, map_k=lambda f: (f.py_ident, f._typed_key), map_v=map_v, is_output=is_output
    )


def unkey_value(value: Any, type: "Statement", is_output: bool = None) -> Any:
    def map_v(value: Any, type: HasType, *args, **kwargs) -> Any:
        if type._effective_tag == TypeTag.ENUM:
            return type.resolved_fields.get(value).py_ident
        else:
            return value

    return map_value(
        value, type, map_k=lambda f: (f._typed_key, f.py_ident), map_v=map_v, is_output=is_output
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
    type: HasFields,
    get_k: Callable[[Field], str] = None,
    filter_v: Callable[[Any, Field], bool] = None,
    ignore_array: bool = False,
    ignore_empty: bool = True,
    is_output: bool = None,
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
    elif type.flags & TypeFlag.IS_ARRAYABLE and not ignore_array:
        if _is_arrayable_not_an_array(type, value):
            return render_value(
                value,
                type,
                get_k=get_k,
                filter_v=filter_v,
                ignore_array=True,
                ignore_empty=ignore_empty,
            )
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
    elif type._effective_tag != TypeTag.STRUCT:
        assert filter_v is None or filter_v(value, type), f"unexpected filtered value {value}"
        return render_value_flat(value, type, filter_k=filter_v)

    # map struct-like types into a dict
    assert type._status >= NS.INTERP, f"unexpected unresolved type {type}"
    elements = {}
    for subtype in type.resolved_fields:
        assert (
            not subtype.flags & TypeFlag.IS_UNION_WITH
        ), f"unexpected union with {type}->{subtype}"
        if is_output is not None and bool(subtype.flags & TypeFlag.IS_OUTPUT) != is_output:
            continue
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
register_mapper(StringTypeMapper(str, TypeTag.STRING), tags=[TypeTag.STRING])
register_mapper(StringTypeMapper(Key, TypeTag.STRING, hint=TypeHint.KEY), hints=[TypeHint.KEY])
register_mapper(
    StaticPyTypeMapper(float, TypeTag.NUMBER, alt_py_types=[int]), tags=[TypeTag.NUMBER]
)
register_mapper(StaticPyTypeMapper(bool, TypeTag.BOOLEAN), tags=[TypeTag.BOOLEAN])
register_mapper(VectorTypeMapper(), tags=[TypeTag.VECTOR])
register_mapper(EnumMapper(), tags=[TypeTag.ENUM])
register_mapper(NodeMapper(), tags=[TypeTag.NODE])
register_mapper(StructMapper(), tags=[TypeTag.STRUCT])
register_mapper(FunctionMapper(), tags=[TypeTag.FUNCTION])
register_mapper(JsonMapper(), tags=[TypeTag.JSON])
# type hints
register_mapper(RichTextMapper(), hints=[TypeHint.RICH_TEXT])
register_mapper(
    StringifyTypeMapping(UUID, TypeTag.STRING, hint=TypeHint.UUID), hints=[TypeHint.UUID]
)
register_mapper(IsoDtTypeMapping(date, TypeTag.STRING), hints=[TypeHint.DATE])
register_mapper(IsoDtTypeMapping(datetime, TypeTag.STRING), hints=[TypeHint.DATETIME])
register_mapper(IsoDtTypeMapping(time, TypeTag.STRING), hints=[TypeHint.TIME])
register_mapper(
    StaticPyTypeMapper(int, TypeTag.NUMBER, hint=TypeHint.INTEGER), hints=[TypeHint.INTEGER]
)
