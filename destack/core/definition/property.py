from typing import TYPE_CHECKING, Any, cast, final

from destack.registry import OBJECT_DEFINITION_REFERENCE_BY_CLASS

from ..builtin import (
    CascadeAction,
    EdgeType,
    Node,
    PropertyDeclaration,
    Struct,
    StructType,
    UInt8,
    ValueFactory,
    declare_method,
    declare_property,
    declare_struct,
)
from ..common import PropertyReference, Type, Value
from .definition import Definition

if TYPE_CHECKING:
    from destack import Condition, ObjectDefinitionReference, Sort

type_ = type


@declare_struct(
    StructType.PROPERTY_DEFINITION,
    frozen=True,
    is_final=True,
)
@final
class PropertyDefinition(Definition):
    """Definition of a builtin Property."""

    id: UInt8 = declare_property(2, is_repr=True)
    type: Type = declare_property(100)
    object: "ObjectDefinitionReference" = declare_property(
        104, description="The object that this property is defined on."
    )
    original_object: "ObjectDefinitionReference" = declare_property(
        105, description="The original object that this property was defined on."
    )
    taggings: list[UInt8] = declare_property(109)

    # defaults
    default_value: Value | None = declare_property(120)
    default_factory: ValueFactory | None = declare_property(121)

    # relationships
    edge_type: EdgeType | None = declare_property(130)
    cascade: CascadeAction | None = declare_property(131)

    # property flags
    is_identity: bool = declare_property(
        140,
        is_repr=True,
        description="""\
Whether this Property is part of the object's identity.
 (And thus is always required, in every instance including partials; only for Nodes.)
""",
    )
    is_unique: bool = declare_property(
        141,
        is_repr=True,
        description="Whether this Property must have a unique value.",
    )
    is_readonly: bool = declare_property(
        142,
        is_repr=True,
        description="Whether this Property is read-only.",
    )
    is_repr: bool = declare_property(143)
    is_hash: bool = declare_property(144)
    is_eq: bool = declare_property(145)
    is_internal: bool = declare_property(146)

    @classmethod
    def from_declaration(cls, prop: PropertyDeclaration) -> "PropertyDefinition":
        """Create PropertyDefinition from a Property."""
        type = prop.type.to_type()
        object_ref = OBJECT_DEFINITION_REFERENCE_BY_CLASS[prop.component]
        original_object_ref = OBJECT_DEFINITION_REFERENCE_BY_CLASS.get(
            prop.original_component, object_ref
        )
        object_cls = prop.component if issubclass(prop.component, (Node, Struct)) else Node

        # resolve taggings locally
        from .object import resolve_tagging

        taggings = [
            resolve_tagging(cast(type_["Node"] | type_["Struct"], object_cls), tag).id
            for tag in prop.tags
        ]

        return cls(
            # meta
            id=prop.id,
            name=prop.name,
            description=prop.description,
            taggings=taggings,
            object=object_ref,
            original_object=original_object_ref,
            # type
            type=type,
            default_value=prop.default_value,
            default_factory=prop.default_factory,
            # node
            edge_type=prop.edge_type,
            cascade=prop.cascade,
            # flags
            is_identity=prop.is_identity,
            is_unique=prop.is_unique,
            is_readonly=prop.is_readonly,
            is_repr=prop.is_repr,
            is_hash=prop.is_hash,
            is_eq=prop.is_eq,
            is_internal=prop.is_internal,
        )

    @declare_method(102)
    def to_ref(self) -> PropertyReference:
        """Convert to a PropertyReference."""
        ...

    @declare_method(110)
    def eq(self, value: Any) -> "Condition":
        """Create a Condition that checks if this Property is equal to a value."""
        ...

    @declare_method(111)
    def neq(self, value: Any) -> "Condition":
        """Create a Condition that checks if this Property is not equal to a value."""
        ...

    @declare_method(112)
    def gt(self, value: Any) -> "Condition":
        """Create a Condition that checks if this Property is greater than a value."""
        ...

    @declare_method(113)
    def gte(self, value: Any) -> "Condition":
        """Create a Condition that checks if this Property is greater than or equal to a value."""
        ...

    @declare_method(114)
    def lt(self, value: Any) -> "Condition":
        """Create a Condition that checks if this Property is less than a value."""
        ...

    @declare_method(115)
    def lte(self, value: Any) -> "Condition":
        """Create a Condition that checks if this Property is less than or equal to a value."""
        ...

    @declare_method(116)
    def starts_with(self, value: str) -> "Condition":
        """Create a Condition that checks if this Property starts with a value."""
        ...

    @declare_method(117)
    def ends_with(self, value: str) -> "Condition":
        """Create a Condition that checks if this Property ends with a value."""
        ...

    @declare_method(118)
    def in_(self, *values: Any) -> "Condition":
        """Create a Condition that checks if this Property is in a list of values."""
        ...

    @declare_method(119)
    def not_in(self, *values: Any) -> "Condition":
        """Create a Condition that checks if this Property is not in a list of values."""
        ...

    @declare_method(120)
    def exists(self) -> "Condition":
        """Create a Condition that checks if this Property exists."""
        ...

    @declare_method(121)
    def is_not_none(self) -> "Condition":
        """Create a Condition that checks if this Property is not None."""
        ...

    @declare_method(122)
    def not_exists(self) -> "Condition":
        """Create a Condition that checks if this Property does not exist."""
        ...

    @declare_method(123)
    def is_none(self) -> "Condition":
        """Create a Condition that checks if this Property is None."""
        ...

    @declare_method(124)
    def asc(self) -> "Sort":
        """Create a Sort that sorts this Property in ascending order."""
        ...

    @declare_method(125)
    def desc(self) -> "Sort":
        """Create a Sort that sorts this Property in descending order."""
        ...
