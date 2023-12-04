import dataclasses
import typing
import uuid
from dataclasses import dataclass
from typing import Any, Collection, Optional, Union
from uuid import UUID

import structlog

from bench.language.builtin import symbolx_lib
from bench.language.const import (
    RESERVED_TYPE_TAGS,
    IssueType,
    NodeType,
    StatementReference,
    StatementType,
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
    _FieldExpressionBase,
    _NodeChange,
    binternal,
    bproperty,
    bruntime,
    get_node_id,
    nchildren,
    node,
    node_component,
    nparent,
)
from bench.language.reference import HasReference
from bench.language.text import HasText
from bench.language.validation import (
    ValidationHandler,
    enum_validator,
    flag_validator,
    on_issue_raise,
    validate_name,
)
from bench.language.value import HasValue
from bench.utils.fractional import generate_n_keys_between
from bench.utils.func import dict_minus, nextn
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
            expected_str = f"field '{expected.py_ident}' ({expected._type_str})"
        else:
            expected_fields_str = ", ".join(
                f"'{f.py_ident}' ({f._type_str})" for f in expected.resolved_fields
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
    TypeTag.BLOB,
    TypeTag.VECTOR,
    TypeTag.NODE,
]
DEFAULT_EMBEDDING_DIMENSION = 768  # currently only support :FixedEmbeddingDimension
Vector = typing.NewType("Vector", Union[bytes, list[float]])
Json = typing.NewType("Json", dict)
Key = typing.NewType("Key", str)
RichText = typing.NewType("RichText", str)

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
    TypeHint.IMAGE: TypeTag.BLOB,
    TypeHint.VIDEO: TypeTag.BLOB,
    TypeHint.AUDIO: TypeTag.BLOB,
    # node
    TypeHint.FILE: TypeTag.NODE,
    TypeHint.STATEMENT: TypeTag.NODE,
    TypeHint.FIELD: TypeTag.NODE,
    TypeHint.RUN: TypeTag.NODE,
    TypeHint.RECORD: TypeTag.NODE,
    TypeHint.BLOB: TypeTag.NODE,
    # embedding
    TypeHint.EMBEDDING: TypeTag.VECTOR,
}

STORAGE_FORMAT_BY_TYPE_TAG = {
    TypeTag.STRING: TypeStorageFormat.STRING,
    TypeTag.JSON: TypeStorageFormat.OBJECT,
    TypeTag.NUMBER: TypeStorageFormat.DOUBLE,
    TypeTag.BOOLEAN: TypeStorageFormat.BOOLEAN,
    TypeTag.VECTOR: TypeStorageFormat.VECTOR,
    TypeTag.BLOB: TypeStorageFormat.OBJECT,
    TypeTag.STRUCT: TypeStorageFormat.OBJECT,
    TypeTag.ENUM: TypeStorageFormat.KEYWORD,
    TypeTag.LITERAL: TypeStorageFormat.KEYWORD,
    TypeTag.NODE: TypeStorageFormat.RELATION,
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
    if flags & TypeFlag.IS_SECRET:
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
        return self.replace(_flags=self._flags | TypeFlag.IS_ARRAY)

    def scalar(self) -> "Type":
        return self.replace(_flags=self._flags & ~TypeFlag.IS_ARRAY)

    def required(self) -> "Type":
        return self.replace(_flags=self._flags & ~TypeFlag.IS_OPTIONAL)

    def optional(self) -> "Type":
        return self.replace(_flags=self._flags | TypeFlag.IS_OPTIONAL)

    def input(self) -> "Type":
        return self.replace(_flags=self._flags & ~TypeFlag.IS_OUTPUT)

    def output(self) -> "Type":
        return self.replace(_flags=self._flags | TypeFlag.IS_OUTPUT)

    def config(self) -> "Type":
        return self.replace(_flags=self._flags | TypeFlag.IS_CONFIG)

    def hidden(self) -> "Type":
        return self.replace(_flags=self._flags | TypeFlag.IS_HIDDEN)

    @staticmethod
    def reference(reference: Union["Statement", StatementReference, None]) -> "Type":
        return Type(
            _tag=TypeTag.TYPE_REFERENCE, _hint=None, _flags=TypeFlag.ZERO, _reference=reference
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
        return Type(_tag=tag, _hint=None, _flags=TypeFlag.ZERO)

    @staticmethod
    def from_hint(hint: TypeHint) -> "Type":
        return Type(_tag=TYPE_TAG_BY_TYPE_HINT[hint], _hint=hint, _flags=TypeFlag.ZERO)

    @staticmethod
    def to_python(node: "Type") -> str:
        """Reconstruct minimal Python code to create this type."""
        if node._tag == TypeTag.TYPE_REFERENCE:
            if isinstance(node._reference, Node):
                reference_str = node._reference.py_ident
            elif isinstance(node._reference, UUID):
                reference_str = f"UUID('{node._reference}')"
            else:
                reference_str = repr(node._reference)
            node_str = f"Type.reference({reference_str})"
        else:
            node_str = f"Type.{node._hint.name if node._hint else node._tag.name}"
        if node._flags & TypeFlag.IS_ARRAY:
            node_str += ".array()"
        if node._flags & TypeFlag.IS_ARRAYABLE:
            node_str += ".arrayable()"
        if not (node._flags & TypeFlag.IS_OPTIONAL):
            node_str += ".required()"
        if node._flags & TypeFlag.IS_OUTPUT:
            node_str += ".output()"
        if node._flags & TypeFlag.IS_CONFIG:
            node_str += ".config()"
        if node._flags & TypeFlag.IS_HIDDEN:
            node_str += ".hidden()"
        return node_str


for tag in TypeTag:
    if tag in RESERVED_TYPE_TAGS:
        continue
    _type = Type(_tag=tag, _hint=None, _flags=TypeFlag.IS_OPTIONAL)
    setattr(Type, tag.name, _type)
for hint in TypeHint:
    _type = Type(_tag=TYPE_TAG_BY_TYPE_HINT[hint], _hint=hint, _flags=TypeFlag.IS_OPTIONAL)
    setattr(Type, hint.name, _type)


@node_component
class HasType(Node):
    key: str | None = binternal(default=None)

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


@node(node_type=NodeType.FIELD)
class Field(HasText, HasValue, HasReference, HasType, _FieldExpressionBase):
    parent: Union["Statement", None] = nparent(NodeType.STATEMENT)
    name: str | None = bproperty(default=None, validate=validate_name)
    order_key: str | None = binternal(default=None)
    tag: TypeTag = bproperty(is_required=True, validate=enum_validator(TypeTag))
    hint: TypeHint | None = bproperty(default=None, validate=enum_validator(TypeHint))
    flags: TypeFlag = bproperty(default=TypeFlag.ZERO, validate=flag_validator(TypeFlag))
    reflected: bool = bruntime(default=False)

    @staticmethod
    def new(
        name: str = None,
        type: Union[TypeTag, TypeHint, "Statement", str, type] = None,
        text: str = None,
        flags: TypeFlag = TypeFlag.ZERO,
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

        # default to optional if parent is not a function (and not set via Type)
        if not (for_parent and for_parent.tag == TypeTag.FUNCTION) and not isinstance(type, Type):
            flags |= TypeFlag.IS_OPTIONAL

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
        elif (
            isinstance(type, Node) and type.node_type == NodeType.STATEMENT or isinstance(type, str)
        ):
            kwargs["tag"] = TypeTag.TYPE_REFERENCE
            kwargs["reference"] = type
        else:
            raise ValueError(f"unexpected type {type!r}")
        if kwargs.get("hint") == TypeHint.SECRET:
            flags = flags | TypeFlag.IS_SECRET

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
        return Field.new(name=name, type=type, flags=TypeFlag.IS_OUTPUT, text=text, *args, **kwargs)

    @staticmethod
    def literal(name: str, text: str = None, *args, **kwargs) -> "Field":
        return Field.new(name=name, text=text, type=TypeTag.LITERAL, *args, **kwargs)

    @staticmethod
    def union(type: Union["Statement", str], *args, **kwargs):
        return Field.new(type=type, flags=TypeFlag.IS_UNION_WITH, *args, **kwargs)

    @staticmethod
    def config(name: str, *args, **kwargs) -> "Field":
        return Field.new(name=name, flags=TypeFlag.IS_CONFIG, *args, **kwargs)

    @staticmethod
    def to_python(
        node: "Field", props: dict, for_parent: "Statement" = None
    ) -> tuple[str, dict, dict]:
        props = {**props}
        implicit_optional = (
            node.flags == TypeFlag.IS_OPTIONAL and for_parent and for_parent.tag != TypeTag.FUNCTION
        )
        if "flags" in props and (node.flags == 0 or implicit_optional):
            del props["flags"]
        if node.tag == TypeTag.LITERAL:
            init_args = {"name": props["name"], "text": props.get("text")}
            init_name = "Field.literal"
        elif node.flags & TypeFlag.IS_UNION_WITH:
            init_args = {"type": node.reference}
            init_name = "Field.union"
        elif not node.flags and node.tag == TypeTag.TYPE_REFERENCE:
            init_args = {"type": node.reference}
            init_name = "Field.new"
        else:
            type = Type.from_field(node)
            init_args = {"name": props["name"], "type": type, "text": props.get("text")}
            if for_parent and for_parent.tag == TypeTag.FUNCTION:
                init_name = "Field.output" if node.flags & TypeFlag.IS_OUTPUT else "Field.input"
                type._flags &= ~TypeFlag.IS_OUTPUT  # ignore flag, already handled
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
        if self.tag == TypeTag.LITERAL and isinstance(other, str):
            return self.name == other or self.py_ident == other

        return _FieldExpressionBase.__eq__(self, other)  # override to avoid recursion

    @property
    def _type_of_value(self) -> "HasFields":
        from bench.language.libs import symbolx_lib

        return symbolx_lib.resolve(".reflect.FieldMetadata")

    def _init_inner(self):
        self.key = self.key or new_dynamic_node_key(self.ck)

    def _validate_inner(self, properties: Collection[str], on_issue: "ValidationHandler") -> None:
        if self.hint is not None:
            tag = TYPE_TAG_BY_TYPE_HINT[self.hint]
            if self.tag != tag:
                on_issue(self, f"expected {tag} for {self.hint} ({self.tag})", ["tag", "hint"])
        if self.flags & TypeFlag.IS_UNION_WITH:
            if self.tag != TypeTag.TYPE_REFERENCE:
                on_issue(
                    self,
                    f"expected reference for IsUnionWith ({self.tag})",
                    ["tag", "reference", "flags"],
                )
        if self.flags & TypeFlag.IS_SECRET:
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
        is_array = bool(self.flags & TypeFlag.IS_ARRAY or self.flags & TypeFlag.IS_ARRAYABLE)
        if self._storage_format == TypeStorageFormat.VECTOR:
            typed_key = f"{self.key}-{self._storage_format.value}{self.dimensions}"
        else:
            typed_key = f"{self.key}-{self._storage_format.value}"
        if is_array:
            typed_key += "-arr"
        return typed_key

    @property
    def _source_key(self) -> str:
        return "value." + self._typed_key

    @property
    def _subkey(self) -> str | None:
        return None  # for FieldQueryOps


@node(node_type=NodeType.RESOLVED_FIELD)
class ResolvedField(Field):
    parent: "Statement" = nparent(NodeType.STATEMENT)
    field: Field = binternal()

    @property
    def field_ck(self) -> UUID:
        return self.field.ck

    @property
    def resolved_fields(self):
        return self.reference.resolved_fields if isinstance(self.reference, Node) else []

    @property
    def _is_from_union(self) -> bool:
        return self.parent != self.field.parent

    @staticmethod
    def from_field(for_parent: Node, field: Field) -> "ResolvedField":
        if isinstance(field, ResolvedField):
            field = field.field
        if field.tag == TypeTag.TYPE_REFERENCE and not isinstance(field.reference, Node):
            raise RuntimeError(f"unresolved reference {field.reference} in {field!r}")
        ck = uuid.uuid5(for_parent.ck, field.ck.hex)
        id = get_node_id(for_parent.module.id, ck) if for_parent.module else None
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
            _status=NS.SOURCE,
        )
        resolved_field._interp_self(for_parent, on_issue=on_issue_raise)
        return resolved_field


@node_component
class HasFields(HasType):
    """A node with fields"""

    fields: NodeList["Field"] = nchildren(NodeType.FIELD, NRel.Named | NRel.Scoped | NRel.Ordered)

    resolved_fields: NodeList["ResolvedField"] = nchildren(
        NodeType.RESOLVED_FIELD, NRel.Named | NRel.Keyed | NRel.Ordered, alias="f"
    )
    _did_resolve_fields: bool = bruntime(default=False)

    def _init_inner(self):
        if self.key is None:
            self.key = new_dynamic_node_key(self.ck)

    def _clear_inner(self, scope: Optional[ScopeNode]) -> None:
        self.resolved_fields.clear(_trigger=_NC.UpdateLists)
        self._did_resolve_fields = False

    def _interp_inner(self, scope: ScopeNode, on_issue: "ValidationHandler") -> None:
        self._resolve_fields([], on_issue)

    def _resolve_fields(
        self: "HasFields", path: list[HasType], on_issue: "ValidationHandler"
    ) -> None:
        """
        Resolves (and inlines) field references and unions.
        """
        if self._did_resolve_fields:
            return  # already resolved

        if any(f.id == self.id for f in path):
            # circular panic
            path = "->".join(n.name for n in path + [self])
            on_issue(type=IssueType.CIRCULAR_UNION, subject=self, path=path)
            self._did_resolve_fields = True
            return

        # resolve fields recursively (inlining any valid unions)
        path = path + [self]
        resolved_fields: list[ResolvedField] = []
        for field in self.fields:
            # try to resolve reference or skip this field
            if field.tag == TypeTag.TYPE_REFERENCE and not isinstance(field.reference, Node):
                HasReference._interp_inner(field, self, field.scope._on_issue)  # resolve ref
                if not isinstance(field.reference, Node):
                    continue  # interp error, ignore

            if field.flags & TypeFlag.IS_UNION_WITH:
                if not isinstance(field.reference, Node):
                    continue  # validation error, ignore
                # inline fields from union-ed type to resolved fields
                field.reference._resolve_fields(path, on_issue)
                for child in field.reference.resolved_fields:
                    if child.flags & TypeFlag.IS_CONFIG:
                        continue  # ignore config fields
                    existing = nextn(f for f in resolved_fields if f.py_ident == child.py_ident)
                    # check if self is compatible if overlapping
                    if existing is None:
                        resolved_fields.append(ResolvedField.from_field(self, child))
                    elif not existing.equals_type(child):
                        on_issue(self=IssueType.MISMATCHED_UNION, subject=self, other=existing)
            else:
                # just a normal field
                resolved_fields.append(ResolvedField.from_field(self, field))

        # add any special inlined fields
        if self.node_type == NodeType.STATEMENT and self.type == StatementType.TASK:
            run_config = symbolx_lib.resolve(".reflect.TaskRunConfig")
            resolved_fields.extend(ResolvedField.from_field(self, f) for f in run_config.fields)

        # maintain resolved field order
        for i, ok in enumerate(generate_n_keys_between(None, None, len(resolved_fields))):
            resolved_fields[i].order_key = ok
        self.resolved_fields.set(resolved_fields, _trigger=_NodeChange.UpdateLists)
        self._did_resolve_fields = True

    def _inputs_from_args(self, args, kwargs) -> dict:
        inputs = {**kwargs}
        input_fields = [f for f in self.resolved_fields if not (f.flags & TypeFlag.IS_OUTPUT)]
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

    def __str__(self):
        return super().__str__()

    def __repr__(self):
        kwargs_str = ", ".join(f"{k}={v!r}" for k, v in self.items())
        return f"{self._type.py_ident}({kwargs_str})"

    def __getitem__(self, item):
        try:
            return dict.__getitem__(self, item)
        except KeyError:
            field = self._type.resolved_fields.get(item)
            if field and (
                self._is_output is None or bool(field.flags & TypeFlag.IS_OUTPUT) == self._is_output
            ):
                return None
        raise KeyError(f"no key {item!r} on {self._type!r}")

    def __getattr__(self, item):
        if item in TypedDict._PROPS:
            return super().__getattr__(item)
        try:
            return dict.__getitem__(self, item)
        except KeyError:
            field = self._type.resolved_fields.get(item)
            if field and (
                self._is_output is None or bool(field.flags & TypeFlag.IS_OUTPUT) == self._is_output
            ):
                return None
        raise AttributeError(f"no attribute {item!r} on {self._type!r}")

    def __setattr__(self, name, value):
        if name in TypedDict._PROPS:
            return super().__setattr__(name, value)

        field = self._type.resolved_fields.get(name)
        if field and (
            self._is_output is None or bool(field.flags & TypeFlag.IS_OUTPUT) == self._is_output
        ):
            return dict.__setitem__(self, name, value)
        raise AttributeError(f"cannot set attribute {name!r} on {self._type!r}")

    def to_dict(self):  # :ToDict
        return self
