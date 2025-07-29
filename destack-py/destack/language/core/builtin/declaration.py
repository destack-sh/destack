from dataclasses import dataclass
from typing import TYPE_CHECKING, Any, Callable, Literal, Optional

from .builtin import EnumType, NodeType, ObjectKind, ObjectStability, StructType, TraitType
from .common import ConstraintType, IndexType
from .enum import Enum, builtin_enum

if TYPE_CHECKING:
    from .node import Node
    from .object import Object
    from .property import PropertyDeclaration
    from .struct import Struct


type_ = type

# pyright: reportIncompatibleVariableOverride=false


@dataclass(slots=True)
class ObjectDeclaration:
    # meta
    cls: type_["Object"]
    kind: ObjectKind | None
    type: int | None
    id: int
    stability: ObjectStability
    is_abstract: bool
    is_frozen: bool
    is_final: bool

    # inheritance
    base_type: int | None
    inherits: list[int]
    inherited_by: list[int]
    extended_by: list[int]

    # content
    properties: list["PropertyDeclaration"]


@dataclass(slots=True)
class StructDeclaration(ObjectDeclaration):
    # meta
    cls: type_["Struct"]
    kind: Literal[ObjectKind.STRUCT]
    type: StructType

    # inheritance
    base_type: StructType | None
    inherits: list[StructType]
    inherited_by: list[StructType]
    extended_by: list[StructType]

    # content
    methods: list["MethodDeclaration"]
    actions: list["ActionDeclaration"]
    constants: list["ConstantDeclaration"]
    tags: list["TagDeclaration"]

    # associations
    enum_types: list[EnumType]
    self_enum_types: list[EnumType]


@dataclass(slots=True)
class NodeDeclaration(ObjectDeclaration):
    # meta
    cls: type_["Node"]
    kind: Literal[ObjectKind.NODE]
    type: NodeType

    # inheritance
    base_type: NodeType | None
    inherits: list[NodeType]
    inherited_by: list[NodeType]
    extended_by: list[NodeType]
    traits: list[TraitType]
    self_traits: list[TraitType]

    # content
    indexes: list["IndexDeclaration"]
    constraints: list["ConstraintDeclaration"]
    permissions: list["PermissionDeclaration"]
    methods: list["MethodDeclaration"]
    actions: list["ActionDeclaration"]
    constants: list["ConstantDeclaration"]
    tags: list["TagDeclaration"]

    # graph
    parent_property: Optional["PropertyDeclaration"]
    parent_types: list[NodeType]
    child_types: list[NodeType]
    ancestor_types: list[NodeType]
    descendant_types: list[NodeType]
    expected_parent_types: list[NodeType]
    expected_child_types: list[NodeType]
    expected_ancestor_types: list[NodeType]
    expected_descendant_types: list[NodeType]

    # associations
    event_types: list[NodeType]
    self_event_types: list[NodeType]
    enum_types: list[EnumType]
    self_enum_types: list[EnumType]


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


@builtin_enum(EnumType.FUNCTION_TYPE)
class FunctionType(Enum):
    PROPERTY = 1, "Property", "Computed property"
    INSTANCE = 2, "Instance", "Instance method"
    STATIC = 3, "Static", "Static method"


@builtin_enum(EnumType.FUNCTION_CARDINALITY)
class FunctionCardinality(Enum):
    UNARY = 1, "Unary", "Single in, single out"
    # UNARY_STREAM = 2, "Unary Stream", "Single in, stream out"

    @property
    def is_boundary(self) -> bool:
        return self < 40


@dataclass(slots=True)
class FunctionDeclaration:
    """Declaration of a FunctionDefinition (internal use only)."""

    id: int
    name: str
    properties: tuple["PropertyDeclaration", ...]
    tags: tuple[str, ...]
    func: Callable
    type: FunctionType
    is_async: bool
    is_abstract: bool


@dataclass(slots=True)
class MethodDeclaration(FunctionDeclaration):
    """Declaration of a MethodDefinition (internal use only)."""


def builtin_method(id: int, *, name: str | None = None, tags: tuple[str, ...] = ()):
    """Declare a builtin Method."""

    def decorate(func):
        # nocheckin: register the method on the BuiltinObject
        return func

    return decorate


@dataclass(slots=True)
class ActionDeclaration(FunctionDeclaration):
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
    component: type_["Object"] | None
    original_component: type_["Object"] | None


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
