from typing import TYPE_CHECKING, Optional, Union

from destack.proto import BranchProto, SnapshotProto

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
    property_,
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
    Node[SnapshotProto],
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
    Node[BranchProto],
):
    """A Branch is a version of a Snapshot."""

    parent: Optional["Space"] = property_parent_(node_is_customizable=False)

    head: Optional["Snapshot"] = property_(40)
