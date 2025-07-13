from typing import TYPE_CHECKING

from destack.language.core import (
    Entity,
    Enum,
    EnumType,
    Event,
    IsOwnable,
    NodeType,
    builtin_enum,
    builtin_node,
    builtin_property,
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


@builtin_node(NodeType.NOTIFICATION_EVENT, frozen=True, is_abstract=True)
class NotificationEvent(Event["Notification"]):
    """A Event regarding a Notification."""

    node: "Notification" = builtin_property(101)


@builtin_node(NodeType.NOTIFICATION_SENT_EVENT, frozen=True)
class NotificationSentEvent(NotificationEvent):
    """A Notification was sent."""

    pass


@builtin_node(NodeType.NOTIFICATION_RESCINDED_EVENT, frozen=True)
class NotificationRescindedEvent(NotificationEvent):
    """A Notification was rescinded."""

    pass


@builtin_node(NodeType.NOTIFICATION_READ_EVENT, frozen=True)
class NotificationReadEvent(NotificationEvent):
    """A Notification was read."""

    pass


@builtin_node(NodeType.NOTIFICATION_DISMISSED_EVENT, frozen=True)
class NotificationDismissedEvent(NotificationEvent):
    """A Notification was dismissed."""

    pass


@builtin_node(NodeType.NOTIFICATION_EXPIRED_EVENT, frozen=True)
class NotificationExpiredEvent(NotificationEvent):
    """A Notification was expired."""

    pass


@builtin_node(
    NodeType.NOTIFICATION,
    event_types=(NodeType.NOTIFICATION_EVENT,),
)
class Notification(IsOwnable, Entity):
    """A Notification is a message about something."""

    status: NotificationStatus = builtin_property(40)
    title: str = builtin_property(50)
    text: "Text | None" = builtin_property(51)
