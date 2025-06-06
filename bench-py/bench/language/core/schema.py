from typing import TYPE_CHECKING, Union

from bench.pb2 import SchemaData

from .node import Node, NodeType, node_
from .property import property_parent_
from .trait import (
    HasEnvironment,
    HasName,
    IsArchivable,
    IsBlockable,
    IsDeletable,
    IsInPackage,
    IsTemplatable,
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
