from typing import TYPE_CHECKING, Optional, Union

from destack.language.core import (
    Entity,
    Enum,
    EnumType,
    IsCustomizable,
    IsDeletable,
    IsRunnable,
    IsScriptable,
    IsSourceable,
    IsSpatial,
    IsTaggable,
    NodeType,
    builtin_enum,
    builtin_node,
    builtin_property,
    builtin_property_parent,
)

if TYPE_CHECKING:
    from destack.language import Icon, Text

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
    IsSpatial,
    IsTaggable,
    IsSourceable,
    IsCustomizable,
    IsDeletable,
    IsRunnable,
    Entity,
):
    """
    An implementation of a unit of work, usually expressed with Code or some tool.
    May defer to a builtin or some other service in a separate system.
    """

    parent: Union["IsScriptable", None] = builtin_property_parent(node_is_extensible=True)

    name: str = builtin_property(101, is_repr=True)
    icon: "Icon | None" = builtin_property(102)
    text: Optional["Text"] = builtin_property(104)

    cardinality: ActionCardinality = builtin_property(110, default=ActionCardinality.UNARY)
