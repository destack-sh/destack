from datetime import datetime
from typing import TYPE_CHECKING, Optional

from .common import RoleType
from .const import UNSET
from .entity import Entity
from .node import Node, NodeType, builtin_node
from .property import builtin_property
from .trait import HasName, IsCustomizable, IsExtensible, IsSourceable, IsSpatial

if TYPE_CHECKING:
    from destack.language import (
        CustomProperty,
        EditOperation,
        EditType,
        IsSubject,
        Metric,
        NodeDefinitionReference,
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

    created_at: datetime = builtin_property(
        15,
        is_managed=True,
        is_eq=False,
        is_readonly=True,
        can_write=RoleType.SYSTEM,
    )
    created_by: Optional["IsSubject"] = builtin_property(
        16,
        default=None,
        is_managed=True,
        is_eq=False,
        is_readonly=True,
        node_space_from="self",
        node_is_extensible=False,
        can_write=RoleType.SYSTEM,
    )

    if TYPE_CHECKING:
        node: Optional[N] = None
        node_ptr: Optional[NodeReference] = None
    else:
        node: Optional["Node"] = builtin_property(35, description="The Node this Event is about.")


@builtin_node(NodeType.CUSTOM_EVENT_DEFINITION)
class CustomEventDefinition(
    IsSpatial,
    HasName,
    IsSourceable,
    IsCustomizable,
    Entity,
):
    """A CustomEventDefinition defines a kind of CustomEvent with custom Properties."""

    prototype: Optional["CustomEvent"] = builtin_property(
        40,
        description="A custom Event's prototype is the default template new CustomEvent instances are based on.",
    )
    base_type: Optional["NodeDefinitionReference"] = builtin_property(41)


@builtin_node(NodeType.CUSTOM_EVENT, pretend_frozen=True, is_abstract=True)
class CustomEvent(Event, IsCustomizable, IsExtensible):
    """A CustomEvent is an instance of a CustomEventDefinition."""

    definition: "CustomEventDefinition" = builtin_property(
        40, description="The CustomEventDefinition this CustomEvent is an instance of."
    )


@builtin_node(NodeType.EDIT_EVENT, pretend_frozen=True)
class EditEvent(Event):
    """A Event of an Edit. Only EditEvents of Entities are allowed."""

    # key
    type: "EditType" = builtin_property(30, is_repr=True)
    operation: "EditOperation | None" = builtin_property(31, is_repr=True)
    node: "Node" = builtin_property(35, is_repr=True)
    prop_ptr: "PropertyReference | None" = builtin_property(36, is_repr=True)
    field: "CustomProperty | None" = builtin_property(37, is_repr=True)  # for IsExtensible.value
    key: "Value | None" = builtin_property(38, is_repr=True)  # for map operations
    if TYPE_CHECKING:
        node_ptr: NodeReference = UNSET
        field_ptr: NodeReference | None = None

    # value
    value: "Value | None" = builtin_property(40)


@builtin_node(NodeType.MEASUREMENT_EVENT, is_abstract=True)
class MeasurementEvent(Event):
    """An Event that represents a Measurement."""

    definition: "Metric" = builtin_property(6, is_managed=True, can_write=None)
