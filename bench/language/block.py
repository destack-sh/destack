from typing import TYPE_CHECKING, Any, Collection, Optional, Union, cast

from bench.language.const import BlockType, NodeType, StructType, TypeKind, Visibility
from bench.language.node import Node, NodeList, node
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
from bench.language.validation import NAME_CONSTRAINT, ValidationHandler
from bench.language.value import HasValues
from bench.proto.wire import BlockData
from bench.utils.casing import IdentifierType
from bench.utils.fractional import INTEGER_ZERO

if TYPE_CHECKING:
    from bench.language import (
        Badge,
        Code,
        Field,
        Icon,
        Notice,
        Package,
        Policy,
        Property,
        Step,
        Text,
        Trigger,
        TypeInfo,
    )

# pyright: reportIncompatibleVariableOverride=false

IDENTIFIER_TYPE_BY_BLOCK_TYPE: dict[BlockType, IdentifierType] = {
    BlockType.ALIAS: IdentifierType.VARIABLE,
    BlockType.PAGE: IdentifierType.VARIABLE,
    BlockType.MODULE: IdentifierType.VARIABLE,
    BlockType.BLANK: IdentifierType.VARIABLE,
    BlockType.CLASS: IdentifierType.TYPE,
    BlockType.SIGNAL: IdentifierType.TYPE,
    BlockType.CHOICE: IdentifierType.TYPE,
    BlockType.PROTOCOL: IdentifierType.TYPE,
    BlockType.TEXT: IdentifierType.FUNCTION,
    BlockType.CODE: IdentifierType.FUNCTION,
    BlockType.SCRIPT: IdentifierType.FUNCTION,
    BlockType.FLOW: IdentifierType.FUNCTION,
    BlockType.VARIABLE: IdentifierType.VARIABLE,
    BlockType.DATABASE: IdentifierType.TYPE,
    BlockType.QUERY: IdentifierType.VARIABLE,
    BlockType.SCREEN: IdentifierType.TYPE,
    BlockType.ROLE: IdentifierType.TYPE,
    BlockType.IDENTITY: IdentifierType.TYPE,
}
assert len(IDENTIFIER_TYPE_BY_BLOCK_TYPE) == len(BlockType)


# TODO :UX: auto-generate node names in code just like in the UI (if unset -> block7, etc.)
#  (Maybe postpone name validation if detached so we can leave it unset?,
#   auto-naming currently only works in NodeList where we know the siblings).
#  see :AutoNaming


@node(NodeType.BLOCK, passthrough="value")
class Block(Node[BlockData], HasValues):
    """A building block containing logic, types, UI, data, AI, - any Bench program source."""

    parent: Union["Block", "Package"] = p_node_parent(4, NodeType.BLOCK, NodeType.PACKAGE)

    # core
    type: BlockType = p_internal(30)
    name: str = p_regular(32, constraint=NAME_CONSTRAINT)
    order_key: str = p_internal(33, default=INTEGER_ZERO)
    policies: list["Policy"] = p_regular(34, array=True, struct=StructType.POLICY)
    bases: list["Block"] = p_regular(35, require=False, array=True, references=NodeType.BLOCK)
    builtin_base: Optional["TypeInfo"] = p_regular(36, default=None, struct=StructType.TYPE_INFO)
    text: Optional["Text"] = p_regular(
        37, default=None, require=False, array=False, struct=StructType.TEXT
    )
    icon: Optional["Icon"] = p_regular(
        38, default=None, require=False, array=False, struct=StructType.ICON
    )
    visibility: Optional[Visibility] = p_regular(39, default=None, require=False)
    value_packed: Any = p_value_packed(40)
    secret_value_packed: Any | None = p_secret_value_packed(41)
    value: Any = p_value_runtime(40, 41, type=lambda self: cast("Block", self).as_type)
    code: Optional["Code"] = p_regular(
        42, default=None, require=False, array=False, struct=StructType.CODE
    )
    delegated_policies: list["Policy"] = p_regular(43, array=True, struct=StructType.POLICY)

    # flags
    is_builtin: bool = p_system(60, default=False)  # intrinsic provided by the system
    is_page: bool = p_regular(61, default=False)  # on its own page
    is_protocol: bool = p_regular(62, default=False)  # defines a protocol
    is_template: bool = p_regular(63, default=False)  # mark as template
    is_materialized: bool = p_regular(64, default=False)  # database should be materialized
    is_paused: bool = p_regular(65, default=False)  # all blocks <= this are paused
    # is_method? (bound to instances of parent)
    # is_unique? (by name in parent module, like in Godot)
    # is_frozen? (read-only in instances of template)

    blocks: NodeList["Block"] = p_node_child(NodeType.BLOCK)
    badges: NodeList["Badge"] = p_node_child(NodeType.BADGE)
    fields: NodeList["Field"] = p_node_child(NodeType.FIELD)
    steps: NodeList["Step"] = p_node_child(NodeType.STEP)
    triggers: NodeList["Trigger"] = p_node_child(NodeType.TRIGGER)
    notices: NodeList["Notice"] = p_node_child(NodeType.NOTICE)

    def _validate_component(
        self, properties: Collection["Property"], invalid: "ValidationHandler"
    ) -> None:
        if self.type == BlockType.PAGE and not self.is_page:
            invalid(self, "type=Page must have is_page=True", (Block.type, Block.is_page))
        elif self.type == BlockType.PROTOCOL and not self.is_protocol:
            invalid(
                self, "type=Protocol must have is_protocol=True", (Block.type, Block.is_protocol)
            )

    @property
    def is_type(self) -> bool:
        return self.type.is_type

    @property
    def is_runnable(self) -> bool:
        return self.type.is_runnable

    def __content_str__(self):
        return ""  # implemented by dynamic components

    def __repr__(self):  # type: ignore we want to override the default repr
        return f"<{self.type.bench_name}Block {self}>"

    def __call__(self, *args, **kwargs) -> Any:
        if self.type.is_runnable:
            raise NotImplementedError
        else:
            typ = self.to_type()
            if typ is None:
                raise ValueError(f"{self!r} does not have an implicit type")
            return typ(*args, **kwargs)

    def to_type(self) -> "TypeInfo | None":
        """
        Get the default implicit type for this Block (if any).
        TODO :Performance: cache Block.to_type (in interp?)
        """
        from bench.language.field import TypeInfo

        if self.type == BlockType.CLASS:
            # NOTE: we turn Class Blocks into Object Types here so we can instantiate them immediately
            #  but for storage we would have to make this an Alias that resolve to an Object Type..
            typ = TypeInfo(kind=TypeKind.OBJECT, base_type=self)
        elif self.type == BlockType.CHOICE:
            typ = TypeInfo(kind=TypeKind.BASED_NODE, base_type=self, bench_type=NodeType.FIELD)
        elif self.type == BlockType.SIGNAL:
            typ = TypeInfo(kind=TypeKind.BASED_NODE, base_type=self, bench_type=NodeType.SIGNAL)
        elif self.type == BlockType.VARIABLE:
            return self.builtin_base  # nothing to resolve
        elif self.type == BlockType.DATABASE:
            typ = TypeInfo(kind=TypeKind.BASED_NODE, base_type=self, bench_type=NodeType.RECORD)
        elif self.type.is_runnable:
            typ = TypeInfo(kind=TypeKind.BASED_NODE, base_type=self, bench_type=NodeType.RUN)
        else:
            return None
        typ._do_resolve_to(typ)
        return typ

    @property
    def as_type(self) -> "TypeInfo":
        typ = self.to_type()
        assert typ, f"{self!r} does not have an implicit type"
        return typ

    def morph(self, to_type: BlockType):
        raise NotImplementedError(f"{self!r} does not support morphing yet")

    @property
    def identifier_type(self) -> IdentifierType:
        return IDENTIFIER_TYPE_BY_BLOCK_TYPE[self.type]
