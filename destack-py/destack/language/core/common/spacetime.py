from typing import TYPE_CHECKING, Optional, Union

from ..builtin import (
    Entity,
    HasName,
    HasSlug,
    IsDeletable,
    IsOwnable,
    IsSpatial,
    NodeType,
    builtin_node,
    property_,
    property_parent_,
)

if TYPE_CHECKING:
    from destack.language import Space

# pyright: reportIncompatibleVariableOverride=false


@builtin_node(NodeType.SNAPSHOT)
class Snapshot(
    IsSpatial,
    HasName,
    HasSlug,
    IsOwnable,
    IsDeletable,
    Entity,
):
    """A Snapshot is a point in Space time."""

    parent: Union["Space", "Branch", None] = property_parent_(node_is_extensible=False)


@builtin_node(NodeType.BRANCH)
class Branch(
    IsSpatial,
    HasName,
    HasSlug,
    IsOwnable,
    IsDeletable,
    Entity,
):
    """A Branch is a version of a Snapshot."""

    parent: Optional["Space"] = property_parent_(node_is_extensible=False)

    head: Optional["Snapshot"] = property_(40)
