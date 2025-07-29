from typing import (
    TYPE_CHECKING,
    Optional,
)

from .builtin import NodeType, TraitType
from .const import UNSET
from .node import builtin_node
from .property import (
    builtin_property,
)

if TYPE_CHECKING:
    from destack.language import (
        Anchor,
        Icon,
        NodeReference,
        Offset2,
        Quaternion,
        Region,
        Vector2,
        Vector3,
    )

from .entity import Entity

# pyright: reportIncompatibleVariableOverride=false

type_ = type
object_set_ = object.__setattr__


@builtin_node(
    NodeType.RECORD,
    is_extensible=True,
    is_abstract=True,
    traits=(TraitType.OWNABLE,),
)
class Record(Entity):
    """
    A generic Record instance of a CustomEntity.
    """

    pass


@builtin_node(
    NodeType.RESOURCE,
    is_extensible=True,
    is_abstract=True,
    traits=(TraitType.OWNABLE,),
)
class Resource(Entity):
    """
    A Resource represents an external asset outside of Destack.
    The lifecycle of a Resource may be managed by some Provisioner (Service).
    """

    region: Optional["Region"] = builtin_property(111)


@builtin_node(
    NodeType.VARIANT,
    is_extensible=True,
    is_abstract=True,
    traits=(TraitType.OWNABLE,),
)
class Variant(Entity):
    """A Variant is an alternative version of an Entity."""

    icon: "Icon | None" = builtin_property(102)


@builtin_node(
    NodeType.TAG,
    is_extensible=True,
    traits=(TraitType.ORDERED,),
)
class Tag(Entity):
    """A Tag to tag an Entity with (in a Tagging)."""

    icon: "Icon | None" = builtin_property(102)


@builtin_node(
    NodeType.TAGGING,
    traits=(TraitType.ORDERED,),
)
class Tagging(Entity):
    """A Tagging of a Node by a Tag."""

    tag: Tag = builtin_property(110)
    if TYPE_CHECKING:
        tag_ptr: NodeReference = UNSET


@builtin_node(
    NodeType.ENTITY2D,
    is_extensible=True,
    is_abstract=True,
)
class Entity2D(Entity):
    """An Entity in 2D space."""

    # transform
    position: Optional["Vector2"] = builtin_property(
        110,
        tags=("transform",),
    )
    offset: Optional["Offset2"] = builtin_property(
        111,
        tags=("transform",),
    )
    scale: Optional["Vector2"] = builtin_property(
        112,
        tags=("transform",),
    )
    rotation: Optional["Vector2"] = builtin_property(
        113,
        tags=("transform",),
    )
    skew: Optional["Vector2"] = builtin_property(
        114,
        tags=("transform",),
    )
    origin: Optional["Vector2"] = builtin_property(
        115,
        tags=("transform",),
    )
    anchor: Optional["Anchor"] = builtin_property(
        116,
        tags=("transform",),
    )


@builtin_node(
    NodeType.ENTITY3D,
    is_extensible=True,
    is_abstract=True,
)
class Entity3D(Entity):
    """An Entity in 3D space."""

    # transform
    position: Optional["Vector3"] = builtin_property(
        110,
        tags=("transform",),
    )
    scale: Optional["Vector3"] = builtin_property(
        111,
        tags=("transform",),
    )
    rotation: Optional["Quaternion"] = builtin_property(
        112,
        tags=("transform",),
    )
    skew: Optional["Vector3"] = builtin_property(
        113,
        tags=("transform",),
    )
    origin: Optional["Vector3"] = builtin_property(
        114,
        tags=("transform",),
    )
    anchor: Optional["Anchor"] = builtin_property(
        115,
        tags=("transform",),
    )
