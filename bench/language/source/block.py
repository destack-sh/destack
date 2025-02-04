from typing import TYPE_CHECKING, Optional, Union, cast

from bench.language.core import (
    BlockType,
    InlineSourceNode,
    NodeSubtypeStub,
    NodeType,
    SourceNode,
    StructType,
    Text,
    node_,
    p_internal,
    p_node_parent,
    p_regular,
    p_system,
)
from bench.pb2 import BlockData
from bench.utils.fractional import INTEGER_ZERO

if TYPE_CHECKING:
    from bench.language import Page

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
    node: Optional["InlineSourceNode"] = p_regular(
        36, references="any", default=None, require=False, array=False, baseless=True
    )

    def __content_str__(self):
        return ""

    def node_as[T: InlineSourceNode](self, node_cls: _type[T]) -> T:
        if not isinstance((node := self.node), node_cls):
            raise TypeError(f"{self!r} has no {node_cls!r}")
        return node  # type: ignore

    @staticmethod
    def new[BlockT: "Block" = "Block"](
        typ: BlockType | _type[BlockT] | NodeSubtypeStub[BlockT],
        node: InlineSourceNode | None = None,
        **kwargs,
    ) -> "BlockT":
        # unravel subtype
        if isinstance(typ, type):
            typ = Block.__subtype_by_subclass__[typ]  # type: ignore
        elif isinstance(typ, NodeSubtypeStub):
            typ = cast(BlockType, typ._node_subtype)
        # make
        block = Block(type=typ, node=node, **kwargs)  # type: ignore
        # set the node definition for inline source nodes
        if node is not None:
            if not isinstance(node, InlineSourceNode):
                raise TypeError(f"expected InlineSourceNode, got {node!r}")
            if node.block is None:
                node.block = block
        return block  # type: ignore
