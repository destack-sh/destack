from typing import NamedTuple

from .common import EnumType
from .enum import Enum, builtin_enum


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
