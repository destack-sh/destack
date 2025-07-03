from datetime import datetime
from typing import TYPE_CHECKING, Optional

from .common import RoleType
from .const import UNSET
from .entity import Entity
from .node import Node, NodeType, builtin_node
from .property import builtin_property, builtin_property_parent
from .trait import IsCustomizable, IsExtensible, IsSourceable, IsSpatial

if TYPE_CHECKING:
    from destack.language import (
        EditOperation,
        EditType,
        Icon,
        IsSubject,
        Metric,
        NodeDefinitionReference,
        NodeReference,
        PropertyReference,
        Space,
        Value,
    )

# pyright: reportIncompatibleVariableOverride=false


@builtin_node(NodeType.EVENT, pretend_frozen=True, is_abstract=True)
class Event[N: Node = Node](IsSpatial, Node):
    """
    An Event is an immutable record of something happening to an Entity.
    """

    parent: Optional["Space"] = builtin_property_parent(
        is_readonly=True,
    )
    created_at: datetime = builtin_property(
        20,
        is_managed=True,
        is_eq=False,
        is_readonly=True,
        can_write=RoleType.SYSTEM,
    )
    created_by: Optional["IsSubject"] = builtin_property(
        21,
        default=None,
        is_managed=True,
        is_eq=False,
        is_readonly=True,
        node_space_from="self",
        can_write=RoleType.SYSTEM,
    )

    if TYPE_CHECKING:
        node: Optional[N] = None
        node_ptr: Optional[NodeReference] = None
    else:
        node: Optional["Node"] = builtin_property(101, description="The Node this Event is about.")


@builtin_node(NodeType.CUSTOM_EVENT_DEFINITION)
class CustomEventDefinition(
    IsSpatial,
    IsSourceable,
    IsCustomizable,
    Entity,
):
    """A CustomEventDefinition defines a kind of CustomEvent with custom Properties."""

    base_type: Optional["NodeDefinitionReference"] = builtin_property(40)
    base_traits: list["NodeDefinitionReference"] = builtin_property(41)
    is_abstract: bool = builtin_property(45, default=False)

    name: str = builtin_property(101, is_repr=True)
    icon: "Icon | None" = builtin_property(102)


@builtin_node(NodeType.SIGNAL, pretend_frozen=True, is_abstract=True)
class Signal(Event, IsExtensible):
    """
    A generic Signal of a CustomEventDefinition.
    More specific base Event types will be instanced of that base type instead.
    """

    definition: "CustomEventDefinition" = builtin_property(
        6,
        is_managed=True,
        is_readonly=True,
        description="The CustomEventDefinition this Signal is an instance of.",
    )
    if TYPE_CHECKING:
        definition_ptr: Optional[NodeReference] = None


@builtin_node(NodeType.EDIT_EVENT, pretend_frozen=True)
class EditEvent(Event):
    """A Event of an Edit. Only EditEvents of Entities are allowed."""

    type: "EditType" = builtin_property(100, is_repr=True)
    node: "Node" = builtin_property(101, is_repr=True)
    operation: "EditOperation | None" = builtin_property(102, is_repr=True)
    prop_ptr: "PropertyReference | None" = builtin_property(103, is_repr=True)
    key: "Value | None" = builtin_property(104, is_repr=True)  # for map operations
    if TYPE_CHECKING:
        node_ptr: NodeReference = UNSET

    value: "Value | None" = builtin_property(110)


@builtin_node(NodeType.MEASUREMENT_EVENT, is_abstract=True)
class MeasurementEvent(Event):
    """An Event that represents a Measurement."""

    definition: "Metric" = builtin_property(6, is_managed=True, can_write=None)
