from typing import TYPE_CHECKING, Optional, Union

from ..builtin import (
    Entity,
    IsDeletable,
    IsOwnable,
    IsSpatial,
    NodeType,
    builtin_node,
    builtin_property,
    builtin_property_parent,
)

if TYPE_CHECKING:
    from destack.language import Icon, Space

# pyright: reportIncompatibleVariableOverride=false


@builtin_node(NodeType.SNAPSHOT)
class Snapshot(
    IsSpatial,
    IsOwnable,
    IsDeletable,
    Entity,
):
    """A Snapshot is a point in Space time."""

    parent: Union["Space", "Branch", None] = builtin_property_parent(node_is_extensible=False)

    name: str = builtin_property(101, is_repr=True)
    icon: "Icon | None" = builtin_property(102)


@builtin_node(NodeType.BRANCH)
class Branch(
    IsSpatial,
    IsOwnable,
    IsDeletable,
    Entity,
):
    """A Branch is a version of a Snapshot."""

    parent: Optional["Space"] = builtin_property_parent(node_is_extensible=False)

    name: str = builtin_property(101, is_repr=True)
    icon: "Icon | None" = builtin_property(102)

    head: Optional["Snapshot"] = builtin_property(110)
