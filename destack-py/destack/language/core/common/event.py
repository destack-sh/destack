from typing import TYPE_CHECKING

from destack.proto import (
    CustomEventDefinitionProto,
    CustomEventProto,
    EditEventProto,
)
from destack.utils.uuid import UUID

from ..builtin import (
    UNSET,
    Entity,
    Event,
    HasName,
    IsSourceable,
    IsTaggable,
    Node,
    NodeType,
    Property,
    Spatial,
    builtin_node,
    property_,
)

if TYPE_CHECKING:
    from destack.language import (
        EditOperation,
        EditType,
        Field,
        NodeReference,
        PropertyReference,
        Value,
    )

# pyright: reportIncompatibleVariableOverride=false


@builtin_node(NodeType.EDIT_EVENT, pretend_frozen=True)
class EditEvent(
    Event,
    IsTaggable,
    Node[EditEventProto],
):
    """A Event of an Edit. Only EditEvents of Entities are allowed."""

    # key
    type: "EditType" = property_(30, is_repr=True)
    operation: "EditOperation | None" = property_(31, is_repr=True)
    node: "Node" = property_(35, is_repr=True)
    prop: "Property | None" = property_(36, is_repr=True)
    field: "Field | None" = property_(37, is_repr=True)  # for IsExtensible.value
    key: "Value | None" = property_(38, is_repr=True)  # for map operations
    if TYPE_CHECKING:
        node_id: UUID = UNSET
        node_ptr: NodeReference = UNSET
        node_type: NodeType = UNSET
        field_id: UUID | None = None
        field_ptr: NodeReference | None = None
        property_ptr: PropertyReference | None = None

    # value
    value: "Value | None" = property_(40)


@builtin_node(NodeType.CUSTOM_EVENT_DEFINITION)
class CustomEventDefinition(
    Spatial,
    Entity,
    HasName,
    IsSourceable,
    Node[CustomEventDefinitionProto],
):
    """A CustomEventDefinition defines a kind of CustomEvent."""

    pass


@builtin_node(NodeType.CUSTOM_EVENT, pretend_frozen=True)
class CustomEvent(
    Event,
    Node[CustomEventProto],
):
    """An instance of a CustomEventDefinition."""

    definition: CustomEventDefinition = property_(
        40, description="The CustomEventDefinition this CustomEvent is an instance of."
    )
