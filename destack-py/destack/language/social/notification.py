from typing import TYPE_CHECKING

from destack.language.core import (
    Entity,
    IsOwnable,
    Node,
    NodeType,
    Spatial,
    node_,
    property_,
)
from destack.pb2 import NotificationData

if TYPE_CHECKING:
    from destack.language import Text

# pyright: reportIncompatibleVariableOverride=false


@node_(NodeType.NOTIFICATION)
class Notification(
    Spatial,
    IsOwnable,
    Entity,
    Node[NotificationData],
):
    """A Notification is a message about something."""

    title: str = property_(40)
    text: "Text | None" = property_(41)
