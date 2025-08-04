from typing import TYPE_CHECKING

from destack.core import (
    Entity,
    EnumType,
    Event,
    NodeType,
    OptionEnum,
    TraitType,
    declare_entity,
    declare_enum,
    declare_event,
    declare_option,
    declare_property,
)

if TYPE_CHECKING:
    from destack import Text


@declare_enum(EnumType.NOTIFICATION_STATUS)
class NotificationStatus(OptionEnum):
    """A Status of a Notification."""

    UNREAD = declare_option(1, description="Pending")
    READ = declare_option(2, description="Read")
    DISMISSED = declare_option(3, description="Dismissed")
    EXPIRED = declare_option(4, description="Expired")
    RESCINDED = declare_option(5, description="Rescinded")


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
