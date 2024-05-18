import dataclasses
import enum
import functools
from dataclasses import dataclass
from sys import intern
from typing import (
    TYPE_CHECKING,
    Any,
    Callable,
    Iterable,
    Literal,
    Optional,
    Union,
    cast,
)
from uuid import UUID

from bench.language.const import (
    BASED_NODE_TYPES,
    PRIMITIVE_TYPE_BY_PY_TYPE,
    SUB_BENCH_NODE_TYPES,
    SUB_PACKAGE_NODE_TYPES,
    UNSET,
    EnumType,
    NodeRelationFlag,
    NodeType,
    NRel,
    ObjectType,
    PrimitiveType,
    ReferenceKind,
    StructType,
    TypeKind,
)
from bench.language.graph import GraphNodeList, NodeList, ValueList
from bench.language.setup import (
    BENCH_CLASS_BY_NAME,
    ENUM_TYPE_BY_CLASS,
    STRUCT_CLASS_BY_TYPE,
    _on_completing_setup,
)
from bench.language.validation import TypeConstraintIn
from bench.sql.core import CascadeAction, Column, Table
from bench.utils.func import IdEnum, parse_py_annotation, try_tuple
from bench.utils.utils import frozendict

if TYPE_CHECKING:
    # noinspection PyUnresolvedReferences
    from bench.language import (
        Node,
        NodeReference,
        Object,
        PropertyReference,
        Struct,
        TypeConstraint,
        TypeInfo,
        TypeInfoBase,
    )
    from bench.language.expression import _TypeQueryBuilder

PropertyReferenceMetadata = Union[
    Literal["id"],
    Literal["ck"],
    Literal["bench_id"],
    Literal["base_ck"],
    Literal["base_bench_id"],
    Literal["type"],
]


@dataclass(eq=False, slots=True)
class Property(_TypeQueryBuilder if TYPE_CHECKING else object):
    """A system-defined attribute of a node or struct."""

    # basics
    # NOTE: yes cast(int, None) is a bit evil but we almost always immediately assign it here and
    #  don't want to deal with asserting id is not None everywhere.
    id: int = cast(
        int, None
    )  # stable id for wiring properties, must be unique per final struct/node
    id_as_str: str = UNSET  # str(id)
    ord: int = cast(int, None)  # unstable ordinal for bit-packing
    name: str = UNSET  # name from LHS of assignment
    component: type["Struct"] | type["Node"] = UNSET  # source component class
    py_type_raw: Any = None  # type annotation on LHS of assignment
    py_type_stripped: Any = UNSET  # stripped type annotation
    primitive_type: PrimitiveType | None = UNSET
    enum_type: EnumType | None = None
    default: Any = UNSET
    default_factory: Callable[[], Any] | None = None
    constraint: "TypeConstraint | TypeConstraintIn | None" = None

    # flags
    is_list: bool = UNSET
    is_required: bool = False  # = must be non-null
    is_internal: bool = False  # = should be edited via accessors, but not enforced
    is_system: bool = False  # = only editable by system
    is_kernel: bool = False  # = only viewable by system
    is_autoset: bool = False  # = set automatically by system, cannot set directly
    is_computed: bool = False
    is_runtime: bool = UNSET  # exists on runtime instance
    is_ephemeral: bool = False  # runtime-only in-memory property
    is_wired: bool = UNSET  # serialized onto wire (in proto)
    is_stored: bool = UNSET  # stored in DB
    is_indexed_in_pg: bool = False  # indexed in DB?
    is_unique: bool = False  # unique index in DB?
    is_deferred: bool = False  # loaded only on demand (only for stored node properties)
    is_sensitive: bool = False  # sensitive data (generally requires special permissions)
    is_encrypted: bool = False  # encrypt at rest (only node properties)

    # value
    is_value_runtime: bool = False  # for user 'value' properties
    is_value_packed: bool = False  # for packed value properties (the underlying value)
    value_packed_ptr: Union[int, "Property", None] = None  # the packed value
    secret_value_packed_ptr: Union[int, "Property", None] = None  # the secret packed value
    value_type_info_ptr: Union[int, "Property", None] = None  # the type info for the value
    value_type_info_getter: Callable[["Struct"], "TypeInfoBase"] | None = None  # type info getter

    # references (nodes and struct/value)
    reference_kind: ReferenceKind | None = None
    reference_nodes: tuple[NodeType, ...] | None = None  # for node relations
    reference_wired_ptr: Optional["Property"] = None  # wired representation
    reference_stored_ids: tuple["Property", ...] | None = None  # stored representation
    reference_stored_props: tuple["Property", ...] | None = None
    reference_stored_ids_by_type: dict[NodeType, "Property"] | None = None
    reference_stored_meta: dict[PropertyReferenceMetadata, "Property"] | None = None
    reference_source: Optional["Property"] = None
    reference_on_delete: CascadeAction | None = UNSET
    reference_struct: StructType | None = None  # for struct child types
    reference_flags: NodeRelationFlag = NodeRelationFlag.DEFAULT
    reference_list_type: type["NodeList"] | type["ValueList"] | None = None
    reference_is_bench_implicit: bool = False
    reference_force_fk: bool = False

    _cached_as_ref: Optional["PropertyReference"] = None
    type_info: Optional["TypeInfo"] = None
    _is_finalized: bool = False

    def __post_init__(self):
        if self.reference_kind is not None and self.default is UNSET:
            self.default = None
        if self.id is not None:
            self.id_as_str = intern(str(self.id))

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
            "is_wired",
            "is_stored",
            "is_computed",
            "is_ephemeral",
            "is_internal",
            "is_system",
            "is_kernel",
            "is_deferred",
            "is_encrypted",
            "reference_kind",
            "reference_nodes",
            "reference_struct",
            "ord",
        ):
            v = getattr(self, k)
            if v is UNSET or (not v and type(v) is not int or v != 0):  # noqa: E721
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
        return f"<{self.__class__.__name__} {str(self)}{attrs_str}>"

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

        if self._cached_as_ref is None:
            assert self.component is not None, f"{self!r} is not finalized"
            from bench.language.expression import PropertyReference

            if (
                self.reference_kind
                and self.reference_kind.is_node
                and self.reference_nodes is not None
                and len(self.reference_nodes) == 1
            ):
                references_type = self.reference_nodes[0]
            else:
                references_type = None
            ref = PropertyReference(
                type=getattr(self.component, "metatype", None),
                id=self.id,
                references_type=references_type,
            )
            self._cached_as_ref = ref
        return self._cached_as_ref

    @property
    def py_ident(self) -> str:
        assert self.name is not None, f"{self!r} has no name"
        return self.name

    @property
    def column(self) -> Column:
        table = getattr(self.component, "__table__", None)
        assert isinstance(table, Table), f"{self.component} has no table"
        return table._columns_by_name[self.name]

    @property
    def type(self) -> Optional[ObjectType]:
        return self.component.metatype

    @property
    def has_id(self) -> int:
        return self.id is not None and self.id is not UNSET

    @property
    def is_introspectable(self) -> bool:
        return (
            # exclude our own runtime-only properties
            not self.is_ephemeral
            # exclude empty references type, TypeInfo can't handle that yet
            and (
                not self.reference_kind or bool(self.reference_nodes) or bool(self.reference_struct)
            )
            # exclude ancestor properties (they're computed but would be nice to have :c)
            and self.reference_kind
            not in (ReferenceKind.NODE_ANCESTOR_FIRST, ReferenceKind.NODE_ANCESTOR_ROOT)
            # exclude contributed reference properties (like parent_id)
            and not self.reference_source
        )

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
    def contributed_props(self) -> Iterable["Property"]:
        if self.reference_wired_ptr is not None:
            yield self.reference_wired_ptr
        if self.reference_stored_props is not None:
            yield from self.reference_stored_props

    @property
    def is_struct(self) -> bool:
        return self.reference_struct is not None

    @property
    def is_property_reference(self) -> bool:
        return self.reference_kind == ReferenceKind.PROPERTY

    @property
    def is_optional(self) -> bool:
        return not self.is_required

    @property
    def is_enum(self):
        return self.enum_type is not None

    def to_wired_ptr(
        self,
        ref: Union["Object", "Node", "Struct", list["Object"], list["Node"], list["Struct"], None],
    ) -> Union[
        "NodeReference",
        "PropertyReference",
        list["NodeReference"],
        list["PropertyReference"],
        int,
        None,
    ]:
        """Transforms the given instantiated reference value for this property int its wired form."""
        if ref is None:
            return None
        elif self.is_list:
            if self.is_node_reference or self.is_property_reference:
                assert isinstance(ref, (list, tuple)), f"expected list for {self!r}: {ref!r}"
                return [cast("Node", r).to_ref() for r in ref]
        else:
            if self.is_node_reference or self.is_property_reference:
                return (cast(Union["Node", "Property"], ref)).to_ref()
            elif self.is_struct_reference:
                from bench.language.value import Object

                ref = cast(Union["Node", "Struct", "Object"], ref)
                if type(ref) is Object or ref.__is_struct_only__ and not ref.__is_struct_inlined__:
                    assert isinstance(ref.id, int), f"expected id for {self!r}: {ref!r}.id={ref.id}"
                    return ref.id
                else:
                    return None  # not stored
        raise ValueError(f"unexpected ref {ref!r} for {self!r}")

    def _equals_type(self, other: "Property") -> bool:
        """Compares everything but the source component."""
        for k in dataclasses.fields(self):
            if k.name in (
                "id",
                "id_as_str",
                "ord",
                "component",
                "ignore_conflicts",
                "reference_wired_ptr",
                "referenced_stored_ids",
                "reference_source",
                "py_type_raw",
                "py_type_stripped",
            ):
                continue
            if getattr(self, k.name) != getattr(other, k.name):
                return False
        return True

    def _to_type_info(self) -> "TypeInfo":
        from bench.language.field import TypeInfo

        if isinstance(self.constraint, TypeConstraintIn):
            constraint = self.constraint.into()
        else:
            constraint = self.constraint

        if self.reference_nodes:
            kind = TypeKind.NODE
            # don't have unions yet so we special case this in validation :FakeNodePropertyUnion
            bench_type = self.reference_nodes[0]
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
            kind = TypeKind.PRIMITIVE
            bench_type = None
            primitive_type = self.primitive_type
        else:
            raise ValueError(f"cannot determine type info for {self!r}")
        typ = TypeInfo(
            kind=kind,
            bench_type=bench_type,
            primitive_type=primitive_type,
            is_list=self.is_list,
            # NOTE: we ignore is_required if deferred since we don't have a mechanism for determining
            #  which properties were loaded in a given graph yet. Revisit this with read info.
            is_required=self.is_required and not self.is_deferred,
            constraint=constraint,
            _from_property=self,
        )
        typ._do_resolve_to(typ)  # auto-resolve to self
        return typ

    @property
    def as_type_info(self) -> "TypeInfo":
        """The type info for this property (can't extend TypeInfo because circles)."""
        assert self.type_info is not None, f"{self!r} is not finalized"
        return self.type_info

    def _finalize(self) -> None:
        """Analyzes the final type and configures storage options. Must run after all class defs."""

        # store/wire property by default if not runtime (and not indicated otherwise)
        if self.primitive_type is UNSET and (self.is_tree_reference or self.reference_nodes):
            if self.is_stored is UNSET:
                self.is_stored = False
            self.primitive_type = None
        elif self.is_stored is UNSET:
            self.is_stored = True
        if self.is_wired is UNSET:
            self.is_wired = self.is_stored
        if self.is_runtime is UNSET:
            self.is_runtime = self.is_stored

        # resolve & check value type info
        if self.is_value_runtime:
            from bench.language.value import HasValues

            assert (
                HasValues in self.component.__static_components__
            ), f"{self.component} is not HasValues"
            if isinstance(self.value_type_info_ptr, int):
                resolved_ptr = self.component.__properties_by_id__.get(self.value_type_info_ptr)
                assert (
                    resolved_ptr is not None
                ), f"invalid type ptr {self.value_type_info_ptr} info for {self!r}"
                self.value_type_info_ptr = resolved_ptr
            assert self.value_packed_ptr is not None, f"{self!r} is missing value_packed_ptr"
            if isinstance(self.value_packed_ptr, int):
                self.value_packed_ptr = self.component.__properties_by_id__[self.value_packed_ptr]
            if isinstance(self.secret_value_packed_ptr, int):
                self.secret_value_packed_ptr = self.component.__properties_by_id__[
                    self.secret_value_packed_ptr
                ]

        # resolve py type
        if self.is_ephemeral or self.reference_kind == ReferenceKind.NODE_CHILDREN:
            # can't resolve these because they may point to non-Bench types
            self.py_type_stripped = self.py_type_raw
            return

        # update/check info from annotation
        annotation = parse_py_annotation(self.py_type_raw, BENCH_CLASS_BY_NAME)
        self.py_type_stripped = annotation.type
        if annotation.is_list != self.is_list:
            raise ValueError(f"array mismatch for {self!r} (expected is_list={self.is_list})")
        if self.is_required is UNSET:
            self.is_required = not annotation.is_optional
        if not self.is_required and self.default is UNSET and self.default_factory is None:
            self.default = None
        if isinstance(annotation.type, type) and issubclass(annotation.type, enum.Enum):
            if not issubclass(annotation.type, IdEnum):
                raise ValueError(f"only IdEnum is supported for enums: {self!r}")
            self.enum_type = ENUM_TYPE_BY_CLASS.get(annotation.type)
            if self.enum_type is None:
                raise ValueError(f"missing enum type for {annotation.type!r} at {self!r}")

        # determine storage type
        if self.primitive_type is UNSET and (self.is_stored or self.is_wired):
            if annotation.is_union:
                raise ValueError(f"cannot store union {self!r}")
            # map to column type
            assert isinstance(annotation.type, type), f"invalid type {annotation!r} for {self!r}"
            if issubclass(annotation.type, IdEnum):
                self.primitive_type = PrimitiveType.INT16
            elif issubclass(annotation.type, enum.IntFlag):
                self.primitive_type = PrimitiveType.INT64
            elif getattr(annotation.type, "__is_node__", False):
                raise ValueError(f"cannot store node directly: {self!r}")
            elif getattr(annotation.type, "__is_struct_only__", False):
                assert self.reference_struct is not None, f"missing struct type for {self!r}"
                self.primitive_type = PrimitiveType.JSON  # robust json
            else:
                primitive_type = PRIMITIVE_TYPE_BY_PY_TYPE.get(annotation.type)
                if primitive_type is None:
                    raise ValueError(f"cannot determine storage for {self!r}: {self.py_type_raw!r}")
                self.primitive_type = primitive_type

        # sanity check some stuff
        from bench.language.node import Node

        if (
            self.component.__is_struct_inlined__
            and self.reference_kind == ReferenceKind.STRUCT_CHILD
            and self.reference_struct
        ):
            referenced_struct_cls = STRUCT_CLASS_BY_TYPE[self.reference_struct]
            if not referenced_struct_cls.__is_struct_inlined__:
                raise ValueError(f"{self!r} cannot reference non-inlined struct {self!r}")
        if self.is_encrypted and not self.is_sensitive:
            raise ValueError(f"encrypted properties should be sensitive {self!r}")
        if self.is_encrypted and not self.is_deferred:
            raise ValueError(f"encrypted properties should be deferred {self!r}")
        if (
            self.id is not None
            and self.id is not UNSET
            and 10 < self.id < 30  # (below 10 would conflict anyway, above 30 is fine)
            and self.component.__name__ not in ("Node", "Struct")
            and self.id not in Node.__properties_by_id__
        ):
            raise ValueError(f"can't use system id {self.id} for {self!r}")

        # derive type info
        if self.is_introspectable or self.reference_source is not None:
            self.type_info = self._to_type_info()

    def _contribute_ptrs(self, is_inlined: bool) -> tuple["Property", ...]:
        """
        Contribute the wired and stored pointer properties required by this property.
        NOTE: contribute mutates this property, so can only be called once.
        """

        assert self.reference_stored_ids is None, f"already contributed {self!r}"
        is_parent = self.reference_kind == ReferenceKind.NODE_PARENT

        # property reference
        if self.reference_kind == ReferenceKind.PROPERTY:
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
                is_wired=True,
                is_stored=True,
                is_required=self.is_required,
                is_list=self.is_list,
                default=None,
                reference_source=self,
            )
            self.reference_stored_props = (property_ptr,)
            self.reference_wired_ptr = property_ptr
            self.is_runtime = True
            return (property_ptr,)

        # struct (parent) references
        elif self.reference_kind == ReferenceKind.STRUCT_PARENT:
            parent_id = Property(
                id=self.id,
                name=self.name + "_id",
                component=self.component,
                primitive_type=PrimitiveType.INT32,
                py_type_raw=int,
                reference_kind=ReferenceKind.STRUCT_PARENT,
                is_runtime=True,
                is_internal=True,
                is_wired=not is_inlined,
                is_stored=not is_inlined,
                is_required=False,
                is_list=False,
                default=None,
                reference_source=self,
            )
            parent_key = Property(
                id=self.id + 1,
                name=self.name + "_key",
                component=self.component,
                primitive_type=PrimitiveType.STRING,
                py_type_raw=str,
                reference_kind=ReferenceKind.STRUCT_PARENT,
                is_runtime=True,
                is_internal=True,
                is_wired=not is_inlined,
                is_stored=not is_inlined,
                is_required=False,
                is_list=False,
                default=None,
                reference_source=self,
            )
            self.is_runtime = True
            # not stored because self.component must be a struct
            self.reference_wired_ptr = parent_id
            return parent_id, parent_key

        # NOTE :Cleanup: mapping stored and wired references (i.e. node pointers) is gnarly.
        #  In wire pointers (=NodeReference[Data]) we conveniently have a struct with all the info:
        #   type+id+[ck]+[bench_id]+[base_ck+base_bench_id]
        #  But we don't want to store pointers as structs for efficiency, so we map them to columns.
        #  We want FKs on some id columns (like parent pointers), so those need to be
        #  distinct id columns, while others can be bunched together into a 'id' + 'ck' + 'type'.
        #  This makes for the rather complex logic here and in unpacking/packing refs into rows.
        #  :StoredPointers

        # wired/stored pointer settings for each node reference kind
        elif is_parent:
            is_wired = True
            is_stored = True
            is_required = False
            is_list = False
            is_computed = False
            is_internal = True
            on_delete = CascadeAction.CASCADE
        elif self.reference_kind in (
            ReferenceKind.NODE_ANCESTOR_ROOT,
            ReferenceKind.NODE_ANCESTOR_FIRST,
        ):
            assert self.is_wired is not UNSET, f"must set is_wired on {self!r}"
            assert self.is_stored is not UNSET, f"must set is_stored on {self!r}"
            is_wired = self.is_wired
            is_stored = self.is_stored
            is_required = self.is_required
            is_list = False
            is_computed = True
            is_internal = True
            on_delete = CascadeAction.CASCADE
        elif self.reference_kind == ReferenceKind.NODE_REGULAR:
            assert self.is_required is not UNSET, f"must set is_required on {self!r}"
            assert self.is_list is not UNSET, f"must set is_list on {self!r}"
            is_wired = True
            is_stored = True
            is_required = self.is_required
            is_list = self.is_list
            is_computed = False
            is_internal = False
            on_delete = CascadeAction.SET_NULL
        else:
            raise ValueError(f"unexpected reference kind {self.reference_kind!r} for {self!r}")

        if is_wired:
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
                is_wired=True,
                is_stored=False,
                is_autoset=self.is_autoset,
                is_computed=is_computed,
                is_list=is_list,
                is_required=is_required,
                is_internal=is_internal,
                default=None,
                primitive_type=None,
            )

        # unravel the reference types into appropriate id/ck/other metadata columns
        stored_ids = []  # the contributed 'ptr'-like properties (id or ck)
        # ... and any other metadata (type, base, etc.)
        extra_stored_props: dict[PropertyReferenceMetadata, "Property"] = {}
        if is_stored and self.component.__is_node__:  # only nodes are stored
            need_fks = self.reference_force_fk or is_parent

            # figure out which reference types (if any) to pack into the shared 'id'/'ck'
            shared_ptr_types: list[NodeType] = []
            if need_fks:
                assert self.reference_nodes is not None, f"unset reference nodes for {self!r}"
                for ref_type in self.reference_nodes:
                    if not is_parent and ref_type in SUB_PACKAGE_NODE_TYPES:
                        shared_ptr_types.append(ref_type)
                        continue
                    if ref_type.name.lower() in self.name:
                        # reduce clutter if type is unambiguous
                        prop_name = f"{self.name}_id"
                    else:
                        prop_name = f"{self.name}_{ref_type.name.lower()}_id"
                    id_ref_prop = Property(
                        id=self.id,
                        name=prop_name,
                        component=self.component,
                        py_type_raw=list[UUID] if is_list else UUID,
                        reference_kind=self.reference_kind,
                        reference_nodes=(ref_type,),
                        reference_source=self,
                        reference_on_delete=on_delete,
                        reference_force_fk=True,
                        is_runtime=False,
                        is_wired=False,
                        is_stored=True,
                        is_internal=is_internal,
                        is_list=is_list,
                        is_required=is_required,
                        primitive_type=PrimitiveType.UUID,
                        is_indexed_in_pg=self.is_indexed_in_pg,
                    )
                    stored_ids.append(id_ref_prop)
            elif self.reference_nodes:
                shared_ptr_types.extend(self.reference_nodes)

            if shared_ptr_types:
                id_ref_prop = Property(
                    id=self.id,
                    name=self.name + "_id",
                    component=self.component,
                    py_type_raw=list[UUID] if is_list else UUID,
                    reference_nodes=tuple(shared_ptr_types),
                    reference_source=self,
                    is_runtime=False,
                    is_wired=False,
                    is_stored=True,
                    is_internal=is_internal,
                    is_list=is_list,
                    is_required=is_required,
                    primitive_type=PrimitiveType.UUID,
                )
                stored_ids.append(id_ref_prop)
                # also remember 'ck' if any of the shared types has one
                if any(t in SUB_PACKAGE_NODE_TYPES for t in shared_ptr_types):
                    ck_ref_prop = Property(
                        id=self.id,
                        name=self.name + "_ck",
                        component=self.component,
                        py_type_raw=list[UUID] if is_list else UUID,
                        reference_nodes=tuple(shared_ptr_types),
                        reference_source=self,
                        is_runtime=False,
                        is_wired=False,
                        is_stored=True,
                        is_internal=is_internal,
                        is_list=is_list,
                        is_required=is_required,
                        primitive_type=PrimitiveType.UUID,
                    )
                    extra_stored_props["ck"] = ck_ref_prop
                if len(shared_ptr_types) > 1:
                    # disambiguate type for heterogeneous ck references :HomogeneousListCk
                    # NOTE: we only support homogenous lists because it would be a pain to mix types.
                    assert not is_list, f"cannot store heterogeneous types in list: {self!r}"
                    extra_stored_props["type"] = Property(
                        id=self.id,
                        name=self.name + "_type",
                        component=self.component,
                        py_type_raw=list[NodeType] if is_list else NodeType,
                        reference_source=self,
                        is_runtime=False,
                        is_wired=False,
                        is_stored=True,
                        is_internal=is_internal,
                        is_list=is_list,
                        is_required=is_required,
                        primitive_type=PrimitiveType.INT16,
                    )

            # we need the 'bench_id' for the reference if it could be in a Bench
            is_sub_bench = any(t in SUB_BENCH_NODE_TYPES for t in self.reference_nodes or ())
            if is_sub_bench and not is_parent and not self.reference_is_bench_implicit:
                extra_stored_props["bench_id"] = Property(
                    id=self.id,
                    name=self.name + "_bench_id",
                    component=self.component,
                    py_type_raw=list[UUID] if is_list else UUID,
                    reference_kind=self.reference_kind,
                    reference_source=self,
                    reference_nodes=(NodeType.BENCH,),
                    reference_on_delete=CascadeAction.SET_NULL,
                    is_runtime=False,
                    is_wired=False,
                    is_stored=True,
                    is_internal=is_internal,
                    is_list=is_list,
                    is_required=is_required,
                    primitive_type=PrimitiveType.UUID,
                )

            # we also need a base and _its_ bench if this is a 'based' node (node with a base node)
            if any(t in BASED_NODE_TYPES for t in shared_ptr_types):
                assert not is_list, f"cannot store based types in list: {self!r}"
                extra_stored_props["base_ck"] = Property(
                    id=self.id,
                    name=self.name + "_base_ck",
                    component=self.component,
                    reference_kind=self.reference_kind,
                    py_type_raw=UUID,
                    reference_source=self,
                    is_runtime=False,
                    is_wired=False,
                    is_stored=True,
                    is_internal=is_internal,
                    is_list=False,
                    is_required=is_required,
                    primitive_type=PrimitiveType.UUID,
                )
                if is_sub_bench and not is_parent and not self.reference_is_bench_implicit:
                    stored_base_bench_id = extra_stored_props["bench_id"].clone()
                    stored_base_bench_id.name = self.name + "_base_bench_id"
                    extra_stored_props["base_bench_id"] = stored_base_bench_id

            # index contributed info into this property
            stored_ids_by_type = {}
            for prop in stored_ids:
                for ref_type in prop.reference_nodes:
                    stored_ids_by_type[ref_type] = prop
            self.reference_stored_ids = tuple(stored_ids)
            self.reference_stored_ids_by_type = frozendict(stored_ids_by_type)
            self.reference_stored_props = tuple(stored_ids + list(extra_stored_props.values()))
            self.reference_stored_meta = frozendict(extra_stored_props)

        return tuple(self.contributed_props)

    def new(self) -> Any:
        """Gets a new default value for this property"""
        if self.default is not UNSET:
            return self.default
        elif self.default_factory is not None:
            return self.default_factory()
        else:
            raise ValueError(f"no default for {self!r}")


@_on_completing_setup
def _add_property_expression_base():
    from bench.language.expression import _TypeQueryBuilder

    for name, attr in _TypeQueryBuilder.__dict__.items():
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
    require: bool = UNSET,
    references: tuple[NodeType, ...] | NodeType | None = None,
    fk: bool = False,
    is_bench_implicit: bool = False,
    struct: StructType | None = None,
    store: bool = True,
    wire: bool = True,
    primitive_type: PrimitiveType = UNSET,
    index_in_pg: bool = False,
    array: bool = False,
    defer: bool = False,
    encrypt: bool = False,
    unique: bool = False,
    sensitive: bool = False,
    custom_list: type["ValueList"] | None = None,
    constraint: "TypeConstraint | TypeConstraintIn | None" = None,
) -> Any:
    if references:
        reference_kind = ReferenceKind.NODE_REGULAR
    elif struct == StructType.PROPERTY_REFERENCE:
        assert custom_list is None, "can't set custom list for property reference"
        reference_kind = ReferenceKind.PROPERTY
        struct = None
        custom_list = ValueList
    elif struct:
        reference_kind = ReferenceKind.STRUCT_CHILD
        custom_list = custom_list or ValueList
    else:
        reference_kind = None
    if (
        array
        and reference_kind != ReferenceKind.STRUCT_CHILD
        and reference_kind != ReferenceKind.NODE_CHILDREN
    ):
        assert (
            default is UNSET and default_factory is None
        ), f"can't set default for array: {default!r}"
        default_factory = list
    if fk:
        assert not array, "can't have foreign key on list"
    return Property(
        id=id,
        default=default,
        default_factory=default_factory,
        primitive_type=primitive_type,
        constraint=constraint,
        reference_kind=reference_kind,
        reference_nodes=try_tuple(references),
        reference_struct=struct,
        reference_list_type=custom_list,
        reference_is_bench_implicit=is_bench_implicit,
        reference_force_fk=fk,
        is_internal=internal,
        is_system=system,
        is_kernel=kernel,
        is_required=require,
        is_autoset=autoset,
        is_runtime=True,
        is_wired=wire,
        is_stored=store,
        is_list=array,
        is_deferred=defer,
        is_encrypted=encrypt,
        is_sensitive=sensitive,
        is_indexed_in_pg=index_in_pg,
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
        is_wired=False,
        is_ephemeral=True,
        is_computed=False,
        is_required=False,
        is_stored=False,
        default=default,
        default_factory=default_factory,
    )


def p_node_parent(id: int, *node_type: NodeType, is_system: bool = False) -> Any:
    """The parent of a node, must be of one of the given types."""
    return Property(
        id=id,
        reference_kind=ReferenceKind.NODE_PARENT,
        reference_nodes=tuple(node_type),
        is_internal=True,
        is_stored=False,
        is_list=False,
        is_system=is_system,
    )


def p_node_ancestor(
    id: int,
    node_type: NodeType,
    store: bool = False,
    wire: bool = False,
    require: bool = UNSET,
    index_in_pg: bool = False,
    is_bench_implicit: bool = False,
    kind: ReferenceKind = ReferenceKind.NODE_ANCESTOR_FIRST,
) -> Any:
    """Computed nearest or farthest ancestor of the given type."""
    return Property(
        id=id,
        reference_kind=kind,
        reference_nodes=(node_type,),
        is_list=False,
        is_internal=True,
        is_computed=True,
        is_system=True,
        is_stored=store,
        is_wired=wire,
        is_required=require,
        is_indexed_in_pg=index_in_pg,
        reference_is_bench_implicit=is_bench_implicit,
    )


p_node_ancestor_root = functools.partial(p_node_ancestor, kind=ReferenceKind.NODE_ANCESTOR_ROOT)


def p_node_child(
    node_type: NodeType,
    flags: NRel = NRel.DEFAULT,
    list: type["NodeList"] | None = None,
) -> Any:
    """Computed read/write children or descendants of the given type."""
    return Property(
        reference_kind=ReferenceKind.NODE_CHILDREN,
        reference_nodes=(node_type,),
        reference_flags=flags,
        is_internal=True,
        is_required=True,
        is_list=True,
        reference_list_type=list or GraphNodeList,
        is_stored=False,
    )


def p_struct_parent(id: int) -> Any:
    """The parent of a struct."""
    return Property(
        id=id,
        reference_kind=ReferenceKind.STRUCT_PARENT,
        reference_nodes=(),
        is_internal=True,
        is_stored=True,
        is_wired=True,
        is_list=False,
    )


def p_value_runtime(
    packed: int,
    secret_packed: int | None = None,
    *,
    type: int | Callable[["Struct"], "TypeInfoBase"] | None = None,
) -> Any:
    """Runtime-only property for a Value and secret value."""
    value_type_info_id = None
    value_type_info_getter = None
    if isinstance(type, int):
        value_type_info_id = type
    elif callable(type):
        value_type_info_getter = type
    elif type is not None:
        raise ValueError(f"invalid type info {type!r} for p_value_runtime")
    return Property(
        is_internal=True,
        is_runtime=True,
        is_wired=False,
        is_stored=False,
        is_ephemeral=True,
        is_required=False,
        is_value_runtime=True,
        is_list=False,
        default=None,
        value_packed_ptr=packed,
        secret_value_packed_ptr=secret_packed,
        value_type_info_ptr=value_type_info_id,
        value_type_info_getter=value_type_info_getter,
    )


def p_value_packed(id: int) -> Any:
    """Packed value property."""
    return Property(
        id=id,
        primitive_type=PrimitiveType.JSON,
        default=None,
        is_value_packed=True,
        is_required=False,
        is_internal=True,
        is_stored=True,
        is_wired=True,
        is_list=False,
    )


def p_secret_value_packed(id: int) -> Any:
    """Packed secret value property."""
    return Property(
        id=id,
        primitive_type=PrimitiveType.JSON,
        default=None,
        is_value_packed=True,
        is_required=False,
        is_internal=True,
        is_stored=True,
        is_wired=True,
        is_sensitive=True,
        is_encrypted=True,
        is_deferred=True,
        is_list=False,
    )


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
    is_wired=True,
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
    p_node_child,
    p_value_runtime,
    p_value_packed,
    p_secret_value_packed,
)
