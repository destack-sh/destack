from typing import TYPE_CHECKING, Optional

from destack.language.core import (
    Entity,
    Enum,
    EnumType,
    IsOrdered,
    IsOwnable,
    IsScriptable,
    NodeType,
    builtin_enum,
    builtin_node,
    builtin_property,
)

if TYPE_CHECKING:
    from destack.language import Icon

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
    # parallax?
