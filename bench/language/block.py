import typing
from dataclasses import dataclass
from datetime import datetime
from typing import TYPE_CHECKING, Any, Collection, Optional, Union

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
from bench.language.validation import ValidationHandler, enum_validator, validate_name
from bench.language.value import HasValues
from bench.proto.wire import BlockData
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
        Property,
        Text,
        TypeInfo,
    )
# pyright: reportIncompatibleVariableOverride=false


@node_component()
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

_describe_block = _BlockTypeDescriptor
_describe_block(BlockType.ALIAS, (IsInstantiable,), IdentT.VARIABLE)
_describe_block(BlockType.PAGE, (), IdentT.VARIABLE)
_describe_block(BlockType.MODULE, (), IdentT.VARIABLE)
_describe_block(BlockType.BLANK, (), IdentT.VARIABLE)
_describe_block(BlockType.CLASS, (IsInstantiable,), IdentT.TYPE)
_describe_block(BlockType.SIGNAL, (IsInstantiable,), IdentT.TYPE)
_describe_block(BlockType.CHOICE, (IsInstantiable,), IdentT.TYPE)
_describe_block(BlockType.PROTOCOL, (), IdentT.TYPE)
_describe_block(BlockType.TEXT, (HasRun,), IdentT.FUNCTION)
_describe_block(BlockType.CODE, (HasRun,), IdentT.FUNCTION)
_describe_block(BlockType.SCRIPT, (HasRun,), IdentT.FUNCTION)
_describe_block(BlockType.FLOW, (HasRun,), IdentT.FUNCTION)
_describe_block(BlockType.VARIABLE, (), IdentT.VARIABLE)
_describe_block(BlockType.DATABASE, (HasDatabase,), IdentT.TYPE)
_describe_block(BlockType.QUERY, (), IdentT.VARIABLE)
_describe_block(BlockType.SCREEN, (), IdentT.TYPE)
_describe_block(BlockType.ROLE, (), IdentT.TYPE)
_describe_block(BlockType.IDENTITY, (), IdentT.TYPE)

assert len(_BLOCK_DESCRIPTORS) == len(BlockType), "missing block descriptors"
del _describe_block

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
class Block(Node[BlockData], HasValues):
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
    type: BlockType = p_internal(30, validate=enum_validator(BlockType))
    # custom type..?
    # TODO :UX: auto-generate node names in code just like in the UI (if unset -> block7, etc.)
    #  (Maybe postpone name validation if detached so we can leave it unset?,
    #   auto-naming currently only works in NodeList where we know the siblings).
    #  see :AutoNaming
    name: str = p_regular(32, validate=validate_name)
    order_key: str = p_internal(33, default=INTEGER_ZERO)
    policies: list["Policy"] = p_regular(34, array=True, struct=StructType.POLICY)
    bases: list["Block"] = p_regular(
        35, default=None, require=False, array=True, references=NodeType.BLOCK
    )
    builtin_base: Optional["TypeInfo"] = p_regular(36, default=None, struct=StructType.TYPE_INFO)
    text: Optional["Text"] = p_regular(
        37, default=None, require=False, array=False, struct=StructType.TEXT
    )
    icon: Optional["Icon"] = p_regular(
        38, default=None, require=False, array=False, struct=StructType.ICON
    )
    visibility: Optional[NodeVisibility] = p_regular(
        39, require=False, default=NodeVisibility.PUBLIC
    )

    value_packed: Any = p_value_packed(40)
    secret_value_packed: Any | None = p_secret_value_packed(41)
    value = p_value_runtime(40, 41)
    code: Optional["Code"] = p_regular(
        42, default=None, require=False, array=False, struct=StructType.CODE
    )
    reference: Optional["Block"] = p_regular(
        43, require=False, array=False, references=NodeType.BLOCK
    )
    delegated_policies: list["Policy"] = p_regular(44, array=True, struct=StructType.POLICY)

    # flags
    is_builtin: bool = p_system(60, default=False)  # intrinsic provided by the system
    is_page: bool = p_regular(61, default=False)  # on its own page
    is_protocol: bool = p_regular(62, default=False)  # defines a protocol
    is_template: bool = p_regular(63, default=False)  # mark as template
    # paused_at acts like a flag (see setter/getter below)
    paused_at: datetime | None = p_internal(66, default=None)  # triggers <=block are paused

    # is_method? (bound to instances of parent)
    # is_unique? (by name in parent module)
    # is_frozen? (read-only in instances of template)

    @staticmethod
    def new(
        type: BlockType,
        name: str,
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
    def _components(self) -> tuple[typing.Type[Node], ...]:
        return _ALL_COMPONENTS_BY_TYPE[self.type]

    @property
    def _dynamic_components(self) -> tuple[typing.Type[Node], ...]:
        return _DYNAMIC_COMPONENTS_BY_TYPE[self.type]

    @property
    def _instance_cache_key(self) -> str:
        return self.type.name

    def _validate_inner(
        self, properties: Collection["Property"], on_invalid: "ValidationHandler"
    ) -> None:
        if self.type == BlockType.PAGE and not self.is_page:
            on_invalid(self, "type=Page must have is_page=True", (Block.type, Block.is_page), None)
        elif self.type == BlockType.PROTOCOL and not self.is_protocol:
            on_invalid(
                self,
                "type=Protocol must have is_protocol=True",
                (Block.type, Block.is_protocol),
                None,
            )

    @property
    def is_type(self) -> bool:
        return self.type.is_type

    @property
    def is_runnable(self) -> bool:
        return self.type.is_runnable

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
                while pausing_ancestor is not None and not getattr(
                    pausing_ancestor, "paused_at", None
                ):
                    pausing_ancestor = pausing_ancestor.parent
                raise ValueError(f"{self!r} is not paused but its ancestor {pausing_ancestor!r} is")
            self.paused_at = None
        else:
            self.paused_at = utcnow_with_tz()

    def __content_str__(self):
        return ""  # implemented by dynamic components

    def __repr__(self):  # type: ignore we want to override the default repr
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
    def identifier_type(self) -> IdentifierType:
        return _IDENTIFIER_BY_TYPE[self.type]

    @property
    def storage_key(self) -> str | None:
        if self.builtin_base is None:
            return None
        else:
            return f"{self.sk}-{self.builtin_base.identity_key}"


_ALL_COMPONENTS_BY_TYPE: dict[BlockType, tuple[typing.Type[Node], ...]] = {
    t: _DYNAMIC_COMPONENTS_BY_TYPE[t] + Block.__static_components__ for t in BlockType
}
