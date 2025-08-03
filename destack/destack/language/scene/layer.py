from typing import TYPE_CHECKING, Optional

from destack.language.core import (
    Entity,
    Enum,
    EnumType,
    Float32,
    NodeType,
    TraitType,
    builtin_entity,
    builtin_enum,
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


@builtin_entity(
    NodeType.LAYER,
    traits=(TraitType.OWNABLE, TraitType.ORDERED),
    expected_ancestor_types=(NodeType.SCENE,),
    expected_descendant_types=(NodeType.VIEW,),
)
class Layer(Entity):
    """A Layer is a container for Views."""

    type: LayerType = builtin_property(100, default=LayerType.GENERAL)
    icon: "Icon | None" = builtin_property(102)

    # style
    is_visible: Optional[bool] = builtin_property(140)
    opacity: Optional[Float32] = builtin_property(141)
    # parallax?
