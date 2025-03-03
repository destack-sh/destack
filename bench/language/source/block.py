from datetime import datetime
from typing import TYPE_CHECKING, Optional, Union, cast, override
from uuid import UUID

from bench.language.core import (
    BuiltinEnum,
    EnumType,
    InlineSourceNode,
    LocalNodeList,
    NodeReference,
    NodeSubtypeStub,
    NodeType,
    SourceNode,
    StructType,
    TextLine,
    TextLineType,
    enum_,
    node_,
    p_internal,
    p_node_children,
    p_node_parent,
    p_regular,
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
    FILE = 1200, "File", "Any File", "fas fa-file"
    PAGE = 5020, "Page", "Page to write", "fas fa-page"
    CHOICE = 5030, "Choice", "Choice of options", "fas fa-check-circle"
    CLASS = 5031, "Class", "Class of Fields", "fas fa-boxes-stacked"
    TAG = 5040, "Tag", "Tag to tag other Nodes", "fas fa-tag"
    FLOW = 5050, "Flow", "Flow to run", "fas fa-play"
    VIEW = 5080, "View", "Graphical Interface", "fas fa-eye"
    DATABASE = 5090, "Database", "Database to store", "fas fa-database"
    CHANNEL = 5100, "Channel", "Channel to send", "fas fa-envelope"
    ROLE = 5110, "Role", "Role to assign", "fas fa-user-shield"

    # text
    # NOTE: text BlockTypes should align with :TextLineTypes
    PARAGRAPH = 10001, "Paragraph", "Line of rich Text", "fas fa-align-left"
    # heading
    HEADING_1 = 10010, "Heading 1", "Very big heading", "fas fa-heading"
    HEADING_2 = 10011, "Heading 2", "Big heading", "fas fa-heading"
    HEADING_3 = 10012, "Heading 3", "Medium heading", "fas fa-heading"
    HEADING_4 = 10013, "Heading 4", "Small heading", "fas fa-heading"
    # callout
    CALLOUT = 10020, "Callout", "Callout", "fas fa-circle-exclamation"
    QUOTE = 10021, "Quote", "Quote", "fas fa-quote-left"
    # list
    LIST_UNORDERED = 10030, "Unorderd list", "Unorderd list", "fas fa-list-ul"
    LIST_ORDERED = 10031, "Numbered list", "Numbered list", "fas fa-list-ol"
    # presentation
    DIVIDER = 10040, "Horizontal line", "Horizontal line", "fas fa-horizontal-rule"
    # table
    TABLE = 10050, "Table", "Table of data", "fas fa-table"
    TABLE_ROW = 10051, "Table row", "Row of a table", "fas fa-table-rows"
    # code
    CODE = 10060, "Code block", "Code block", "fas fa-code"

    # layout?
    # ROW


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


@node_(NodeType.BLOCK, has_subtypes=True)
class Block(SourceNode[BlockData]):
    """A Block on a Page."""

    parent: Union["Page", "Block", None] = p_node_parent(4, NodeType.PAGE, NodeType.BLOCK)

    # meta
    type: BlockType = p_regular(30, description="The type of block.")
    order_key: str = p_internal(33, default=INTEGER_ZERO)

    # content
    line: Optional["TextLine"] = p_regular(
        40, default=None, require=False, array=False, struct=StructType.TEXT_LINE
    )
    node: Optional["InlineSourceNode"] = p_regular(
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
        elif (text := self.line) is not None:
            return text.__content_str__()
        else:
            return ""

    @property
    def container(self) -> "Page | None":
        """The container of this Block."""
        return self.page

    @property
    def name(self) -> str | None:
        """The name of the delegate (if any)."""
        if (node := self.node) is not None and "name" in node.__properties__:
            return node.name
        return None

    @property
    def code_name(self) -> str | None:
        """The code name of the delegate (if any)."""
        if (node := self.node) is not None and "name" in node.__properties__:
            return node.code_name
        return None

    @override
    def delete(self, _now: datetime | None = None):
        super().delete(_now=_now)
        # also delete linked Node (if any)
        if (
            (node := self.node) is not None
            and node.definition_id == self.id
            and not node.is_deleted
        ):
            node.delete(_now=_now)

    @override
    def restore(self, _now: datetime | None = None):
        super().restore(_now=_now)
        # also restore linked Node (if any)
        if (node := self.node) is not None and node.definition_id == self.id and node.is_deleted:
            node.restore(_now=_now)

    def node_as[T: InlineSourceNode](self, node_cls: _type[T]) -> T:
        if not isinstance((node := self.node), node_cls):
            raise TypeError(f"{self!r} has no {node_cls.__name__} (node={node!r})")
        return node  # type: ignore

    @staticmethod
    def wrap(node: InlineSourceNode) -> "Block":
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
            if node.definition is None:
                node.definition = block
        return block  # type: ignore
