from typing import TYPE_CHECKING, Optional

from destack.core import (
    Entity,
    EnumDeclaration,
    EnumType,
    Float32,
    Icon,
    NodeType,
    TraitType,
    declare_entity,
    declare_enum,
    declare_property,
)

if TYPE_CHECKING:
    pass

# pyright: reportIncompatibleVariableOverride=false


@declare_enum(EnumType.LAYER_TYPE)
class LayerType(EnumDeclaration):
    """Built-in layer types."""

    GENERAL = 1
    SHAPE = 2
    # RASTER, ...


@declare_entity(
    NodeType.LAYER,
    traits=(TraitType.OWNABLE, TraitType.ORDERED),
    expected_ancestor_types=(NodeType.SCENE,),
    expected_descendant_types=(NodeType.VIEW,),
)
class Layer(Entity):
    """A Layer is a container for Views."""

    type: LayerType = declare_property(100, default=LayerType.GENERAL)
    icon: "Icon | None" = declare_property(102)

    # style
    is_visible: Optional[bool] = declare_property(140)
    opacity: Optional[Float32] = declare_property(141)
    # parallax?
