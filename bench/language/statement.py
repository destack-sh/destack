from datetime import datetime
import typing
from copy import deepcopy
from dataclasses import dataclass
from typing import TYPE_CHECKING, Any, Optional, Union
from uuid import UUID

from bench.language.code_ import HasCode
from bench.language.const import NodeType, StatementType, TextHeadingLevel, TypeFlag, TypeTag
from bench.language.database import HasDatabase
from bench.language.field import HasFields, TypedDict
from bench.language.model import HasModel
from bench.language.module import (
    Node,
    NodeList,
    NRel,
    ScopeNode,
    _Passthrough,
    node,
    node_ancestor,
    node_children,
    node_component,
    node_parent,
    struct_internal,
    struct_property,
)
from bench.language.run import HasRun
from bench.language.tagging import HasTags
from bench.language.task import HasTask
from bench.language.text import HasText
from bench.language.trigger import HasTriggers
from bench.language.validation import enum_validator, validate_is_str, validate_name
from bench.language.value import HasValue
from bench.sql.core import ColumnType
from bench.utils.func import dict_minus
from bench.utils.utils import IdentifierType, IdentT, identity, to_pyidentifier

if TYPE_CHECKING:
    from bench.language import File


@node_component
class InstantiableType(Node):
    def prune(self, *args, **kwargs) -> Any:
        from bench.language.packer import check_type

        assert self.type == StatementType.CLASS, f"{self!r} is not a class"
        combined = {}
        for field, arg in zip(self.resolved_fields, args):
            combined[field.py_ident] = arg
        for field in self.resolved_fields:
            if field.name in kwargs:
                combined[field.py_ident] = kwargs[field.name]
            elif field.py_ident in kwargs:
                combined[field.py_ident] = kwargs[field.py_ident]
        check_type(combined, self)
        return TypedDict(combined, self)

    def _call_inner(self, *args, **kwargs) -> Any:
        if self.type == StatementType.CLASS:
            from bench.language.packer import check_type

            inputs = self._inputs_from_args(args, kwargs)
            check_type(inputs, self)
            return TypedDict(inputs, self)
        elif self.type == StatementType.CHOICE:
            assert len(args) == 1, f"{self!r} must be called with a single argument"
            resolved = self.resolved_fields.get(args[0])
            if resolved is None:
                raise ValueError(f"{self!r} has no field {args[0]}")
            return resolved.field
        else:
            raise RuntimeError(f"cannot call instantiate on {self!r}")


_STATEMENT_DESCRIPTORS: dict[StatementType, "_StatementDescriptor"] = {}


@dataclass(slots=True)
class _StatementDescriptor:
    type: StatementType
    dynamic_components: tuple[typing.Type[Node], ...]
    identifier: IdentifierType
    passthrough: tuple[tuple[str, _Passthrough], ...] = tuple()
    tag: Optional[TypeTag] = None

    def __post_init__(self):
        if self.type in _STATEMENT_DESCRIPTORS:
            raise ValueError(f"statement descriptor for {self.type} already exists")
        _STATEMENT_DESCRIPTORS[self.type] = self


# :StatementDescriptors
_s = _StatementDescriptor
_s(StatementType.TAG, (HasFields, HasText), IdentT.VARIABLE, tag=TypeTag.STRUCT)
_s(StatementType.TEXT, (HasText,), IdentT.VARIABLE)
_s(StatementType.BLANK, (), IdentT.VARIABLE)
_s(
    StatementType.CLASS,
    (InstantiableType, HasFields, HasText),
    IdentT.TYPE,
    tag=TypeTag.STRUCT,
    passthrough=(("fields", _Passthrough.Full),),
)
_s(
    StatementType.CHOICE,
    (InstantiableType, HasFields, HasText),
    IdentT.TYPE,
    tag=TypeTag.ENUM,
    passthrough=(("fields", _Passthrough.Full),),
)
_s(
    StatementType.TASK,
    (HasTask, HasRun, HasFields, HasText),
    IdentT.METHOD,
    tag=TypeTag.FUNCTION,
)
_s(
    StatementType.CODE,
    (HasCode, HasRun, HasTriggers, HasFields, HasText),
    IdentT.METHOD,
    tag=TypeTag.FUNCTION,
)
_s(StatementType.FLOW, (HasRun, HasFields, HasText), IdentT.METHOD, tag=TypeTag.FUNCTION)
_s(
    StatementType.MODEL,
    (HasModel, HasRun, HasFields, HasText),
    IdentT.METHOD,
    tag=TypeTag.FUNCTION,
)
_s(
    StatementType.VARIABLE,
    (HasValue, HasFields, HasText),
    IdentT.VARIABLE,
    tag=TypeTag.STRUCT,
    passthrough=(("value", _Passthrough.Full),),
)
_s(
    StatementType.DATABASE,
    (HasDatabase, HasFields, HasText),
    IdentT.VARIABLE,
    tag=TypeTag.STRUCT,
    passthrough=(
        ("records", _Passthrough.Full),
        ("resolved_fields", _Passthrough.Scope),
    ),
)
_s(
    StatementType.VIEW,
    (HasFields, HasText),
    IdentT.VARIABLE,
    passthrough=(("fields", _Passthrough.Full),),
)
_s(StatementType.SCREEN, (HasText,), IdentT.VARIABLE)
assert len(_STATEMENT_DESCRIPTORS) == len(StatementType), "missing statement descriptors"
del _s

_IDENTIFIER_BY_TYPE: dict[StatementType, IdentifierType] = {
    t.type: t.identifier for t in _STATEMENT_DESCRIPTORS.values()
}
_PASSTHROUGH_BY_TYPE: dict[StatementType, tuple[tuple[str, _Passthrough]]] = {
    t.type: t.passthrough for t in _STATEMENT_DESCRIPTORS.values()
}
_DYNAMIC_COMPONENTS_BY_TYPE: dict[StatementType, tuple[typing.Type[Node]]] = {
    t.type: t.dynamic_components for t in _STATEMENT_DESCRIPTORS.values()
}
_ALL_DYNAMIC_COMPONENTS: tuple[typing.Type[Node]] = tuple(
    c for t in _STATEMENT_DESCRIPTORS.values() for c in t.dynamic_components
)


@node(
    NodeType.STATEMENT,
    passthrough=(("children", _Passthrough.Scope),),
    dynamic_components=_ALL_DYNAMIC_COMPONENTS,
)
class Statement(ScopeNode, HasTags):
    """A Bench statement."""

    parent: Union["Statement", "File"] = node_parent(4, NodeType.STATEMENT, NodeType.FILE)
    children: NodeList["Statement"] = node_children(
        NodeType.STATEMENT, NRel.Ordered | NRel.Named | NRel.Scoped
    )

    archived_at: datetime = struct_internal(14, default=None, is_cru=True, reflect=True)
    type: StatementType = struct_internal(20, default=StatementType.BLANK)
    file: Optional["File"] = node_ancestor(21, NodeType.FILE)
    name: str | None = struct_property(22, default=None, validate=validate_name)
    order_key: str | None = struct_internal(23, default=None)
    reference: Optional["Statement"] = struct_property(
        24, copy=identity, references=NodeType.STATEMENT
    )
    heading_level: Optional["TextHeadingLevel"] = struct_property(
        25, default=None, validate=enum_validator(TextHeadingLevel)
    )
    text: str | None = struct_property(26, default=None, validate=validate_is_str)
    key: str | None = struct_internal(27, default=None)
    code: str | None = struct_property(28, default=None, validate=validate_is_str)
    value: Any | None = struct_property(
        29, default_factory=dict, copy=deepcopy, store_as=ColumnType.JSON
    )
    versioned: bool = struct_internal(30, default=True)

    @staticmethod
    def new(
        type: Union[str, StatementType] = None,
        name: str = None,
        *args,
        for_parent: Union["Statement", "File", None] = None,
        **kwargs,
    ) -> "Statement":
        if type is None:
            raise ValueError("type must be specified")
        if not isinstance(type, StatementType):
            type = StatementType(type.lower())
        return Statement(type=type, name=name, *args, **kwargs)

    @staticmethod
    def text_(text: str, *args, **kwargs):
        return Statement.new(type=StatementType.TEXT, text=text, *args, **kwargs)

    # the others are defined after the class

    @staticmethod
    def to_python(
        node: "Statement", props: dict, for_parent: Union["Statement", "File", None] = None
    ) -> tuple[str, dict, dict]:
        init_name = f"Statement.{node.type.lower()}"
        if init_name == "Statement.class":
            init_name = "Statement.class_"
        if node.type == StatementType.BLANK:
            init_args = {}
        elif node.type == StatementType.TEXT:
            init_args = {"text": node.text}
            if node.name:
                init_args = {"name": node.name, **init_args}
            props = dict_minus(props, "text")
        else:
            init_args = {"name": node.name}
        return init_name, init_args, dict_minus(props, "name", "type", "tag", "flags")

    @property
    def _components(self) -> tuple[typing.Type[Node]]:
        return _ALL_COMPONENTS_BY_TYPE[self.type]

    @property
    def _dynamic_components(self) -> tuple[typing.Type[Node]]:
        return _DYNAMIC_COMPONENTS_BY_TYPE[self.type]

    @property
    def _instance_cache_key(self) -> str:
        return self.type

    @property
    def _passthrough_targets(self) -> tuple[tuple[str, _Passthrough]] | None:
        return _ALL_PASSTHROUGH_BY_TYPE[self.type]

    def __str__(self):
        return f"{self.path} '{self.name}'" if self.name else self.path

    def __repr__(self):
        return f"<Statement.{self.type.camel_name} {self}>"

    def _init_inner(self) -> None:
        # add runtime properties from dynamic components
        for component in self._dynamic_components:
            for prop in component.__properties__.values():
                if prop.is_runtime and prop.name not in self.__dict__:
                    setattr(self, prop.name, prop.new())

        # IsTyped
        setattr(self, "tag", _STATEMENT_DESCRIPTORS[self.type].tag)
        setattr(self, "flags", TypeFlag.ZERO)
        setattr(self, "hint", None)

    def morph(self, to_type: StatementType):
        self.type = to_type
        Statement._init_inner(self)
        if self.attached:
            self._session._tracer.node_update(self, ["type"])
        # what else to do? trigger global reinterp? flush local PG edits?
        raise NotImplementedError(f"{self!r} does not support morphing yet")

    @property
    def path(self) -> str:
        if self.file is None:
            return f"<detached>.{self.infile_path}"
        else:
            return self.file.path + "." + str(self.infile_path)

    @property
    def infile_path(self) -> str:
        parent = self.parent
        ancestor_parts = [self.py_ident or "<anon>"]
        seen_ids = [self.ck]
        while isinstance(parent, Statement):
            if parent.id in seen_ids:
                ancestor_parts.append("<!loop>")
                break
            ancestor_parts.append(parent.py_ident or "<anon>")
            seen_ids.append(parent.ck)
            parent = parent.parent
        return ".".join(reversed(ancestor_parts))

    @property
    def py_ident(self) -> Optional[str]:
        if self.name is None:
            return None
        else:
            return to_pyidentifier(self.name, _IDENTIFIER_BY_TYPE[self.type])

    @property
    def reference_ck(self) -> Optional[UUID]:
        if isinstance(self.reference, Statement):
            return self.reference.ck
        elif isinstance(self.reference, UUID):
            return self.reference
        else:
            return None


# Statement.<type> convenience constructors
Statement.text = Statement.text_
for _type in StatementType:
    if _type in StatementType.TEXT:
        continue
    method = staticmethod(
        lambda name=None, _type=_type, *args, **kwargs: Statement.new(
            _type, name=name, *args, **kwargs
        )
    )
    method_name = _type.lower()
    if method_name in ("type", "class"):
        method_name += "_"
    setattr(Statement, method_name, method)

_ALL_COMPONENTS_BY_TYPE: dict[StatementType, tuple[typing.Type[Node]]] = {
    t: _DYNAMIC_COMPONENTS_BY_TYPE[t] + Statement.__static_components__ for t in StatementType
}
_ALL_PASSTHROUGH_BY_TYPE: dict[StatementType, tuple[tuple[str, _Passthrough]]] = {
    # custom passthrough + default passthrough
    t: _PASSTHROUGH_BY_TYPE.get(t, tuple()) + Statement.__static_passthrough__
    for t in StatementType
}
