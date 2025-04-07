from typing import TYPE_CHECKING, Optional, Union

from bench.language.core import (
    BuiltinEnum,
    EnumType,
    InlineNode,
    IsInstantiable,
    IsJoinable,
    IsModal,
    IsNamed,
    IsTracked,
    LocalNodeList,
    NodeType,
    RemoteNodeList,
    enum_,
    p_node_children,
    p_node_parent,
    p_regular,
    timed_node_,
)
from bench.pb2 import ChannelData, MessageData

if TYPE_CHECKING:
    from bench.language import Agent, File, Membership, Message, Package, Page, Plan

# pyright: reportIncompatibleVariableOverride=false


@enum_(EnumType.CHANNEL_STATUS)
class ChannelStatus(BuiltinEnum):
    OPEN = 10
    CLOSED = 30


@timed_node_(NodeType.CHANNEL)
class Channel(IsInstantiable, IsTracked, IsJoinable, IsModal, IsNamed, InlineNode[ChannelData]):
    """
    A Channel for communcating with Messages and Threads.
    """

    # meta
    parent: Union["Page", "Package", None] = p_node_parent(4, NodeType.PAGE, NodeType.PACKAGE)
    # type: ChannelType? (text, voice, etc.)

    # content
    main_page: Optional["Page"] = p_regular(
        60, require=False, array=False, references=NodeType.PAGE, same_bench=True
    )
    main_plan: Optional["Plan"] = p_regular(
        61, require=False, array=False, references=NodeType.PLAN, same_bench=True
    )

    # status [80-90]

    messages: RemoteNodeList["Message", MessageData] = p_node_children(
        NodeType.MESSAGE, list=RemoteNodeList
    )
    agents: LocalNodeList["Agent"] = p_node_children(NodeType.AGENT)
    memberships: LocalNodeList["Membership"] = p_node_children(NodeType.MEMBERSHIP)
    files: LocalNodeList["File"] = p_node_children(NodeType.FILE)

    @staticmethod
    def new(name: str) -> "Channel":
        return Channel(name=name)
