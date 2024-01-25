import typing
from copy import deepcopy
from dataclasses import dataclass
from typing import TYPE_CHECKING, Any, Optional, Union

from bench.language.code_ import HasCode
from bench.language.const import (
    NodeType,
    NodeVisibility,
    StatementType,
    StructType,
)
from bench.language.database import HasDatabase
from bench.language.field import HasFields, TypedDict
from bench.language.model import HasModel
from bench.language.node import (
    Node,
    NodeList,
    NRel,
    ScopeNode,
    _Passthrough,
    node,
    node_children,
    node_component,
    node_parent,
    struct_internal,
    struct_property,
)
from bench.language.run import HasRun
from bench.language.tagging import HasTags
from bench.language.task import HasTask
from bench.language.trigger import HasTriggers
from bench.language.validation import validate_is_str, validate_name
from bench.language.value import HasValue
from bench.sql.core import ColumnType
from bench.utils.casing import IdentifierType
from bench.utils.func import dict_minus

if TYPE_CHECKING:
    from bench.language import File, Policy, TypeInfo


@node_component
class IsInstantiable(Node):
    def prune(self, *args, **kwargs) -> Any:
        from bench.language.value import check_type

        assert self.type == StatementType.CLASS, f"{self!r} is not a class"
        combined = {}
        for field, arg in zip(self.fields, args):
            combined[field.py_ident] = arg
        for field in self.fields:
            if field.name in kwargs:
                combined[field.py_ident] = kwargs[field.name]
            elif field.py_ident in kwargs:
                combined[field.py_ident] = kwargs[field.py_ident]
        check_type(combined, self)
        return TypedDict(combined, self)

    def _call_inner(self, *args, **kwargs) -> Any:
        if self.type == StatementType.CLASS:
            from bench.language.value import check_type

            inputs = self._inputs_from_args(args, kwargs)
            check_type(inputs, self)
            return TypedDict(inputs, self)
        elif self.type == StatementType.CHOICE:
            assert len(args) == 1, f"{self!r} must be called with a single argument"
            resolved = self.fields.get(args[0])
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

    def __post_init__(self):
        if self.type in _STATEMENT_DESCRIPTORS:
            raise ValueError(f"statement descriptor for {self.type} already exists")
        _STATEMENT_DESCRIPTORS[self.type] = self


IdentT = IdentifierType

# :StatementDescriptors
_s = _StatementDescriptor
_s(StatementType.BOX, (HasFields,), IdentT.VARIABLE)
_s(StatementType.TAG, (HasFields,), IdentT.VARIABLE)
_s(StatementType.TEXT, (), IdentT.VARIABLE)
_s(StatementType.LINK, (), IdentT.VARIABLE)
_s(StatementType.BLANK, (), IdentT.VARIABLE)
_s(StatementType.CLASS, (IsInstantiable, HasFields), IdentT.TYPE)
_s(StatementType.SIGNAL, (IsInstantiable, HasFields), IdentT.TYPE)
_s(StatementType.CHOICE, (IsInstantiable, HasFields), IdentT.TYPE)
_s(StatementType.TASK, (HasTask, HasRun, HasFields), IdentT.FUNCTION)
_s(StatementType.CODE, (HasCode, HasRun, HasTriggers, HasFields), IdentT.FUNCTION)
_s(StatementType.FLOW, (HasRun, HasFields), IdentT.FUNCTION)
_s(StatementType.MODEL, (HasModel, HasRun, HasFields), IdentT.FUNCTION)
_s(StatementType.SINGLE_VARIABLE, (HasFields, HasValue), IdentT.VARIABLE)
_s(StatementType.MULTI_VARIABLE, (HasFields, HasValue), IdentT.VARIABLE)
_s(StatementType.DATABASE, (HasDatabase, HasFields), IdentT.VARIABLE)
_s(StatementType.VIEW, (HasFields,), IdentT.VARIABLE)
_s(StatementType.SCREEN, (), IdentT.VARIABLE)

assert len(_STATEMENT_DESCRIPTORS) == len(StatementType), "missing statement descriptors"
del _s

_IDENTIFIER_BY_TYPE: dict[StatementType, IdentifierType] = {
    t.type: t.identifier for t in _STATEMENT_DESCRIPTORS.values()
}
_DYNAMIC_COMPONENTS_BY_TYPE: dict[StatementType, tuple[typing.Type[Node], ...]] = {
    t.type: t.dynamic_components for t in _STATEMENT_DESCRIPTORS.values()
}
_ALL_DYNAMIC_COMPONENTS: tuple[typing.Type[Node], ...] = tuple(
    c for t in _STATEMENT_DESCRIPTORS.values() for c in t.dynamic_components
)


@node(
    NodeType.STATEMENT,
    passthrough=(("value", _Passthrough.Full),),
    dynamic_components=_ALL_DYNAMIC_COMPONENTS,
)
class Statement(ScopeNode, HasTags):
    """A Bench building block, the core building block containing logic, schemas, data and AI stuff."""

    parent: Union["Statement", "File"] = node_parent(4, NodeType.STATEMENT, NodeType.FILE)
    children: NodeList["Statement"] = node_children(
        NodeType.STATEMENT, NRel.ORDERED | NRel.NAMED | NRel.SCOPED
    )

    visibility: NodeVisibility = struct_internal(20, default=NodeVisibility.PUBLIC)
    policies: Optional[list["Policy"]] = struct_internal(
        21, default_factory=list, struct_t=StructType.POLICY
    )
    type: StatementType = struct_internal(30, default=StatementType.BLANK)
    bases: list["Statement"] | None = struct_internal(
        31, default=None, require=False, array=True, references=NodeType.STATEMENT
    )
    builtin_base: Optional["TypeInfo"] = struct_internal(
        32, default=None, struct_t=StructType.TYPE_INFO
    )
    is_inline: bool = struct_internal(33, default=True)

    # shared
    name: str | None = struct_property(40, default=None, validate=validate_name)
    order_key: str | None = struct_internal(41, default=None)
    dynamic_key: str | None = struct_internal(42, default=None)
    text: str | None = struct_property(43, default=None, validate=validate_is_str)
    value_packed: Any | None = struct_property(
        44, default=None, copy=deepcopy, column_type=ColumnType.JSON
    )
    secret_value_packed: Any | None = struct_internal(
        45, default=None, encrypt=True, defer=True, copy=deepcopy, column_type=ColumnType.JSON
    )

    # specific
    code: str | None = struct_property(50, default=None, validate=validate_is_str)
    reference: Optional["Statement"] = struct_internal(
        51, require=False, array=False, references=NodeType.STATEMENT
    )

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
        init_name = f"{node.type.lower()}Statement"
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

    def __content_str__(self):
        return ""

    def __repr__(self):  # noqa: we want to override the default repr
        return f"<{self.type.bench_name}Block {self}>"

    def _init_inner(self) -> None:
        # add runtime properties from dynamic components
        for component in self._dynamic_components:
            for prop in component.__properties__.values():
                if prop.is_runtime_only and prop.name not in self.__dict__:
                    setattr(self, prop.name, prop.new())

    def morph(self, to_type: StatementType):
        self.type = to_type
        Statement._init_inner(self)
        if self.attached:
            self._session.update(self, ["type"])
        # what else to do? trigger global reinterp? flush local PG edits?
        raise NotImplementedError(f"{self!r} does not support morphing yet")

    @property
    def identifier_type(self) -> Optional[IdentifierType]:
        return _IDENTIFIER_BY_TYPE[self.type]


# Statement.<type> convenience constructors
Statement.text = Statement.text_
for _type in StatementType:
    if _type == StatementType.TEXT:
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
