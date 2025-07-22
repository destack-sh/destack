from typing import TYPE_CHECKING, Any, Optional

from ..builtin import (
    CascadeAction,
    EdgeType,
    Entity,
    EnumType,
    NodeType,
    PropertyType,
    builtin_node,
    builtin_property,
)
from .query import Condition, ConditionalType, Sort, SortType
from .type import (
    CollectionConstraint,
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
        Icon,
        Type,
        Value,
    )


# pyright: reportIncompatibleVariableOverride=false, reportIncompatibleMethodOverride=false


@builtin_node(NodeType.CUSTOM_PROPERTY)
class CustomProperty(
    Entity,
):
    """
    A CustomProperty is a custom attribute of an IsCustomizable or IsExtensible.
    """

    type: PropertyType = builtin_property(100, default=PropertyType.MEMBER)
    icon: "Icon | None" = builtin_property(102)

    # scalar
    cardinality: TypeCardinality = builtin_property(
        110, default=TypeCardinality.SCALAR, is_repr=True
    )
    scalar_type: ScalarType = builtin_property(111, is_repr=True)
    primitive_type: Optional[PrimitiveType] = builtin_property(112, is_repr=True)
    enum_type: Optional[EnumType] = builtin_property(113, is_repr=True)
    node_types: list[NodeType] = builtin_property(114, is_repr=True)
    struct_type: Optional[StructType] = builtin_property(115, is_repr=True)
    key_type: Optional["Type"] = builtin_property(117, is_repr=True)  # for maps

    # value
    value: Optional["Value"] = builtin_property(120)
    value_factory: Optional[ValueFactory] = builtin_property(121)

    # constraints
    collection_constraint: Optional["CollectionConstraint"] = builtin_property(130)
    string_constraint: Optional["StringConstraint"] = builtin_property(131)
    number_constraint: Optional["NumberConstraint"] = builtin_property(132)

    # relationship
    edge_type: Optional[EdgeType] = builtin_property(140)
    cascade: Optional[CascadeAction] = builtin_property(141)

    # flags
    is_required: bool | None = builtin_property(
        150,
        description="Whether this property must be set.",
    )
    is_unique: bool | None = builtin_property(
        151,
        description="Whether this property must have a unique value.",
    )
    is_computed: bool | None = builtin_property(
        152,
        description="Whether this property is computed.",
    )
    is_readonly: bool | None = builtin_property(
        153,
        description="Whether this property is read-only.",
    )
    is_main: bool | None = builtin_property(
        154,
        description="Whether this property is the main property of the object.",
    )

    def to_type(self) -> "Type":
        type = Type(
            cardinality=self.cardinality,
            scalar_type=self.scalar_type,
            primitive_type=self.primitive_type,
            enum_type=self.enum_type,
            node_types=self.node_types,
            struct_type=self.struct_type,
            key_type=self.key_type,
            value=self.value,
            value_factory=self.value_factory,
            collection_constraint=self.collection_constraint,
            string_constraint=self.string_constraint,
            number_constraint=self.number_constraint,
            is_required=self.is_required,
        )
        return type

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
