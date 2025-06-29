from typing import TYPE_CHECKING

from ..builtin import (
    UNSET,
    Event,
    IsTaggable,
    Node,
    NodeType,
    builtin_node,
    property_,
)

if TYPE_CHECKING:
    from destack.language import (
        CustomProperty,
        EditOperation,
        EditType,
        NodeReference,
        PropertyReference,
        Value,
    )

# pyright: reportIncompatibleVariableOverride=false


@builtin_node(NodeType.EDIT_EVENT, pretend_frozen=True)
class EditEvent(
    Event,
    IsTaggable,
    Node,
):
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
