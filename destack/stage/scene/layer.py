from typing import TYPE_CHECKING, Optional

from destack.core import (
    Entity,
    Float32,
    Icon,
    NodeType,
    TraitType,
    declare_entity,
    declare_property,
)

if TYPE_CHECKING:
    pass


@declare_entity(
    NodeType.LAYER,
    traits=(TraitType.OWNABLE, TraitType.ORDERED),
    expected_ancestor_types=(NodeType.SCENE,),
    expected_descendant_types=(NodeType.VIEW,),
)
class Layer(Entity):
    """A Layer is a container for Views."""

    icon: "Icon | None" = declare_property(102)

    # style
    is_visible: Optional[bool] = declare_property(140)
    opacity: Optional[Float32] = declare_property(141)
    # parallax?
