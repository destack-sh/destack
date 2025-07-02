from typing import TYPE_CHECKING

from destack.language.core import (
    Entity,
    IsCustomizable,
    IsDeletable,
    IsExtensible,
    IsOwnable,
    IsRunnable,
    IsScriptable,
    IsSourceable,
    IsSpatial,
    IsTaggable,
    NodeType,
    builtin_node,
    builtin_property,
)

if TYPE_CHECKING:
    from destack.language import Icon


# pyright: reportIncompatibleVariableOverride=false, reportIncompatibleMethodOverride=false


@builtin_node(NodeType.SERVICE)
class Service(
    IsSpatial,
    IsDeletable,
    IsOwnable,
    IsTaggable,
    IsRunnable,
    IsScriptable,
    IsExtensible,
    IsSourceable,
    IsCustomizable,
    Entity,
):
    """
    A set of Actions for a Node.
    """

    name: str = builtin_property(101, is_repr=True)
    icon: "Icon | None" = builtin_property(102)
