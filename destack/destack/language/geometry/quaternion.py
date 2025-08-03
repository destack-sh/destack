from typing import final

from destack.language.core import (
    Float32,
    StructFrozen,
    StructType,
    builtin_property,
    builtin_struct,
)

# pyright: reportIncompatibleVariableOverride=false, reportIncompatibleMethodOverride=false


@builtin_struct(
    StructType.QUATERNION,
    frozen=True,
    is_final=True,
)
@final
class Quaternion(StructFrozen):
    """A quaternion."""

    x: Float32 = builtin_property(
        101,
        is_repr=True,
        description="The x-coordinate of the Vector4.",
    )
    y: Float32 = builtin_property(
        102,
        is_repr=True,
        description="The y-coordinate of the Vector4.",
    )
    z: Float32 = builtin_property(
        103,
        is_repr=True,
        description="The z-coordinate of the Vector4.",
    )
    w: Float32 = builtin_property(
        104,
        is_repr=True,
        description="The w-coordinate of the Vector4.",
    )
