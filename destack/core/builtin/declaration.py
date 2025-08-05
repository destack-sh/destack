import dataclasses
import inspect
import typing
from collections.abc import Collection, Sequence
from dataclasses import dataclass
from typing import TYPE_CHECKING, Any, Callable, Literal, Optional, Self, cast

from .const import UNSET
from .hoisted import (
    ActionType,
    ConstraintType,
    FunctionOperator,
    IndexType,
    MethodType,
    PrimitiveType,
    RuntimeLanguage,
    RuntimePlatform,
    RuntimeType,
    ScalarType,
    TypeCardinality,
)
from .universe import (
    EnumType,
    HandleType,
    NodeType,
    ObjectKind,
    ObjectStability,
    StructType,
    TraitType,
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
    is_stream: bool = False

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
    into_node_types: list[NodeType]


@dataclass(slots=True, repr=False)
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
    base_struct_type: StructType | None


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
    """Declaration of an IndexDefinition."""

    id: int
    properties: tuple[str, ...]
    cover: tuple[str, ...] = ()
    tags: tuple[str, ...] = ()
    name: str | None = None
    type: IndexType = IndexType.BTREE


@dataclass(slots=True, repr=False)
class ConstraintDeclaration(Declaration):
    """Declaration of a ConstraintDefinition."""

    id: int
    type: ConstraintType
    properties: tuple[str, ...]
    description: str | None = None
    name: str | None = None
    tags: tuple[str, ...] = ()


@dataclass(slots=True, repr=False)
class PermissionDeclaration(Declaration):
    """Declaration of a PermissionDefinition."""

    id: int
    name: str
    description: str
    tags: tuple[str, ...] = ()


@dataclass(slots=True, repr=False)
class FunctionDeclaration(Declaration):
    """Declaration of a FunctionDefinition."""

    # meta
    id: int
    name: str
    description: str
    outer_func: Callable | classmethod | property
    inner_func: Callable
    is_async: bool
    is_internal: bool

    # content
    tags: tuple[str, ...]

    runtimes: tuple[RuntimeType, ...]


@dataclass(slots=True, repr=False)
class SignatureDeclaration(Declaration):
    """Declaration of a function signature."""

    input_properties: list["PropertyDeclaration"]
    output_property: "PropertyDeclaration | None"


def _get_runtimes(
    platforms: tuple[RuntimePlatform, ...],
    languages: tuple[RuntimeLanguage, ...],
    runtimes: tuple[RuntimeType, ...],
) -> tuple[RuntimeType, ...]:
    """
    Get the runtimes from the platforms and languages.
    """
    if platforms or languages:
        if runtimes:
            raise ValueError("only one of runtimes or platforms/languages may be provided")
        if not platforms:
            platforms = tuple(RuntimePlatform)
        if not languages:
            languages = tuple(RuntimeLanguage)
        runtimes = tuple(
            RuntimeType.__options_by_id__[platform + language]
            for platform in platforms
            for language in languages
            if platform + language in RuntimeType.__options_by_id__
        )
    return runtimes


def _parse_signature(
    qualname: str,
    func: Callable,
    parameters: Collection[inspect.Parameter],
    return_annotation: Any,
    operator: FunctionOperator | None,
) -> SignatureDeclaration:
    from .property import PropertyDeclaration
    from .type import parse_type_declaration

    input_properties: list[PropertyDeclaration] = []
    output_property: PropertyDeclaration | None = None

    # process input parameters
    for param in parameters:
        param_name = param.name
        if param_name == "self" or param_name == "cls":
            continue
        prop_py_type = (
            param.annotation if param.annotation != inspect.Parameter.empty else type(None)
        )
        prop = PropertyDeclaration(
            name=param_name,
            py_type=prop_py_type,
            default_value=param.default if param.default != inspect.Parameter.empty else UNSET,
        )
        try:
            prop.type = parse_type_declaration(prop_py_type, is_builtin=True)
        except Exception as e:
            raise ValueError(
                f"unexpected parameter type: {qualname}.{param_name} ({prop.py_type})"
            ) from e

        input_properties.append(prop)

    # process return type
    if return_annotation != inspect.Parameter.empty and return_annotation is not None:
        prop = PropertyDeclaration(name="return", py_type=return_annotation)
        try:
            prop.type = parse_type_declaration(prop.py_type, is_builtin=True)
        except Exception as e:
            raise ValueError(f"unexpected return type: {qualname} ({prop.py_type})") from e
        output_property = prop

    return SignatureDeclaration(
        input_properties=input_properties,
        output_property=output_property,
    )


@dataclass(slots=True, repr=False)
class MethodDeclaration(FunctionDeclaration):
    """Declaration of a MethodDefinition."""

    # meta
    type: MethodType
    is_implemented: bool

    # content
    input_properties: tuple["PropertyDeclaration", ...]
    output_property: "PropertyDeclaration | None"


def _process_method(
    # meta
    func: Callable | classmethod | property,
    *,
    id: int,
    name: str | None,
    operator: FunctionOperator | None,
    is_implemented: bool,
    is_internal: bool,
    # associations
    tags: tuple[str, ...],
    runtimes: tuple[RuntimeType, ...],
) -> tuple[Callable, MethodDeclaration]:
    """
    Process a method to create a MethodDeclaration.
    """

    # unwrap class methods and properties
    outer_func = func
    type: MethodType = MethodType.INSTANCE
    if isinstance(func, classmethod):
        inner_func = func.__func__
        type = MethodType.STATIC
    elif isinstance(func, property):
        inner_func = cast(Callable, func.fget)
        type = MethodType.PROPERTY
    else:
        inner_func = func

    # meta
    qualname = f"{inner_func.__module__}.{inner_func.__qualname__}"
    is_async = inspect.iscoroutinefunction(inner_func)

    # parse method signature
    signature = inspect.signature(inner_func)
    signature_parameters = signature.parameters.values()
    signature_return_annotation = signature.return_annotation
    if operator == FunctionOperator.ITER:
        signature_return_annotation = typing.get_args(signature_return_annotation)[0]
    signature_declaration = _parse_signature(
        qualname=qualname,
        func=inner_func,
        parameters=signature_parameters,
        return_annotation=signature_return_annotation,
        operator=operator,
    )

    # implementation must be empty
    if not is_implemented and len(inner_func.__code__.co_code) > 12:
        source = inspect.getsource(inner_func).strip()
        raise ValueError(f"abstract method declaration must be empty: {qualname}\n{source}")

    declaration = MethodDeclaration(
        id=id,
        name=name or inner_func.__name__,
        description=inner_func.__doc__ or "",
        outer_func=outer_func,
        inner_func=inner_func,
        is_implemented=is_implemented,
        is_async=is_async,
        is_internal=is_internal,
        tags=tags,
        runtimes=runtimes,
        type=type,
        input_properties=tuple(signature_declaration.input_properties),
        output_property=signature_declaration.output_property,
    )

    return inner_func, declaration


def declare_method(
    id: int,
    *,
    name: str | None = None,
    tags: tuple[str, ...] = (),
    proxies_method: str | None = None,
    proxies_runtime: RuntimeType | None = None,
    type: MethodType = MethodType.INSTANCE,
    operator: FunctionOperator | None = None,
    platforms: tuple[RuntimePlatform, ...] = (),
    languages: tuple[RuntimeLanguage, ...] = (),
    runtimes: tuple[RuntimeType, ...] = (),
    is_implemented: bool = False,
    is_internal: bool = False,
):
    """Declare a builtin Method."""

    def decorate(func):
        all_runtimes = _get_runtimes(platforms=platforms, languages=languages, runtimes=runtimes)
        func, declaration = _process_method(
            # meta
            func,
            id=id,
            name=name,
            operator=operator,
            runtimes=all_runtimes,
            is_implemented=is_implemented,
            is_internal=is_internal,
            # associations
            tags=tags,
        )
        if TYPE_CHECKING:
            return func
        else:
            return declaration

    return decorate


@dataclass(slots=True, repr=False)
class ActionDeclaration(FunctionDeclaration):
    """
    Declaration of an ActionDefinition.
    Actions must communicate with Messages.
    """

    type: ActionType

    input_message_type: StructType | None
    output_message_type: StructType | None


def _process_action(
    # meta
    func: Callable,
    *,
    id: int,
    name: str | None,
    type: ActionType,
    runtimes: tuple[RuntimeType, ...] = (),
    is_internal: bool,
    # associations
    tags: tuple[str, ...] = (),
) -> tuple[Callable, ActionDeclaration]:
    """
    Process an action to create an ActionDeclaration.
    """

    from .type import parse_type_declaration

    # meta
    qualname = f"{func.__module__}.{func.__qualname__}"
    is_async = inspect.iscoroutinefunction(func)

    # parse action signature
    signature = inspect.signature(func)
    signature_parameters = signature.parameters.values()

    # parse input type
    input_message_type = None
    for param in signature_parameters:
        if param.name == "self" or param.name == "cls":
            continue
        elif param.name == "request":
            param_py_type = param.annotation
            if type in (ActionType.STREAM_IN_UNARY_OUT, ActionType.STREAM_IN_STREAM_OUT):
                # strip stream type
                param_py_type = typing.get_args(param_py_type)[0]
            param_type = parse_type_declaration(param_py_type, is_builtin=True)
            if param_type.cardinality != TypeCardinality.SCALAR or param_type.struct_type is None:
                raise ValueError(
                    f"invalid input message type: {qualname}.{param.name} ({param.annotation}->{param_type!r})"
                )
            input_message_type = param_type.struct_type
        else:
            raise ValueError(f"extraneous parameter: {qualname}.{param.name} ({param.annotation})")

    # parse output type
    signature_return_annotation = signature.return_annotation
    output_message_type = None
    if (
        signature_return_annotation != inspect.Parameter.empty
        and signature_return_annotation is not None
    ):
        if type in (ActionType.UNARY_IN_STREAM_OUT, ActionType.STREAM_IN_STREAM_OUT):
            # strip stream type
            signature_return_annotation = typing.get_args(signature_return_annotation)[0]
        output_message_type = parse_type_declaration(signature_return_annotation, is_builtin=True)
        if (
            output_message_type.cardinality != TypeCardinality.SCALAR
            or output_message_type.struct_type is None
        ):
            raise ValueError(
                f"invalid output message type: {qualname}.<output> ({signature_return_annotation}->{output_message_type!r})"
            )
        output_message_type = output_message_type.struct_type

    # implementation must be empty
    if len(func.__code__.co_code) > 12:
        source = inspect.getsource(func).strip()
        raise ValueError(f"abstract action declaration must be empty: {qualname}\n{source}")

    declaration = ActionDeclaration(
        id=id,
        name=name or func.__name__,
        description=func.__doc__ or "",
        outer_func=func,
        inner_func=func,
        is_async=is_async,
        is_internal=is_internal,
        tags=tags,
        runtimes=runtimes,
        type=type,
        input_message_type=input_message_type,
        output_message_type=output_message_type,
    )
    return func, declaration


def declare_action(
    # meta
    id: int,
    *,
    name: str | None = None,
    type: ActionType,
    platforms: tuple[RuntimePlatform, ...] = (),
    languages: tuple[RuntimeLanguage, ...] = (),
    runtimes: tuple[RuntimeType, ...] = (),
    is_internal: bool = False,
    # associations
    tags: tuple[str, ...] = (),
):
    """Declare a builtin Action."""

    def decorate(func):
        all_runtimes = _get_runtimes(platforms=platforms, languages=languages, runtimes=runtimes)
        func, declaration = _process_action(
            # meta
            func,
            id=id,
            name=name,
            type=type,
            runtimes=all_runtimes,
            is_internal=is_internal,
            # associations
            tags=tags,
        )
        if TYPE_CHECKING:
            return func
        else:
            return declaration

    return decorate


@dataclass(slots=True, repr=False)
class MessageDeclaration(Declaration):
    """Declaration of a MessageDefinition."""

    id: int
    name: str
    description: str
    properties: tuple["PropertyDeclaration", ...]


@dataclass(slots=True, repr=False)
class TagDeclaration(Declaration):
    """Declaration of a TagDefinition."""

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
