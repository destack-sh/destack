from typing import final

from destack.core import (
    Float32,
    ObjectStability,
    Struct,
    StructType,
    declare_constant,
    declare_property,
    declare_struct,
)

# pyright: reportIncompatibleVariableOverride=false, reportIncompatibleMethodOverride=false


@declare_struct(
    StructType.QUATERNION,
    stability=ObjectStability.STATIC,
    is_final=True,
)
@final
class Quaternion(Struct):
    """A quaternion."""

    ZERO = declare_constant(
        100,
        value=lambda: Quaternion(x=0.0, y=0.0, z=0.0, w=0.0),
        description="The zero quaternion.",
    )
    IDENTITY = declare_constant(
        101,
        value=lambda: Quaternion(x=0.0, y=0.0, z=0.0, w=1.0),
        description="The identity quaternion.",
    )

    x: Float32 = declare_property(
        101,
        is_repr=True,
        description="The x-coordinate of the Vector4.",
        tag=None,
    )
    y: Float32 = declare_property(
        102,
        is_repr=True,
        description="The y-coordinate of the Vector4.",
        tag=None,
    )
    z: Float32 = declare_property(
        103,
        is_repr=True,
        description="The z-coordinate of the Vector4.",
        tag=None,
    )
    w: Float32 = declare_property(
        104,
        is_repr=True,
        description="The w-coordinate of the Vector4.",
        tag=None,
    )
