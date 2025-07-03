from datetime import datetime
from typing import TYPE_CHECKING, Optional

from destack.utils.uuid import UUID

from .common import RoleType
from .const import UNSET
from .entity import Entity
from .node import Node, NodeType, builtin_node
from .property import builtin_property, builtin_property_parent
from .trait import IsCustomizable, IsExtensible, IsSourceable, IsSpatial

if TYPE_CHECKING:
    from destack.language import (
        Aggregation,
        ChangeDebounce,
        Condition,
        Edit,
        EditOperation,
        EditType,
        Expression,
        Icon,
        IsSubject,
        Join,
        Metric,
        NodeDefinitionReference,
        NodeReference,
        Origin,
        PropertyReference,
        Query,
        QueryType,
        Select,
        Snapshot,
        Sort,
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
    """A recorded Edit of an Entity."""

    type: "EditType" = builtin_property(100, is_repr=True)
    node: "Entity" = builtin_property(101, is_repr=True)
    operation: "EditOperation | None" = builtin_property(102, is_repr=True)
    attribute: "PropertyReference | None" = builtin_property(103, is_repr=True)
    key: "Value | None" = builtin_property(104, is_repr=True)  # for map operations
    if TYPE_CHECKING:
        node_ptr: NodeReference = UNSET
    value: "Value | None" = builtin_property(110)

    undo: Optional["Edit"] = builtin_property(
        120,
        description="The inverse Edit *if* it cannot be unambiguously derived from the Edit).",
    )
    snapshot: Optional["Snapshot"] = builtin_property(121)
    ancestors_ids: list[UUID] = builtin_property(122)


@builtin_node(NodeType.CHANGE_EVENT, pretend_frozen=True)
class ChangeEvent(Event):
    """A recorded Change."""

    name: str | None = builtin_property(102, is_repr=True)
    origin: "Origin | None" = builtin_property(103)
    debounce: "ChangeDebounce | None" = builtin_property(104)

    edits_ids: list[UUID] = builtin_property(120)


@builtin_node(NodeType.QUERY_EVENT, pretend_frozen=True)
class QueryEvent(Event):
    """A recorded Query."""

    type: "QueryType" = builtin_property(100, is_repr=True)
    name: str = builtin_property(
        102,
        description="Name for this subquery. Must be unique within the parent Query.",
        is_repr=True,
    )
    definition: "NodeDefinitionReference" = builtin_property(103, is_repr=True)
    join: Optional["Join"] = builtin_property(
        104, description="Relative to parent Query.", is_repr=True
    )
    select: Optional["Select"] = builtin_property(105, is_repr=True)
    subqueries: list["Query"] = builtin_property(106, is_repr=True)
    # is_live/refreshing/routing/area/...

    # content
    where: Optional["Condition"] = builtin_property(110, is_repr=True)
    having: Optional["Condition"] = builtin_property(111, is_repr=True)
    group_by: list["Expression"] = builtin_property(112, is_repr=True)
    aggregation: Optional["Aggregation"] = builtin_property(113, is_repr=True)
    sort: list["Sort"] = builtin_property(114, is_repr=True)

    # pagination
    limit: Optional[int] = builtin_property(120, is_repr=True)
    offset: Optional[int] = builtin_property(121, is_repr=True)


@builtin_node(NodeType.MEASUREMENT_EVENT, is_abstract=True)
class MeasurementEvent(Event):
    """An Event that represents a Measurement."""

    definition: "Metric" = builtin_property(6, is_managed=True, can_write=None)
