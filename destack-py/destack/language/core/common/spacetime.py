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
    builtin_property,
    builtin_property_parent,
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

    parent: Union["Space", "Branch", None] = builtin_property_parent(node_is_extensible=False)


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

    parent: Optional["Space"] = builtin_property_parent(node_is_extensible=False)

    head: Optional["Snapshot"] = builtin_property(40)
