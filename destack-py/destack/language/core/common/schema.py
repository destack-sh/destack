from typing import TYPE_CHECKING

from destack.language.core.builtin.trait import HasIcon
from destack.pb2 import SchemaData

from ..builtin import (
    Entity,
    HasName,
    IsDeletable,
    IsSourceable,
    IsTaggable,
    IsTemplatable,
    Node,
    NodeType,
    Spatial,
    node_,
)

if TYPE_CHECKING:
    pass

# pyright: reportIncompatibleVariableOverride=false


@node_(NodeType.SCHEMA)
class Schema(
    HasName,
    HasIcon,
    Entity,
    IsTaggable,
    IsTemplatable,
    IsDeletable,
    Spatial,
    IsSourceable,
    Node[SchemaData],
):
    """A Schema describes a Type."""

    pass
