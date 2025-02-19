from typing import TYPE_CHECKING, Union

from bench.language.core import (
    BuiltinEnum,
    EnumType,
    InlineSourceNode,
    NodeType,
    enum_,
    p_node_parent,
    p_regular,
    timed_node_,
)
from bench.pb2 import ChannelData

if TYPE_CHECKING:
    from bench.language import Package, Page

# pyright: reportIncompatibleVariableOverride=false


@enum_(EnumType.CHANNEL_TYPE)
class ChannelType(BuiltinEnum):
    TEXT = 1
    # CLASS?
    # VOICE?


@timed_node_(NodeType.CHANNEL, has_subtypes=True)
class Channel(InlineSourceNode[ChannelData]):
    """
    A Channel for communcating with Messages and Threads.
    """

    # meta
    parent: Union["Page", "Package", None] = p_node_parent(4, NodeType.PAGE, NodeType.PACKAGE)
    type: ChannelType = p_regular(30, require=True, default=ChannelType.TEXT)

    @staticmethod
    def new(name: str, *, type: ChannelType = ChannelType.TEXT) -> "Channel":
        return Channel(name=name, type=type)
