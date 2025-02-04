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
    from bench.language import Node, Page, Text

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
        36, references="any", default=None, require=False, array=False
    )

    def __content_str__(self):
        return ""

    @property
    def is_type(self) -> bool:
        return self.type.is_type

    @staticmethod
    def new[BlockT: "Block" = "Block"](
        typ: BlockType | _type[BlockT] | NodeSubtypeStub[BlockT], name: str, **kwargs
    ) -> "BlockT":
        if isinstance(typ, type):
            typ = Block.__subtype_by_subclass__[typ]  # type: ignore
        elif isinstance(typ, NodeSubtypeStub):
            typ = cast(BlockType, typ._node_subtype)
        return Block(type=typ, name=name, **kwargs)  # type: ignore
