from typing import TYPE_CHECKING, Optional, Union

from fastuuid import UUID

from bench.language.core import (
    BuiltinEnum,
    EnumType,
    HasEnvironment,
    IsArchivable,
    IsBlockable,
    IsDeletable,
    IsInPackage,
    IsOrdered,
    IsTemplatable,
    Node,
    NodeReference,
    NodeType,
    Text,
    TextLine,
    TextLineIn,
    TextLineType,
    enum_,
    node_,
    property_,
    property_parent_,
    text_line,
)
from bench.pb2 import BlockData

if TYPE_CHECKING:
    from bench.language import IsView, Page

# pyright: reportIncompatibleVariableOverride=false

_type = type


@enum_(EnumType.BLOCK_TYPE)
class BlockType(BuiltinEnum):
    # text
    # NOTE: text BlockTypes should align with :TextLineTypes
    PARAGRAPH = 1
    HEADING_1 = 10
    HEADING_2 = 11
    HEADING_3 = 12
    HEADING_4 = 13
    CALLOUT = 20
    QUOTE = 21
    LIST_UNORDERED = 30
    LIST_ORDERED = 31
    DIVIDER = 40
    CODE = 50
    NODE = 1000


# copy NodeType properties to BlockType
for block_type in BlockType:
    text_line_type = TextLineType(block_type.id)
    block_type.title = text_line_type.title
    block_type.icon = text_line_type.icon
    block_type.text = text_line_type.text


BLOCK_TYPES: tuple[BlockType, ...] = tuple(BlockType)


@node_(NodeType.BLOCK)
class Block(
    IsTemplatable,
    HasEnvironment,
    IsOrdered,
    IsInPackage,
    IsArchivable,
    IsDeletable,
    Node[BlockData],
):
    """
    A Block on a Page.
    """

    parent: Union["Page", "Block", None] = property_parent_()

    # meta
    type: BlockType = property_(30, description="The type of block.", is_repr=True)

    # content
    # NOTE: maybe there should be a general mechanism for tying Nodes like Blocks? :NodeTying
    line: Optional["TextLine"] = property_(40)
    node: Optional["Node"] = property_(41, is_repr=True)
    view: Optional["IsView"] = property_(42)
    # size?
    if TYPE_CHECKING:
        node_id: Optional[UUID] = None
        node_ptr: Optional[NodeReference] = None

    @property
    def page(self) -> "Page | None":
        """Gets the containing ancestor Page (if any)"""
        from bench.language import Page

        parent = self.parent
        while parent is not None:
            if isinstance(parent, Page):
                return parent
            parent = parent.parent
        return None

    def get_node_as[T: IsBlockable](self, node_cls: _type[T]) -> T:
        """Get the Inline Node as a specific type (error if wrong type)."""
        if not isinstance((node := self.node), node_cls):
            raise TypeError(f"{self!r} has no {node_cls.__name__} (node={node!r})")
        return node  # type: ignore

    @staticmethod
    def wrap(node: IsBlockable) -> "Block":
        """Wrap a Node as a Block."""
        try:
            block_type = BlockType(node.metatype)
        except ValueError as exc:
            raise TypeError(f"cannot wrap {node!r} as a Block") from exc
        block = Block(type=block_type, node=node)
        if node.block_id is None:
            node.block = block
        return block

    @staticmethod
    def heading(line: TextLineIn, level: int = 1) -> "Block":
        """Create a heading Block."""
        return Block(type=BlockType.HEADING_1, line=text_line(line))

    @staticmethod
    def from_text(text: Text) -> list["Block"]:
        """Create a list of Blocks for each line of Text."""
        blocks: list[Block] = []
        for line in text.lines:
            block_type = BlockType(line.type)
            blocks.append(Block(type=block_type, line=line))
        return blocks
