from typing import TYPE_CHECKING, Optional, Union

from destack.language.core import (
    Enum,
    EnumType,
    HasIcon,
    HasName,
    HasSlug,
    Instance,
    IsDeletable,
    IsOwnable,
    Length,
    Node,
    NodeType,
    Spatial,
    builtin_enum,
    builtin_node,
    property_,
    property_parent_,
)
from destack.pb2 import VariantData

if TYPE_CHECKING:
    from destack.language import Layer, Scene, View

# pyright: reportIncompatibleVariableOverride=false

# NOTE Architecture: maybe Variant should be split into specific Variant Nodes (IsVariant trait?)


@builtin_enum(EnumType.VARIANT_TYPE)
class VariantType(Enum):
    DYNAMIC = 1, "Dynamic", "Dynamic Variant (controlled by other logic)", "fas fa-shapes"
    BREAKPOINT = 2, "Breakpoint", "Breakpoint Variant", "fas fa-shapes"
    # DARK, STATE, ...


@builtin_node(NodeType.VARIANT)
class Variant(
    Spatial,
    Instance,
    HasName,
    HasSlug,
    HasIcon,
    IsOwnable,
    IsDeletable,
    Node[VariantData],
):
    """A Variant is an alternative presentation of a visual."""

    parent: Union["Scene", "Layer", "View", None] = property_parent_(node_is_customizable=True)
    type: VariantType = property_(30)

    max_width: Optional[Length] = property_(50)
    max_height: Optional[Length] = property_(51)
    min_width: Optional[Length] = property_(52)
    min_height: Optional[Length] = property_(53)
