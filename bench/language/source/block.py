from typing import TYPE_CHECKING, Optional, Union, cast

from bench.language.core import (
    BlockType,
    NodeSubtypeStub,
    NodeType,
    SourceNode,
    StructType,
    node_,
    p_internal,
    p_node_parent,
    p_regular,
    p_system,
)
from bench.pb2 import BlockData
from bench.utils.fractional import INTEGER_ZERO

if TYPE_CHECKING:
    from bench.language import Choice, Class, Database, Flow, Identity, Node, Page, Role, Text, View

# pyright: reportIncompatibleVariableOverride=false

_type = type


@node_(NodeType.BLOCK, passthrough_get=("fields",), has_subtypes=True)
class Block(SourceNode[BlockData]):
    """A Block on a Page."""

    parent: Union["Page", None] = p_node_parent(4, NodeType.PAGE)

    # content
    type: BlockType = p_system(30, description="The type of block. Cannot be changed.")
    order_key: str = p_internal(33, default=INTEGER_ZERO)
    text: Optional["Text"] = p_regular(
        35, default=None, require=False, array=False, struct=StructType.TEXT
    )
    node: Optional["Node"] = p_regular(
        36, references="any", default=None, require=False, array=False, baseless=True
    )

    def __content_str__(self):
        return ""

    @property
    def is_type(self) -> bool:
        return self.type.is_type

    #
    # Nested accessors
    #

    @property
    def flow(self) -> "Flow":
        from bench.language import Flow

        if not isinstance((node := self.node), Flow):
            raise TypeError(f"{self!r} has no Flow")
        return node

    @property
    def database(self) -> "Database":
        from bench.language import Database

        if not isinstance((node := self.node), Database):
            raise TypeError(f"{self!r} has no Database")
        return node

    @property
    def choice(self) -> "Choice":
        from bench.language import Choice

        if not isinstance((node := self.node), Choice):
            raise TypeError(f"{self!r} has no Choice")
        return node

    @property
    def class_(self) -> "Class":
        from bench.language import Class

        if not isinstance((node := self.node), Class):
            raise TypeError(f"{self!r} has no Class")
        return node

    @property
    def view(self) -> "View":
        from bench.language import View

        if not isinstance((node := self.node), View):
            raise TypeError(f"{self!r} has no View")
        return node

    @property
    def role(self) -> "Role":
        from bench.language import Role

        if not isinstance((node := self.node), Role):
            raise TypeError(f"{self!r} has no Role")
        return node

    @property
    def identity(self) -> "Identity":
        from bench.language import Identity

        if not isinstance((node := self.node), Identity):
            raise TypeError(f"{self!r} has no Identity")
        return node

    @staticmethod
    def new[BlockT: "Block" = "Block"](
        typ: BlockType | _type[BlockT] | NodeSubtypeStub[BlockT], name: str, **kwargs
    ) -> "BlockT":
        if isinstance(typ, type):
            typ = Block.__subtype_by_subclass__[typ]  # type: ignore
        elif isinstance(typ, NodeSubtypeStub):
            typ = cast(BlockType, typ._node_subtype)
        return Block(type=typ, name=name, **kwargs)  # type: ignore
