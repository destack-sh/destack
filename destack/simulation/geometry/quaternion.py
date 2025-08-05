from typing import final

from destack.core import (
    Float32,
    ObjectStability,
    StructFrozen,
    StructType,
    declare_property,
    declare_struct,
)

# pyright: reportIncompatibleVariableOverride=false, reportIncompatibleMethodOverride=false


@declare_struct(
    StructType.QUATERNION,
    stability=ObjectStability.STATIC,
    frozen=True,
    is_final=True,
)
@final
class Quaternion(StructFrozen):
    """A quaternion."""

    x: Float32 = declare_property(
        101,
        is_repr=True,
        description="The x-coordinate of the Vector4.",
    )
    y: Float32 = declare_property(
        102,
        is_repr=True,
        description="The y-coordinate of the Vector4.",
    )
    z: Float32 = declare_property(
        103,
        is_repr=True,
        description="The z-coordinate of the Vector4.",
    )
    w: Float32 = declare_property(
        104,
        is_repr=True,
        description="The w-coordinate of the Vector4.",
    )
