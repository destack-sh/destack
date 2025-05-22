from typing import TYPE_CHECKING, Union

from bench.language.core import (
    BuiltinEnum,
    EnumType,
    IsArchivable,
    IsDeletable,
    IsInstantiable,
    IsJoinable,
    IsModal,
    IsNamed,
    IsProcessable,
    Node,
    NodeType,
    enum_,
    node_,
    p_node_parent,
)
from bench.pb2 import ChannelData

if TYPE_CHECKING:
    from bench.language import Package, Page

# pyright: reportIncompatibleVariableOverride=false


@enum_(EnumType.CHANNEL_STATUS)
class ChannelStatus(BuiltinEnum):
    OPEN = 10
    CLOSED = 30


@node_(NodeType.CHANNEL)
class Channel(
    IsArchivable,
    IsDeletable,
    IsInstantiable,
    IsJoinable,
    IsModal,
    IsNamed,
    IsProcessable,
    Node[ChannelData],
):
    """
    A Channel for organizing Messages and Threads.
    """

    # meta
    parent: Union["Page", "Package", None] = p_node_parent()
    # type: ChannelType? (text, voice, etc.)

    # ...IsProcessable[80-]

    @staticmethod
    def new(name: str) -> "Channel":
        return Channel(name=name)
