from collections.abc import Iterator
from typing import TYPE_CHECKING

from ..builtin import (
    StructFrozen,
    StructType,
    builtin_struct,
    property_,
)

if TYPE_CHECKING:
    pass


@builtin_struct(StructType.VECTOR2, frozen=True)
class Vector2(StructFrozen):
    """A 2D float vector."""

    x: float = property_(50, is_repr=True)
    y: float = property_(51, is_repr=True)

    def __add__(self, other: "Vector2 | float") -> "Vector2":
        if isinstance(other, Vector2):
            return Vector2(x=self.x + other.x, y=self.y + other.y)
        else:
            return Vector2(x=self.x + other, y=self.y + other)

    def __sub__(self, other: "Vector2 | float") -> "Vector2":
        if isinstance(other, Vector2):
            return Vector2(x=self.x - other.x, y=self.y - other.y)
        else:
            return Vector2(x=self.x - other, y=self.y - other)

    def __mul__(self, other: "Vector2 | float") -> "Vector2":
        if isinstance(other, Vector2):
            return Vector2(x=self.x * other.x, y=self.y * other.y)
        else:
            return Vector2(x=self.x * other, y=self.y * other)

    def __truediv__(self, other: "Vector2 | float") -> "Vector2":
        if isinstance(other, Vector2):
            return Vector2(x=self.x / other.x, y=self.y / other.y)
        else:
            return Vector2(x=self.x / other, y=self.y / other)

    def __rmul__(self, other: float) -> "Vector2":
        return Vector2(x=other * self.x, y=other * self.y)

    def __radd__(self, other: float) -> "Vector2":
        return Vector2(x=other + self.x, y=other + self.y)

    def __rsub__(self, other: float) -> "Vector2":
        return Vector2(x=other - self.x, y=other - self.y)

    def __rtruediv__(self, other: float) -> "Vector2":
        return Vector2(x=other / self.x, y=other / self.y)

    def __neg__(self) -> "Vector2":
        return Vector2(x=-self.x, y=-self.y)

    def __abs__(self) -> "Vector2":
        return Vector2(x=abs(self.x), y=abs(self.y))

    def neg(self) -> "Vector2":
        """Negate a vector."""
        return Vector2(x=-self.x, y=-self.y)

    def perp(self) -> "Vector2":
        """Get the perpendicular vector (rotated 90 degrees counterclockwise)."""
        return Vector2(x=self.y, y=-self.x)

    def dot(self, other: "Vector2") -> float:
        """Calculate the dot product with another vector."""
        return self.x * other.x + self.y * other.y

    def magnitude(self) -> float:
        """Calculate the magnitude (length) of the vector."""
        return (self.x**2 + self.y**2) ** 0.5

    def normalize(self) -> "Vector2":
        """Return a normalized (unit) vector."""
        mag = self.magnitude()
        if mag == 0:
            return Vector2(x=0.0, y=0.0)
        return Vector2(x=self.x / mag, y=self.y / mag)

    def distance(self, other: "Vector2") -> float:
        """Calculate the distance to another vector."""
        return (self - other).magnitude()

    def distance2(self, other: "Vector2") -> float:
        """Calculate the squared distance to another vector."""
        diff = self - other
        return diff.x**2 + diff.y**2

    def angle(self, other: "Vector2") -> float:
        """Calculate the angle to another vector in radians."""
        import math

        return math.atan2(other.y - self.y, other.x - self.x)

    def lerp(self, other: "Vector2", t: float) -> "Vector2":
        """Linear interpolation between this vector and another."""
        return Vector2(x=self.x + (other.x - self.x) * t, y=self.y + (other.y - self.y) * t)

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
class Vector3(StructFrozen):
    """A 3D float vector."""

    x: float = property_(50, is_repr=True)
    y: float = property_(51, is_repr=True)
    z: float = property_(52, is_repr=True)

    def __add__(self, other: "Vector3 | float") -> "Vector3":
        if isinstance(other, Vector3):
            return Vector3(x=self.x + other.x, y=self.y + other.y, z=self.z + other.z)
        else:
            return Vector3(x=self.x + other, y=self.y + other, z=self.z + other)

    def __sub__(self, other: "Vector3 | float") -> "Vector3":
        if isinstance(other, Vector3):
            return Vector3(x=self.x - other.x, y=self.y - other.y, z=self.z - other.z)
        else:
            return Vector3(x=self.x - other, y=self.y - other, z=self.z - other)

    def __mul__(self, other: "Vector3 | float") -> "Vector3":
        if isinstance(other, Vector3):
            return Vector3(x=self.x * other.x, y=self.y * other.y, z=self.z * other.z)
        else:
            return Vector3(x=self.x * other, y=self.y * other, z=self.z * other)

    def __truediv__(self, other: "Vector3 | float") -> "Vector3":
        if isinstance(other, Vector3):
            return Vector3(x=self.x / other.x, y=self.y / other.y, z=self.z / other.z)
        else:
            return Vector3(x=self.x / other, y=self.y / other, z=self.z / other)

    def __rmul__(self, other: float) -> "Vector3":
        return Vector3(x=other * self.x, y=other * self.y, z=other * self.z)

    def __radd__(self, other: float) -> "Vector3":
        return Vector3(x=other + self.x, y=other + self.y, z=other + self.z)

    def __rsub__(self, other: float) -> "Vector3":
        return Vector3(x=other - self.x, y=other - self.y, z=other - self.z)

    def __rtruediv__(self, other: float) -> "Vector3":
        return Vector3(x=other / self.x, y=other / self.y, z=other / self.z)

    def __neg__(self) -> "Vector3":
        return Vector3(x=-self.x, y=-self.y, z=-self.z)

    def __abs__(self) -> "Vector3":
        return Vector3(x=abs(self.x), y=abs(self.y), z=abs(self.z))

    def dot(self, other: "Vector3") -> float:
        """Calculate the dot product with another vector."""
        return self.x * other.x + self.y * other.y + self.z * other.z

    def magnitude(self) -> float:
        """Calculate the magnitude (length) of the vector."""
        return (self.x**2 + self.y**2 + self.z**2) ** 0.5

    def magnitude2(self) -> float:
        """Calculate the squared magnitude of the vector."""
        return self.x**2 + self.y**2 + self.z**2

    def normalize(self) -> "Vector3":
        """Return a normalized (unit) vector."""
        mag = self.magnitude()
        if mag == 0:
            return Vector3(x=0.0, y=0.0, z=0.0)
        return Vector3(x=self.x / mag, y=self.y / mag, z=self.z / mag)

    def distance(self, other: "Vector3") -> float:
        """Calculate the distance to another vector."""
        return (self - other).magnitude()

    def distance2(self, other: "Vector3") -> float:
        """Calculate the squared distance to another vector."""
        diff = self - other
        return diff.x**2 + diff.y**2 + diff.z**2

    def angle(self, other: "Vector3") -> float:
        """Calculate the angle to another vector in radians."""
        import math

        dot_product = self.dot(other)
        mag_product = self.magnitude() * other.magnitude()
        if mag_product == 0:
            return 0.0
        return math.acos(max(-1.0, min(1.0, dot_product / mag_product)))

    def lerp(self, other: "Vector3", t: float) -> "Vector3":
        """Linear interpolation between this vector and another."""
        return Vector3(
            x=self.x + (other.x - self.x) * t,
            y=self.y + (other.y - self.y) * t,
            z=self.z + (other.z - self.z) * t,
        )

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
class Vector4(StructFrozen):
    """A 4D float vector."""

    x: float = property_(50, is_repr=True)
    y: float = property_(51, is_repr=True)
    z: float = property_(52, is_repr=True)
    w: float = property_(53, is_repr=True)

    def __add__(self, other: "Vector4 | float") -> "Vector4":
        if isinstance(other, Vector4):
            return Vector4(
                x=self.x + other.x, y=self.y + other.y, z=self.z + other.z, w=self.w + other.w
            )
        else:
            return Vector4(x=self.x + other, y=self.y + other, z=self.z + other, w=self.w + other)

    def __sub__(self, other: "Vector4 | float") -> "Vector4":
        if isinstance(other, Vector4):
            return Vector4(
                x=self.x - other.x, y=self.y - other.y, z=self.z - other.z, w=self.w - other.w
            )
        else:
            return Vector4(x=self.x - other, y=self.y - other, z=self.z - other, w=self.w - other)

    def __mul__(self, other: "Vector4 | float") -> "Vector4":
        if isinstance(other, Vector4):
            return Vector4(
                x=self.x * other.x, y=self.y * other.y, z=self.z * other.z, w=self.w * other.w
            )
        else:
            return Vector4(x=self.x * other, y=self.y * other, z=self.z * other, w=self.w * other)

    def __truediv__(self, other: "Vector4 | float") -> "Vector4":
        if isinstance(other, Vector4):
            return Vector4(
                x=self.x / other.x, y=self.y / other.y, z=self.z / other.z, w=self.w / other.w
            )
        else:
            return Vector4(x=self.x / other, y=self.y / other, z=self.z / other, w=self.w / other)

    def __rmul__(self, other: float) -> "Vector4":
        return Vector4(x=other * self.x, y=other * self.y, z=other * self.z, w=other * self.w)

    def __radd__(self, other: float) -> "Vector4":
        return Vector4(x=other + self.x, y=other + self.y, z=other + self.z, w=other + self.w)

    def __rsub__(self, other: float) -> "Vector4":
        return Vector4(x=other - self.x, y=other - self.y, z=other - self.z, w=other - self.w)

    def __rtruediv__(self, other: float) -> "Vector4":
        return Vector4(x=other / self.x, y=other / self.y, z=other / self.z, w=other / self.w)

    def __neg__(self) -> "Vector4":
        return Vector4(x=-self.x, y=-self.y, z=-self.z, w=-self.w)

    def __abs__(self) -> "Vector4":
        return Vector4(x=abs(self.x), y=abs(self.y), z=abs(self.z), w=abs(self.w))

    def dot(self, other: "Vector4") -> float:
        """Calculate the dot product with another vector."""
        return self.x * other.x + self.y * other.y + self.z * other.z + self.w * other.w

    def magnitude(self) -> float:
        """Calculate the magnitude (length) of the vector."""
        return (self.x**2 + self.y**2 + self.z**2 + self.w**2) ** 0.5

    def magnitude2(self) -> float:
        """Calculate the squared magnitude of the vector."""
        return self.x**2 + self.y**2 + self.z**2 + self.w**2

    def normalize(self) -> "Vector4":
        """Return a normalized (unit) vector."""
        mag = self.magnitude()
        if mag == 0:
            return Vector4(x=0.0, y=0.0, z=0.0, w=0.0)
        return Vector4(x=self.x / mag, y=self.y / mag, z=self.z / mag, w=self.w / mag)

    def distance(self, other: "Vector4") -> float:
        """Calculate the distance to another vector."""
        return (self - other).magnitude()

    def distance2(self, other: "Vector4") -> float:
        """Calculate the squared distance to another vector."""
        diff = self - other
        return diff.x**2 + diff.y**2 + diff.z**2 + diff.w**2

    def angle(self, other: "Vector4") -> float:
        """Calculate the angle to another vector in radians."""
        import math

        dot_product = self.dot(other)
        mag_product = self.magnitude() * other.magnitude()
        if mag_product == 0:
            return 0.0
        return math.acos(max(-1.0, min(1.0, dot_product / mag_product)))

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
class Vector2i(StructFrozen):
    """A 2D integer vector."""

    x: int = property_(50, is_repr=True)
    y: int = property_(51, is_repr=True)

    def __add__(self, other: "Vector2i | int") -> "Vector2i":
        if isinstance(other, Vector2i):
            return Vector2i(x=self.x + other.x, y=self.y + other.y)
        else:
            return Vector2i(x=self.x + other, y=self.y + other)

    def __sub__(self, other: "Vector2i | int") -> "Vector2i":
        if isinstance(other, Vector2i):
            return Vector2i(x=self.x - other.x, y=self.y - other.y)
        else:
            return Vector2i(x=self.x - other, y=self.y - other)

    def __mul__(self, other: "Vector2i | int") -> "Vector2i":
        if isinstance(other, Vector2i):
            return Vector2i(x=self.x * other.x, y=self.y * other.y)
        else:
            return Vector2i(x=self.x * other, y=self.y * other)

    def __truediv__(self, other: "Vector2i | int") -> "Vector2i":
        if isinstance(other, Vector2i):
            return Vector2i(x=self.x // other.x, y=self.y // other.y)
        else:
            return Vector2i(x=self.x // other, y=self.y // other)

    def __rmul__(self, other: int) -> "Vector2i":
        return Vector2i(x=other * self.x, y=other * self.y)

    def __radd__(self, other: int) -> "Vector2i":
        return Vector2i(x=other + self.x, y=other + self.y)

    def __rsub__(self, other: int) -> "Vector2i":
        return Vector2i(x=other - self.x, y=other - self.y)

    def __rtruediv__(self, other: int) -> "Vector2i":
        return Vector2i(x=other // self.x, y=other // self.y)

    def __neg__(self) -> "Vector2i":
        return Vector2i(x=-self.x, y=-self.y)

    def __abs__(self) -> "Vector2i":
        return Vector2i(x=abs(self.x), y=abs(self.y))

    def dot(self, other: "Vector2i") -> int:
        """Calculate the dot product with another vector."""
        return self.x * other.x + self.y * other.y

    def magnitude(self) -> float:
        """Calculate the magnitude (length) of the vector."""
        return (self.x**2 + self.y**2) ** 0.5

    def magnitude2(self) -> int:
        """Calculate the squared magnitude of the vector."""
        return self.x**2 + self.y**2

    def normalize(self) -> "Vector2":
        """Return a normalized (unit) vector as float vector."""
        mag = self.magnitude()
        if mag == 0:
            return Vector2(x=0.0, y=0.0)
        return Vector2(x=self.x / mag, y=self.y / mag)

    def distance(self, other: "Vector2i") -> float:
        """Calculate the distance to another vector."""
        return (self - other).magnitude()

    def distance2(self, other: "Vector2i") -> int:
        """Calculate the squared distance to another vector."""
        diff = self - other
        return diff.x**2 + diff.y**2

    def angle(self, other: "Vector2i") -> float:
        """Calculate the angle to another vector in radians."""
        import math

        return math.atan2(other.y - self.y, other.x - self.x)

    def lerp(self, other: "Vector2i", t: float) -> "Vector2":
        """Linear interpolation between this vector and another as float vector."""
        return Vector2(x=self.x + (other.x - self.x) * t, y=self.y + (other.y - self.y) * t)

    def rot_with(self, center: "Vector2i", angle: float) -> "Vector2":
        """Rotate this vector around another point by the given angle as float vector."""
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
class Vector3i(StructFrozen):
    """A 3D integer vector."""

    x: int = property_(50, is_repr=True)
    y: int = property_(51, is_repr=True)
    z: int = property_(52, is_repr=True)

    def __add__(self, other: "Vector3i | int") -> "Vector3i":
        if isinstance(other, Vector3i):
            return Vector3i(x=self.x + other.x, y=self.y + other.y, z=self.z + other.z)
        else:
            return Vector3i(x=self.x + other, y=self.y + other, z=self.z + other)

    def __sub__(self, other: "Vector3i | int") -> "Vector3i":
        if isinstance(other, Vector3i):
            return Vector3i(x=self.x - other.x, y=self.y - other.y, z=self.z - other.z)
        else:
            return Vector3i(x=self.x - other, y=self.y - other, z=self.z - other)

    def __mul__(self, other: "Vector3i | int") -> "Vector3i":
        if isinstance(other, Vector3i):
            return Vector3i(x=self.x * other.x, y=self.y * other.y, z=self.z * other.z)
        else:
            return Vector3i(x=self.x * other, y=self.y * other, z=self.z * other)

    def __truediv__(self, other: "Vector3i") -> "Vector3i":
        if isinstance(other, Vector3i):
            return Vector3i(x=self.x // other.x, y=self.y // other.y, z=self.z // other.z)
        else:
            return Vector3i(x=self.x // other, y=self.y // other, z=self.z // other)

    def __rmul__(self, other: int) -> "Vector3i":
        return Vector3i(x=other * self.x, y=other * self.y, z=other * self.z)

    def __radd__(self, other: int) -> "Vector3i":
        return Vector3i(x=other + self.x, y=other + self.y, z=other + self.z)

    def __rsub__(self, other: int) -> "Vector3i":
        return Vector3i(x=other - self.x, y=other - self.y, z=other - self.z)

    def __rtruediv__(self, other: int) -> "Vector3i":
        return Vector3i(x=other // self.x, y=other // self.y, z=other // self.z)

    def __neg__(self) -> "Vector3i":
        return Vector3i(x=-self.x, y=-self.y, z=-self.z)

    def __abs__(self) -> "Vector3i":
        return Vector3i(x=abs(self.x), y=abs(self.y), z=abs(self.z))

    def dot(self, other: "Vector3i") -> int:
        """Calculate the dot product with another vector."""
        return self.x * other.x + self.y * other.y + self.z * other.z

    def magnitude(self) -> float:
        """Calculate the magnitude (length) of the vector."""
        return (self.x**2 + self.y**2 + self.z**2) ** 0.5

    def magnitude2(self) -> int:
        """Calculate the squared magnitude of the vector."""
        return self.x**2 + self.y**2 + self.z**2

    def normalize(self) -> "Vector3":
        """Return a normalized (unit) vector as float vector."""
        mag = self.magnitude()
        if mag == 0:
            return Vector3(x=0.0, y=0.0, z=0.0)
        return Vector3(x=self.x / mag, y=self.y / mag, z=self.z / mag)

    def distance(self, other: "Vector3i") -> float:
        """Calculate the distance to another vector."""
        return (self - other).magnitude()

    def distance2(self, other: "Vector3i") -> int:
        """Calculate the squared distance to another vector."""
        diff = self - other
        return diff.x**2 + diff.y**2 + diff.z**2

    def angle(self, other: "Vector3i") -> float:
        """Calculate the angle to another vector in radians."""
        import math

        dot_product = self.dot(other)
        mag_product = self.magnitude() * other.magnitude()
        if mag_product == 0:
            return 0.0
        return math.acos(max(-1.0, min(1.0, dot_product / mag_product)))

    def lerp(self, other: "Vector3i", t: float) -> "Vector3":
        """Linear interpolation between this vector and another as float vector."""
        return Vector3(
            x=self.x + (other.x - self.x) * t,
            y=self.y + (other.y - self.y) * t,
            z=self.z + (other.z - self.z) * t,
        )

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
class Vector4i(StructFrozen):
    """A 4D integer vector."""

    x: int = property_(50, is_repr=True)
    y: int = property_(51, is_repr=True)
    z: int = property_(52, is_repr=True)
    w: int = property_(53, is_repr=True)

    def __add__(self, other: "Vector4i | int") -> "Vector4i":
        if isinstance(other, Vector4i):
            return Vector4i(
                x=self.x + other.x, y=self.y + other.y, z=self.z + other.z, w=self.w + other.w
            )
        else:
            return Vector4i(x=self.x + other, y=self.y + other, z=self.z + other, w=self.w + other)

    def __sub__(self, other: "Vector4i | int") -> "Vector4i":
        if isinstance(other, Vector4i):
            return Vector4i(
                x=self.x - other.x, y=self.y - other.y, z=self.z - other.z, w=self.w - other.w
            )
        else:
            return Vector4i(x=self.x - other, y=self.y - other, z=self.z - other, w=self.w - other)

    def __mul__(self, other: "Vector4i | int") -> "Vector4i":
        if isinstance(other, Vector4i):
            return Vector4i(
                x=self.x * other.x, y=self.y * other.y, z=self.z * other.z, w=self.w * other.w
            )
        else:
            return Vector4i(x=self.x * other, y=self.y * other, z=self.z * other, w=self.w * other)

    def __truediv__(self, other: "Vector4i | int") -> "Vector4i":
        if isinstance(other, Vector4i):
            return Vector4i(
                x=self.x // other.x, y=self.y // other.y, z=self.z // other.z, w=self.w // other.w
            )
        else:
            return Vector4i(
                x=self.x // other, y=self.y // other, z=self.z // other, w=self.w // other
            )

    def __rmul__(self, other: int) -> "Vector4i":
        return Vector4i(x=other * self.x, y=other * self.y, z=other * self.z, w=other * self.w)

    def __radd__(self, other: int) -> "Vector4i":
        return Vector4i(x=other + self.x, y=other + self.y, z=other + self.z, w=other + self.w)

    def __rsub__(self, other: int) -> "Vector4i":
        return Vector4i(x=other - self.x, y=other - self.y, z=other - self.z, w=other - self.w)

    def __rtruediv__(self, other: int) -> "Vector4i":
        return Vector4i(x=other // self.x, y=other // self.y, z=other // self.z, w=other // self.w)

    def __neg__(self) -> "Vector4i":
        return Vector4i(x=-self.x, y=-self.y, z=-self.z, w=-self.w)

    def __abs__(self) -> "Vector4i":
        return Vector4i(x=abs(self.x), y=abs(self.y), z=abs(self.z), w=abs(self.w))

    def dot(self, other: "Vector4i") -> int:
        """Calculate the dot product with another vector."""
        return self.x * other.x + self.y * other.y + self.z * other.z + self.w * other.w

    def magnitude(self) -> float:
        """Calculate the magnitude (length) of the vector."""
        return (self.x**2 + self.y**2 + self.z**2 + self.w**2) ** 0.5

    def magnitude2(self) -> int:
        """Calculate the squared magnitude of the vector."""
        return self.x**2 + self.y**2 + self.z**2 + self.w**2

    def normalize(self) -> "Vector4":
        """Return a normalized (unit) vector as float vector."""
        mag = self.magnitude()
        if mag == 0:
            return Vector4(x=0.0, y=0.0, z=0.0, w=0.0)
        return Vector4(x=self.x / mag, y=self.y / mag, z=self.z / mag, w=self.w / mag)

    def distance(self, other: "Vector4i") -> float:
        """Calculate the distance to another vector."""
        return (self - other).magnitude()

    def distance2(self, other: "Vector4i") -> int:
        """Calculate the squared distance to another vector."""
        diff = self - other
        return diff.x**2 + diff.y**2 + diff.z**2 + diff.w**2

    def angle(self, other: "Vector4i") -> float:
        """Calculate the angle to another vector in radians."""
        import math

        dot_product = self.dot(other)
        mag_product = self.magnitude() * other.magnitude()
        if mag_product == 0:
            return 0.0
        return math.acos(max(-1.0, min(1.0, dot_product / mag_product)))

    def lerp(self, other: "Vector4i", t: float) -> "Vector4":
        """Linear interpolation between this vector and another as float vector."""
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
