from collections.abc import Iterator
from typing import TYPE_CHECKING

from destack.language.core import (
    StructFrozen,
    StructType,
    builtin_method,
    builtin_property,
    builtin_struct,
)

if TYPE_CHECKING:
    pass


@builtin_struct(StructType.VECTOR, frozen=True, is_abstract=True)
class Vector(StructFrozen):
    """A Vector."""

    pass


@builtin_struct(StructType.VECTORF, frozen=True, is_abstract=True)
class Vectorf(Vector):
    """A floating point Vector."""

    pass


@builtin_struct(StructType.VECTORI, frozen=True, is_abstract=True)
class Vectori(Vector):
    """An integer Vector."""

    pass


@builtin_struct(StructType.VECTOR2, frozen=True)
class Vector2(Vectorf):
    """A 2D floating point Vector."""

    x: float = builtin_property(
        101,
        is_repr=True,
        description="The x-coordinate of the Vector2.",
    )
    y: float = builtin_property(
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

    @builtin_method(102)
    def sub(self, other: "Vector2 | float") -> "Vector2":
        """Subtract a Vector2 or a scalar from a Vector2."""
        if isinstance(other, Vector2):
            return Vector2(x=self.x - other.x, y=self.y - other.y)
        else:
            return Vector2(x=self.x - other, y=self.y - other)

    @builtin_method(103)
    def mul(self, other: "Vector2 | float") -> "Vector2":
        """Multiply a Vector2 or a scalar by a Vector2."""
        if isinstance(other, Vector2):
            return Vector2(x=self.x * other.x, y=self.y * other.y)
        else:
            return Vector2(x=self.x * other, y=self.y * other)

    @builtin_method(104)
    def truediv(self, other: "Vector2 | float") -> "Vector2":
        """Divide a Vector2 or a scalar by a Vector2."""
        if isinstance(other, Vector2):
            return Vector2(x=self.x / other.x, y=self.y / other.y)
        else:
            return Vector2(x=self.x / other, y=self.y / other)

    @builtin_method(109)
    def abs(self) -> "Vector2":
        """Get the absolute value of a Vector2."""
        return Vector2(x=abs(self.x), y=abs(self.y))

    @builtin_method(110)
    def neg(self) -> "Vector2":
        """Negate a vector."""
        return Vector2(x=-self.x, y=-self.y)

    __add__ = add
    __sub__ = sub
    __mul__ = mul
    __truediv__ = truediv
    __radd__ = add
    __rsub__ = sub
    __rtruediv__ = truediv
    __neg__ = neg
    __abs__ = abs

    @builtin_method(111)
    def perp(self) -> "Vector2":
        """Get the perpendicular vector (rotated 90 degrees counterclockwise)."""
        return Vector2(x=self.y, y=-self.x)

    @builtin_method(112)
    def dot(self, other: "Vector2") -> float:
        """Calculate the dot product with another vector."""
        return self.x * other.x + self.y * other.y

    @builtin_method(113)
    def magnitude(self) -> float:
        """Calculate the magnitude (length) of the vector."""
        return (self.x**2 + self.y**2) ** 0.5

    @builtin_method(114)
    def magnitude2(self) -> float:
        """Calculate the squared magnitude of the vector."""
        return self.x**2 + self.y**2

    @builtin_method(115)
    def normalize(self) -> "Vector2":
        """Return a normalized (unit) vector."""
        mag = self.magnitude()
        if mag == 0:
            return Vector2(x=0.0, y=0.0)
        return Vector2(x=self.x / mag, y=self.y / mag)

    @builtin_method(116)
    def distance(self, other: "Vector2") -> float:
        """Calculate the distance to another vector."""
        return (self - other).magnitude()

    @builtin_method(117)
    def distance2(self, other: "Vector2") -> float:
        """Calculate the squared distance to another vector."""
        diff = self - other
        return diff.x**2 + diff.y**2

    @builtin_method(118)
    def angle(self, other: "Vector2") -> float:
        """Calculate the angle to another vector in radians."""
        import math

        return math.atan2(other.y - self.y, other.x - self.x)

    @builtin_method(119)
    def lerp(self, other: "Vector2", t: float) -> "Vector2":
        """Linear interpolation between this vector and another."""
        return Vector2(x=self.x + (other.x - self.x) * t, y=self.y + (other.y - self.y) * t)

    @builtin_method(120)
    def rot_with(self, center: "Vector2", angle: float) -> "Vector2":
        """Rotate this vector around another point by the given angle."""
        import math

        x = self.x - center.x
        y = self.y - center.y
        s = math.sin(angle)
        c = math.cos(angle)
        return Vector2(x=center.x + (x * c - y * s), y=center.y + (x * s + y * c))

    def __iter__(self) -> Iterator[float]:
        yield self.x
        yield self.y

    def __getitem__(self, index: int) -> float:
        if index == 0:
            return self.x
        elif index == 1:
            return self.y
        else:
            raise IndexError(f"index out of range: {index}")

    def __len__(self) -> int:
        return 2


@builtin_struct(StructType.VECTOR3, frozen=True)
class Vector3(Vectorf):
    """A 3D floating point vector."""

    x: float = builtin_property(
        101,
        is_repr=True,
        description="The x-coordinate of the Vector3.",
    )
    y: float = builtin_property(
        102,
        is_repr=True,
        description="The y-coordinate of the Vector3.",
    )
    z: float = builtin_property(
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

    @builtin_method(102)
    def sub(self, other: "Vector3 | float") -> "Vector3":
        """Subtract a Vector3 or a scalar from a Vector3."""
        if isinstance(other, Vector3):
            return Vector3(x=self.x - other.x, y=self.y - other.y, z=self.z - other.z)
        else:
            return Vector3(x=self.x - other, y=self.y - other, z=self.z - other)

    @builtin_method(103)
    def mul(self, other: "Vector3 | float") -> "Vector3":
        """Multiply a Vector3 or a scalar by a Vector3."""
        if isinstance(other, Vector3):
            return Vector3(x=self.x * other.x, y=self.y * other.y, z=self.z * other.z)
        else:
            return Vector3(x=self.x * other, y=self.y * other, z=self.z * other)

    @builtin_method(104)
    def truediv(self, other: "Vector3 | float") -> "Vector3":
        """Divide a Vector3 or a scalar by a Vector3."""
        if isinstance(other, Vector3):
            return Vector3(x=self.x / other.x, y=self.y / other.y, z=self.z / other.z)
        else:
            return Vector3(x=self.x / other, y=self.y / other, z=self.z / other)

    @builtin_method(109)
    def neg(self) -> "Vector3":
        """Negate a vector."""
        return Vector3(x=-self.x, y=-self.y, z=-self.z)

    @builtin_method(110)
    def abs(self) -> "Vector3":
        """Get the absolute value of a Vector3."""
        return Vector3(x=abs(self.x), y=abs(self.y), z=abs(self.z))

    __add__ = add
    __sub__ = sub
    __mul__ = mul
    __truediv__ = truediv
    __rmul__ = mul
    __radd__ = add
    __rsub__ = sub
    __rtruediv__ = truediv
    __neg__ = neg
    __abs__ = abs

    @builtin_method(111)
    def dot(self, other: "Vector3") -> float:
        """Calculate the dot product with another vector."""
        return self.x * other.x + self.y * other.y + self.z * other.z

    @builtin_method(112)
    def magnitude(self) -> float:
        """Calculate the magnitude (length) of the vector."""
        return (self.x**2 + self.y**2 + self.z**2) ** 0.5

    @builtin_method(113)
    def magnitude2(self) -> float:
        """Calculate the squared magnitude of the vector."""
        return self.x**2 + self.y**2 + self.z**2

    @builtin_method(114)
    def normalize(self) -> "Vector3":
        """Return a normalized (unit) vector."""
        mag = self.magnitude()
        if mag == 0:
            return Vector3(x=0.0, y=0.0, z=0.0)
        return Vector3(x=self.x / mag, y=self.y / mag, z=self.z / mag)

    @builtin_method(115)
    def distance(self, other: "Vector3") -> float:
        """Calculate the distance to another vector."""
        return (self - other).magnitude()

    @builtin_method(116)
    def distance2(self, other: "Vector3") -> float:
        """Calculate the squared distance to another vector."""
        diff = self - other
        return diff.x**2 + diff.y**2 + diff.z**2

    @builtin_method(117)
    def angle(self, other: "Vector3") -> float:
        """Calculate the angle to another vector in radians."""
        import math

        dot_product = self.dot(other)
        mag_product = self.magnitude() * other.magnitude()
        if mag_product == 0:
            return 0.0
        return math.acos(max(-1.0, min(1.0, dot_product / mag_product)))

    @builtin_method(118)
    def lerp(self, other: "Vector3", t: float) -> "Vector3":
        """Linear interpolation between this vector and another."""
        return Vector3(
            x=self.x + (other.x - self.x) * t,
            y=self.y + (other.y - self.y) * t,
            z=self.z + (other.z - self.z) * t,
        )

    @builtin_method(119)
    def cross(self, other: "Vector3") -> "Vector3":
        """Calculate the cross product with another vector."""
        return Vector3(
            x=self.y * other.z - self.z * other.y,
            y=self.z * other.x - self.x * other.z,
            z=self.x * other.y - self.y * other.x,
        )

    def __iter__(self) -> Iterator[float]:
        yield self.x
        yield self.y
        yield self.z

    def __getitem__(self, index: int) -> float:
        if index == 0:
            return self.x
        elif index == 1:
            return self.y
        elif index == 2:
            return self.z
        else:
            raise IndexError(f"index out of range: {index}")

    def __len__(self) -> int:
        return 3


@builtin_struct(StructType.VECTOR4, frozen=True)
class Vector4(Vectorf):
    """A 4D floating point vector."""

    x: float = builtin_property(
        101,
        is_repr=True,
        description="The x-coordinate of the Vector4.",
    )
    y: float = builtin_property(
        102,
        is_repr=True,
        description="The y-coordinate of the Vector4.",
    )
    z: float = builtin_property(
        103,
        is_repr=True,
        description="The z-coordinate of the Vector4.",
    )
    w: float = builtin_property(
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

    @builtin_method(102)
    def sub(self, other: "Vector4 | float") -> "Vector4":
        """Subtract a Vector4 or a scalar from a Vector4."""
        if isinstance(other, Vector4):
            return Vector4(
                x=self.x - other.x, y=self.y - other.y, z=self.z - other.z, w=self.w - other.w
            )
        else:
            return Vector4(x=self.x - other, y=self.y - other, z=self.z - other, w=self.w - other)

    @builtin_method(103)
    def mul(self, other: "Vector4 | float") -> "Vector4":
        """Multiply a Vector4 or a scalar by a Vector4."""
        if isinstance(other, Vector4):
            return Vector4(
                x=self.x * other.x, y=self.y * other.y, z=self.z * other.z, w=self.w * other.w
            )
        else:
            return Vector4(x=self.x * other, y=self.y * other, z=self.z * other, w=self.w * other)

    @builtin_method(104)
    def truediv(self, other: "Vector4 | float") -> "Vector4":
        """Divide a Vector4 or a scalar by a Vector4."""
        if isinstance(other, Vector4):
            return Vector4(
                x=self.x / other.x, y=self.y / other.y, z=self.z / other.z, w=self.w / other.w
            )
        else:
            return Vector4(x=self.x / other, y=self.y / other, z=self.z / other, w=self.w / other)

    @builtin_method(109)
    def neg(self) -> "Vector4":
        """Negate a vector."""
        return Vector4(x=-self.x, y=-self.y, z=-self.z, w=-self.w)

    @builtin_method(110)
    def abs(self) -> "Vector4":
        """Get the absolute value of a Vector4."""
        return Vector4(x=abs(self.x), y=abs(self.y), z=abs(self.z), w=abs(self.w))

    __add__ = add
    __sub__ = sub
    __mul__ = mul
    __truediv__ = truediv
    __rmul__ = mul
    __radd__ = add
    __rsub__ = sub
    __rtruediv__ = truediv
    __neg__ = neg
    __abs__ = abs

    @builtin_method(111)
    def dot(self, other: "Vector4") -> float:
        """Calculate the dot product with another vector."""
        return self.x * other.x + self.y * other.y + self.z * other.z + self.w * other.w

    @builtin_method(112)
    def magnitude(self) -> float:
        """Calculate the magnitude (length) of the vector."""
        return (self.x**2 + self.y**2 + self.z**2 + self.w**2) ** 0.5

    @builtin_method(113)
    def magnitude2(self) -> float:
        """Calculate the squared magnitude of the vector."""
        return self.x**2 + self.y**2 + self.z**2 + self.w**2

    @builtin_method(114)
    def normalize(self) -> "Vector4":
        """Return a normalized (unit) vector."""
        mag = self.magnitude()
        if mag == 0:
            return Vector4(x=0.0, y=0.0, z=0.0, w=0.0)
        return Vector4(x=self.x / mag, y=self.y / mag, z=self.z / mag, w=self.w / mag)

    @builtin_method(115)
    def distance(self, other: "Vector4") -> float:
        """Calculate the distance to another vector."""
        return (self - other).magnitude()

    @builtin_method(116)
    def distance2(self, other: "Vector4") -> float:
        """Calculate the squared distance to another vector."""
        diff = self - other
        return diff.x**2 + diff.y**2 + diff.z**2 + diff.w**2

    @builtin_method(117)
    def angle(self, other: "Vector4") -> float:
        """Calculate the angle to another vector in radians."""
        import math

        dot_product = self.dot(other)
        mag_product = self.magnitude() * other.magnitude()
        if mag_product == 0:
            return 0.0
        return math.acos(max(-1.0, min(1.0, dot_product / mag_product)))

    @builtin_method(118)
    def lerp(self, other: "Vector4", t: float) -> "Vector4":
        """Linear interpolation between this vector and another."""
        return Vector4(
            x=self.x + (other.x - self.x) * t,
            y=self.y + (other.y - self.y) * t,
            z=self.z + (other.z - self.z) * t,
            w=self.w + (other.w - self.w) * t,
        )

    def __iter__(self) -> Iterator[float]:
        yield self.x
        yield self.y
        yield self.z
        yield self.w

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

    def __len__(self) -> int:
        return 4


@builtin_struct(StructType.VECTOR2I, frozen=True)
class Vector2i(Vectori):
    """A 2D integer vector."""

    x: int = builtin_property(
        101,
        is_repr=True,
        description="The x-coordinate of the Vector2i.",
    )
    y: int = builtin_property(
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

    @builtin_method(102)
    def sub(self, other: "Vector2i | int") -> "Vector2i":
        """Subtract a Vector2i or a scalar from a Vector2i."""
        if isinstance(other, Vector2i):
            return Vector2i(x=self.x - other.x, y=self.y - other.y)
        else:
            return Vector2i(x=self.x - other, y=self.y - other)

    @builtin_method(103)
    def mul(self, other: "Vector2i | int") -> "Vector2i":
        """Multiply a Vector2i or a scalar by a Vector2i."""
        if isinstance(other, Vector2i):
            return Vector2i(x=self.x * other.x, y=self.y * other.y)
        else:
            return Vector2i(x=self.x * other, y=self.y * other)

    @builtin_method(104)
    def truediv(self, other: "Vector2i | int") -> "Vector2i":
        """Divide a Vector2i or a scalar by a Vector2i."""
        if isinstance(other, Vector2i):
            return Vector2i(x=self.x // other.x, y=self.y // other.y)
        else:
            return Vector2i(x=self.x // other, y=self.y // other)

    @builtin_method(109)
    def neg(self) -> "Vector2i":
        """Negate a vector."""
        return Vector2i(x=-self.x, y=-self.y)

    @builtin_method(110)
    def abs(self) -> "Vector2i":
        """Get the absolute value of a Vector2i."""
        return Vector2i(x=abs(self.x), y=abs(self.y))

    __add__ = add
    __sub__ = sub
    __mul__ = mul
    __truediv__ = truediv
    __rmul__ = mul
    __radd__ = add
    __rsub__ = sub
    __rtruediv__ = truediv
    __neg__ = neg
    __abs__ = abs

    @builtin_method(111)
    def dot(self, other: "Vector2i") -> int:
        """Calculate the dot product with another vector."""
        return self.x * other.x + self.y * other.y

    @builtin_method(112)
    def magnitude(self) -> float:
        """Calculate the magnitude (length) of the vector."""
        return (self.x**2 + self.y**2) ** 0.5

    @builtin_method(113)
    def magnitude2(self) -> int:
        """Calculate the squared magnitude of the vector."""
        return self.x**2 + self.y**2

    @builtin_method(114)
    def normalize(self) -> "Vector2":
        """Return a normalized (unit) vector as floating point vector."""
        mag = self.magnitude()
        if mag == 0:
            return Vector2(x=0.0, y=0.0)
        return Vector2(x=self.x / mag, y=self.y / mag)

    @builtin_method(115)
    def distance(self, other: "Vector2i") -> float:
        """Calculate the distance to another vector."""
        return (self - other).magnitude()

    @builtin_method(116)
    def distance2(self, other: "Vector2i") -> int:
        """Calculate the squared distance to another vector."""
        diff = self - other
        return diff.x**2 + diff.y**2

    @builtin_method(117)
    def angle(self, other: "Vector2i") -> float:
        """Calculate the angle to another vector in radians."""
        import math

        return math.atan2(other.y - self.y, other.x - self.x)

    @builtin_method(118)
    def lerp(self, other: "Vector2i", t: float) -> "Vector2":
        """Linear interpolation between this vector and another as floating point vector."""
        return Vector2(x=self.x + (other.x - self.x) * t, y=self.y + (other.y - self.y) * t)

    @builtin_method(119)
    def rot_with(self, center: "Vector2i", angle: float) -> "Vector2":
        """Rotate this vector around another point by the given angle as floating point vector."""
        import math

        x = self.x - center.x
        y = self.y - center.y
        s = math.sin(angle)
        c = math.cos(angle)
        return Vector2(x=center.x + (x * c - y * s), y=center.y + (x * s + y * c))

    def __iter__(self) -> Iterator[int]:
        yield self.x
        yield self.y

    def __getitem__(self, index: int) -> int:
        if index == 0:
            return self.x
        elif index == 1:
            return self.y
        else:
            raise IndexError(f"index out of range: {index}")

    def __len__(self) -> int:
        return 2


@builtin_struct(StructType.VECTOR3I, frozen=True)
class Vector3i(Vectori):
    """A 3D integer vector."""

    x: int = builtin_property(
        101,
        is_repr=True,
        description="The x-coordinate of the Vector3i.",
    )
    y: int = builtin_property(
        102,
        is_repr=True,
        description="The y-coordinate of the Vector3i.",
    )
    z: int = builtin_property(
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

    @builtin_method(102)
    def sub(self, other: "Vector3i | int") -> "Vector3i":
        """Subtract a Vector3i or a scalar from a Vector3i."""
        if isinstance(other, Vector3i):
            return Vector3i(x=self.x - other.x, y=self.y - other.y, z=self.z - other.z)
        else:
            return Vector3i(x=self.x - other, y=self.y - other, z=self.z - other)

    @builtin_method(103)
    def mul(self, other: "Vector3i | int") -> "Vector3i":
        """Multiply a Vector3i or a scalar by a Vector3i."""
        if isinstance(other, Vector3i):
            return Vector3i(x=self.x * other.x, y=self.y * other.y, z=self.z * other.z)
        else:
            return Vector3i(x=self.x * other, y=self.y * other, z=self.z * other)

    @builtin_method(104)
    def truediv(self, other: "Vector3i | int") -> "Vector3i":
        """Divide a Vector3i or a scalar by a Vector3i."""
        if isinstance(other, Vector3i):
            return Vector3i(x=self.x // other.x, y=self.y // other.y, z=self.z // other.z)
        else:
            return Vector3i(x=self.x // other, y=self.y // other, z=self.z // other)

    @builtin_method(109)
    def neg(self) -> "Vector3i":
        """Negate a vector."""
        return Vector3i(x=-self.x, y=-self.y, z=-self.z)

    @builtin_method(110)
    def abs(self) -> "Vector3i":
        """Get the absolute value of a Vector3i."""
        return Vector3i(x=abs(self.x), y=abs(self.y), z=abs(self.z))

    __add__ = add
    __sub__ = sub
    __mul__ = mul
    __truediv__ = truediv
    __rmul__ = mul
    __radd__ = add
    __rsub__ = sub
    __rtruediv__ = truediv
    __neg__ = neg
    __abs__ = abs

    @builtin_method(111)
    def dot(self, other: "Vector3i") -> int:
        """Calculate the dot product with another vector."""
        return self.x * other.x + self.y * other.y + self.z * other.z

    @builtin_method(112)
    def magnitude(self) -> float:
        """Calculate the magnitude (length) of the vector."""
        return (self.x**2 + self.y**2 + self.z**2) ** 0.5

    @builtin_method(113)
    def magnitude2(self) -> int:
        """Calculate the squared magnitude of the vector."""
        return self.x**2 + self.y**2 + self.z**2

    @builtin_method(114)
    def normalize(self) -> "Vector3":
        """Return a normalized (unit) vector as floating point vector."""
        mag = self.magnitude()
        if mag == 0:
            return Vector3(x=0.0, y=0.0, z=0.0)
        return Vector3(x=self.x / mag, y=self.y / mag, z=self.z / mag)

    @builtin_method(115)
    def distance(self, other: "Vector3i") -> float:
        """Calculate the distance to another vector."""
        return (self - other).magnitude()

    @builtin_method(116)
    def distance2(self, other: "Vector3i") -> int:
        """Calculate the squared distance to another vector."""
        diff = self - other
        return diff.x**2 + diff.y**2 + diff.z**2

    @builtin_method(117)
    def angle(self, other: "Vector3i") -> float:
        """Calculate the angle to another vector in radians."""
        import math

        dot_product = self.dot(other)
        mag_product = self.magnitude() * other.magnitude()
        if mag_product == 0:
            return 0.0
        return math.acos(max(-1.0, min(1.0, dot_product / mag_product)))

    @builtin_method(118)
    def lerp(self, other: "Vector3i", t: float) -> "Vector3":
        """Linear interpolation between this vector and another as floating point vector."""
        return Vector3(
            x=self.x + (other.x - self.x) * t,
            y=self.y + (other.y - self.y) * t,
            z=self.z + (other.z - self.z) * t,
        )

    @builtin_method(119)
    def cross(self, other: "Vector3i") -> "Vector3i":
        """Calculate the cross product with another vector."""
        return Vector3i(
            x=self.y * other.z - self.z * other.y,
            y=self.z * other.x - self.x * other.z,
            z=self.x * other.y - self.y * other.x,
        )

    def __iter__(self) -> Iterator[int]:
        yield self.x
        yield self.y
        yield self.z

    def __getitem__(self, index: int) -> int:
        if index == 0:
            return self.x
        elif index == 1:
            return self.y
        elif index == 2:
            return self.z
        else:
            raise IndexError(f"index out of range: {index}")

    def __len__(self) -> int:
        return 3


@builtin_struct(StructType.VECTOR4I, frozen=True)
class Vector4i(Vectori):
    """A 4D integer vector."""

    x: int = builtin_property(
        101,
        is_repr=True,
        description="The x-coordinate of the Vector4i.",
    )
    y: int = builtin_property(
        102,
        is_repr=True,
        description="The y-coordinate of the Vector4i.",
    )
    z: int = builtin_property(
        103,
        is_repr=True,
        description="The z-coordinate of the Vector4i.",
    )
    w: int = builtin_property(
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

    @builtin_method(102)
    def sub(self, other: "Vector4i | int") -> "Vector4i":
        """Subtract a Vector4i or a scalar from a Vector4i."""
        if isinstance(other, Vector4i):
            return Vector4i(
                x=self.x - other.x, y=self.y - other.y, z=self.z - other.z, w=self.w - other.w
            )
        else:
            return Vector4i(x=self.x - other, y=self.y - other, z=self.z - other, w=self.w - other)

    @builtin_method(103)
    def mul(self, other: "Vector4i | int") -> "Vector4i":
        """Multiply a Vector4i or a scalar by a Vector4i."""
        if isinstance(other, Vector4i):
            return Vector4i(
                x=self.x * other.x, y=self.y * other.y, z=self.z * other.z, w=self.w * other.w
            )
        else:
            return Vector4i(x=self.x * other, y=self.y * other, z=self.z * other, w=self.w * other)

    @builtin_method(104)
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

    @builtin_method(109)
    def neg(self) -> "Vector4i":
        """Negate a vector."""
        return Vector4i(x=-self.x, y=-self.y, z=-self.z, w=-self.w)

    @builtin_method(110)
    def abs(self) -> "Vector4i":
        """Get the absolute value of a Vector4i."""
        return Vector4i(x=abs(self.x), y=abs(self.y), z=abs(self.z), w=abs(self.w))

    __add__ = add
    __sub__ = sub
    __mul__ = mul
    __truediv__ = truediv
    __rmul__ = mul
    __radd__ = add
    __rsub__ = sub
    __rtruediv__ = truediv
    __neg__ = neg
    __abs__ = abs

    @builtin_method(111)
    def dot(self, other: "Vector4i") -> int:
        """Calculate the dot product with another vector."""
        return self.x * other.x + self.y * other.y + self.z * other.z + self.w * other.w

    @builtin_method(112)
    def magnitude(self) -> float:
        """Calculate the magnitude (length) of the vector."""
        return (self.x**2 + self.y**2 + self.z**2 + self.w**2) ** 0.5

    @builtin_method(113)
    def magnitude2(self) -> int:
        """Calculate the squared magnitude of the vector."""
        return self.x**2 + self.y**2 + self.z**2 + self.w**2

    @builtin_method(114)
    def normalize(self) -> "Vector4":
        """Return a normalized (unit) vector as floating point vector."""
        mag = self.magnitude()
        if mag == 0:
            return Vector4(x=0.0, y=0.0, z=0.0, w=0.0)
        return Vector4(x=self.x / mag, y=self.y / mag, z=self.z / mag, w=self.w / mag)

    @builtin_method(115)
    def distance(self, other: "Vector4i") -> float:
        """Calculate the distance to another vector."""
        return (self - other).magnitude()

    @builtin_method(116)
    def distance2(self, other: "Vector4i") -> int:
        """Calculate the squared distance to another vector."""
        diff = self - other
        return diff.x**2 + diff.y**2 + diff.z**2 + diff.w**2

    @builtin_method(117)
    def angle(self, other: "Vector4i") -> float:
        """Calculate the angle to another vector in radians."""
        import math

        dot_product = self.dot(other)
        mag_product = self.magnitude() * other.magnitude()
        if mag_product == 0:
            return 0.0
        return math.acos(max(-1.0, min(1.0, dot_product / mag_product)))

    @builtin_method(118)
    def lerp(self, other: "Vector4i", t: float) -> "Vector4":
        """Linear interpolation between this vector and another as floating point vector."""
        return Vector4(
            x=self.x + (other.x - self.x) * t,
            y=self.y + (other.y - self.y) * t,
            z=self.z + (other.z - self.z) * t,
            w=self.w + (other.w - self.w) * t,
        )

    def __iter__(self) -> Iterator[int]:
        yield self.x
        yield self.y
        yield self.z
        yield self.w

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

    def __len__(self) -> int:
        return 4


def vector2(x: float, y: float) -> "Vector2":
    return Vector2(x=float(x), y=float(y))


def vector3(x: float, y: float, z: float) -> "Vector3":
    return Vector3(x=float(x), y=float(y), z=float(z))


def vector4(x: float, y: float, z: float, w: float) -> "Vector4":
    return Vector4(x=float(x), y=float(y), z=float(z), w=float(w))


def vector2i(x: int, y: int) -> "Vector2i":
    return Vector2i(x=int(x), y=int(y))


def vector3i(x: int, y: int, z: int) -> "Vector3i":
    return Vector3i(x=int(x), y=int(y), z=int(z))


def vector4i(x: int, y: int, z: int, w: int) -> "Vector4i":
    return Vector4i(x=int(x), y=int(y), z=int(z), w=int(w))
