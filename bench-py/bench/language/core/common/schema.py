from typing import TYPE_CHECKING, Union

from bench.language.core.builtin.trait import HasIcon
from bench.pb2 import SchemaData

from ..builtin import (
    HasName,
    IsArchivable,
    IsBlockable,
    IsDeletable,
    IsEnvironmental,
    IsInPackage,
    IsSourceable,
    IsTemplatable,
    IsTracked,
    Node,
    NodeType,
    node_,
    property_parent_,
)

if TYPE_CHECKING:
    from bench.language import Page

# pyright: reportIncompatibleVariableOverride=false


@node_(NodeType.SCHEMA)
class Schema(
    HasName,
    IsEnvironmental,
    HasIcon,
    IsTracked,
    IsTemplatable,
    IsBlockable,
    IsDeletable,
    IsArchivable,
    IsInPackage,
    IsSourceable,
    Node[SchemaData],
):
    """A Schema for a specific Type."""

    parent: Union["Page", None] = property_parent_(node_is_customizable=True)
