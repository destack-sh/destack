from typing import TYPE_CHECKING, Any, final

from ..builtin import (
    UNSET,
    PropertyDeclaration,
    ReferenceType,
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
    from destack import Condition, Sort

type_ = type


@declare_struct(
    StructType.PROPERTY_DEFINITION,
    is_final=True,
)
@final
class PropertyDefinition(Definition):
    """Definition of a builtin Property."""

    id: UInt8 = declare_property(2, is_repr=True, tag=None)
    type: Type = declare_property(100, tag=None)
    tag: UInt8 | None = declare_property(109, tag=None)

    # defaults
    default_value: Value | None = declare_property(120, tag=None)
    default_factory: ValueFactory | None = declare_property(121, tag=None)

    # relationships
    reference_type: ReferenceType | None = declare_property(130, tag=None)

    # property flags
    is_readonly: bool = declare_property(
        142,
        is_repr=True,
        description="Whether this Property is read-only.",
        tag=None,
    )
    is_repr: bool = declare_property(
        143,
        description="Whether this Property is included in the object's string representation.",
        tag=None,
    )
    is_hash: bool = declare_property(
        144,
        description="Whether this Property is included in the object's hash.",
        tag=None,
    )
    is_eq: bool = declare_property(
        145,
        description="Whether this Property is included in the object's equality check.",
        tag=None,
    )
    is_managed: bool = declare_property(
        146,
        description="Whether this Property is internally managed by the system.",
        tag=None,
    )
    is_static: bool = declare_property(
        147,
        description="Whether this Property is static (only exists once per object).",
        tag=None,
    )
    is_runtime_only: bool = declare_property(
        148,
        description="Whether this Property is only set at runtime (is not wired).",
        tag=None,
    )
    is_interned: bool = declare_property(
        149,
        description="Whether this Property is interned (stored in a shared pool).",
        tag=None,
    )

    @classmethod
    def from_declaration(cls, prop: PropertyDeclaration) -> "PropertyDefinition":
        """Create PropertyDefinition from a Property."""
        from .node import Node
        from .struct import Struct

        type = prop.type.to_type()
        object_cls = (
            prop.component
            if isinstance(prop.component, type_) and issubclass(prop.component, (Node, Struct))
            else Node
        )

        # resolve taggings locally
        from .object import resolve_tagging

        return cls(
            # meta
            id=prop.id,
            name=prop.name,
            description=prop.description or "",
            tag=resolve_tagging(object_cls, prop.tag).id if prop.tag else None,
            # type
            type=type,
            default_value=Value.of(prop.default_value) if prop.default_value is not UNSET else None,
            default_factory=prop.default_factory,
            # node
            reference_type=prop.reference_type,
            # flags
            is_readonly=prop.is_readonly,
            is_repr=prop.is_repr,
            is_hash=prop.is_hash,
            is_eq=prop.is_eq,
            is_managed=prop.is_managed,
            is_static=prop.is_static,
            is_runtime_only=prop.is_runtime_only,
            is_interned=prop.is_interned,
        )

    @declare_method(102)
    def to_ref(self) -> PropertyReference:
        """Convert to a PropertyReference."""
        raise NotImplementedError

    @declare_method(110)
    def eq(self, value: Any) -> "Condition":
        """Create a Condition that checks if this Property is equal to a value."""
        raise NotImplementedError

    @declare_method(111)
    def neq(self, value: Any) -> "Condition":
        """Create a Condition that checks if this Property is not equal to a value."""
        raise NotImplementedError

    @declare_method(112)
    def gt(self, value: Any) -> "Condition":
        """Create a Condition that checks if this Property is greater than a value."""
        raise NotImplementedError

    @declare_method(113)
    def gte(self, value: Any) -> "Condition":
        """Create a Condition that checks if this Property is greater than or equal to a value."""
        raise NotImplementedError

    @declare_method(114)
    def lt(self, value: Any) -> "Condition":
        """Create a Condition that checks if this Property is less than a value."""
        raise NotImplementedError

    @declare_method(115)
    def lte(self, value: Any) -> "Condition":
        """Create a Condition that checks if this Property is less than or equal to a value."""
        raise NotImplementedError

    @declare_method(116)
    def starts_with(self, value: str) -> "Condition":
        """Create a Condition that checks if this Property starts with a value."""
        raise NotImplementedError

    @declare_method(117)
    def ends_with(self, value: str) -> "Condition":
        """Create a Condition that checks if this Property ends with a value."""
        raise NotImplementedError

    @declare_method(118)
    def in_(self, *values: Any) -> "Condition":
        """Create a Condition that checks if this Property is in a list of values."""
        raise NotImplementedError

    @declare_method(119)
    def not_in(self, *values: Any) -> "Condition":
        """Create a Condition that checks if this Property is not in a list of values."""
        raise NotImplementedError

    @declare_method(120)
    def exists(self) -> "Condition":
        """Create a Condition that checks if this Property exists."""
        raise NotImplementedError

    @declare_method(121)
    def is_not_none(self) -> "Condition":
        """Create a Condition that checks if this Property is not None."""
        raise NotImplementedError

    @declare_method(122)
    def not_exists(self) -> "Condition":
        """Create a Condition that checks if this Property does not exist."""
        raise NotImplementedError

    @declare_method(123)
    def is_none(self) -> "Condition":
        """Create a Condition that checks if this Property is None."""
        raise NotImplementedError

    @declare_method(124)
    def asc(self) -> "Sort":
        """Create a Sort that sorts this Property in ascending order."""
        raise NotImplementedError

    @declare_method(125)
    def desc(self) -> "Sort":
        """Create a Sort that sorts this Property in descending order."""
        raise NotImplementedError
