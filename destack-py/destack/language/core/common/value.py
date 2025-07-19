from typing import TYPE_CHECKING, Any, cast

import structlog
from opentelemetry import trace

from destack.proto import ValueProto

from ..builtin import (
    Cson,
    Node,
    StructFrozen,
    StructType,
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

    # NOTE :Performance: also support Value encoding in our binary/proto format?

    type: Type = builtin_property(100, is_repr=True)
    value: "Cson | None" = builtin_property(110, default=None)

    _unpacked: Any | None = builtin_property_runtime()

    def unpack[T = Any](self, type: type_[T] | None = None) -> T:
        """Get the unpacked value of this generic Value."""
        if self._unpacked is None:
            from .cson import unpack_cson

            value_unpacked = unpack_cson(self.value, self.type)
            object.__setattr__(self, "_unpacked", value_unpacked)
        if type is not None and not isinstance(self._unpacked, type):
            raise TypeError(f"expected {type!r}, got {self._unpacked!r}")
        return cast(T, self._unpacked)


def to_value(value_unpacked: Any, type: "Type | None" = None, node_as_value: bool = False) -> Value:
    """
    Convert an arbitrary (legal) value to a Value.
    If Type isn't provided, it will be inferred from the value.
    """
    from .cson import pack_cson

    # infer type
    if type is None:
        if value_unpacked is None:
            raise ValueError("cannot infer type for None")
        type = to_type(value_unpacked, node_as_value=node_as_value)
    # coerce nodes into node references
    if type.scalar_type == ScalarType.NODE_REFERENCE:
        if type.cardinality == TypeCardinality.SCALAR and isinstance(value_unpacked, Node):
            value_unpacked = value_unpacked.to_ref()
        elif type.cardinality == TypeCardinality.LIST and value_unpacked:
            value_unpacked = [
                item.to_ref() if isinstance(item, Node) else item for item in value_unpacked
            ]
    # pack value
    value_packed = pack_cson(value_unpacked, type)
    value = Value(type=type, value=value_packed, _unpacked=value_unpacked)
    return value
