from typing import TYPE_CHECKING, Optional

from destack.language.core import (
    HasIcon,
    HasName,
    IsDeletable,
    IsSpatial,
    Node,
    NodeType,
    node_,
    property_parent_,
)

if TYPE_CHECKING:
    from destack.language import Space

# pyright: reportIncompatibleVariableOverride=false


@node_(NodeType.ENVIRONMENT)
class Environment(
    HasName,
    HasIcon,
    IsSpatial,
    IsDeletable,
    Node,
):
    """An Environment is a deployment of a Space."""

    parent: Optional["Space"] = property_parent_(node_is_customizable=False)
