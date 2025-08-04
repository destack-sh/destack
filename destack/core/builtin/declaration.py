import dataclasses
from collections.abc import Sequence
from dataclasses import dataclass
from typing import TYPE_CHECKING, Any, Callable, Literal, Optional, Self

from .builtin import (
    EnumType,
    HandleType,
    NodeType,
    ObjectKind,
    ObjectStability,
    StructType,
    TraitType,
)
from .hoisted import (
    ActionType,
    ConstraintType,
    FunctionOperator,
    IndexType,
    MethodType,
    PrimitiveType,
    RuntimeLanguage,
    RuntimePlatform,
    ScalarType,
    TypeCardinality,
)

if TYPE_CHECKING:
    from destack import Handle, Node, Object, PropertyDeclaration, Struct, Type


type_ = type


@dataclass(slots=True)
class Declaration:
    def diff(self, other: Self) -> dict[str, Any]:
        """Diff this declaration against another."""
        diff = {}
        for field in dataclasses.fields(self):
            if (value := getattr(self, field.name)) != getattr(other, field.name):
                diff[field.name] = value
        return diff

    def __repr__(self) -> str:
        # only repr properties that are set
        props = []
        for prop in dataclasses.fields(self):
            if (prop_value := getattr(self, prop.name)) is not None:
                props.append(f"{prop.name}={prop_value!r}")
        return f"<{self.__class__.__name__} {' '.join(props)}>"


@dataclass(slots=True, repr=False)
class TypeDeclaration(Declaration):
    """Type annotation to be turned into a Property/Type."""

    # cardinality
    cardinality: TypeCardinality
    key_type: "TypeDeclaration | None" = None
    value_type: "TypeDeclaration | None" = None
    element_types: Sequence["TypeDeclaration"] | None = None

    # scalar
    scalar_type: ScalarType | None = None
    primitive_type: PrimitiveType | None = None
    struct_type: StructType | None = None
    handle_type: HandleType | None = None
    enum_type: EnumType | None = None
    node_types: Sequence[NodeType] | None = None  # for node scalar nodes

    # flags
    is_required: bool = True
    is_self: bool = False
    is_any: bool = False

    _type: Optional["Type"] = None  # cached

    def to_type(self) -> "Type":
        """Map this TypeDeclaration to a Type."""
        if self._type is None:
            from ..common import Type

            self._type = Type.from_declaration(self)

        return self._type


@dataclass(slots=True, repr=False)
class ObjectDeclaration(Declaration):
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


@dataclass(slots=True, repr=False)
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


@dataclass(slots=True, repr=False)
class NodeDeclaration(ObjectDeclaration):
    # meta
    cls: type_["Node"]
    kind: Literal[ObjectKind.NODE]
    type: NodeType
    is_singleton: bool

    # inheritance
    struct_type: StructType | None
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


@dataclass(slots=True, repr=False)
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


@dataclass(slots=True, repr=False)
class IndexDeclaration(Declaration):
    """Declaration of an IndexDefinition (internal use only)."""

    id: int
    properties: tuple[str, ...]
    cover: tuple[str, ...] = ()
    tags: tuple[str, ...] = ()
    name: str | None = None
    type: IndexType = IndexType.BTREE


@dataclass(slots=True, repr=False)
class ConstraintDeclaration(Declaration):
    """Declaration of a ConstraintDefinition (internal use only)."""

    id: int
    type: ConstraintType
    properties: tuple[str, ...]
    description: str | None = None
    name: str | None = None
    tags: tuple[str, ...] = ()


@dataclass(slots=True, repr=False)
class PermissionDeclaration(Declaration):
    """Declaration of a PermissionDefinition (internal use only)."""

    id: int
    name: str
    description: str
    tags: tuple[str, ...] = ()


@dataclass(slots=True, repr=False)
class FunctionDeclaration(Declaration):
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


@dataclass(slots=True, repr=False)
class MethodDeclaration(FunctionDeclaration):
    """Declaration of a MethodDefinition (internal use only)."""

    # meta
    type: MethodType

    # content
    input_properties: tuple["PropertyDeclaration", ...]
    output_properties: tuple["PropertyDeclaration", ...]
    output_is_scalar: bool


def declare_method(
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
        # nocheckin(py, language): register Functions (methods, actions/messages) on the Object
        return func

    return decorate


@dataclass(slots=True, repr=False)
class ActionDeclaration(FunctionDeclaration):
    """Declaration of an ActionDefinition (internal use only)."""

    type: ActionType


def declare_action(
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


@dataclass(slots=True, repr=False)
class MessageDeclaration(Declaration):
    """Declaration of a MessageDefinition (internal use only)."""

    id: int
    name: str
    description: str
    properties: tuple["PropertyDeclaration", ...]


@dataclass(slots=True, repr=False)
class TagDeclaration(Declaration):
    """Declaration of a TagDefinition (internal use only)."""

    id: int
    name: str
    description: str


@dataclass(slots=True, repr=False)
class ConstantDeclaration(Declaration):
    """Declaration of a builtin Constant (may be deferred)."""

    id: int
    value: Any | Callable[[], Any]
    is_deferred: bool
    description: str | None
    name: str | None
    component: type_["Object"] | None
    original_component: type_["Object"] | None


def declare_constant[T](
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
