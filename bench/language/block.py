import typing
from copy import deepcopy
from dataclasses import dataclass
from typing import TYPE_CHECKING, Any, Optional, Union

from bench.language.code_ import HasCode
from bench.language.const import BlockType, NodeType, NodeVisibility, StructType
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
    p_child,
    node_component,
    p_parent,
    p_internal,
    p_tracked,
)
from bench.language.run import HasRun
from bench.language.tag import HasTags
from bench.language.task import HasTask
from bench.language.trigger import HasTriggers
from bench.language.validation import validate_name
from bench.language.value import HasValue
from bench.sql.core import PrimitiveType
from bench.utils.casing import IdentifierType
from bench.utils.func import dict_minus

if TYPE_CHECKING:
    from bench.language import Package, Policy, TypeInfo, Badge


@node_component
class IsInstantiable(Node):
    def prune(self, *args, **kwargs) -> Any:
        from bench.language.value import check_type

        assert self.type == BlockType.CLASS, f"{self!r} is not a class"
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
        if self.type == BlockType.CLASS:
            from bench.language.value import check_type

            inputs = self._inputs_from_args(args, kwargs)
            check_type(inputs, self)
            return TypedDict(inputs, self)
        elif self.type == BlockType.CHOICE:
            assert len(args) == 1, f"{self!r} must be called with a single argument"
            resolved = self.fields.get(args[0])
            if resolved is None:
                raise ValueError(f"{self!r} has no field {args[0]}")
            return resolved.field
        else:
            raise RuntimeError(f"cannot call instantiate on {self!r}")


_BLOCK_DESCRIPTORS: dict[BlockType, "_BlockTypeDescriptor"] = {}


@dataclass(slots=True)
class _BlockTypeDescriptor:
    type: BlockType
    dynamic_components: tuple[typing.Type[Node], ...]
    identifier: IdentifierType

    def __post_init__(self):
        if self.type in _BLOCK_DESCRIPTORS:
            raise ValueError(f"block descriptor for {self.type} already exists")
        _BLOCK_DESCRIPTORS[self.type] = self


IdentT = IdentifierType

_block = _BlockTypeDescriptor
_block(BlockType.PAGE, (HasFields,), IdentT.VARIABLE)
_block(BlockType.TAG, (HasFields,), IdentT.VARIABLE)
_block(BlockType.TEXT, (), IdentT.VARIABLE)
_block(BlockType.BLANK, (), IdentT.VARIABLE)
_block(BlockType.ALIAS, (IsInstantiable,), IdentT.VARIABLE)
_block(BlockType.CLASS, (IsInstantiable, HasFields), IdentT.TYPE)
_block(BlockType.SIGNAL, (IsInstantiable, HasFields), IdentT.TYPE)
_block(BlockType.CHOICE, (IsInstantiable, HasFields), IdentT.TYPE)
_block(BlockType.TASK, (HasTask, HasRun, HasFields), IdentT.FUNCTION)
_block(BlockType.ROUTINE, (HasCode, HasRun, HasTriggers, HasFields), IdentT.FUNCTION)
_block(BlockType.SCRIPT, (HasCode, HasRun, HasTriggers, HasFields), IdentT.FUNCTION)
_block(BlockType.FLOW, (HasRun, HasFields), IdentT.FUNCTION)
_block(BlockType.MODEL, (HasModel, HasRun, HasFields), IdentT.FUNCTION)
_block(BlockType.SINGLE_VARIABLE, (HasFields, HasValue), IdentT.VARIABLE)
_block(BlockType.MULTI_VARIABLE, (HasFields, HasValue), IdentT.VARIABLE)
_block(BlockType.DATABASE, (HasDatabase, HasFields), IdentT.TYPE)
_block(BlockType.QUERY, (HasFields,), IdentT.VARIABLE)
_block(BlockType.SCREEN, (), IdentT.TYPE)
_block(BlockType.ROLE, (), IdentT.TYPE)
_block(BlockType.IDENTITY, (), IdentT.TYPE)

assert len(_BLOCK_DESCRIPTORS) == len(BlockType), "missing block descriptors"
del _block

_IDENTIFIER_BY_TYPE: dict[BlockType, IdentifierType] = {
    t.type: t.identifier for t in _BLOCK_DESCRIPTORS.values()
}
_DYNAMIC_COMPONENTS_BY_TYPE: dict[BlockType, tuple[typing.Type[Node], ...]] = {
    t.type: t.dynamic_components for t in _BLOCK_DESCRIPTORS.values()
}
_ALL_DYNAMIC_COMPONENTS: tuple[typing.Type[Node], ...] = tuple(
    c for t in _BLOCK_DESCRIPTORS.values() for c in t.dynamic_components
)


@node(
    NodeType.BLOCK,
    passthrough=(("value", _Passthrough.Full),),
    dynamic_components=_ALL_DYNAMIC_COMPONENTS,
)
class Block(ScopeNode, HasTags):
    """A core Bench building block containing logic, types, UI, data, AI, and basically anything source."""

    parent: Union["Block", "Package"] = p_parent(4, NodeType.BLOCK, NodeType.PACKAGE)
    children: NodeList["Block"] = p_child(NodeType.BLOCK, NRel.ORDERED | NRel.NAMED | NRel.SCOPED)
    badges: NodeList["Badge"] = p_child(NodeType.BADGE)

    visibility: NodeVisibility = p_tracked(20, default=NodeVisibility.PUBLIC)
    policies: list["Policy"] | None = p_tracked(
        24, default_factory=list, struct=StructType.POLICY, sensitive=True
    )
    type: BlockType = p_internal(30, default=BlockType.BLANK)
    bases: list["Block"] | None = p_tracked(
        31, default=None, require=False, array=True, references=NodeType.BLOCK
    )
    builtin_base: Optional["TypeInfo"] = p_tracked(32, default=None, struct=StructType.TYPE_INFO)
    is_page: bool = p_tracked(33, default=False)

    # shared
    name: str | None = p_tracked(40, default=None, validate=validate_name)
    order_key: str | None = p_internal(41, default=None)
    dynamic_key: str | None = p_internal(42, default=None)
    text: str | None = p_tracked(43, default=None)
    value_packed: Any | None = p_internal(
        44, default=None, copy=deepcopy, primitive_type=PrimitiveType.JSON
    )
    secret_value_packed: Any | None = p_internal(
        45,
        default=None,
        encrypt=True,
        defer=True,
        sensitive=True,
        copy=deepcopy,
        primitive_type=PrimitiveType.JSON,
    )
    code: str | None = p_tracked(50, default=None)

    # specific
    reference: Optional["Block"] = p_tracked(
        51, require=False, array=False, references=NodeType.BLOCK
    )
    delegated_policies: list["Policy"] | None = p_tracked(
        52, default_factory=list, struct=StructType.POLICY, sensitive=True
    )

    @staticmethod
    def new(
        type: Union[str, BlockType] = None,
        name: str = None,
        *args,
        for_parent: Union["Block", "Package", None] = None,
        **kwargs,
    ) -> "Block":
        if type is None:
            raise ValueError("type must be specified")
        if not isinstance(type, BlockType):
            type = BlockType(type.lower())
        return Block(type=type, name=name, *args, **kwargs)

    @staticmethod
    def text_(text: str, *args, **kwargs):
        return Block.new(type=BlockType.TEXT, text=text, *args, **kwargs)

    # the others are defined after the class

    @staticmethod
    def to_python(
        node, props: dict, for_parent: Union["Block", "Package", None] = None
    ) -> tuple[str, dict, dict]:
        init_name = f"{node.type.lower()}Block"
        if init_name == "Block.class":
            init_name = "Block.class_"
        if node.type == BlockType.BLANK:
            init_args = {}
        elif node.type == BlockType.TEXT:
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
    def is_type(self) -> bool:
        return self.type.is_type

    @property
    def is_runnable(self) -> bool:
        return self.type.is_runnable

    @property
    def is_scriptable(self) -> bool:
        return self.type.is_scriptable

    @property
    def is_nestable(self) -> bool:
        return self.type.is_nestable

    def __content_str__(self):
        return ""  # implemented by dynamic components

    def __repr__(self):  # noqa: we want to override the default repr
        return f"<{self.type.bench_name}Block {self}>"

    def _init_inner(self) -> None:
        # add runtime properties from dynamic components
        for component in self._dynamic_components:
            for prop in component.__properties__.values():
                if prop.is_runtime_only and prop.name not in self.__dict__:
                    setattr(self, prop.name, prop.new())

    def morph(self, to_type: BlockType):
        raise NotImplementedError(f"{self!r} does not support morphing yet")

    @property
    def identifier_type(self) -> Optional[IdentifierType]:
        return _IDENTIFIER_BY_TYPE[self.type]


# Block.<type> convenience constructors
Block.text = Block.text_
for _type in BlockType:
    if _type == BlockType.TEXT:
        continue
    method = staticmethod(
        lambda name=None, _type=_type, *args, **kwargs: Block.new(_type, name=name, *args, **kwargs)
    )
    method_name = _type.lower()
    if method_name in ("type", "class"):
        method_name += "_"
    setattr(Block, method_name, method)

_ALL_COMPONENTS_BY_TYPE: dict[BlockType, tuple[typing.Type[Node]]] = {
    t: _DYNAMIC_COMPONENTS_BY_TYPE[t] + Block.__static_components__ for t in BlockType
}
