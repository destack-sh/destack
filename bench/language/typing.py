import enum
import inspect
from dataclasses import dataclass, field, fields, is_dataclass
from datetime import date, datetime, time
from functools import partial
from typing import (
    TYPE_CHECKING,
    Any,
    Callable,
    Collection,
    ForwardRef,
    Iterable,
    Mapping,
    NamedTuple,
    Optional,
    Union,
    get_args,
    get_origin,
    get_type_hints,
    is_typeddict,
)
from uuid import UUID

import structlog
from more_itertools import first

from bench.language.const import RemoteObjectStatus, TypeFlag, TypeHint, TypeTag
from bench.language.field import (
    PRIMITIVE_TYPES,
    TYPE_TAG_BY_TYPE_HINT,
    Field,
    HasFields,
    HasType,
    Json,
    Key,
    RichText,
    TypedDict,
    TypeError,
    Vector,
)
from bench.language.module import NS, Node, ScopeNode
from bench.language.remote import RemoteObject, Secret
from bench.language.text import Text, parse_text_multi, render_text_html
from bench.utils.utils import IdentifierType, to_pyidentifier

if TYPE_CHECKING:
    from bench.language import Statement

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
        or isinstance(value, str)
        or (
            type._effective_tag == TypeTag.VECTOR
            and isinstance(value, list)
            and (not value or isinstance(value[0], float))
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
    ignore_outer_map: bool = False,
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
    assert type._status >= NS.Interpreted, f"unexpected unresolved type {type}"
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
    if not ignore_outer_map:
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
            # type error, ignore here
            return
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
        # type error, ignore here
        return

    assert type._status >= NS.Interpreted, f"unexpected unresolved type {type}"
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


TYPENAME_SENTINEL = "__typename"  # :TypeSentinel
OMITTED_SENTINEL = "__omitted"  # :OmittedSentinel
REMOTE_OBJECT_TYPENAME = "RemoteObject"
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

    def is_instance_type(self, py_type: type) -> bool:
        """Whether this mapper can represent the given Python instance type."""
        raise NotImplementedError

    def from_instance_type(self, py_type: type, type_map: dict[type, Any]) -> StatementOrField:
        """Converts a Python instance type into a Bench Type."""
        raise NotImplementedError

    def is_instance_value(self, type: HasType, value: Any) -> bool:
        """
        Whether this mapper can represent the given Python instance value.
        For nested types (like structs) this only checks the top-level value (no walking).
        """
        raise NotImplementedError

    def unpack_value(self, type: HasType, scope: ScopeNode, value: Any) -> Any:
        """Converts and coerces a raw flat value of the type into an instance value."""
        return value

    def pack_value(self, type: HasType, value: Any) -> Any:
        """Converts a value of the given instance type back into a flat value."""
        return value


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


def get_type_mapper_by_instance_type(py_type: type) -> tuple[TypeMapper, type, TypeFlag]:
    """
    Gets the most appropriate mapping for the given Python type.
    (flat because we "ignore" list and optional types (inside the mapper)).
    """
    py_type, flags = _strip_py_type(py_type)
    # get mapping
    for mapping in type_mappers.values():
        if mapping.is_instance_type(py_type):
            return mapping, py_type, flags
    raise LookupError(f"no mapping found for {py_type} ({flags}, type={type(py_type)})")


def _strip_py_type(py_type: type) -> tuple[type, TypeFlag]:
    flags = TypeFlag.ZERO
    # strip optional
    if get_origin(py_type) is Union:
        args = get_args(py_type)
        if len(args) == 2 and args[1] == type(None):  # noqa: E721
            py_type = args[0]
            flags |= TypeFlag.IS_OPTIONAL
        # convert x | list[x] as isarrayable
        elif len(args) == 2 and get_origin(args[1]) is list:
            if args[0] != get_args(args[1])[0]:
                raise ValueError(f"cannot map generic union types: {py_type}")
            py_type = args[0]
            flags |= TypeFlag.IS_ARRAYABLE
        else:
            raise ValueError(f"cannot map generic union types: {py_type}")
    # strip list
    if get_origin(py_type) is list:
        py_type = get_args(py_type)[0]
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

    def is_instance_type(self, py_type: type) -> bool:
        return py_type == self.py_type or (
            self.alt_py_types is not None and py_type in self.alt_py_types
        )

    def from_instance_type(self, py_type: type, type_map: dict[type, Any]) -> StatementOrField:
        return Field(name=None, tag=self.tag, hint=self.hint)

    def is_instance_value(self, type: HasType, value: Any) -> bool:
        return isinstance(value, self._all_py_types)

    def unpack_value(self, type: HasType, scope: ScopeNode, value: Any) -> Any:
        return self.py_type(value)


@dataclass
class StringTypeMapper(StaticPyTypeMapper):
    py_type: type = str
    tag: TypeTag = TypeTag.STRING

    def pack_value(self, type: HasType, value: Any) -> str:
        # sanitize null character
        return str(value).replace("\x00", "")


@dataclass
class VectorTypeMapper(StaticPyTypeMapper):
    py_type: type = Vector
    tag: TypeTag = TypeTag.VECTOR

    def is_instance_value(self, type: HasType, value: Any) -> bool:
        # not quite right but good enough for now
        return isinstance(value, Collection) and len(value) > 0 and isinstance(value[0], float)


@dataclass
class StringifyTypeMapping(StaticPyTypeMapper):
    def is_instance_type(self, py_type: type) -> bool:
        return self.py_type == py_type

    def unpack_value(self, type: HasType, scope: ScopeNode, value: Any) -> Any:
        return self.py_type(value)

    def pack_value(self, type: HasType, value: Any) -> str:
        return str(value)


class IsoDtTypeMapping(StaticPyTypeMapper):
    HINT_BY_PY_TYPE = {
        date: TypeHint.DATE,
        datetime: TypeHint.DATETIME,
        time: TypeHint.TIME,
    }

    def is_instance_type(self, py_type: type) -> bool:
        return inspect.isclass(py_type) and any(
            issubclass(py_type, t) for t in self.HINT_BY_PY_TYPE
        )

    def from_instance_type(self, py_type: type, type_map: dict[type, Any]) -> StatementOrField:
        return Field(tag=TypeTag.STRING, hint=self.HINT_BY_PY_TYPE[py_type])

    def unpack_value(self, type: HasType, scope: ScopeNode, value: Any) -> Any:
        if isinstance(value, self.py_type):
            return value
        return self.py_type.fromisoformat(value)

    def pack_value(self, type: HasType, value: Any) -> str:
        return value.isoformat()


class EnumMapper(TypeMapper):
    def is_instance_type(self, py_type: type) -> bool:
        return inspect.isclass(py_type) and issubclass(py_type, enum.StrEnum)

    def from_instance_type(self, py_type: type, type_map: dict[type, Any]) -> HasType:
        from bench.language.statement import Statement

        assert issubclass(py_type, enum.StrEnum)
        if py_type in type_map:
            return type_map[py_type]
        type = Statement.choice(name=py_type.__name__)
        type_map[py_type] = type
        for py_member in py_type.__members__.values():
            member = Field(name=py_member.name, key=py_member.name, tag=TypeTag.LITERAL)
            type.fields.append(member)
        return type

    def is_instance_value(self, type: HasType, value: Any) -> bool:
        if isinstance(value, str):
            # allow string values for built-in enums
            # (that also function as regular enums in code)
            return value in type.resolved_fields
        return isinstance(value, Field) and value.key in type.resolved_fields

    def unpack_value(self, type: HasFields, scope: ScopeNode, value: Any) -> Any:
        field_ = type.resolved_fields.get(value)
        return field_.name if field_ else value

    def pack_value(self, type: HasFields, value: Any) -> Any:
        field_ = type.resolved_fields.get(value) if not isinstance(value, Field) else value
        return field_.key if field_ else value


class RichTextMapper(TypeMapper):
    def is_instance_type(self, py_type: type) -> bool:
        return py_type is RichText

    def from_instance_type(self, py_type: type, type_map: dict[type, Any]) -> StatementOrField:
        return Field(name=None, tag=TypeTag.STRING, hint=TypeHint.RICH_TEXT)

    def is_instance_value(self, type: HasType, value: Any) -> bool:
        return isinstance(value, (str, Text))

    def unpack_value(self, type: HasType, scope: ScopeNode, value: Any) -> Any:
        spans = parse_text_multi(value)
        text = Text(spans=spans, _raw_text=value)
        text._resolve(scope)
        return text

    def pack_value(self, type: HasType, value: Any) -> Any:
        if not isinstance(value, Text):
            return value
        return render_text_html(value.spans)


class NodeMapper(TypeMapper):
    def is_instance_type(self, py_type: type) -> bool:
        return issubclass(py_type, Node)

    def from_instance_type(self, py_type: type, type_map: dict[type, Any]) -> StatementOrField:
        hint = {Statement: TypeHint.STATEMENT, Field: TypeHint.FIELD}.get(py_type)
        if hint is None:
            raise ValueError(f"cannot map {py_type}")
        return Field(name=None, tag=TypeTag.NODE, hint=hint)

    def unpack_value(self, type: HasType, scope: ScopeNode, value: Any) -> Any:
        raise NotImplementedError(":NodesAsValues not yet supported")

    def pack_value(self, type: HasType, value: Any) -> Any:
        raise NotImplementedError(":NodesAsValues not yet supported")


class RemoteObjectMapper(TypeMapper):
    def is_instance_type(self, py_type: type) -> bool:
        return py_type is RemoteObject

    def from_instance_type(self, py_type: type, type_map: dict[type, Any]) -> HasType:
        return Field(name=None, tag=TypeTag.FILE)

    def is_instance_value(self, type: HasType, value: Any) -> bool:
        return isinstance(value, RemoteObject)

    def unpack_value(self, type: HasType, scope: ScopeNode, value: Any) -> Any:
        return RemoteObject(
            id=UUID(value["id"]),
            name=value["name"],
            content_type=value["content_type"],
            content_length=value["content_length"],
            sha512=value["sha512"],
            status=RemoteObjectStatus[value["status"]],
        )

    def pack_value(self, type: HasType, value: Any) -> Any:
        return {
            TYPENAME_SENTINEL: REMOTE_OBJECT_TYPENAME,
            "id": str(value.id),
            "name": value.name,
            "content_type": value.content_type,
            "content_length": value.content_length,
            "sha512": value.sha512,
            "status": value.status.name,
        }


class SecretTypeMapper(TypeMapper):
    def is_instance_type(self, py_type: type) -> bool:
        return py_type is Secret

    def from_instance_type(self, py_type: type, type_map: dict[type, Any]) -> HasType:
        return Field(name=None, tag=TypeTag.STRING, hint=TypeHint.SECRET, flags=TypeFlag.IS_SECRET)

    def is_instance_value(self, type: HasType, value: Any) -> bool:
        return isinstance(value, Secret)

    def unpack_value(self, type: HasType, scope: ScopeNode, value: Any) -> Any:
        return Secret(id=UUID(value["id"]), sha512=value["sha512"])

    def pack_value(self, type: HasType, value: Any) -> Any:
        return {
            TYPENAME_SENTINEL: SECRET_TYPENAME,
            "id": str(value.id),
            "sha512": value.sha512,
        }


class StructTypeMapper(TypeMapper):
    def is_instance_type(self, py_type: type) -> bool:
        return is_dataclass(py_type) or is_typeddict(py_type)

    def from_instance_type(self, py_type: type, type_map: dict[str, Any]) -> StatementOrField:
        if py_type in type_map:
            return type_map[py_type]
        from bench.language.statement import Statement

        type = Statement.class_(name=py_type.__name__)
        type_map[py_type] = type
        if is_dataclass(py_type):
            for py_field in fields(py_type):
                field_ = field_from_instance_type(py_field.type, py_field.name, type_map)
                type.fields.append(field_)
        elif is_typeddict(py_type):
            for py_field_name, py_field in get_type_hints(py_type).items():
                field_ = field_from_instance_type(py_field, py_field_name, type_map)
                type.fields.append(field_)
        else:
            raise ValueError(f"unsupported struct type: {py_type}")
        return type

    def is_instance_value(self, type: HasType, value: Any) -> bool:
        return isinstance(value, Mapping) or is_dataclass(value)

    def unpack_value(self, type: HasType, scope: ScopeNode, value: Any) -> Any:
        return TypedDict(value, type) if not isinstance(value, TypedDict) else value

    def pack_value(self, type: HasType, value: Any) -> Any:
        return {TYPENAME_SENTINEL: type.key, **value}


class JsonTypeMapper(TypeMapper):
    def is_instance_type(self, py_type: type) -> bool:
        return py_type is Json or py_type is dict or get_origin(py_type) is dict

    def is_instance_value(self, type: HasType, value: Any) -> bool:
        return True  # not sure how to check this

    def from_instance_type(self, py_type: type, type_map: dict[type, Any]) -> StatementOrField:
        return Field(name=None, tag=TypeTag.JSON)


class FunctionTypeMapper(TypeMapper):
    def is_instance_type(self, py_type: type) -> bool:
        return inspect.isfunction(py_type)

    def is_instance_value(self, type: HasType, value: Any) -> bool:
        return value is None or isinstance(value, dict)

    def from_instance_type(self, py_type: type, type_map: dict[type, Any]) -> StatementOrField:
        if py_type in type_map:
            return type_map[py_type]
        from bench.language.statement import Statement

        type = Statement.class_(name=py_type.__name__)
        type_map[py_type] = type
        signature = inspect.signature(py_type)
        for py_param in signature.parameters.values():
            if py_param.name == "self" and (
                py_param.annotation is py_param.empty
                or py_param.annotation.__name__ is py_type.__name__
            ):
                continue
            param = field_from_instance_type(py_param.annotation, py_param.name, type_map)
            type.fields.append(param)

        # output must be a struct, inline it with output flag
        if signature.return_annotation is inspect.Signature.empty:
            raise ValueError(f"missing return annotation for {py_type}")
        output = type_from_instance_type(signature.return_annotation, None, type_map)
        if output.tag != TypeTag.STRUCT:
            raise ValueError(f"function output must be a struct: {py_type}")
        for output_field in output.fields:
            output_field = output_field._copy_self()
            output_field.flags |= TypeFlag.IS_OUTPUT
            output_field.order_key = None  # reset order
            type.fields.append(output_field)

        return type


_TYPE_MAP: dict[Any, HasFields] = {}


def type_from_instance_type(
    py_type: type, name: Optional[str], type_map: dict[Any, StatementOrField] = None
) -> StatementOrField:
    """
    Maps a python type to a Type (recursively).
    Nested types are read/written in the given type_map.
    Types are keyed by name since we have no way to associate keys over time.
    """
    type_map = type_map or _TYPE_MAP
    map, stripped, flags = get_type_mapper_by_instance_type(py_type)
    type = map.from_instance_type(stripped, type_map)
    if name:
        type.name = name
    if flags:
        type.flags |= flags
    return type


def field_from_instance_type(
    py_type: type | str, name: str, type_map: dict[Any, StatementOrField]
) -> Field:
    name_nice = name.replace("_", " ")
    if to_pyidentifier(name_nice, IdentifierType.FIELD) != name:
        raise ValueError(f"inconsistent field name: {name} != {name_nice}")

    stripped, flags = _strip_py_type(py_type)
    if isinstance(stripped, str):
        # lookup by name in type_map
        type = first((t for k, t in type_map.items() if k.__name__ == stripped), None)
        if type is None:
            raise ValueError(f"unknown type name: {stripped}")
    elif isinstance(stripped, ForwardRef):
        # lookup by name in type_map
        type = first(
            (t for k, t in type_map.items() if k.__name__ == stripped.__forward_arg__), None
        )
        if type is None:
            raise ValueError(f"unknown type name: {stripped}")
    elif stripped in type_map:
        type = type_map[stripped]
    else:
        type = type_from_instance_type(py_type, name, type_map)
    # key is set to None so we error if they're not set later
    if type.tag in (TypeTag.STRUCT, TypeTag.ENUM, TypeTag.TYPE_REFERENCE):
        return Field(
            name=name_nice, key=None, tag=TypeTag.TYPE_REFERENCE, reference=type, flags=flags
        )
    else:
        return Field(
            name=name_nice,
            key=None,
            tag=type.tag,
            hint=type.hint,
            flags=flags,
        )


def unpack_value_flat(
    value: Any, type: HasType, scope: Optional[ScopeNode] = None, ignore_array: bool = False
) -> Any:
    """Maps to the proper Python representation of the given value."""
    if value is None:  # skip null values
        return None  # type checking is done elsewhere
    scope = scope or type.scope
    if scope is None:
        raise ValueError(f"cannot unpack without scope: {type!r}")
    # auto coerce lists to element and vice versa (like in frontend) :ArrayCoercion
    mapping = get_type_mapper_by_type(type)
    try:
        if type.flags & TypeFlag.IS_ARRAYABLE:  # keep as is
            if not isinstance(value, list):
                return mapping.unpack_value(type, scope, value)
            else:
                return [mapping.unpack_value(type, scope, v) for v in value]
        elif type.flags & TypeFlag.IS_ARRAY and not ignore_array:  # promote to array
            if not isinstance(value, list):
                value = [value]
            else:
                return [mapping.unpack_value(type, scope, v) for v in value]
        else:  # trim to element
            if isinstance(value, list):
                value = value[0]
            else:
                return mapping.unpack_value(type, scope, value)
    except (KeyError, ValueError, TypeError):
        logger.warning("unpack_failed", exc_info=True, value=value, type=type)
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


def unpack_value(
    value: Any,
    type: HasFields,
    # TODO @Cleanup: always pass unpacking scope explicitly?
    scope: Optional[ScopeNode] = None,
    ignore_array: bool = False,
    ignore_outer_map: bool = False,
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
        map_v=partial(unpack_value_flat, scope=scope),
        ignore_array=ignore_array,
        ignore_outer_map=ignore_outer_map,
        ignore_empty=ignore_empty,
        is_output=is_output,
    )


def pack_value(
    value: Any,
    type: HasFields,
    ignore_array: bool = False,
    ignore_outer_map: bool = False,
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
        ignore_outer_map=ignore_outer_map,
        ignore_empty=ignore_empty,
        none_if_invalid=none_if_invalid,
        is_output=is_output,
    )


# type tags
register_mapper(StringTypeMapper(str, TypeTag.STRING), tags=[TypeTag.STRING])
register_mapper(StringTypeMapper(Key, TypeTag.STRING, hint=TypeHint.KEY), hints=[TypeHint.KEY])
register_mapper(
    StaticPyTypeMapper(float, TypeTag.NUMBER, alt_py_types=[int]), tags=[TypeTag.NUMBER]
)
register_mapper(StaticPyTypeMapper(bool, TypeTag.BOOLEAN), tags=[TypeTag.BOOLEAN])
register_mapper(VectorTypeMapper(), tags=[TypeTag.VECTOR])
register_mapper(RemoteObjectMapper(), tags=[TypeTag.FILE])
register_mapper(EnumMapper(), tags=[TypeTag.ENUM])
register_mapper(StructTypeMapper(), tags=[TypeTag.STRUCT])
register_mapper(FunctionTypeMapper(), tags=[TypeTag.FUNCTION])
register_mapper(JsonTypeMapper(), tags=[TypeTag.JSON])
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
# other
register_mapper(SecretTypeMapper(), tags=[TypeTag.STRING, TypeTag.NUMBER], flags=TypeFlag.IS_SECRET)
