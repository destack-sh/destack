import enum
import typing
import uuid
from dataclasses import dataclass
from typing import Any, Collection, Optional, Self, Union
from uuid import UUID

import structlog

from bench.language.const import (
    MNT,
    IssueType,
    StatementReference,
    TypeFlag,
    TypeHint,
    TypeStorageFormat,
    TypeTag,
    new_dynamic_node_key,
)
from bench.language.module import (
    NS,
    Node,
    NodeList,
    NodeVisitor,
    NRel,
    ScopeNode,
    get_node_id,
    nchildren,
    ninternal,
    node,
    node_component,
    nparent,
    nproperty,
    nruntime,
)
from bench.language.query import FieldQueryOps
from bench.language.reference import HasReference
from bench.language.text import HasText
from bench.language.validation import (
    ValidationHandler,
    enum_validator,
    flag_validator,
    validate_name,
)
from bench.language.value import HasValue
from bench.utils.func import dict_minus
from bench.utils.proxy import ProxyDict, ProxyList, unproxy_value
from bench.utils.utils import IdentifierType, to_pyidentifier

if typing.TYPE_CHECKING:
    from bench.language import Statement, Type

logger = structlog.get_logger(__name__)


class TypeError(TypeError):
    def __init__(
        self,
        value: Any,
        expected: "IsTyped",
        message: str = None,
        suberrors: list["TypeError"] = None,
    ):
        if isinstance(value, (ProxyDict, ProxyList)):
            value = unproxy_value(value)
        value_str = repr(value)
        max_value_str_len = 300
        if len(value_str) > max_value_str_len:
            value_str = value_str[: max_value_str_len - 100] + "..." + value_str[-100:]

        if isinstance(expected, Field) and not expected.resolved_fields:
            expected_str = (
                f"field {expected.py_ident} ({expected._type_str}, from {expected.parent!r})"
            )
        else:
            expected_fields_str = ", ".join(
                f"{f.py_ident} ({f._type_str})" for f in expected.resolved_fields
            )
            expected_str = f"fields {expected_fields_str or '<empty>'} from {expected!r}"

        super().__init__(
            f"{message or 'type mismatch'}: expected {expected_str}, got {value_str} ({type(value).__name__})"
        )
        self.value = value
        self.expected = expected
        self.message = message
        self.suberrors = suberrors or []


PRIMITIVE_TYPES = [
    TypeTag.ANY,
    TypeTag.NULL,
    TypeTag.BOOLEAN,
    TypeTag.NUMBER,
    TypeTag.STRING,
    TypeTag.FILE,
    TypeTag.VECTOR,
]
DEFAULT_EMBEDDING_DIMENSION = 1536  # currently only support :FixedEmbeddingDimension
Vector = typing.NewType("Vector", list[float])
Json = typing.NewType("Json", dict)
Key = typing.NewType("Key", str)


@dataclass
class XYPoint:
    """The value of an OpenSearch/GeoJSON-compatible geo_point field."""

    x: float
    y: float


class XYShapeType(enum.StrEnum):
    """The type in an OpenSearch-compatible geo_shape field."""

    POINT = "point"
    LINE_STRING = "line_string"
    POLYGON = "polygon"
    MULTI_POINT = "multi_point"
    MULTI_LINE_STRING = "multi_line_string"
    MULTI_POLYGON = "multi_polygon"
    GEOMETRY_COLLECTION = "geometry_collection"
    ENVELOPE = "envelope"


@dataclass
class XYShape:
    """The value of an OpenSearch-compatible geo_shape field."""

    type: XYShapeType
    coordinates: typing.Union[list[float], list[list[float]]]


TYPE_TAG_BY_TYPE_HINT = {
    # string
    TypeHint.NAME: TypeTag.STRING,
    TypeHint.UUID: TypeTag.STRING,
    TypeHint.DATE: TypeTag.STRING,
    TypeHint.DATETIME: TypeTag.STRING,
    TypeHint.TIME: TypeTag.STRING,
    TypeHint.DURATION: TypeTag.STRING,
    TypeHint.EMAIL: TypeTag.STRING,
    TypeHint.URL: TypeTag.STRING,
    TypeHint.MARKDOWN: TypeTag.STRING,
    TypeHint.RICH_TEXT: TypeTag.STRING,
    TypeHint.HTML: TypeTag.STRING,
    TypeHint.CODE: TypeTag.STRING,
    TypeHint.KEY: TypeTag.STRING,
    TypeHint.PHONE: TypeTag.STRING,
    TypeHint.SECRET: TypeTag.STRING,
    # number
    TypeHint.INTEGER: TypeTag.NUMBER,
    TypeHint.FLOAT: TypeTag.NUMBER,
    TypeHint.SLIDER: TypeTag.NUMBER,
    TypeHint.RATING: TypeTag.NUMBER,
    # boolean
    TypeHint.TOGGLE: TypeTag.BOOLEAN,
    TypeHint.CHECKBOX: TypeTag.BOOLEAN,
    TypeHint.THUMBS: TypeTag.BOOLEAN,
    # file
    TypeHint.IMAGE: TypeTag.FILE,
    TypeHint.VIDEO: TypeTag.FILE,
    TypeHint.AUDIO: TypeTag.FILE,
}

STORAGE_FORMAT_BY_TYPE_TAG = {
    TypeTag.STRING: TypeStorageFormat.STRING,
    TypeTag.JSON: TypeStorageFormat.OBJECT,
    TypeTag.NUMBER: TypeStorageFormat.DOUBLE,
    TypeTag.BOOLEAN: TypeStorageFormat.BOOLEAN,
    TypeTag.VECTOR: TypeStorageFormat.VECTOR,
    TypeTag.FILE: TypeStorageFormat.OBJECT,
    TypeTag.STRUCT: TypeStorageFormat.OBJECT,
    TypeTag.ENUM: TypeStorageFormat.KEYWORD,
    TypeTag.LITERAL: TypeStorageFormat.KEYWORD,
}
STORAGE_FORMAT_BY_TYPE_HINT = {
    # for special types that are not the same as their type tag
    TypeHint.UUID: TypeStorageFormat.KEYWORD,
    TypeHint.DATE: TypeStorageFormat.DATE,
    TypeHint.DATETIME: TypeStorageFormat.DATE,
    TypeHint.TIME: TypeStorageFormat.LONG,
    TypeHint.DURATION: TypeStorageFormat.DOUBLE,
    TypeHint.KEY: TypeStorageFormat.KEYWORD,
    TypeHint.INTEGER: TypeStorageFormat.LONG,
    TypeHint.FLOAT: TypeStorageFormat.DOUBLE,
}


def get_storage_format(tag: TypeTag, hint: TypeHint, flags: TypeFlag) -> TypeStorageFormat:
    # :TypeStorageFormat
    if flags & TypeFlag.IsSecret:
        return TypeStorageFormat.OBJECT  # stored as secret object
    if hint in STORAGE_FORMAT_BY_TYPE_HINT:
        return STORAGE_FORMAT_BY_TYPE_HINT[hint]
    return STORAGE_FORMAT_BY_TYPE_TAG[tag]


@node_component
class IsTyped(Node):
    """Abstract base for Field nas HasFields/Statement types"""

    tag: TypeTag = nproperty(is_required=True, validate=enum_validator(TypeTag))
    hint: TypeHint | None = nproperty(default=None, validate=enum_validator(TypeHint))
    flags: TypeFlag = nproperty(default=TypeFlag.Zero, validate=flag_validator(TypeFlag))
    key: str = nproperty(default=None)

    def _validate_inner(self, properties: Collection[str], on_issue: "ValidationHandler") -> None:
        if self.hint is not None:
            tag = TYPE_TAG_BY_TYPE_HINT[self.hint]
            if self.tag != tag:
                on_issue(self, f"expected {tag} for {self.hint} ({self.tag})", ["tag", "hint"])

    @property
    def reference(self) -> Union["Statement", StatementReference, None]:
        return None

    @property
    def resolved_fields(self) -> Collection["ResolvedField"]:
        raise NotImplementedError

    @property
    def py_ident(self) -> Optional[str]:
        if self.name is None:
            return None
        elif self.tag == TypeTag.LITERAL:
            return to_pyidentifier(self.name, IdentifierType.CONSTANT)
        else:
            return to_pyidentifier(self.name, IdentifierType.FIELD)

    @property
    def _storage_format(self) -> TypeStorageFormat:
        if self.tag == TypeTag.TYPE_REFERENCE and isinstance(self.reference, Node):
            return self.reference._storage_format
        return get_storage_format(self.tag, self.hint, self.flags)

    @property
    def _effective_type(self) -> Union["IsTyped", "Statement"]:
        if isinstance(self.reference, Node):
            return self.reference
        else:
            return self

    @property
    def _type_str(self) -> str:
        flag_str = ", ".join(flag.short_name.lower() for flag in TypeFlag if self.flags & flag)
        flags_str = f" ({flag_str})" if flag_str else ""
        if self.hint:
            return f"{self.hint}{flags_str}"
        else:
            return f"{self.tag}{flags_str}"

    @property
    def _effective_tag(self) -> TypeTag:
        return self._effective_type.tag

    @property
    def _effective_hint(self) -> Optional[TypeHint]:
        return self._effective_type.hint

    def equals_type(self, other: "IsTyped") -> bool:
        return (
            self._effective_tag == other._effective_tag
            and self._effective_hint == other._effective_hint
            and self.flags == other.flags
            and self.reference == other.reference
        )

    def get_field(self, some_id: str, is_output: bool = None) -> Optional["Field"]:
        # TODO @Cleanup: get rid if get_field/has_field in favor of fields.get
        #  (but need is_output filtering for that to work, so maybe computed inputs/output NodeList?)
        for field_ in self.resolved_fields:
            if is_output is not None and bool(field_.flags & TypeFlag.IsOutput) != is_output:
                continue
            if field_.py_ident == some_id or field_.name == some_id or field_.key == some_id:
                return field_
        return None

    def has_field(self, some_id: str, is_output: bool = None) -> bool:
        return self.get_field(some_id, is_output=is_output) is not None


@node(mnt=MNT.Field)
class Field(HasText, HasValue, HasReference, IsTyped, FieldQueryOps):
    parent: Union["Statement", None] = nparent(MNT.Statement)
    name: str | None = nproperty(default=None, validate=validate_name)
    order_key: str | None = ninternal(default=None)

    @staticmethod
    def new(
        name: str = None,
        type: Union[TypeTag, TypeHint, "Statement", type] = None,
        text: str = None,
        *args,
        for_parent: "Statement" = None,
        flags: TypeFlag = TypeFlag.Zero,
        **kwargs,
    ) -> "Field":
        # default to literal or string if no type is specified
        if type is None:
            if for_parent.tag == TypeTag.ENUM:
                type = TypeTag.LITERAL
                if name is None:
                    name = f"Option {len(for_parent.fields) + 1}"
            else:
                type = TypeTag.STRING

        # coerce type
        if isinstance(type, TypeTag):
            kwargs["tag"] = type
        elif isinstance(type, TypeHint):
            kwargs["hint"] = type
            kwargs["tag"] = TYPE_TAG_BY_TYPE_HINT[type]
        elif type == str:
            kwargs["tag"] = TypeTag.STRING
        elif type == int:
            kwargs["tag"] = TypeTag.NUMBER
            kwargs["hint"] = TypeHint.INTEGER
        elif type == float:
            kwargs["tag"] = TypeTag.NUMBER
        elif type == bool:
            kwargs["tag"] = TypeTag.BOOLEAN
        elif isinstance(type, Node) and type.mnt == MNT.Statement or isinstance(type, str):
            kwargs["tag"] = TypeTag.TYPE_REFERENCE
            kwargs["reference"] = type
        else:
            raise ValueError(f"unexpected type {type!r}")

        # default to optional if parent is not a function
        if not (for_parent and for_parent.tag == TypeTag.FUNCTION):
            flags |= TypeFlag.IsOptional

        return Field(name=name, text=text, flags=flags, *args, **kwargs)

    @staticmethod
    def literal(name: str, text: str = None, *args, **kwargs) -> "Field":
        return Field(name=name, text=text, tag=TypeTag.LITERAL, *args, **kwargs)

    @staticmethod
    def to_python(
        node: "Field", props: dict, for_parent: "Statement" = None
    ) -> tuple[str, dict, dict]:
        props = {**props}
        implicit_optional = (
            node.flags == TypeFlag.IsOptional and for_parent and for_parent.tag != TypeTag.FUNCTION
        )
        if node.flags == 0 or implicit_optional:
            del props["flags"]
        type = node.reference or node.hint or node.tag
        if for_parent and for_parent.tag == TypeTag.ENUM and node.tag == TypeTag.LITERAL:
            init_args = {"name": props["name"], "text": props.get("text")}
            init_name = "Field.literal"
        else:
            init_args = {"name": props["name"], "type": type, "text": props.get("text")}
            init_name = "Field.new"
        init_kwargs = dict_minus(props, "name", "text", "tag", "hint", "reference")
        return init_name, init_args, init_kwargs

    def __str__(self):
        name_str = f"{self.py_ident} '{self.name}' " if self.name else ""
        return f"{name_str}{self._type_str}"

    def __repr__(self):
        return f"<{self.__class__.__name__} {self}>"

    def __eq__(self, other):
        return FieldQueryOps.__eq__(self, other)  # override to avoid recursion

    @property
    def _type_of_value(self) -> "HasFields":
        from bench.language.libs import symbolx_lib

        return symbolx_lib.resolve(".reflect.FieldMetadata")

    def _init_inner(self):
        self.key = self.key or new_dynamic_node_key(self.ck)

    def _visit_inner(self, visitor: NodeVisitor) -> None:
        if isinstance(self.reference, Node):
            visitor.visit_reference(self.reference)

    @property
    def path(self) -> str:
        if self.parent is None:
            return f"<detached>.{self.py_ident}"
        else:
            return f"{self.parent.path}.{self.py_ident}"

    @property
    def dimensions(self) -> int:
        if self.tag != TypeTag.VECTOR:
            raise ValueError(f"{self} does not have dimensions")
        return (self.value or {}).get("dimensions", DEFAULT_EMBEDDING_DIMENSION)

    @property
    def _typed_key(self) -> str:
        if self._storage_format == TypeStorageFormat.VECTOR:
            return f"{self.key}-{self._storage_format.value}{self.dimensions}"
        else:
            return f"{self.key}-{self._storage_format.value}"

    @property
    def _source_key(self) -> str:
        return "value." + self._typed_key


@node(mnt=MNT.ResolvedField)
class ResolvedField(Field):
    parent: "Statement" = nparent(MNT.Statement)
    field: Field = ninternal()

    @property
    def field_ck(self) -> UUID:
        return self.field.ck

    @property
    def resolved_fields(self):
        return self.reference.resolved_fields if isinstance(self.reference, Node) else []

    @property
    def _is_foreign(self) -> bool:
        return self.parent != self.field.parent

    @staticmethod
    def from_field(for_parent: Node, field: Field) -> "ResolvedField":
        if isinstance(field, ResolvedField):
            field = field.field
        if field.tag == TypeTag.TYPE_REFERENCE and not isinstance(field.reference, Node):
            raise RuntimeError(f"unresolved reference {field.reference} in {field!r}")
        ck = uuid.uuid5(for_parent.ck, field.ck.hex)
        id = get_node_id(field.module.id, ck) if field.module else None
        resolved_field = ResolvedField(
            id=id,
            ck=ck,
            name=field.name,
            tag=field.tag,
            hint=field.hint,
            order_key=field.order_key,
            key=field.key,
            text=field.text,
            flags=field.flags,
            reference=field.reference,
            field=field,
            _status=NS.Source,
        )
        resolved_field._interp_self(for_parent)
        return resolved_field


@node_component
class HasFields(IsTyped):
    """A node with fields"""

    fields: NodeList["Field"] = nchildren(MNT.Field, NRel.Named | NRel.Scoped | NRel.Ordered)

    resolved_fields: NodeList["ResolvedField"] = nchildren(
        MNT.ResolvedField, NRel.Named | NRel.Keyed | NRel.Ordered
    )
    _resolved_fields: bool = nruntime(default=False)

    def _init_inner(self):
        if self.key is None:
            self.key = new_dynamic_node_key(self.ck)

    def _clear_inner(self) -> None:
        self.resolved_fields.clear()
        self._resolved_fields = False

    def _interp_inner(self, scope: ScopeNode) -> None:
        self._resolve_fields([])

    def _resolve_fields(self: "HasFields", path: list[IsTyped]) -> None:
        """
        Resolves (and inlines) field references and unions.
        """
        if self._resolved_fields:
            return  # already resolved

        if any(f.id == self.id for f in path):
            # circular panic
            path = "->".join(n.name for n in path + [self])
            self._on_issue(type=IssueType.CIRCULAR_UNION, subject=self, path=path)
            self._resolved_fields = True
            return

        path = path + [self]
        resolved_fields: list[ResolvedField] = []
        for field in self.fields:
            # try to resolve reference or skip this field
            if field.tag == TypeTag.TYPE_REFERENCE and not isinstance(field.reference, Node):
                HasReference._interp_inner(field, self)  # resolve reference
                if not isinstance(field.reference, Node):
                    continue  # ignore

            if field.flags & TypeFlag.IsUnionWith:
                # inline fields from union-ed type to resolved fields
                field.reference._resolve_fields(path)
                for child in field.reference.resolved_fields:
                    existing = self.resolved_fields.get(child.py_ident)
                    # check if self is compatible if overlapping
                    if existing is not None and not existing.equals_type(child):
                        self._on_issue(
                            self=IssueType.MISMATCHED_UNION, subject=self, other=existing
                        )
                        continue
                    resolved_fields.append(ResolvedField.from_field(self, child))
            else:
                # just a normal field
                resolved_fields.append(ResolvedField.from_field(self, field))
        self.resolved_fields.set(resolved_fields)
        self._resolved_fields = True

    def extend_type(self, *bases: "Type") -> "Self":
        """Adds the fields of another type to this one"""
        for base in bases:
            field_ = Field(
                name=None, tag=TypeTag.TYPE_REFERENCE, reference=base, flags=TypeFlag.IsUnionWith
            )
            self.fields.append(field_)
        return self

    def _inputs_from_args(self, args, kwargs) -> dict:
        inputs = {**kwargs}
        input_fields = [f for f in self.resolved_fields if not (f.flags & TypeFlag.IsOutput)]
        for input_t, input in zip(input_fields, args):
            inputs[input_t.py_ident] = input
        return inputs


@node_component
class IsType(Node):
    def _call_inner(self, *args, **kwargs) -> Any:
        inputs = self._inputs_from_args(args, kwargs)
        return TypedDict(self, inputs)


class TypedDict(dict):
    """
    A dot dict based on a type.
    Errors on attribute access if the field doesn't exist, otherwise returns the value (or None).
    """

    _PROPS = ("_type", "_is_output")

    def __init__(self, type: "HasFields", d: dict, is_output: bool = None):
        super().__init__(**d)
        self._type = type
        self._is_output = is_output

    def __getattr__(self, item):
        if item in TypedDict._PROPS:
            return super().__getattr__(item)
        try:
            return dict.__getitem__(self, item)
        except KeyError:
            field = self._type.resolved_fields.get(item)
            if self._is_output is None or bool(field.flags & TypeFlag.IsOutput) == self._is_output:
                return None
        raise AttributeError(item)

    def __setattr__(self, name, value):
        if name in TypedDict._PROPS:
            return super().__setattr__(name, value)

        field = self._type.resolved_fields.get(name)
        if self._is_output is None or bool(field.flags & TypeFlag.IsOutput) == self._is_output:
            return dict.__setitem__(self, name, value)
        raise AttributeError(name)

    def to_dict(self):  # :ToDict
        return self
