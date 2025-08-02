from dataclasses import dataclass
from typing import TYPE_CHECKING, Any, Callable, Literal, Optional

from .builtin import (
    EnumType,
    HandleType,
    NodeType,
    ObjectKind,
    ObjectStability,
    StructType,
    TraitType,
)
from .common import (
    ActionType,
    ConstraintType,
    FunctionOperator,
    IndexType,
    MethodType,
    RuntimeLanguage,
    RuntimePlatform,
)

if TYPE_CHECKING:
    from .handle import Handle
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
    is_singleton: bool

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
    message_types: list[StructType]
    self_message_types: list[StructType]


@dataclass(slots=True)
class HandleDeclaration(ObjectDeclaration):
    # meta
    cls: type_["Handle"]
    kind: Literal[ObjectKind.HANDLE]
    type: HandleType

    # inheritance
    base_type: HandleType | None
    inherits: list[HandleType]
    inherited_by: list[HandleType]
    extended_by: list[HandleType]

    # content
    properties: list["PropertyDeclaration"]
    methods: list["MethodDeclaration"]
    constants: list["ConstantDeclaration"]
    tags: list["TagDeclaration"]

    # associations
    enum_types: list[EnumType]
    self_enum_types: list[EnumType]
    event_types: list[NodeType]
    self_event_types: list[NodeType]


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


@dataclass(slots=True)
class FunctionDeclaration:
    """Declaration of a FunctionDefinition (internal use only)."""

    # meta
    id: int
    name: str
    description: str
    func: Callable
    is_async: bool
    is_abstract: bool
    is_internal: bool

    # content
    tags: tuple[str, ...]

    languages: tuple[RuntimeLanguage, ...] | None
    platforms: tuple[RuntimePlatform, ...] | None


@dataclass(slots=True)
class MethodDeclaration(FunctionDeclaration):
    """Declaration of a MethodDefinition (internal use only)."""

    # meta
    type: MethodType

    # content
    input_properties: tuple["PropertyDeclaration", ...]
    output_properties: tuple["PropertyDeclaration", ...]
    output_is_scalar: bool


def builtin_method(
    id: int,
    *,
    name: str | None = None,
    tags: tuple[str, ...] = (),
    proxies_method: str | None = None,
    proxies_runtime: RuntimeLanguage | None = None,
    type: MethodType = MethodType.INSTANCE,
    operator: FunctionOperator | None = None,
    languages: tuple[RuntimeLanguage, ...] | None = None,
    platforms: tuple[RuntimePlatform, ...] | None = None,
    is_internal: bool = False,
):
    """Declare a builtin Method."""

    def decorate(func):
        # nocheckin(py, language): register the functions (methods, actions/messages) on the Object
        return func

    return decorate


@dataclass(slots=True)
class ActionDeclaration(FunctionDeclaration):
    """Declaration of an ActionDefinition (internal use only)."""

    type: ActionType


def builtin_action(
    id: int,
    *,
    name: str | None = None,
    tags: tuple[str, ...] = (),
    type: ActionType = ActionType.UNARY_IN_UNARY_OUT,
    languages: tuple[RuntimeLanguage, ...] | None = None,
    platforms: tuple[RuntimePlatform, ...] | None = None,
    is_internal: bool = False,
):
    """Declare a builtin Action."""

    def decorate(func):
        return func

    return decorate


@dataclass(slots=True)
class MessageDeclaration:
    """Declaration of a MessageDefinition (internal use only)."""

    id: int
    name: str
    description: str
    properties: tuple["PropertyDeclaration", ...]


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
