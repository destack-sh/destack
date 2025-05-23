from datetime import datetime
from typing import TYPE_CHECKING, Optional, Union

import structlog

from bench.language.core import (
    BuiltinEnum,
    EnumType,
    IsDeletable,
    IsInPackage,
    IsModal,
    IsTitled,
    Node,
    NodeType,
    enum_,
    node_,
    property_,
    property_parent_,
)
from bench.pb2 import NotificationData

if TYPE_CHECKING:
    from bench.language import Channel, Package, Text, Thread

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


@node_(NodeType.NOTIFICATION)
class Notification(
    IsTitled,
    IsModal,
    IsInPackage,
    IsDeletable,
    Node[NotificationData],
):
    """
    A Notification about something.
    """

    # meta
    parent: Union["Package", None] = property_parent_()
    type: NotificationType = property_(30)
    channel: Optional["Channel"] = property_(33, node_bench_from="self", can_write="system")
    thread: Optional["Thread"] = property_(34, node_bench_from="self", can_write="system")

    # status
    status: NotificationStatus = property_(40, default=NotificationStatus.SENT)
    failed_at: Optional[datetime] = property_(42)
    sent_at: Optional[datetime] = property_(43)
    received_at: Optional[datetime] = property_(44)
    read_at: Optional[datetime] = property_(45)

    # content
    text: Optional["Text"] = property_(51)

    def __content_str__(self) -> str:
        if self.title:
            return self.title.to_plain(max_characters=100)
        elif self.text:
            return self.text.to_plain(max_characters=100)
        else:
            return "<empty>"
