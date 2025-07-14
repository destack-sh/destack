from typing import TYPE_CHECKING

from destack.language.core import (
    Entity,
    IsActor,
    IsDeletable,
    IsExtensible,
    IsOwnable,
    IsRunnable,
    IsSourceable,
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
    IsDeletable,
    IsOwnable,
    IsTaggable,
    IsExtensible,
    IsSourceable,
    IsActor,
    IsRunnable,
    Entity,
):
    """
    A Service provides related functionality via Actions (and Methods).
    Services may be stateful (with custom Properties and runtime only state).
    """

    name: str = builtin_property(101, is_repr=True)
    icon: "Icon | None" = builtin_property(102)
