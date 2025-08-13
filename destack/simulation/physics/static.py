from typing import TYPE_CHECKING

from destack.core import NodeType, declare_entity

from .body import Body2D, Body3D

if TYPE_CHECKING:
    pass


@declare_entity(NodeType.STATIC_BODY2D)
class StaticBody2D(Body2D):
    """A static Body in 2D space."""

    pass


@declare_entity(NodeType.STATIC_BODY3D)
class StaticBody3D(Body3D):
    """A static Body in 3D space."""

    pass
