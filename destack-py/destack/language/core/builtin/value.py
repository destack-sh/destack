from typing import TYPE_CHECKING, Any

from destack.utils.log import get_logger
from destack.utils.telemetry import get_tracer

from .property import builtin_property
from .struct import Struct, StructType, builtin_struct
from .type import ScalarType, Type, TypeCardinality

if TYPE_CHECKING:
    pass

# pyright: reportIncompatibleVariableOverride=false


logger = get_logger(__name__)
tracer = get_tracer(__name__)
type_ = type


@builtin_struct(StructType.VALUE)
class Value(Struct):
    """
    A generic Value of any Type.
    Values are used to represent any generic data.
    """

    type: Type = builtin_property(
        100,
        is_repr=True,
        description="The Type of the Value.",
    )
    value: Any | None = builtin_property(
        200,
        is_repr=True,
        description="The generic Value.",
    )

    def get(self):
        return self.value

    def set(self, value: Any):
        self.type = Type.infer(value)
        self.value = value

    @classmethod
    def wrap(
        cls,
        value: Any,
        type: "Type | None" = None,
        node_as_value: bool = False,
    ) -> "Value":
        """
        Convert an arbitrary (legal) value to a Value.
        If Type isn't provided, it will be inferred from the value.
        """
        from .node import Node

        # infer type
        if type is None:
            if value is None:
                raise ValueError("cannot infer type for None")
            type = Type.infer(value, node_as_value=node_as_value)
        # coerce nodes into node references
        if type.scalar_type == ScalarType.NODE_REFERENCE:
            if type.cardinality == TypeCardinality.SCALAR and isinstance(value, Node):
                value = value.to_ref()
            elif type.cardinality == TypeCardinality.LIST and value:
                value = [item.to_ref() if isinstance(item, Node) else item for item in value]

        return Value(type=type, value=value)
