from typing import TYPE_CHECKING, Optional, Union

from bench.language.core import (
    IsBlockable,
    IsInstantiable,
    IsModal,
    IsNamed,
    p_node_parent,
    p_regular,
    trait_,
)
from bench.language.core.const import Trait
from bench.pb2 import AnyNodeData

if TYPE_CHECKING:
    from bench.language import Dimension, Page, Position, Space

# pyright: reportIncompatibleVariableOverride=false


@trait_(Trait.VIEW)
class IsView[NodeDataT: AnyNodeData](
    IsInstantiable,
    IsModal,
    IsNamed,
    IsBlockable,
):
    """A View is a graphical interface."""

    parent: Union["Space", "IsView", "Page", None] = p_node_parent()
    # variant_of, ...

    # sizing
    position: Optional["Position"] = p_regular(40)
    width: Optional["Dimension"] = p_regular(41)
    height: Optional["Dimension"] = p_regular(42)
    min_width: Optional["Dimension"] = p_regular(43)
    min_height: Optional["Dimension"] = p_regular(44)
    max_width: Optional["Dimension"] = p_regular(45)
    max_height: Optional["Dimension"] = p_regular(46)
