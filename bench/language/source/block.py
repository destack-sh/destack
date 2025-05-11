from datetime import datetime
from typing import TYPE_CHECKING, Optional, Union, override
from uuid import UUID

from bench.language.core import (
    BuiltinEnum,
    EnumType,
    IsModal,
    IsNamed,
    IsTemplatable,
    LocalNodeList,
    Node,
    NodeReference,
    NodeType,
    PackageNode,
    PageNode,
    StructType,
    Text,
    TextLine,
    TextLineIn,
    TextLineType,
    enum_,
    node_,
    p_internal,
    p_node_children,
    p_node_parent,
    p_regular,
    text_line,
)
from bench.pb2 import BlockData
from bench.utils.env import IS_DEV, IS_TEST
from bench.utils.fractional import INTEGER_ZERO

if TYPE_CHECKING:
    from bench.language import Page

# pyright: reportIncompatibleVariableOverride=false

_type = type


@enum_(EnumType.BLOCK_TYPE)
class BlockType(BuiltinEnum):
    # NOTE: see NodeType
    COMPUTER = 2100
    FILE = 2200
    PAGE = 5020
    CHOICE = 5100
    FLOW = 5220
    SERVICE = 5200
    DATABASE = 5300
    AGENT = 5640
    THREAD = 5510
    TASK = 6110

    # text
    # NOTE: text BlockTypes should align with :TextLineTypes
    PARAGRAPH = 10001
    # heading
    HEADING_1 = 10010
    HEADING_2 = 10011
    HEADING_3 = 10012
    HEADING_4 = 10013
    # callout
    CALLOUT = 10020
    QUOTE = 10021
    # list
    LIST_UNORDERED = 10030
    LIST_ORDERED = 10031
    # presentation
    DIVIDER = 10040
    # code
    CODE = 10060

    # layout?
    # ROW

    @property
    def is_node(self) -> bool:
        return self.id < 10000

    @property
    def is_text(self) -> bool:
        return self.id >= 10000


# copy NodeType properties to BlockType
for block_type in BlockType:
    if block_type.id < 10000:
        node_type = NodeType(block_type.id)
        block_type.title = node_type.title
        block_type.color = node_type.color
        block_type.icon = node_type.icon
        block_type.text = node_type.text
    else:
        text_line_type = TextLineType(block_type.id - 10000)
        block_type.title = text_line_type.title
        block_type.color = text_line_type.color
        block_type.icon = text_line_type.icon
        block_type.text = text_line_type.text


BLOCK_TYPES: tuple[BlockType, ...] = tuple(BlockType)
NODE_BLOCK_TYPES: tuple[BlockType, ...] = tuple(t for t in BLOCK_TYPES if t.id < 10000)
TEXT_BLOCK_TYPES: tuple[BlockType, ...] = tuple(t for t in BLOCK_TYPES if t.id >= 10000)

# cross-check BlockType
if IS_DEV or IS_TEST:
    # check that every NodeType is a real NodeType
    for block_type in NODE_BLOCK_TYPES:
        try:
            node_type = NodeType(block_type.id)
        except ValueError as e:
            raise ValueError(f"no NodeType with id {block_type.id}") from e
        if node_type.name != block_type.name:
            raise ValueError(f"BlockType {block_type.name} != NodeType {node_type.name}")

    # check that every TextLineType has a BlockType
    for text_line_type in TextLineType:
        block_type_id = text_line_type.id + 10000
        try:
            block_type = BlockType(block_type_id)
        except ValueError as e:
            raise ValueError(f"no BlockType with id {block_type_id}") from e
        if block_type.name != text_line_type.name:
            raise ValueError(f"TextLineType {text_line_type.name} != BlockType {block_type.name}")


@node_(NodeType.BLOCK)
class Block(IsTemplatable, IsModal, IsNamed, PackageNode[BlockData]):
    """
    A Block on a Page.
    NOTE :Architecture: Block should be IsView (or some subtrait)? Also Page, Flow, Action, ..?
    """

    parent: Union["Page", "Block", None] = p_node_parent(4, NodeType.PAGE, NodeType.BLOCK)

    # meta
    type: BlockType = p_regular(30, description="The type of block.")
    order_key: str = p_internal(33, default=INTEGER_ZERO)

    # content
    line: Optional["TextLine"] = p_regular(
        40, default=None, require=False, array=False, struct=StructType.TEXT_LINE
    )
    node: Optional["Node"] = p_regular(
        41, references="any", default=None, require=False, array=False, baseless=True
    )
    if TYPE_CHECKING:
        node_id: Optional[UUID] = None
        node_ck: Optional[UUID] = None
        node_ptr: Optional[NodeReference] = None

    blocks: LocalNodeList["Block"] = p_node_children(NodeType.BLOCK)

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

    @override
    def delete(self, _now: datetime | None = None):
        super().delete(_now=_now)
        # also delete linked Node (if any)
        if (
            isinstance(node := self.node, PageNode)
            and node.definition_id == self.id
            and not node.is_deleted
        ):
            node.delete(_now=_now)

    @override
    def restore(self, _now: datetime | None = None):
        super().restore(_now=_now)
        # also restore linked Node (if any)
        if (
            isinstance(node := self.node, PageNode)
            and node.definition_id == self.id
            and node.is_deleted
        ):
            node.restore(_now=_now)

    def get_node_as[T: PageNode](self, node_cls: _type[T]) -> T:
        """Get the Inline Node as a specific type (error if wrong type)."""
        if not isinstance((node := self.node), node_cls):
            raise TypeError(f"{self!r} has no {node_cls.__name__} (node={node!r})")
        return node  # type: ignore

    @staticmethod
    def wrap(node: PageNode) -> "Block":
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
        if isinstance(node, PageNode):
            if node.definition is None:
                node.definition = block
        return block  # type: ignore

    @staticmethod
    def from_text(text: Text) -> list["Block"]:
        """Create a list of Blocks for each line of Text."""
        blocks: list[Block] = []
        for line in text.lines:
            block_type = BlockType(line.type + 10_000)
            blocks.append(Block.new(block_type, line=line))
        return blocks
