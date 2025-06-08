from typing import TYPE_CHECKING, Optional, Union

from bench.language.core import (
    BuiltinEnum,
    EnumType,
    HasName,
    IsArchivable,
    IsDeletable,
    IsEnvironmental,
    IsInPackage,
    IsOrdered,
    IsRunnable,
    IsSourceable,
    IsTemplatable,
    IsTracked,
    Node,
    NodeType,
    RunType,
    enum_,
    node_,
    property_,
    property_parent_,
)
from bench.pb2 import ActionData

if TYPE_CHECKING:
    from bench.language import Service, Text

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
    HasName,
    IsEnvironmental,
    IsTemplatable,
    IsSourceable,
    IsOrdered,
    IsArchivable,
    IsDeletable,
    IsRunnable,
    IsInPackage,
    IsTracked,
    Node[ActionData],
):
    """
    An implementation of a unit of work, usually expressed with Code or some tool.
    May defer to a builtin or some other service in a separate system.
    """

    parent: Union["Service", None] = property_parent_(node_is_customizable=True)

    cardinality: ActionCardinality = property_(40, default=ActionCardinality.UNARY)
    text: Optional["Text"] = property_(41)

    def run_type(self) -> RunType:
        return RunType.ACTION
