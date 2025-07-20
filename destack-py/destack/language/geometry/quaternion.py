from destack.language.core import StructType, builtin_struct

from .vector import Vector4

# pyright: reportIncompatibleVariableOverride=false, reportIncompatibleMethodOverride=false


@builtin_struct(StructType.QUATERNION, frozen=True)
class Quaternion(Vector4):
    """A quaternion."""

    pass
