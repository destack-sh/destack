from typing import TYPE_CHECKING

from bench.language.core import (
    IsEnvironmental,
    IsInPackage,
    Node,
    NodeType,
    node_,
    property_,
)

if TYPE_CHECKING:
    pass

# pyright: reportIncompatibleVariableOverride=false


@node_(NodeType.LOG)
class Log(
    IsEnvironmental,
    IsInPackage,
    Node,
):
    """A Log message."""

    content: str = property_(40)
