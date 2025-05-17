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
from uuid import UUID

from bench.language.registry import ENUM_TYPE_BY_CLASS, _on_completing_setup
from bench.utils.func import hash_stable, parse_py_annotation
from bench.utils.utils import frozendict

from .const import (
    BASED_NODE_TYPES,
    BENCH_NODE_TYPES,
    EMPTY_DICT,
    INSTANTIABLE_NODE_TYPES,
    NODE_TYPES,
    PACKAGE_NODE_TYPES,
    PRIMITIVE_TYPE_BY_PY_TYPE,
    UNSET,
    BuiltinEnum,
    EnumType,
    NodeType,
    ObjectType,
    PrimitiveType,
    ReferenceKind,
    StructType,
    TypeKind,
    enum_,
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


@enum_(EnumType.PROPERTY_REFERENCE_TYPE)
class PropertyReferenceType(BuiltinEnum):
    ID = 1
    CK = 2
    BENCH_ID = 3
    BASE_ID = 4
    NODE_TYPE = 6


PROPERTY_META_KEY_BY_TYPE = {
    PropertyReferenceType.ID: "id",
    PropertyReferenceType.CK: "ck",
    PropertyReferenceType.BENCH_ID: "bench_id",
    PropertyReferenceType.BASE_ID: "base_id",
    PropertyReferenceType.NODE_TYPE: "node_type",
}
PROPERTY_TYPE_BY_META_KEY = {v: k for k, v in PROPERTY_META_KEY_BY_TYPE.items()}


@dataclass(eq=False, slots=True)
class Property(_IntoQuery if TYPE_CHECKING else object):
    """A system-defined attribute of a BuiltinObject (Struct or Node)."""

    # basics
    # NOTE: yes cast(int, None) is a bit evil but we almost always immediately assign it here and
    #  don't want to deal with asserting id is not None everywhere.
    # stable id for wiring properties, must be unique per final struct/node
    id: int = cast(int, None)  # noqa: RUF009
    key: str = UNSET  # str(id)
    cache_key: str | None = None  # for runtime value caching
    # ephemeral ordinal for bit-packing
    ord: int = cast(int, None)  # noqa: RUF009
    name: str = UNSET  # name from LHS of assignment
    component: type["BuiltinObject"] = UNSET  # source component class
    py_type_raw: Any = None  # type annotation on LHS of assignment
    py_type: Any = UNSET  # clean type annotation
    primitive_type: PrimitiveType | None = UNSET
    enum_type: EnumType | None = None
    default: Any = UNSET
    default_factory: Callable[[], Any] | None = None
    default_sql: Any = UNSET
    constraint: "TypeConstraint | TypeConstraintIn | None" = None

    # flags
    is_list: bool = UNSET
    is_required: bool = UNSET  # must be non-null
    is_variable: bool = False  # may be wrapped in an indirect Variable lookupg
    is_internal: bool = False  # should be edited via accessors, but not enforced
    is_autoset: bool = False  # set automatically by system, cannot set directly
    is_computed: bool = False
    is_runtime: bool = UNSET  # exists on runtime object
    is_ephemeral: bool = False  # runtime-only in-memory property
    is_proto: bool = UNSET  # serialized onto wire (in proto)
    is_stored: bool = UNSET  # stored in DB
    is_indexed: bool = False  # indexed in DB?
    is_unique: bool = False  # unique index in DB?
    is_sensitive: bool = False  # sensitive data (generally requires special permissions)
    is_untracked: bool = False  # whether writes are tracked

    # references to nodes or structs
    reference_kind: ReferenceKind | None = None
    reference_nodes: tuple[NodeType, ...] | None = None  # for node relations
    reference_wired_ptr: Optional["Property"] = None  # wired representation
    reference_stored_ids: tuple["Property", ...] | None = None  # stored representation
    reference_stored_props: tuple["Property", ...] | None = None
    reference_stored_ids_by_type: dict[NodeType, "Property"] | None = None
    reference_stored_metas: dict[PropertyReferenceType, "Property"] | None = None
    reference_type: PropertyReferenceType | None = None
    reference_source: Optional["Property"] = None
    reference_struct: StructType | None = None  # for struct child types
    reference_is_bench_implicit: bool = False
    reference_is_ckless: bool = False
    reference_is_baseless: bool = False
    reference_force_fk: bool = False
    reference_is_node_data: bool = False

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
        if self.reference_kind:
            non_default.append(self.reference_kind.bench_name)
            if self.reference_nodes:
                if self.reference_nodes == "any":
                    non_default.append("*")
                else:
                    non_default.append("|".join(t.bench_name for t in self.reference_nodes))
            elif self.reference_struct:
                non_default.append(self.reference_struct.bench_name)
        elif self.enum_type:
            non_default.append(self.enum_type.bench_name)
        elif self.primitive_type and self.primitive_type is not UNSET:
            non_default.append(self.primitive_type.bench_name)
        for k in (
            "is_list",
            "is_required",
            "is_runtime",
            "is_proto",
            "is_stored",
            "is_computed",
            "is_ephemeral",
            "is_internal",
            "is_system",
            "is_kernel",
            "reference_kind",
            "reference_nodes",
            "reference_struct",
            "ord",
        ):
            v = getattr(self, k)
            if v is UNSET or ((not v and type(v) is not int) or v != 0):
                continue
            elif k == "id":
                non_default.append(str(v))
            else:
                if isinstance(v, bool):
                    non_default.append(k)
                else:
                    non_default.append(f"{k}={v}")
        attrs_str = ", ".join(non_default)
        attrs_str = f" ({attrs_str})" if attrs_str else ""
        return f"<{self.__class__.__name__} {self!s}{attrs_str}>"

    # see IntoQuery.__eq__ for Property==Property equality

    def _stable_hash(self):
        """Hash the Property identity."""
        return hash_stable((self.component.__name__, self.id, self.reference_type))

    __hash__ = _stable_hash  # type: ignore

    def clone(self):
        return dataclasses.replace(
            self,
            component=None,
            # reset contributed properties
            reference_wired_ptr=None,
            reference_stored_ids=None,
        )

    def to_ref(self) -> "PropertyReference":
        """A pointer to this property. `to_ref()` for consistency with `Node.to_ref()`."""

        if self._ref is None:
            assert self.component is not None, f"{self!r} is not finalized"
            from .object import PropertyReference

            ref = PropertyReference(
                object_type=getattr(self.component, "metatype", None), id=self.id
            )
            if self.reference_source is not None and self.reference_source.is_node_reference:
                if self.reference_nodes == "any":
                    ref.references_node_type = NODE_TYPES.tuple[0]
                else:
                    assert self.reference_nodes is not None, f"missing reference nodes for {self!r}"
                    ref.references_node_type = self.reference_nodes[0]
                ref.references_meta = self.reference_type
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
        return self.reference_kind is not None and self.reference_kind.is_node_tree

    @property
    def is_node_reference(self):
        return self.reference_kind is not None and self.reference_kind.is_node

    @property
    def is_struct_reference(self):
        """Whether this is a reference to a parent struct/value. *Not* an inlined Struct."""
        return self.reference_kind is not None and self.reference_kind.is_struct_tree

    @property
    def is_struct(self) -> bool:
        return self.reference_struct is not None

    @property
    def is_property_reference(self) -> bool:
        return self.reference_struct == StructType.PROPERTY_REFERENCE

    @property
    def is_optional(self) -> bool:
        return not self.is_required

    @property
    def is_optional_scalar(self) -> bool:
        return self.is_optional and not self.is_list

    @property
    def is_enum(self):
        return self.enum_type is not None

    def equals_type(self, other: "Property") -> bool:
        """Compares everything but the source component."""
        for k in dataclasses.fields(self):
            if k.name in (
                "id",
                "key",
                "ord",
                "component",
                "ignore_conflicts",
                "reference_wired_ptr",
                "reference_stored_ids",
                "reference_source",
                "py_type_raw",
                "py_type_stripped",
            ):
                continue
            if getattr(self, k.name) != getattr(other, k.name):
                return False
        return True

    def _to_type(self) -> "Type":
        from bench.language.core import Type, TypeConstraintIn

        if isinstance(self.constraint, TypeConstraintIn):
            constraint = self.constraint.into()
        else:
            constraint = self.constraint or TypeConstraintIn()

        if self.reference_nodes and not self.reference_source:
            kind = TypeKind.NODE
            bench_type = None
            if self.reference_nodes != "any":
                constraint.node_types = list(self.reference_nodes)
            primitive_type = None
        elif self.reference_struct:
            kind = TypeKind.STRUCT
            bench_type = self.reference_struct
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
                self.reference_source is not None
                or self.reference_nodes
                or self.is_property_reference
                or self.id == 1
            ):
                self._type = self._to_type()
            assert self._type is not None, f"{self!r} is not finalized"
        return self._type

    def _contribute_ptrs(self, *, is_root: bool) -> tuple["Property", ...]:
        """
        Contribute the wired and stored pointer properties required by this property.
        NOTE: contribute mutates this property, so can only be called once.
        """

        assert self.reference_stored_ids is None, f"already contributed {self!r}"

        # property reference
        if self.is_property_reference:
            assert self.is_list is not UNSET, f"must set is_list on {self!r}"
            assert self.is_required is not UNSET, f"must set is_required on {self!r}"
            property_ptr = Property(
                id=self.id,
                name=self.name + "_ptr",
                component=self.component,
                py_type_raw=list["PropertyReference"] if self.is_list else "PropertyReference",
                reference_struct=StructType.PROPERTY_REFERENCE,
                is_runtime=True,
                is_internal=self.is_internal,
                is_proto=True,
                is_stored=True,
                is_required=self.is_required,
                is_list=self.is_list,
                default=None,
                reference_source=self,
                constraint=self.constraint,
            )
            self.reference_stored_props = (property_ptr,)
            self.reference_wired_ptr = property_ptr
            self.is_runtime = True
            return (property_ptr,)

        # NOTE :Cleanup: mapping stored and wired references (i.e. node pointers) is gnarly.
        #  In wire pointers (=NodeReference[Data]) we conveniently have a struct with all the info:
        #   type+id+[ck]+[bench_id]+[base_id]
        #  But we don't want to store pointers as structs for efficiency, so we map them to columns.
        #  We want FKs on some id columns (like parent pointers), so those need to be
        #  distinct id columns, while others can be bunched together into (id, ck, type) tuple.
        #  This makes for the rather complex logic here and in unpacking/packing refs into rows.
        #  :StoredPointers

        # wired/stored pointer settings for each node reference kind
        elif self.reference_kind == ReferenceKind.NODE_PARENT:
            is_proto = True
            is_stored = True
            is_required = False  # parent is always optional
            is_list = False
            is_computed = False
            is_internal = True
        elif self.reference_kind in (
            ReferenceKind.NODE_ANCESTOR_OR_SELF,
            ReferenceKind.NODE_ANCESTOR,
        ):
            assert self.is_proto is not UNSET, f"must set is_proto on {self!r}"
            assert self.is_stored is not UNSET, f"must set is_stored on {self!r}"
            is_proto = self.is_proto
            is_stored = self.is_stored
            is_required = self.is_required
            is_list = False
            is_computed = True
            is_internal = True
        elif self.reference_kind in (ReferenceKind.NODE_REGULAR, ReferenceKind.NODE_TEMPLATE):
            assert self.is_required is not UNSET, f"must set is_required on {self!r}"
            assert self.is_list is not UNSET, f"must set is_list on {self!r}"
            is_proto = True
            is_stored = True
            is_required = self.is_required
            is_list = self.is_list
            is_computed = False
            is_internal = False
        else:
            raise ValueError(f"unexpected reference kind {self.reference_kind!r} for {self!r}")

        if is_proto:
            # wired reference representation is just a nice NodeReference struct
            self.reference_wired_ptr = Property(
                id=self.id,  # re-use id, self is not stored
                name=self.name + "_ptr",
                component=self.component,
                py_type_raw=list["NodeReference"] if is_list else "NodeReference",
                reference_kind=self.reference_kind,
                reference_nodes=self.reference_nodes,
                reference_source=self,
                reference_struct=StructType.NODE_REFERENCE,
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
                self.reference_kind == ReferenceKind.NODE_ANCESTOR
                or self.reference_kind == ReferenceKind.NODE_ANCESTOR_OR_SELF
            ):
                # wired ancestors are not required (even though stored ancestors are)
                self.reference_wired_ptr.is_required = False

        if is_stored and self.component.__is_node__:  # only nodes are stored
            assert self.reference_wired_ptr is not None, f"missing wired ptr for {self!r}"
            # unravel the reference types into appropriate id/ck/other metadata columns
            stored_ids = []  # the contributed 'ptr'-like properties (id or ck)
            # ... and any other metadata (type, base, etc.)
            stored_meta_props: dict[PropertyReferenceType, Property] = {}
            need_fks = self.reference_force_fk
            if self.reference_nodes == "any":
                reference_nodes = NODE_TYPES.tuple
            else:
                reference_nodes = self.reference_nodes or ()

            # figure out which reference types (if any) to pack into the shared 'id'/'ck'
            shared_ptr_types: list[NodeType] = []
            if need_fks:
                for ref_type in reference_nodes:
                    if ref_type in PACKAGE_NODE_TYPES:
                        shared_ptr_types.append(ref_type)
                        continue  # no FKs for package types
                    elif ref_type.name.lower() in self.name:
                        # reduce clutter if type is unambiguous
                        prop_name = f"{self.name}_id"
                    else:
                        prop_name = f"{self.name}_{ref_type.name.lower()}_id"
                    id_prop = Property(
                        id=self.id,
                        name=prop_name,
                        component=self.component,
                        py_type_raw=list[UUID] if is_list else UUID,
                        reference_kind=self.reference_kind,
                        reference_nodes=(ref_type,),
                        reference_source=self,
                        reference_force_fk=True,
                        is_runtime=False,
                        is_proto=False,
                        is_stored=True,
                        is_internal=is_internal,
                        is_list=is_list,
                        is_required=is_required,
                        primitive_type=PrimitiveType.UUID,
                        is_indexed=self.is_indexed,
                    )
                    stored_ids.append(id_prop)
            elif reference_nodes:
                shared_ptr_types.extend(reference_nodes)

            if shared_ptr_types:
                id_prop = Property(
                    id=self.id,
                    name=self.name + "_id",
                    component=self.component,
                    py_type_raw=list[UUID] if is_list else UUID,
                    reference_nodes=tuple(shared_ptr_types),
                    reference_source=self,
                    reference_type=PropertyReferenceType.ID,
                    is_runtime=False,
                    is_proto=False,
                    is_stored=True,
                    is_internal=is_internal,
                    is_list=is_list,
                    is_required=is_required,
                    primitive_type=PrimitiveType.UUID,
                )
                stored_ids.append(id_prop)
                # also remember 'ck' if any of the shared types has one
                if (
                    any(t in INSTANTIABLE_NODE_TYPES for t in shared_ptr_types)
                    and not self.reference_is_ckless
                ):
                    ck_prop = Property(
                        id=self.id,
                        name=self.name + "_ck",
                        component=self.component,
                        py_type_raw=list[UUID] if is_list else UUID,
                        reference_nodes=tuple(shared_ptr_types),
                        reference_source=self,
                        reference_type=PropertyReferenceType.CK,
                        is_runtime=False,
                        is_proto=False,
                        is_stored=True,
                        is_internal=is_internal,
                        is_list=is_list,
                        is_required=is_required,
                        primitive_type=PrimitiveType.UUID,
                    )
                    stored_meta_props[PropertyReferenceType.CK] = ck_prop
                if len(shared_ptr_types) > 1:
                    # disambiguate type for heterogeneous ck references :HomogeneousListCk
                    stored_meta_props[PropertyReferenceType.NODE_TYPE] = Property(
                        id=self.id,
                        name=self.name + "_type",
                        component=self.component,
                        py_type_raw=list[NodeType] if is_list else NodeType,
                        reference_source=self,
                        reference_type=PropertyReferenceType.NODE_TYPE,
                        is_runtime=False,
                        is_proto=False,
                        is_stored=True,
                        is_internal=is_internal,
                        is_list=is_list,
                        is_required=is_required,
                        primitive_type=PrimitiveType.INT16,
                    )

            # we need the 'bench_id' for the reference if it could be in a Bench
            is_sub_bench = any(
                t != NodeType.BENCH and t in BENCH_NODE_TYPES for t in reference_nodes or ()
            )
            if (
                is_sub_bench
                and self.reference_kind != ReferenceKind.NODE_PARENT
                and not self.reference_is_bench_implicit
            ):
                stored_meta_props[PropertyReferenceType.BENCH_ID] = Property(
                    id=self.id,
                    name=self.name + "_bench_id",
                    component=self.component,
                    py_type_raw=list[UUID] if is_list else UUID,
                    reference_kind=self.reference_kind,
                    reference_source=self,
                    reference_type=PropertyReferenceType.BENCH_ID,
                    reference_nodes=(NodeType.BENCH,),
                    is_runtime=False,
                    is_proto=False,
                    is_stored=True,
                    is_internal=is_internal,
                    is_list=is_list,
                    is_required=is_required and all(t in BENCH_NODE_TYPES for t in reference_nodes),
                    primitive_type=PrimitiveType.UUID,
                )

            # we also need a base and _its_ bench if this is a 'based' node (node with a base node)
            if (
                any(t in BASED_NODE_TYPES for t in shared_ptr_types)
                and not self.reference_is_baseless
            ):
                stored_meta_props[PropertyReferenceType.BASE_ID] = Property(
                    id=self.id,
                    name=self.name + "_base_id",
                    component=self.component,
                    reference_kind=self.reference_kind,
                    py_type_raw=list[UUID] if is_list else UUID,
                    reference_source=self,
                    reference_type=PropertyReferenceType.BASE_ID,
                    is_runtime=False,
                    is_proto=False,
                    is_stored=True,
                    is_internal=is_internal,
                    is_list=is_list,
                    is_required=False,
                    primitive_type=PrimitiveType.UUID,
                )

            # index contributed info into this property
            stored_ids_by_type = {}
            for prop in stored_ids:
                for ref_type in prop.reference_nodes:
                    stored_ids_by_type[ref_type] = prop
            self.reference_stored_ids = tuple(stored_ids)
            self.reference_stored_ids_by_type = frozendict(stored_ids_by_type)
            self.reference_stored_props = tuple(stored_ids + list(stored_meta_props.values()))
            self.reference_stored_metas = frozendict(stored_meta_props)

            return (self.reference_wired_ptr, *stored_ids, *stored_meta_props.values())
        elif self.reference_wired_ptr:
            return (self.reference_wired_ptr,)
        else:
            return ()

    def _finalize_meta(self) -> None:
        """Analyzes the storage options. Must run after all class defs."""
        # store/wire property by default if not runtime (and not indicated otherwise)
        if self.primitive_type is UNSET and (self.is_tree_reference or self.reference_nodes):
            if self.is_stored is UNSET:
                self.is_stored = False
            self.primitive_type = None
        elif self.is_stored is UNSET:
            self.is_stored = True
        if self.is_proto is UNSET:
            self.is_proto = self.is_stored
        if self.is_runtime is UNSET:
            self.is_runtime = self.is_stored
        # obviously, we don't store the runtime properties with different wired/stored representations
        #  directly, we just use is_proto/is_stored to indicate whether to contribute those (above)
        if self.reference_wired_ptr or self.reference_stored_ids:
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
                assert self.reference_struct is not None, f"missing struct type for {self!r}"
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
    default_factory: Callable[[], Any] | None = None,
    default_sql: Any = UNSET,
    same_bench: bool = False,
    baseless: bool = False,
    ckless: bool = False,
    is_node_data: bool = False,
    store: bool = True,
    wire: bool = True,
    primitive_type: PrimitiveType | None = UNSET,
    index_in_pg: bool = False,
    unique: bool = False,
    sensitive: bool = False,
    constraint: "TypeConstraint | TypeConstraintIn | None" = None,
) -> Any:
    return Property(
        id=id,
        default=default,
        default_sql=default_sql,
        default_factory=default_factory,
        primitive_type=primitive_type,
        constraint=constraint,
        reference_is_node_data=is_node_data,
        reference_is_bench_implicit=same_bench,
        reference_is_baseless=baseless,
        reference_is_ckless=ckless,
        is_internal=internal,
        is_autoset=autoset,
        is_untracked=autoset,
        is_runtime=True,
        is_proto=wire,
        is_stored=store,
        is_sensitive=sensitive,
        is_indexed=index_in_pg,
        is_unique=unique,
    )


def p_runtime(
    *,
    default: Any = None,
    default_factory: Callable[[], Any] | None = None,
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
        reference_kind=ReferenceKind.NODE_PARENT,
        reference_nodes=tuple(node_type),
        default=None,
        is_internal=True,
        is_stored=False,
        is_list=False,
        is_runtime=True,
        is_untracked=True,
        is_required=False,
        reference_is_ckless=ckless,
        reference_is_baseless=baseless,
    )


def _p_node_ancestor(
    id: int,
    node_type: NodeType,
    kind: ReferenceKind,
    store: bool = False,
    wire: bool = False,
    require: bool = UNSET,
    index_in_pg: bool = False,
    is_bench_implicit: bool = False,
) -> Any:
    """Computed nearest or farthest ancestor of the given type."""
    return Property(
        id=id,
        reference_kind=kind,
        reference_nodes=(node_type,),
        is_list=False,
        is_internal=True,
        is_computed=True,
        is_stored=store,
        is_proto=wire,
        is_required=require,
        is_indexed=index_in_pg,
        reference_is_bench_implicit=is_bench_implicit,
    )


p_node_ancestor = functools.partial(_p_node_ancestor, kind=ReferenceKind.NODE_ANCESTOR)
p_node_ancestor_with_self = functools.partial(
    p_node_ancestor, kind=ReferenceKind.NODE_ANCESTOR_OR_SELF
)


def p_node_template(id: int) -> Any:
    """Template property for a node."""
    return Property(
        id=id,
        reference_kind=ReferenceKind.NODE_TEMPLATE,
        reference_is_baseless=True,
        reference_is_ckless=True,
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
