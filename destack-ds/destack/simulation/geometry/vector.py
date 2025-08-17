from typing import TYPE_CHECKING, Union, final

from destack.core import (
    Float32,
    FunctionOperator,
    Int32,
    ObjectStability,
    RuntimeLanguage,
    Struct,
    StructType,
    declare_constant,
    declare_method,
    declare_property,
    declare_struct,
)

if TYPE_CHECKING:
    pass


@declare_struct(
    StructType.VECTOR2,
    is_final=True,
    stability=ObjectStability.STATIC,
)
@final
class Vector2(Struct):
    """A 2D floating point Vector."""

    ZERO = declare_constant(
        100,
        value=lambda: Vector2(x=0.0, y=0.0),
        description="The zero Vector2.",
    )
    ONE = declare_constant(
        101,
        value=lambda: Vector2(x=1.0, y=1.0),
        description="The one Vector2.",
    )
    X_AXIS = declare_constant(
        110,
        value=lambda: Vector2(x=1.0, y=0.0),
        description="The x-axis Vector2.",
    )
    Y_AXIS = declare_constant(
        111,
        value=lambda: Vector2(x=0.0, y=1.0),
        description="The y-axis Vector2.",
    )

    x: Float32 = declare_property(
        101,
        is_repr=True,
        description="The x-coordinate of the Vector2.",
        tag=None,
    )
    y: Float32 = declare_property(
        102,
        is_repr=True,
        description="The y-coordinate of the Vector2.",
        tag=None,
    )

    @declare_method(101)
    def add(self, other: Union["Vector2", Float32]) -> "Vector2":
        """Add a Vector2 or a scalar to a Vector2."""
        raise NotImplementedError

    @declare_method(
        102,
        operator=FunctionOperator.ADD,
        alias_of="add",
        languages=(RuntimeLanguage.PYTHON,),
    )
    def __add__(self, other: Union["Vector2", Float32]) -> "Vector2":
        """Add a Vector2 or a scalar to a Vector2."""
        raise NotImplementedError

    @declare_method(103)
    def sub(self, other: Union["Vector2", Float32]) -> "Vector2":
        """Subtract a Vector2 or a scalar from a Vector2."""
        raise NotImplementedError

    @declare_method(
        104,
        operator=FunctionOperator.SUB,
        alias_of="sub",
        languages=(RuntimeLanguage.PYTHON,),
    )
    def __sub__(self, other: Union["Vector2", Float32]) -> "Vector2":
        """Subtract a Vector2 or a scalar from a Vector2."""
        raise NotImplementedError

    @declare_method(105)
    def mul(self, other: Union["Vector2", Float32]) -> "Vector2":
        """Multiply a Vector2 or a scalar by a Vector2."""
        raise NotImplementedError

    @declare_method(
        106,
        operator=FunctionOperator.MUL,
        alias_of="mul",
        languages=(RuntimeLanguage.PYTHON,),
    )
    def __mul__(self, other: Union["Vector2", Float32]) -> "Vector2":
        """Multiply a Vector2 or a scalar by a Vector2."""
        raise NotImplementedError

    @declare_method(107)
    def truediv(self, other: Union["Vector2", Float32]) -> "Vector2":
        """Divide a Vector2 or a scalar by a Vector2."""
        raise NotImplementedError

    @declare_method(
        108,
        operator=FunctionOperator.TRUEDIV,
        alias_of="truediv",
        languages=(RuntimeLanguage.PYTHON,),
    )
    def __truediv__(self, other: Union["Vector2", Float32]) -> "Vector2":
        """Divide a Vector2 or a scalar by a Vector2."""
        raise NotImplementedError

    @declare_method(109)
    def abs(self) -> "Vector2":
        """Get the absolute value of a Vector2."""
        raise NotImplementedError

    @declare_method(
        110,
        operator=FunctionOperator.ABS,
        alias_of="abs",
        languages=(RuntimeLanguage.PYTHON,),
    )
    def __abs__(self) -> "Vector2":
        """Get the absolute value of a Vector2."""
        raise NotImplementedError

    @declare_method(111)
    def neg(self) -> "Vector2":
        """Negate a vector."""
        raise NotImplementedError

    @declare_method(
        112,
        operator=FunctionOperator.NEG,
        alias_of="neg",
        languages=(RuntimeLanguage.PYTHON,),
    )
    def __neg__(self) -> "Vector2":
        """Negate a vector."""
        raise NotImplementedError

    @declare_method(
        113,
        operator=FunctionOperator.ADD,
        alias_of="add",
        languages=(RuntimeLanguage.PYTHON,),
    )
    def __radd__(self, other: Union["Vector2", Float32]) -> "Vector2":
        """Add a Vector2 or a scalar to a Vector2."""
        raise NotImplementedError

    @declare_method(
        114,
        operator=FunctionOperator.SUB,
        alias_of="sub",
        languages=(RuntimeLanguage.PYTHON,),
    )
    def __rsub__(self, other: Union["Vector2", Float32]) -> "Vector2":
        """Subtract a Vector2 or a scalar from a Vector2."""
        raise NotImplementedError

    @declare_method(
        115,
        operator=FunctionOperator.MUL,
        alias_of="mul",
        languages=(RuntimeLanguage.PYTHON,),
    )
    def __rmul__(self, other: Union["Vector2", Float32]) -> "Vector2":
        """Multiply a Vector2 or a scalar by a Vector2."""
        raise NotImplementedError

    @declare_method(
        116,
        operator=FunctionOperator.TRUEDIV,
        alias_of="truediv",
        languages=(RuntimeLanguage.PYTHON,),
    )
    def __rtruediv__(self, other: Union["Vector2", Float32]) -> "Vector2":
        """Divide a Vector2 or a scalar by a Vector2."""
        raise NotImplementedError

    @declare_method(117)
    def perp(self) -> "Vector2":
        """Get the perpendicular vector (rotated 90 degrees counterclockwise)."""
        raise NotImplementedError

    @declare_method(118)
    def dot(self, other: "Vector2") -> Float32:
        """Calculate the dot product with another vector."""
        raise NotImplementedError

    @declare_method(119)
    def magnitude(self) -> Float32:
        """Calculate the magnitude (length) of the vector."""
        raise NotImplementedError

    @declare_method(120)
    def magnitude2(self) -> Float32:
        """Calculate the squared magnitude of the vector."""
        raise NotImplementedError

    @declare_method(121)
    def normalize(self) -> "Vector2":
        """Return a normalized (unit) vector."""
        raise NotImplementedError

    @declare_method(122)
    def distance(self, other: "Vector2") -> Float32:
        """Calculate the distance to another vector."""
        raise NotImplementedError

    @declare_method(123)
    def distance2(self, other: "Vector2") -> Float32:
        """Calculate the squared distance to another vector."""
        raise NotImplementedError

    @declare_method(124)
    def angle(self, other: "Vector2") -> Float32:
        """Calculate the angle to another vector in radians."""
        raise NotImplementedError

    @declare_method(125)
    def lerp(self, other: "Vector2", t: Float32) -> "Vector2":
        """Linear interpolation between this vector and another."""
        raise NotImplementedError

    @declare_method(126)
    def rot_with(self, center: "Vector2", angle: Float32) -> "Vector2":
        """Rotate this vector around another point by the given angle."""
        raise NotImplementedError


@declare_struct(
    StructType.VECTOR3,
    is_final=True,
    stability=ObjectStability.STATIC,
)
@final
class Vector3(Struct):
    """A 3D floating point Vector."""

    ZERO = declare_constant(
        100,
        value=lambda: Vector3(x=0.0, y=0.0, z=0.0),
        description="The zero Vector3.",
    )
    ONE = declare_constant(
        101,
        value=lambda: Vector3(x=1.0, y=1.0, z=1.0),
        description="The one Vector3.",
    )
    X_AXIS = declare_constant(
        110,
        value=lambda: Vector3(x=1.0, y=0.0, z=0.0),
        description="The x-axis Vector3.",
    )
    Y_AXIS = declare_constant(
        111,
        value=lambda: Vector3(x=0.0, y=1.0, z=0.0),
        description="The y-axis Vector3.",
    )
    Z_AXIS = declare_constant(
        112,
        value=lambda: Vector3(x=0.0, y=0.0, z=1.0),
        description="The z-axis Vector3.",
    )

    x: Float32 = declare_property(
        101,
        is_repr=True,
        description="The x-coordinate of the Vector3.",
        tag=None,
    )
    y: Float32 = declare_property(
        102,
        is_repr=True,
        description="The y-coordinate of the Vector3.",
        tag=None,
    )
    z: Float32 = declare_property(
        103,
        is_repr=True,
        description="The z-coordinate of the Vector3.",
        tag=None,
    )

    @declare_method(101)
    def add(self, other: Union["Vector3", Float32]) -> "Vector3":
        """Add a Vector3 or a scalar to a Vector3."""
        raise NotImplementedError

    @declare_method(
        102,
        operator=FunctionOperator.ADD,
        alias_of="add",
        languages=(RuntimeLanguage.PYTHON,),
    )
    def __add__(self, other: Union["Vector3", Float32]) -> "Vector3":
        """Add a Vector3 or a scalar to a Vector3."""
        raise NotImplementedError

    @declare_method(103)
    def sub(self, other: Union["Vector3", Float32]) -> "Vector3":
        """Subtract a Vector3 or a scalar from a Vector3."""
        raise NotImplementedError

    @declare_method(
        104,
        operator=FunctionOperator.SUB,
        alias_of="sub",
        languages=(RuntimeLanguage.PYTHON,),
    )
    def __sub__(self, other: Union["Vector3", Float32]) -> "Vector3":
        """Subtract a Vector3 or a scalar from a Vector3."""
        raise NotImplementedError

    @declare_method(105)
    def mul(self, other: Union["Vector3", Float32]) -> "Vector3":
        """Multiply a Vector3 or a scalar by a Vector3."""
        raise NotImplementedError

    @declare_method(
        106,
        operator=FunctionOperator.MUL,
        alias_of="mul",
        languages=(RuntimeLanguage.PYTHON,),
    )
    def __mul__(self, other: Union["Vector3", Float32]) -> "Vector3":
        """Multiply a Vector3 or a scalar by a Vector3."""
        raise NotImplementedError

    @declare_method(107)
    def truediv(self, other: Union["Vector3", Float32]) -> "Vector3":
        """Divide a Vector3 or a scalar by a Vector3."""
        raise NotImplementedError

    @declare_method(
        108,
        operator=FunctionOperator.TRUEDIV,
        alias_of="truediv",
        languages=(RuntimeLanguage.PYTHON,),
    )
    def __truediv__(self, other: Union["Vector3", Float32]) -> "Vector3":
        """Divide a Vector3 or a scalar by a Vector3."""
        raise NotImplementedError

    @declare_method(109)
    def abs(self) -> "Vector3":
        """Get the absolute value of a Vector3."""
        raise NotImplementedError

    @declare_method(
        110,
        operator=FunctionOperator.ABS,
        alias_of="abs",
        languages=(RuntimeLanguage.PYTHON,),
    )
    def __abs__(self) -> "Vector3":
        """Get the absolute value of a Vector3."""
        raise NotImplementedError

    @declare_method(111)
    def neg(self) -> "Vector3":
        """Negate a vector."""
        raise NotImplementedError

    @declare_method(
        112,
        operator=FunctionOperator.NEG,
        alias_of="neg",
        languages=(RuntimeLanguage.PYTHON,),
    )
    def __neg__(self) -> "Vector3":
        """Negate a vector."""
        raise NotImplementedError

    @declare_method(
        113,
        operator=FunctionOperator.ADD,
        alias_of="add",
        languages=(RuntimeLanguage.PYTHON,),
    )
    def __radd__(self, other: Union["Vector3", Float32]) -> "Vector3":
        """Add a Vector3 or a scalar to a Vector3."""
        raise NotImplementedError

    @declare_method(
        114,
        operator=FunctionOperator.SUB,
        alias_of="sub",
        languages=(RuntimeLanguage.PYTHON,),
    )
    def __rsub__(self, other: Union["Vector3", Float32]) -> "Vector3":
        """Subtract a Vector3 or a scalar from a Vector3."""
        raise NotImplementedError

    @declare_method(
        115,
        operator=FunctionOperator.MUL,
        alias_of="mul",
        languages=(RuntimeLanguage.PYTHON,),
    )
    def __rmul__(self, other: Union["Vector3", Float32]) -> "Vector3":
        """Multiply a Vector3 or a scalar by a Vector3."""
        raise NotImplementedError

    @declare_method(
        116,
        operator=FunctionOperator.TRUEDIV,
        alias_of="truediv",
        languages=(RuntimeLanguage.PYTHON,),
    )
    def __rtruediv__(self, other: Union["Vector3", Float32]) -> "Vector3":
        """Divide a Vector3 or a scalar by a Vector3."""
        raise NotImplementedError

    @declare_method(117)
    def dot(self, other: "Vector3") -> Float32:
        """Calculate the dot product with another vector."""
        raise NotImplementedError

    @declare_method(118)
    def magnitude(self) -> Float32:
        """Calculate the magnitude (length) of the vector."""
        raise NotImplementedError

    @declare_method(119)
    def magnitude2(self) -> Float32:
        """Calculate the squared magnitude of the vector."""
        raise NotImplementedError

    @declare_method(120)
    def normalize(self) -> "Vector3":
        """Return a normalized (unit) vector."""
        raise NotImplementedError

    @declare_method(121)
    def distance(self, other: "Vector3") -> Float32:
        """Calculate the distance to another vector."""
        raise NotImplementedError

    @declare_method(122)
    def distance2(self, other: "Vector3") -> Float32:
        """Calculate the squared distance to another vector."""
        raise NotImplementedError

    @declare_method(123)
    def angle(self, other: "Vector3") -> Float32:
        """Calculate the angle to another vector in radians."""
        raise NotImplementedError

    @declare_method(124)
    def lerp(self, other: "Vector3", t: Float32) -> "Vector3":
        """Linear interpolation between this vector and another."""
        raise NotImplementedError

    @declare_method(125)
    def cross(self, other: "Vector3") -> "Vector3":
        """Calculate the cross product with another vector."""
        raise NotImplementedError


@declare_struct(
    StructType.VECTOR4,
    is_final=True,
    stability=ObjectStability.STATIC,
)
@final
class Vector4(Struct):
    """A 4D floating point Vector."""

    ZERO = declare_constant(
        100,
        value=lambda: Vector4(x=0.0, y=0.0, z=0.0, w=0.0),
        description="The zero Vector4.",
    )
    ONE = declare_constant(
        101,
        value=lambda: Vector4(x=1.0, y=1.0, z=1.0, w=1.0),
        description="The one Vector4.",
    )
    X_AXIS = declare_constant(
        110,
        value=lambda: Vector4(x=1.0, y=0.0, z=0.0, w=0.0),
        description="The x-axis Vector4.",
    )
    Y_AXIS = declare_constant(
        111,
        value=lambda: Vector4(x=0.0, y=1.0, z=0.0, w=0.0),
        description="The y-axis Vector4.",
    )
    Z_AXIS = declare_constant(
        112,
        value=lambda: Vector4(x=0.0, y=0.0, z=1.0, w=0.0),
        description="The z-axis Vector4.",
    )
    W_AXIS = declare_constant(
        113,
        value=lambda: Vector4(x=0.0, y=0.0, z=0.0, w=1.0),
        description="The w-axis Vector4.",
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

    @declare_method(101)
    def add(self, other: Union["Vector4", Float32]) -> "Vector4":
        """Add a Vector4 or a scalar to a Vector4."""
        raise NotImplementedError

    @declare_method(
        102,
        operator=FunctionOperator.ADD,
        alias_of="add",
        languages=(RuntimeLanguage.PYTHON,),
    )
    def __add__(self, other: Union["Vector4", Float32]) -> "Vector4":
        """Add a Vector4 or a scalar to a Vector4."""
        raise NotImplementedError

    @declare_method(103)
    def sub(self, other: Union["Vector4", Float32]) -> "Vector4":
        """Subtract a Vector4 or a scalar from a Vector4."""
        raise NotImplementedError

    @declare_method(
        104,
        operator=FunctionOperator.SUB,
        alias_of="sub",
        languages=(RuntimeLanguage.PYTHON,),
    )
    def __sub__(self, other: Union["Vector4", Float32]) -> "Vector4":
        """Subtract a Vector4 or a scalar from a Vector4."""
        raise NotImplementedError

    @declare_method(105)
    def mul(self, other: Union["Vector4", Float32]) -> "Vector4":
        """Multiply a Vector4 or a scalar by a Vector4."""
        raise NotImplementedError

    @declare_method(
        106,
        operator=FunctionOperator.MUL,
        alias_of="mul",
        languages=(RuntimeLanguage.PYTHON,),
    )
    def __mul__(self, other: Union["Vector4", Float32]) -> "Vector4":
        """Multiply a Vector4 or a scalar by a Vector4."""
        raise NotImplementedError

    @declare_method(107)
    def truediv(self, other: Union["Vector4", Float32]) -> "Vector4":
        """Divide a Vector4 or a scalar by a Vector4."""
        raise NotImplementedError

    @declare_method(
        108,
        operator=FunctionOperator.TRUEDIV,
        alias_of="truediv",
        languages=(RuntimeLanguage.PYTHON,),
    )
    def __truediv__(self, other: Union["Vector4", Float32]) -> "Vector4":
        """Divide a Vector4 or a scalar by a Vector4."""
        raise NotImplementedError

    @declare_method(109)
    def abs(self) -> "Vector4":
        """Get the absolute value of a Vector4."""
        raise NotImplementedError

    @declare_method(
        110,
        operator=FunctionOperator.ABS,
        alias_of="abs",
        languages=(RuntimeLanguage.PYTHON,),
    )
    def __abs__(self) -> "Vector4":
        """Get the absolute value of a Vector4."""
        raise NotImplementedError

    @declare_method(111)
    def neg(self) -> "Vector4":
        """Negate a vector."""
        raise NotImplementedError

    @declare_method(
        112,
        operator=FunctionOperator.NEG,
        alias_of="neg",
        languages=(RuntimeLanguage.PYTHON,),
    )
    def __neg__(self) -> "Vector4":
        """Negate a vector."""
        raise NotImplementedError

    @declare_method(
        113,
        operator=FunctionOperator.ADD,
        alias_of="add",
        languages=(RuntimeLanguage.PYTHON,),
    )
    def __radd__(self, other: Union["Vector4", Float32]) -> "Vector4":
        """Add a Vector4 or a scalar to a Vector4."""
        raise NotImplementedError

    @declare_method(
        114,
        operator=FunctionOperator.SUB,
        alias_of="sub",
        languages=(RuntimeLanguage.PYTHON,),
    )
    def __rsub__(self, other: Union["Vector4", Float32]) -> "Vector4":
        """Subtract a Vector4 or a scalar from a Vector4."""
        raise NotImplementedError

    @declare_method(
        115,
        operator=FunctionOperator.MUL,
        alias_of="mul",
        languages=(RuntimeLanguage.PYTHON,),
    )
    def __rmul__(self, other: Union["Vector4", Float32]) -> "Vector4":
        """Multiply a Vector4 or a scalar by a Vector4."""
        raise NotImplementedError

    @declare_method(
        116,
        operator=FunctionOperator.TRUEDIV,
        alias_of="truediv",
        languages=(RuntimeLanguage.PYTHON,),
    )
    def __rtruediv__(self, other: Union["Vector4", Float32]) -> "Vector4":
        """Divide a Vector4 or a scalar by a Vector4."""
        raise NotImplementedError

    @declare_method(117)
    def dot(self, other: "Vector4") -> Float32:
        """Calculate the dot product with another vector."""
        raise NotImplementedError

    @declare_method(118)
    def magnitude(self) -> Float32:
        """Calculate the magnitude (length) of the vector."""
        raise NotImplementedError

    @declare_method(119)
    def magnitude2(self) -> Float32:
        """Calculate the squared magnitude of the vector."""
        raise NotImplementedError

    @declare_method(120)
    def normalize(self) -> "Vector4":
        """Return a normalized (unit) vector."""
        raise NotImplementedError

    @declare_method(121)
    def distance(self, other: "Vector4") -> Float32:
        """Calculate the distance to another vector."""
        raise NotImplementedError

    @declare_method(122)
    def distance2(self, other: "Vector4") -> Float32:
        """Calculate the squared distance to another vector."""
        raise NotImplementedError

    @declare_method(123)
    def angle(self, other: "Vector4") -> Float32:
        """Calculate the angle to another vector in radians."""
        raise NotImplementedError

    @declare_method(124)
    def lerp(self, other: "Vector4", t: Float32) -> "Vector4":
        """Linear interpolation between this vector and another."""
        raise NotImplementedError


@declare_struct(
    StructType.VECTOR2I,
    is_final=True,
    stability=ObjectStability.STATIC,
)
@final
class Vector2i(Struct):
    """A 2D integer Vector."""

    ZERO = declare_constant(
        100,
        value=lambda: Vector2i(x=0, y=0),
        description="The zero Vector2i.",
    )
    ONE = declare_constant(
        101,
        value=lambda: Vector2i(x=1, y=1),
        description="The one Vector2i.",
    )
    X_AXIS = declare_constant(
        110,
        value=lambda: Vector2i(x=1, y=0),
        description="The x-axis Vector2i.",
    )
    Y_AXIS = declare_constant(
        111,
        value=lambda: Vector2i(x=0, y=1),
        description="The y-axis Vector2i.",
    )

    x: Int32 = declare_property(
        101,
        is_repr=True,
        description="The x-coordinate of the Vector2i.",
        tag=None,
    )
    y: Int32 = declare_property(
        102,
        is_repr=True,
        description="The y-coordinate of the Vector2i.",
        tag=None,
    )

    @declare_method(101)
    def add(self, other: Union["Vector2i", Int32]) -> "Vector2i":
        """Add a Vector2i or a scalar to a Vector2i."""
        raise NotImplementedError

    @declare_method(
        102,
        operator=FunctionOperator.ADD,
        alias_of="add",
        languages=(RuntimeLanguage.PYTHON,),
    )
    def __add__(self, other: Union["Vector2i", Int32]) -> "Vector2i":
        """Add a Vector2i or a scalar to a Vector2i."""
        raise NotImplementedError

    @declare_method(103)
    def sub(self, other: Union["Vector2i", Int32]) -> "Vector2i":
        """Subtract a Vector2i or a scalar from a Vector2i."""
        raise NotImplementedError

    @declare_method(
        104,
        operator=FunctionOperator.SUB,
        alias_of="sub",
        languages=(RuntimeLanguage.PYTHON,),
    )
    def __sub__(self, other: Union["Vector2i", Int32]) -> "Vector2i":
        """Subtract a Vector2i or a scalar from a Vector2i."""
        raise NotImplementedError

    @declare_method(105)
    def mul(self, other: Union["Vector2i", Int32]) -> "Vector2i":
        """Multiply a Vector2i or a scalar by a Vector2i."""
        raise NotImplementedError

    @declare_method(
        106,
        operator=FunctionOperator.MUL,
        alias_of="mul",
        languages=(RuntimeLanguage.PYTHON,),
    )
    def __mul__(self, other: Union["Vector2i", Int32]) -> "Vector2i":
        """Multiply a Vector2i or a scalar by a Vector2i."""
        raise NotImplementedError

    @declare_method(107)
    def truediv(self, other: Union["Vector2i", Int32]) -> "Vector2i":
        """Divide a Vector2i or a scalar by a Vector2i."""
        raise NotImplementedError

    @declare_method(
        108,
        operator=FunctionOperator.TRUEDIV,
        alias_of="truediv",
        languages=(RuntimeLanguage.PYTHON,),
    )
    def __truediv__(self, other: Union["Vector2i", Int32]) -> "Vector2i":
        """Divide a Vector2i or a scalar by a Vector2i."""
        raise NotImplementedError

    @declare_method(109)
    def abs(self) -> "Vector2i":
        """Get the absolute value of a Vector2i."""
        raise NotImplementedError

    @declare_method(
        110,
        operator=FunctionOperator.ABS,
        alias_of="abs",
        languages=(RuntimeLanguage.PYTHON,),
    )
    def __abs__(self) -> "Vector2i":
        """Get the absolute value of a Vector2i."""
        raise NotImplementedError

    @declare_method(111)
    def neg(self) -> "Vector2i":
        """Negate a vector."""
        raise NotImplementedError

    @declare_method(
        112,
        operator=FunctionOperator.NEG,
        alias_of="neg",
        languages=(RuntimeLanguage.PYTHON,),
    )
    def __neg__(self) -> "Vector2i":
        """Negate a vector."""
        raise NotImplementedError

    @declare_method(
        113,
        operator=FunctionOperator.ADD,
        alias_of="add",
        languages=(RuntimeLanguage.PYTHON,),
    )
    def __radd__(self, other: Union["Vector2i", Int32]) -> "Vector2i":
        """Add a Vector2i or a scalar to a Vector2i."""
        raise NotImplementedError

    @declare_method(
        114,
        operator=FunctionOperator.SUB,
        alias_of="sub",
        languages=(RuntimeLanguage.PYTHON,),
    )
    def __rsub__(self, other: Union["Vector2i", Int32]) -> "Vector2i":
        """Subtract a Vector2i or a scalar from a Vector2i."""
        raise NotImplementedError

    @declare_method(
        115,
        operator=FunctionOperator.MUL,
        alias_of="mul",
        languages=(RuntimeLanguage.PYTHON,),
    )
    def __rmul__(self, other: Union["Vector2i", Int32]) -> "Vector2i":
        """Multiply a Vector2i or a scalar by a Vector2i."""
        raise NotImplementedError

    @declare_method(
        116,
        operator=FunctionOperator.TRUEDIV,
        alias_of="truediv",
        languages=(RuntimeLanguage.PYTHON,),
    )
    def __rtruediv__(self, other: Union["Vector2i", Int32]) -> "Vector2i":
        """Divide a Vector2i or a scalar by a Vector2i."""
        raise NotImplementedError

    @declare_method(117)
    def perp(self) -> "Vector2i":
        """Get the perpendicular vector (rotated 90 degrees counterclockwise)."""
        raise NotImplementedError

    @declare_method(118)
    def dot(self, other: "Vector2i") -> Int32:
        """Calculate the dot product with another vector."""
        raise NotImplementedError

    @declare_method(119)
    def magnitude(self) -> Int32:
        """Calculate the magnitude (length) of the vector."""
        raise NotImplementedError

    @declare_method(120)
    def magnitude2(self) -> Int32:
        """Calculate the squared magnitude of the vector."""
        raise NotImplementedError

    @declare_method(121)
    def normalize(self) -> "Vector2":
        """Return a normalized (unit) vector as floating point vector."""
        raise NotImplementedError

    @declare_method(122)
    def distance(self, other: "Vector2i") -> Int32:
        """Calculate the distance to another vector."""
        raise NotImplementedError

    @declare_method(123)
    def distance2(self, other: "Vector2i") -> Int32:
        """Calculate the squared distance to another vector."""
        raise NotImplementedError

    @declare_method(124)
    def angle(self, other: "Vector2i") -> Float32:
        """Calculate the angle to another vector in radians."""
        raise NotImplementedError

    @declare_method(125)
    def lerp(self, other: "Vector2i", t: Float32) -> "Vector2":
        """Linear interpolation between this vector and another as floating point vector."""
        raise NotImplementedError

    @declare_method(126)
    def rot_with(self, center: "Vector2i", angle: Float32) -> "Vector2":
        """Rotate this vector around another point by the given angle as floating point vector."""
        raise NotImplementedError


@declare_struct(
    StructType.VECTOR3I,
    is_final=True,
    stability=ObjectStability.STATIC,
)
@final
class Vector3i(Struct):
    """A 3D integer Vector."""

    ZERO = declare_constant(
        100,
        value=lambda: Vector3i(x=0, y=0, z=0),
        description="The zero Vector3i.",
    )
    ONE = declare_constant(
        101,
        value=lambda: Vector3i(x=1, y=1, z=1),
        description="The one Vector3i.",
    )
    X_AXIS = declare_constant(
        110,
        value=lambda: Vector3i(x=1, y=0, z=0),
        description="The x-axis Vector3i.",
    )
    Y_AXIS = declare_constant(
        111,
        value=lambda: Vector3i(x=0, y=1, z=0),
        description="The y-axis Vector3i.",
    )
    Z_AXIS = declare_constant(
        112,
        value=lambda: Vector3i(x=0, y=0, z=1),
        description="The z-axis Vector3i.",
    )

    x: Int32 = declare_property(
        101,
        is_repr=True,
        description="The x-coordinate of the Vector3i.",
        tag=None,
    )
    y: Int32 = declare_property(
        102,
        is_repr=True,
        description="The y-coordinate of the Vector3i.",
        tag=None,
    )
    z: Int32 = declare_property(
        103,
        is_repr=True,
        description="The z-coordinate of the Vector3i.",
        tag=None,
    )

    @declare_method(101)
    def add(self, other: Union["Vector3i", Int32]) -> "Vector3i":
        """Add a Vector3i or a scalar to a Vector3i."""
        raise NotImplementedError

    @declare_method(
        102,
        operator=FunctionOperator.ADD,
        alias_of="add",
        languages=(RuntimeLanguage.PYTHON,),
    )
    def __add__(self, other: Union["Vector3i", Int32]) -> "Vector3i":
        """Add a Vector3i or a scalar to a Vector3i."""
        raise NotImplementedError

    @declare_method(103)
    def sub(self, other: Union["Vector3i", Int32]) -> "Vector3i":
        """Subtract a Vector3i or a scalar from a Vector3i."""
        raise NotImplementedError

    @declare_method(
        104,
        operator=FunctionOperator.SUB,
        alias_of="sub",
        languages=(RuntimeLanguage.PYTHON,),
    )
    def __sub__(self, other: Union["Vector3i", Int32]) -> "Vector3i":
        """Subtract a Vector3i or a scalar from a Vector3i."""
        raise NotImplementedError

    @declare_method(105)
    def mul(self, other: Union["Vector3i", Int32]) -> "Vector3i":
        """Multiply a Vector3i or a scalar by a Vector3i."""
        raise NotImplementedError

    @declare_method(
        106,
        operator=FunctionOperator.MUL,
        alias_of="mul",
        languages=(RuntimeLanguage.PYTHON,),
    )
    def __mul__(self, other: Union["Vector3i", Int32]) -> "Vector3i":
        """Multiply a Vector3i or a scalar by a Vector3i."""
        raise NotImplementedError

    @declare_method(107)
    def truediv(self, other: Union["Vector3i", Int32]) -> "Vector3i":
        """Divide a Vector3i or a scalar by a Vector3i."""
        raise NotImplementedError

    @declare_method(
        108,
        operator=FunctionOperator.TRUEDIV,
        alias_of="truediv",
        languages=(RuntimeLanguage.PYTHON,),
    )
    def __truediv__(self, other: Union["Vector3i", Int32]) -> "Vector3i":
        """Divide a Vector3i or a scalar by a Vector3i."""
        raise NotImplementedError

    @declare_method(109)
    def abs(self) -> "Vector3i":
        """Get the absolute value of a Vector3i."""
        raise NotImplementedError

    @declare_method(
        110,
        operator=FunctionOperator.ABS,
        alias_of="abs",
        languages=(RuntimeLanguage.PYTHON,),
    )
    def __abs__(self) -> "Vector3i":
        """Get the absolute value of a Vector3i."""
        raise NotImplementedError

    @declare_method(111)
    def neg(self) -> "Vector3i":
        """Negate a vector."""
        raise NotImplementedError

    @declare_method(
        112,
        operator=FunctionOperator.NEG,
        alias_of="neg",
        languages=(RuntimeLanguage.PYTHON,),
    )
    def __neg__(self) -> "Vector3i":
        """Negate a vector."""
        raise NotImplementedError

    @declare_method(
        113,
        operator=FunctionOperator.ADD,
        alias_of="add",
        languages=(RuntimeLanguage.PYTHON,),
    )
    def __radd__(self, other: Union["Vector3i", Int32]) -> "Vector3i":
        """Add a Vector3i or a scalar to a Vector3i."""
        raise NotImplementedError

    @declare_method(
        114,
        operator=FunctionOperator.SUB,
        alias_of="sub",
        languages=(RuntimeLanguage.PYTHON,),
    )
    def __rsub__(self, other: Union["Vector3i", Int32]) -> "Vector3i":
        """Subtract a Vector3i or a scalar from a Vector3i."""
        raise NotImplementedError

    @declare_method(
        115,
        operator=FunctionOperator.MUL,
        alias_of="mul",
        languages=(RuntimeLanguage.PYTHON,),
    )
    def __rmul__(self, other: Union["Vector3i", Int32]) -> "Vector3i":
        """Multiply a Vector3i or a scalar by a Vector3i."""
        raise NotImplementedError

    @declare_method(
        116,
        operator=FunctionOperator.TRUEDIV,
        alias_of="truediv",
        languages=(RuntimeLanguage.PYTHON,),
    )
    def __rtruediv__(self, other: Union["Vector3i", Int32]) -> "Vector3i":
        """Divide a Vector3i or a scalar by a Vector3i."""
        raise NotImplementedError

    @declare_method(117)
    def dot(self, other: "Vector3i") -> Int32:
        """Calculate the dot product with another vector."""
        raise NotImplementedError

    @declare_method(118)
    def magnitude(self) -> Float32:
        """Calculate the magnitude (length) of the vector."""
        raise NotImplementedError

    @declare_method(119)
    def magnitude2(self) -> Int32:
        """Calculate the squared magnitude of the vector."""
        raise NotImplementedError

    @declare_method(120)
    def normalize(self) -> "Vector3":
        """Return a normalized (unit) vector as floating point vector."""
        raise NotImplementedError

    @declare_method(121)
    def distance(self, other: "Vector3i") -> Float32:
        """Calculate the distance to another vector."""
        raise NotImplementedError

    @declare_method(122)
    def distance2(self, other: "Vector3i") -> Int32:
        """Calculate the squared distance to another vector."""
        raise NotImplementedError

    @declare_method(123)
    def angle(self, other: "Vector3i") -> Float32:
        """Calculate the angle to another vector in radians."""
        raise NotImplementedError

    @declare_method(124)
    def lerp(self, other: "Vector3i", t: Float32) -> "Vector3":
        """Linear interpolation between this vector and another as floating point vector."""
        raise NotImplementedError

    @declare_method(125)
    def cross(self, other: "Vector3i") -> "Vector3i":
        """Calculate the cross product with another vector."""
        raise NotImplementedError


@declare_struct(
    StructType.VECTOR4I,
    is_final=True,
    stability=ObjectStability.STATIC,
)
@final
class Vector4i(Struct):
    """A 4D integer Vector."""

    ZERO = declare_constant(
        100,
        value=lambda: Vector4i(x=0, y=0, z=0, w=0),
        description="The zero Vector4i.",
    )
    ONE = declare_constant(
        101,
        value=lambda: Vector4i(x=1, y=1, z=1, w=1),
        description="The one Vector4i.",
    )
    X_AXIS = declare_constant(
        110,
        value=lambda: Vector4i(x=1, y=0, z=0, w=0),
        description="The x-axis Vector4i.",
    )
    Y_AXIS = declare_constant(
        111,
        value=lambda: Vector4i(x=0, y=1, z=0, w=0),
        description="The y-axis Vector4i.",
    )
    Z_AXIS = declare_constant(
        112,
        value=lambda: Vector4i(x=0, y=0, z=1, w=0),
        description="The z-axis Vector4i.",
    )
    W_AXIS = declare_constant(
        113,
        value=lambda: Vector4i(x=0, y=0, z=0, w=1),
        description="The w-axis Vector4i.",
    )

    x: Int32 = declare_property(
        101,
        is_repr=True,
        description="The x-coordinate of the Vector4i.",
        tag=None,
    )
    y: Int32 = declare_property(
        102,
        is_repr=True,
        description="The y-coordinate of the Vector4i.",
        tag=None,
    )
    z: Int32 = declare_property(
        103,
        is_repr=True,
        description="The z-coordinate of the Vector4i.",
        tag=None,
    )
    w: Int32 = declare_property(
        104,
        is_repr=True,
        description="The w-coordinate of the Vector4i.",
        tag=None,
    )

    @declare_method(101)
    def add(self, other: Union["Vector4i", Int32]) -> "Vector4i":
        """Add a Vector4i or a scalar to a Vector4i."""
        raise NotImplementedError

    @declare_method(
        102,
        operator=FunctionOperator.ADD,
        alias_of="add",
        languages=(RuntimeLanguage.PYTHON,),
    )
    def __add__(self, other: Union["Vector4i", Int32]) -> "Vector4i":
        """Add a Vector4i or a scalar to a Vector4i."""
        raise NotImplementedError

    @declare_method(103)
    def sub(self, other: Union["Vector4i", Int32]) -> "Vector4i":
        """Subtract a Vector4i or a scalar from a Vector4i."""
        raise NotImplementedError

    @declare_method(
        104,
        operator=FunctionOperator.SUB,
        alias_of="sub",
        languages=(RuntimeLanguage.PYTHON,),
    )
    def __sub__(self, other: Union["Vector4i", Int32]) -> "Vector4i":
        """Subtract a Vector4i or a scalar from a Vector4i."""
        raise NotImplementedError

    @declare_method(105)
    def mul(self, other: Union["Vector4i", Int32]) -> "Vector4i":
        """Multiply a Vector4i or a scalar by a Vector4i."""
        raise NotImplementedError

    @declare_method(
        106,
        operator=FunctionOperator.MUL,
        alias_of="mul",
        languages=(RuntimeLanguage.PYTHON,),
    )
    def __mul__(self, other: Union["Vector4i", Int32]) -> "Vector4i":
        """Multiply a Vector4i or a scalar by a Vector4i."""
        raise NotImplementedError

    @declare_method(107)
    def truediv(self, other: Union["Vector4i", Int32]) -> "Vector4i":
        """Divide a Vector4i or a scalar by a Vector4i."""
        raise NotImplementedError

    @declare_method(
        108,
        operator=FunctionOperator.TRUEDIV,
        alias_of="truediv",
        languages=(RuntimeLanguage.PYTHON,),
    )
    def __truediv__(self, other: Union["Vector4i", Int32]) -> "Vector4i":
        """Divide a Vector4i or a scalar by a Vector4i."""
        raise NotImplementedError

    @declare_method(109)
    def abs(self) -> "Vector4i":
        """Get the absolute value of a Vector4i."""
        raise NotImplementedError

    @declare_method(
        110,
        operator=FunctionOperator.ABS,
        alias_of="abs",
        languages=(RuntimeLanguage.PYTHON,),
    )
    def __abs__(self) -> "Vector4i":
        """Get the absolute value of a Vector4i."""
        raise NotImplementedError

    @declare_method(111)
    def neg(self) -> "Vector4i":
        """Negate a vector."""
        raise NotImplementedError

    @declare_method(
        112,
        operator=FunctionOperator.NEG,
        alias_of="neg",
        languages=(RuntimeLanguage.PYTHON,),
    )
    def __neg__(self) -> "Vector4i":
        """Negate a vector."""
        raise NotImplementedError

    @declare_method(
        113,
        operator=FunctionOperator.ADD,
        alias_of="add",
        languages=(RuntimeLanguage.PYTHON,),
    )
    def __radd__(self, other: Union["Vector4i", Int32]) -> "Vector4i":
        """Add a Vector4i or a scalar to a Vector4i."""
        raise NotImplementedError

    @declare_method(
        114,
        operator=FunctionOperator.SUB,
        alias_of="sub",
        languages=(RuntimeLanguage.PYTHON,),
    )
    def __rsub__(self, other: Union["Vector4i", Int32]) -> "Vector4i":
        """Subtract a Vector4i or a scalar from a Vector4i."""
        raise NotImplementedError

    @declare_method(
        115,
        operator=FunctionOperator.MUL,
        alias_of="mul",
        languages=(RuntimeLanguage.PYTHON,),
    )
    def __rmul__(self, other: Union["Vector4i", Int32]) -> "Vector4i":
        """Multiply a Vector4i or a scalar by a Vector4i."""
        raise NotImplementedError

    @declare_method(
        116,
        operator=FunctionOperator.TRUEDIV,
        alias_of="truediv",
        languages=(RuntimeLanguage.PYTHON,),
    )
    def __rtruediv__(self, other: Union["Vector4i", Int32]) -> "Vector4i":
        """Divide a Vector4i or a scalar by a Vector4i."""
        raise NotImplementedError

    @declare_method(117)
    def dot(self, other: "Vector4i") -> Int32:
        """Calculate the dot product with another vector."""
        raise NotImplementedError

    @declare_method(118)
    def magnitude(self) -> Float32:
        """Calculate the magnitude (length) of the vector."""
        raise NotImplementedError

    @declare_method(119)
    def magnitude2(self) -> Int32:
        """Calculate the squared magnitude of the vector."""
        raise NotImplementedError

    @declare_method(120)
    def normalize(self) -> "Vector4":
        """Return a normalized (unit) vector as floating point vector."""
        raise NotImplementedError

    @declare_method(121)
    def distance(self, other: "Vector4i") -> Float32:
        """Calculate the distance to another vector."""
        raise NotImplementedError

    @declare_method(122)
    def distance2(self, other: "Vector4i") -> Int32:
        """Calculate the squared distance to another vector."""
        raise NotImplementedError

    @declare_method(123)
    def angle(self, other: "Vector4i") -> Float32:
        """Calculate the angle to another vector in radians."""
        raise NotImplementedError

    @declare_method(124)
    def lerp(self, other: "Vector4i", t: Float32) -> "Vector4":
        """Linear interpolation between this vector and another as floating point vector."""
        raise NotImplementedError


@declare_method(301)
def vector2(x: Float32, y: Float32) -> Vector2:
    """Create a 2D floating point vector."""
    raise NotImplementedError


@declare_method(301)
def vector3(x: Float32, y: Float32, z: Float32) -> Vector3:
    """Create a 3D floating point vector."""
    raise NotImplementedError


@declare_method(302)
def vector4(x: Float32, y: Float32, z: Float32, w: Float32) -> Vector4:
    """Create a 4D floating point vector."""
    raise NotImplementedError


@declare_method(303)
def vector2i(x: Int32, y: Int32) -> Vector2i:
    """Create a 2D integer vector."""
    raise NotImplementedError


@declare_method(304)
def vector3i(x: Int32, y: Int32, z: Int32) -> Vector3i:
    """Create a 3D integer vector."""
    raise NotImplementedError


@declare_method(305)
def vector4i(x: Int32, y: Int32, z: Int32, w: Int32) -> Vector4i:
    """Create a 4D integer vector."""
    raise NotImplementedError
