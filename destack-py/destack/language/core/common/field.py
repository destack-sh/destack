from typing import TYPE_CHECKING, Optional, Union

from destack.proto import FieldProto

from ..builtin import (
    CascadeAction,
    EdgeType,
    Entity,
    Enum,
    EnumType,
    HasIcon,
    HasName,
    IsDeletable,
    IsSourceable,
    IsTaggable,
    Node,
    NodeType,
    Spatial,
    builtin_enum,
    builtin_node,
    property_,
    property_parent_,
)
from .query import IntoQuery
from .type import (
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


@builtin_enum(EnumType.FIELD_TYPE)
class FieldType(Enum):
    MEMBER = 1, "Member", "Member", "fas fa-arrow-down"
    INPUT = 2, "Input", "Input", "fas fa-arrow-down"
    OUTPUT = 3, "Output", "Output", "fas fa-arrow-up"


@builtin_node(NodeType.FIELD)
class Field(
    Spatial,
    Entity,
    HasName,
    HasIcon,
    IsTaggable,
    IsDeletable,
    IsSourceable,
    IntoQuery,
    Node[FieldProto],
):
    """
    A Field is a custom attribute of a CustomStructDefinition or an IsExtensible.
    """

    parent: Union["IsExtensible", "Field", None] = property_parent_(node_is_customizable=True)
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
        base_ptr: Optional["NodeReference"] = None

    # meta
    is_required: bool | None = property_(50)
    # is_external
    default_value: Optional["Value"] = property_(55)
    default_factory: Optional[DefaultFactory] = property_(56)

    # constraints
    collection_constraint: Optional["CollectionConstraint"] = property_(60)
    string_constraint: Optional["StringConstraint"] = property_(61)
    number_constraint: Optional["NumberConstraint"] = property_(62)
    node_constraint: Optional["NodeConstraint"] = property_(63)

    # relationship
    edge_type: Optional[EdgeType] = property_(70)
    cascade: Optional[CascadeAction] = property_(71)
