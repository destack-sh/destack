from datetime import datetime
from typing import TYPE_CHECKING, Optional, Union

from bench.language.core import (
    TITLE_CONSTRAINT,
    BuiltinEnum,
    EnumType,
    HasTimeIdentity,
    Node,
    NodeType,
    StateNode,
    StructType,
    Text,
    enum_,
    p_internal,
    p_node_parent,
    p_regular,
    p_system,
    timed_node_,
)
from bench.pb2 import ThreadData

if TYPE_CHECKING:
    from bench.language import Bench, Channel, Message, Package, Page, Run

# pyright: reportIncompatibleVariableOverride=false


@enum_(EnumType.THREAD_TYPE)
class ThreadType(BuiltinEnum):
    SOURCE = 1
    RUN = 2


@enum_(EnumType.THREAD_STATUS)
class ThreadStatus(BuiltinEnum):
    OPEN = 10
    CLOSED = 30


@timed_node_(NodeType.THREAD, has_subtypes=True)
class Thread(HasTimeIdentity, StateNode[ThreadData]):
    """
    A Thread for communicating with Messages on something.
    """

    # meta
    parent: Union["Bench", None] = p_node_parent(4, NodeType.BENCH)
    type: ThreadType = p_regular(30, require=True, default=ThreadType.SOURCE)
    channel: "Channel | None" = p_system(
        32,
        require=False,
        store=True,
        wire=True,
        same_bench=True,
        references=NodeType.CHANNEL,
    )
    scope: Union["Page", "Package"] = p_regular(
        35, require=False, references=(NodeType.PAGE, NodeType.PACKAGE)
    )
    run: Optional["Run"] = p_regular(
        37,
        require=False,
        array=False,
        references=NodeType.RUN,
        description="The Run this Message is scoped to.",
    )

    # status
    status: ThreadStatus = p_internal(40, default=ThreadStatus.OPEN)
    closed_at: Optional[datetime] = p_internal(45, default=None)

    # content
    title: Optional[str] = p_regular(50, require=False, default=None, constraint=TITLE_CONSTRAINT)
    text: Optional["Text"] = p_regular(51, require=False, default=None, struct=StructType.TEXT)

    # routing
    created_from: Optional["Message"] = p_regular(
        60, require=False, array=False, references=NodeType.MESSAGE, same_bench=True
    )

    @property
    def container(self) -> "Node | None":
        if (channel := self.channel) is not None:
            return channel
        else:
            return self.scope
