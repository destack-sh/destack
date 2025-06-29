from typing import TYPE_CHECKING, Optional, Union

from destack.language.core import (
    Entity,
    Enum,
    EnumType,
    HasName,
    IsActionable,
    IsDeletable,
    IsExtensible,
    IsRunnable,
    IsSourceable,
    IsTaggable,
    Node,
    NodeType,
    Spatial,
    builtin_enum,
    builtin_node,
    property_,
    property_parent_,
)

if TYPE_CHECKING:
    from destack.language import Text

# pyright: reportIncompatibleVariableOverride=false


@builtin_enum(EnumType.ACTION_CARDINALITY)
class ActionCardinality(Enum):
    UNARY = 1, "Unary", "Single in, single out"
    # UNARY_STREAM = 2, "Unary Stream", "Single in, stream out"

    @property
    def is_boundary(self) -> bool:
        return self < 40


@builtin_node(NodeType.ACTION)
class Action(
    Spatial,
    Entity,
    HasName,
    IsTaggable,
    IsSourceable,
    IsExtensible,
    IsDeletable,
    IsRunnable,
    Node,
):
    """
    An implementation of a unit of work, usually expressed with Code or some tool.
    May defer to a builtin or some other service in a separate system.
    """

    parent: Union["IsActionable", None] = property_parent_(node_is_customizable=True)

    cardinality: ActionCardinality = property_(40, default=ActionCardinality.UNARY)
    text: Optional["Text"] = property_(41)
