from typing import TYPE_CHECKING, Union

from bench.language.core import (
    HasEnvironment,
    HasName,
    IsArchivable,
    IsBlockable,
    IsDeletable,
    IsInPackage,
    IsTemplatable,
    Node,
    NodeType,
    TypeBase,
    node_,
    property_parent_,
)
from bench.pb2 import SchemaData

if TYPE_CHECKING:
    from bench.language import Page

# pyright: reportIncompatibleVariableOverride=false


@node_(NodeType.SCHEMA)
class Schema(
    IsTemplatable,
    HasEnvironment,
    HasName,
    IsBlockable,
    IsDeletable,
    IsArchivable,
    IsInPackage,
    TypeBase,
    Node[SchemaData],
):
    """A Schema for a specific Type."""

    parent: Union["Page", None] = property_parent_()
