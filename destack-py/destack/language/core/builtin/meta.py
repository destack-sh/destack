from typing import TYPE_CHECKING, Callable, NamedTuple

from .common import EnumType
from .enum import Enum, builtin_enum

if TYPE_CHECKING:
    from .property import PropertyDeclaration


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


class IndexDeclaration(NamedTuple):
    """Declaration of an IndexDefinition (internal use only)."""

    id: int
    properties: tuple[str, ...]
    cover: tuple[str, ...] = ()
    name: str | None = None
    type: IndexType = IndexType.BTREE


class ConstraintDeclaration(NamedTuple):
    """Declaration of a ConstraintDefinition (internal use only)."""

    id: int
    type: ConstraintType
    properties: tuple[str, ...]
    name: str | None = None


class PermissionDeclaration(NamedTuple):
    """Declaration of a PermissionDefinition (internal use only)."""

    id: int
    name: str


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


class MethodDeclaration(NamedTuple):
    """Declaration of a MethodDefinition (internal use only)."""

    id: int
    name: str
    properties: tuple["PropertyDeclaration", ...]
    func: Callable
    type: MethodType
    is_async: bool
    is_abstract: bool


def builtin_method(id: int, *, name: str | None = None):
    """Declare a builtin Method."""

    def decorate(func):
        # TODO: register the method on the BuiltinObject
        return func

    return decorate


class ActionDeclaration(MethodDeclaration):
    """Declaration of an ActionDefinition (internal use only)."""

    pass


def builtin_action(id: int, *, name: str | None = None):
    """Declare a builtin Action."""

    def decorate(func):
        return func

    return decorate
