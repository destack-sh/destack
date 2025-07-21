from dataclasses import dataclass
from typing import TYPE_CHECKING, Any, Callable

from .common import EnumType
from .enum import Enum, builtin_enum

if TYPE_CHECKING:
    from .object import BuiltinObject
    from .property import PropertyDeclaration


type_ = type


@builtin_enum(EnumType.CONSTRAINT_TYPE)
class ConstraintType(Enum):
    """Type of a Constraint."""

    UNIQUE = 1
    # CHECK, ...


@builtin_enum(EnumType.INDEX_TYPE)
class IndexType(Enum):
    """Type of an Index."""

    BTREE = 1
    # HASH, ...


@dataclass(slots=True)
class IndexDeclaration:
    """Declaration of an IndexDefinition (internal use only)."""

    id: int
    properties: tuple[str, ...]
    cover: tuple[str, ...] = ()
    tags: tuple[str, ...] = ()
    name: str | None = None
    type: IndexType = IndexType.BTREE


@dataclass(slots=True)
class ConstraintDeclaration:
    """Declaration of a ConstraintDefinition (internal use only)."""

    id: int
    type: ConstraintType
    properties: tuple[str, ...]
    description: str | None = None
    name: str | None = None
    tags: tuple[str, ...] = ()


@dataclass(slots=True)
class PermissionDeclaration:
    """Declaration of a PermissionDefinition (internal use only)."""

    id: int
    name: str
    description: str
    tags: tuple[str, ...] = ()


@builtin_enum(EnumType.METHOD_TYPE)
class MethodType(Enum):
    PROPERTY = 1, "Property", "Computed property"
    INSTANCE = 2, "Instance", "Instance method"
    STATIC = 3, "Static", "Static method"


@builtin_enum(EnumType.METHOD_CARDINALITY)
class MethodCardinality(Enum):
    UNARY = 1, "Unary", "Single in, single out"
    # UNARY_STREAM = 2, "Unary Stream", "Single in, stream out"

    @property
    def is_boundary(self) -> bool:
        return self < 40


@dataclass(slots=True)
class MethodDeclaration:
    """Declaration of a MethodDefinition (internal use only)."""

    id: int
    name: str
    properties: tuple["PropertyDeclaration", ...]
    tags: tuple[str, ...]
    func: Callable
    type: MethodType
    is_async: bool
    is_abstract: bool


def builtin_method(id: int, *, name: str | None = None, tags: tuple[str, ...] = ()):
    """Declare a builtin Method."""

    def decorate(func):
        # TODO: register the method on the BuiltinObject
        return func

    return decorate


@dataclass(slots=True)
class ActionDeclaration(MethodDeclaration):
    """Declaration of an ActionDefinition (internal use only)."""

    pass


def builtin_action(id: int, *, name: str | None = None, tags: tuple[str, ...] = ()):
    """Declare a builtin Action."""

    def decorate(func):
        return func

    return decorate


@dataclass(slots=True)
class TagDeclaration:
    """Declaration of a TagDefinition (internal use only)."""

    id: int
    name: str
    description: str


@dataclass(slots=True)
class ConstantDeclaration:
    """Declaration of a builtin Constant (may be deferred)."""

    id: int
    value: Any | Callable[[], Any]
    is_deferred: bool
    description: str | None
    name: str | None
    component: type_["BuiltinObject"] | None
    original_component: type_["BuiltinObject"] | None


def builtin_constant[T](
    id: int,
    value: T | Callable[[], T],
    *,
    description: str | None = None,
) -> T:  # replaced with T after finalization
    """Declare a builtin Constant. Constants are replaced with their value during finalization."""

    declaration = ConstantDeclaration(
        id=id,
        description=description,
        value=value,
        is_deferred=isinstance(value, Callable),
        # set during class processing
        name=None,
        component=None,
        original_component=None,
    )
    return declaration  # type: ignore
