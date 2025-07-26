import dataclasses
import types
import typing
from collections.abc import Sequence
from dataclasses import dataclass
from typing import (
    TYPE_CHECKING,
    Any,
    Callable,
    Optional,
    TypeAliasType,
    assert_never,
)

from destack.utils.func import hash_stable
from destack.utils.string import Casing, to_casing

from .builtin import (
    NODE_TYPES,
    NodeType,
    StructType,
    TraitType,
)
from .common import (
    PRIMITIVE_TYPE_BY_ANNOTATION,
    CascadeAction,
    EdgeType,
    EnumType,
    PrimitiveType,
    PropertyZone,
    ScalarType,
    TypeCardinality,
    ValueFactory,
)
from .const import UNSET

if TYPE_CHECKING:
    from destack.language import (
        BuiltinObject,
        CheckedType,
        Condition,
        PropertyDefinition,
        PropertyReference,
        Sort,
        Type,
        TypeConstraint,
    )

type_ = type


def resolve_enum_type(class_name: str) -> EnumType | None:
    """Get the EnumType for the given enum name."""
    enum_name = to_casing(class_name, Casing.ALL_CAPS)
    if enum_type := EnumType.__members__.get(enum_name):
        return enum_type
    enum_name = class_name.upper()
    if enum_type := EnumType.__members__.get(enum_name):
        return enum_type
    return None


def resolve_struct_type(class_name: str) -> StructType | None:
    """Get the StructType for the given struct name."""
    enum_name = to_casing(class_name, Casing.ALL_CAPS)
    if struct_type := StructType.__members__.get(enum_name):
        return struct_type
    enum_name = class_name.upper()
    if struct_type := StructType.__members__.get(enum_name):
        return struct_type
    return None


def resolve_trait_type(name: str) -> TraitType | None:
    """Get a trait by name."""
    from .trait import TRAIT_PREFIXES

    for trait_prefix in TRAIT_PREFIXES:
        if name.startswith(trait_prefix):
            name = name[len(trait_prefix) :]
            break
    name = to_casing(name, Casing.ALL_CAPS)
    trait = TraitType.__members__.get(name)
    return trait


def resolve_node_types(class_name: str) -> tuple[NodeType, ...] | None:
    """Get the NodeType for the given node name."""
    enum_name = to_casing(class_name, Casing.ALL_CAPS)
    if node_type := NodeType.__members__.get(enum_name):
        return (node_type,)
    if node_type := NodeType.__members__.get(class_name.upper()):
        return (node_type,)
    if class_name == "Node":
        return NODE_TYPES
    return None


@dataclass(slots=True)
class TypeDeclaration:
    """Type annotation to be turned into a Property/Type."""

    # cardinality
    cardinality: TypeCardinality = UNSET
    key_type: "TypeDeclaration | None" = None
    value_type: "TypeDeclaration | None" = None
    element_types: Sequence["TypeDeclaration"] | None = None

    # scalar
    scalar_type: ScalarType | None = None
    primitive_type: PrimitiveType | None = None
    enum_type: EnumType | None = None
    struct_type: StructType | None = None
    node_types: Sequence[NodeType] | None = None  # for node scalar nodes
    literal_value: Any | None = None
    union_types: Sequence["TypeDeclaration"] | None = None

    # flags
    is_required: bool = True
    is_self: bool = False

    _basic_type: Optional["Type"] = None  # cached

    def to_type(self) -> "Type":
        """Map this TypeDeclaration to a Type."""
        if self._basic_type is not None:
            return self._basic_type

        from .type import Type
        from .value import Value

        # type
        type = Type(
            # cardinality
            cardinality=self.cardinality,
            key_type=self.key_type.to_type() if self.key_type else None,
            value_type=self.value_type.to_type() if self.value_type else None,
            element_types=[t.to_type() for t in self.element_types] if self.element_types else None,
            # scalar
            scalar_type=self.scalar_type,
            primitive_type=self.primitive_type,
            enum_type=self.enum_type,
            node_types=list(self.node_types) if self.node_types else None,
            struct_type=self.struct_type,
            literal_value=Value.wrap(self.literal_value) if self.literal_value else None,
            union_types=[t.to_type() for t in self.union_types] if self.union_types else None,
            # flags
            is_required=self.is_required,
        )

        self._basic_type = type
        return type


@dataclass(slots=True)
class CheckedTypeDeclaration(TypeDeclaration):
    """A CheckedTypeDeclaration is a CheckedType with constraints and flags."""

    # flags
    is_required: bool = True
    is_identity: bool = False
    is_self: bool = False
    is_main: bool = False

    # constraints
    constraint: "TypeConstraint | None" = None

    # defaults
    default_value: Any | None = None
    default_factory: ValueFactory | None = None

    # cached
    _type: Optional["CheckedType"] = None  # cached

    constraint: "TypeConstraint | None" = None

    _type: Optional["CheckedType"] = None  # cached

    def to_type(self) -> "CheckedType":
        """Map this TypeDeclaration to a Type."""
        if self._type is not None:
            return self._type

        from .type import CheckedType, CollectionConstraint, NumberConstraint, StringConstraint
        from .value import Value

        # type
        default_value = (
            Value.wrap(self.default_value)
            if self.default_value is not UNSET and self.default_value is not None
            else None
        )

        # build constraints first since they're frozen
        string_constraint = None
        number_constraint = None
        collection_constraint = None

        # constraints
        if self.constraint is not None:
            if isinstance(self.constraint, StringConstraint):
                string_constraint = self.constraint
            elif isinstance(self.constraint, NumberConstraint):
                number_constraint = self.constraint
            elif isinstance(self.constraint, CollectionConstraint):
                collection_constraint = self.constraint
            else:
                assert_never(self.constraint)

        # type
        type = CheckedType(
            # cardinality
            cardinality=self.cardinality,
            key_type=self.key_type.to_type() if self.key_type else None,
            value_type=self.value_type.to_type() if self.value_type else None,
            element_types=[t.to_type() for t in self.element_types] if self.element_types else None,
            # scalar
            scalar_type=self.scalar_type,
            primitive_type=self.primitive_type,
            enum_type=self.enum_type,
            node_types=list(self.node_types) if self.node_types else None,
            struct_type=self.struct_type,
            literal_value=Value.wrap(self.literal_value) if self.literal_value else None,
            union_types=[t.to_type() for t in self.union_types] if self.union_types else None,
            is_required=self.is_required,
            # default
            default_value=default_value,
            default_factory=self.default_factory,
            # constraints
            string_constraint=string_constraint,
            number_constraint=number_constraint,
            collection_constraint=collection_constraint,
        )

        self._type = type
        return type


@dataclass(eq=False, slots=True)
class PropertyDeclaration(CheckedTypeDeclaration):
    """
    A system-defined attribute of a BuiltinObject (Struct or Node).
    PropertyDeclarations are turned into PropertyDefinitions during construction,
     PropertyDeclarations (like their *Declaration brethren) are only for internal use.
    """

    py_type: Any = None

    # meta
    type: PropertyZone = PropertyZone.MEMBER
    id: int | None = None
    ord: int | None = None
    name: str = UNSET  # name from LHS of assignment
    description: str | None = None
    component: type_["BuiltinObject"] = UNSET  # builtin object component
    original_component: type_["BuiltinObject"] = UNSET  # original component (first in chain)
    tags: tuple[str, ...] = ()

    # relationships
    edge_type: EdgeType | None = None
    cascade: CascadeAction | None = None

    # flags
    is_unique: bool = False  # unique in DB
    is_wired: bool = False  # serialized onto wire
    is_stored: bool = False  # stored in DB
    is_repr: bool = False  # included in BuiltinObject.__repr__
    is_hash: bool = True  # included in BuiltinObject.__hash__
    is_eq: bool = True  # included in BuiltinObject.equals check
    is_internal: bool = False  # managed internally by the system
    is_computed: bool = False  # set automatically at runtime
    is_readonly: bool = False  # can only be set once (at init time)
    is_main: bool = False  # root property (for return types with single value)

    _ref: Optional["PropertyReference"] = None
    _definition: Optional["PropertyDefinition"] = None

    def __str__(self):
        if self.component is None:
            return "<detached>"
        return f"{self.component.__name__}.{self.name}"

    def __repr__(self):
        return f"<{self.__class__.__name__} {self!s} ({self.id or '<unset>'})>"

    def __eq__(self, other: Any) -> bool:
        if not isinstance(other, PropertyDeclaration):
            return False
        return self.component == other.component and self.id == other.id

    def hash(self):
        """Hash the Property identity."""
        return hash_stable((self.component.__name__, self.id))

    __hash__ = hash  # type: ignore

    def clone(self):
        return dataclasses.replace(
            self,
            component=None,
            original_component=self.original_component,
            _ref=None,
            _type=None,
            _definition=None,
        )

    def to_ref(self) -> "PropertyReference":
        """A pointer to this Property. `to_ref()` for consistency with `Node.to_ref()`."""

        if self._ref is None:
            from .relation import PropertyReference, PropertyReferenceType

            assert self.component is not None, f"{self!r} has no component"
            assert self.id is not None, f"{self!r} has no id"
            metatype = getattr(self.component, "metatype", None)

            if self.component.__is_trait__:
                ref = PropertyReference(
                    type=PropertyReferenceType.BUILTIN,
                    trait_type=metatype,
                    id=self.id,
                )
            elif self.component.__is_node__:
                ref = PropertyReference(
                    type=PropertyReferenceType.BUILTIN,
                    node_type=metatype,
                    id=self.id,
                )
            else:
                ref = PropertyReference(
                    type=PropertyReferenceType.BUILTIN,
                    struct_type=metatype,
                    id=self.id,
                )
            self._ref = ref

        return self._ref

    @property
    def code_name(self) -> str:
        return self.name

    @property
    def has_id(self) -> int:
        return self.id is not None and self.id is not UNSET

    @property
    def is_optional(self) -> bool:
        return not self.is_required

    @property
    def definition(self) -> "PropertyDefinition":
        if self._definition is None:
            from .definition import PropertyDefinition

            self._definition = PropertyDefinition.from_declaration(self)
        return self._definition

    def determine(self, object_type: NodeType | StructType | None, is_root_node: bool) -> None:
        """Determine type information from annotation, add _ptr property if needed."""
        if not self.is_wired:
            return  # runtime only, nothing to do

        # parse annotation
        try:
            annotation = parse_type_annotation(self.py_type, is_builtin_member=True)
        except Exception as e:
            raise ValueError(
                f"invalid type: {self.component.__name__}.{self.name} ({self.py_type})"
            ) from e
        # copy over all the annotation info
        for f in dataclasses.fields(annotation):
            setattr(self, f.name, getattr(annotation, f.name))

        # resolve self type
        if annotation.is_self and object_type is not None:
            if isinstance(object_type, NodeType):
                self.node_types = (object_type,)
            elif isinstance(object_type, StructType):
                self.struct_type = object_type
            else:
                assert_never(object_type)

        # parent must be optional
        if self.name == "parent" and self.is_required:
            raise ValueError(f"Entity.parent must be optional: {self!r}")
        # root nodes don't have a parent
        if self.name == "parent" and is_root_node:
            self.node_types = ()
        # 'type' must be 30
        if (self.name == "type") != (self.id == 100):
            raise ValueError(f"'type' must be 100: {self!r}")

        # default to None if not required and no default
        if not self.is_required and self.default_value is UNSET:
            self.default_value = None
        # default to regular node references
        if self.scalar_type == ScalarType.NODE_REFERENCE and self.edge_type is None:
            self.edge_type = EdgeType.REGULAR
        # references get a _ptr property (which is wired/stored)
        if self.scalar_type == ScalarType.NODE_REFERENCE:
            # (don't want lists of Node references or Property references in Nodes, it's a mess)
            assert self.cardinality == TypeCardinality.SCALAR or not self.component.__is_node__, (
                f"invalid list: {self!r}"
            )
            self.primitive_type = PrimitiveType.JSON

    #
    # Querying
    #

    def eq(self, value: Any) -> "Condition":
        from destack.language import Condition, ConditionalType

        if value is None:
            return self.not_exists()
        return Condition.of(self, ConditionalType.EQUALS, value=value)

    def neq(self, value: Any) -> "Condition":
        from destack.language import Condition, ConditionalType

        if value is None:
            return self.exists()
        return Condition.of(self, ConditionalType.NOT_EQUALS, value=value)

    def gt(self, value: Any) -> "Condition":
        from destack.language import Condition, ConditionalType

        return Condition.of(self, ConditionalType.GREATER_THAN, value=value)

    def gte(self, value: Any) -> "Condition":
        from destack.language import Condition, ConditionalType

        return Condition.of(self, ConditionalType.GREATER_THAN_OR_EQUALS, value=value)

    def lt(self, value: Any) -> "Condition":
        from destack.language import Condition, ConditionalType

        return Condition.of(self, ConditionalType.LESS_THAN, value=value)

    def lte(self, value: Any) -> "Condition":
        from destack.language import Condition, ConditionalType

        return Condition.of(self, ConditionalType.LESS_THAN_OR_EQUALS, value=value)

    def starts_with(self, value: str) -> "Condition":
        from destack.language import Condition, ConditionalType

        return Condition.of(self, ConditionalType.STARTS_WITH, value=value)

    def ends_with(self, value: str) -> "Condition":
        from destack.language import Condition, ConditionalType

        return Condition.of(self, ConditionalType.ENDS_WITH, value=value)

    def in_(self, *values: Any) -> "Condition":
        from destack.language import Condition, ConditionalType

        return Condition.of(self, ConditionalType.IN, value=values)

    def not_in(self, *values: Any) -> "Condition":
        from destack.language import Condition, ConditionalType

        return Condition.of(self, ConditionalType.NOT_IN, value=values)

    def exists(self) -> "Condition":
        from destack.language import Condition, ConditionalType

        return Condition.of(self, ConditionalType.EXISTS)

    def is_not_none(self) -> "Condition":
        from destack.language import Condition, ConditionalType

        return Condition.of(self, ConditionalType.EXISTS)

    def not_exists(self) -> "Condition":
        from destack.language import Condition, ConditionalType

        return Condition.of(self, ConditionalType.NOT_EXISTS)

    def is_none(self) -> "Condition":
        from destack.language import Condition, ConditionalType

        return Condition.of(self, ConditionalType.NOT_EXISTS)

    def asc(self) -> "Sort":
        from destack.language import Sort, SortType

        return Sort.of(self, SortType.ASCENDING)

    def desc(self) -> "Sort":
        from destack.language import Sort, SortType

        return Sort.of(self, SortType.DESCENDING)


def parse_type_annotation(
    py_type: type | str | typing.ForwardRef, *, is_builtin_member: bool = False
) -> TypeDeclaration:
    """
    Parses the TypeDeclaration from a given py type.
    NOTE: builtin members have some additional limitations (no unions, flat types, etc.).
    """
    is_required: bool = True
    scalar_type: ScalarType | None = None
    primitive_type: PrimitiveType | None = None
    enum_type: EnumType | None = None
    struct_type: StructType | None = None
    node_types: list[NodeType] | None = None

    # try to resolve
    if not isinstance(py_type, type):
        if isinstance(py_type, typing.ForwardRef):
            py_type = py_type.__forward_arg__
        if isinstance(py_type, str):
            if py_type.endswith(" | None"):
                is_required = False
                py_type = py_type[:-7]

    origin = typing.get_origin(py_type)

    # handle Literal
    if origin is typing.Literal:
        literal_values = typing.get_args(py_type)
        if len(literal_values) == 1:
            # single literal value
            return TypeDeclaration(
                cardinality=TypeCardinality.SCALAR,
                scalar_type=ScalarType.LITERAL,
                literal_value=literal_values[0],
                is_required=is_required,
            )
        else:
            # multi-value Literal (union of literals)
            return TypeDeclaration(
                cardinality=TypeCardinality.SCALAR,
                scalar_type=ScalarType.UNION,
                union_types=[
                    TypeDeclaration(
                        cardinality=TypeCardinality.SCALAR,
                        scalar_type=ScalarType.LITERAL,
                        literal_value=value,
                        is_required=is_required,
                    )
                    for value in literal_values
                ],
            )

    # unwrap list
    if origin is list:
        element_type_arg = typing.get_args(py_type)[0]
        element_annotation = parse_type_annotation(
            element_type_arg, is_builtin_member=is_builtin_member
        )
        return TypeDeclaration(
            cardinality=TypeCardinality.LIST,
            value_type=element_annotation,
            is_required=is_required,
        )

    # unwrap tuple
    if origin is tuple:
        type_args = typing.get_args(py_type)
        if not type_args:
            raise ValueError(f"cannot infer type of empty tuple annotation: {py_type!r}")
        # check for homogeneous tuple notation: tuple[int, ...]
        if len(type_args) == 2 and type_args[1] is ...:
            raise ValueError(f"cannot infer type of tuple: {py_type!r}")
        # heterogeneous tuple
        element_types = [
            parse_type_annotation(arg, is_builtin_member=is_builtin_member) for arg in type_args
        ]
        return TypeDeclaration(
            cardinality=TypeCardinality.TUPLE,
            element_types=element_types,
            is_required=is_required,
        )

    # unwrap union/optional
    if origin in (typing.Union, types.UnionType):
        union_args = typing.get_args(py_type)
        is_required = not any(t is type(None) for t in union_args)
        non_none_types = tuple(t for t in union_args if t is not type(None))
        assert len(non_none_types) > 0, f"empty union: {py_type!r}"

        if len(non_none_types) == 1:
            # it's just an optional of that type
            result = parse_type_annotation(non_none_types[0], is_builtin_member=is_builtin_member)
            result.is_required = is_required
            return result
        else:
            # check if all types are Node types
            all_node_types: list[NodeType] = []
            all_are_nodes = True
            for union_type in non_none_types:
                union_class_name = get_class_name(union_type)
                if union_class_name and (node_t := resolve_node_types(union_class_name)):
                    all_node_types.extend(node_t)
                else:
                    all_are_nodes = False
                    break

            if all_are_nodes and all_node_types:
                # union of node types
                return TypeDeclaration(
                    cardinality=TypeCardinality.SCALAR,
                    scalar_type=ScalarType.NODE_REFERENCE,
                    node_types=tuple(all_node_types),
                    is_required=is_required,
                )
            else:
                # general union
                assert not is_builtin_member, f"builtin Types don't support unions: {py_type!r}"
                union_types = [
                    parse_type_annotation(t, is_builtin_member=is_builtin_member)
                    for t in non_none_types
                ]
                return TypeDeclaration(
                    cardinality=TypeCardinality.SCALAR,
                    scalar_type=ScalarType.UNION,
                    union_types=union_types,
                    is_required=is_required,
                )

    # unwrap map (dict)
    if origin is dict:
        key_type_arg, value_type_arg = typing.get_args(py_type)
        key_annotation = parse_type_annotation(key_type_arg, is_builtin_member=is_builtin_member)
        value_annotation = parse_type_annotation(
            value_type_arg, is_builtin_member=is_builtin_member
        )
        if is_builtin_member:
            assert key_annotation.cardinality == TypeCardinality.SCALAR, (
                f"builtin Types don't support map keys: {py_type!r}"
            )
            assert value_annotation.cardinality == TypeCardinality.SCALAR, (
                f"builtin Types don't support map values: {py_type!r}"
            )
        return TypeDeclaration(
            cardinality=TypeCardinality.MAP,
            key_type=key_annotation,
            value_type=value_annotation,
            is_required=is_required,
        )

    # determine scalar type
    is_self = False
    class_name = get_class_name(py_type)
    if isinstance(py_type, (type, TypeAliasType)) and (
        primitive_t := PRIMITIVE_TYPE_BY_ANNOTATION.get(py_type)
    ):
        if is_builtin_member and py_type in (float, int):
            # shouldn't use float/int directly, use a specific precision/size
            raise ValueError(f"unspecific primitive type: {py_type!r}")
        scalar_type = ScalarType.PRIMITIVE
        primitive_type = primitive_t
    elif class_name == "Self":
        scalar_type = ScalarType.NODE_REFERENCE
        is_self = True
    elif class_name and (enum_t := resolve_enum_type(class_name)):
        scalar_type = ScalarType.ENUM
        enum_type = enum_t
    elif class_name and (struct_t := resolve_struct_type(class_name)):
        scalar_type = ScalarType.STRUCT
        struct_type = struct_t
    elif class_name and (node_t := resolve_node_types(class_name)):
        scalar_type = ScalarType.NODE_REFERENCE
        node_types = list(node_t)
    assert scalar_type is not None, f"undetermined scalar type: {py_type!r}"

    # default: scalar
    return TypeDeclaration(
        cardinality=TypeCardinality.SCALAR,
        scalar_type=scalar_type,
        primitive_type=primitive_type,
        enum_type=enum_type,
        struct_type=struct_type,
        node_types=node_types,
        is_self=is_self,
        is_required=is_required,
    )


def get_class_name(
    py_type: type | typing.ForwardRef | typing.TypeAliasType | typing._SpecialForm | str,
) -> str | None:
    if isinstance(py_type, str):
        return py_type
    elif isinstance(py_type, type):  # noqa: SIM114
        return py_type.__name__
    elif isinstance(py_type, typing.TypeAliasType):
        return py_type.__name__
    elif isinstance(py_type, typing.ForwardRef):
        return py_type.__forward_arg__
    elif isinstance(py_type, typing._SpecialForm):
        return getattr(py_type, "__name__", None)
    else:
        raise ValueError(f"unexpected type: {py_type!r}")


def builtin_property(
    id: int,
    *,
    description: str | None = None,
    default: Any = UNSET,
    default_factory: ValueFactory | None = None,
    primitive_type: PrimitiveType | None = UNSET,
    constraint: "TypeConstraint | None" = None,
    edge_type: EdgeType | None = None,
    cascade: CascadeAction | None = None,
    is_internal: bool = False,
    is_repr: bool = False,
    is_hash: bool = True,
    is_eq: bool = True,
    is_identity: bool = False,
    is_unique: bool = False,
    is_readonly: bool = False,
    is_main: bool = False,
    tags: tuple[str, ...] = (),
) -> Any:
    assert id < 256, f"id must be less than 256: {id}"  # for :Encoding
    return PropertyDeclaration(
        id=id,
        description=description,
        default_value=default,
        default_factory=default_factory,
        primitive_type=primitive_type,
        constraint=constraint,
        edge_type=edge_type,
        cascade=cascade,
        is_wired=True,
        is_stored=True,
        is_internal=is_internal,
        is_repr=is_repr,
        is_hash=is_hash,
        is_eq=is_eq,
        is_identity=is_identity,
        is_unique=is_unique,
        is_readonly=is_readonly,
        is_main=is_main,
        tags=tags,
    )


def builtin_property_parent(*, is_readonly: bool = False, description: str | None = None) -> Any:
    """The parent of a node, must be of one of the given types."""
    return PropertyDeclaration(
        id=3,  # NOTE: never change this id! :Encoding
        edge_type=EdgeType.PARENT,
        default_value=None,
        is_wired=True,
        is_stored=True,
        is_required=False,
        is_internal=True,
        is_eq=False,
        is_readonly=is_readonly,
        cascade=CascadeAction.CASCADE,
        description=description,
    )


def builtin_property_runtime(*, default: Any = UNSET) -> Any:
    """A property that is only used at runtime."""
    return PropertyDeclaration(
        id=None,
        is_internal=True,
        is_wired=False,
        is_stored=False,
        is_repr=False,
        is_hash=False,
        is_eq=False,
        default_value=default,
    )


_PROPERTY_SPECIFIERS: tuple[Callable, ...] = (
    builtin_property,
    builtin_property_parent,
    builtin_property_runtime,
)
