import dataclasses
import enum
import functools
from dataclasses import dataclass
from datetime import datetime
from sys import intern
from typing import TYPE_CHECKING, Any, Callable, Iterable, Optional, Union
from uuid import UUID

from bench.language.const import (
    BASED_NODE_TYPES,
    IN_BENCH_NODE_TYPES,
    SUB_PACKAGE_NODE_TYPES,
    UNSET,
    BenchType,
    NodeRelationFlag,
    NodeType,
    NRel,
    PrimitiveType,
    ReferenceKind,
    StructType,
)
from bench.language.graph import InMemoryGraphNodeList, NodeList, ValueList
from bench.language.setup import BENCH_CLASSES_BY_NAME, STRUCT_CLASS_BY_TYPE, _on_completing_setup
from bench.language.validation import PropertyValidationHandler
from bench.sql.core import CascadeAction, Column, Table
from bench.utils.func import IdEnum, parse_py_annotation, try_tuple
from bench.utils.utils import frozendict

if TYPE_CHECKING:
    # noinspection PyUnresolvedReferences
    from bench.language import Node, NodeReference, PropertyReference, Struct, TypeInfo
    from bench.language.expression import _TypeQueryBuilder

PRIMITIVE_TYPE_BY_PY_TYPE: dict[type, PrimitiveType] = {
    bool: PrimitiveType.BOOLEAN,
    int: PrimitiveType.INT32,
    float: PrimitiveType.FLOAT32,
    str: PrimitiveType.STRING,
    bytes: PrimitiveType.BYTES,
    datetime: PrimitiveType.DATETIME,
    UUID: PrimitiveType.UUID,
}


@dataclass(eq=False, slots=True)
class Property(_TypeQueryBuilder if TYPE_CHECKING else object):
    """A system-defined attribute of a node or struct."""

    # basics
    id: int | None = None  # stable id for wiring properties, must be unique per final struct/node
    id_as_str: str | None = None  # str(id)
    ord: int | None = None  # unstable ordinal for bit-packing
    name: str | None = None  # name from LHS of assignment
    description: str | None = None  # description from docstring
    component: type["Struct"] | type["Node"] | None = None  # source component class
    py_type_raw: Any = None  # type annotation on LHS of assignment
    py_type_stripped: Any = UNSET  # stripped type annotation
    alias: str | None = None  # for node list relations
    primitive_type: PrimitiveType | None = UNSET
    default: Any = UNSET
    default_factory: Callable[[], Any] | None = None
    custom_validate: Callable[[Any, "PropertyValidationHandler"], bool | None] | None = None

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
    is_serial: bool = False  # auto-incrementing integer

    # value
    is_value_runtime: bool = False  # for user 'value' properties
    is_value_packed: bool = False  # for packed value properties (the underlying value)
    is_value_dynamic: bool = False  # for freeform value properties (dynamically untyped)
    value_packed_ptr: Union[int, "Property", None] = None  # the packed value
    secret_value_packed_ptr: Union[int, "Property", None] = None  # the secret packed value
    value_type_info_ptr: Union[int, "Property", None] = None  # the type info for the value
    value_type_info_getter: Callable[["Node"], "TypeInfo"] | None = None  # type info getter

    # references (nodes and struct/value)
    reference_kind: ReferenceKind | None = None
    reference_nodes: tuple[NodeType, ...] | None = None  # for node relations
    reference_wired_ptr: Optional["Property"] = None  # wired representation
    reference_stored_ptrs: tuple["Property", ...] | None = None  # stored representation
    reference_stored_props: tuple["Property", ...] | None = None
    reference_stored_ptrs_by_type: dict[NodeType, "Property"] | None = None
    reference_stored_extras: dict[str, "Property"] | None = None
    reference_source: Optional["Property"] = None
    reference_on_delete: CascadeAction | None = UNSET
    reference_struct: StructType | None = None  # for struct child types
    reference_flags: NodeRelationFlag = NodeRelationFlag.DEFAULT
    reference_list_type: type["NodeList"] | type["ValueList"] | None = None
    reference_force_by_id: bool = False

    _cached_as_ref: Optional["PropertyReference"] = None
    _cached_as_type: Optional["TypeInfo"] = None
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
        elif self.primitive_type is not UNSET:
            non_default.append(self.primitive_type.bench_name)
        for k in (
            "alias",
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
            if v is UNSET or (not v and type(v) is not int or v != 0):
                continue
            elif k == "id":
                non_default.append(str(v))
            elif k == "reference_types":
                types_str = "|".join(t.bench_name for t in v)
                if types_str:
                    non_default.append(f"references={types_str}")
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
            reference_stored_ptrs=None,
        )

    @property
    def _as_type(self) -> "TypeInfo":
        """The type info for this property (can't extend TypeInfo because circles)."""

        if self._cached_as_type is None:
            assert self.is_introspectable, f"{self!r} is not introspectable"
            from bench.language.field import TypeInfo

            if self.reference_kind:
                # don't have unions yet, doesn't matter
                bench_type = (
                    self.reference_nodes[0] if self.reference_nodes else self.reference_struct
                )
                self._cached_as_type = TypeInfo(
                    bench_type=bench_type,
                    is_list=self.is_list,
                    is_required=self.is_required,
                )
            elif self.is_struct:
                self._cached_as_type = TypeInfo(
                    bench_type=self.reference_struct,
                    is_list=self.is_list,
                    is_required=self.is_required,
                )
            elif self.primitive_type:
                self._cached_as_type = TypeInfo(
                    primitive_type=self.primitive_type,
                    is_list=self.is_list,
                    is_required=self.is_required,
                )
            else:
                raise ValueError(f"cannot determine type info for {self!r}")
        return self._cached_as_type

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
                type=self.component.metatype, id=self.id, references_type=references_type
            )
            self._cached_as_ref = ref
        return self._cached_as_ref

    @property
    def py_ident(self) -> str:
        return self.name

    @property
    def column(self) -> Column:
        assert isinstance(self.component.__table__, Table), f"{self.component} has no table"
        return self.component.__table__._columns_by_name[self.name]

    @property
    def type(self) -> Optional[BenchType]:
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
            and (not self.reference_kind or self.reference_nodes or self.reference_struct)
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
        return isinstance(self.py_type_stripped, enum.EnumMeta)

    def to_wired_ptr(
        self, ref: Union["Node", "Struct", list["Node"], list["Struct"], None]
    ) -> Union[Any, None]:
        if ref is None:
            return None
        elif self.is_list:
            if self.is_node_reference or self.is_property_reference:
                return [r.to_ref() for r in ref]
        else:
            if self.is_node_reference or self.is_property_reference:
                return ref.to_ref()
            elif self.is_struct_reference:
                if ref.__is_struct_only__ and not ref.__is_struct_inlined__:
                    assert isinstance(ref.id, int), f"expected int id to wire {self!r}: {ref.id}"
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
                "reference_stored_ptrs",
                "reference_source",
                "py_type_raw",
                "py_type_stripped",
            ):
                continue
            if getattr(self, k.name) != getattr(other, k.name):
                return False
        return True

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

        # resolve py type
        if self.is_ephemeral or self.reference_kind == ReferenceKind.NODE_CHILD:
            # can't resolve these because they may point to non-Bench types
            self.py_type_stripped = self.py_type_raw
            return

        # update/check info from annotation
        annotation = parse_py_annotation(self.py_type_raw, BENCH_CLASSES_BY_NAME)
        self.py_type_stripped = annotation.type
        if annotation.is_list != self.is_list:
            raise ValueError(f"array mismatch for {self!r} (expected is_list={self.is_list})")
        if self.is_required is UNSET:
            self.is_required = not annotation.is_optional
        if not self.is_required and self.default is UNSET and self.default_factory is None:
            self.default = None

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

        # resolve & check value type info
        if self.is_value_runtime:
            from bench.language.value import HasValues

            assert (
                HasValues in self.component.__static_components__
            ), f"{self.component} is not HasValues"
            if isinstance(self.value_type_info_ptr, int):
                self.value_type_info_ptr = self.component.__properties_by_id__[
                    self.value_type_info_ptr
                ]
            assert self.value_packed_ptr is not None, f"{self!r} is missing value_packed_ptr"
            if isinstance(self.value_packed_ptr, int):
                self.value_packed_ptr = self.component.__properties_by_id__[self.value_packed_ptr]
            if isinstance(self.secret_value_packed_ptr, int):
                self.secret_value_packed_ptr = self.component.__properties_by_id__[
                    self.secret_value_packed_ptr
                ]

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

    def _contribute_ptrs(self, is_inlined: bool) -> tuple["Property", ...]:
        """
        Contribute the wired and stored pointer properties required by this property.
        NOTE: contribute mutates this property, so can only be called once.
        """

        assert self.reference_stored_ptrs is None, f"already contributed {self!r}"

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
            self.reference_stored_ptrs = (property_ptr,)
            self.reference_wired_ptr = property_ptr
            self.is_runtime = True
            return (property_ptr,)

        # struct (parent) references
        if self.reference_kind == ReferenceKind.STRUCT_PARENT:
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

        # wired/stored pointer settings for each node reference kind
        if self.reference_kind == ReferenceKind.NODE_PARENT:
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

        # contribute the properties
        if is_wired:
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
                is_computed=is_computed,
                is_list=is_list,
                is_required=is_required,
                is_internal=is_internal,
                default=None,
                primitive_type=None,
            )
        extra_stored_props: dict[str, "Property"] = {}
        if is_stored and self.component.__is_node__:  # only nodes are stored
            stored_ptr_props = []
            sub_package_ref_types: list[NodeType] = []
            for ref_type in self.reference_nodes:  # :RavelReferences
                store_as_id = (
                    self.reference_force_by_id is True
                    or self.reference_kind
                    in (ReferenceKind.NODE_PARENT, ReferenceKind.NODE_ANCESTOR_FIRST)
                    or ref_type not in SUB_PACKAGE_NODE_TYPES
                )
                if not store_as_id:
                    sub_package_ref_types.append(ref_type)
                    continue
                if ref_type.name.lower() in self.name:  # reduce clutter if type is unambiguous
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
                    is_runtime=False,
                    is_wired=False,
                    is_stored=True,
                    is_internal=is_internal,
                    is_list=is_list,
                    is_required=is_required,
                    primitive_type=PrimitiveType.UUID,
                    is_indexed_in_pg=self.is_indexed_in_pg,
                )
                stored_ptr_props.append(id_ref_prop)
            if sub_package_ref_types:
                ck_ref_prop = Property(
                    id=self.id,
                    name=self.name + "_ck",
                    component=self.component,
                    py_type_raw=list[UUID] if is_list else UUID,
                    reference_nodes=tuple(sub_package_ref_types),
                    reference_source=self,
                    is_runtime=False,
                    is_wired=False,
                    is_stored=True,
                    is_internal=is_internal,
                    is_list=is_list,
                    is_required=is_required,
                    primitive_type=PrimitiveType.UUID,
                )
                stored_ptr_props.append(ck_ref_prop)
                if len(sub_package_ref_types) > 1:
                    # disambiguate type for heterogeneous ck references :HomogeneousListCk
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
            is_ref_in_bench = any(t in IN_BENCH_NODE_TYPES for t in sub_package_ref_types)
            if is_ref_in_bench:
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
            if any(t in BASED_NODE_TYPES for t in sub_package_ref_types):
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
                if is_ref_in_bench:
                    stored_base_bench_id = extra_stored_props["bench_id"].clone()
                    stored_base_bench_id.name = self.name + "_base_bench_id"
                    extra_stored_props["base_bench_id"] = stored_base_bench_id

            stored_ptrs_by_type = {}
            for prop in stored_ptr_props:
                for ref_type in prop.reference_nodes:
                    stored_ptrs_by_type[ref_type] = prop
            self.reference_stored_ptrs = tuple(stored_ptr_props)
            self.reference_stored_ptrs_by_type = frozendict(stored_ptrs_by_type)
            self.reference_stored_props = tuple(
                stored_ptr_props + list(extra_stored_props.values())
            )
            self.reference_stored_extras = frozendict(extra_stored_props)

        return tuple(self.contributed_props)

    def new(self) -> Any:
        """Gets a new default value for this property"""
        if self.default is not UNSET:
            return self.default
        elif self.default_factory is not None:
            return self.default_factory()
        else:
            raise ValueError(f"no default for {self!r}")

    def validate(self, value: Any, on_notice: "PropertyValidationHandler") -> bool | None:
        """Validates a non-None value of this property"""
        if self.custom_validate is not None:
            return self.custom_validate(value, on_notice)
        else:
            return None


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
    description: str = None,
    default: Any = UNSET,
    default_factory: Callable[[], Any] = None,
    require: bool = UNSET,
    references: tuple[NodeType, ...] | NodeType = None,
    reference_force_by_id: bool = False,
    struct: StructType = None,
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
    validate: Callable[[Any, "PropertyValidationHandler"], bool | None] = None,
):
    if references:
        reference_kind = ReferenceKind.NODE_REGULAR
    elif struct == StructType.PROPERTY_REFERENCE:
        reference_kind = ReferenceKind.PROPERTY
        struct = None
        custom_list = ValueList
    elif struct:
        reference_kind = ReferenceKind.STRUCT_CHILD
        custom_list = custom_list or ValueList
    else:
        reference_kind = None
    if array and not (struct or references):
        assert default is UNSET and default_factory is None, "can't set default for array"
        default_factory = list
    return Property(
        id=id,
        description=description,
        default=default,
        default_factory=default_factory,
        primitive_type=primitive_type,
        custom_validate=validate,
        reference_kind=reference_kind,
        reference_nodes=try_tuple(references),
        reference_struct=struct,
        reference_list_type=custom_list,
        reference_force_by_id=reference_force_by_id,
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
    default_factory: Callable[[], Any] = None,
) -> object:
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


def p_node_parent(id: int, *node_type: NodeType, is_system: bool = False):
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
    kind: ReferenceKind = ReferenceKind.NODE_ANCESTOR_FIRST,
):
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
    )


p_node_ancestor_root = functools.partial(p_node_ancestor, kind=ReferenceKind.NODE_ANCESTOR_ROOT)


def p_node_child(
    node_type: NodeType,
    flags: NRel = NRel.DEFAULT,
    list: type["NodeList"] = None,
    alias: str = None,
):
    """Computed read/write children or descendants of the given type."""
    return Property(
        reference_kind=ReferenceKind.NODE_CHILD,
        reference_nodes=(node_type,),
        reference_flags=flags,
        is_internal=True,
        is_required=True,
        is_list=True,
        reference_list_type=list or InMemoryGraphNodeList,
        is_stored=False,
        alias=alias,
    )


def p_struct_parent(id: int):
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
    type: int | Callable[["Node"], "TypeInfo"] | None = None,
) -> Property:
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


def p_value_packed(id: int) -> Property:
    """Packed value property."""
    return Property(
        id=id,
        primitive_type=PrimitiveType.JSON,
        default=None,
        is_value_packed=True,
        is_required=False,
        is_internal=True,
        is_system=True,
        is_stored=True,
        is_wired=True,
        is_list=False,
    )


def p_secret_value_packed(id: int) -> Property:
    """Packed secret value property."""
    return Property(
        id=id,
        primitive_type=PrimitiveType.JSON,
        default=None,
        is_value_packed=True,
        is_required=False,
        is_internal=True,
        is_system=True,
        is_stored=True,
        is_wired=True,
        is_sensitive=True,
        is_encrypted=True,
        is_deferred=True,
        is_list=False,
    )


def p_value_dynamic(id: int) -> Property:
    """Freeform value property."""
    return Property(
        id=id,
        primitive_type=PrimitiveType.JSON,
        default=None,
        is_value_dynamic=True,
        is_required=False,
        is_internal=True,
        is_system=True,
        is_stored=True,
        is_wired=True,
        is_list=False,
    )


p_regular = functools.partial(p_property, internal=False, system=False)
p_internal = functools.partial(p_property, internal=True, system=False)
p_system = functools.partial(p_property, internal=True, system=True)
p_kernel = functools.partial(p_property, internal=True, system=True, sensitive=True, kernel=True)
METATYPE_PROPERTY = Property(
    id=1,
    name="metatype",
    default=None,
    py_type_raw=BenchType,
    is_internal=True,
    is_required=True,
    is_computed=True,  # is set statically by class decorator
    is_ephemeral=True,
    is_runtime=False,
    is_wired=True,
    is_stored=False,
    is_list=False,
    primitive_type=PrimitiveType.STRING,
)
_PROPERTY_SPECIFIERS = (
    p_property,
    p_runtime,
    p_node_parent,
    p_node_ancestor,
    p_node_child,
    p_value_runtime,
    p_value_packed,
    p_secret_value_packed,
)
