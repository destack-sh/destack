from typing import TYPE_CHECKING, Any, Optional, Union

from ..builtin import (
    CascadeAction,
    EdgeType,
    Entity,
    EnumType,
    IsArchivable,
    IsDeletable,
    IsSourceable,
    IsTaggable,
    NodeType,
    PropertyType,
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
        Icon,
        IsCustomizable,
        Type,
        Value,
    )


# pyright: reportIncompatibleVariableOverride=false, reportIncompatibleMethodOverride=false


@builtin_node(NodeType.CUSTOM_PROPERTY)
class CustomProperty(
    IsTaggable,
    IsArchivable,
    IsDeletable,
    IsSourceable,
    Entity,
):
    """
    A CustomProperty is a custom attribute of an IsCustomizable or IsExtensible.
    """

    parent: Union["IsCustomizable", None] = builtin_property_parent()
    type: PropertyType = builtin_property(100, default=PropertyType.MEMBER)
    name: str = builtin_property(101, is_repr=True)
    icon: "Icon | None" = builtin_property(102)
    # key?

    # scalar
    cardinality: TypeCardinality = builtin_property(
        110, default=TypeCardinality.SCALAR, is_repr=True
    )
    scalar_type: ScalarType = builtin_property(111, is_repr=True)
    primitive_type: Optional[PrimitiveType] = builtin_property(112, is_repr=True)
    enum_type: Optional[EnumType] = builtin_property(113, is_repr=True)
    node_type: Optional[NodeType] = builtin_property(114, is_repr=True)
    struct_type: Optional[StructType] = builtin_property(115, is_repr=True)
    definition: Union[
        "CustomEntityDefinition",
        "CustomEventDefinition",
        "CustomEnumDefinition",
        "CustomStructDefinition",
        "CustomTraitDefinition",
        None,
    ] = builtin_property(116, is_repr=True)
    key_type: Optional["Type"] = builtin_property(117, is_repr=True)  # for maps

    # value
    value: Optional["Value"] = builtin_property(120)
    value_factory: Optional[ValueFactory] = builtin_property(121)

    # constraints
    collection_constraint: Optional["CollectionConstraint"] = builtin_property(130)
    string_constraint: Optional["StringConstraint"] = builtin_property(131)
    number_constraint: Optional["NumberConstraint"] = builtin_property(132)
    node_constraint: Optional["NodeConstraint"] = builtin_property(133)

    # relationship
    edge_type: Optional[EdgeType] = builtin_property(140)
    cascade: Optional[CascadeAction] = builtin_property(141)

    # flags
    is_required: bool | None = builtin_property(150)
    is_unique: bool | None = builtin_property(151)
    is_computed: bool | None = builtin_property(152)
    is_readonly: bool | None = builtin_property(153)

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
