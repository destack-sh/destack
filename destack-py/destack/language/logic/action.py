from typing import TYPE_CHECKING, Optional, Union

from destack.language.core import (
    BuiltinEnum,
    Entity,
    EnumType,
    HasName,
    IsActionable,
    IsDeletable,
    IsExtensible,
    IsRunnable,
    IsSourceable,
    IsTaggable,
    IsTemplatable,
    Node,
    NodeType,
    RunType,
    Spatial,
    enum_,
    node_,
    property_,
    property_parent_,
)
from destack.pb2 import ActionData

if TYPE_CHECKING:
    from destack.language import Text

# pyright: reportIncompatibleVariableOverride=false


@enum_(EnumType.ACTION_CARDINALITY)
class ActionCardinality(BuiltinEnum):
    UNARY = 1, "Unary", "Single in, single out"
    # UNARY_STREAM = 2, "Unary Stream", "Single in, stream out"

    @property
    def is_boundary(self) -> bool:
        return self < 40


@node_(NodeType.ACTION)
class Action(
    Spatial,
    Entity,
    HasName,
    IsTaggable,
    IsTemplatable,
    IsSourceable,
    IsExtensible,
    IsDeletable,
    IsRunnable,
    Node[ActionData],
):
    """
    An implementation of a unit of work, usually expressed with Code or some tool.
    May defer to a builtin or some other service in a separate system.
    """

    parent: Union["IsActionable", None] = property_parent_(node_is_customizable=True)

    cardinality: ActionCardinality = property_(40, default=ActionCardinality.UNARY)
    text: Optional["Text"] = property_(41)

    def run_type(self) -> RunType:
        return RunType.ACTION
