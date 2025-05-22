from typing import TYPE_CHECKING, Union

from bench.language.core import (
    IsArchivable,
    IsBlockable,
    IsDeletable,
    IsModal,
    IsNamed,
    IsTemplatable,
    Node,
    NodeType,
    TypeBase,
    node_,
    p_node_parent,
)
from bench.pb2 import SchemaData

if TYPE_CHECKING:
    from bench.language import Page

# pyright: reportIncompatibleVariableOverride=false


@node_(NodeType.SCHEMA)
class Schema(
    IsTemplatable,
    IsModal,
    IsNamed,
    IsBlockable,
    IsDeletable,
    IsArchivable,
    TypeBase,
    Node[SchemaData],
):
    """A Schema for a specific Type."""

    parent: Union["Page", None] = p_node_parent()
