from typing import TYPE_CHECKING

from destack.core import (
    Entity,
    Enum,
    EnumType,
    Event,
    NodeType,
    TraitType,
    declare_entity,
    declare_enum,
    declare_event,
    declare_property,
)

if TYPE_CHECKING:
    from destack import Text

# pyright: reportIncompatibleVariableOverride=false


@declare_enum(EnumType.NOTIFICATION_STATUS)
class NotificationStatus(Enum):
    """A Status of a Notification."""

    UNREAD = 1, "Pending", "Pending", "fas fa-circle"
    READ = 2, "Read", "Read", "fas fa-check"
    DISMISSED = 3, "Dismissed", "Dismissed", "fas fa-times"
    EXPIRED = 4, "Expired", "Expired", "fas fa-clock"
    RESCINDED = 5, "Rescinded", "Rescinded", "fas fa-times"


@declare_event(NodeType.NOTIFICATION_EVENT, is_abstract=True)
class NotificationEvent(Event):
    """A Event regarding a Notification."""

    notification: "Notification" = declare_property(101)


@declare_event(NodeType.NOTIFICATION_SENT_EVENT)
class NotificationSentEvent(NotificationEvent):
    """A Notification was sent."""

    pass


@declare_event(NodeType.NOTIFICATION_RESCINDED_EVENT)
class NotificationRescindedEvent(NotificationEvent):
    """A Notification was rescinded."""

    pass


@declare_event(NodeType.NOTIFICATION_READ_EVENT)
class NotificationReadEvent(NotificationEvent):
    """A Notification was read."""

    pass


@declare_event(NodeType.NOTIFICATION_DISMISSED_EVENT)
class NotificationDismissedEvent(NotificationEvent):
    """A Notification was dismissed."""

    pass


@declare_event(NodeType.NOTIFICATION_EXPIRED_EVENT)
class NotificationExpiredEvent(NotificationEvent):
    """A Notification was expired."""

    pass


@declare_entity(
    NodeType.NOTIFICATION,
    traits=(TraitType.OWNABLE,),
    event_types=(NodeType.NOTIFICATION_EVENT,),
)
class Notification(Entity):
    """A Notification is a message about something."""

    title: str = declare_property(101)

    status: NotificationStatus = declare_property(110)

    text: "Text | None" = declare_property(120)
