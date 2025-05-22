from typing import TYPE_CHECKING, Optional, Union, override

from fastuuid import UUID

from bench.language.core import (
    BuiltinEnum,
    EnumType,
    IsArchivable,
    IsBlockable,
    IsDeletable,
    IsInPackage,
    IsModal,
    IsNamed,
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
    p_node_parent,
    p_regular,
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
    IsModal,
    IsNamed,
    IsOrdered,
    IsInPackage,
    IsArchivable,
    IsDeletable,
    Node[BlockData],
):
    """
    A Block on a Page.
    """

    parent: Union["Page", "Block", None] = p_node_parent()

    # meta
    type: BlockType = p_regular(30, description="The type of block.")

    # content
    # NOTE: maybe there should be a general mechanism for tying Nodes like Blocks? :NodeTying
    line: Optional["TextLine"] = p_regular(40)
    node: Optional["Node"] = p_regular(41, node_exclude=("base_id",))
    view: Optional["IsView"] = p_regular(42, node_exclude=("base_id",))
    # size?
    if TYPE_CHECKING:
        node_id: Optional[UUID] = None
        node_ck: Optional[UUID] = None
        node_ptr: Optional[NodeReference] = None

    def __content_str__(self):
        if self.node_ptr is not None and (node := self.node) is not None:
            return node.__content_str__()
        elif (line := self.line) is not None:
            return line.__content_str__()
        else:
            return ""

    @property
    @override
    def _path_key(self) -> str:
        if (
            (node := self.node) is not None
            and node.parent_id == self.parent_id
            and (node_ident := node._ident) is not None
        ):
            return f"Block[{node_ident}]"

        return f"{self.metatype.bench_name}[id={self.id}]"

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

    @property
    def name(self) -> str | None:
        """The name of the delegate (if any)."""
        if isinstance(node := self.node, IsNamed):
            return node.name
        return None

    @property
    def code_name(self) -> str | None:
        """The code name of the delegate (if any)."""
        if isinstance(node := self.node, IsNamed):
            return node.code_name
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
        block = Block.new(block_type, node=node)
        if node.definition_id is None:
            node.definition = block
        return block

    @staticmethod
    def heading(line: TextLineIn, level: int = 1) -> "Block":
        """Create a heading Block."""
        return Block.new(BlockType.HEADING_1, line=text_line(line))

    @staticmethod
    def new[BlockT: "Block" = "Block"](
        typ: BlockType | _type[BlockT],
        node: Node | None = None,
        **kwargs,
    ) -> "BlockT":
        # unravel subtype
        if isinstance(typ, type):
            typ = Block.__subtype_by_subclass__[typ]  # type: ignore
        # make
        block = Block(type=typ, node=node, **kwargs)  # type: ignore
        # set the node definition for inline source nodes
        if isinstance(node, IsBlockable):
            if node.definition is None:
                node.definition = block
        return block  # type: ignore

    @staticmethod
    def from_text(text: Text) -> list["Block"]:
        """Create a list of Blocks for each line of Text."""
        blocks: list[Block] = []
        for line in text.lines:
            block_type = BlockType(line.type)
            blocks.append(Block.new(block_type, line=line))
        return blocks
