from typing import TYPE_CHECKING

from fastuuid import UUID

from destack.pb2 import (
    CustomEventData,
    CustomEventDefinitionData,
    EditEventData,
)

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
    node_,
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


@node_(NodeType.EDIT_EVENT, pretend_frozen=True)
class EditEvent(
    Event,
    IsTaggable,
    Node[EditEventData],
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


@node_(NodeType.CUSTOM_EVENT_DEFINITION)
class CustomEventDefinition(
    Spatial,
    Entity,
    HasName,
    IsSourceable,
    Node[CustomEventDefinitionData],
):
    """A CustomEventDefinition defines a kind of CustomEvent."""

    pass


@node_(NodeType.CUSTOM_EVENT, pretend_frozen=True)
class CustomEvent(
    Event,
    Node[CustomEventData],
):
    """An instance of a CustomEventDefinition."""

    definition: CustomEventDefinition = property_(
        40, description="The CustomEventDefinition this CustomEvent is an instance of."
    )
