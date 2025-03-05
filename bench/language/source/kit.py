import typing
from typing import Optional, Union

from bench.language.core import (
    INLINE_SOURCE_NODE_TYPES,
    NAME_CONSTRAINT,
    InlineSourceNode,
    LocalNodeList,
    NodeType,
    StructType,
    node_,
    p_internal,
    p_node_children,
    p_node_parent,
    p_regular,
)
from bench.pb2 import KitData
from bench.utils.fractional import INTEGER_ZERO

if typing.TYPE_CHECKING:
    from bench.language import Action, Icon, Package, Page, Text


# pyright: reportIncompatibleVariableOverride=false, reportIncompatibleMethodOverride=false


@node_(NodeType.KIT)
class Kit(InlineSourceNode[KitData]):
    """
    A Kit of Actions for a SourceNode.
    """

    parent: Union["Page", "Package", None] = p_node_parent(4, NodeType.PAGE, NodeType.PACKAGE)
    name: str = p_regular(31, constraint=NAME_CONSTRAINT)
    order_key: str = p_internal(32, default=INTEGER_ZERO)
    text: Optional["Text"] = p_regular(
        33, default=None, require=False, array=False, struct=StructType.TEXT
    )
    icon: Optional["Icon"] = p_regular(34, require=False, array=False, struct=StructType.ICON)

    target: Optional["InlineSourceNode"] = p_regular(
        40, require=False, array=False, references=INLINE_SOURCE_NODE_TYPES.tuple
    )

    actions: LocalNodeList["Action"] = p_node_children(NodeType.ACTION)

    @staticmethod
    def new(name: str, **kwargs) -> "Kit":
        return Kit(name=name, **kwargs)
