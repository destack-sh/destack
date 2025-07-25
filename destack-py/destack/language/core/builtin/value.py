from typing import TYPE_CHECKING, Any

from destack.utils.log import get_logger
from destack.utils.telemetry import get_tracer

from .common import Cson, Encoding
from .const import ENCODERS, active_session
from .property import builtin_property, builtin_property_runtime
from .struct import StructFrozen, StructType, builtin_struct
from .type import BasicType, ScalarType, Type, TypeCardinality

if TYPE_CHECKING:
    pass

# pyright: reportIncompatibleVariableOverride=false


logger = get_logger(__name__)
tracer = get_tracer(__name__)
type_ = type


@builtin_struct(StructType.VALUE, frozen=True)
class Value(StructFrozen):
    """
    A generic Value of any Type.
    Values are used to represent any generic or user-provided data.
    """

    type: BasicType = builtin_property(100, is_repr=True)
    # nocheckin: Value.value should be KOMPAKT
    value: Cson | None = builtin_property(110, default=None)

    _unpacked_value: Any | None = builtin_property_runtime()

    def get(self) -> Any:
        """Get the unpacked value of this generic Value."""
        if self._unpacked_value is None:
            session = active_session()
            encoder = ENCODERS[Encoding.CSON]
            value_unpacked = encoder.unpack_value(self.type, self.value, session)
            object.__setattr__(self, "_unpacked_value", value_unpacked)
        return self._unpacked_value

    @classmethod
    def wrap(
        cls,
        value_unpacked: Any,
        type: "Type | None" = None,
        is_required: bool = False,
        node_as_value: bool = False,
    ) -> "Value":
        """
        Convert an arbitrary (legal) value to a Value.
        If Type isn't provided, it will be inferred from the value.
        """
        from .node import Node

        encoder = ENCODERS[Encoding.CSON]

        # infer type
        if type is None:
            if value_unpacked is None:
                raise ValueError("cannot infer type for None")
            type = Type.infer(value_unpacked, node_as_value=node_as_value)
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
