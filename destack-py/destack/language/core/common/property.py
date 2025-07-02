from typing import TYPE_CHECKING, Any, Optional, Union

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
    IsSpatial,
    IsTaggable,
    NodeType,
    builtin_enum,
    builtin_node,
    builtin_property,
    builtin_property_parent,
)
from .query import Condition, ConditionalType, Sort, SortType
from .type import (
    CollectionConstraint,
    NodeConstraint,
    NumberConstraint,
    PrimitiveType,
    ScalarType,
    StringConstraint,
    StructType,
    TypeCardinality,
    ValueFactory,
)

if TYPE_CHECKING:
    from destack.language import (
        CustomEntityDefinition,
        CustomEnumDefinition,
        CustomEventDefinition,
        CustomStructDefinition,
        CustomTraitDefinition,
        IsCustomizable,
        IsExtensible,
        Type,
        Value,
    )


# pyright: reportIncompatibleVariableOverride=false, reportIncompatibleMethodOverride=false


@builtin_enum(EnumType.CUSTOM_PROPERTY_TYPE)
class CustomPropertyType(Enum):
    MEMBER = 1, "Member", "Member", "fas fa-arrow-down"
    INPUT = 2, "Input", "Input", "fas fa-arrow-down"
    OUTPUT = 3, "Output", "Output", "fas fa-arrow-up"


@builtin_node(NodeType.CUSTOM_PROPERTY)
class CustomProperty(
    IsSpatial,
    HasName,
    HasIcon,
    IsTaggable,
    IsDeletable,
    IsSourceable,
    Entity,
):
    """
    A CustomProperty is a custom attribute of an IsCustomizable or IsExtensible.
    """

    parent: Union["IsCustomizable", "IsExtensible", "CustomProperty", None] = (
        builtin_property_parent(node_is_extensible=True)
    )
    type: CustomPropertyType = builtin_property(30, default=CustomPropertyType.MEMBER)

    # scalar
    cardinality: TypeCardinality = builtin_property(
        40, default=TypeCardinality.SCALAR, is_repr=True
    )
    scalar_type: ScalarType = builtin_property(41, is_repr=True)
    primitive_type: Optional[PrimitiveType] = builtin_property(42, is_repr=True)
    enum_type: Optional[EnumType] = builtin_property(43, is_repr=True)
    node_type: Optional[NodeType] = builtin_property(44, is_repr=True)
    struct_type: Optional[StructType] = builtin_property(45, is_repr=True)
    definition: Union[
        "CustomEntityDefinition",
        "CustomEventDefinition",
        "CustomEnumDefinition",
        "CustomStructDefinition",
        "CustomTraitDefinition",
        None,
    ] = builtin_property(46, is_repr=True)
    key_type: Optional["Type"] = builtin_property(48, is_repr=True)  # for maps

    # meta
    value: Optional["Value"] = builtin_property(50)
    value_factory: Optional[ValueFactory] = builtin_property(51)

    # constraints
    collection_constraint: Optional["CollectionConstraint"] = builtin_property(60)
    string_constraint: Optional["StringConstraint"] = builtin_property(61)
    number_constraint: Optional["NumberConstraint"] = builtin_property(62)
    node_constraint: Optional["NodeConstraint"] = builtin_property(63)

    # relationship
    edge_type: Optional[EdgeType] = builtin_property(70)
    cascade: Optional[CascadeAction] = builtin_property(71)

    # flags
    is_required: bool | None = builtin_property(80)
    is_unique: bool | None = builtin_property(81)
    is_computed: bool | None = builtin_property(82)
    is_readonly: bool | None = builtin_property(83)
    is_static: bool | None = builtin_property(84)

    def eq(self, value: Any) -> Condition:
        if value is None:
            return self.not_exists()
        return Condition.of(self, ConditionalType.EQUALS, value=value)

    def neq(self, value: Any) -> Condition:
        if value is None:
            return self.exists()
        return Condition.of(self, ConditionalType.NOT_EQUALS, value=value)

    def gt(self, value: Any) -> Condition:
        return Condition.of(self, ConditionalType.GREATER_THAN, value=value)

    def gte(self, value: Any) -> Condition:
        return Condition.of(self, ConditionalType.GREATER_THAN_OR_EQUALS, value=value)

    def lt(self, value: Any) -> Condition:
        return Condition.of(self, ConditionalType.LESS_THAN, value=value)

    def lte(self, value: Any) -> Condition:
        return Condition.of(self, ConditionalType.LESS_THAN_OR_EQUALS, value=value)

    def starts_with(self, value: str) -> Condition:
        return Condition.of(self, ConditionalType.STARTS_WITH, value=value)

    def ends_with(self, value: str) -> Condition:
        return Condition.of(self, ConditionalType.ENDS_WITH, value=value)

    def in_(self, *values: Any) -> Condition:
        return Condition.of(self, ConditionalType.IN, value=values)

    def not_in(self, *values: Any) -> Condition:
        return Condition.of(self, ConditionalType.NOT_IN, value=values)

    def exists(self) -> Condition:
        return Condition.of(self, ConditionalType.EXISTS)

    def is_not_none(self) -> Condition:
        return Condition.of(self, ConditionalType.EXISTS)

    def not_exists(self) -> "Condition":
        return Condition.of(self, ConditionalType.NOT_EXISTS)

    def is_none(self) -> "Condition":
        return Condition.of(self, ConditionalType.NOT_EXISTS)

    def asc(self) -> "Sort":
        return Sort.of(self, SortType.ASCENDING)

    def desc(self) -> "Sort":
        return Sort.of(self, SortType.DESCENDING)
