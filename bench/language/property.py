import dataclasses
from dataclasses import dataclass
from datetime import datetime
import enum
import functools
from typing import TYPE_CHECKING, Any, Union, Callable, Optional, Iterable
from uuid import UUID

from bench.language.const import (
    UNSET,
    StructType,
    ReferenceKind,
    NodeType,
    NodeRelationFlag,
    BenchType,
    IN_PACKAGE_NODE_TYPES,
    NRel,
    PrimitiveType,
)
from bench.language.graph import NodeList, InMemoryGraphNodeList, ValueList
from bench.language.setup import _on_completing_setup, BENCH_CLASSES_BY_NAME
from bench.language.validation import PropertyValidationHandler
from bench.sql.core import CascadeAction, Column, Table
from bench.utils.func import parse_py_annotation, IdEnum, try_tuple

if TYPE_CHECKING:
    from bench.language import Node, Struct, TypeInfo, PropertyReference
    from bench.language.expression import _TypeExpressionBase

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
class Property(_TypeExpressionBase if TYPE_CHECKING else object):
    """A system-defined attribute of a node or struct."""

    # basics
    id: int | None = None  # stable id for wiring properties, must be unique per final struct/node
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
    is_array: bool = UNSET
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
    reference_kind: ReferenceKind | None = None  # for reference relations
    reference_nodes: tuple[NodeType, ...] | None = None  # for node relations
    reference_wired_ptr: Optional["Property"] = None  # wired representation
    reference_stored_ptrs: tuple["Property", ...] | None = None  # stored representation
    reference_source: Optional["Property"] = None  # for contributed properties
    reference_on_delete: CascadeAction | None = UNSET
    reference_struct: StructType | None = None  # for struct child types
    reference_flags: NodeRelationFlag = NodeRelationFlag.DEFAULT
    reference_list_type: type["NodeList"] | type["ValueList"] | None = None

    _cached_as_ref: Optional["PropertyReference"] = None
    _cached_as_type: Optional["TypeInfo"] = None
    _is_finalized: bool = False

    def __post_init__(self):
        if self.reference_kind is not None and self.default is UNSET:
            self.default = None

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
        elif self.primitive_type is not UNSET:
            non_default.append(self.primitive_type.bench_name)
        for k in (
            "alias",
            "is_array",
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
            # clear contributed properties
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
                self._cached_as_type = TypeInfo(
                    bench_type=self.reference_nodes[0],  # don't have unions yet, doesn't matter
                    is_array=self.is_array,
                    is_required=self.is_required,
                )
            elif self.is_struct:
                self._cached_as_type = TypeInfo(
                    bench_type=self.reference_struct,
                    is_array=self.is_array,
                    is_required=self.is_required,
                )
            elif self.primitive_type:
                self._cached_as_type = TypeInfo(
                    primitive_type=self.primitive_type,
                    is_array=self.is_array,
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
            and (not self.reference_kind or self.reference_nodes)
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
    def reference_type(self) -> NodeType:
        assert len(self.reference_nodes) == 1, f"expected single reference type for {self!r}"
        return self.reference_nodes[0]

    @property
    def reference_ptrs(self) -> Iterable["Property"]:
        if self.reference_wired_ptr is not None:
            yield self.reference_wired_ptr
        if self.reference_stored_ptrs is not None:
            yield from self.reference_stored_ptrs

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

    def _equals_type(self, other: "Property") -> bool:
        """Compares everything but the source component."""
        for k in dataclasses.fields(self):
            if k.name in (
                "id",
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
        if annotation.is_array != self.is_array:
            raise ValueError(f"array mismatch for {self!r} (expected is_array={self.is_array})")
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
            elif getattr(annotation.type, "__is_struct__", False):
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

    def _contribute_ptrs(self) -> tuple["Property", ...]:
        """
        Contribute the wired and stored pointer properties required by this property.
        NOTE: contribute mutates this property, so can only be called once.
        """

        assert self.reference_stored_ptrs is None, f"already contributed {self!r}"

        # property reference
        if self.reference_kind == ReferenceKind.PROPERTY:
            assert self.is_array is not UNSET, f"must set is_array on {self!r}"
            assert self.is_required is not UNSET, f"must set is_required on {self!r}"
            property_ptr = Property(
                id=self.id,
                name=self.name + "_ptr",
                component=self.component,
                py_type_raw=list["PropertyReference"] if self.is_array else "PropertyReference",
                reference_struct=StructType.PROPERTY_REFERENCE,
                is_runtime=True,
                is_internal=self.is_internal,
                is_wired=True,
                is_stored=True,
                is_required=self.is_required,
                is_array=self.is_array,
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
                is_wired=True,
                is_stored=True,
                is_required=False,
                is_array=False,
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
                is_wired=True,
                is_stored=True,
                is_required=False,
                is_array=False,
                default=None,
                reference_source=self,
            )
            self.is_runtime = True
            self.reference_stored_ptrs = (parent_id, parent_key)
            self.reference_wired_ptr = parent_id
            return parent_id, parent_key

        # wired/stored pointer settings for each node reference kind
        if self.reference_kind == ReferenceKind.NODE_PARENT:
            is_wired = True
            is_stored = True
            is_required = False
            is_array = False
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
            is_array = False
            is_computed = True
            is_internal = True
            on_delete = CascadeAction.CASCADE
        elif self.reference_kind == ReferenceKind.NODE_REGULAR:
            assert self.is_required is not UNSET, f"must set is_required on {self!r}"
            assert self.is_array is not UNSET, f"must set is_array on {self!r}"
            is_wired = True
            is_stored = True
            is_required = self.is_required
            is_array = self.is_array
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
                py_type_raw=list["NodeReference"] if is_array else "NodeReference",
                reference_kind=self.reference_kind,
                reference_nodes=self.reference_nodes,
                reference_source=self,
                reference_struct=StructType.NODE_REFERENCE,
                is_runtime=True,
                is_wired=True,
                is_stored=False,
                is_computed=is_computed,
                is_array=is_array,
                is_required=is_required,
                is_internal=is_internal,
                default=None,
                primitive_type=None,
            )
        if is_stored:
            stored_ptr_props = []
            for ref_type in self.reference_nodes:  # :RavelReferences
                store_as_id = (
                    self.reference_kind
                    in (ReferenceKind.NODE_PARENT, ReferenceKind.NODE_ANCESTOR_FIRST)
                    or ref_type not in IN_PACKAGE_NODE_TYPES
                    or ref_type == NodeType.PACKAGE
                )
                prop_postfix = "id" if store_as_id else "ck"
                if ref_type.name.lower() in self.name:  # reduce clutter if type is unambiguous
                    prop_name = f"{self.name}_{prop_postfix}"
                else:
                    prop_name = f"{self.name}_{ref_type.name.lower()}_{prop_postfix}"
                stored_prop = Property(
                    id=self.id,
                    name=prop_name,
                    component=self.component,
                    py_type_raw=list[UUID] if is_array else UUID,
                    reference_kind=self.reference_kind,
                    reference_nodes=(ref_type,),
                    reference_source=self,
                    reference_on_delete=on_delete,
                    is_runtime=False,
                    is_wired=False,
                    is_stored=True,
                    is_internal=is_internal,
                    is_array=is_array,
                    is_required=is_required,
                    primitive_type=PrimitiveType.UUID,
                    is_indexed_in_pg=self.is_indexed_in_pg,
                )
                stored_ptr_props.append(stored_prop)
            self.reference_stored_ptrs = tuple(stored_ptr_props)

        return tuple(self.reference_ptrs)

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
    from bench.language.expression import _TypeExpressionBase

    for name, attr in _TypeExpressionBase.__dict__.items():
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
    elif struct:
        reference_kind = ReferenceKind.STRUCT_CHILD
        custom_list = custom_list or ValueList
    else:
        reference_kind = None
    if array and not (struct or references):
        assert default is UNSET and default_factory is None, f"can't set default for array"
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
        is_internal=internal,
        is_system=system,
        is_kernel=kernel,
        is_required=require,
        is_autoset=autoset,
        is_runtime=True,
        is_wired=wire,
        is_stored=store,
        is_array=array,
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
        is_array=False,
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
        is_array=False,
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
        is_array=True,
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
        is_array=False,
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
        is_array=False,
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
        is_array=False,
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
        is_array=False,
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
        is_array=False,
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
    is_array=False,
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
