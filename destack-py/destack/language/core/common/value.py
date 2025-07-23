from typing import TYPE_CHECKING, Any

import structlog
from opentelemetry import trace

from destack.proto import ValueProto

from ..builtin import (
    ENCODERS,
    Cson,
    Encoding,
    Node,
    StructFrozen,
    StructType,
    active_session,
    builtin_property,
    builtin_property_runtime,
    builtin_struct,
)
from .type import ScalarType, Type, TypeCardinality, to_type

if TYPE_CHECKING:
    pass

# pyright: reportIncompatibleVariableOverride=false


logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)
type_ = type


@builtin_struct(StructType.VALUE, frozen=True)
class Value(StructFrozen[ValueProto]):
    """
    A generic Value of any Type.
    Values are used to represent any generic or user-provided data.
    """

    type: Type = builtin_property(100, is_repr=True)
    value: "Cson | None" = builtin_property(110, default=None)

    _unpacked_value: Any | None = builtin_property_runtime()

    def get(self) -> Any:
        """Get the unpacked value of this generic Value."""
        if self._unpacked_value is None:
            session = active_session()
            encoder = ENCODERS[Encoding.CSON]
            value_unpacked = encoder.unpack_value(self.type, self.value, session)
            object.__setattr__(self, "_unpacked_value", value_unpacked)
        return self._unpacked_value


def to_value(
    value_unpacked: Any,
    type: "Type | None" = None,
    is_required: bool = False,
    node_as_value: bool = False,
) -> Value:
    """
    Convert an arbitrary (legal) value to a Value.
    If Type isn't provided, it will be inferred from the value.
    """

    encoder = ENCODERS[Encoding.CSON]

    # infer type
    if type is None:
        if value_unpacked is None:
            raise ValueError("cannot infer type for None")
        type = to_type(value_unpacked, node_as_value=node_as_value)
        if is_required:
            type = type.clone(is_required=True)
    # coerce nodes into node references
    if type.scalar_type == ScalarType.NODE_REFERENCE:
        if type.cardinality == TypeCardinality.SCALAR and isinstance(value_unpacked, Node):
            value_unpacked = value_unpacked.to_ref()
        elif type.cardinality == TypeCardinality.LIST and value_unpacked:
            value_unpacked = [
                item.to_ref() if isinstance(item, Node) else item for item in value_unpacked
            ]
    # pack value
    value_packed = encoder.pack_value(value_unpacked, type)
    value = Value(type=type, value=value_packed, _unpacked_value=value_unpacked)
    return value
