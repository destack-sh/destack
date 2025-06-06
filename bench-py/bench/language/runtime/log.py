from typing import Optional

from bench.language.core import (
    Change,
    Edit,
    HasEnvironment,
    IsLog,
    Node,
    NodeType,
    Query,
    node_,
    property_,
)

# pyright: reportIncompatibleMethodOverride=false


@node_(NodeType.LOG)
class Log(HasEnvironment, IsLog, Node):
    """A Log."""

    edit: Optional["Edit"] = property_(40)
    change: Optional["Change"] = property_(41)
    query: Optional["Query"] = property_(42)
