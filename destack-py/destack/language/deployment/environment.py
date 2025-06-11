from typing import TYPE_CHECKING, Optional

from destack.language.core import (
    Entity,
    HasIcon,
    HasName,
    IsDeletable,
    Node,
    NodeType,
    Spatial,
    node_,
    property_parent_,
)
from destack.pb2 import EnvironmentData

if TYPE_CHECKING:
    from destack.language import Space

# pyright: reportIncompatibleVariableOverride=false


@node_(NodeType.ENVIRONMENT)
class Environment(
    Spatial,
    Entity,
    HasName,
    HasIcon,
    IsDeletable,
    Node[EnvironmentData],
):
    """An Environment is a deployment of a Space."""

    parent: Optional["Space"] = property_parent_(node_is_customizable=False)
