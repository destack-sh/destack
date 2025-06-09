from typing import TYPE_CHECKING

from destack.language.core.builtin.trait import HasIcon
from destack.pb2 import SchemaData

from ..builtin import (
    HasName,
    IsArchivable,
    IsDeletable,
    IsEnvironmental,
    IsInFolder,
    IsSourceable,
    IsTemplatable,
    IsTracked,
    Node,
    NodeType,
    node_,
)

if TYPE_CHECKING:
    pass

# pyright: reportIncompatibleVariableOverride=false


@node_(NodeType.SCHEMA)
class Schema(
    HasName,
    IsEnvironmental,
    HasIcon,
    IsTracked,
    IsTemplatable,
    IsDeletable,
    IsArchivable,
    IsInFolder,
    IsSourceable,
    Node[SchemaData],
):
    """A Schema for a specific Type."""

    pass
