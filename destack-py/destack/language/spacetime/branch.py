from typing import TYPE_CHECKING, Optional

from destack.language.core import (
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
    from destack.language import Icon, Snapshot, Space

# pyright: reportIncompatibleVariableOverride=false


@builtin_node(NodeType.BRANCH)
class Branch(
    IsSpatial,
    IsOwnable,
    IsDeletable,
    Entity,
):
    """A Branch is a version of a Snapshot."""

    parent: Optional["Space"] = builtin_property_parent()

    name: str = builtin_property(101, is_repr=True)
    icon: "Icon | None" = builtin_property(102)

    head: Optional["Snapshot"] = builtin_property(110)
