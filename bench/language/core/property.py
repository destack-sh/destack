import dataclasses
import types
import typing
from dataclasses import dataclass
from typing import (
    TYPE_CHECKING,
    Any,
    Callable,
    Literal,
    Mapping,
    Optional,
    assert_never,
)

from bench.language.registry import _on_completing_setup
from bench.utils.func import hash_stable
from bench.utils.string import Casing, to_casing

from .const import (
    EMPTY_DICT,
    NODE_TYPES,
    PRIMITIVE_TYPE_BY_PY_TYPE,
    UNSET,
    CascadeAction,
    EdgeType,
    EnumType,
    NodeType,
    PrimitiveType,
    StructType,
    TraitType,
)

if TYPE_CHECKING:
    from bench.language import (
        BuiltinObject,
        Constraint,
        Format,
        PropertyReference,
        Type,
    )

    from .query import IntoQuery


def _resolve_enum_type(class_name: str) -> EnumType | None:
    """Get the EnumType for the given enum name."""
    enum_name = to_casing(class_name, Casing.ALL_CAPS)
    if enum_type := EnumType.__members__.get(enum_name):
        return enum_type
    enum_name = class_name.upper()
    if enum_type := EnumType.__members__.get(enum_name):
        return enum_type
    return None


def _resolve_struct_type(class_name: str) -> StructType | None:
    """Get the StructType for the given struct name."""
    enum_name = to_casing(class_name, Casing.ALL_CAPS)
    if struct_type := StructType.__members__.get(enum_name):
        return struct_type
    enum_name = class_name.upper()
    if struct_type := StructType.__members__.get(enum_name):
        return struct_type
    return None


def _resolve_trait_type(name: str) -> TraitType | None:
    """Get a trait by name."""
    if name.startswith("Is"):
        name = name[2:]
    elif name.startswith("Has"):
        name = name[3:]
    name = to_casing(name, Casing.ALL_CAPS)
    trait = TraitType.__members__.get(name)
    return trait


def _resolve_node_types(class_name: str) -> tuple[NodeType | TraitType, ...] | None:
    """Get the NodeType for the given node name."""
    enum_name = to_casing(class_name, Casing.ALL_CAPS)
    if node_type := NodeType.__members__.get(enum_name):
        return (node_type,)
    if trait := _resolve_trait_type(class_name):
        return (trait,)
    if node_type := NodeType.__members__.get(class_name.upper()):
        return (node_type,)
    if class_name == "Node":
        return NODE_TYPES.tuple
    return None


def _try_resolve(
    py_type: type | str | typing.ForwardRef, type_map: Mapping[str, type]
) -> type | str:
    """Resolves the py type."""
    if isinstance(py_type, str):
        return type_map.get(py_type, py_type)
    elif isinstance(py_type, typing.ForwardRef):
        return type_map.get(py_type.__forward_arg__, py_type.__forward_arg__)
    else:
        return py_type


@dataclass(slots=True)
class IntoType:
    """Type annotation to be turned into a Property/Type/Field."""

    py_type: Any = None
    cardinality: Literal["scalar", "list", "map"] = UNSET
    scalar_type: Literal["primitive", "enum", "struct", "node"] = UNSET
    primitive_type: PrimitiveType | None = None
    enum_type: EnumType | None = None
    struct_type: StructType | None = None
    node_types: tuple[NodeType | TraitType, ...] = ()  # for node scalar nodes
    key_type: "IntoType | None" = None
    is_required: bool = True
    is_variable: bool = False

    default: Any = UNSET
    default_factory: Literal["uuid", "now"] | None = None
    format: "Format | None" = None
    constraint: "Constraint | None" = None

    def _to_type(self) -> "Type":
        """Create the Type for this Property."""
        from .type import (
            CollectionConstraint,
            DefaultFactory,
            NodeConstraint,
            NumberConstraint,
            NumberFormat,
            ScalarType,
            StringConstraint,
            StringFormat,
            Type,
            TypeCardinality,
        )

        # type
        type_cardinality = TypeCardinality[self.cardinality.upper()]
        scalar_type = ScalarType[self.scalar_type.upper()]
        default = self.default if self.default is not UNSET else None
        default_factory = (
            DefaultFactory[self.default_factory.upper()] if self.default_factory else None
        )
        type_obj = Type(
            cardinality=type_cardinality,
            scalar_type=scalar_type,
            primitive_type=self.primitive_type,
            enum_type=self.enum_type,
            node_type=None,
            struct_type=self.struct_type,
            is_required=self.is_required,
            is_variable=self.is_variable,
            default=default,
            default_factory=default_factory,
            key_type=self.key_type._to_type() if self.key_type else None,
        )

        # constraints
        if self.constraint is not None:
            if isinstance(self.constraint, StringConstraint):
                type_obj.string_constraint = self.constraint
            elif isinstance(self.constraint, NumberConstraint):
                type_obj.number_constraint = self.constraint
            elif isinstance(self.constraint, CollectionConstraint):
                type_obj.collection_constraint = self.constraint
            elif isinstance(self.constraint, NodeConstraint):
                type_obj.node_constraint = self.constraint
            else:
                assert_never(self.constraint)

        # format
        if self.format is not None:
            if isinstance(self.format, StringFormat):
                if not isinstance(self.constraint, StringConstraint):
                    self.constraint = StringConstraint()
                self.constraint = self.constraint.replace(format=self.format)
            elif isinstance(self.format, NumberFormat):
                if not isinstance(self.constraint, NumberConstraint):
                    self.constraint = NumberConstraint()
                self.constraint = self.constraint.replace(format=self.format)
            else:
                assert_never(self.format)

        return type_obj


def parse_type_annotation(
    py_type: type | str | typing.ForwardRef, type_map: Mapping[str, type] = EMPTY_DICT
) -> IntoType:
    """Parses the type information from a given py type. Uses type map to resolve forward refs."""
    is_required = True
    is_variable = False
    primitive_type = None
    enum_type = None
    struct_type = None
    scalar_type = None

    # unwrap VariableProperty[...]
    if (
        isinstance(origin_cls := typing.get_origin(py_type), typing.TypeAliasType)
        and origin_cls.__name__ == "VariableProperty"
    ):
        is_variable = True
        is_required = False  # variable Properties are automatically optional
        py_type = typing.get_args(py_type)[0]

    # try to resolve
    if not isinstance(py_type, type):
        if isinstance(py_type, typing.ForwardRef):
            py_type = py_type.__forward_arg__
        if isinstance(py_type, str):
            if py_type.endswith(" | None"):
                is_required = False
                py_type = py_type[:-7]
        py_type = _try_resolve(py_type, type_map)

    # unwrap list
    if typing.get_origin(py_type) in (list, tuple):
        element_annotation = parse_type_annotation(typing.get_args(py_type)[0], type_map)
        assert not element_annotation.is_variable, f"variable element: {py_type!r}"
        assert element_annotation.cardinality == "scalar", f"non-scalar element: {py_type!r}"
        return IntoType(
            cardinality="list",
            py_type=py_type,
            scalar_type=element_annotation.scalar_type,
            primitive_type=element_annotation.primitive_type,
            enum_type=element_annotation.enum_type,
            struct_type=element_annotation.struct_type,
            node_types=element_annotation.node_types,
            is_required=is_required,
            is_variable=is_variable,
        )

    # unwrap union/optional
    if typing.get_origin(py_type) in (typing.Union, types.UnionType):
        union_args = typing.get_args(py_type)
        is_required = not any(t is type(None) for t in union_args)
        non_none_types = tuple(t for t in union_args if t is not type(None))
        assert len(non_none_types) > 0, f"empty union: {py_type!r}"

        if len(non_none_types) == 1:
            # it's just an optional of that type
            result = parse_type_annotation(non_none_types[0], type_map)
            result.is_required = is_required
            return result
        else:
            # only node unions are supported for now (no real unions)
            node_types = []
            for union_type in non_none_types:
                union_class_name = get_class_name(union_type)
                if union_class_name and (new_node_types := _resolve_node_types(union_class_name)):
                    node_types.extend(new_node_types)
            assert node_types, f"non-node union: {py_type!r}"
            return IntoType(
                cardinality="scalar",
                py_type=py_type,
                scalar_type="node",
                node_types=tuple(node_types),
                is_required=is_required,
                is_variable=is_variable,
            )

    # unwrap map (dict)
    if typing.get_origin(py_type) is dict:
        key_type_arg, value_type_arg = typing.get_args(py_type)
        key_annotation = parse_type_annotation(key_type_arg, type_map)
        assert not key_annotation.is_variable, f"variable key: {py_type!r}"
        assert key_annotation.cardinality == "scalar", f"non-scalar key: {py_type!r}"
        value_annotation = parse_type_annotation(value_type_arg, type_map)
        assert not value_annotation.is_variable, f"variable value: {py_type!r}"
        assert value_annotation.cardinality == "scalar", f"non-scalar value: {py_type!r}"
        return IntoType(
            cardinality="map",
            py_type=py_type,
            key_type=key_annotation,
            scalar_type=value_annotation.scalar_type,
            primitive_type=value_annotation.primitive_type,
            enum_type=value_annotation.enum_type,
            struct_type=value_annotation.struct_type,
            node_types=value_annotation.node_types,
            is_required=is_required,
            is_variable=is_variable,
        )

    # determine scalar type
    class_name = get_class_name(py_type)
    if isinstance(py_type, type) and (primitive_t := PRIMITIVE_TYPE_BY_PY_TYPE.get(py_type)):
        scalar_type = "primitive"
        primitive_type = primitive_t
    elif class_name == "Json":
        scalar_type = "primitive"
        primitive_type = PrimitiveType.JSON
    elif class_name and (enum_t := _resolve_enum_type(class_name)):
        scalar_type = "enum"
        enum_type = enum_t
    elif class_name and (struct_t := _resolve_struct_type(class_name)):
        scalar_type = "struct"
        struct_type = struct_t
    elif class_name and (new_node_types := _resolve_node_types(class_name)):
        scalar_type = "node"
        node_types = new_node_types
    elif class_name == "Property":
        scalar_type = "struct"
        struct_type = StructType.PROPERTY_REFERENCE
    assert scalar_type is not None, f"undetermined scalar type: {py_type!r}"

    # default: scalar
    return IntoType(
        cardinality="scalar",
        py_type=py_type,
        scalar_type=scalar_type,
        primitive_type=primitive_type,
        enum_type=enum_type,
        struct_type=struct_type,
        is_required=is_required,
        is_variable=is_variable,
    )


def get_class_name(py_type: type | typing.ForwardRef | str) -> str | None:
    if isinstance(py_type, str):
        return py_type
    elif isinstance(py_type, type):  # noqa: SIM114
        return py_type.__name__
    elif isinstance(py_type, typing.TypeAliasType):
        return py_type.__name__
    elif isinstance(py_type, typing.ForwardRef):
        return py_type.__forward_arg__
    else:
        return None


@dataclass(eq=False, slots=True)
class Property(IntoType, IntoQuery if TYPE_CHECKING else object):
    """A system-defined attribute of a BuiltinObject (Struct or Node)."""

    # meta
    id: int | None = None
    ord: int | None = None
    name: str = UNSET  # name from LHS of assignment
    description: str | None = None
    component: type["BuiltinObject"] = UNSET  # builtin object component

    # pointers
    ptr_prop: Optional["Property"] = None  # wired representation for pointers
    runtime_prop: Optional["Property"] = None  # for the proto property
    edge_type: EdgeType | None = None
    node_bench_from: Literal["self"] | None = None
    node_exclude: tuple[Literal["ck", "definition_id"], ...] = ()
    cascade: CascadeAction | None = None

    is_wired: bool = False  # serialized onto wire (in proto)
    is_stored: bool = False  # stored in DB
    is_unique: bool = False  # unique index in DB

    is_repr: bool = False  # printed BuiltinObject.__repr__
    is_hash: bool = True  # included BuiltinObject.__hash__
    is_eq: bool = True  # included BuiltinObject.equals check
    is_managed: bool = False  # set automatically by the system
    is_computed: bool = False  # set automatically at runtime
    can_read: Literal["any", "owner", "system"] = "any"
    can_write: Literal["any", "owner", "system"] = "any"

    _ref: Optional["PropertyReference"] = None
    _type: Optional["Type"] = None

    def __str__(self):
        if self.component is None:
            return "<detached>"
        return f"{self.component.__name__}.{self.name}"

    def __repr__(self):
        return f"<{self.__class__.__name__} {self!s} ({self.id or '<unset>'})>"

    # see IntoQuery.__eq__ for Property==Property equality

    def _stable_hash(self):
        """Hash the Property identity."""
        return hash_stable((self.component.__name__, self.id))

    __hash__ = _stable_hash  # type: ignore

    def clone(self):
        return dataclasses.replace(self, component=None, runtime_prop=None, ptr_prop=None)

    def to_ref(self) -> "PropertyReference":
        """A pointer to this property. `to_ref()` for consistency with `Node.to_ref()`."""

        if self._ref is None:
            from .struct import PropertyReference

            assert self.component is not None, f"{self!r} has no component"
            assert self.id is not None, f"{self!r} has no id"

            if self.component.__is_node__:
                ref = PropertyReference(
                    node_type=getattr(self.component, "metatype", None), id=self.id
                )
            else:
                ref = PropertyReference(
                    struct_type=getattr(self.component, "metatype", None), id=self.id
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
    def is_node_reference(self):
        return self.scalar_type == "node"

    @property
    def is_struct(self) -> bool:
        return self.scalar_type == "struct"

    @property
    def is_enum(self):
        return self.scalar_type == "enum"

    @property
    def is_property(self) -> bool:
        return self.struct_type == StructType.PROPERTY_REFERENCE

    @property
    def is_optional(self) -> bool:
        return not self.is_required

    @property
    def type(self) -> "Type":
        """The type info for this property (can't extend TypeInfo because circles)."""
        if self._type is None:
            self._type = self._to_type()
            assert self._type is not None, f"{self!r} has no type"
        return self._type

    def _to_ptr_prop(self) -> Optional["Property"]:
        """
        Contribute the wired and stored pointer properties required by this property.
        """

        # property reference
        if self.is_property:
            assert self.cardinality in ("scalar", "list"), f"invalid property reference: {self!r}"
            assert self.is_required is not UNSET, f"must set is_required on {self!r}"
            ptr_prop = Property(
                id=self.id,
                name=self.name + "_ptr",
                component=self.component,
                primitive_type=PrimitiveType.JSON,
                scalar_type="struct",
                struct_type=StructType.PROPERTY_REFERENCE,
                cardinality=self.cardinality,
                is_wired=True,
                is_stored=True,
                is_required=self.is_required,
                is_variable=self.is_variable,
                is_eq=self.is_eq,
                default=None,
                runtime_prop=self,
            )
            return ptr_prop

        # node reference
        elif self.edge_type:
            if self.edge_type == EdgeType.NODE_PARENT:
                is_computed = False
            elif self.edge_type == EdgeType.NODE_ANCESTOR:
                is_computed = True
            elif self.edge_type in (
                EdgeType.NODE_REGULAR,
                EdgeType.NODE_TEMPLATE,
            ):
                is_computed = False
            else:
                raise ValueError(f"unexpected reference kind {self.edge_type!r} for {self!r}")
            ptr_prop = Property(
                id=self.id,
                name=self.name + "_ptr",
                component=self.component,
                edge_type=self.edge_type,
                node_types=self.node_types,
                runtime_prop=self,
                scalar_type="node",
                struct_type=StructType.NODE_REFERENCE,
                primitive_type=PrimitiveType.JSON,
                is_wired=True,
                is_stored=True,
                is_eq=self.is_eq,
                cardinality=self.cardinality,
                is_computed=is_computed,
                is_required=self.is_required,
                is_variable=self.is_variable,
                default=None,
                constraint=self.constraint,
            )
            if self.edge_type == EdgeType.NODE_ANCESTOR:
                # wired ancestors are not required (even though stored ancestors are)
                ptr_prop.is_required = False

            return ptr_prop

    def finalize(self, object_type: NodeType | StructType | None) -> None:
        """Determine type information from annotation, add _ptr property if needed."""
        if not self.is_wired:
            return  # runtime only, nothing to do

        # parse annotation
        try:
            annotation = parse_type_annotation(self.py_type, EMPTY_DICT)
        except Exception as e:
            raise ValueError(
                f"invalid type: {self.component.__name__}.{self.name} ({self.py_type})"
            ) from e
        self.cardinality = annotation.cardinality
        self.scalar_type = annotation.scalar_type
        self.primitive_type = annotation.primitive_type
        self.enum_type = annotation.enum_type
        self.struct_type = annotation.struct_type
        self.node_types = annotation.node_types
        self.key_type = annotation.key_type
        self.is_required = annotation.is_required
        self.is_variable = annotation.is_variable

        # only scalar types can be optional
        if not self.is_required and self.cardinality != "scalar":
            raise ValueError(f"non-scalar {self!r} cannot be optional")
        # parent must be optional
        if self.name == "parent" and self.is_required:
            raise ValueError(f"parent must be optional: {self!r}")
        # 'type' must be 30
        if (self.name == "type") != (self.id == 30):
            raise ValueError(f"'type' must be 30: {self!r}")
        # variables must be scalar on a Node
        if self.is_variable and (self.cardinality != "scalar" or not self.component.__is_node__):
            raise ValueError(f"variable must be scalars on a Node: {self!r}")

        # default to None if not required and no default
        if not self.is_required and self.default is UNSET:
            self.default = None
        # default to regular node references
        if self.scalar_type == "node" and self.edge_type is None:
            self.edge_type = EdgeType.NODE_REGULAR

        # node templates always point to their own type
        if self.edge_type == EdgeType.NODE_TEMPLATE and object_type is not None:
            self.node_types = (NodeType(object_type),)
        # references get a _ptr property (which is wired/stored)
        if self.edge_type is not None or self.is_property:
            # (don't want lists of Node references or Property references in Nodes, it's a mess)
            assert (
                self.cardinality == "scalar" or not self.component.__is_node__
            ), f"invalid list: {self!r}"
            self.ptr_prop = self._to_ptr_prop()
            return  # bail, no need to determine primitive type

        # determine primitive type
        if self.primitive_type is None:
            if self.cardinality == "map":
                self.primitive_type = PrimitiveType.JSON
            elif self.scalar_type == "enum":
                assert self.enum_type is not None
                if self.enum_type.get_max_ord() < 2**16:
                    self.primitive_type = PrimitiveType.INT16
                else:
                    self.primitive_type = PrimitiveType.INT32
            elif self.scalar_type == "struct":
                assert self.struct_type is not None
                self.primitive_type = PrimitiveType.JSON
        assert (
            self.primitive_type is not None
        ), f"undetermined type {self.py_type!r} for {self!r} ({annotation!r})"


@_on_completing_setup
def _add_property_into_query():
    from .query import IntoQuery

    for name, attr in IntoQuery.__dict__.items():
        if name not in Property.__dict__ and name not in ("__annotations__", "__dict__"):
            setattr(Property, name, attr)


def property_(
    id: int | None = None,
    *,
    description: str | None = None,
    default: Any = UNSET,
    default_factory: Literal["uuid", "now"] | None = None,
    primitive_type: PrimitiveType | None = UNSET,
    format: "Format | None" = None,
    constraint: "Constraint | None" = None,
    node_bench_from: Literal["self"] | None = None,
    node_exclude: tuple[Literal["ck", "definition_id"], ...] = (),
    node_kind: EdgeType | None = None,
    cascade: CascadeAction | None = None,
    is_managed: bool = False,
    is_unique: bool = False,
    is_repr: bool = False,
    is_hash: bool = True,
    is_eq: bool = True,
    can_read: Literal["any", "owner", "system"] = "any",
    can_write: Literal["any", "owner", "system"] = "any",
) -> Any:
    return Property(
        id=id,
        description=description,
        default=default,
        default_factory=default_factory,
        primitive_type=primitive_type,
        format=format,
        constraint=constraint,
        node_bench_from=node_bench_from,
        node_exclude=node_exclude,
        edge_type=node_kind,
        cascade=cascade,
        is_wired=True,
        is_stored=True,
        is_unique=is_unique,
        is_managed=is_managed,
        is_repr=is_repr,
        is_hash=is_hash,
        is_eq=is_eq,
        can_read=can_read,
        can_write=can_write,
    )


def property_parent_(id: int = 4) -> Any:
    """The parent of a node, must be of one of the given types."""
    return Property(
        id=id,
        edge_type=EdgeType.NODE_PARENT,
        default=None,
        is_wired=True,
        is_stored=False,
        is_required=False,
        is_eq=False,
        node_bench_from="self",
        node_exclude=("ck", "definition_id"),
    )


def property_ancestor_(
    id: int,
    is_required: bool,
) -> Any:
    """Computed nearest or farthest ancestor of the given type."""
    return Property(
        id=id,
        edge_type=EdgeType.NODE_ANCESTOR,
        is_computed=True,
        is_required=is_required,
        is_wired=True,
        is_stored=True,
        is_eq=False,  # no point since it's derived
        node_bench_from="self",
    )


def property_runtime_(*, default: Any = UNSET) -> Any:
    """A property that is only used at runtime."""
    return Property(
        id=None,
        is_managed=True,
        is_wired=False,
        is_stored=False,
        is_repr=False,
        is_hash=False,
        is_eq=False,
        default=default,
    )


_PROPERTY_SPECIFIERS: tuple[Callable, ...] = (
    property_,
    property_parent_,
    property_ancestor_,
    property_runtime_,
)
