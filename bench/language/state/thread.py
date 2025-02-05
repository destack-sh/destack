from datetime import datetime
from typing import TYPE_CHECKING, Optional, Union

from bench.language.core import (
    TITLE_CONSTRAINT,
    BuiltinEnum,
    EnumType,
    HasTimeIdentity,
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
    from bench.language import Bench, Block, Channel, Package, Run
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
    scope: Union["Block", "Package"] = p_regular(
        33, require=False, references=(NodeType.BLOCK, NodeType.PACKAGE), same_bench=True
    )
    run: Optional["Run"] = p_regular(
        34,
        require=False,
        array=False,
        references=NodeType.RUN,
        description="The Run this Message is scoped to. If no Run, this is a general in-source Message.",
    )

    # status
    status: ThreadStatus = p_internal(50, default=ThreadStatus.OPEN)
    closed_at: Optional[datetime] = p_internal(55, default=None)

    # content
    title: Optional[str] = p_regular(60, require=False, default=None, constraint=TITLE_CONSTRAINT)
    text: Optional["Text"] = p_regular(61, require=False, default=None, struct=StructType.TEXT)
