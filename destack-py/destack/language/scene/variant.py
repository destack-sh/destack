from typing import TYPE_CHECKING, Optional, Union

from destack.language.core import (
    Entity,
    Enum,
    EnumType,
    IsDeletable,
    IsOwnable,
    IsSpatial,
    Length,
    NodeType,
    builtin_enum,
    builtin_node,
    builtin_property,
    builtin_property_parent,
)

if TYPE_CHECKING:
    from destack.language import Icon, Layer, Scene, View

# pyright: reportIncompatibleVariableOverride=false

# NOTE Architecture: maybe Variant should be split into specific Variant Nodes (IsVariant trait?)


@builtin_enum(EnumType.VARIANT_TYPE)
class VariantType(Enum):
    DYNAMIC = 1, "Dynamic", "Dynamic Variant (controlled by other logic)", "fas fa-shapes"
    BREAKPOINT = 2, "Breakpoint", "Breakpoint Variant", "fas fa-shapes"
    PLATFORM = 3, "Platform", "Platform Variant", "fas fa-linux"
    # DARK, STATE, ...


@builtin_enum(EnumType.VARIANT_STATE_TYPE)
class VariantStateType(Enum):
    LOADING = 10, "Loading", "Loading Variant", "fas fa-shapes"
    ERROR = 11, "Error", "Error Variant", "fas fa-shapes"


@builtin_node(NodeType.VARIANT)
class Variant(
    IsSpatial,
    IsOwnable,
    IsDeletable,
    Entity,
):
    """A Variant is an alternative presentation of a visual."""

    parent: Union["Scene", "Layer", "View", None] = builtin_property_parent()
    type: VariantType = builtin_property(100)
    name: str = builtin_property(101, is_repr=True)
    icon: "Icon | None" = builtin_property(102)

    max_width: Optional[Length] = builtin_property(110)
    max_height: Optional[Length] = builtin_property(111)
    min_width: Optional[Length] = builtin_property(112)
    min_height: Optional[Length] = builtin_property(113)
