from typing import TYPE_CHECKING

from destack.language.core import (
    Entity,
    Enum,
    EnumType,
    Event,
    IsOwnable,
    Node,
    NodeType,
    Spatial,
    builtin_enum,
    builtin_node,
    property_,
)
from destack.pb2 import NotificationData, NotificationEventData

if TYPE_CHECKING:
    from destack.language import Text

# pyright: reportIncompatibleVariableOverride=false


@builtin_enum(EnumType.NOTIFICATION_STATUS)
class NotificationStatus(Enum):
    """A Status of a Notification."""

    UNREAD = 1, "Pending", "Pending", "fas fa-circle"
    READ = 2, "Read", "Read", "fas fa-check"
    DISMISSED = 3, "Dismissed", "Dismissed", "fas fa-times"
    EXPIRED = 4, "Expired", "Expired", "fas fa-clock"
    RESCINDED = 5, "Rescinded", "Rescinded", "fas fa-times"


@builtin_enum(EnumType.NOTIFICATION_EVENT_TYPE)
class NotificationEventType(Enum):
    """A Type of Notification Event."""

    SENT = 1, "Sent", "Sent", "fas fa-circle"
    RESCINDED = 2, "Rescinded", "Rescinded", "fas fa-times"
    READ = 3, "Read", "Read", "fas fa-check"
    DISMISSED = 4, "Dismissed", "Dismissed", "fas fa-times"
    EXPIRED = 5, "Expired", "Expired", "fas fa-clock"


@builtin_node(NodeType.NOTIFICATION_EVENT)
class NotificationEvent(
    Event["Notification"],
    Node[NotificationEventData],
):
    """A Event regarding a Notification."""

    type: NotificationEventType = property_(30)
    node: "Notification" = property_(35)


@builtin_node(NodeType.NOTIFICATION)
class Notification(
    Spatial,
    IsOwnable,
    Entity,
    Node[NotificationData],
):
    """A Notification is a message about something."""

    status: NotificationStatus = property_(40)
    title: str = property_(50)
    text: "Text | None" = property_(51)
