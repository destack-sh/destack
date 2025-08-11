from typing import TYPE_CHECKING, Any, final

from ..builtin import Node, ObjectStability, Struct, StructType, declare_property, declare_struct
from .type import ReferenceType, ScalarType, Type, TypeCardinality, infer_type

if TYPE_CHECKING:
    pass


type_ = type


@declare_struct(
    StructType.VALUE,
    stability=ObjectStability.STATIC,
    is_final=True,
)
@final
class Value(Struct):
    """
    A generic Value of any Type.
    Values are used to represent any generic data.
    """

    type: Type = declare_property(
        100,
        is_repr=True,
        description="The Type of the Value.",
    )
    value: Any | None = declare_property(
        200,
        is_repr=True,
        description="The generic Value.",
    )

    def get(self):
        return self.value

    def set(self, value: Any):
        self.type = Type.of(value)
        self.value = value

    @classmethod
    def of(
        cls,
        value: Any,
        type: "Type | None" = None,
        reference_type: ReferenceType | None = ReferenceType.REGULAR,
    ) -> "Value":
        """
        Convert an arbitrary (legal) value to a Value.
        If Type isn't provided, it will be inferred from the value.
        """
        # infer type
        if type is None:
            type = infer_type(value, reference_type=reference_type)
        # coerce nodes into node references
        if type.scalar_type == ScalarType.NODE_REFERENCE:
            if type.cardinality == TypeCardinality.SCALAR and isinstance(value, Node):
                value = value.to_ref()
            elif type.cardinality == TypeCardinality.LIST and value:
                value = [item.to_ref() if isinstance(item, Node) else item for item in value]

        return Value(type=type, value=value)
