from typing import TYPE_CHECKING, Optional, Union

from destack.language.core import (
    Entity,
    Enum,
    EnumType,
    IsDeletable,
    IsOrdered,
    IsOwnable,
    IsTaggable,
    IsViewable,
    NodeType,
    builtin_enum,
    builtin_node,
    builtin_property,
    builtin_property_parent,
)

if TYPE_CHECKING:
    from destack.language import (
        Axis3,
        Fill,
        Icon,
        Scene,
        Vector2f,
    )

# pyright: reportIncompatibleVariableOverride=false


@builtin_enum(EnumType.LAYER_TYPE)
class LayerType(Enum):
    """Built-in layer types."""

    GENERAL = 1
    SHAPE = 2
    # RASTER, ...


@builtin_node(NodeType.LAYER)
class Layer(
    IsViewable,
    IsOwnable,
    IsOrdered,
    IsTaggable,
    IsDeletable,
    Entity,
):
    """A Layer is a named container for Views."""

    parent: Union["Scene", None] = builtin_property_parent()
    type: LayerType = builtin_property(100, default=LayerType.GENERAL)
    name: str = builtin_property(101, is_repr=True)
    icon: "Icon | None" = builtin_property(102)

    # appearance
    is_visible: Optional[bool] = builtin_property(140)
    opacity: Optional[float] = builtin_property(141)
    fill: Optional["Fill"] = builtin_property(142)
    rotation: Optional["Axis3"] = builtin_property(143)
    skew: Optional["Vector2f"] = builtin_property(144)
    scale: Optional[float] = builtin_property(145)
