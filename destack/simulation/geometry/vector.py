import math
from collections.abc import Iterator
from typing import TYPE_CHECKING, final

from destack.core import (
    Float32,
    FunctionOperator,
    Int32,
    ObjectStability,
    RuntimeLanguage,
    StructFrozen,
    StructType,
    builtin_constant,
    builtin_method,
    builtin_property,
    builtin_struct,
)

if TYPE_CHECKING:
    pass


@builtin_struct(
    StructType.VECTOR2,
    frozen=True,
    is_final=True,
    stability=ObjectStability.STATIC,
)
@final
class Vector2(StructFrozen):
    """A 2D floating point Vector."""

    ZERO = builtin_constant(
        100,
        value=lambda: Vector2(x=0.0, y=0.0),
        description="The zero Vector2.",
    )
    ONE = builtin_constant(
        101,
        value=lambda: Vector2(x=1.0, y=1.0),
        description="The one Vector2.",
    )
    X_AXIS = builtin_constant(
        110,
        value=lambda: Vector2(x=1.0, y=0.0),
        description="The x-axis Vector2.",
    )
    Y_AXIS = builtin_constant(
        111,
        value=lambda: Vector2(x=0.0, y=1.0),
        description="The y-axis Vector2.",
    )

    x: Float32 = builtin_property(
        101,
        is_repr=True,
        description="The x-coordinate of the Vector2.",
    )
    y: Float32 = builtin_property(
        102,
        is_repr=True,
        description="The y-coordinate of the Vector2.",
    )

    @builtin_method(101)
    def add(self, other: "Vector2 | float") -> "Vector2":
        """Add a Vector2 or a scalar to a Vector2."""
        if isinstance(other, Vector2):
            return Vector2(x=self.x + other.x, y=self.y + other.y)
        else:
            return Vector2(x=self.x + other, y=self.y + other)

    @builtin_method(
        102,
        operator=FunctionOperator.ADD,
        proxies_method="add",
        languages=(RuntimeLanguage.PYTHON,),
    )
    def __add__(self, other: "Vector2 | float") -> "Vector2":
        """Add a Vector2 or a scalar to a Vector2."""
        return self.add(other)

    @builtin_method(103)
    def sub(self, other: "Vector2 | float") -> "Vector2":
        """Subtract a Vector2 or a scalar from a Vector2."""
        if isinstance(other, Vector2):
            return Vector2(x=self.x - other.x, y=self.y - other.y)
        else:
            return Vector2(x=self.x - other, y=self.y - other)

    @builtin_method(
        104,
        operator=FunctionOperator.SUB,
        proxies_method="sub",
        languages=(RuntimeLanguage.PYTHON,),
    )
    def __sub__(self, other: "Vector2 | float") -> "Vector2":
        """Subtract a Vector2 or a scalar from a Vector2."""
        return self.sub(other)

    @builtin_method(105)
    def mul(self, other: "Vector2 | float") -> "Vector2":
        """Multiply a Vector2 or a scalar by a Vector2."""
        if isinstance(other, Vector2):
            return Vector2(x=self.x * other.x, y=self.y * other.y)
        else:
            return Vector2(x=self.x * other, y=self.y * other)

    @builtin_method(
        106,
        operator=FunctionOperator.MUL,
        proxies_method="mul",
        languages=(RuntimeLanguage.PYTHON,),
    )
    def __mul__(self, other: "Vector2 | float") -> "Vector2":
        """Multiply a Vector2 or a scalar by a Vector2."""
        return self.mul(other)

    @builtin_method(107)
    def truediv(self, other: "Vector2 | float") -> "Vector2":
        """Divide a Vector2 or a scalar by a Vector2."""
        if isinstance(other, Vector2):
            return Vector2(x=self.x / other.x, y=self.y / other.y)
        else:
            return Vector2(x=self.x / other, y=self.y / other)

    @builtin_method(
        108,
        operator=FunctionOperator.TRUEDIV,
        proxies_method="truediv",
        languages=(RuntimeLanguage.PYTHON,),
    )
    def __truediv__(self, other: "Vector2 | float") -> "Vector2":
        """Divide a Vector2 or a scalar by a Vector2."""
        return self.truediv(other)

    @builtin_method(109)
    def abs(self) -> "Vector2":
        """Get the absolute value of a Vector2."""
        return Vector2(x=abs(self.x), y=abs(self.y))

    @builtin_method(
        110,
        operator=FunctionOperator.ABS,
        proxies_method="abs",
        languages=(RuntimeLanguage.PYTHON,),
    )
    def __abs__(self) -> "Vector2":
        """Get the absolute value of a Vector2."""
        return self.abs()

    @builtin_method(111)
    def neg(self) -> "Vector2":
        """Negate a vector."""
        return Vector2(x=-self.x, y=-self.y)

    @builtin_method(
        112,
        operator=FunctionOperator.NEG,
        proxies_method="neg",
        languages=(RuntimeLanguage.PYTHON,),
    )
    def __neg__(self) -> "Vector2":
        """Negate a vector."""
        return self.neg()

    @builtin_method(
        113,
        operator=FunctionOperator.ADD,
        proxies_method="add",
        languages=(RuntimeLanguage.PYTHON,),
    )
    def __radd__(self, other: "Vector2 | float") -> "Vector2":
        """Add a Vector2 or a scalar to a Vector2."""
        return self.add(other)

    @builtin_method(
        114,
        operator=FunctionOperator.SUB,
        proxies_method="sub",
        languages=(RuntimeLanguage.PYTHON,),
    )
    def __rsub__(self, other: "Vector2 | float") -> "Vector2":
        """Subtract a Vector2 or a scalar from a Vector2."""
        if isinstance(other, Vector2):
            return Vector2(x=other.x - self.x, y=other.y - self.y)
        else:
            return Vector2(x=other - self.x, y=other - self.y)

    @builtin_method(
        115,
        operator=FunctionOperator.MUL,
        proxies_method="mul",
        languages=(RuntimeLanguage.PYTHON,),
    )
    def __rmul__(self, other: "Vector2 | float") -> "Vector2":
        """Multiply a Vector2 or a scalar by a Vector2."""
        return self.mul(other)

    @builtin_method(
        116,
        operator=FunctionOperator.TRUEDIV,
        proxies_method="truediv",
        languages=(RuntimeLanguage.PYTHON,),
    )
    def __rtruediv__(self, other: "Vector2 | float") -> "Vector2":
        """Divide a Vector2 or a scalar by a Vector2."""
        if isinstance(other, Vector2):
            return Vector2(x=other.x / self.x, y=other.y / self.y)
        else:
            return Vector2(x=other / self.x, y=other / self.y)

    @builtin_method(117)
    def perp(self) -> "Vector2":
        """Get the perpendicular vector (rotated 90 degrees counterclockwise)."""
        return Vector2(x=self.y, y=-self.x)

    @builtin_method(118)
    def dot(self, other: "Vector2") -> float:
        """Calculate the dot product with another vector."""
        return self.x * other.x + self.y * other.y

    @builtin_method(119)
    def magnitude(self) -> float:
        """Calculate the magnitude (length) of the vector."""
        return (self.x**2 + self.y**2) ** 0.5

    @builtin_method(120)
    def magnitude2(self) -> float:
        """Calculate the squared magnitude of the vector."""
        return self.x**2 + self.y**2

    @builtin_method(121)
    def normalize(self) -> "Vector2":
        """Return a normalized (unit) vector."""
        mag = self.magnitude()
        if mag == 0:
            return Vector2(x=0.0, y=0.0)
        return Vector2(x=self.x / mag, y=self.y / mag)

    @builtin_method(122)
    def distance(self, other: "Vector2") -> float:
        """Calculate the distance to another vector."""
        return (self - other).magnitude()

    @builtin_method(123)
    def distance2(self, other: "Vector2") -> float:
        """Calculate the squared distance to another vector."""
        diff = self - other
        return diff.x**2 + diff.y**2

    @builtin_method(124)
    def angle(self, other: "Vector2") -> float:
        """Calculate the angle to another vector in radians."""

        return math.atan2(other.y - self.y, other.x - self.x)

    @builtin_method(125)
    def lerp(self, other: "Vector2", t: float) -> "Vector2":
        """Linear interpolation between this vector and another."""
        return Vector2(x=self.x + (other.x - self.x) * t, y=self.y + (other.y - self.y) * t)

    @builtin_method(126)
    def rot_with(self, center: "Vector2", angle: float) -> "Vector2":
        """Rotate this vector around another point by the given angle."""

        x = self.x - center.x
        y = self.y - center.y
        s = math.sin(angle)
        c = math.cos(angle)
        return Vector2(x=center.x + (x * c - y * s), y=center.y + (x * s + y * c))

    @builtin_method(
        127,
        operator=FunctionOperator.ITER,
        languages=(RuntimeLanguage.PYTHON,),
    )
    def __iter__(self) -> Iterator[float]:
        yield self.x
        yield self.y

    @builtin_method(
        128,
        operator=FunctionOperator.GETITEM,
        languages=(RuntimeLanguage.PYTHON,),
    )
    def __getitem__(self, index: int) -> float:
        if index == 0:
            return self.x
        elif index == 1:
            return self.y
        else:
            raise IndexError(f"index out of range: {index}")

    @builtin_method(
        129,
        operator=FunctionOperator.LEN,
        languages=(RuntimeLanguage.PYTHON,),
    )
    def __len__(self) -> int:
        return 2


@builtin_struct(
    StructType.VECTOR3,
    frozen=True,
    is_final=True,
    stability=ObjectStability.STATIC,
)
@final
class Vector3(StructFrozen):
    """A 3D floating point Vector."""

    ZERO = builtin_constant(
        100,
        value=lambda: Vector3(x=0.0, y=0.0, z=0.0),
        description="The zero Vector3.",
    )
    ONE = builtin_constant(
        101,
        value=lambda: Vector3(x=1.0, y=1.0, z=1.0),
        description="The one Vector3.",
    )
    X_AXIS = builtin_constant(
        110,
        value=lambda: Vector3(x=1.0, y=0.0, z=0.0),
        description="The x-axis Vector3.",
    )
    Y_AXIS = builtin_constant(
        111,
        value=lambda: Vector3(x=0.0, y=1.0, z=0.0),
        description="The y-axis Vector3.",
    )
    Z_AXIS = builtin_constant(
        112,
        value=lambda: Vector3(x=0.0, y=0.0, z=1.0),
        description="The z-axis Vector3.",
    )

    x: Float32 = builtin_property(
        101,
        is_repr=True,
        description="The x-coordinate of the Vector3.",
    )
    y: Float32 = builtin_property(
        102,
        is_repr=True,
        description="The y-coordinate of the Vector3.",
    )
    z: Float32 = builtin_property(
        103,
        is_repr=True,
        description="The z-coordinate of the Vector3.",
    )

    @builtin_method(101)
    def add(self, other: "Vector3 | float") -> "Vector3":
        """Add a Vector3 or a scalar to a Vector3."""
        if isinstance(other, Vector3):
            return Vector3(x=self.x + other.x, y=self.y + other.y, z=self.z + other.z)
        else:
            return Vector3(x=self.x + other, y=self.y + other, z=self.z + other)

    @builtin_method(
        102,
        operator=FunctionOperator.ADD,
        proxies_method="add",
        languages=(RuntimeLanguage.PYTHON,),
    )
    def __add__(self, other: "Vector3 | float") -> "Vector3":
        """Add a Vector3 or a scalar to a Vector3."""
        return self.add(other)

    @builtin_method(103)
    def sub(self, other: "Vector3 | float") -> "Vector3":
        """Subtract a Vector3 or a scalar from a Vector3."""
        if isinstance(other, Vector3):
            return Vector3(x=self.x - other.x, y=self.y - other.y, z=self.z - other.z)
        else:
            return Vector3(x=self.x - other, y=self.y - other, z=self.z - other)

    @builtin_method(
        104,
        operator=FunctionOperator.SUB,
        proxies_method="sub",
        languages=(RuntimeLanguage.PYTHON,),
    )
    def __sub__(self, other: "Vector3 | float") -> "Vector3":
        """Subtract a Vector3 or a scalar from a Vector3."""
        return self.sub(other)

    @builtin_method(105)
    def mul(self, other: "Vector3 | float") -> "Vector3":
        """Multiply a Vector3 or a scalar by a Vector3."""
        if isinstance(other, Vector3):
            return Vector3(x=self.x * other.x, y=self.y * other.y, z=self.z * other.z)
        else:
            return Vector3(x=self.x * other, y=self.y * other, z=self.z * other)

    @builtin_method(
        106,
        operator=FunctionOperator.MUL,
        proxies_method="mul",
        languages=(RuntimeLanguage.PYTHON,),
    )
    def __mul__(self, other: "Vector3 | float") -> "Vector3":
        """Multiply a Vector3 or a scalar by a Vector3."""
        return self.mul(other)

    @builtin_method(107)
    def truediv(self, other: "Vector3 | float") -> "Vector3":
        """Divide a Vector3 or a scalar by a Vector3."""
        if isinstance(other, Vector3):
            return Vector3(x=self.x / other.x, y=self.y / other.y, z=self.z / other.z)
        else:
            return Vector3(x=self.x / other, y=self.y / other, z=self.z / other)

    @builtin_method(
        108,
        operator=FunctionOperator.TRUEDIV,
        proxies_method="truediv",
        languages=(RuntimeLanguage.PYTHON,),
    )
    def __truediv__(self, other: "Vector3 | float") -> "Vector3":
        """Divide a Vector3 or a scalar by a Vector3."""
        return self.truediv(other)

    @builtin_method(109)
    def abs(self) -> "Vector3":
        """Get the absolute value of a Vector3."""
        return Vector3(x=abs(self.x), y=abs(self.y), z=abs(self.z))

    @builtin_method(
        110,
        operator=FunctionOperator.ABS,
        proxies_method="abs",
        languages=(RuntimeLanguage.PYTHON,),
    )
    def __abs__(self) -> "Vector3":
        """Get the absolute value of a Vector3."""
        return self.abs()

    @builtin_method(111)
    def neg(self) -> "Vector3":
        """Negate a vector."""
        return Vector3(x=-self.x, y=-self.y, z=-self.z)

    @builtin_method(
        112,
        operator=FunctionOperator.NEG,
        proxies_method="neg",
        languages=(RuntimeLanguage.PYTHON,),
    )
    def __neg__(self) -> "Vector3":
        """Negate a vector."""
        return self.neg()

    @builtin_method(
        113,
        operator=FunctionOperator.ADD,
        proxies_method="add",
        languages=(RuntimeLanguage.PYTHON,),
    )
    def __radd__(self, other: "Vector3 | float") -> "Vector3":
        """Add a Vector3 or a scalar to a Vector3."""
        return self.add(other)

    @builtin_method(
        114,
        operator=FunctionOperator.SUB,
        proxies_method="sub",
        languages=(RuntimeLanguage.PYTHON,),
    )
    def __rsub__(self, other: "Vector3 | float") -> "Vector3":
        """Subtract a Vector3 or a scalar from a Vector3."""
        if isinstance(other, Vector3):
            return Vector3(x=other.x - self.x, y=other.y - self.y, z=other.z - self.z)
        else:
            return Vector3(x=other - self.x, y=other - self.y, z=other - self.z)

    @builtin_method(
        115,
        operator=FunctionOperator.MUL,
        proxies_method="mul",
        languages=(RuntimeLanguage.PYTHON,),
    )
    def __rmul__(self, other: "Vector3 | float") -> "Vector3":
        """Multiply a Vector3 or a scalar by a Vector3."""
        return self.mul(other)

    @builtin_method(
        116,
        operator=FunctionOperator.TRUEDIV,
        proxies_method="truediv",
        languages=(RuntimeLanguage.PYTHON,),
    )
    def __rtruediv__(self, other: "Vector3 | float") -> "Vector3":
        """Divide a Vector3 or a scalar by a Vector3."""
        if isinstance(other, Vector3):
            return Vector3(x=other.x / self.x, y=other.y / self.y, z=other.z / self.z)
        else:
            return Vector3(x=other / self.x, y=other / self.y, z=other / self.z)

    @builtin_method(117)
    def dot(self, other: "Vector3") -> float:
        """Calculate the dot product with another vector."""
        return self.x * other.x + self.y * other.y + self.z * other.z

    @builtin_method(118)
    def magnitude(self) -> float:
        """Calculate the magnitude (length) of the vector."""
        return (self.x**2 + self.y**2 + self.z**2) ** 0.5

    @builtin_method(119)
    def magnitude2(self) -> float:
        """Calculate the squared magnitude of the vector."""
        return self.x**2 + self.y**2 + self.z**2

    @builtin_method(120)
    def normalize(self) -> "Vector3":
        """Return a normalized (unit) vector."""
        mag = self.magnitude()
        if mag == 0:
            return Vector3(x=0.0, y=0.0, z=0.0)
        return Vector3(x=self.x / mag, y=self.y / mag, z=self.z / mag)

    @builtin_method(121)
    def distance(self, other: "Vector3") -> float:
        """Calculate the distance to another vector."""
        return (self - other).magnitude()

    @builtin_method(122)
    def distance2(self, other: "Vector3") -> float:
        """Calculate the squared distance to another vector."""
        diff = self - other
        return diff.x**2 + diff.y**2 + diff.z**2

    @builtin_method(123)
    def angle(self, other: "Vector3") -> float:
        """Calculate the angle to another vector in radians."""

        dot_product = self.dot(other)
        mag_product = self.magnitude() * other.magnitude()
        if mag_product == 0:
            return 0.0
        return math.acos(max(-1.0, min(1.0, dot_product / mag_product)))

    @builtin_method(124)
    def lerp(self, other: "Vector3", t: float) -> "Vector3":
        """Linear interpolation between this vector and another."""
        return Vector3(
            x=self.x + (other.x - self.x) * t,
            y=self.y + (other.y - self.y) * t,
            z=self.z + (other.z - self.z) * t,
        )

    @builtin_method(125)
    def cross(self, other: "Vector3") -> "Vector3":
        """Calculate the cross product with another vector."""
        return Vector3(
            x=self.y * other.z - self.z * other.y,
            y=self.z * other.x - self.x * other.z,
            z=self.x * other.y - self.y * other.x,
        )

    @builtin_method(
        126,
        operator=FunctionOperator.ITER,
        languages=(RuntimeLanguage.PYTHON,),
    )
    def __iter__(self) -> Iterator[float]:
        yield self.x
        yield self.y
        yield self.z

    @builtin_method(
        127,
        operator=FunctionOperator.GETITEM,
        languages=(RuntimeLanguage.PYTHON,),
    )
    def __getitem__(self, index: int) -> float:
        if index == 0:
            return self.x
        elif index == 1:
            return self.y
        elif index == 2:
            return self.z
        else:
            raise IndexError(f"index out of range: {index}")

    @builtin_method(
        128,
        operator=FunctionOperator.LEN,
        languages=(RuntimeLanguage.PYTHON,),
    )
    def __len__(self) -> int:
        return 3


@builtin_struct(
    StructType.VECTOR4,
    frozen=True,
    is_final=True,
    stability=ObjectStability.STATIC,
)
@final
class Vector4(StructFrozen):
    """A 4D floating point Vector."""

    ZERO = builtin_constant(
        100,
        value=lambda: Vector4(x=0.0, y=0.0, z=0.0, w=0.0),
        description="The zero Vector4.",
    )
    ONE = builtin_constant(
        101,
        value=lambda: Vector4(x=1.0, y=1.0, z=1.0, w=1.0),
        description="The one Vector4.",
    )
    X_AXIS = builtin_constant(
        110,
        value=lambda: Vector4(x=1.0, y=0.0, z=0.0, w=0.0),
        description="The x-axis Vector4.",
    )
    Y_AXIS = builtin_constant(
        111,
        value=lambda: Vector4(x=0.0, y=1.0, z=0.0, w=0.0),
        description="The y-axis Vector4.",
    )
    Z_AXIS = builtin_constant(
        112,
        value=lambda: Vector4(x=0.0, y=0.0, z=1.0, w=0.0),
        description="The z-axis Vector4.",
    )
    W_AXIS = builtin_constant(
        113,
        value=lambda: Vector4(x=0.0, y=0.0, z=0.0, w=1.0),
        description="The w-axis Vector4.",
    )

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

    @builtin_method(101)
    def add(self, other: "Vector4 | float") -> "Vector4":
        """Add a Vector4 or a scalar to a Vector4."""
        if isinstance(other, Vector4):
            return Vector4(
                x=self.x + other.x, y=self.y + other.y, z=self.z + other.z, w=self.w + other.w
            )
        else:
            return Vector4(x=self.x + other, y=self.y + other, z=self.z + other, w=self.w + other)

    @builtin_method(
        102,
        operator=FunctionOperator.ADD,
        proxies_method="add",
        languages=(RuntimeLanguage.PYTHON,),
    )
    def __add__(self, other: "Vector4 | float") -> "Vector4":
        """Add a Vector4 or a scalar to a Vector4."""
        return self.add(other)

    @builtin_method(103)
    def sub(self, other: "Vector4 | float") -> "Vector4":
        """Subtract a Vector4 or a scalar from a Vector4."""
        if isinstance(other, Vector4):
            return Vector4(
                x=self.x - other.x, y=self.y - other.y, z=self.z - other.z, w=self.w - other.w
            )
        else:
            return Vector4(x=self.x - other, y=self.y - other, z=self.z - other, w=self.w - other)

    @builtin_method(
        104,
        operator=FunctionOperator.SUB,
        proxies_method="sub",
        languages=(RuntimeLanguage.PYTHON,),
    )
    def __sub__(self, other: "Vector4 | float") -> "Vector4":
        """Subtract a Vector4 or a scalar from a Vector4."""
        return self.sub(other)

    @builtin_method(105)
    def mul(self, other: "Vector4 | float") -> "Vector4":
        """Multiply a Vector4 or a scalar by a Vector4."""
        if isinstance(other, Vector4):
            return Vector4(
                x=self.x * other.x, y=self.y * other.y, z=self.z * other.z, w=self.w * other.w
            )
        else:
            return Vector4(x=self.x * other, y=self.y * other, z=self.z * other, w=self.w * other)

    @builtin_method(
        106,
        operator=FunctionOperator.MUL,
        proxies_method="mul",
        languages=(RuntimeLanguage.PYTHON,),
    )
    def __mul__(self, other: "Vector4 | float") -> "Vector4":
        """Multiply a Vector4 or a scalar by a Vector4."""
        return self.mul(other)

    @builtin_method(107)
    def truediv(self, other: "Vector4 | float") -> "Vector4":
        """Divide a Vector4 or a scalar by a Vector4."""
        if isinstance(other, Vector4):
            return Vector4(
                x=self.x / other.x, y=self.y / other.y, z=self.z / other.z, w=self.w / other.w
            )
        else:
            return Vector4(x=self.x / other, y=self.y / other, z=self.z / other, w=self.w / other)

    @builtin_method(
        108,
        operator=FunctionOperator.TRUEDIV,
        proxies_method="truediv",
        languages=(RuntimeLanguage.PYTHON,),
    )
    def __truediv__(self, other: "Vector4 | float") -> "Vector4":
        """Divide a Vector4 or a scalar by a Vector4."""
        return self.truediv(other)

    @builtin_method(109)
    def abs(self) -> "Vector4":
        """Get the absolute value of a Vector4."""
        return Vector4(x=abs(self.x), y=abs(self.y), z=abs(self.z), w=abs(self.w))

    @builtin_method(
        110,
        operator=FunctionOperator.ABS,
        proxies_method="abs",
        languages=(RuntimeLanguage.PYTHON,),
    )
    def __abs__(self) -> "Vector4":
        """Get the absolute value of a Vector4."""
        return self.abs()

    @builtin_method(111)
    def neg(self) -> "Vector4":
        """Negate a vector."""
        return Vector4(x=-self.x, y=-self.y, z=-self.z, w=-self.w)

    @builtin_method(
        112,
        operator=FunctionOperator.NEG,
        proxies_method="neg",
        languages=(RuntimeLanguage.PYTHON,),
    )
    def __neg__(self) -> "Vector4":
        """Negate a vector."""
        return self.neg()

    @builtin_method(
        113,
        operator=FunctionOperator.ADD,
        proxies_method="add",
        languages=(RuntimeLanguage.PYTHON,),
    )
    def __radd__(self, other: "Vector4 | float") -> "Vector4":
        """Add a Vector4 or a scalar to a Vector4."""
        return self.add(other)

    @builtin_method(
        114,
        operator=FunctionOperator.SUB,
        proxies_method="sub",
        languages=(RuntimeLanguage.PYTHON,),
    )
    def __rsub__(self, other: "Vector4 | float") -> "Vector4":
        """Subtract a Vector4 or a scalar from a Vector4."""
        if isinstance(other, Vector4):
            return Vector4(
                x=other.x - self.x, y=other.y - self.y, z=other.z - self.z, w=other.w - self.w
            )
        else:
            return Vector4(x=other - self.x, y=other - self.y, z=other - self.z, w=other - self.w)

    @builtin_method(
        115,
        operator=FunctionOperator.MUL,
        proxies_method="mul",
        languages=(RuntimeLanguage.PYTHON,),
    )
    def __rmul__(self, other: "Vector4 | float") -> "Vector4":
        """Multiply a Vector4 or a scalar by a Vector4."""
        return self.mul(other)

    @builtin_method(
        116,
        operator=FunctionOperator.TRUEDIV,
        proxies_method="truediv",
        languages=(RuntimeLanguage.PYTHON,),
    )
    def __rtruediv__(self, other: "Vector4 | float") -> "Vector4":
        """Divide a Vector4 or a scalar by a Vector4."""
        if isinstance(other, Vector4):
            return Vector4(
                x=other.x / self.x, y=other.y / self.y, z=other.z / self.z, w=other.w / self.w
            )
        else:
            return Vector4(x=other / self.x, y=other / self.y, z=other / self.z, w=other / self.w)

    @builtin_method(117)
    def dot(self, other: "Vector4") -> float:
        """Calculate the dot product with another vector."""
        return self.x * other.x + self.y * other.y + self.z * other.z + self.w * other.w

    @builtin_method(118)
    def magnitude(self) -> float:
        """Calculate the magnitude (length) of the vector."""
        return (self.x**2 + self.y**2 + self.z**2 + self.w**2) ** 0.5

    @builtin_method(119)
    def magnitude2(self) -> float:
        """Calculate the squared magnitude of the vector."""
        return self.x**2 + self.y**2 + self.z**2 + self.w**2

    @builtin_method(120)
    def normalize(self) -> "Vector4":
        """Return a normalized (unit) vector."""
        mag = self.magnitude()
        if mag == 0:
            return Vector4(x=0.0, y=0.0, z=0.0, w=0.0)
        return Vector4(x=self.x / mag, y=self.y / mag, z=self.z / mag, w=self.w / mag)

    @builtin_method(121)
    def distance(self, other: "Vector4") -> float:
        """Calculate the distance to another vector."""
        return (self - other).magnitude()

    @builtin_method(122)
    def distance2(self, other: "Vector4") -> float:
        """Calculate the squared distance to another vector."""
        diff = self - other
        return diff.x**2 + diff.y**2 + diff.z**2 + diff.w**2

    @builtin_method(123)
    def angle(self, other: "Vector4") -> float:
        """Calculate the angle to another vector in radians."""

        dot_product = self.dot(other)
        mag_product = self.magnitude() * other.magnitude()
        if mag_product == 0:
            return 0.0
        return math.acos(max(-1.0, min(1.0, dot_product / mag_product)))

    @builtin_method(124)
    def lerp(self, other: "Vector4", t: float) -> "Vector4":
        """Linear interpolation between this vector and another."""
        return Vector4(
            x=self.x + (other.x - self.x) * t,
            y=self.y + (other.y - self.y) * t,
            z=self.z + (other.z - self.z) * t,
            w=self.w + (other.w - self.w) * t,
        )

    @builtin_method(
        125,
        operator=FunctionOperator.ITER,
        languages=(RuntimeLanguage.PYTHON,),
    )
    def __iter__(self) -> Iterator[float]:
        yield self.x
        yield self.y
        yield self.z
        yield self.w

    @builtin_method(
        126,
        operator=FunctionOperator.GETITEM,
        languages=(RuntimeLanguage.PYTHON,),
    )
    def __getitem__(self, index: int) -> float:
        if index == 0:
            return self.x
        elif index == 1:
            return self.y
        elif index == 2:
            return self.z
        elif index == 3:
            return self.w
        else:
            raise IndexError(f"index out of range: {index}")

    @builtin_method(
        127,
        operator=FunctionOperator.LEN,
        languages=(RuntimeLanguage.PYTHON,),
    )
    def __len__(self) -> int:
        return 4


@builtin_struct(
    StructType.VECTOR2I,
    frozen=True,
    is_final=True,
    stability=ObjectStability.STATIC,
)
@final
class Vector2i(StructFrozen):
    """A 2D integer Vector."""

    ZERO = builtin_constant(
        100,
        value=lambda: Vector2i(x=0, y=0),
        description="The zero Vector2i.",
    )
    ONE = builtin_constant(
        101,
        value=lambda: Vector2i(x=1, y=1),
        description="The one Vector2i.",
    )
    X_AXIS = builtin_constant(
        110,
        value=lambda: Vector2i(x=1, y=0),
        description="The x-axis Vector2i.",
    )
    Y_AXIS = builtin_constant(
        111,
        value=lambda: Vector2i(x=0, y=1),
        description="The y-axis Vector2i.",
    )

    x: Int32 = builtin_property(
        101,
        is_repr=True,
        description="The x-coordinate of the Vector2i.",
    )
    y: Int32 = builtin_property(
        102,
        is_repr=True,
        description="The y-coordinate of the Vector2i.",
    )

    @builtin_method(101)
    def add(self, other: "Vector2i | int") -> "Vector2i":
        """Add a Vector2i or a scalar to a Vector2i."""
        if isinstance(other, Vector2i):
            return Vector2i(x=self.x + other.x, y=self.y + other.y)
        else:
            return Vector2i(x=self.x + other, y=self.y + other)

    @builtin_method(
        102,
        operator=FunctionOperator.ADD,
        proxies_method="add",
        languages=(RuntimeLanguage.PYTHON,),
    )
    def __add__(self, other: "Vector2i | int") -> "Vector2i":
        """Add a Vector2i or a scalar to a Vector2i."""
        return self.add(other)

    @builtin_method(103)
    def sub(self, other: "Vector2i | int") -> "Vector2i":
        """Subtract a Vector2i or a scalar from a Vector2i."""
        if isinstance(other, Vector2i):
            return Vector2i(x=self.x - other.x, y=self.y - other.y)
        else:
            return Vector2i(x=self.x - other, y=self.y - other)

    @builtin_method(
        104,
        operator=FunctionOperator.SUB,
        proxies_method="sub",
        languages=(RuntimeLanguage.PYTHON,),
    )
    def __sub__(self, other: "Vector2i | int") -> "Vector2i":
        """Subtract a Vector2i or a scalar from a Vector2i."""
        return self.sub(other)

    @builtin_method(105)
    def mul(self, other: "Vector2i | int") -> "Vector2i":
        """Multiply a Vector2i or a scalar by a Vector2i."""
        if isinstance(other, Vector2i):
            return Vector2i(x=self.x * other.x, y=self.y * other.y)
        else:
            return Vector2i(x=self.x * other, y=self.y * other)

    @builtin_method(
        106,
        operator=FunctionOperator.MUL,
        proxies_method="mul",
        languages=(RuntimeLanguage.PYTHON,),
    )
    def __mul__(self, other: "Vector2i | int") -> "Vector2i":
        """Multiply a Vector2i or a scalar by a Vector2i."""
        return self.mul(other)

    @builtin_method(107)
    def truediv(self, other: "Vector2i | int") -> "Vector2i":
        """Divide a Vector2i or a scalar by a Vector2i."""
        if isinstance(other, Vector2i):
            return Vector2i(x=self.x // other.x, y=self.y // other.y)
        else:
            return Vector2i(x=self.x // other, y=self.y // other)

    @builtin_method(
        108,
        operator=FunctionOperator.TRUEDIV,
        proxies_method="truediv",
        languages=(RuntimeLanguage.PYTHON,),
    )
    def __truediv__(self, other: "Vector2i | int") -> "Vector2i":
        """Divide a Vector2i or a scalar by a Vector2i."""
        return self.truediv(other)

    @builtin_method(109)
    def abs(self) -> "Vector2i":
        """Get the absolute value of a Vector2i."""
        return Vector2i(x=abs(self.x), y=abs(self.y))

    @builtin_method(
        110,
        operator=FunctionOperator.ABS,
        proxies_method="abs",
        languages=(RuntimeLanguage.PYTHON,),
    )
    def __abs__(self) -> "Vector2i":
        """Get the absolute value of a Vector2i."""
        return self.abs()

    @builtin_method(111)
    def neg(self) -> "Vector2i":
        """Negate a vector."""
        return Vector2i(x=-self.x, y=-self.y)

    @builtin_method(
        112,
        operator=FunctionOperator.NEG,
        proxies_method="neg",
        languages=(RuntimeLanguage.PYTHON,),
    )
    def __neg__(self) -> "Vector2i":
        """Negate a vector."""
        return self.neg()

    @builtin_method(
        113,
        operator=FunctionOperator.ADD,
        proxies_method="add",
        languages=(RuntimeLanguage.PYTHON,),
    )
    def __radd__(self, other: "Vector2i | int") -> "Vector2i":
        """Add a Vector2i or a scalar to a Vector2i."""
        return self.add(other)

    @builtin_method(
        114,
        operator=FunctionOperator.SUB,
        proxies_method="sub",
        languages=(RuntimeLanguage.PYTHON,),
    )
    def __rsub__(self, other: "Vector2i | int") -> "Vector2i":
        """Subtract a Vector2i or a scalar from a Vector2i."""
        if isinstance(other, Vector2i):
            return Vector2i(x=other.x - self.x, y=other.y - self.y)
        else:
            return Vector2i(x=other - self.x, y=other - self.y)

    @builtin_method(
        115,
        operator=FunctionOperator.MUL,
        proxies_method="mul",
        languages=(RuntimeLanguage.PYTHON,),
    )
    def __rmul__(self, other: "Vector2i | int") -> "Vector2i":
        """Multiply a Vector2i or a scalar by a Vector2i."""
        return self.mul(other)

    @builtin_method(
        116,
        operator=FunctionOperator.TRUEDIV,
        proxies_method="truediv",
        languages=(RuntimeLanguage.PYTHON,),
    )
    def __rtruediv__(self, other: "Vector2i | int") -> "Vector2i":
        """Divide a Vector2i or a scalar by a Vector2i."""
        if isinstance(other, Vector2i):
            return Vector2i(x=other.x // self.x, y=other.y // self.y)
        else:
            return Vector2i(x=other // self.x, y=other // self.y)

    @builtin_method(117)
    def perp(self) -> "Vector2i":
        """Get the perpendicular vector (rotated 90 degrees counterclockwise)."""
        return Vector2i(x=self.y, y=-self.x)

    @builtin_method(118)
    def dot(self, other: "Vector2i") -> int:
        """Calculate the dot product with another vector."""
        return self.x * other.x + self.y * other.y

    @builtin_method(119)
    def magnitude(self) -> float:
        """Calculate the magnitude (length) of the vector."""
        return (self.x**2 + self.y**2) ** 0.5

    @builtin_method(120)
    def magnitude2(self) -> int:
        """Calculate the squared magnitude of the vector."""
        return self.x**2 + self.y**2

    @builtin_method(121)
    def normalize(self) -> "Vector2":
        """Return a normalized (unit) vector as floating point vector."""
        mag = self.magnitude()
        if mag == 0:
            return Vector2(x=0.0, y=0.0)
        return Vector2(x=self.x / mag, y=self.y / mag)

    @builtin_method(122)
    def distance(self, other: "Vector2i") -> float:
        """Calculate the distance to another vector."""
        return (self - other).magnitude()

    @builtin_method(123)
    def distance2(self, other: "Vector2i") -> int:
        """Calculate the squared distance to another vector."""
        diff = self - other
        return diff.x**2 + diff.y**2

    @builtin_method(124)
    def angle(self, other: "Vector2i") -> float:
        """Calculate the angle to another vector in radians."""

        return math.atan2(other.y - self.y, other.x - self.x)

    @builtin_method(125)
    def lerp(self, other: "Vector2i", t: float) -> "Vector2":
        """Linear interpolation between this vector and another as floating point vector."""
        return Vector2(x=self.x + (other.x - self.x) * t, y=self.y + (other.y - self.y) * t)

    @builtin_method(126)
    def rot_with(self, center: "Vector2i", angle: float) -> "Vector2":
        """Rotate this vector around another point by the given angle as floating point vector."""

        x = self.x - center.x
        y = self.y - center.y
        s = math.sin(angle)
        c = math.cos(angle)
        return Vector2(x=center.x + (x * c - y * s), y=center.y + (x * s + y * c))

    @builtin_method(
        127,
        operator=FunctionOperator.ITER,
        languages=(RuntimeLanguage.PYTHON,),
    )
    def __iter__(self) -> Iterator[int]:
        yield self.x
        yield self.y

    @builtin_method(
        128,
        operator=FunctionOperator.GETITEM,
        languages=(RuntimeLanguage.PYTHON,),
    )
    def __getitem__(self, index: int) -> int:
        if index == 0:
            return self.x
        elif index == 1:
            return self.y
        else:
            raise IndexError(f"index out of range: {index}")

    @builtin_method(
        129,
        operator=FunctionOperator.LEN,
        languages=(RuntimeLanguage.PYTHON,),
    )
    def __len__(self) -> int:
        return 2


@builtin_struct(
    StructType.VECTOR3I,
    frozen=True,
    is_final=True,
    stability=ObjectStability.STATIC,
)
@final
class Vector3i(StructFrozen):
    """A 3D integer Vector."""

    ZERO = builtin_constant(
        100,
        value=lambda: Vector3i(x=0, y=0, z=0),
        description="The zero Vector3i.",
    )
    ONE = builtin_constant(
        101,
        value=lambda: Vector3i(x=1, y=1, z=1),
        description="The one Vector3i.",
    )
    X_AXIS = builtin_constant(
        110,
        value=lambda: Vector3i(x=1, y=0, z=0),
        description="The x-axis Vector3i.",
    )
    Y_AXIS = builtin_constant(
        111,
        value=lambda: Vector3i(x=0, y=1, z=0),
        description="The y-axis Vector3i.",
    )
    Z_AXIS = builtin_constant(
        112,
        value=lambda: Vector3i(x=0, y=0, z=1),
        description="The z-axis Vector3i.",
    )

    x: Int32 = builtin_property(
        101,
        is_repr=True,
        description="The x-coordinate of the Vector3i.",
    )
    y: Int32 = builtin_property(
        102,
        is_repr=True,
        description="The y-coordinate of the Vector3i.",
    )
    z: Int32 = builtin_property(
        103,
        is_repr=True,
        description="The z-coordinate of the Vector3i.",
    )

    @builtin_method(101)
    def add(self, other: "Vector3i | int") -> "Vector3i":
        """Add a Vector3i or a scalar to a Vector3i."""
        if isinstance(other, Vector3i):
            return Vector3i(x=self.x + other.x, y=self.y + other.y, z=self.z + other.z)
        else:
            return Vector3i(x=self.x + other, y=self.y + other, z=self.z + other)

    @builtin_method(
        102,
        operator=FunctionOperator.ADD,
        proxies_method="add",
        languages=(RuntimeLanguage.PYTHON,),
    )
    def __add__(self, other: "Vector3i | int") -> "Vector3i":
        """Add a Vector3i or a scalar to a Vector3i."""
        return self.add(other)

    @builtin_method(103)
    def sub(self, other: "Vector3i | int") -> "Vector3i":
        """Subtract a Vector3i or a scalar from a Vector3i."""
        if isinstance(other, Vector3i):
            return Vector3i(x=self.x - other.x, y=self.y - other.y, z=self.z - other.z)
        else:
            return Vector3i(x=self.x - other, y=self.y - other, z=self.z - other)

    @builtin_method(
        104,
        operator=FunctionOperator.SUB,
        proxies_method="sub",
        languages=(RuntimeLanguage.PYTHON,),
    )
    def __sub__(self, other: "Vector3i | int") -> "Vector3i":
        """Subtract a Vector3i or a scalar from a Vector3i."""
        return self.sub(other)

    @builtin_method(105)
    def mul(self, other: "Vector3i | int") -> "Vector3i":
        """Multiply a Vector3i or a scalar by a Vector3i."""
        if isinstance(other, Vector3i):
            return Vector3i(x=self.x * other.x, y=self.y * other.y, z=self.z * other.z)
        else:
            return Vector3i(x=self.x * other, y=self.y * other, z=self.z * other)

    @builtin_method(
        106,
        operator=FunctionOperator.MUL,
        proxies_method="mul",
        languages=(RuntimeLanguage.PYTHON,),
    )
    def __mul__(self, other: "Vector3i | int") -> "Vector3i":
        """Multiply a Vector3i or a scalar by a Vector3i."""
        return self.mul(other)

    @builtin_method(107)
    def truediv(self, other: "Vector3i | int") -> "Vector3i":
        """Divide a Vector3i or a scalar by a Vector3i."""
        if isinstance(other, Vector3i):
            return Vector3i(x=self.x // other.x, y=self.y // other.y, z=self.z // other.z)
        else:
            return Vector3i(x=self.x // other, y=self.y // other, z=self.z // other)

    @builtin_method(
        108,
        operator=FunctionOperator.TRUEDIV,
        proxies_method="truediv",
        languages=(RuntimeLanguage.PYTHON,),
    )
    def __truediv__(self, other: "Vector3i | int") -> "Vector3i":
        """Divide a Vector3i or a scalar by a Vector3i."""
        return self.truediv(other)

    @builtin_method(109)
    def abs(self) -> "Vector3i":
        """Get the absolute value of a Vector3i."""
        return Vector3i(x=abs(self.x), y=abs(self.y), z=abs(self.z))

    @builtin_method(
        110,
        operator=FunctionOperator.ABS,
        proxies_method="abs",
        languages=(RuntimeLanguage.PYTHON,),
    )
    def __abs__(self) -> "Vector3i":
        """Get the absolute value of a Vector3i."""
        return self.abs()

    @builtin_method(111)
    def neg(self) -> "Vector3i":
        """Negate a vector."""
        return Vector3i(x=-self.x, y=-self.y, z=-self.z)

    @builtin_method(
        112,
        operator=FunctionOperator.NEG,
        proxies_method="neg",
        languages=(RuntimeLanguage.PYTHON,),
    )
    def __neg__(self) -> "Vector3i":
        """Negate a vector."""
        return self.neg()

    @builtin_method(
        113,
        operator=FunctionOperator.ADD,
        proxies_method="add",
        languages=(RuntimeLanguage.PYTHON,),
    )
    def __radd__(self, other: "Vector3i | int") -> "Vector3i":
        """Add a Vector3i or a scalar to a Vector3i."""
        return self.add(other)

    @builtin_method(
        114,
        operator=FunctionOperator.SUB,
        proxies_method="sub",
        languages=(RuntimeLanguage.PYTHON,),
    )
    def __rsub__(self, other: "Vector3i | int") -> "Vector3i":
        """Subtract a Vector3i or a scalar from a Vector3i."""
        if isinstance(other, Vector3i):
            return Vector3i(x=other.x - self.x, y=other.y - self.y, z=other.z - self.z)
        else:
            return Vector3i(x=other - self.x, y=other - self.y, z=other - self.z)

    @builtin_method(
        115,
        operator=FunctionOperator.MUL,
        proxies_method="mul",
        languages=(RuntimeLanguage.PYTHON,),
    )
    def __rmul__(self, other: "Vector3i | int") -> "Vector3i":
        """Multiply a Vector3i or a scalar by a Vector3i."""
        return self.mul(other)

    @builtin_method(
        116,
        operator=FunctionOperator.TRUEDIV,
        proxies_method="truediv",
        languages=(RuntimeLanguage.PYTHON,),
    )
    def __rtruediv__(self, other: "Vector3i | int") -> "Vector3i":
        """Divide a Vector3i or a scalar by a Vector3i."""
        if isinstance(other, Vector3i):
            return Vector3i(x=other.x // self.x, y=other.y // self.y, z=other.z // self.z)
        else:
            return Vector3i(x=other // self.x, y=other // self.y, z=other // self.z)

    @builtin_method(117)
    def dot(self, other: "Vector3i") -> int:
        """Calculate the dot product with another vector."""
        return self.x * other.x + self.y * other.y + self.z * other.z

    @builtin_method(118)
    def magnitude(self) -> float:
        """Calculate the magnitude (length) of the vector."""
        return (self.x**2 + self.y**2 + self.z**2) ** 0.5

    @builtin_method(119)
    def magnitude2(self) -> int:
        """Calculate the squared magnitude of the vector."""
        return self.x**2 + self.y**2 + self.z**2

    @builtin_method(120)
    def normalize(self) -> "Vector3":
        """Return a normalized (unit) vector as floating point vector."""
        mag = self.magnitude()
        if mag == 0:
            return Vector3(x=0.0, y=0.0, z=0.0)
        return Vector3(x=self.x / mag, y=self.y / mag, z=self.z / mag)

    @builtin_method(121)
    def distance(self, other: "Vector3i") -> float:
        """Calculate the distance to another vector."""
        return (self - other).magnitude()

    @builtin_method(122)
    def distance2(self, other: "Vector3i") -> int:
        """Calculate the squared distance to another vector."""
        diff = self - other
        return diff.x**2 + diff.y**2 + diff.z**2

    @builtin_method(123)
    def angle(self, other: "Vector3i") -> float:
        """Calculate the angle to another vector in radians."""

        dot_product = self.dot(other)
        mag_product = self.magnitude() * other.magnitude()
        if mag_product == 0:
            return 0.0
        return math.acos(max(-1.0, min(1.0, dot_product / mag_product)))

    @builtin_method(124)
    def lerp(self, other: "Vector3i", t: float) -> "Vector3":
        """Linear interpolation between this vector and another as floating point vector."""
        return Vector3(
            x=self.x + (other.x - self.x) * t,
            y=self.y + (other.y - self.y) * t,
            z=self.z + (other.z - self.z) * t,
        )

    @builtin_method(125)
    def cross(self, other: "Vector3i") -> "Vector3i":
        """Calculate the cross product with another vector."""
        return Vector3i(
            x=self.y * other.z - self.z * other.y,
            y=self.z * other.x - self.x * other.z,
            z=self.x * other.y - self.y * other.x,
        )

    @builtin_method(
        126,
        operator=FunctionOperator.ITER,
        languages=(RuntimeLanguage.PYTHON,),
    )
    def __iter__(self) -> Iterator[int]:
        yield self.x
        yield self.y
        yield self.z

    @builtin_method(
        127,
        operator=FunctionOperator.GETITEM,
        languages=(RuntimeLanguage.PYTHON,),
    )
    def __getitem__(self, index: int) -> int:
        if index == 0:
            return self.x
        elif index == 1:
            return self.y
        elif index == 2:
            return self.z
        else:
            raise IndexError(f"index out of range: {index}")

    @builtin_method(
        128,
        operator=FunctionOperator.LEN,
        languages=(RuntimeLanguage.PYTHON,),
    )
    def __len__(self) -> int:
        return 3


@builtin_struct(
    StructType.VECTOR4I,
    frozen=True,
    is_final=True,
    stability=ObjectStability.STATIC,
)
@final
class Vector4i(StructFrozen):
    """A 4D integer Vector."""

    ZERO = builtin_constant(
        100,
        value=lambda: Vector4i(x=0, y=0, z=0, w=0),
        description="The zero Vector4i.",
    )
    ONE = builtin_constant(
        101,
        value=lambda: Vector4i(x=1, y=1, z=1, w=1),
        description="The one Vector4i.",
    )
    X_AXIS = builtin_constant(
        110,
        value=lambda: Vector4i(x=1, y=0, z=0, w=0),
        description="The x-axis Vector4i.",
    )
    Y_AXIS = builtin_constant(
        111,
        value=lambda: Vector4i(x=0, y=1, z=0, w=0),
        description="The y-axis Vector4i.",
    )
    Z_AXIS = builtin_constant(
        112,
        value=lambda: Vector4i(x=0, y=0, z=1, w=0),
        description="The z-axis Vector4i.",
    )
    W_AXIS = builtin_constant(
        113,
        value=lambda: Vector4i(x=0, y=0, z=0, w=1),
        description="The w-axis Vector4i.",
    )

    x: Int32 = builtin_property(
        101,
        is_repr=True,
        description="The x-coordinate of the Vector4i.",
    )
    y: Int32 = builtin_property(
        102,
        is_repr=True,
        description="The y-coordinate of the Vector4i.",
    )
    z: Int32 = builtin_property(
        103,
        is_repr=True,
        description="The z-coordinate of the Vector4i.",
    )
    w: Int32 = builtin_property(
        104,
        is_repr=True,
        description="The w-coordinate of the Vector4i.",
    )

    @builtin_method(101)
    def add(self, other: "Vector4i | int") -> "Vector4i":
        """Add a Vector4i or a scalar to a Vector4i."""
        if isinstance(other, Vector4i):
            return Vector4i(
                x=self.x + other.x, y=self.y + other.y, z=self.z + other.z, w=self.w + other.w
            )
        else:
            return Vector4i(x=self.x + other, y=self.y + other, z=self.z + other, w=self.w + other)

    @builtin_method(
        102,
        operator=FunctionOperator.ADD,
        proxies_method="add",
        languages=(RuntimeLanguage.PYTHON,),
    )
    def __add__(self, other: "Vector4i | int") -> "Vector4i":
        """Add a Vector4i or a scalar to a Vector4i."""
        return self.add(other)

    @builtin_method(103)
    def sub(self, other: "Vector4i | int") -> "Vector4i":
        """Subtract a Vector4i or a scalar from a Vector4i."""
        if isinstance(other, Vector4i):
            return Vector4i(
                x=self.x - other.x, y=self.y - other.y, z=self.z - other.z, w=self.w - other.w
            )
        else:
            return Vector4i(x=self.x - other, y=self.y - other, z=self.z - other, w=self.w - other)

    @builtin_method(
        104,
        operator=FunctionOperator.SUB,
        proxies_method="sub",
        languages=(RuntimeLanguage.PYTHON,),
    )
    def __sub__(self, other: "Vector4i | int") -> "Vector4i":
        """Subtract a Vector4i or a scalar from a Vector4i."""
        return self.sub(other)

    @builtin_method(105)
    def mul(self, other: "Vector4i | int") -> "Vector4i":
        """Multiply a Vector4i or a scalar by a Vector4i."""
        if isinstance(other, Vector4i):
            return Vector4i(
                x=self.x * other.x, y=self.y * other.y, z=self.z * other.z, w=self.w * other.w
            )
        else:
            return Vector4i(x=self.x * other, y=self.y * other, z=self.z * other, w=self.w * other)

    @builtin_method(
        106,
        operator=FunctionOperator.MUL,
        proxies_method="mul",
        languages=(RuntimeLanguage.PYTHON,),
    )
    def __mul__(self, other: "Vector4i | int") -> "Vector4i":
        """Multiply a Vector4i or a scalar by a Vector4i."""
        return self.mul(other)

    @builtin_method(107)
    def truediv(self, other: "Vector4i | int") -> "Vector4i":
        """Divide a Vector4i or a scalar by a Vector4i."""
        if isinstance(other, Vector4i):
            return Vector4i(
                x=self.x // other.x, y=self.y // other.y, z=self.z // other.z, w=self.w // other.w
            )
        else:
            return Vector4i(
                x=self.x // other, y=self.y // other, z=self.z // other, w=self.w // other
            )

    @builtin_method(
        108,
        operator=FunctionOperator.TRUEDIV,
        proxies_method="truediv",
        languages=(RuntimeLanguage.PYTHON,),
    )
    def __truediv__(self, other: "Vector4i | int") -> "Vector4i":
        """Divide a Vector4i or a scalar by a Vector4i."""
        return self.truediv(other)

    @builtin_method(109)
    def abs(self) -> "Vector4i":
        """Get the absolute value of a Vector4i."""
        return Vector4i(x=abs(self.x), y=abs(self.y), z=abs(self.z), w=abs(self.w))

    @builtin_method(
        110,
        operator=FunctionOperator.ABS,
        proxies_method="abs",
        languages=(RuntimeLanguage.PYTHON,),
    )
    def __abs__(self) -> "Vector4i":
        """Get the absolute value of a Vector4i."""
        return self.abs()

    @builtin_method(111)
    def neg(self) -> "Vector4i":
        """Negate a vector."""
        return Vector4i(x=-self.x, y=-self.y, z=-self.z, w=-self.w)

    @builtin_method(
        112,
        operator=FunctionOperator.NEG,
        proxies_method="neg",
        languages=(RuntimeLanguage.PYTHON,),
    )
    def __neg__(self) -> "Vector4i":
        """Negate a vector."""
        return self.neg()

    @builtin_method(
        113,
        operator=FunctionOperator.ADD,
        proxies_method="add",
        languages=(RuntimeLanguage.PYTHON,),
    )
    def __radd__(self, other: "Vector4i | int") -> "Vector4i":
        """Add a Vector4i or a scalar to a Vector4i."""
        return self.add(other)

    @builtin_method(
        114,
        operator=FunctionOperator.SUB,
        proxies_method="sub",
        languages=(RuntimeLanguage.PYTHON,),
    )
    def __rsub__(self, other: "Vector4i | int") -> "Vector4i":
        """Subtract a Vector4i or a scalar from a Vector4i."""
        if isinstance(other, Vector4i):
            return Vector4i(
                x=other.x - self.x, y=other.y - self.y, z=other.z - self.z, w=other.w - self.w
            )
        else:
            return Vector4i(x=other - self.x, y=other - self.y, z=other - self.z, w=other - self.w)

    @builtin_method(
        115,
        operator=FunctionOperator.MUL,
        proxies_method="mul",
        languages=(RuntimeLanguage.PYTHON,),
    )
    def __rmul__(self, other: "Vector4i | int") -> "Vector4i":
        """Multiply a Vector4i or a scalar by a Vector4i."""
        return self.mul(other)

    @builtin_method(
        116,
        operator=FunctionOperator.TRUEDIV,
        proxies_method="truediv",
        languages=(RuntimeLanguage.PYTHON,),
    )
    def __rtruediv__(self, other: "Vector4i | int") -> "Vector4i":
        """Divide a Vector4i or a scalar by a Vector4i."""
        if isinstance(other, Vector4i):
            return Vector4i(
                x=other.x // self.x, y=other.y // self.y, z=other.z // self.z, w=other.w // self.w
            )
        else:
            return Vector4i(
                x=other // self.x, y=other // self.y, z=other // self.z, w=other // self.w
            )

    @builtin_method(117)
    def dot(self, other: "Vector4i") -> int:
        """Calculate the dot product with another vector."""
        return self.x * other.x + self.y * other.y + self.z * other.z + self.w * other.w

    @builtin_method(118)
    def magnitude(self) -> float:
        """Calculate the magnitude (length) of the vector."""
        return (self.x**2 + self.y**2 + self.z**2 + self.w**2) ** 0.5

    @builtin_method(119)
    def magnitude2(self) -> int:
        """Calculate the squared magnitude of the vector."""
        return self.x**2 + self.y**2 + self.z**2 + self.w**2

    @builtin_method(120)
    def normalize(self) -> "Vector4":
        """Return a normalized (unit) vector as floating point vector."""
        mag = self.magnitude()
        if mag == 0:
            return Vector4(x=0.0, y=0.0, z=0.0, w=0.0)
        return Vector4(x=self.x / mag, y=self.y / mag, z=self.z / mag, w=self.w / mag)

    @builtin_method(121)
    def distance(self, other: "Vector4i") -> float:
        """Calculate the distance to another vector."""
        return (self - other).magnitude()

    @builtin_method(122)
    def distance2(self, other: "Vector4i") -> int:
        """Calculate the squared distance to another vector."""
        diff = self - other
        return diff.x**2 + diff.y**2 + diff.z**2 + diff.w**2

    @builtin_method(123)
    def angle(self, other: "Vector4i") -> float:
        """Calculate the angle to another vector in radians."""
        dot_product = self.dot(other)
        mag_product = self.magnitude() * other.magnitude()
        if mag_product == 0:
            return 0.0
        return math.acos(max(-1.0, min(1.0, dot_product / mag_product)))

    @builtin_method(124)
    def lerp(self, other: "Vector4i", t: float) -> "Vector4":
        """Linear interpolation between this vector and another as floating point vector."""
        return Vector4(
            x=self.x + (other.x - self.x) * t,
            y=self.y + (other.y - self.y) * t,
            z=self.z + (other.z - self.z) * t,
            w=self.w + (other.w - self.w) * t,
        )

    @builtin_method(
        125,
        operator=FunctionOperator.ITER,
        languages=(RuntimeLanguage.PYTHON,),
    )
    def __iter__(self) -> Iterator[int]:
        yield self.x
        yield self.y
        yield self.z
        yield self.w

    @builtin_method(
        126,
        operator=FunctionOperator.GETITEM,
        languages=(RuntimeLanguage.PYTHON,),
    )
    def __getitem__(self, index: int) -> int:
        if index == 0:
            return self.x
        elif index == 1:
            return self.y
        elif index == 2:
            return self.z
        elif index == 3:
            return self.w
        else:
            raise IndexError(f"index out of range: {index}")

    @builtin_method(
        127,
        operator=FunctionOperator.LEN,
        languages=(RuntimeLanguage.PYTHON,),
    )
    def __len__(self) -> int:
        return 4
