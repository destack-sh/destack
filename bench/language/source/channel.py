from typing import TYPE_CHECKING, Optional, Union

from bench.language.core import (
    NAME_CONSTRAINT,
    BuiltinEnum,
    EnumType,
    NodeType,
    SourceNode,
    StructType,
    Text,
    enum_,
    p_internal,
    p_node_parent,
    p_regular,
    timed_node_,
)
from bench.pb2 import ChannelData
from bench.utils.fractional import INTEGER_ZERO

if TYPE_CHECKING:
    from bench.language import Icon, Package, Page

# pyright: reportIncompatibleVariableOverride=false


@enum_(EnumType.CHANNEL_TYPE)
class ChannelType(BuiltinEnum):
    TEXT = 1
    VOICE = 2


@timed_node_(NodeType.CHANNEL, has_subtypes=True)
class Channel(SourceNode[ChannelData]):
    """
    A Channel for communcating with Messages and Threads.
    """

    # meta
    parent: Union["Page", "Package", None] = p_node_parent(4, NodeType.PAGE, NodeType.PACKAGE)
    type: ChannelType = p_regular(30, require=True, default=ChannelType.TEXT)
    name: str = p_regular(32, constraint=NAME_CONSTRAINT)
    order_key: str = p_internal(33, default=INTEGER_ZERO)
    icon: Optional["Icon"] = p_regular(
        34, default=None, require=False, array=False, struct=StructType.ICON
    )
    text: Optional["Text"] = p_regular(
        35, default=None, require=False, array=False, struct=StructType.TEXT
    )
