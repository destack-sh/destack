from typing import TYPE_CHECKING, Optional

from destack.language.core import (
    HasSlug,
    IsEntity,
    IsGlobal,
    IsInSpace,
    IsTracked,
    Node,
    NodeType,
    node_,
    property_,
    property_parent_,
)
from destack.pb2 import HandleData

if TYPE_CHECKING:
    from destack.language import Space

# pyright: reportIncompatibleVariableOverride=false


@node_(NodeType.HANDLE)
class Handle(
    HasSlug,
    IsGlobal,
    IsEntity,
    IsInSpace,
    IsTracked,
    Node[HandleData],
):
    """A Destack @handle."""

    parent: Optional["Space"] = property_parent_(node_is_customizable=False)

    slug: str = property_(33, is_repr=True)
