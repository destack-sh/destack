from typing import TYPE_CHECKING

from fastuuid import UUID

from destack.pb2 import (
    ChangeEventData,
    CustomEventData,
    CustomEventDefinitionData,
    EditEventData,
    QueryEventData,
)

from ..builtin import (
    UNSET,
    HasName,
    IsEnvironmental,
    IsEvent,
    IsInFolder,
    IsSourceable,
    Node,
    NodeType,
    Property,
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
    IsEnvironmental,
    IsEvent,
    IsInFolder,
    Node[EditEventData],
):
    """A Event of an Edit."""

    # key
    type: "EditType" = property_(30, is_repr=True)
    operation: "EditOperation | None" = property_(31, is_repr=True)
    node: "Node" = property_(32, is_repr=True)
    prop: "Property | None" = property_(33, is_repr=True)
    field: "Field | None" = property_(34, is_repr=True)  # for IsExtensible.value
    key: "Value | None" = property_(35, is_repr=True)  # for map operations
    if TYPE_CHECKING:
        node_id: UUID = UNSET
        node_ptr: NodeReference = UNSET
        node_type: NodeType = UNSET
        field_id: UUID | None = None
        field_ptr: NodeReference | None = None
        property_ptr: PropertyReference | None = None

    # value
    value: "Value | None" = property_(40)


@node_(NodeType.CHANGE_EVENT, pretend_frozen=True)
class ChangeEvent(
    IsEnvironmental,
    IsEvent,
    IsInFolder,
    Node[ChangeEventData],
):
    """A Event of a Change."""

    pass


@node_(NodeType.QUERY_EVENT, pretend_frozen=True)
class QueryEvent(
    IsEnvironmental,
    IsEvent,
    IsInFolder,
    Node[QueryEventData],
):
    """A Event of a Query."""

    pass


@node_(NodeType.CUSTOM_EVENT_DEFINITION)
class CustomEventDefinition(
    HasName,
    IsSourceable,
    IsInFolder,
    Node[CustomEventDefinitionData],
):
    """A CustomEventDefinition defines a kind of CustomEvent."""

    pass


@node_(NodeType.CUSTOM_EVENT, pretend_frozen=True)
class CustomEvent(IsInFolder, IsEvent, Node[CustomEventData]):
    """An instance of a CustomEventDefinition."""

    definition: CustomEventDefinition = property_(
        40, description="The CustomEventDefinition this CustomEvent is an instance of."
    )
