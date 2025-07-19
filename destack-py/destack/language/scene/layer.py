from typing import TYPE_CHECKING, Optional

from destack.language.core import (
    Entity,
    Enum,
    EnumType,
    IsOrdered,
    IsOwnable,
    IsScriptable,
    IsViewable,
    NodeType,
    builtin_enum,
    builtin_node,
    builtin_property,
)

if TYPE_CHECKING:
    from destack.language import (
        Axis3,
        Fill,
        Icon,
        Vector2f,
    )

# pyright: reportIncompatibleVariableOverride=false


@builtin_enum(EnumType.LAYER_TYPE)
class LayerType(Enum):
    """Built-in layer types."""

    GENERAL = 1
    SHAPE = 2
    # RASTER, ...


@builtin_node(
    NodeType.LAYER,
    expected_ancestor_types=(NodeType.SCENE,),
    expected_descendant_types=(NodeType.VIEW,),
)
class Layer(
    IsViewable,
    IsOwnable,
    IsOrdered,
    IsScriptable,
    Entity,
):
    """A Layer is a container for Views."""

    type: LayerType = builtin_property(100, default=LayerType.GENERAL)
    icon: "Icon | None" = builtin_property(102)

    # appearance
    is_visible: Optional[bool] = builtin_property(140)
    opacity: Optional[float] = builtin_property(141)
    fill: Optional["Fill"] = builtin_property(142)
    rotation: Optional["Axis3"] = builtin_property(143)
    skew: Optional["Vector2f"] = builtin_property(144)
    scale: Optional[float] = builtin_property(145)
    # parallax?
