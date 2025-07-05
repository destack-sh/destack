from typing import TYPE_CHECKING

from destack.language.core import (
    Entity,
    IsDeletable,
    IsExtensible,
    IsOwnable,
    IsRunnable,
    IsSourceable,
    IsSpatial,
    IsSubject,
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
    IsExtensible,
    IsSourceable,
    IsSubject,
    Entity,
):
    """
    A Service provides functionality.
    """

    name: str = builtin_property(101, is_repr=True)
    icon: "Icon | None" = builtin_property(102)
