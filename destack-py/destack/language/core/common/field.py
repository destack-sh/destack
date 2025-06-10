from typing import TYPE_CHECKING, Optional, Union

from destack.pb2 import FieldData

from ..builtin import (
    BuiltinEnum,
    CascadeAction,
    EdgeType,
    EnumType,
    HasIcon,
    HasName,
    IsArchivable,
    IsDeletable,
    IsEnvironmental,
    IsInFolder,
    IsOrdered,
    IsSourceable,
    IsTemplatable,
    IsTracked,
    Node,
    NodeType,
    enum_,
    node_,
    property_,
    property_parent_,
)
from .query import IntoQuery
from .type import (
    UUID,
    CollectionConstraint,
    DefaultFactory,
    NodeConstraint,
    NumberConstraint,
    PrimitiveType,
    ScalarType,
    StringConstraint,
    StructType,
    TypeCardinality,
)

if TYPE_CHECKING:
    from destack.language import (
        CustomEntityDefinition,
        IsExtensible,
        NodeReference,
        Type,
        Value,
    )


# pyright: reportIncompatibleVariableOverride=false, reportIncompatibleMethodOverride=false


@enum_(EnumType.FIELD_TYPE)
class FieldType(BuiltinEnum):
    MEMBER = 1, "Member", "Member", "fas fa-arrow-down"
    INPUT = 2, "Input", "Input", "fas fa-arrow-down"
    OUTPUT = 3, "Output", "Output", "fas fa-arrow-up"


@node_(NodeType.FIELD)
class Field(
    HasName,
    HasIcon,
    IsEnvironmental,
    IsTracked,
    IsTemplatable,
    IsOrdered,
    IsDeletable,
    IsArchivable,
    IsInFolder,
    IntoQuery,
    IsSourceable,
    Node[FieldData],
):
    """
    A Field is a user-defined attribute.
    """

    parent: Union["IsExtensible", None] = property_parent_(node_is_customizable=True)
    type: FieldType = property_(30, default=FieldType.MEMBER)

    # scalar
    cardinality: TypeCardinality = property_(40, default=TypeCardinality.SCALAR, is_repr=True)
    scalar_type: ScalarType = property_(41, is_repr=True)
    primitive_type: Optional[PrimitiveType] = property_(42, is_repr=True)
    enum_type: Optional[EnumType] = property_(43, is_repr=True)
    node_type: Optional[NodeType] = property_(44, is_repr=True)
    node_definition: Optional["CustomEntityDefinition"] = property_(45, is_repr=True)
    struct_type: Optional[StructType] = property_(46, is_repr=True)
    base_type: Optional["Node"] = property_(47, is_repr=True)
    key_type: Optional["Type"] = property_(48, is_repr=True)  # for maps
    if TYPE_CHECKING:
        base_id: Optional[UUID] = None
        base_ptr: Optional["NodeReference"] = None

    # meta
    is_required: bool | None = property_(50)
    # is_external
    default: Optional["Value"] = property_(55)
    default_factory: Optional[DefaultFactory] = property_(56)

    # constraints
    collection_constraint: Optional["CollectionConstraint"] = property_(60)
    string_constraint: Optional["StringConstraint"] = property_(61)
    number_constraint: Optional["NumberConstraint"] = property_(62)
    node_constraint: Optional["NodeConstraint"] = property_(63)

    # relationship
    edge_type: Optional[EdgeType] = property_(70)
    cascade: Optional[CascadeAction] = property_(71)
