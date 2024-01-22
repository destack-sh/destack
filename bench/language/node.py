import abc
import dataclasses
import enum
import functools
import inspect
import uuid
from collections import defaultdict
from dataclasses import dataclass
from datetime import datetime
from itertools import chain
from typing import (
    TYPE_CHECKING,
    Any,
    Callable,
    ClassVar,
    Collection,
    ForwardRef,
    Iterable,
    Optional,
    TypeVar,
    Union,
    cast,
    dataclass_transform,
    final,
)
from uuid import UUID, uuid4

import structlog
from cachetools import cached

from bench.language.const import (
    IN_BENCH_NODE_TYPES,
    IN_MODULE_NODE_TYPES,
    INTERP_NODE_TYPES,
    NODE_TYPES,
    NS,
    STRUCT_TYPES,
    UNSET,
    UUID_NAMESPACE,
    BenchType,
    IssueKind,
    IssueType,
    ModuleReference,
    NodePath,
    NodeRelationType,
    NodeSource,
    NodeStatus,
    NodeTrackingLevel,
    NodeType,
    NRel,
    StatementType,
    StructType,
    TypeHint,
    TypeTag,
    parse_absolute_node_reference,
    parse_node_path,
)
from bench.language.link import (
    _NC,
    NodeList,
    NodeListBase,
    _FieldExpressionBase,
    _InterpChange,
    _NodeExpressionBase,
    on_issue_raise,
)
from bench.language.tree import DetachedNodeTree, NodeDataTree, NodeTree, NodeTreeBase
from bench.language.validation import (
    PropertyValidationHandler,
    ValidationError,
    ValidationHandler,
    on_invalid_raise,
)
from bench.proto.core import ProtoStrEnum
from bench.proto.wire import EditData, NodeReferenceData, SomeNodeData
from bench.sql.core import (
    CascadeAction,
    Column,
    ColumnType,
    Constraint,
    ConstraintType,
    Index,
    IndexType,
    Table,
)
from bench.utils.dt import utcnow_with_tz
from bench.utils.func import (
    check_collections_equal,
    did_you_mean_str,
    get_subclasses,
    strip_py_type,
    try_tuple,
)
from bench.utils.utils import LOCAL_ENV, IdentifierType, frozendict, required_field, to_identifier

if TYPE_CHECKING:
    from bench.language import (
        Field,
        File,
        Issue,
        NodeVisitor,
        Organization,
        Policy,
        PropertyReference,
        Session,
        User,
        WorkerSet,
        symbolx_lib,
    )
    from bench.language.issue import IssueHandler

logger = structlog.get_logger(__name__)


def get_node_id(module_id: UUID, ck: UUID):
    """Derive the version-specific node id from its constant key"""
    return uuid.uuid5(module_id, str(ck))


PROPERTY_COLUMN_TYPE_BY_PY_TYPE: dict[type, ColumnType] = {
    bool: ColumnType.BOOLEAN,
    int: ColumnType.BIGINT,
    float: ColumnType.FLOAT,
    str: ColumnType.STRING,
    bytes: ColumnType.BYTES,
    datetime: ColumnType.DATETIME,
    UUID: ColumnType.UUID,
}


class NodeReferenceKind(enum.StrEnum):
    PARENT = "parent"
    ANCESTOR = "ancestor"
    REGULAR = "regular"
    CHILD = "child"


@dataclass
class Property(_FieldExpressionBase):
    """A property of a module node or struct."""

    id: int | None = None  # stable id for wiring properties, must be unique per final struct/node
    name: str | None = None  # name from LHS of assignment
    description: str | None = None  # description from docstring
    component: type["Struct"] | None = None  # source component class
    py_type_raw: Any = None  # type annotation on LHS of assignment
    py_type_stripped: Any = UNSET  # stripped type annotation
    alias: str | None = None  # for node list relations

    is_array: bool = UNSET
    is_required: bool = False  # = must be non-null
    is_internal: bool = False  # = not directly editable for user
    is_protected: bool = False  # = only editable by system
    is_reflected: bool = UNSET  # eventually all properties should be reflected, for now only some
    is_computed: bool = False
    is_runtime_only: bool = False
    is_runtime: bool = UNSET  # exists on runtime instance?
    is_wired: bool = UNSET  # serialized onto wire?
    is_stored: bool = UNSET  # stored in DB?
    is_indexed_in_pg: bool = False  # indexed in DB?
    is_unique: bool = False  # unique index in DB?
    is_deferred: bool = False  # loaded only on demand (only for stored node properties)
    is_encrypted: bool = False  # encrypt at rest (only node properties)

    is_ancestor_nearest: bool | None = None  # for ancestor relations
    is_ancestor_self: bool | None = None  # for ancestor relations
    reference_kind: NodeReferenceKind | None = None  # for reference relations
    reference_types: tuple[NodeType, ...] | None = None  # for reference relations
    reference_wired_ptr: Optional["Property"] = None  # wired reference for references
    reference_stored_ptrs: tuple["Property", ...] | None = None  # stored reference for references
    reference_source: Optional["Property"] = None  # for reference relations (reverse)
    reference_on_delete: CascadeAction | None = UNSET
    children_flags: NodeRelationType = NodeRelationType.DEFAULT

    struct_type: StructType | None = None  # for struct properties
    column_type: ColumnType | None = UNSET
    default: Any = UNSET
    default_factory: Callable[[], Any] | None = None
    list_type: type["NodeListBase"] | None = None
    custom_validate: Callable[[Any, "PropertyValidationHandler"], bool | None] | None = None
    custom_copy: Callable[[Any], Any] | None = None
    ignore_conflicts_with: tuple[type["Node"], ...] | None = None

    def __post_init__(self):
        if self.reference_kind is not None and self.default is UNSET:
            self.default = None
        if self.is_runtime_only and self.default is UNSET and self.default_factory is None:
            raise ValueError(f"missing default for {self!r}")

    def __str__(self):
        if self.component is None:
            return "<detached>"
        return f"{self.component.__name__}.{self.name}"

    def __repr__(self):
        non_default = []
        if self.id is not None:
            non_default.append(str(self.id))
        if isinstance(self.py_type_stripped, type):
            non_default.append(self.py_type_stripped.__name__)
        for k in (
            "alias",
            "is_array",
            "is_required",
            "is_internal",
            "is_protected",
            "is_runtime_only",
            "is_computed",
            "is_reflected",
            "is_ancestor_nearest",
            "is_ancestor_self",
            "is_deferred",
            "is_encrypted",
            "struct_type",
            "reference_kind",
            "reference_types",
        ):
            v = getattr(self, k)
            if v is UNSET or not v:
                continue
            elif k == "id":
                non_default.append(str(v))
            elif k in ("custom_copy", "custom_validate"):
                func_str = f"{v.__name__}@{hex(id(v))}"
                non_default.append(f"{k}={func_str}")
            elif k == "children_flags":
                flags_str = ", ".join([f.name for f in NodeRelationType if v & f])
                if flags_str:
                    non_default.append(flags_str)
            elif k == "reference_types":
                types_str = "|".join(t.name for t in v)
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
        return dataclasses.replace(self, component=None)

    @functools.cached_property
    def _as_field(self) -> "Field":
        assert self.is_reflected is True, f"{self!r} is not reflected"

        from bench.language.field import Field

        # derive constant ck for field using ids
        metatype = getattr(self.component, "metatype", None)  # (ABCs don't have a metatype)
        metatype_id = metatype.id if metatype is not None else None
        if self.column_type == ColumnType.BOOLEAN:
            tag, hint = TypeTag.BOOLEAN, None
        elif self.column_type == ColumnType.BIGINT:
            tag, hint = TypeTag.NUMBER, TypeHint.INTEGER
        elif self.column_type == ColumnType.FLOAT:
            tag, hint = TypeTag.NUMBER, None
        elif self.column_type == ColumnType.STRING:
            tag, hint = TypeTag.STRING, None
        elif self.column_type == ColumnType.DATETIME:
            tag, hint = TypeTag.STRING, TypeHint.DATETIME
        elif self.column_type == ColumnType.UUID:
            tag, hint = TypeTag.STRING, TypeHint.UUID
        elif self.column_type == ColumnType.JSON:
            tag, hint = TypeTag.JSON, None
        else:
            raise ValueError(f"unexpected column type in {self!r}: {self.column_type}")
        field_ck = uuid.uuid5(UUID_NAMESPACE, f"{metatype_id}.{self.id}")
        field = Field(name=self.name, ck=field_ck, tag=tag, hint=hint, _reflected_from=self)
        return field

    @functools.cached_property
    def ptr(self) -> "PropertyReference":
        """A pointer to this property."""
        assert self.component is not None, f"{self!r} is not finalized"
        from bench.language.expression import PropertyReference

        if self.reference_kind and len(self.reference_types) == 1:
            references_type = self.reference_types[0]
        else:
            references_type = None
        return PropertyReference(
            type=self.component.metatype, id=self.id, references_type=references_type
        )

    @property
    def ident(self) -> str:
        return self.name

    @property
    def column(self) -> Column:
        assert issubclass(self.component, Node), f"{self!r} is not a node property"
        assert isinstance(self.component.__table__, Table), f"{self.component} has no table"
        return self.component.__table__._columns_by_name[self.name]

    @property
    def is_tree_relation(self) -> bool:
        """Whether this is a node relation property (parent/child/ancestor)."""
        return self.reference_kind in (
            NodeReferenceKind.PARENT,
            NodeReferenceKind.ANCESTOR,
            NodeReferenceKind.CHILD,
        )

    @property
    def reference_ptrs(self) -> Iterable["Property"]:
        if self.reference_wired_ptr is not None:
            yield self.reference_wired_ptr
        if self.reference_stored_ptrs:
            yield from self.reference_stored_ptrs

    @property
    def is_static(self):
        return not self.is_runtime_only

    @property
    def is_struct(self) -> bool:
        return self.struct_type is not None

    @property
    def is_optional(self) -> bool:
        return not self.is_required

    @property
    def is_enum(self):
        return isinstance(self.py_type_stripped, enum.EnumMeta)

    def equals_type(self, other: "Property") -> bool:
        """Compares everything but the source component."""
        for k in dataclasses.fields(self):
            if k.name in (
                "id",
                "component",
                "ignore_conflicts_with",
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

    def _finalize_type(self) -> None:
        """Analyzes the final type and configures storage options. Must run after all class defs."""

        # store/wire property by default if not runtime (and not marked as _not_ store)
        if self.column_type is UNSET and (
            self.is_tree_relation or self.is_runtime_only or self.reference_types
        ):
            if self.is_stored is UNSET:
                self.is_stored = False
            self.column_type = None
        elif self.is_stored is UNSET:
            self.is_stored = True
        if self.is_wired is UNSET:
            self.is_wired = self.is_stored
        if self.is_runtime is UNSET:
            self.is_runtime = self.is_stored
        if self.is_reflected is UNSET:
            if not self.is_struct and not self.is_encrypted:
                self.is_reflected = self.is_stored
            else:
                self.is_reflected = False  # can't deal with that yet

        # resolve py type
        if self.is_runtime_only or self.reference_kind:
            # can't resolve these because they point to non-Bench types
            self.py_type_stripped = self.py_type_raw
            py_type = self.py_type_raw
        else:
            py_type, info = strip_py_type(self.py_type_raw)
            # resolve manually if needed
            if isinstance(py_type, (str, ForwardRef)):
                py_type = py_type.__forward_arg__ if not isinstance(py_type, str) else py_type
                if py_type not in _BENCH_CLASSES_BY_NAME:
                    raise ValueError(f"cannot resolve type for {self!r}: {py_type!r}")
                py_type = _BENCH_CLASSES_BY_NAME[py_type]
            self.py_type_stripped = py_type
            # update info from annotation
            if self.is_array is UNSET:
                self.is_array = info.is_array
            if self.is_required is UNSET:
                self.is_required = not info.is_optional

        # determine storage type
        if self.column_type is UNSET and (self.is_stored or self.is_wired):
            # map to column type
            assert isinstance(py_type, type), f"invalid type {py_type!r} for {self!r}"
            if issubclass(py_type, enum.StrEnum):
                self.column_type = ColumnType.STRING
            elif issubclass(py_type, (enum.IntFlag, enum.IntEnum)):
                self.column_type = ColumnType.BIGINT
            elif issubclass(py_type, Struct):
                assert self.struct_type is not None, f"missing struct type for {self!r}"
                self.column_type = ColumnType.BYTES
            elif issubclass(py_type, Node):
                raise ValueError(f"cannot store node directly: {self!r}")
            else:
                column_type = PROPERTY_COLUMN_TYPE_BY_PY_TYPE.get(py_type)
                if column_type is None:
                    raise ValueError(f"cannot determine storage for {self!r}: {self.py_type_raw!r}")
                self.column_type = column_type

    def _contribute_ptrs(self) -> tuple["Property", ...]:
        """
        Contribute the wired and stored pointer properties required by this property..
        NOTE: contribute mutates this property
        """

        # wired/stored pointer settings for each reference kind
        if self.reference_kind == NodeReferenceKind.PARENT:
            is_wired = True
            is_stored = True
            is_required = False
            is_array = False
            is_computed = False
            on_delete = CascadeAction.CASCADE
        elif self.reference_kind == NodeReferenceKind.ANCESTOR:
            assert self.is_wired is not UNSET, f"must set is_wired on {self!r}"
            assert self.is_stored is not UNSET, f"must set is_stored on {self!r}"
            is_wired = self.is_wired
            is_stored = self.is_stored
            is_required = self.is_required
            is_array = False
            is_computed = True
            on_delete = CascadeAction.CASCADE
        elif self.reference_kind == NodeReferenceKind.REGULAR:
            assert self.is_required is not UNSET, f"must set is_required on {self!r}"
            assert self.is_array is not UNSET, f"must set is_array on {self!r}"
            is_wired = True
            is_stored = True
            is_required = False
            is_computed = False
            is_array = self.is_array
            on_delete = CascadeAction.SET_NULL
        else:
            raise ValueError(f"unexpected reference kind {self.reference_kind!r} for {self!r}")

        # contribute the properties
        if is_wired:
            self.reference_wired_ptr = Property(
                id=self.id,  # re-use id, self is not stored
                name=self.name + "_ptr",
                component=self.component,
                py_type_raw=NodeReferenceData,
                reference_kind=self.reference_kind,
                reference_types=self.reference_types,
                reference_source=self,
                is_runtime=True,
                is_wired=True,
                is_stored=False,
                is_computed=is_computed,
                is_array=is_array,
                is_required=is_required,
                column_type=None,
            )
        if is_stored:
            stored_ptr_props = []
            for ref_type in self.reference_types:
                store_as_id = (
                    self.reference_kind in (NodeReferenceKind.PARENT, NodeReferenceKind.ANCESTOR)
                    or ref_type not in IN_MODULE_NODE_TYPES
                    or ref_type == NodeType.MODULE
                )
                prop_postfix = "id" if store_as_id else "ck"
                if self.name == ref_type.name.lower():  # reduce clutter if type is unambiguous
                    prop_name = f"{self.name}_{prop_postfix}"
                else:
                    prop_name = f"{self.name}_{ref_type.name.lower()}_{prop_postfix}"
                stored_prop = Property(
                    id=self.id,
                    name=prop_name,
                    component=self.component,
                    py_type_raw=UUID,
                    reference_kind=self.reference_kind,
                    reference_types=(ref_type,),
                    reference_source=self,
                    reference_on_delete=on_delete,
                    is_runtime=False,
                    is_wired=False,
                    is_stored=True,
                    is_array=is_array,
                    is_required=is_required,
                    column_type=ColumnType.UUID,
                    is_indexed_in_pg=self.is_indexed_in_pg,
                )
                stored_ptr_props.append(stored_prop)
            self.reference_stored_ptrs = tuple(stored_ptr_props)

        # the runtime resolved pointer is not stored/wired directly
        self.is_wired = False
        self.is_stored = False

        return tuple(self.reference_ptrs)

    def new(self) -> Any:
        """Gets a new default value for this property"""
        if self.default is not UNSET:
            return self.default
        elif self.default_factory is not None:
            return self.default_factory()
        else:
            raise ValueError(f"no default for {self!r}")

    def copy(self, value: Any) -> Any:
        """Copies a non-None value of this property"""
        if self.is_tree_relation:
            raise ValueError(f"cannot copy relation {self!r}")
        elif self.reference_types:
            return value  # identity
        elif self.custom_copy is not None:
            return self.custom_copy(value)
        # auto-copy if it's trivial (primitives, immutable, enum, ...)
        elif isinstance(value, (type(None), bool, int, float, str, UUID, datetime, enum.Enum)):
            return value
        else:
            raise ValueError(f"cannot copy {self!r}")

    def validate(self, value: Any, on_issue: "PropertyValidationHandler") -> bool | None:
        """Validates a non-None value of this property"""
        if self.custom_validate is not None:
            return self.custom_validate(value, on_issue)
        else:
            return None


def struct_property(
    id: int,
    *,
    description: str = None,
    default: Any = UNSET,
    default_factory: Callable[[], Any] = None,
    copy: Callable[[Any], Any] = None,
    validate: Callable[[Any, "PropertyValidationHandler"], bool | None] = None,
    require: bool = UNSET,
    reflect: bool = UNSET,
    unique: bool = False,
    encrypt: bool = False,
    array: bool = UNSET,
    ignore_conflicts_with: tuple[type["Node"], ...] = None,
    references: tuple[NodeType, ...] | NodeType = None,
    struct_t: StructType = None,
    column_type: ColumnType = UNSET,
):
    """Standard user facing struct/node property."""
    return Property(
        id=id,
        description=description,
        default=default,
        default_factory=default_factory,
        custom_copy=copy,
        custom_validate=validate,
        is_required=require,
        is_reflected=reflect,
        ignore_conflicts_with=ignore_conflicts_with,
        reference_kind=NodeReferenceKind.REGULAR if references else None,
        reference_types=try_tuple(references),
        struct_type=struct_t,
        column_type=column_type,
        is_unique=unique,
        is_array=array,
        is_encrypted=encrypt,
    )


def struct_internal(
    id: int,
    *,
    description: str = None,
    default: Any = UNSET,
    default_factory: Callable[[], Any] = None,
    copy: Callable[[Any], Any] = None,
    require: bool = UNSET,
    reflect: bool = UNSET,
    ignore_conflicts_with: tuple[type["Node"], ...] = None,
    references: tuple[NodeType, ...] | NodeType = None,
    struct_t: StructType = None,
    store: bool = UNSET,
    column_type: ColumnType = UNSET,
    index_in_pg: bool = False,
    array: bool = UNSET,
    defer: bool = False,
    encrypt: bool = False,
    unique: bool = False,
    protect: bool = False,
):
    """Internal only struct/node property."""
    return Property(
        id=id,
        description=description,
        is_internal=True,
        is_protected=protect,
        is_required=require,
        default=default,
        default_factory=default_factory,
        custom_copy=copy,
        is_reflected=reflect,
        reference_kind=NodeReferenceKind.REGULAR if references else None,
        reference_types=try_tuple(references),
        ignore_conflicts_with=ignore_conflicts_with,
        is_stored=store,
        struct_type=struct_t,
        column_type=column_type,
        is_array=array,
        is_deferred=defer,
        is_encrypted=encrypt,
        is_indexed_in_pg=index_in_pg,
        is_unique=unique,
    )


def struct_runtime(
    *,
    default: Any = UNSET,
    default_factory: Callable[[], Any] = None,
    copy: Callable[[Any], Any] = None,
) -> object:
    """Internal runtime-only struct/node property (not persisted)."""
    return Property(
        is_internal=True,
        is_runtime_only=True,
        is_required=False,
        is_stored=False,
        default=default,
        default_factory=default_factory,
        custom_copy=copy,
    )


def node_parent(id: int, *node_type: NodeType):
    """The parent of a node, must be of one of the given types."""
    return Property(
        id=id,
        reference_kind=NodeReferenceKind.PARENT,
        reference_types=tuple(node_type),
        is_internal=True,
        is_stored=False,
    )


def node_ancestor(
    id: int,
    node_type: NodeType,
    nearest: bool = True,
    include_self: bool = True,
    store: bool = False,
    wire: bool = False,
    require: bool = UNSET,
    index_in_pg: bool = False,
):
    """Computed nearest or farthest ancestor of the given type."""
    return Property(
        id=id,
        reference_kind=NodeReferenceKind.ANCESTOR,
        reference_types=(node_type,),
        is_internal=True,
        is_computed=True,
        is_protected=True,
        is_ancestor_nearest=nearest,
        is_ancestor_self=include_self,
        is_stored=store,
        is_wired=wire,
        is_required=require,
        is_indexed_in_pg=index_in_pg,
    )


def node_children(
    node_type: NodeType,
    flags: NRel = NRel.DEFAULT,
    custom_list: type["NodeListBase"] = None,
    alias: str = None,
):
    """Computed read/write children or descendants of the given type."""
    return Property(
        reference_kind=NodeReferenceKind.CHILD,
        reference_types=(node_type,),
        children_flags=flags,
        is_internal=True,
        is_required=True,
        list_type=custom_list or NodeList,
        is_stored=False,
        alias=alias,
    )


class _ComponentMethod(enum.Enum):
    init = "init"
    walk = "walk"
    clear = "clear"
    index = "index"
    interp = "interp"
    visit = "visit"
    validate = "validate"
    activate = "activate"
    deactivate = "deactivate"
    attached = "attached"
    detached = "detached"
    updated = "updated"
    call = "call"
    iter = "iter"
    aiter = "aiter"
    len = "len"
    getitem = "getitem"

    @property
    def inner(self) -> str:
        return f"_{self.value}_inner"

    @property
    def self(self) -> str:
        return f"_{self.value}_self"

    @property
    def rec(self) -> str:
        return f"_{self.value}_rec"


# :NodeMethods
_NODE_INNER_METHODS: list[str] = [m.inner for m in _ComponentMethod]
_FORBIDDEN_NODE_METHODS = (
    [m.self for m in _ComponentMethod]
    + [m.rec for m in _ComponentMethod]
    + ["__post_init__", "__del__"]
)
NODE_CLASS_BY_TYPE: dict[NodeType, type["NodeT"]] = {}
NODE_COMPONENT_CLASS_BY_NAME: dict[str, type["Node"]] = {}
STRUCT_CLASS_BY_TYPE: dict[StructType, type["Struct"]] = {}
BENCH_CLASS_BY_TYPE: dict[BenchType, type["Node"] | type["Struct"]] = {}
_COMPONENT_METHODS: dict[[_ComponentMethod, type["Node"]], Any] = {}
_COMPONENT_CALL_ORDER: list[str] = [
    "Node",
    "ScopeNode",
    "HasFields",  # for resolved_fields
    # the rest
]


@cached(cache={})
def _sort_components_in_call_order(
    components: list[type["Node"]],
) -> list[type["Node"]]:
    """Sorts components by call order. Nodes without call order are left as-is."""
    sorted_components = []
    for component in components:
        if component.__name__ in _COMPONENT_CALL_ORDER:
            sorted_components.append(component)
    sorted_components.sort(key=lambda c: _COMPONENT_CALL_ORDER.index(c.__name__))
    for component in components:
        if component.__name__ not in _COMPONENT_CALL_ORDER:
            sorted_components.append(component)
    return sorted_components


@cached(cache={}, key=lambda components, method, concrete_key: f"{concrete_key}.{method.name}")
def _get_component_methods(
    components: Collection[type["Node"]], method: _ComponentMethod, concrete_key: str
) -> list[Any]:
    """Get the actually implemented methods in the given components in call order."""
    methods = []
    for component in _sort_components_in_call_order(components):
        if _COMPONENT_METHODS.get((method, component), None) is not None:
            methods.append(getattr(component, method.inner))
    return methods


METATYPE_PROPERTY = Property(
    id=1,
    name="metatype",
    default=None,
    py_type_raw=BenchType,
    is_internal=True,
    is_required=True,
    is_computed=True,  # is set statically in runtime
    is_runtime=False,
    is_wired=True,
    is_stored=False,
    column_type=ColumnType.STRING,
)


def _process_struct_base_cls(
    cls: Union[type["Node"], type["Struct"]],
    dynamic_components: tuple[type["Node"], ...] = (),
    reserved: set[str | int] = None,
    is_in_module: bool = False,
    is_in_bench: bool = False,
    is_final: bool = False,
) -> tuple[type["Node"], dict[str, Property]]:
    """Process a struct base class and return the processed class and its properties."""
    properties_by_name: dict[str, Property] = {METATYPE_PROPERTY.name: METATYPE_PROPERTY}
    static_components: list[type["Node"] | type["Struct"]] = [cls]

    # check that no forbidden methods are defined in non-base classes
    CORE_TYPES = ("Struct", "Node", "ScopeNode")
    if cls.__name__ not in CORE_TYPES:
        for name in _FORBIDDEN_NODE_METHODS:
            meth = getattr(cls, name, None)
            good_meths = (getattr(cls, name, None) for cls in (Struct, Node, ScopeNode))
            if meth is not None and meth not in good_meths:
                raise ValueError(f"forbidden method {name} defined in {cls}")

    # collect static components from class hierarchy
    for base in cls.__bases__:
        if base.__name__ in ("Struct", "Node", "ABC"):
            continue
        if hasattr(base, "__properties__"):
            base: type["Struct"]
            static_components.append(base)
            for gp in base.__static_components__:
                if gp.__name__ not in CORE_TYPES and gp not in static_components:
                    static_components.append(gp)
    if cls.__name__ not in ("Struct", "Node"):
        if issubclass(cls, Node):
            static_components.append(Node)
            static_components.append(Struct)
        elif issubclass(cls, Struct):
            static_components.append(Struct)
        else:
            raise ValueError(f"invalid struct base {cls}")

    # collect properties from this
    for name, prop in list(cls.__dict__.items()):
        if (
            name.startswith("__")
            or type(prop).__name__.startswith("_")
            or inspect.ismethod(prop)
            or inspect.isfunction(prop)
            or isinstance(prop, property)
            or isinstance(prop, classmethod)
            or isinstance(prop, staticmethod)
            or type(prop) == functools.cached_property
        ):
            continue  # ignore reserved names and non-fields
        if not isinstance(prop, Property):
            raise TypeError(f"{cls.__name__}.{name} is not a NodeProperty: {prop} ({type(prop)})")
        prop.name = name
        prop.component = cls
        prop.py_type_raw = cls.__annotations__.get(name, None)
        properties_by_name[name] = prop
        # collect any extra contributed properties
        if prop.reference_kind in (
            NodeReferenceKind.PARENT,
            NodeReferenceKind.ANCESTOR,
            NodeReferenceKind.REGULAR,
        ):
            for p in prop._contribute_ptrs():
                if p.name in properties_by_name:
                    raise ValueError(
                        f"property conflict '{p.name}': {p!r}, {properties_by_name[prop.name]!r}"
                    )
                properties_by_name[p.name] = p
                if not p.is_computed:
                    setattr(cls, p.name, p)
    cls.__own_properties__ = frozendict(properties_by_name)  # remember 'own' properties

    # collect properties from all components (static and dynamic, least to most specific)
    is_node = cls.__name__ != "Struct" and (
        cls.__name__ in ("Node", "ScopeNode") or issubclass(cls, Node)
    )
    cls.__properties__ = {**properties_by_name}  # start with own properties
    reserved_properties: set[str | int] = set(reserved or ())
    for component in chain(reversed(static_components), reversed(dynamic_components)):
        for name, prop in component.__own_properties__.items():
            existing = properties_by_name.get(name, None)
            # override parent & id with more specific values
            if existing is None or name.startswith("parent") or existing.id is UNSET:
                if prop.is_static or component not in dynamic_components:
                    prop = prop.clone()
                    prop.component = cls
                    properties_by_name[name] = prop
            elif not prop.equals_type(existing):
                if existing.ignore_conflicts_with and any(
                    issubclass(component, c) for c in existing.ignore_conflicts_with
                ):
                    continue
                raise ValueError(f"property conflict '{name}': {prop!r}, {existing!r}")
            if not is_node and prop.is_tree_relation:
                raise ValueError(f"non-node {cls} has node-only relation {prop}")
        reserved_properties.update(component.__reserved_properties__)
    cls.__reserved_properties__ = frozenset(reserved_properties)

    # create class (map to dataclass)
    for name, prop in list(properties_by_name.items()):
        # map property to class attribute or dataclass field
        if ((not is_in_bench or cls.__name__ == "Bench") and prop.name == "bench") or (
            (not is_in_module or cls.__name__ == "Module") and prop.name == "module"
        ):
            # remove 'bench'/'module' ancestor property if not actually a descendant :MagicNodeProps
            attr = None
            del properties_by_name[name]
            # remove contributed reference keys too
            for key in prop.reference_ptrs:
                del properties_by_name[key.name]
        elif prop.name == "ck" and is_node and not is_in_module:
            # remove node ck (is == id if outside a module) :MagicNodeProps
            attr = _node_ck_from_id_prop(prop)
            del properties_by_name[name]
        elif not is_final:
            attr = None  # only set attributes in final class
        elif prop.reference_kind == NodeReferenceKind.ANCESTOR and cls.__name__ not in (
            "ScopeNode",
            "Node",
        ):
            attr = _node_computed_ancestor_prop(prop)
        elif prop.is_computed or not prop.is_runtime:
            attr = UNSET
        elif prop.default is not UNSET:
            attr = dataclasses.field(default=prop.default)
        elif prop.default_factory is not None:
            attr = dataclasses.field(default_factory=prop.default_factory)
        else:
            attr = required_field()
        # set attribute and annotation accordingly
        if attr is not UNSET:
            setattr(cls, name, attr)
        if isinstance(attr, dataclasses.Field):
            cls.__annotations__[name] = prop.py_type_raw
        elif name in cls.__annotations__:
            del cls.__annotations__[name]
        # also set extra computed reference properties
        if prop.reference_kind and not prop.reference_source and prop.reference_wired_ptr:
            for computed_attr in ("id", "ck"):
                computed_prop = _node_computed_attr(computed_attr, prop, prop.reference_wired_ptr)
                setattr(cls, prop.name + "_" + computed_attr, computed_prop)

    # nocheckin: use slots for struct/node classes
    cls = dataclass(cls, repr=False, eq=False)  # type: ignore

    # collect methods implemented in this class (specifically)
    for meth_type in _ComponentMethod:
        meth = getattr(cls, meth_type.inner, None)
        if meth is not None and not any(
            meth is getattr(base, meth_type.inner, None) for base in cls.__bases__
        ):
            _COMPONENT_METHODS[(meth_type, cls)] = meth

    # register components and index properties
    cls.__static_components__ = tuple(static_components)
    cls.__dynamic_components__ = tuple(dynamic_components or ())
    cls.__properties__ = frozendict(properties_by_name)
    properties_by_id: dict[int, Property] = {}
    for prop in properties_by_name.values():
        if prop.id is not None and prop.is_wired and not prop.reference_wired_ptr:
            existing = properties_by_id.get(prop.id, None)
            if existing is not None:
                raise ValueError(f"property id conflict: {prop!r}, {existing!r}")
            properties_by_id[prop.id] = prop
    props = properties_by_name.values()
    cls.__properties_by_id__ = frozendict(properties_by_id)
    cls.__tracked_properties__ = frozendict({p.name: p for p in props if not p.is_internal})
    cls.__internal_properties__ = frozendict({p.name: p for p in props if p.is_internal})
    cls.__reference_properties__ = frozendict({p.name: p for p in props if p.reference_types})
    cls.__struct_properties__ = frozendict({p.name: p for p in props if p.is_struct})

    return cls, properties_by_name


@dataclass_transform()
def struct_component(
    cls: Optional[type] = None,
    struct_type: StructType = None,
    reserved: set[str | int] = None,
    is_final: bool = False,
):
    """
    Mark a class as a struct component (or concrete struct for a StructType).
    """

    def decorate(cls):
        cls, properties = _process_struct_base_cls(cls=cls, reserved=reserved, is_final=is_final)

        # register struct
        if struct_type:
            cls.metatype = struct_type
            if struct_type in STRUCT_CLASS_BY_TYPE:
                raise ValueError(
                    f"struct class conflict for {struct_type}: {cls}, {STRUCT_CLASS_BY_TYPE[struct_type]}"
                )
            STRUCT_CLASS_BY_TYPE[struct_type] = cls
        return cls

    if cls is not None:
        return decorate(cls)
    return decorate


def struct(
    struct_type: StructType,
    reserved: set[str | int] = None,
    index_in_os: bool = False,
):
    def decorate(cls):
        cls = struct_component(cls, struct_type=struct_type, reserved=reserved, is_final=True)
        cls.__is_indexed_in_os__ = index_in_os
        return cls

    return decorate


@dataclass_transform()
def node_component(
    cls: Optional[type] = None,
    node_type: NodeType = None,
    passthrough: tuple[tuple[str, "_Passthrough"]] = (),
    dynamic_components: tuple[type["Node"], ...] = (),
    reserved: set[str | int] = None,
    is_in_module: bool = False,
    is_in_bench: bool = False,
    is_final: bool = False,
):
    """
    Mark a class as a node component (or concrete node for a NodeType).
    """

    def decorate(cls):
        cls, properties = _process_struct_base_cls(
            cls=cls,
            dynamic_components=dynamic_components,
            reserved=reserved,
            is_in_module=is_in_module,
            is_in_bench=is_in_bench,
            is_final=is_final,
        )
        cls.__static_passthrough__ = passthrough
        # register node properties
        props = properties.values()
        list_properties: dict[str, Property] = {}
        list_properties_by_child: dict[NodeType, list[Property]] = defaultdict(list)
        for prop in properties.values():
            if prop.reference_kind == NodeReferenceKind.CHILD:
                if (
                    cls.__name__ != "ScopeNode"
                    and not issubclass(cls, ScopeNode)
                    and node_type is not None
                ):
                    raise ValueError(f"{cls} is not a ScopeNode for {prop}")
                list_properties[prop.name] = prop
                for ref_t in prop.reference_types:
                    list_properties_by_child[ref_t].append(prop)
        cls.__list_properties__ = frozendict(list_properties)
        cls.__list_properties_by_child__ = frozendict(list_properties_by_child)
        cls.__ancestor_properties__ = frozendict(
            {p.name: p for p in props if p.reference_kind == NodeReferenceKind.ANCESTOR}
        )

        # register as concrete node class for node_type
        if node_type:
            cls.metatype = node_type
            if node_type in NODE_CLASS_BY_TYPE:
                raise ValueError(
                    f"node class conflict for {node_type}: {cls}, {NODE_CLASS_BY_TYPE[node_type]}"
                )
            NODE_CLASS_BY_TYPE[node_type] = cls
        NODE_COMPONENT_CLASS_BY_NAME[cls.__name__] = cls

        return cls

    if cls is not None:
        return decorate(cls)

    return decorate


def node(
    node_type: NodeType,
    passthrough: tuple[tuple[str, "_Passthrough"]] = (),
    dynamic_components: tuple[type["Node"], ...] = (),
    stored: bool = True,
    stored_custom: bool = False,
    index_in_os: bool = False,
    local: bool = False,
    root: NodeType | None = NodeType.BENCH,
    in_module: bool = True,
    in_bench: bool = True,
    reserved: set[str | int] = None,
    indexes: tuple[Index, ...] = (),
    constraints: tuple[Constraint, ...] = (),
    unique_together: tuple[tuple[str, ...], ...] = (),
):
    """Register a class as a concrete node for the given node type."""

    def decorate(cls):
        cls = node_component(
            cls,
            node_type=node_type,
            passthrough=passthrough,
            dynamic_components=dynamic_components,
            reserved=reserved,
            is_in_module=in_module,
            is_in_bench=in_bench,
            is_final=True,
        )
        cls.__is_stored__ = stored
        cls.__is_stored_custom__ = stored_custom
        cls.__is_indexed_in_os__ = index_in_os
        cls.__is_local__ = local

        extra_indexes: list[Index] = [*indexes]
        extra_constraints: list[Constraint] = [*constraints]
        for columns in unique_together:
            index_name = f"bench_idx_{'_'.join(columns)}"
            index = Index(index_name, type=IndexType.BTREE, is_unique=True, columns=columns)
            constraint = Constraint(
                index.inner_name,
                type=ConstraintType.UNIQUE,
                columns=columns,
                index=index.inner_name,
            )
            extra_indexes.append(index)
            extra_constraints.append(constraint)

        cls.__extra_indexes__ = tuple(extra_indexes)
        cls.__extra_constraints__ = tuple(extra_constraints)

        parent_property = cls.__properties__.get("parent", None)
        if parent_property is None:
            raise ValueError(f"node {cls} has no parent property")
        cls.__parent_property__ = parent_property
        cls.__root__ = root
        cls.__is_in_module__ = in_module
        cls.__is_in_bench__ = in_bench

        return cls

    return decorate


NodeT = TypeVar("NodeT", bound="Node")


def _node_ck_from_id_prop(prop: Property) -> property:
    """Get ck from id (read-only)."""

    def get(self: NodeT) -> UUID:
        return self.id

    def set(self: NodeT, value: UUID):
        raise NotImplementedError(f"cannot set computed property {prop!r}: {value!r}")

    return property(get, set)


def _node_computed_ancestor_prop(prop: Property) -> property:
    """Computed ancestor property for Node instances."""

    if prop.is_ancestor_nearest:

        def get_nearest(self: NodeT) -> Optional[NodeT]:
            parent = self if prop.is_ancestor_self else self.parent
            while parent is not None:
                if parent.metatype in prop.reference_types:
                    return parent
                parent = parent.parent
            return None

        get = get_nearest
    else:

        def get_farthest(self: NodeT) -> Optional[NodeT]:
            parent = self if prop.is_ancestor_self else self.parent
            farthest = None
            while parent is not None:
                if parent.metatype in prop.reference_types:
                    farthest = parent
                parent = parent.parent
            return farthest

        get = get_farthest

    def set(self: NodeT, value: NodeT):
        raise NotImplementedError(f"cannot set computed property {prop!r}: {value!r}")

    return property(get, set)


def _node_computed_attr(x: str, prop: Property, backup_prop: Property) -> property:
    """Computed value from another property. If prop is not set, use backup prop."""

    def get(self: NodeT) -> Optional[Any]:
        reference = getattr(self, prop.name)
        if reference is not None:
            return getattr(reference, x)
        reference = getattr(self, backup_prop.name)
        if reference is not None:
            return getattr(reference, x)
        return None

    def set(self: NodeT, value: Any):
        raise NotImplementedError(f"cannot set computed property {prop!r}: {value!r}")

    return property(get, set)


def _make_self_method(
    method: _ComponentMethod,
    wraps,
    from_status: NodeStatus = None,
    to_status: NodeStatus = None,
):
    """Creates method that calls _method_inner for all components in call order"""

    @functools.wraps(wraps)
    def self_method(self: "Struct", *args, _coerce: bool = True, _ignore: bool = False, **kwargs):
        if from_status is not None and self._status != from_status:
            if not _coerce:
                raise RuntimeError(f"cannot {method.name} {self!r} (status={self._status.name})")
            # auto coerce the node into the desired to_status if allowed and feasible
            if isinstance(self, Node):
                if self._status == NS.SOURCE and to_status > NS.INDEX:
                    self._index_self()
                if self._status == NS.INDEX and to_status > NS.INTERP:
                    self._interp_self(self, on_issue=self.scope._on_issue)
            else:  # Struct
                if self._status == NS.SOURCE and to_status > NS.INTERP:
                    # where to get struct scope? track 'parent node' in struct? :StructScope
                    self._interp_self(self, on_issue_raise)
            if from_status <= to_status <= self._status or from_status >= to_status >= self._status:
                return  # nothing to do
            if self._status < from_status:
                raise RuntimeError(
                    f"cannot coerce {method.name} {self!r} (status={self._status.name})"
                )

        for meth in _get_component_methods(self._components, method, self._instance_cache_key):
            meth(self, *args, **kwargs)
        if to_status is not None:
            self._status = to_status

    self_method.__name__ = method.self
    return self_method


def _make_inner_dunder_method(method: _ComponentMethod):
    """Creates method that proxies a builtin dunder method to the first _method_inner"""

    def inner_method(self: "Node", *args, **kwargs):
        meths = _get_component_methods(self._components, method, self._instance_cache_key)
        if len(meths) <= 1:  # includes this one
            raise RuntimeError(f"{self!r} does not support {method.name}")
        return meths[1](self, *args, **kwargs)

    inner_method.__name__ = method.inner
    return inner_method


class _Passthrough(enum.StrEnum):
    Full = "full"
    Scope = "scope"


@struct_component
class Struct(abc.ABC):
    """
    A non-node data structure, usually inside a node (which is the only way to store/retrieve it).
    Will activate, track, etc. when we start using these in nodes.
    """

    metatype: ClassVar[StructType]  # type discriminator is field 0 if needed?
    __static_components__: ClassVar[tuple[type["Node"], ...]] = []
    __dynamic_components__: ClassVar[tuple[type["Node"], ...]] = ()
    __properties__: ClassVar[dict[str, Property]] = {}
    __own_properties__: ClassVar[dict[str, Property]] = {}
    __properties_by_id__: ClassVar[dict[int, Property]] = {}
    __tracked_properties__: ClassVar[dict[str, Property]] = {}
    __internal_properties__: ClassVar[dict[str, Property]] = {}
    __reference_properties__: ClassVar[dict[str, Property]] = {}
    __struct_properties__: ClassVar[dict[str, Property]] = {}
    __stored_properties__: ClassVar[dict[str, Property]] = {}
    __wired_properties__: ClassVar[dict[str, Property]] = {}
    __reserved_properties__: ClassVar[set[int | str]] = set()
    __is_indexed_in_os__: ClassVar[bool] = False  # stored in local OS (only for logs really)

    _status: NodeStatus = struct_runtime(default=None)

    def __post_init__(self):
        if self._status is None:
            from bench.language.session import _active_session

            # not sure if this is totally right... where do we get :StructScope?
            self._status = NS.INTERP if _active_session.get() else NS.SOURCE
        self._init_self()

    @final
    def __str__(self):
        return self.__content_str__()

    def __content_str__(self) -> str:
        return ""

    def __repr__(self):
        return f"<{self.__class__.__name__} {str(self)}>"

    @classmethod
    def _get_property(cls, ptr: "PropertyReference") -> Property | None:
        if not ptr.references_type:
            return cls.__properties_by_id__.get(ptr.id, None)
        else:
            for prop in cls.__stored_properties__.values():
                if prop.id == ptr.id and prop.reference_types[0] == ptr.references_type:
                    return prop
            return None

    @classmethod
    def _resolve_property(cls, ptr: "PropertyReference") -> Property | None:
        prop = cls._get_property(ptr)
        if prop is None:
            raise ValueError(f"unknown property pointer: {ptr!r} in {cls!r}")
        return prop

    @property
    def _components(self) -> tuple[type["Node"], ...]:
        return self.__static_components__

    @property
    def _instance_cache_key(self) -> str:
        """Identifier for dynamic components"""
        return type(self).__name__

    def __eq__(self, other):
        return self is other  # structs have no 'real' identity

    def _set_untracked(self, key, value):
        self.__dict__[key] = value

    # TODO @Broken: track in-struct edits (__setattr__) :StructScope

    def _init_inner(self):
        # init reference pointers if references are set :NodeReferences
        for prop in self.__reference_properties__.values():
            ref = getattr(self, prop.name)
            if prop.reference_wired_ptr is not None and isinstance(ref, Node):
                from bench.language.expression import NodeReference

                self.__dict__[prop.reference_wired_ptr.name] = NodeReference.from_node(ref)

    def _clear_inner(self, scope: Optional["ScopeNode"] = None):
        # clear node references :NodeReferences
        scope_tree = scope._local_tree if scope is not None else None
        for prop in self.__reference_properties__.values():
            if scope_tree is not None:  # if scope is set only clear nodes in scope
                val = getattr(self, prop.name)
                if val is None or val.ck not in scope_tree:
                    continue
            # TODO @Broken?: reset node references in clear for real (if still needed)
            # setattr(self, prop.name, None)

    def _interp_inner(self, scope: "ScopeNode", on_issue: "IssueHandler"):
        # resolve node references :NodeReferences
        for prop in self.__reference_properties__.values():
            if getattr(self, prop.name, None) is not None:
                continue  # already resolved
            ptr = getattr(self, prop.reference_wired_ptr.name)
            if ptr is not None:
                resolved = scope.resolve(ptr.id or ptr.ck)
                if resolved is None:
                    on_issue(type=IssueType.MISSING_REFERENCE, subject=self, path=prop.name)
                setattr(self, prop.name, resolved)

    def _visit_inner(self, visitor: "NodeVisitor"):
        # visit node references :NodeReferences
        for prop in self.__reference_properties__.values():
            value = getattr(self, prop.name)
            if isinstance(value, Node):
                visitor.visit_reference(value)

    def _validate_inner(self, properties: Collection[str], on_invalid: "ValidationHandler") -> None:
        """Validate cross-property constraints given the modified properties."""
        # since this is the root module, we also validate the properties directly
        from bench.language.builtin import _should_validate

        if LOCAL_ENV and not _should_validate():
            return  # escape hatch for testing
        for name in properties:
            prop = self.__properties__.get(name)
            assert prop is not None, f"unknown property '{name}' on {self!r}"
            value = getattr(self, name)
            if value is None:
                if prop.is_required:
                    on_invalid(self, f"{prop.name}: is required", [prop.name])
            elif prop.custom_validate is not None:
                handler = PropertyValidationHandler(self, prop, on_invalid)
                valid = prop.validate(value, handler)
                if valid is False:
                    on_invalid(self, f"{prop.name}: invalid value", [prop.name])

    # struct has basic set of lifecycle methods (no index because no scope)
    _init_self = _make_self_method(_ComponentMethod.init, _init_inner)
    _clear_self = _make_self_method(_ComponentMethod.clear, _clear_inner, NS.INTERP, NS.SOURCE)
    _interp_self = _make_self_method(_ComponentMethod.interp, _interp_inner, NS.SOURCE, NS.INTERP)
    _visit_self = _make_self_method(_ComponentMethod.visit, _visit_inner)
    _validate_self = _make_self_method(_ComponentMethod.validate, _validate_inner)

    def _walk_self(self) -> Iterable["Struct"]:
        yield self
        for prop in self.__struct_properties__.values():
            value = getattr(self, prop.name)
            if isinstance(value, list):
                for item in value:
                    yield from item._walk_self()
            elif value is not None:
                yield from value._walk_self()

    @staticmethod
    def _make_rec_method(method: _ComponentMethod, wraps):
        """Creates method that calls _method_self for all contained structs"""

        @functools.wraps(wraps)
        def rec_method(self: "Node", *args, **kwargs):
            # just walk self, every struct can only appear once
            for descendant in self._walk_self():
                getattr(descendant, method.self)(*args, **kwargs)

        rec_method.__name__ = method.rec
        return rec_method

    _clear_rec = _make_rec_method(_ComponentMethod.clear, _clear_self)
    _interp_rec = _make_rec_method(_ComponentMethod.interp, _interp_self)
    _visit_rec = _make_rec_method(_ComponentMethod.visit, _visit_self)


@node_component
class Node(Struct, _NodeExpressionBase):
    """
    A node in a Bench module tree - basically struct + identity, so it can relate nodes.
    A node has a per-version unique id (id) and a constant identifier key (ck).
    The id is derived from the module id, so it's only assigned when the node is attached.
    """

    metatype: ClassVar[NodeType]
    __static_components__: ClassVar[tuple[type["Node"], ...]] = []
    __dynamic_components__: ClassVar[tuple[type["Node"], ...]] = ()
    __static_passthrough__: ClassVar[tuple[tuple[str, _Passthrough]]] = ()

    __properties__: ClassVar[dict[str, Property]] = {}
    __own_properties__: ClassVar[dict[str, Property]] = {}
    __properties_by_id__: ClassVar[dict[int, Property]] = {}
    __ancestor_properties__: ClassVar[dict[str, Property]] = {}
    __list_properties__: ClassVar[dict[str, Property]] = {}
    __list_properties_by_child__: ClassVar[dict[NodeType, tuple[Property, ...]]] = defaultdict(list)
    __tracked_properties__: ClassVar[dict[str, Property]] = {}
    __internal_properties__: ClassVar[dict[str, Property]] = {}
    __reference_properties__: ClassVar[dict[str, Property]] = {}
    __struct_properties__: ClassVar[dict[str, Property]] = {}
    __stored_properties__: ClassVar[dict[str, Property]] = {}
    __wired_properties__: ClassVar[dict[str, Property]] = {}
    __reserved_properties__: ClassVar[set[int | str]] = set()
    __parent_property__: ClassVar[Property] = None

    __has_scope__: ClassVar[bool] = False  # can have node children
    __root__: ClassVar[NodeType | None] = UNSET
    __is_in_module__: ClassVar[bool] = UNSET  # part of a Module
    __is_in_bench__: ClassVar[bool] = UNSET  # part of a Bench
    __is_stored__: ClassVar[bool] = False  # stored in PG (runtime or local)
    __is_stored_custom__: ClassVar[bool] = False  # custom PG storage logic (for records)
    __is_indexed_in_os__: ClassVar[bool] = False  # stored in local OS
    __is_local__: ClassVar[bool] = False  # stored in Bench-local DB (instead of global Bench DB)
    __extra_indexes__: ClassVar[tuple[Index, ...]] = ()  # extra indexes for PG
    __extra_constraints__: ClassVar[tuple[Constraint, ...]] = ()  # extra constraints for PG
    __table__: ClassVar[Table] = UNSET  # if stored regularly, set after finalization

    # 1-9: reserved for node identity
    id: UUID = struct_internal(2, default=None, require=True, protect=True)
    # NOTE: ck/module/branch/bench only exist if __is_in_module__/__is_in_bench__ :MagicNodeProps
    ck: UUID = struct_internal(3, default=None, require=True, protect=True)
    parent: Optional["Node"] = node_parent(4)
    # prototype/template: Optional["Node"] = node_template(5)
    module: "Module" = node_ancestor(6, NodeType.MODULE, require=True, store=True, wire=True)
    # branch: Optional["Branch"] = node_ancestor(7, NodeType.BRANCH, require=True, store=True, wire=True)
    bench: "Bench" = node_ancestor(8, NodeType.BENCH, require=True, store=False, wire=False)
    source: NodeSource = struct_internal(
        9, default=NodeSource.PERSISTED, store=False, require=True, protect=True
    )

    # 10-29: reserved for node tracking
    revision: int = struct_internal(10, default=0, require=True, protect=True)
    created_at: datetime = struct_internal(11, default=None, require=True, protect=True)
    updated_at: datetime = struct_internal(12, default=None, require=True, protect=True)
    deleted_at: Optional[datetime] = struct_internal(13, default=None, protect=True)
    archived_at: Optional[datetime] = struct_internal(14, default=None, protect=True)
    last_edited_at: Optional[datetime] = struct_internal(
        15, default=None, require=True, protect=True
    )
    # only scope nodes can have 'inner' changes
    # last_changed_at: datetime = struct_internal(16, default=None)
    # created_by: ... = struct_internal(17, default=None)
    # last_edited_by: ... = struct_internal(18, default=None)
    # last_changed_by: ... = struct_internal(19, default=None)
    # only some nodes have further constraints
    # visibility: ... = struct_internal(20, default=None)
    # policies: ... = struct_internal(21, default=None, struct_t=StructType.POLICY)

    # 30+ for 'user' node/struct properties
    # <... defined in concrete type ...>

    _session: Optional["Session"] = struct_runtime(default=None)
    _status: NodeStatus = struct_runtime(default=None)
    _track: NodeTrackingLevel = struct_runtime(default=NodeTrackingLevel.FULL)
    _new: bool = struct_runtime(default=False)
    _deferred_properties: tuple[str, ...] | None = struct_runtime(default=None)

    def __post_init__(self):
        # init ck/id
        if self.__is_in_module__:
            if self.ck is None:
                self.ck = uuid4()
                self._new = True
            if self.id is None and self.attached:
                self._assign_id(self.module.id)
        elif self.id is None:
            self.id = uuid4()
            self._new = True
        # init tracking
        if self.created_at is None:
            # init cru timestamps
            now = utcnow_with_tz()
            self.created_at = now
            self.updated_at = now
            self.last_edited_at = now
            self.last_changed_at = now
        # get session
        if self._session is None and self._session is not UNSET:
            from bench.language.builtin import _active_session

            self._session = _active_session.get()
        if self._session and self._session is not UNSET and self._new and not self.parent:
            self._session._dangling_nodes_by_ck[self.ck] = self
        # init status
        if self._status is None:
            self._status = NS.INTERP if self._session is not None else NS.SOURCE
        self._init_self()
        if self._status == NS.INTERP and self._session is not None:
            self._activate_self(self._session)

    @property
    def _components(self) -> tuple[type["Node"], ...]:
        return self.__static_components__

    @property
    def _dynamic_components(self) -> tuple[type["Node"], ...]:
        return ()

    @property
    def _instance_cache_key(self) -> str:
        """Identity for dynamic components"""
        return type(self).__name__

    @property
    def _passthrough_targets(self) -> tuple[tuple[str, _Passthrough]] | None:
        """Pass through __getattr__/__setattr__ properties (before defaulting to usual)"""
        return self.__static_passthrough__

    @property
    def _local_root(self) -> "Node":
        parent = self
        while parent.parent is not None:
            parent = parent.parent
        return parent

    @property
    def _local_root_scope(self) -> "ScopeNode":
        assert self.scope is not None, f"{self!r} has no parent"
        return self.scope._local_root_scope

    @property
    def _local_root_tree(self) -> "NodeTreeBase":
        return self._local_root._local_tree

    def _assign_id(self, module_id: UUID):
        assert module_id, f"cannot assign id to {self!r} without a module id"
        assert self.id is None, f"cannot assign id to {self!r} twice"
        assert self.ck is not None, f"cannot assign id to {self!r} without ck"
        self.id = get_node_id(module_id, self.ck)

    @final
    def __str__(self):  # noqa: we want to override the default __str__ for nodes
        content_str = self.__content_str__()
        ident_str = self.ident
        if ident_str is None:
            ident_str = str(self.id)
        if content_str:
            content_str = f" ({content_str})"
        if self.__parent_property__ is None:
            return f"{ident_str}{content_str}"
        elif self.parent is None:
            return f"<detached>.{ident_str}{content_str}"
        else:
            path_segments: list[str] = []
            parent = self.parent
            while (
                parent is not None and parent.metatype != NodeType.BENCH
            ):  # skip bench (same path as pkg)
                path_segments.append(parent.ident)
                parent = parent.parent
            path_segments.reverse()
            path = ".".join(path_segments)
            return f"{path}.{ident_str}{content_str}"

    @final
    def __repr__(self):
        return f"<{self.__class__.__name__} {str(self)}>"

    @property
    def attached(self) -> bool:
        if self.__is_in_module__:
            return self.parent is not None and self.module is not None
        elif self.__is_in_bench__:
            return self.parent is not None and self.bench is not None

    @property
    def scope(self) -> Optional["ScopeNode"]:
        return self.parent

    @property
    def ident(self) -> Optional[str]:
        return None

    @property
    def path(self) -> str:
        raise NotImplementedError(f"{self.__class__.__name__} does not implement path")

    @property
    def session(self) -> "Session":
        """Access the session, error-ing if there is none."""
        if self._session is None:
            raise RuntimeError(f"no active session for {self!r}")
        return self._session

    @session.setter
    def session(self, session: Optional["Session"]):
        self._session = session

    def __eq__(self, other):
        return isinstance(other, self.__class__) and self.id == other.id and self.ck == other.ck

    def __hash__(self):
        return hash(self.id)

    def __setattr__(self, key, value):
        if self._status != NS.ACTIVE:
            return super().__setattr__(key, value)

        # tracked set
        prop = self.__properties__.get(key)
        if prop is not None:
            if prop.reference_kind == NodeReferenceKind.CHILD:
                return getattr(self, key).set(value)
            elif prop.is_internal:
                return super().__setattr__(key, value)
            else:
                prev = getattr(self, key)
                self.__dict__[key] = value
                try:
                    self._validate_self([key], on_invalid=on_invalid_raise)
                except ValidationError as e:  # reset on error
                    self.__dict__[key] = prev
                    raise e
                if prop.reference_wired_ptr:  # update reference pointer  :NodeReferences
                    from bench.language.expression import NodeReference

                    self.__dict__[prop.reference_wired_ptr.name] = NodeReference.from_node(value)
                    if self.attached:
                        self._session.update(self, [key])
                        self._updated_self((prop.reference_wired_ptr.name,))
                elif self.attached:
                    self._session.update(self, [key])
                    self._updated_self((key,))
                return
        elif key in self.__dict__:
            self.__dict__[key] = value
            return

        # try first full passthrough target (if any)
        for target, mode in self._passthrough_targets:
            target = getattr(self, target)
            if mode == _Passthrough.Full:
                setattr(target, key, value)
                return  # success

        # report set error with additional info
        candidates = {
            **(self.__tracked_properties__ if self._status == NS.ACTIVE else self.__properties__),
            **{s.name: s for s in self._scopes_by_name.values()},
        }
        did_you_mean = did_you_mean_str(candidates, key)
        raise AttributeError(f"Cannot set '{key}' on {self!r}. {did_you_mean}")

    def __getattr__(self, item):
        if item in self.__dict__:  # 'native' property or method
            return self.__dict__[item]

        attr = UNSET
        # prefer components methods
        for component in self._components:
            attr = getattr(component, item, UNSET)
            if attr is not UNSET:
                break
        # check passthrough targets if tracked in session
        if attr is UNSET and self._session is not None:
            for target, mode in self._passthrough_targets:
                target = getattr(self, target)
                if mode == _Passthrough.Full:
                    attr = getattr(target, item, UNSET)
                elif mode == _Passthrough.Scope:
                    assert isinstance(target, NodeList), f"invalid scope passthrough: {attr!r}"
                    attr = target.get(item) or UNSET
                if attr is not UNSET:
                    break
        # attribute could be property, method, or just plain value
        if attr is not UNSET:
            if isinstance(attr, property):
                return attr.fget(self)
            elif not isinstance(attr, Node) and callable(attr) and not inspect.ismethod(attr):
                return functools.partial(attr, self)
            else:
                return attr

        # report lookup error with additional info
        candidates = {k: v for k, v in self.__properties__.items() if not k.startswith("_")}
        if isinstance(self, ScopeNode):
            candidates.update(self._scopes_by_name)
        did_you_mean = did_you_mean_str(candidates, item)
        raise AttributeError(f"{self!r} has no attribute '{item}'. {did_you_mean}")

    def _walk_structs(self) -> Iterable["Struct"]:
        for prop in self.__struct_properties__.values():
            value = getattr(self, prop.name)
            if value is not None:
                yield from value._walk_self()

    # abstract :ComponentMethods in addition to Struct

    def _index_inner(self) -> None:
        """Index this node."""
        pass

    def _clear_inner(self, scope: Optional["ScopeNode"] = None):
        """Clear this node."""
        # clear all structs recursive
        for s in self._walk_structs():
            s._clear_rec()

    def _interp_inner(self, scope: "ScopeNode", on_issue: "IssueHandler"):
        """Interp this node."""
        # interp all structs recursive
        for s in self._walk_structs():
            s._interp_rec(scope, on_issue)

    def _activate_inner(self, session: "Session") -> None:
        """'Instantiate' this object in the given session."""
        self._session = session
        self._status = NS.ACTIVE

    def _deactivate_inner(self) -> None:
        """'Deinstantiate' this object."""
        self._session = None

    def _attached_inner(self) -> None:
        """Called when this node is attached to a module."""
        pass

    def _detached_inner(self) -> None:
        """Called when this node is detached from a module."""
        pass

    def _updated_inner(self, properties: Collection[str]) -> None:
        """Called when this node is updated."""
        pass

    _call_inner = _make_inner_dunder_method(_ComponentMethod.call)
    _iter_inner = _make_inner_dunder_method(_ComponentMethod.iter)
    _aiter_inner = _make_inner_dunder_method(_ComponentMethod.aiter)
    _len_inner = _make_inner_dunder_method(_ComponentMethod.len)
    _getitem_inner = _make_inner_dunder_method(_ComponentMethod.getitem)

    __call__ = _call_inner
    __iter__ = _iter_inner
    __aiter__ = _aiter_inner
    __len__ = _len_inner
    __getitem__ = _getitem_inner

    def __bool__(self):
        return True  # allow truthy checks for nodes

    # final :ComponentMethods

    def _init_self(self):
        # init lists
        existing_lists: dict[str, Any] | None = None
        for name, prop in self.__list_properties__.items():
            existing = getattr(self, name, None)
            node_list = prop.list_type(self, prop)
            setattr(self, name, node_list)
            if prop.alias:
                setattr(self, prop.alias, node_list)
            if existing and not isinstance(existing, NodeList):
                if existing_lists is None:
                    existing_lists = {}
                existing_lists[name] = existing

        # run actual init methods
        for meth in _get_component_methods(
            self._components, _ComponentMethod.init, self._instance_cache_key
        ):
            meth(self)

        # keep manually set node lists if passed in
        if existing_lists:
            changed_nodes: list[NodeT] = []
            was_interp = self._status >= NS.INTERP
            detach_trigger = _NC.Detach if was_interp else _NC.Ignore
            for name, existing in existing_lists.items():
                if existing and not isinstance(existing, NodeList):
                    getattr(self, name).extend(*existing, _trigger=detach_trigger)
                    changed_nodes.extend(existing)
            if changed_nodes and was_interp:
                _InterpChange._collect(None, self, changed_nodes, _NC.Attach)._effect(_NC.Attach)

        # validate if in session after all init are done
        if self._status >= NS.INTERP and self._session and self._session is not UNSET:
            self._validate_self(self.__tracked_properties__.keys(), on_invalid=on_invalid_raise)

    # node has extended set of lifecycle methods
    _index_self = _make_self_method(_ComponentMethod.index, _index_inner, NS.SOURCE, NS.INDEX)
    _clear_self = _make_self_method(_ComponentMethod.clear, _clear_inner, NS.INTERP, NS.SOURCE)
    _interp_self = _make_self_method(_ComponentMethod.interp, _interp_inner, NS.SOURCE, NS.INTERP)
    _activate_self = _make_self_method(
        _ComponentMethod.activate, _activate_inner, NS.INTERP, NS.ACTIVE
    )
    _deactivate_self = _make_self_method(
        _ComponentMethod.deactivate, _deactivate_inner, NS.ACTIVE, NS.INTERP
    )
    _attached_self = _make_self_method(_ComponentMethod.attached, _attached_inner)
    _detached_self = _make_self_method(_ComponentMethod.detached, _detached_inner)
    _updated_self = _make_self_method(_ComponentMethod.updated, _updated_inner)

    def _copy_self(self, keep_parent: bool = False, reset_id: bool = True) -> "Node":
        """
        Copies this node without any descendants.
        All non-relational properties are copied using NodeProperty.copy, relations are reset.
        """
        props = {}
        for name, prop in self.__properties__.items():
            if prop.is_tree_relation or prop.is_computed:
                continue
            props[name] = prop.copy(getattr(self, name))
        if keep_parent:
            props["parent"] = self.parent
        if reset_id:
            props["id"] = None
            props["ck"] = uuid.uuid4()
        copy = self.__class__(**props)
        return copy

    def _walk_rec(self) -> Collection["Node"]:
        """
        Walks this node and all descendants in breadth-first order.
        """
        return [self]

    def _on_issue(self, subject: "Node", type: IssueType, message: str = None, **kwargs) -> None:
        # only scope nodes can host issues, forward to parent
        self.parent._on_issue(subject=self, type=type, message=message, **kwargs)


def _make_rec_method(
    method: _ComponentMethod, wraps, custom_kwargs: Callable[["Node"], dict] = None
):
    """Creates method that calls _method_self for self and all descendants"""

    @functools.wraps(wraps)
    def rec_method(self: "ScopeNode", *args, **kwargs):
        # tree has only host and inlined nodes, so this ignores out-of-line descendants (like records)
        descendants = self._local_root_tree.collect_descendants(self, recursive=True)
        method_name = method.self
        if custom_kwargs:
            for node in descendants:
                node_kwargs = custom_kwargs(node)
                getattr(node, method_name)(*args, **kwargs, **node_kwargs)
            node_kwargs = custom_kwargs(self)
            getattr(self, method_name)(*args, **kwargs, **node_kwargs)
        else:
            for node in descendants:
                getattr(node, method_name)(*args, **kwargs)
            getattr(self, method_name)(*args, **kwargs)

    rec_method.__name__ = method.rec
    return rec_method


@node_component
class ScopeNode(Node):
    """A scope for hosting and looking up nodes. Required for any node with children."""

    __has_scope__: ClassVar[bool] = True
    last_changed_at: Optional[datetime] = struct_internal(16, default=None)
    issues: NodeList["Issue"] = node_children(NodeType.ISSUE, NRel.CUMULATIVE)
    # the local tree is maintained at the local root (usually module, maybe a detached root node)
    _local_tree: Union["NodeTreeBase", None] = struct_runtime(default=None)

    @property
    def scope(self) -> "ScopeNode":
        return self

    def _init_inner(self) -> None:
        if self.parent is None:
            if not isinstance(self, Module):
                self._local_tree = DetachedNodeTree()
            else:
                self._local_tree = NodeTree()
            self._local_tree.add(self)

    def _updated_inner(self, properties: Collection[str]) -> None:
        if "name" in properties:
            _InterpChange._collect(self.parent, self.parent, (self,), _NC.Full)._effect(_NC.Full)

    _clear_rec = _make_rec_method(
        _ComponentMethod.clear, Node._clear_self, custom_kwargs=lambda n: dict(scope=n.scope)
    )
    _index_rec = _make_rec_method(_ComponentMethod.index, Node._index_self)
    _interp_rec = _make_rec_method(
        _ComponentMethod.interp,
        Node._interp_self,
        custom_kwargs=lambda n: dict(
            scope=n.scope, on_issue=n.scope._on_issue if n.scope else on_issue_raise
        ),
    )
    _visit_rec = _make_rec_method(_ComponentMethod.visit, Node._visit_self)
    _validate_rec = _make_rec_method(
        _ComponentMethod.validate,
        Node._validate_self,
        custom_kwargs=lambda n: dict(
            properties=n.__tracked_properties__.keys(), on_invalid=on_invalid_raise
        ),
    )
    _activate_rec = _make_rec_method(_ComponentMethod.activate, Node._activate_self)
    _deactivate_rec = _make_rec_method(_ComponentMethod.deactivate, Node._deactivate_self)

    def _get_scope(self, name: str) -> Union["ScopeNode", None]:
        return self._scopes_by_ident.get(name)

    def _find_scope(self, name: str) -> Union["ScopeNode", None]:
        scope = self._get_scope(name)
        if scope is not None:
            # check that we're not resolving something from an out-of-sync cache
            assert scope.attached == self.attached, f"{scope!r} isn't in the same tree as {self!r}"
            return scope
        if self.parent is not None:
            return self.parent._find_scope(name)
        return None

    def _index_inner(self) -> None:
        for prop in self.__list_properties__.values():
            if prop.children_flags & NRel.SCOPED:
                for child in getattr(self, prop.name):
                    if child.name:
                        self._add_node_to_scope(child)

    def _walk_rec(self) -> Iterable["Node"]:
        yield self
        yield from self._local_root_tree.collect_descendants(self, recursive=True)

    def _add_node_to_scope(self, node: "ScopeNode") -> None:
        """
        Adds a child node into this scope. Idempotent for the same node.
        """
        if node.name in self._scopes_by_ident or node.ident in self._names_by_ident:
            if node.ident in self._names_by_ident:
                existing = self._scopes_by_ident[self._names_by_ident[node.ident]]
            else:
                existing = self._scopes_by_ident[node.name]
            if existing.id != node.id:
                self._on_issue(type=IssueType.AMBIGUOUS_DEFINITION, subject=node, path=node.path)
        else:
            self._scopes_by_ident[node.name] = node
            self._names_by_ident[node.ident] = node.name

    def _import_scope_tree(self, scope: "ScopeNode") -> None:
        """Adds the given tree into this scope."""
        assert isinstance(
            scope._local_tree, DetachedNodeTree
        ), f"no tree to import {scope!r} into {self!r}"
        self._local_root_tree.add_tree(scope._local_tree)

    @property
    def _local_root_scope(self) -> "ScopeNode":
        """The root of the 'local' node tree (usually module, but maybe a detached root node)"""
        if self.parent is None:
            return self
        return self.parent._local_root_scope

    @property
    def _local_root_tree(self) -> Union["NodeTreeBase"]:
        """The 'local' node tree (see _local_root_scope)"""
        tree = self._local_root_scope._local_tree
        assert tree is not None, f"no local tree for {self!r} in {self._local_root_scope!r}"
        return tree

    def lookup(
        self,
        path: Union["NodePath", UUID, str],
        node_t: NodeType | StatementType | type[NodeT] | None = None,
    ) -> NodeT | None:
        """
        Lookup the symbol either by path or id. If path is a string, it can be
        it can be a name (lookup upwards) or a full relative/absolute path.
        """
        if isinstance(path, UUID):
            if self._local_tree is not None:
                return self._local_tree.get(path)
            else:
                return self._local_root_scope.lookup(path, node_t=node_t)

        if isinstance(path, str):
            path = parse_node_path(path)
        if path.path == ".":
            return self._find_scope(path.name)
        elif path.path.startswith("."):
            path = NodePath(path.path[1:], path.name)
        parts = path.path.split(".", 2)
        if len(parts) > 1:
            first_part, inner_part = parts[0], NodePath(parts[1], path.name)
        else:
            first_part, inner_part = parts[0], path.name
        scope = self._find_scope(first_part)
        if scope is None:
            return None
        return scope.lookup(inner_part, node_t=node_t)

    def resolve(
        self,
        path: Union["NodePath", UUID, str],
        node_t: type[NodeT] | None = None,
    ) -> NodeT:
        result = self.lookup(path, node_t=node_t)
        if result is None:
            raise LookupError(f"{path} not found in {self!r}")
        return result

    def _get_visible_scopes(self) -> dict[str, "ScopeNode"]:
        """Returns the scopes visible from this node."""
        scopes = {**self._scopes_by_ident}
        if self.parent:
            for name, child in self.parent._get_visible_scopes().items():
                if name not in scopes:  # shadowing
                    scopes[name] = child
        return scopes

    def _on_issue(self, subject: "Node", type: IssueType, message: str = None, **kwargs):
        from bench.language.issue import Issue

        if not subject.attached:
            return  # no way to derive issue id, so just ignore?
        issue = Issue.from_subject(subject, type, message, **kwargs)
        issue.subject.issues.append(issue, _trigger=_NC.Ignore)

    @property
    def errors(self) -> list["Issue"]:
        if self.issues is None:
            return []
        return [i for i in self.issues or [] if i.kind == IssueKind.ERROR]

    @property
    def self_errors(self):
        return [i for i in self.errors or [] if i.parent == self]


# all sub-module node types (inside a module)
LINK_TARGET_NODE_TYPES: tuple[NodeType, ...] = tuple(
    nt
    for nt in NODE_TYPES
    if NodeType.MODULE.id < nt.id < NodeType.SESSION.id and nt != NodeType.LINK
)
LINK_PARENT_NODE_TYPES: tuple[NodeType, ...] = (NodeType.MODULE, NodeType.FILE, NodeType.STATEMENT)


@node(NodeType.LINK)
class Link(Node):
    """A link node refers to another node in some tree. The referenced subtree is inlined during interp."""

    parent: ScopeNode = node_parent(4, *LINK_PARENT_NODE_TYPES)
    reference: Optional[Node] = struct_property(
        30, array=False, references=LINK_TARGET_NODE_TYPES, require=True
    )


@node(NodeType.BENCH, in_module=False)
class Bench(ScopeNode):
    """
    A Bench contains everything a young and growing AI needs to learn and grow.
    """

    parent: None = node_parent(4)
    policies: Optional[list["Policy"]] = struct_internal(
        20, default_factory=list, struct_t=StructType.POLICY
    )
    slug: str = struct_internal(30, protect=True, unique=True)
    name: str = struct_property(31)
    description: Optional[str] = struct_property(32, default=None)
    organization: Optional["Organization"] = struct_internal(
        33, protect=True, require=False, array=False, references=NodeType.ORGANIZATION
    )
    user: Optional["User"] = struct_internal(
        34, protect=True, require=False, array=False, references=NodeType.USER
    )
    # status: BenchStatus = struct_internal(35, protect=True)

    # *per* environment/.../? stuff (will be moved there later)
    head = struct_internal(40, protect=True, require=False, array=False, references=NodeType.MODULE)
    pg_name: Optional[str] = struct_internal(41, protect=True, default=None)
    pg_username: Optional[str] = struct_internal(42, protect=True, default=None, defer=True)
    pg_password: Optional[str] = struct_internal(
        43, protect=True, default=None, defer=True, encrypt=True
    )
    os_name: Optional[str] = struct_internal(44, protect=True, default=None)
    os_username: Optional[str] = struct_internal(45, protect=True, default=None, defer=True)
    os_password: Optional[str] = struct_internal(
        46, protect=True, default=None, defer=True, encrypt=True
    )

    worker_sets: NodeList["WorkerSet"] = node_children(NodeType.WORKER_SET)

    # versions: NodeList["Module"] = node_children(NodeType.MODULE, NRel.Remote)

    @property
    def owner(self) -> Union["Organization", "User", None]:
        return self.organization or self.user

    @property
    def ident(self) -> str:
        return self.slug


@dataclass
class NodeChange:
    source_edits: list[EditData]  # incoming external edits
    interp_edits: list[EditData]  # resulting interp state change
    added: list[Node]
    updated: list[Node]
    removed: list[Node]
    touched_types: set[NodeType | StatementType] = dataclasses.field(init=False)
    all_edits: list[EditData] = dataclasses.field(init=False)

    def __post_init__(self):
        self.touched_types = {n.metatype for n in self.touched} | {
            n.type for n in self.touched if n.metatype == NodeType.STATEMENT
        }
        self.all_edits = self.source_edits + self.interp_edits

    def includes(self, *node_types: NodeType | StatementType) -> bool:
        return any(nt in self.touched_types for nt in node_types)

    @property
    def touched(self) -> Iterable[Node]:
        return chain(self.added, self.updated, self.removed)

    @staticmethod
    def empty() -> "NodeChange":
        return NodeChange([], [], [], [], [])


@node(NodeType.MODULE)
class Module(ScopeNode):
    """A module is a semi-isolated version of a Bench, containing the actual files and so on."""

    parent: Bench = node_parent(4, NodeType.BENCH)
    policies: Optional[list["Policy"]] = struct_internal(
        20, default_factory=list, struct_t=StructType.POLICY
    )
    is_snapshot: bool = struct_internal(32, protect=True, default=False)  # snapshot or head?

    files: NodeList["File"] = node_children(NodeType.FILE, NRel.NAMED | NRel.SCOPED)
    dependencies: dict[str, "Module"] = struct_runtime(default_factory=dict)
    builtins: list["File"] = struct_runtime(default_factory=list)

    _lookup_cache: dict[str, NodeT] = struct_runtime(default_factory=dict)
    _source: Optional[NodeDataTree] = struct_runtime(default=None)

    @property
    def name(self):
        return self.parent.name

    @property
    def is_active(self):
        return not self.is_snapshot

    @property
    def pg_name(self) -> str:
        return self.parent.pg_name

    @property
    def os_name(self) -> str:
        return self.parent.os_name

    @property
    def _tree(self) -> NodeTree:
        return cast(NodeTree, self._local_tree)  # module always has a local tree

    @property
    def _nodes(self) -> Collection[Node]:
        return self._tree.nodes_by_ck.values()

    @property
    def ident(self) -> str:
        return self.parent.ident

    def __content_str__(self):
        return f"is_snapshot={self.is_snapshot}"

    def add_builtin(self, file: "File") -> None:
        if not any(dep == file.module for dep in self.dependencies.values()):
            raise ValueError(f"cannot add builtin {file!r} to {self!r} without {file.module!r}")
        self.builtins.append(file)

    def add_dependency(self, dependency: Union["Module", ModuleReference]) -> None:
        if dependency.name in self.dependencies:
            raise ValueError(
                f"{self!r} already has dependency {dependency.name}: {self.dependencies[dependency.name]}"
            )
        self.dependencies[dependency.name] = dependency

    def lookup(
        self,
        path: Union["NodePath", UUID, str],
        node_t: NodeType | type[NodeT] | None = None,
    ) -> NodeT | None:
        if path in self._lookup_cache:
            return self._lookup_cache[path]

        # extended lookup with dependencies, defaults to regular scope lookup
        if isinstance(path, UUID):
            resolved = self._local_tree.get(path)
            if resolved is None:
                for dependency in self.dependencies.values():
                    if path in dependency._tree:
                        resolved = dependency._tree[path]
                        break
        elif isinstance(path, str) and path.startswith("."):
            resolved = ScopeNode.lookup(self, path, node_t=node_t)
        else:
            if isinstance(path, str):
                path = parse_absolute_node_reference(path)
            module_name, sub_path = path
            if module_name == self.name:
                dependency = self
            else:
                dependency = self.dependencies.get(module_name)
            if dependency is None:
                resolved = None
            else:
                resolved = dependency.lookup(sub_path, node_t=node_t)

        if not resolved:
            return resolved
        assert resolved.attached, f"resolved {path} to detached {resolved!r} (index out of sync?)"
        # cache result
        if self.committed:
            self._lookup_cache[path] = resolved
        return resolved

    def _get_visible_scopes(self) -> dict[str, "ScopeNode"]:
        scopes = {**self._scopes_by_ident}
        for builtin in self.builtins:
            scopes.update(builtin._get_visible_scopes())
        return scopes

    def _activate_inner(self, session: "Session"):
        for dependency in self.dependencies.values():
            if dependency._status != NS.ACTIVE:
                # multiple modules can depend on the same module, only activate once
                dependency._activate_rec(session)

    def _deactivate_inner(self) -> None:
        for dependency in self.dependencies.values():
            if dependency._status == NS.ACTIVE:  # see above
                dependency._deactivate_rec()

    def _index_inner(self):
        for builtin in self.builtins:
            self._add_node_to_scope(builtin)

    def _apply_edits(self, edits: list[EditData], old_source: NodeTree | None = None) -> NodeChange:
        """
        Applies the given external edits to the module.
        TODO @Performance @UX: :HotReload patch edits directly?
        """
        assert self._source is not None, f"cannot apply edits to {self!r} without source"

        if not edits:
            return NodeChange.empty()

        # update source
        old_source = old_source if old_source is not None else self._source.copy()
        self._apply_edits_to_source(edits)

        # update self (this is obviously inefficient, but performs surprisingly okay)
        self._reset_from_source()

        # compute change, apply interp source changes if any
        change = self._compute_change(edits, old_source)
        if change.interp_edits:
            self._apply_edits_to_source(change.interp_edits)
        return change

    def _reset_from_source(self, source: Optional["NodeTree"] = None):
        """Resets the module completely from the source."""
        from bench.proto.wiring import unpack_node_inline

        if source is not None:
            self._source = source
        assert self._source and self.id in self._source, f"cannot reset {self!r} without source"

        prev_session = self.module._session
        if prev_session:
            self.module._deactivate_self()

        if self.module._tree.nodes:  # may be force-reset (_rec methods wouldn't work)
            self.module._clear_rec()
        self.module._tree.clear()
        _ = unpack_node_inline(self._source, parent=self, exclude=INTERP_NODE_TYPES)
        self.module._interp_rec()

        if prev_session:
            self.module._activate_rec(prev_session)

    def _apply_edits_to_source(self, edits: list[EditData]) -> None:
        """Applies the edits directly to the source without any interp."""
        for edit in edits:
            self._source.apply_edit(edit)

    def _compute_change(
        self,
        source_edits: list[EditData],
        old_source: NodeTree,
    ) -> NodeChange:
        """Computes the change between the old and new module state."""
        raise NotImplementedError("nocheckin: _compute_change")

    @staticmethod
    def make(source: list["SomeNodeData"]) -> "Module":
        """Create an interpreted Module from a source module node tree."""
        from bench.proto import wiring

        source = [wiring.unwrap_some_node(s) for s in source]
        source = NodeTree(source)
        root: Bench = wiring.unpack_node_inline(source, parent=None, exclude=INTERP_NODE_TYPES)
        module: Module = root.resolve()  # ???
        assert isinstance(module, Module), f"unexpected module: {module!r}"
        module._source = source
        old_source = module._source.copy()

        module.add_dependency(symbolx_lib)
        module.add_builtin(symbolx_lib.files.get("builtins"))
        module._interp_rec()

        # update source with interp edits (doesn't have them)
        change = module._compute_change([], old_source)
        if change.interp_edits:
            module._apply_edits_to_source(change.interp_edits)

        return module


# quick access to all the classes
_BENCH_CLASSES_BY_NAME: dict[str, type[Node | Struct | enum.Enum]] = {}
BENCH_CLASSES: frozenset[type[Node | Struct | enum.Enum]] = frozenset()
NODE_CLASSES: frozenset[type[Node]] = frozenset()
STRUCT_CLASSES: frozenset[type[Struct]] = frozenset()

# direct parent/child
PARENT_NODE_TYPES: dict[NodeType, tuple[NodeType, ...]] = {}
CHILD_NODE_TYPES: dict[NodeType, tuple[NodeType, ...]] = {}
FERTILE_CHILD_NODE_TYPES: dict[NodeType, tuple[NodeType, ...]] = {}
# transient parent/child
ANCESTOR_NODE_TYPES: dict[NodeType, tuple[NodeType, ...]] = {}
DESCENDANT_NODE_TYPES: dict[NodeType, tuple[NodeType, ...]] = {}


def _complete_bench_setup():
    """Finalize setup of all language constructs after everything is imported."""
    global BENCH_CLASSES
    global NODE_CLASSES
    global STRUCT_CLASSES
    from bench.language import const

    # populate known types
    for bench_t in chain(NODE_CLASS_BY_TYPE.values(), STRUCT_CLASS_BY_TYPE.values()):
        _BENCH_CLASSES_BY_NAME[bench_t.__name__] = bench_t
    for maybe_bench_t in const.__dict__.values():
        if isinstance(maybe_bench_t, type) and issubclass(maybe_bench_t, enum.Enum):
            _BENCH_CLASSES_BY_NAME[maybe_bench_t.__name__] = maybe_bench_t
    BENCH_CLASSES = frozenset(_BENCH_CLASSES_BY_NAME.values())
    for node_t in NODE_TYPES:
        BENCH_CLASS_BY_TYPE[node_t] = NODE_CLASS_BY_TYPE[node_t]
    for struct_t in STRUCT_TYPES:
        BENCH_CLASS_BY_TYPE[struct_t] = STRUCT_CLASS_BY_TYPE[struct_t]
    NODE_CLASSES = frozenset(NODE_CLASS_BY_TYPE.values())
    STRUCT_CLASSES = frozenset(STRUCT_CLASS_BY_TYPE.values())

    # finalize classes
    for cls in chain(get_subclasses(Node), get_subclasses(Struct)):
        # misc finalization on properties
        is_node = issubclass(cls, Node)
        for name, prop in cls.__properties__.items():
            prop: Property
            # determine final storage type
            prop._finalize_type()

            # set properties (that exist at runtime) on class
            if prop.is_runtime:
                setattr(cls, name, prop)

            # ensure the reflected field works (and cache it)
            if prop.is_reflected:
                prop._as_field  # noqa

            # check deferred/encrypted properties
            if prop.is_deferred and not prop.is_stored:
                raise ValueError(f"{prop!r} cannot be deferred and not stored on {cls!r}")
            if not is_node and (prop.is_deferred or prop.is_encrypted):
                raise ValueError(f"{prop!r} cannot be deferred or encrypted on {cls!r}")

            # check py_type matches struct type as defined
            if (
                isinstance(prop.py_type_raw, type)
                and issubclass(prop.py_type_raw, Struct)
                and not issubclass(prop.py_type_raw, Node)
            ):
                if not prop.struct_type:
                    raise ValueError(
                        f"cannot store {prop!r} as {prop.py_type_raw!r} (missing struct_type)"
                    )
                if STRUCT_CLASS_BY_TYPE[prop.struct_type] is not prop.py_type_raw:
                    raise ValueError(f"{prop!r} {prop.struct_type} != {prop.py_type_raw}")

        cls.__stored_properties__ = frozendict(
            {p.name: p for p in cls.__properties__.values() if p.is_stored is True}
        )
        cls.__wired_properties__ = frozendict(
            {p.name: p for p in cls.__properties__.values() if p.is_wired is True}
        )

    # set tables
    from bench.sql.engine import TABLE_BY_NODE_TYPE

    for node_cls in NODE_CLASS_BY_TYPE.values():
        if node_cls.__is_stored__ and not node_cls.__is_stored_custom__:
            node_cls.__table__ = TABLE_BY_NODE_TYPE.get(node_cls.metatype)
        else:
            node_cls.__table__ = None

    # determine node ancestry relationships (parent/child)
    parent_types: dict[NodeType, set[NodeType]] = {nt: set() for nt in NODE_TYPES}
    child_types: dict[NodeType, set[NodeType]] = {nt: set() for nt in NODE_TYPES}
    for node_cls in NODE_CLASS_BY_TYPE.values():
        for parent_type in node_cls.__parent_property__.reference_types:
            parent_types[node_cls.metatype].add(parent_type)
            child_types[parent_type].add(node_cls.metatype)
    # fertile child types: child types that can have children
    fertile_child_types: dict[NodeType, set[NodeType]] = defaultdict(set)
    for node_type, child_type in child_types.items():
        for parent_type in child_type:
            fertile_child_types[parent_type].add(node_type)
    # ancestor/descendant: extend parent/child transitively
    ancestor_types: dict[NodeType, set[NodeType]] = defaultdict(set)
    descendant_types: dict[NodeType, set[NodeType]] = defaultdict(set)
    for node_type in NODE_TYPES:
        new_parents = list(parent_types[node_type])
        while new_parents:
            new_parent = new_parents.pop()
            ancestor_types[node_type].add(new_parent)
            ancestor_types[node_type] |= ancestor_types[new_parent]
            new_parents.extend(parent_types[new_parent] - ancestor_types[node_type])
        new_children = list(child_types[node_type])
        while new_children:
            new_child = new_children.pop()
            descendant_types[node_type].add(new_child)
            descendant_types[node_type] |= descendant_types[new_child]
            new_children.extend(child_types[new_child] - descendant_types[node_type])

    global ANCESTOR_NODE_TYPES, DESCENDANT_NODE_TYPES, PARENT_NODE_TYPES, CHILD_NODE_TYPES
    global FERTILE_CHILD_NODE_TYPES
    for node_type in NODE_TYPES:
        ANCESTOR_NODE_TYPES[node_type] = tuple(ancestor_types[node_type])
        DESCENDANT_NODE_TYPES[node_type] = tuple(descendant_types[node_type])
        PARENT_NODE_TYPES[node_type] = tuple(parent_types[node_type])
        CHILD_NODE_TYPES[node_type] = tuple(child_types[node_type])
        FERTILE_CHILD_NODE_TYPES[node_type] = tuple(fertile_child_types[node_type])

    # check that is_in_module/is_in_bench was declared correctly
    #  (need to set that in @node upfront because traversing parents can only happen in finalization)
    for node_cls in NODE_CLASS_BY_TYPE.values():
        in_bench = (
            node_cls.metatype == NodeType.BENCH
            or node_cls.metatype in DESCENDANT_NODE_TYPES[NodeType.BENCH]
        )
        in_module = (
            node_cls.metatype == NodeType.MODULE
            or node_cls.metatype in DESCENDANT_NODE_TYPES[NodeType.MODULE]
        )
        if in_bench != node_cls.__is_in_bench__ or in_module != node_cls.__is_in_module__:
            raise ValueError(
                f"{node_cls!r} parent types are inconsistent: root={node_cls.__root__} implies in_bench={in_bench} and in_module={in_module}, but got in_bench={node_cls.__is_in_bench__} and in_module={node_cls.__is_in_module__}"
            )
    check_collections_equal(
        IN_BENCH_NODE_TYPES, [t.metatype for t in NODE_CLASS_BY_TYPE.values() if t.__is_in_bench__]
    )
    check_collections_equal(
        IN_MODULE_NODE_TYPES,
        [t.metatype for t in NODE_CLASS_BY_TYPE.values() if t.__is_in_module__],
    )

    # check that all enum types are valid proto-able enums
    for struct_t in chain(STRUCT_CLASS_BY_TYPE.values(), NODE_CLASS_BY_TYPE.values()):
        for prop in struct_t.__properties__.values():
            if prop.is_enum and not issubclass(
                prop.py_type_stripped, (ProtoStrEnum, enum.IntEnum, enum.IntFlag)
            ):
                raise ValueError(f"{prop!r} is not a valid proto enum")
