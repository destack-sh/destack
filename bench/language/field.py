import dataclasses
import enum
import typing
import uuid
from dataclasses import dataclass
from typing import Any, Collection, Optional, Union
from uuid import UUID

import structlog

from bench.language.const import (
    MNT,
    RESERVED_TYPE_TAGS,
    IssueType,
    StatementReference,
    TypeFlag,
    TypeHint,
    TypeStorageFormat,
    TypeTag,
    new_dynamic_node_key,
)
from bench.language.module import (
    _NC,
    NS,
    Node,
    NodeList,
    NRel,
    ScopeNode,
    _NodeChange,
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
from bench.language.reference import HasReference, NodeVisitor
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
    from bench.language import Statement

logger = structlog.get_logger(__name__)


class TypeError(TypeError):
    def __init__(
        self,
        value: Any,
        expected: "HasType",
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
RichText = typing.NewType("RichText", str)


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
    # node
    TypeHint.STATEMENT: TypeTag.NODE,
    TypeHint.FIELD: TypeTag.NODE,
    TypeHint.RUN: TypeTag.NODE,
    # embedding
    TypeHint.EMBEDDING: TypeTag.VECTOR,
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


def _type_str(tag: TypeTag, hint: TypeHint, flags: TypeFlag) -> str:
    flag_str = ", ".join(flag.short_name.lower() for flag in TypeFlag if flags & flag)
    flags_str = f" ({flag_str})" if flag_str else ""
    if hint:
        return f"{hint}{flags_str}"
    else:
        return f"{tag}{flags_str}"


@dataclass
class Type:
    """Detached type information. Mostly for convenient Field construction."""

    # private because this Type isn't meant to be used directly, only for construction
    _tag: TypeTag
    _hint: Optional[TypeHint]
    _flags: TypeFlag
    _reference: Union["Statement", StatementReference, None] = None

    def __str__(self) -> str:
        return _type_str(self._tag, self._hint, self._flags)

    def __repr__(self):
        return f"<{self.__class__.__name__} {self}>"

    def replace(self, **kwargs) -> "Type":
        return dataclasses.replace(self, **kwargs)

    def array(self) -> "Type":
        return self.replace(_flags=self._flags | TypeFlag.IsArray)

    def scalar(self) -> "Type":
        return self.replace(_flags=self._flags & ~TypeFlag.IsArray)

    def required(self) -> "Type":
        return self.replace(_flags=self._flags & ~TypeFlag.IsOptional)

    def optional(self) -> "Type":
        return self.replace(_flags=self._flags | TypeFlag.IsOptional)

    def input(self) -> "Type":
        return self.replace(_flags=self._flags & ~TypeFlag.IsOutput)

    def output(self) -> "Type":
        return self.replace(_flags=self._flags | TypeFlag.IsOutput)

    def config(self) -> "Type":
        return self.replace(_flags=self._flags | TypeFlag.IsConfig)

    def hidden(self) -> "Type":
        return self.replace(_flags=self._flags | TypeFlag.IsHidden)

    @staticmethod
    def reference(reference: Union["Statement", StatementReference, None]) -> "Type":
        return Type(
            _tag=TypeTag.TYPE_REFERENCE, _hint=None, _flags=TypeFlag.Zero, _reference=reference
        )

    @staticmethod
    def from_field(field: "Field") -> "Type":
        return Type(
            _tag=field.tag,
            _hint=field.hint,
            _flags=field.flags,
            _reference=field.reference,
        )

    @staticmethod
    def from_tag(tag: TypeTag) -> "Type":
        return Type(_tag=tag, _hint=None, _flags=TypeFlag.Zero)

    @staticmethod
    def from_hint(hint: TypeHint) -> "Type":
        return Type(_tag=TYPE_TAG_BY_TYPE_HINT[hint], _hint=hint, _flags=TypeFlag.Zero)

    @staticmethod
    def to_python(node: "Type") -> str:
        """Reconstruct minimal Python code to create this type."""
        if node._tag == TypeTag.TYPE_REFERENCE:
            node_str = f'Type.reference("{node._reference.py_ident}")'
        else:
            node_str = f"Type.{node._hint.name if node._hint else node._tag.name}"
        if node._flags != TypeFlag.Zero:
            if node._flags & TypeFlag.IsArray:
                node_str += ".array()"
            if node._flags & TypeFlag.IsArrayable:
                node_str += ".arrayable()"
            if node._flags & ~TypeFlag.IsOptional:
                node_str += ".required()"
            if node._flags & TypeFlag.IsOutput:
                node_str += ".output()"
            if node._flags & TypeFlag.IsConfig:
                node_str += ".config()"
            if node._flags & TypeFlag.IsHidden:
                node_str += ".hidden()"
        return node_str


for tag in TypeTag:
    if tag in RESERVED_TYPE_TAGS:
        continue
    _type = Type(_tag=tag, _hint=None, _flags=TypeFlag.IsOptional)
    setattr(Type, tag.name, _type)
for hint in TypeHint:
    _type = Type(_tag=TYPE_TAG_BY_TYPE_HINT[hint], _hint=hint, _flags=TypeFlag.IsOptional)
    setattr(Type, hint.name, _type)


@node_component
class HasType(Node):
    key: str = ninternal(default=None)

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
    def _effective_type(self) -> Union["HasType", "Statement"]:
        if isinstance(self.reference, Node):
            return self.reference
        else:
            return self

    @property
    def _type_str(self) -> str:
        return _type_str(self.tag, self.hint, self.flags)

    @property
    def _effective_tag(self) -> TypeTag:
        return self._effective_type.tag

    @property
    def _effective_hint(self) -> Optional[TypeHint]:
        return self._effective_type.hint

    def equals_type(self, other: "HasType") -> bool:
        return (
            self._effective_tag == other._effective_tag
            and self._effective_hint == other._effective_hint
            and self.flags == other.flags
            and self.reference == other.reference
        )


@node(mnt=MNT.FIELD)
class Field(HasText, HasValue, HasReference, HasType, FieldQueryOps):
    parent: Union["Statement", None] = nparent(MNT.STATEMENT)
    name: str | None = nproperty(default=None, validate=validate_name)
    order_key: str | None = ninternal(default=None)
    tag: TypeTag = nproperty(is_required=True, validate=enum_validator(TypeTag))
    hint: TypeHint | None = nproperty(default=None, validate=enum_validator(TypeHint))
    flags: TypeFlag = nproperty(default=TypeFlag.Zero, validate=flag_validator(TypeFlag))

    @staticmethod
    def new(
        name: str = None,
        type: Union[TypeTag, TypeHint, "Statement", str, type] = None,
        text: str = None,
        flags: TypeFlag = TypeFlag.Zero,
        *args,
        for_parent: "Statement" = None,
        **kwargs,
    ) -> "Field":
        # default to literal or string if no type is specified
        if type is None:
            if for_parent and for_parent.tag == TypeTag.ENUM:
                type = TypeTag.LITERAL
                if name is None:
                    name = f"Option {len(for_parent.fields) + 1}"
            else:
                type = TypeTag.STRING

        # default to optional if parent is not a function
        if not (for_parent and for_parent.tag == TypeTag.FUNCTION):
            flags |= TypeFlag.IsOptional

        # coerce type
        if isinstance(type, Type):
            kwargs["tag"] = type._tag
            kwargs["hint"] = type._hint
            flags = type._flags | flags
            kwargs["reference"] = type._reference
        elif isinstance(type, TypeTag):
            kwargs["tag"] = type
        elif isinstance(type, TypeHint):
            kwargs["hint"] = type
            kwargs["tag"] = TYPE_TAG_BY_TYPE_HINT[type]
        elif type is str:
            kwargs["tag"] = TypeTag.STRING
        elif type is int:
            kwargs["tag"] = TypeTag.NUMBER
            kwargs["hint"] = TypeHint.INTEGER
        elif type is float:
            kwargs["tag"] = TypeTag.NUMBER
        elif type is bool:
            kwargs["tag"] = TypeTag.BOOLEAN
        elif isinstance(type, Node) and type.mnt == MNT.STATEMENT or isinstance(type, str):
            kwargs["tag"] = TypeTag.TYPE_REFERENCE
            kwargs["reference"] = type
        else:
            raise ValueError(f"unexpected type {type!r}")
        if kwargs.get("hint") == TypeHint.SECRET:
            flags = flags | TypeFlag.IsSecret

        return Field(name=name, text=text, flags=flags, *args, **kwargs)

    input = new  # same as new but more explicit

    @staticmethod
    def output(
        name: str,
        type: Union[TypeTag, TypeHint, "Statement", str, type] = None,
        text: str = None,
        *args,
        **kwargs,
    ) -> "Field":
        return Field.new(name=name, type=type, flags=TypeFlag.IsOutput, text=text, *args, **kwargs)

    @staticmethod
    def literal(name: str, text: str = None, *args, **kwargs) -> "Field":
        return Field.new(name=name, text=text, type=TypeTag.LITERAL, *args, **kwargs)

    @staticmethod
    def union(type: Union["Statement", str], *args, **kwargs):
        return Field.new(type=type, flags=TypeFlag.IsUnionWith, *args, **kwargs)

    @staticmethod
    def config(name: str, *args, **kwargs) -> "Field":
        return Field.new(name=name, flags=TypeFlag.IsConfig, *args, **kwargs)

    @staticmethod
    def to_python(
        node: "Field", props: dict, for_parent: "Statement" = None
    ) -> tuple[str, dict, dict]:
        props = {**props}
        implicit_optional = (
            node.flags == TypeFlag.IsOptional and for_parent and for_parent.tag != TypeTag.FUNCTION
        )
        if "flags" in props and (node.flags == 0 or implicit_optional):
            del props["flags"]
        if node.tag == TypeTag.LITERAL:
            init_args = {"name": props["name"], "text": props.get("text")}
            init_name = "Field.literal"
        elif node.flags & TypeFlag.IsUnionWith:
            init_args = {"type": node.reference}
            init_name = "Field.union"
        else:
            type = Type.from_field(node)
            init_args = {"name": props["name"], "type": type, "text": props.get("text")}
            if for_parent and for_parent.tag == TypeTag.FUNCTION:
                init_name = "Field.output" if node.flags & TypeFlag.IsOutput else "Field.input"
                type._flags &= ~TypeFlag.IsOutput  # ignore flag, already handled
            else:
                init_name = "Field.new"

        init_kwargs = dict_minus(props, "name", "flags", "text", "tag", "hint", "reference")
        return init_name, init_args, init_kwargs

    def __str__(self):
        name_str = f"{self.path} '{self.name}' " if self.name else ""
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

    def _validate_inner(self, properties: Collection[str], on_issue: "ValidationHandler") -> None:
        if self.hint is not None:
            tag = TYPE_TAG_BY_TYPE_HINT[self.hint]
            if self.tag != tag:
                on_issue(self, f"expected {tag} for {self.hint} ({self.tag})", ["tag", "hint"])
        if self.flags & TypeFlag.IsUnionWith:
            if self.tag != TypeTag.TYPE_REFERENCE:
                on_issue(
                    self,
                    f"expected reference for IsUnionWith ({self.tag})",
                    ["tag", "reference", "flags"],
                )
        if self.flags & TypeFlag.IsSecret:
            if self.hint != TypeHint.SECRET:
                on_issue(
                    self, f"expected secret hint for IsSecret ({self.hint})", ["hint", "flags"]
                )

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


@node(mnt=MNT.RESOLVED_FIELD)
class ResolvedField(Field):
    parent: "Statement" = nparent(MNT.STATEMENT)
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
class HasFields(HasType):
    """A node with fields"""

    fields: NodeList["Field"] = nchildren(MNT.FIELD, NRel.Named | NRel.Scoped | NRel.Ordered)

    resolved_fields: NodeList["ResolvedField"] = nchildren(
        MNT.RESOLVED_FIELD, NRel.Named | NRel.Keyed | NRel.Ordered
    )
    _did_resolve_fields: bool = nruntime(default=False)

    def _init_inner(self):
        if self.key is None:
            self.key = new_dynamic_node_key(self.ck)

    def _clear_inner(self) -> None:
        self.resolved_fields.clear(_trigger=_NC.UpdateLists)
        self._did_resolve_fields = False

    def _interp_inner(self, scope: ScopeNode) -> None:
        self._resolve_fields([])

    def _resolve_fields(self: "HasFields", path: list[HasType]) -> None:
        """
        Resolves (and inlines) field references and unions.
        """
        if self._did_resolve_fields:
            return  # already resolved

        if any(f.id == self.id for f in path):
            # circular panic
            path = "->".join(n.name for n in path + [self])
            self._on_issue(type=IssueType.CIRCULAR_UNION, subject=self, path=path)
            self._did_resolve_fields = True
            return

        path = path + [self]
        resolved_fields: list[ResolvedField] = []
        for field in self.fields:
            # try to resolve reference or skip this field
            if field.tag == TypeTag.TYPE_REFERENCE and not isinstance(field.reference, Node):
                HasReference._interp_inner(field, self)  # resolve reference
                if not isinstance(field.reference, Node):
                    continue  # interp error, ignore

            if field.flags & TypeFlag.IsUnionWith:
                if not isinstance(field.reference, Node):
                    continue  # validation error, ignore
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
        self.resolved_fields.set(resolved_fields, _trigger=_NodeChange.UpdateLists)
        self._did_resolve_fields = True

    def _inputs_from_args(self, args, kwargs) -> dict:
        inputs = {**kwargs}
        input_fields = [f for f in self.resolved_fields if not (f.flags & TypeFlag.IsOutput)]
        for input_t, input in zip(input_fields, args):
            inputs[input_t.py_ident] = input
        return inputs


class TypedDict(dict):
    """
    A dot dict based on a type.
    Errors on attribute access if the field doesn't exist, otherwise returns the value (or None).
    """

    _PROPS = ("_type", "_is_output")

    def __init__(self, d: dict, type: "HasFields", is_output: bool = None):
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
