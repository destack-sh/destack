import dataclasses
import enum
import functools
from dataclasses import dataclass
from sys import intern
from typing import (
    TYPE_CHECKING,
    Any,
    Callable,
    Optional,
    cast,
)

from bench.language.registry import ENUM_TYPE_BY_CLASS, _on_completing_setup
from bench.utils.func import hash_stable, parse_py_annotation

from .const import (
    EMPTY_DICT,
    PRIMITIVE_TYPE_BY_PY_TYPE,
    UNSET,
    BuiltinEnum,
    EnumType,
    NodeReferenceKind,
    NodeType,
    ObjectType,
    PrimitiveType,
    StructType,
    TypeKind,
)

if TYPE_CHECKING:
    from bench.language import (
        BuiltinObject,
        NodeReference,
        PropertyReference,
        Type,
        TypeConstraint,
        TypeConstraintIn,
    )

    from .expression import _IntoQuery


class NodeReferenceMeta(enum.IntEnum):
    ID = 1
    CK = 2
    BENCH_ID = 3
    BASE_ID = 4
    NODE_TYPE = 6


PROPERTY_META_KEY_BY_TYPE = {
    NodeReferenceMeta.ID: "id",
    NodeReferenceMeta.CK: "ck",
    NodeReferenceMeta.BENCH_ID: "bench_id",
    NodeReferenceMeta.BASE_ID: "base_id",
    NodeReferenceMeta.NODE_TYPE: "node_type",
}
PROPERTY_TYPE_BY_META_KEY = {v: k for k, v in PROPERTY_META_KEY_BY_TYPE.items()}


@dataclass(eq=False, slots=True)
class Property(_IntoQuery if TYPE_CHECKING else object):
    """A system-defined attribute of a BuiltinObject (Struct or Node)."""

    # NOTE: yes cast(int, None) is a bit evil but we almost always immediately assign it here and
    #  don't want to deal with asserting id is not None everywhere.
    # stable id for wiring properties, must be unique per final struct/node
    id: int = cast(int, None)  # noqa: RUF009
    key: str = UNSET  # str(id)
    ord: int = cast(int, None)  # noqa: RUF009
    name: str = UNSET  # name from LHS of assignment
    component: type["BuiltinObject"] = UNSET  # source component class
    py_type_raw: Any = None  # type annotation on LHS of assignment
    py_type: Any = UNSET  # clean type annotation
    primitive_type: PrimitiveType | None = UNSET
    enum_type: EnumType | None = None
    struct_type: StructType | None = None
    is_node_data: bool = False  # special case

    ptr_prop: Optional["Property"] = None  # wired representation for pointers
    runtime_prop: Optional["Property"] = None  # for the proto property
    node_types: tuple[NodeType, ...] | None = None  # for node relations
    node_kind: NodeReferenceKind | None = None
    node_same_bench: bool = False
    node_ckless: bool = False
    node_basless: bool = False

    default: Any = UNSET
    default_factory: Callable[[], Any] = UNSET
    constraint: "TypeConstraint | TypeConstraintIn | None" = None

    is_list: bool = UNSET
    is_required: bool = UNSET  # must be non-null
    is_variable: bool = UNSET  # may be wrapped in an indirect Variable lookup
    is_runtime: bool = UNSET  # exists on runtime object
    is_proto: bool = UNSET  # serialized onto wire (in proto)
    is_stored: bool = UNSET  # stored in DB
    is_internal: bool = False  # should be edited via accessors, but not enforced
    is_autoset: bool = False  # set automatically by system, cannot set directly
    is_computed: bool = False
    is_ephemeral: bool = False  # runtime-only in-memory property
    is_unique: bool = False  # unique index in DB?
    is_sensitive: bool = False  # sensitive data (generally requires special permissions)
    is_untracked: bool = False  # whether writes are tracked

    _ref: Optional["PropertyReference"] = None
    _type: Optional["Type"] = None

    def __post_init__(self):
        if self.default is UNSET:
            self.default = None
        if self.id is not None:
            self.key = intern(str(self.id))

    def __str__(self):
        if self.component is None:
            return "<detached>"
        return f"{self.component.__name__}.{self.name}"

    def __repr__(self):
        non_default = []
        if self.id is not None:
            non_default.append(str(self.id))
        if self.node_kind:
            non_default.append(self.node_kind.bench_name)
            if self.node_types:
                if self.node_types == "any":
                    non_default.append("*")
                else:
                    non_default.append("|".join(t.bench_name for t in self.node_types))
            elif self.struct_type:
                non_default.append(self.struct_type.bench_name)
        elif self.enum_type:
            non_default.append(self.enum_type.bench_name)
        elif self.primitive_type and self.primitive_type is not UNSET:
            non_default.append(self.primitive_type.bench_name)
        if self.is_list:
            non_default.append("list")
        if self.is_required:
            non_default.append("required")
        attrs_str = ", ".join(non_default)
        attrs_str = f" ({attrs_str})" if attrs_str else ""
        return f"<{self.__class__.__name__} {self!s}{attrs_str}>"

    # see IntoQuery.__eq__ for Property==Property equality

    def _stable_hash(self):
        """Hash the Property identity."""
        return hash_stable((self.component.__name__, self.id))

    __hash__ = _stable_hash  # type: ignore

    def clone(self):
        return dataclasses.replace(self, component=None, ptr_prop=None)

    def to_ref(self) -> "PropertyReference":
        """A pointer to this property. `to_ref()` for consistency with `Node.to_ref()`."""

        if self._ref is None:
            assert self.component is not None, f"{self!r} is not finalized"
            from .object import PropertyReference

            ref = PropertyReference(
                object_type=getattr(self.component, "metatype", None), id=self.id
            )
            self._ref = ref
        return self._ref

    @property
    def code_name(self) -> str:
        return self.name

    @property
    def type(self) -> Optional[ObjectType]:
        return self.component.metatype

    @property
    def has_id(self) -> int:
        return self.id is not None and self.id is not UNSET

    @property
    def is_tree_reference(self) -> bool:
        """Whether this is a tree relation property (parent/child/ancestor)."""
        return self.node_kind is not None and self.node_kind.is_node_tree

    @property
    def is_node_reference(self):
        return self.node_kind is not None and self.node_kind.is_node

    @property
    def is_struct_reference(self):
        """Whether this is a reference to a parent struct/value. *Not* an inlined Struct."""
        return self.node_kind is not None and self.node_kind.is_struct_tree

    @property
    def is_struct(self) -> bool:
        return self.struct_type is not None

    @property
    def is_property_reference(self) -> bool:
        return self.struct_type == StructType.PROPERTY_REFERENCE

    @property
    def is_optional(self) -> bool:
        return not self.is_required

    @property
    def is_optional_scalar(self) -> bool:
        return self.is_optional and not self.is_list

    @property
    def is_enum(self):
        return self.enum_type is not None

    def _to_type(self) -> "Type":
        from bench.language.core import Type, TypeConstraintIn

        if isinstance(self.constraint, TypeConstraintIn):
            constraint = self.constraint.into()
        else:
            constraint = self.constraint or TypeConstraintIn()

        if self.node_types and not self.runtime_prop:
            kind = TypeKind.NODE
            bench_type = None
            if self.node_types != "any":
                constraint.node_types = list(self.node_types)
            primitive_type = None
        elif self.struct_type:
            kind = TypeKind.STRUCT
            bench_type = self.struct_type
            primitive_type = None
        elif self.enum_type:
            kind = TypeKind.ENUM
            bench_type = self.enum_type
            primitive_type = None
        elif self.primitive_type:
            assert self.primitive_type is not UNSET, f"missing primitive type for {self!r}"
            kind = TypeKind.PRIMITIVE
            bench_type = None
            primitive_type = self.primitive_type
        else:
            raise ValueError(f"cannot determine type info for {self!r}")

        typ = Type(
            kind=kind,
            bench_type=bench_type,
            primitive_type=primitive_type,
            is_list=self.is_list,
            is_required=self.is_required,
            constraint=constraint.into()
            if isinstance(constraint, TypeConstraintIn)
            else constraint,
            _from_property=self,
        )
        return typ

    @property
    def type_info(self) -> "Type":
        """The type info for this property (can't extend TypeInfo because circles)."""
        if self._type is None:
            if (
                self.runtime_prop is not None
                or self.node_types
                or self.is_property_reference
                or self.id == 1
            ):
                self._type = self._to_type()
            assert self._type is not None, f"{self!r} is not finalized"
        return self._type

    def _to_ptr_prop(self) -> Optional["Property"]:
        """
        Contribute the wired and stored pointer properties required by this property.
        """

        # property reference
        if self.is_property_reference:
            assert self.is_list is not UNSET, f"must set is_list on {self!r}"
            assert self.is_required is not UNSET, f"must set is_required on {self!r}"
            ptr_prop = Property(
                id=self.id,
                name=self.name + "_ptr",
                component=self.component,
                struct_type=StructType.PROPERTY_REFERENCE,
                is_runtime=True,
                is_internal=self.is_internal,
                is_proto=True,
                is_stored=True,
                is_required=self.is_required,
                is_list=self.is_list,
                default=None,
                runtime_prop=self,
                constraint=self.constraint,
            )
            return ptr_prop

        # node reference
        elif self.node_kind:
            if self.node_kind == NodeReferenceKind.NODE_PARENT:
                is_required = False  # parent is always optional
                is_list = False
                is_computed = False
                is_internal = True
            elif self.node_kind in (
                NodeReferenceKind.NODE_ANCESTOR_OR_SELF,
                NodeReferenceKind.NODE_ANCESTOR,
            ):
                is_required = self.is_required
                is_list = False
                is_computed = True
                is_internal = True
            elif self.node_kind in (
                NodeReferenceKind.NODE_REGULAR,
                NodeReferenceKind.NODE_TEMPLATE,
            ):
                is_required = self.is_required
                is_list = self.is_list
                is_computed = False
                is_internal = False
            else:
                raise ValueError(f"unexpected reference kind {self.node_kind!r} for {self!r}")

            # wired reference representation is just a nice NodeReference struct
            ptr_prop = Property(
                id=self.id,  # re-use id, self is not stored
                name=self.name + "_ptr",
                component=self.component,
                py_type_raw=list["NodeReference"] if is_list else "NodeReference",
                node_kind=self.node_kind,
                node_types=self.node_types,
                runtime_prop=self,
                struct_type=StructType.NODE_REFERENCE,
                is_runtime=True,
                is_proto=True,
                is_stored=False,
                is_autoset=self.is_autoset,
                is_computed=is_computed,
                is_list=is_list,
                is_required=is_required,
                is_internal=is_internal,
                default=None,
                primitive_type=None,
                constraint=self.constraint,
            )
            if (
                self.node_kind == NodeReferenceKind.NODE_ANCESTOR
                or self.node_kind == NodeReferenceKind.NODE_ANCESTOR_OR_SELF
            ):
                # wired ancestors are not required (even though stored ancestors are)
                ptr_prop.is_required = False

            return ptr_prop

    def _finalize_meta(self) -> None:
        """Analyzes the storage options. Must run after all class defs."""
        # store/wire property by default if not runtime (and not indicated otherwise)
        if self.primitive_type is UNSET and (self.is_tree_reference or self.node_types):
            if self.is_stored is UNSET:
                self.is_stored = False
            self.primitive_type = None
        elif self.is_stored is UNSET:
            self.is_stored = True
        if self.is_proto is UNSET:
            self.is_proto = self.is_stored
        if self.is_runtime is UNSET:
            self.is_runtime = self.is_stored
        if self.ptr_prop:
            self.is_proto = False
            self.is_stored = False

    def _finalize_type(self) -> None:
        """Finalizes the type info for this property."""
        if self.is_ephemeral:
            return  # nothing to do

        # get info from annotation
        annotation = parse_py_annotation(self.py_type_raw, EMPTY_DICT)
        self.py_type = annotation.type
        self.is_required = not annotation.is_optional
        self.is_list = annotation.is_list
        if not self.is_required and self.default is UNSET and self.default_factory is None:
            self.default = None
        if isinstance(annotation.type, type) and issubclass(annotation.type, enum.Enum):
            if not issubclass(annotation.type, BuiltinEnum):
                raise ValueError(f"only BuiltinEnum is supported for enums: {self!r}")
            self.enum_type = ENUM_TYPE_BY_CLASS.get(annotation.type)
            if self.enum_type is None:
                raise ValueError(f"missing enum type for {annotation.type!r} at {self!r}")

        # determine storage type
        if self.primitive_type is UNSET and (self.is_stored or self.is_proto):
            if annotation.is_union:
                raise ValueError(f"cannot store union {self!r}")
            # map to column type
            assert isinstance(annotation.type, type), f"invalid type {annotation!r} for {self!r}"
            if issubclass(annotation.type, BuiltinEnum):
                if max(annotation.type) < 2**16:
                    self.primitive_type = PrimitiveType.INT16
                else:
                    self.primitive_type = PrimitiveType.INT32
            elif getattr(annotation.type, "__is_node__", False):
                raise ValueError(f"cannot store/wire node directly: {self!r}")
            elif getattr(annotation.type, "__is_struct__", False):
                assert self.struct_type is not None, f"missing struct type for {self!r}"
                self.primitive_type = PrimitiveType.JSON  # packed builtin object json
            else:
                primitive_type = PRIMITIVE_TYPE_BY_PY_TYPE.get(annotation.type)
                if primitive_type is None:
                    raise ValueError(f"cannot determine storage for {self!r}: {self.py_type_raw!r}")
                self.primitive_type = primitive_type


@_on_completing_setup
def _add_property_expression_base():
    from .expression import _IntoQuery

    for name, attr in _IntoQuery.__dict__.items():
        if name not in Property.__dict__ and name not in ("__annotations__", "__dict__"):
            setattr(Property, name, attr)


def p_property(
    id: int,
    *,
    internal: bool = False,
    system: bool = False,
    kernel: bool = False,
    autoset: bool = False,
    description: str | None = None,
    default: Any = UNSET,
    default_factory: Callable[[], Any] = UNSET,
    same_bench: bool = False,
    baseless: bool = False,
    ckless: bool = False,
    is_node_data: bool = False,
    store: bool = True,
    wire: bool = True,
    primitive_type: PrimitiveType | None = UNSET,
    unique: bool = False,
    sensitive: bool = False,
    constraint: "TypeConstraint | TypeConstraintIn | None" = None,
) -> Any:
    return Property(
        id=id,
        default=default,
        default_factory=default_factory,
        primitive_type=primitive_type,
        constraint=constraint,
        is_node_data=is_node_data,
        node_same_bench=same_bench,
        node_basless=baseless,
        node_ckless=ckless,
        is_internal=internal,
        is_autoset=autoset,
        is_untracked=autoset,
        is_runtime=True,
        is_proto=wire,
        is_stored=store,
        is_sensitive=sensitive,
        is_unique=unique,
    )


def p_runtime(
    *,
    default: Any = None,
    default_factory: Callable[[], Any] = UNSET,
) -> Any:
    """Internal runtime-only struct/node property (not persisted)."""
    return Property(
        is_internal=True,
        is_runtime=True,
        is_proto=False,
        is_ephemeral=True,
        is_untracked=True,
        is_computed=False,
        is_required=False,
        is_stored=False,
        default=default,
        default_factory=default_factory,
    )


def p_node_parent(
    id: int,
    *node_type: NodeType,
    is_system: bool = False,
    ckless: bool = True,
    baseless: bool = True,
) -> Any:
    """The parent of a node, must be of one of the given types."""
    return Property(
        id=id,
        node_kind=NodeReferenceKind.NODE_PARENT,
        node_types=tuple(node_type),
        default=None,
        is_internal=True,
        is_stored=False,
        is_list=False,
        is_runtime=True,
        is_untracked=True,
        is_required=False,
        node_ckless=ckless,
        node_basless=baseless,
    )


def _p_node_ancestor(
    id: int,
    node_type: NodeType,
    kind: NodeReferenceKind,
    store: bool = False,
    wire: bool = False,
    require: bool = UNSET,
    is_bench_implicit: bool = False,
) -> Any:
    """Computed nearest or farthest ancestor of the given type."""
    return Property(
        id=id,
        node_kind=kind,
        node_types=(node_type,),
        is_list=False,
        is_internal=True,
        is_computed=True,
        is_stored=store,
        is_proto=wire,
        is_required=require,
        node_same_bench=is_bench_implicit,
    )


p_node_ancestor = functools.partial(_p_node_ancestor, kind=NodeReferenceKind.NODE_ANCESTOR)
p_node_ancestor_with_self = functools.partial(
    p_node_ancestor, kind=NodeReferenceKind.NODE_ANCESTOR_OR_SELF
)


def p_node_template(id: int) -> Any:
    """Template property for a node."""
    return Property(
        id=id,
        node_kind=NodeReferenceKind.NODE_TEMPLATE,
        node_basless=True,
        node_ckless=True,
        is_internal=True,
        is_stored=True,
        is_proto=True,
        is_list=False,
    )


# TODO :Architecture!: only one 'value/value_packed' property per Node at top-level
#  (for *all* custom values, simplify edit paths into just one element: property id or field key,
#   which means we can drastically simplify edit tracking/syncing)
#  (what about values in Structs like Expression.value and Field.default?)


p_regular = functools.partial(p_property, internal=False, system=False)
p_internal = functools.partial(p_property, internal=True, system=False)
p_system = functools.partial(p_property, internal=True, system=True)
p_kernel = functools.partial(p_property, internal=True, system=True, sensitive=True, kernel=True)

if TYPE_CHECKING:
    p_regular = p_internal = p_system = p_kernel = p_property  # type: ignore

METATYPE_PROPERTY = Property(
    id=1,
    name="metatype",
    default=None,
    py_type_raw=ObjectType,
    is_internal=True,
    is_required=True,
    is_computed=True,  # is set statically by class decorator
    is_ephemeral=True,
    is_runtime=False,
    is_proto=True,
    is_stored=False,
    is_list=False,
    primitive_type=PrimitiveType.INT16,
    enum_type=EnumType.OBJECT_TYPE,
)
_PROPERTY_SPECIFIERS: tuple[Callable, ...] = (
    p_property,
    p_runtime,
    p_node_parent,
    p_node_ancestor,
)
