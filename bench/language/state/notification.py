from datetime import datetime
from typing import TYPE_CHECKING, Optional, Union

import structlog

from bench.language.core import (
    TITLE_CONSTRAINT,
    BenchNode,
    BuiltinEnum,
    EnumType,
    IsTimed,
    Node,
    NodeType,
    StructType,
    enum_,
    p_internal,
    p_node_parent,
    p_regular,
    p_system,
    timed_node_,
)
from bench.pb2 import NotificationData

if TYPE_CHECKING:
    from bench.language import Bench, Channel, Message, Text, Thread

# pyright: reportIncompatibleVariableOverride=false

logger = structlog.get_logger(__name__)


@enum_(EnumType.NOTIFICATION_TYPE)
class NotificationType(BuiltinEnum):
    MESSAGE = 100


@enum_(EnumType.NOTIFICATION_STATUS)
class NotificationStatus(BuiltinEnum):
    SENDING = 20
    SENT = 30
    FAILED = 40
    RECEIVED = 50
    READ = 60


@timed_node_(NodeType.NOTIFICATION)
class Notification(IsTimed, BenchNode[NotificationData]):
    """
    A Notification about something.
    """

    # meta
    parent: Union["Bench", None] = p_node_parent(4, NodeType.BENCH)
    type: NotificationType = p_regular(30, require=True)
    channel: Optional["Channel"] = p_system(
        32,
        require=False,
        array=False,
        same_bench=True,
        references=NodeType.CHANNEL,
    )
    thread: Optional["Thread"] = p_regular(
        33,
        require=False,
        array=False,
        same_bench=True,
        references=NodeType.THREAD,
    )

    # status
    status: NotificationStatus = p_internal(40, default=NotificationStatus.SENT)
    failed_at: Optional[datetime] = p_system(42, default=None)
    sent_at: Optional[datetime] = p_system(43, default=None)
    received_at: Optional[datetime] = p_system(44, default=None)
    read_at: Optional[datetime] = p_system(45, default=None)

    # content
    title: Optional[str] = p_regular(50, require=False, default=None, constraint=TITLE_CONSTRAINT)
    text: Optional["Text"] = p_regular(51, require=False, default=None, struct=StructType.TEXT)
    nodes: list["Node"] = p_regular(53, array=True, require=False, references="any")
    message: Optional["Message"] = p_regular(
        54, require=False, array=False, baseless=True, references=NodeType.MESSAGE
    )

    def __content_str__(self) -> str:
        if self.title:
            return self.title
        elif self.text:
            return self.text.to_markdown()
        else:
            return "<empty>"
