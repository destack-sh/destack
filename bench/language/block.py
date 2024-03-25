import typing
from dataclasses import dataclass
from datetime import datetime
from typing import TYPE_CHECKING, Any, Optional, Union

from bench.language.const import BlockType, NodeType, NodeVisibility, StructType
from bench.language.database import HasDatabase
from bench.language.node import Node, NodeList, NRel, _Passthrough, node, node_component
from bench.language.property import (
    p_internal,
    p_node_child,
    p_node_parent,
    p_regular,
    p_secret_value_packed,
    p_system,
    p_value_packed,
    p_value_runtime,
)
from bench.language.session import HasRun
from bench.language.validation import validate_name
from bench.language.value import HasValues
from bench.utils.casing import IdentifierType
from bench.utils.dt import utcnow_with_tz
from bench.utils.fractional import INTEGER_ZERO
from bench.utils.func import dict_minus

if TYPE_CHECKING:
    from bench.language import (
        Badge,
        Code,
        Field,
        Icon,
        Package,
        Policy,
        Text,
        TypeInfo,
    )


@node_component
class IsInstantiable(Node):
    def _call_inner(self, *args, **kwargs) -> Any:
        raise NotImplementedError("create value :Incomplete")


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
_block(BlockType.PAGE, (), IdentT.VARIABLE)
_block(BlockType.TEXT, (), IdentT.VARIABLE)
_block(BlockType.BLANK, (), IdentT.VARIABLE)
_block(BlockType.ALIAS, (IsInstantiable,), IdentT.VARIABLE)
_block(BlockType.CLASS, (IsInstantiable,), IdentT.TYPE)
_block(BlockType.SIGNAL, (IsInstantiable,), IdentT.TYPE)
_block(BlockType.CHOICE, (IsInstantiable,), IdentT.TYPE)
_block(BlockType.PROTOCOL, (), IdentT.TYPE)
_block(BlockType.NATURAL_ROUTINE, (HasRun,), IdentT.FUNCTION)
_block(BlockType.CODE_ROUTINE, (HasRun,), IdentT.FUNCTION)
_block(BlockType.SCRIPT, (HasRun,), IdentT.FUNCTION)
_block(BlockType.FLOW, (HasRun,), IdentT.FUNCTION)
_block(BlockType.SINGLE_VARIABLE, (), IdentT.VARIABLE)
_block(BlockType.MULTI_VARIABLE, (), IdentT.VARIABLE)
_block(BlockType.DATABASE, (HasDatabase,), IdentT.TYPE)
_block(BlockType.QUERY, (), IdentT.VARIABLE)
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
class Block(Node, HasValues):
    """A building block containing logic, types, UI, data, AI, - any Bench program source."""

    parent: Union["Block", "Package"] = p_node_parent(4, NodeType.BLOCK, NodeType.PACKAGE)
    blocks: NodeList["Block"] = p_node_child(
        NodeType.BLOCK, NRel.NAMED | NRel.SCOPED | NRel.ORDERED
    )
    badges: NodeList["Badge"] = p_node_child(NodeType.BADGE)
    fields: NodeList["Field"] = p_node_child(
        NodeType.FIELD, NRel.NAMED | NRel.SCOPED | NRel.ORDERED
    )

    # core
    type: BlockType = p_internal(30, default=BlockType.BLANK)
    # custom type..?
    name: str | None = p_regular(32, default=None, validate=validate_name)
    order_key: str = p_internal(33, default=INTEGER_ZERO)
    visibility: NodeVisibility = p_regular(34, default=NodeVisibility.ALL)
    policies: list["Policy"] = p_regular(35, array=True, struct=StructType.POLICY)
    bases: list["Block"] = p_regular(
        36, default=None, require=False, array=True, references=NodeType.BLOCK
    )
    builtin_base: Optional["TypeInfo"] = p_regular(37, default=None, struct=StructType.TYPE_INFO)

    dynamic_key: str | None = p_internal(40, default=None)
    text: Optional["Text"] = p_regular(
        41, default=None, require=False, array=False, struct=StructType.TEXT
    )
    value_packed: Any = p_value_packed(42)
    secret_value_packed: Any | None = p_secret_value_packed(43)
    value = p_value_runtime(42, 43)
    code: Optional["Code"] = p_regular(
        44, default=None, require=False, array=False, struct=StructType.CODE
    )
    icon: Optional["Icon"] = p_regular(
        45, default=None, require=False, array=False, struct=StructType.ICON
    )
    reference: Optional["Block"] = p_regular(
        46, require=False, array=False, references=NodeType.BLOCK
    )
    delegated_policies: list["Policy"] = p_regular(47, array=True, struct=StructType.POLICY)

    # flags
    is_page: bool = p_regular(60, default=False)  # on its own page
    is_module: bool = p_regular(61, default=False)  # has a module scope
    is_unique: bool = p_regular(62, default=False)  # unique by name in parent module
    is_intrinsic: bool = p_system(63, default=False)  # provided by the system
    is_protocol: bool = p_regular(64, default=False)  # has a protocol
    is_method: bool = p_regular(65, default=False)  # bound to instances of parent (with 'self')
    # paused_at acts like a flag (see setter/getter below)
    paused_at: datetime | None = p_internal(66, default=None)  # triggers <=block are paused

    # is_frozen? (read-only in instances of template)

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

    # the others are defined after the class

    @staticmethod
    def to_python(
        node, props: dict, for_parent: Union["Block", "Package", None] = None
    ) -> tuple[str, dict, dict]:
        init_name = f"{node.type.lower()}Block"
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
        return self.type.name

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
    def is_paused(self) -> bool:
        return (
            self.paused_at is not None
            or self.parent_type == NodeType.BLOCK
            and self.parent.is_paused
        )

    @is_paused.setter
    def is_paused(self, is_paused: bool):
        if is_paused:
            if self.paused_at is None:
                pausing_ancestor = self.parent
                while pausing_ancestor is not None and pausing_ancestor.paused_at is None:
                    pausing_ancestor = pausing_ancestor.parent
                raise ValueError(f"{self!r} is not paused but its ancestor {pausing_ancestor!r} is")
            self.paused_at = None
        else:
            self.paused_at = utcnow_with_tz()

    def __content_str__(self):
        return ""  # implemented by dynamic components

    def __repr__(self):  # noqa: we want to override the default repr
        return f"<{self.type.bench_name}Block {self}>"

    def _init_inner(self) -> None:
        # add runtime properties from dynamic components
        for component in self._dynamic_components:
            for prop in component.__properties__.values():
                if not prop.is_computed and prop.is_ephemeral and prop.name not in self.__dict__:
                    setattr(self, prop.name, prop.new())

    def morph(self, to_type: BlockType):
        raise NotImplementedError(f"{self!r} does not support morphing yet")

    @property
    def identifier_type(self) -> Optional[IdentifierType]:
        return _IDENTIFIER_BY_TYPE[self.type]


# Block.<type> convenience constructors
for _t in BlockType:
    method = staticmethod(
        lambda name=None, _type=_t, *args, **kwargs: Block.new(_type, name=name, *args, **kwargs)
    )
    method_name = _t.name.lower()
    if method_name in Block.__properties__:
        method_name += "_"
    setattr(Block, method_name, method)

_ALL_COMPONENTS_BY_TYPE: dict[BlockType, tuple[typing.Type[Node]]] = {
    t: _DYNAMIC_COMPONENTS_BY_TYPE[t] + Block.__static_components__ for t in BlockType
}
