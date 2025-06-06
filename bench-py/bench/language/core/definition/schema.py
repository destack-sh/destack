from typing import TYPE_CHECKING, Union

from bench.pb2 import SchemaData

from ..builtin import (
    HasEnvironment,
    HasName,
    IsArchivable,
    IsBlockable,
    IsDeletable,
    IsInPackage,
    IsTemplatable,
    Node,
    NodeType,
    node_,
    property_parent_,
)
from .type import TypeBase

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
