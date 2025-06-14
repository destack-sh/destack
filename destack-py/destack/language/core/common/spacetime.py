from typing import TYPE_CHECKING, Optional, Union

from destack.pb2 import BranchData, SnapshotData

from ..builtin import (
    Entity,
    HasName,
    HasSlug,
    IsDeletable,
    IsOwnable,
    Node,
    NodeType,
    Spatial,
    builtin_node,
    property_parent_,
)

if TYPE_CHECKING:
    from destack.language import Space

# pyright: reportIncompatibleVariableOverride=false


@builtin_node(NodeType.SNAPSHOT)
class Snapshot(
    Spatial,
    Entity,
    HasName,
    HasSlug,
    IsOwnable,
    IsDeletable,
    Node[SnapshotData],
):
    """A Snapshot is a point in Space time."""

    parent: Union["Space", "Branch", None] = property_parent_(node_is_customizable=False)


@builtin_node(NodeType.BRANCH)
class Branch(
    Spatial,
    Entity,
    HasName,
    HasSlug,
    IsOwnable,
    IsDeletable,
    Node[BranchData],
):
    """A Branch is a version of a Snapshot."""

    parent: Optional["Space"] = property_parent_(node_is_customizable=False)
