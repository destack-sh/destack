from datetime import datetime
from typing import TYPE_CHECKING, Optional

from .common import RoleType
from .const import UNSET
from .entity import Entity
from .node import Node, NodeType, builtin_node
from .property import property_
from .trait import HasName, IsSourceable, IsSpatial

if TYPE_CHECKING:
    from destack.language import (
        CustomProperty,
        EditOperation,
        EditType,
        IsSubject,
        Metric,
        NodeReference,
        PropertyReference,
        Value,
    )

# pyright: reportIncompatibleVariableOverride=false


@builtin_node(NodeType.EVENT, pretend_frozen=True, is_abstract=True)
class Event[N: Node = Node](IsSpatial, Node):
    """
    An Event is an immutable record of something happening to an Entity.
    """

    created_at: datetime = property_(15, is_managed=True, is_eq=False, can_write=RoleType.SYSTEM)
    created_by: Optional["IsSubject"] = property_(
        16,
        default=None,
        is_managed=True,
        is_eq=False,
        node_space_from="self",
        node_is_customizable=False,
        can_write=RoleType.SYSTEM,
    )

    if TYPE_CHECKING:
        node: Optional[N] = None
        node_ptr: Optional[NodeReference] = None
    else:
        node: Optional["Node"] = property_(35, description="The Node this Event is about.")


@builtin_node(NodeType.CUSTOM_EVENT_DEFINITION)
class CustomEventDefinition(IsSpatial, HasName, IsSourceable, Entity):
    """A CustomEventDefinition defines a kind of CustomEvent."""

    pass


@builtin_node(NodeType.CUSTOM_EVENT, pretend_frozen=True, is_abstract=True)
class CustomEvent(Event):
    """An instance of a CustomEventDefinition."""

    definition: "CustomEventDefinition" = property_(
        40, description="The CustomEventDefinition this CustomEvent is an instance of."
    )


@builtin_node(NodeType.EDIT_EVENT, pretend_frozen=True)
class EditEvent(Event):
    """A Event of an Edit. Only EditEvents of Entities are allowed."""

    # key
    type: "EditType" = property_(30, is_repr=True)
    operation: "EditOperation | None" = property_(31, is_repr=True)
    node: "Node" = property_(35, is_repr=True)
    prop_ptr: "PropertyReference | None" = property_(36, is_repr=True)
    field: "CustomProperty | None" = property_(37, is_repr=True)  # for IsExtensible.value
    key: "Value | None" = property_(38, is_repr=True)  # for map operations
    if TYPE_CHECKING:
        node_ptr: NodeReference = UNSET
        field_ptr: NodeReference | None = None

    # value
    value: "Value | None" = property_(40)


@builtin_node(NodeType.MEASUREMENT_EVENT, is_abstract=True)
class MeasurementEvent(Event):
    """An Event that represents a Measurement."""

    definition: "Metric" = property_(6, is_managed=True, can_write=None)
