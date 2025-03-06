import typing
from typing import Optional, Union

from bench.language.core import (
    NAME_CONSTRAINT,
    InlineSourceNode,
    NodeType,
    StructType,
    node_,
    p_internal,
    p_node_parent,
    p_regular,
)
from bench.pb2 import TagData
from bench.utils.fractional import INTEGER_ZERO

if typing.TYPE_CHECKING:
    from bench.language import Icon, Package, Page, Text


# pyright: reportIncompatibleVariableOverride=false, reportIncompatibleMethodOverride=false


@node_(NodeType.TAG)
class Tag(InlineSourceNode[TagData]):
    """
    A Tag to tag other Nodes.
    """

    parent: Union["Page", "Package", None] = p_node_parent(4, NodeType.PAGE, NodeType.PACKAGE)
    name: str | None = p_regular(31, constraint=NAME_CONSTRAINT)
    order_key: str = p_internal(32, default=INTEGER_ZERO)
    text: Optional["Text"] = p_regular(
        33, default=None, require=False, array=False, struct=StructType.TEXT
    )
    icon: Optional["Icon"] = p_regular(34, require=False, array=False, struct=StructType.ICON)

    @staticmethod
    def new(name: str, **kwargs) -> "Tag":
        return Tag(name=name, **kwargs)
