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


@builtin_node(NodeType.NOTIFICATION_SENT_EVENT)
class NotificationSentEvent(
    Event["Notification"],
    Node,
):
    """A Event regarding a Notification."""

    node: "Notification" = property_(35)


@builtin_node(NodeType.NOTIFICATION_RESCINDED_EVENT)
class NotificationRescindedEvent(
    Event["Notification"],
    Node,
):
    """A Event regarding a Notification."""

    node: "Notification" = property_(35)


@builtin_node(NodeType.NOTIFICATION_READ_EVENT)
class NotificationReadEvent(
    Event["Notification"],
    Node,
):
    """A Event regarding a Notification."""

    node: "Notification" = property_(35)


@builtin_node(NodeType.NOTIFICATION_DISMISSED_EVENT)
class NotificationDismissedEvent(
    Event["Notification"],
    Node,
):
    """A Event regarding a Notification."""

    node: "Notification" = property_(35)


@builtin_node(NodeType.NOTIFICATION_EXPIRED_EVENT)
class NotificationExpiredEvent(
    Event["Notification"],
    Node,
):
    """A Event regarding a Notification."""

    node: "Notification" = property_(35)


@builtin_node(NodeType.NOTIFICATION)
class Notification(
    Spatial,
    Entity,
    IsOwnable,
    Node,
):
    """A Notification is a message about something."""

    status: NotificationStatus = property_(40)
    title: str = property_(50)
    text: "Text | None" = property_(51)
