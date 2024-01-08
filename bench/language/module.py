import abc
import dataclasses
import enum
import functools
import inspect
import typing
import uuid
from collections import defaultdict
from dataclasses import dataclass
from datetime import datetime
from itertools import chain
from logging import Logger
from typing import (
    TYPE_CHECKING,
    Any,
    Callable,
    ClassVar,
    Collection,
    Iterator,
    Optional,
    Union,
)
from uuid import UUID, uuid4

import structlog
from cachetools import cached

from bench.language.const import (
    IN_BENCH_NODE_TYPES,
    IN_MODULE_NODE_TYPES,
    INTERP_NODE_TYPES,
    UUID_NAMESPACE,
    BenchType,
    ConditionalOp,
    ExpressionOp,
    IssueKind,
    IssueType,
    ModuleReference,
    NodePath,
    NodeTrackingLevel,
    NodeType,
    SortOp,
    StatementType,
    StructType,
    TypeHint,
    TypeTag,
    parse_absolute_node_reference,
    parse_node_path,
)
from bench.language.tree import DetachedNodeTree, NodeTree, NodeTreeBase
from bench.language.validation import (
    PropertyValidationHandler,
    ValidationError,
    ValidationHandler,
    on_invalid_raise,
)
from bench.proto.core import ProtoStrEnum
from bench.proto.wire import EditData, SomeNodeData
from bench.sql.core import CascadeAction, ColumnType
from bench.utils.dt import utcnow_with_tz
from bench.utils.fractional import BIGGEST_INTEGER, generate_key_between, generate_n_keys_between
from bench.utils.func import did_you_mean_str, get_subclasses, nextn, strip_py_type, try_tuple
from bench.utils.utils import (
    DEBUG,
    LOCAL,
    IdentifierType,
    flatten,
    frozendict,
    required_field,
    to_pyidentifier,
)

if TYPE_CHECKING:
    from bench.language import (
        Expression,
        Field,
        File,
        Issue,
        NodeVisitor,
        Organization,
        Policy,
        Session,
        User,
        WorkerSet,
        symbolx_lib,
    )
    from bench.language.issue import IssueHandler

logger = structlog.get_logger(__name__)


def on_issue_raise(
    subject: "Node",
    type: IssueType,
    message: Optional[str] = None,
    path: Optional[str] = None,
    **kwargs,
):
    from bench.language.issue import Issue

    raise Issue.from_subject(subject=subject, type=type, message=message, path=path).to_error()


def new_node_identity(module_id: UUID) -> tuple[UUID, UUID]:
    ck = uuid4()
    id = get_node_id(module_id, ck)
    return id, ck


def new_detached_node_identity() -> tuple[UUID, UUID]:
    ck = uuid4()
    return ck, ck


def get_node_id(module_id: UUID, ck: UUID):
    """Derive the version-specific node id from its constant key"""
    return uuid.uuid5(module_id, str(ck))


class LookupBy(enum.StrEnum):
    Name = "name"
    PyIdent = "py_ident"


class NodeRelationType(enum.IntFlag):
    """Parent relation between node and descendants."""

    Default = 0  # default inline relation
    Remote = 2**0  # not inline: Statement->Record, ...
    Shared = 2**1  # across versions: Statement->Comment, Statement[versioned=False]->Record, ...
    Flat = 2**2  # flattened inner hierarchy: Module->File, File->Statement, ...
    Cumulative = 2**3  # sum of descendants: Module->Issue, File->Issue, ...
    Named = 2**4  # indexed by name: Module->File, File->Statement, ...
    Scoped = 2**5  # scoped by name: Module->File, File->Statement, ...
    Keyed = 2**6  # indexed by key: File->Tagging, Statement->Tagging, ...
    Ordered = 2**7  # ordered: File->Statement, Statement->Field, ...


NRel = NodeRelationType

FLATTENED_RELATIONS = (
    (NodeType.MODULE, NodeType.FILE),
    (NodeType.FILE, NodeType.STATEMENT),
    (NodeType.STATEMENT, NodeType.FIELD),
    (NodeType.SESSION, NodeType.RUN),
)

UNSET = object()


def _require_expr_op(op: ExpressionOp):
    def decorator(func):
        @functools.wraps(func)
        def wrapper(self: "_FieldExpressionBase", *args, **kwargs):
            from bench.language.expression import _check_field_supports

            _check_field_supports(self._as_field, op)
            return func(self, *args, **kwargs)

        return wrapper

    return decorator


def _to_conditional(op: ConditionalOp, field: "Field", value: Any = None):
    from bench.language.expression import C

    return C(op, field=field, value=value)


class _FieldExpressionBase:
    """
    Base for field-like expressions on a field-like class.
    We define this here to use it for Property and Field.
    """

    @property
    def _as_field(self) -> "Field":
        from bench.language.field import Field

        assert isinstance(self, Field), f"{self!r} is not a Field"
        return self

        # basic support checks

    def _coerce_value(self: "Field", value: Any) -> Any:
        from bench.language.field import Field

        if self._as_field._effective_tag == TypeTag.ENUM and not isinstance(value, Field):
            value = self.resolved_fields.get(value)
        return value

    # comparison

    @_require_expr_op(ConditionalOp.EQUALS)
    def equals(self, value: Any) -> "Expression":
        value = self._coerce_value(value)
        if value is None:
            return self.not_exists()
        return _to_conditional(ConditionalOp.EQUALS, self._as_field, value=value)

    @_require_expr_op(ConditionalOp.NOT_EQUALS)
    def not_equal(self, value: Any) -> "Expression":
        value = self._coerce_value(value)
        return _to_conditional(ConditionalOp.NOT_EQUALS, self._as_field, value=value)

    @_require_expr_op(ConditionalOp.GREATER_THAN)
    def greater_than(self, value: Any) -> "Expression":
        value = self._coerce_value(value)
        return _to_conditional(ConditionalOp.GREATER_THAN, self._as_field, value=value)

    @_require_expr_op(ConditionalOp.GREATER_THAN_OR_EQUALS)
    def greater_than_or_equals(self, value: Any) -> "Expression":
        value = self._coerce_value(value)
        return _to_conditional(ConditionalOp.GREATER_THAN_OR_EQUALS, self._as_field, value=value)

    @_require_expr_op(ConditionalOp.LESS_THAN)
    def less_than(self, value: Any) -> "Expression":
        value = self._coerce_value(value)
        return _to_conditional(ConditionalOp.LESS_THAN, self._as_field, value=value)

    @_require_expr_op(ConditionalOp.LESS_THAN_OR_EQUALS)
    def less_than_or_equals(self, value: Any) -> "Expression":
        value = self._coerce_value(value)
        return _to_conditional(ConditionalOp.LESS_THAN_OR_EQUALS, self._as_field, value=value)

    def __eq__(self, other):
        if isinstance(self, Node) and isinstance(other, Node):
            return Node.__eq__(self._as_field, other)  # imitate Field equality
        return self.equals(other)

    def __ne__(self, other):
        if isinstance(self, Node) and isinstance(other, Node):
            return Node.__ne__(self._as_field, other)
        return self.not_equal(other)

    __gt__ = greater_than
    __ge__ = greater_than_or_equals
    __lt__ = less_than
    __le__ = less_than_or_equals

    # string comparison

    @_require_expr_op(ConditionalOp.STARTS_WITH)
    def starts_with(self, value: str) -> "Expression":
        return _to_conditional(ConditionalOp.STARTS_WITH, self._as_field, value=value)

    @_require_expr_op(ConditionalOp.MATCHES)
    def matches(self, value: str) -> "Expression":
        return _to_conditional(ConditionalOp.MATCHES, self._as_field, value=value)

    # containment

    @_require_expr_op(ConditionalOp.IN)
    def in_(self, *values: list[Any]) -> "Expression":
        values = [self._coerce_value(value) for value in values]
        return _to_conditional(ConditionalOp.IN, self._as_field, value=values)

    @_require_expr_op(ConditionalOp.NOT_IN)
    def not_in(self, *values: list[Any]) -> "Expression":
        values = [self._coerce_value(value) for value in values]
        return _to_conditional(ConditionalOp.NOT_IN, self._as_field, value=values)

    @_require_expr_op(ConditionalOp.CONTAINS)
    def contains(self, value: Any) -> "Expression":
        value = self._coerce_value(value)
        return _to_conditional(ConditionalOp.CONTAINS, self._as_field, value=value)

    @_require_expr_op(ConditionalOp.NOT_CONTAINS)
    def not_contains(self, value: Any) -> "Expression":
        value = self._coerce_value(value)
        return _to_conditional(ConditionalOp.NOT_CONTAINS, self._as_field, value=value)

    # existence

    @_require_expr_op(ConditionalOp.EXISTS)
    def exists(self) -> "Expression":
        return _to_conditional(ConditionalOp.EXISTS, self._as_field)

    @_require_expr_op(ConditionalOp.NOT_EXISTS)
    def not_exists(self) -> "Expression":
        return _to_conditional(ConditionalOp.NOT_EXISTS, self._as_field)

    # knn

    @_require_expr_op(ConditionalOp.NEAR)
    def near(self, value: list[float]) -> "Expression":
        return _to_conditional(ConditionalOp.NEAR, self._as_field, value=value)

    # sort

    @_require_expr_op(SortOp.ASCENDING)
    def asc(self) -> "Expression":
        from bench.language.expression import S

        return S(SortOp.ASCENDING, field=self._as_field)

    ascending = asc

    @_require_expr_op(SortOp.DESCENDING)
    def desc(self) -> "Expression":
        from bench.language.expression import S

        return S(SortOp.DESCENDING, field=self._as_field)

    descending = desc


PROPERTY_COLUMN_TYPE_BY_PY_TYPE: dict[type, ColumnType] = {
    bool: ColumnType.BOOLEAN,
    int: ColumnType.BIGINT,
    float: ColumnType.FLOAT,
    str: ColumnType.STRING,
    bytes: ColumnType.BYTES,
    datetime: ColumnType.DATETIME,
    UUID: ColumnType.UUID,
}


@dataclass
class Property(_FieldExpressionBase):
    """A property of a module node or struct."""

    id: int | None = None  # stable id for wiring properties, must be unique per final struct/node
    name: str | None = None  # name from LHS of assignment
    description: str | None = None  # description from docstring
    component: type["Node"] | None = None  # source component class
    py_type_raw: typing.Any = None  # type annotation on LHS of assignment
    py_type_stripped: typing.Any = UNSET  # stripped type annotation
    # config
    alias: str | None = None  # for node list relations
    is_array: bool = UNSET
    is_required: bool = False  # = must be non-null
    is_internal: bool = False  # = not directly editable for user
    is_protected: bool = False  # = only editable by us/supervisor
    is_reflected: bool = False  # eventually all properties should be reflected, for now only some
    is_ancestor_nearest: bool | None = None  # for ancestor relations
    is_ancestor_self: bool | None = None  # for ancestor relations
    is_computed: bool = False
    is_runtime_only: bool = False
    is_runtime: bool = UNSET  # exists on runtime instance?
    is_wired: bool = UNSET  # serialized onto wire?
    is_stored: bool = UNSET  # stored in DB?
    is_indexed_in_pg: bool = False  # indexed in DB?
    is_unique: bool = False  # unique index in DB?
    is_deferred: bool = False  # loaded only on demand (only for stored node properties)
    is_encrypted: bool = False  # encrypt at rest (only node properties)
    struct_type: StructType | None = None  # for struct properties
    references: tuple[NodeType, ...] | None = None  # for reference relations
    reference_key: Optional["Property"] = None  # for reference relations
    reference_source: Optional["Property"] = None  # for reference relations (reverse)
    reference_on_delete: CascadeAction | None = UNSET
    parents: tuple[NodeType, ...] | None = None
    ancestor: NodeType | None = None
    column_type: ColumnType | None = UNSET
    default: typing.Any = UNSET
    default_factory: Callable[[], typing.Any] | None = None
    list_type: type["NodeListBase"] | None = None
    child_node_type: NodeType | None = None
    children_flags: NodeRelationType = NodeRelationType.Default
    custom_validate: Callable[[typing.Any, "PropertyValidationHandler"], bool | None] | None = None
    custom_copy: Callable[[typing.Any], typing.Any] | None = None
    ignore_conflicts_with: tuple[type["Node"], ...] | None = None

    @functools.cached_property
    def _as_field(self) -> "Field":
        assert self.is_reflected, f"{self!r} is not reflected"

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
        else:
            raise ValueError(f"unexpected column type in {self!r}: {self.column_type}")
        field = Field(
            name=self.name,
            ck=uuid.uuid5(UUID_NAMESPACE, f"{metatype_id}.{self.id}"),
            tag=tag,
            hint=hint,
        )
        return field

    def __post_init__(self):
        if (
            not self.is_tree_relation
            and self.is_runtime_only
            and self.default is UNSET
            and self.default_factory is None
        ):
            raise ValueError(f"missing default for {self!r}")
        if self.references:
            if self.default is not UNSET:
                raise ValueError(f"cannot set default for reference property {self!r}")
            self.default = None

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
            "references",
            "parents",
            "ancestor",
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
            elif k in ("parent_node_types", "reference_types"):
                types_str = "|".join(t.name for t in v)
                if types_str:
                    non_default.append(f"to={types_str}")
            else:
                if isinstance(v, bool):
                    non_default.append(k)
                else:
                    non_default.append(f"{k}={v}")
        attrs_str = ", ".join(non_default)
        attrs_str = f" ({attrs_str})" if attrs_str else ""
        return f"<{self.__class__.__name__} {str(self)}{attrs_str}>"

    @property
    def is_tree_relation(self) -> bool:
        """Whether this is a node relation property (parent/child/ancestor)."""
        return bool(self.parents) or self.child_node_type or self.ancestor

    @property
    def is_node_reference(self):
        return bool(self.references)

    @property
    def is_static(self):
        return not self.is_runtime_only

    @property
    def is_struct(self) -> bool:
        return self.struct_type is not None

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
                "reference_key",
                "reference_source",
                "py_type_raw",
                "py_type_stripped",
            ):
                continue
            if getattr(self, k.name) != getattr(other, k.name):
                return False
        return True

    def finalize_type(self) -> None:
        """Analyzes the final type and configures storage options. Must run after complete setup."""
        # store/wire property by default if not runtime (and not marked as _not_ store)
        if self.column_type is UNSET and (
            self.is_tree_relation or self.is_runtime_only or self.references
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

        # resolve py type
        if self.is_runtime_only or self.parents is not None or self.references is not None:
            # can't resolve these because they point to non-Bench types
            self.py_type_stripped = self.py_type_raw
        else:
            py_type, info = strip_py_type(self.py_type_raw)
            # resolve manually if needed
            if isinstance(py_type, (str, typing.ForwardRef)):
                py_type = py_type.__forward_arg__ if not isinstance(py_type, str) else py_type
                if py_type not in _BENCH_TYPES_BY_NAME:
                    raise ValueError(f"cannot resolve type for {self!r}: {py_type!r}")
                py_type = _BENCH_TYPES_BY_NAME[py_type]
            self.py_type_stripped = py_type
            # update info from annotation
            if self.is_array is UNSET:
                self.is_array = info.is_array

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

    def contribute_properties(self) -> tuple["Property"]:
        """Contribute any extra properties required by this property."""

        if self.parents is not None:
            # special reference to parent (via id, resolved before instantiating)
            # in wire we unify into parent_id, for store split per parent_<type>_id for integrity
            wired_id_prop = Property(
                id=self.id,  # re-use id, self is not stored
                name=self.name + "_id",
                component=self.component,
                py_type_raw=UUID,
                default=None,
                reference_source=self,
                is_required=self.is_required,
                is_internal=True,
                is_computed=True,
                is_wired=True,
                is_stored=False,
                is_array=False,
                column_type=ColumnType.UUID,
            )
            parent_id_props: list[Property] = []
            for parent_node_type in self.parents:
                parent_id_prop = Property(
                    id=self.id,
                    name=self.name + f"_{parent_node_type.name.lower()}_id",
                    component=self.component,
                    py_type_raw=UUID,
                    references=(parent_node_type,),
                    reference_source=self,
                    reference_on_delete=CascadeAction.CASCADE,
                    is_required=self.is_required,
                    is_internal=True,
                    is_runtime=False,
                    is_wired=False,
                    is_stored=True,
                    is_array=False,
                    is_indexed_in_pg=self.is_indexed_in_pg,
                    column_type=ColumnType.UUID,
                )
                parent_id_props.append(parent_id_prop)
            self.reference_key = wired_id_prop
            return (wired_id_prop, *parent_id_props)
        elif self.ancestor is not None:
            assert self.is_stored is not UNSET, f"must set is_stored on {self!r}"
            assert self.is_wired is not UNSET, f"must set is_wired on {self!r}"
            ancestor_id_prop = Property(
                id=self.id,
                name=self.name + "_id",
                component=self.component,
                py_type_raw=UUID,
                references=(self.ancestor,),
                reference_source=self,
                reference_on_delete=CascadeAction.CASCADE,
                is_required=self.is_required,
                is_internal=True,
                is_computed=True,
                is_wired=self.is_wired,
                is_stored=self.is_stored,
                is_array=False,
                is_indexed_in_pg=self.is_indexed_in_pg,
                column_type=ColumnType.UUID,
            )
            self.reference_key = ancestor_id_prop
            if self.is_stored or self.is_wired:
                self.is_stored = False  # the key is stored instead
                if self.is_wired is True:
                    self.is_wired = False  # the key is wired instead
                return (ancestor_id_prop,)
        elif self.references is not None:
            # regular reference to node (via ck for in-module nodes, id otherwise)
            assert self.is_array is not UNSET, f"must set is_array on {self!r}"
            #  (move contribute_properties to finalization step)
            self.reference_key = Property(
                id=self.id,
                name=self.name + "_ck",  # we merge id/ck into 'ck' for wire/runtime
                component=self.component,
                py_type_raw=UUID,
                references=self.references,
                reference_source=self,
                is_required=self.is_required,
                is_internal=True,
                is_runtime=True,
                is_wired=True,
                is_stored=False,
                is_array=self.is_array,
                is_indexed_in_pg=self.is_indexed_in_pg,
                column_type=ColumnType.UUID,
            )
            reference_props = []
            for ref_type in self.references:
                store_as_id = ref_type not in IN_MODULE_NODE_TYPES
                prop_postfix = "id" if store_as_id else "ck"
                if self.name == ref_type.name.lower():
                    prop_name = f"{self.name}_{prop_postfix}"
                else:
                    prop_name = f"{self.name}_{ref_type.name.lower()}_{prop_postfix}"
                if prop_name == self.reference_key.name:
                    self.reference_key.is_stored = True
                    continue  # no need for a special store-only property
                reference_prop = Property(
                    id=self.id,
                    name=prop_name,
                    component=self.component,
                    py_type_raw=UUID,
                    references=(ref_type,),
                    reference_source=self,
                    reference_on_delete=CascadeAction.SET_NULL,
                    is_required=self.is_required,
                    is_internal=True,
                    is_runtime=False,
                    is_wired=False,
                    is_stored=True,
                    is_array=self.is_array,
                    is_indexed_in_pg=self.is_indexed_in_pg,
                    column_type=ColumnType.UUID,
                )
                reference_props.append(reference_prop)
            return (self.reference_key, *reference_props)

        return tuple()

    def new(self) -> typing.Any:
        if self.default is not UNSET:
            return self.default
        elif self.default_factory is not None:
            return self.default_factory()
        else:
            raise ValueError(f"no default for {self!r}")

    def copy(self, value: typing.Any) -> typing.Any:
        if self.is_tree_relation:
            raise ValueError(f"cannot copy relation {self!r}")
        elif self.references:
            return value  # identity
        elif self.custom_copy is not None:
            return self.custom_copy(value)
        # auto-copy if it's trivial (primitives, immutable, enum, ...)
        elif isinstance(value, (type(None), bool, int, float, str, UUID, datetime, enum.Enum)):
            return value
        else:
            raise ValueError(f"cannot copy {self!r}")

    def validate(self, value: typing.Any, on_issue: "PropertyValidationHandler") -> bool | None:
        if self.custom_validate is not None:
            return self.custom_validate(value, on_issue)
        else:
            return None


def struct_property(
    id: int,
    *,
    description: str = None,
    default: typing.Any = UNSET,
    default_factory: Callable[[], typing.Any] = None,
    copy: Callable[[typing.Any], typing.Any] = None,
    validate: Callable[[typing.Any, "PropertyValidationHandler"], bool | None] = None,
    require: bool = False,
    reflect: bool = False,
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
        references=try_tuple(references),
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
    default: typing.Any = UNSET,
    default_factory: Callable[[], typing.Any] = None,
    copy: Callable[[typing.Any], typing.Any] = None,
    require: bool = False,
    reflect: bool = False,
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
        references=try_tuple(references),
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
    default: typing.Any = UNSET,
    default_factory: Callable[[], typing.Any] = None,
    copy: Callable[[typing.Any], typing.Any] = None,
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
        id=id, parents=tuple(node_type), default=None, is_internal=True, is_stored=False
    )


def node_ancestor(
    id: int,
    node_type: NodeType,
    nearest: bool = True,
    include_self: bool = True,
    store: bool = False,
    wire: bool = False,
    index_in_pg: bool = False,
):
    """Computed nearest or farthest ancestor of the given type."""
    return Property(
        id=id,
        ancestor=node_type,
        default=None,
        is_internal=True,
        is_computed=True,
        is_protected=True,
        is_ancestor_nearest=nearest,
        is_ancestor_self=include_self,
        is_stored=store,
        is_wired=wire,
        is_indexed_in_pg=index_in_pg,
    )


def node_children(
    node_type: NodeType,
    flags: NRel = NRel.Default,
    custom_list: type["NodeListBase"] = None,
    alias: str = None,
):
    """Computed read/write children or descendants of the given type."""
    return Property(
        child_node_type=node_type,
        children_flags=flags,
        is_internal=True,
        is_required=True,
        default=None,  # set in our custom init
        list_type=custom_list or NodeList,
        is_stored=False,
        alias=alias,
    )


class NodeStatus(enum.IntEnum):
    SOURCE = 0
    INDEX = 1
    INTERP = 2
    ACTIVE = 3


NS = NodeStatus


class ComponentMethod(enum.Enum):
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
_NODE_INNER_METHODS: list[str] = [m.inner for m in ComponentMethod]
_FORBIDDEN_NODE_METHODS = (
    [m.self for m in ComponentMethod]
    + [m.rec for m in ComponentMethod]
    + ["__post_init__", "__del__"]
)
NODE_CLASS_BY_NODE_TYPE: dict[NodeType, type["NodeT"]] = {}
NODE_COMPONENT_CLASS_BY_NAME: dict[str, type["Node"]] = {}
STRUCT_CLASS_BY_STRUCT_TYPE: dict[StructType, type["Struct"]] = {}
BENCH_CLASS_BY_TYPE: dict[BenchType, type["Node"] | type["Struct"]] = {}
_COMPONENT_METHODS: dict[[ComponentMethod, type["Node"]], typing.Any] = {}
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
    components: list[type["Node"]], method: ComponentMethod, concrete_key: str
) -> list[typing.Any]:
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
        for p in prop.contribute_properties():
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
            if existing is None or name == "parent" or existing.id is UNSET:
                if prop.is_static or component not in dynamic_components:
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
        if not is_in_bench and prop.name == "bench" or not is_in_module and prop.name == "module":
            # remove 'bench'/'module' ancestor property if not actually a descendant :MagicNodeProps
            attr = None
            del properties_by_name[name]
            del properties_by_name[prop.reference_key.name]  # remove contributed reference key too
        elif prop.name == "ck" and is_node and not is_in_module:
            # remove node ck (is computed from id if outside module) :MagicNodeProps
            attr = _node_ck_from_id_prop(prop)
            del properties_by_name[name]
        elif not is_final:
            attr = None  # only set attributes in final class
        elif prop.ancestor and cls.__name__ not in ("ScopeNode", "Node"):
            attr = _node_ancestor_prop(prop)
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
        # also set extra computed parent/ancestor id property
        if prop.parents or prop.ancestor:
            setattr(cls, prop.name + "_id", _node_ancestor_id_prop(prop))

    cls = dataclass(cls, repr=False, eq=False)  # type: ignore

    # collect methods implemented in this class (specifically)
    for meth_type in ComponentMethod:
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
        if prop.id is not None and prop.is_wired and not prop.reference_key:
            existing = properties_by_id.get(prop.id, None)
            if existing is not None:
                raise ValueError(f"property id conflict: {prop!r}, {existing!r}")
            properties_by_id[prop.id] = prop
    props = properties_by_name.values()
    cls.__properties_by_id__ = frozendict(properties_by_id)
    cls.__tracked_properties__ = frozendict({p.name: p for p in props if not p.is_internal})
    cls.__internal_properties__ = frozendict({p.name: p for p in props if p.is_internal})
    cls.__reference_properties__ = frozendict({p.name: p for p in props if p.references})
    cls.__struct_properties__ = frozendict({p.name: p for p in props if p.is_struct})

    return cls, properties_by_name


@typing.dataclass_transform()
def struct_component(
    cls: Optional[typing.Type] = None,
    struct_type: StructType = None,
    reserved: set[str | int] = None,
    is_final: bool = False,
):
    """
    Mark a class as a struct component (or concrete struct for a StructType).
    """

    def decorate(cls):
        cls, properties = _process_struct_base_cls(cls=cls, reserved=reserved)

        # register struct
        if struct_type:
            cls.metatype = struct_type
            if struct_type in STRUCT_CLASS_BY_STRUCT_TYPE:
                raise ValueError(
                    f"struct class conflict for {struct_type}: {cls}, {STRUCT_CLASS_BY_STRUCT_TYPE[struct_type]}"
                )
            STRUCT_CLASS_BY_STRUCT_TYPE[struct_type] = cls
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


@typing.dataclass_transform()
def node_component(
    cls: Optional[typing.Type] = None,
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
            if prop.child_node_type:
                if (
                    cls.__name__ != "ScopeNode"
                    and not issubclass(cls, ScopeNode)
                    and node_type is not None
                ):
                    raise ValueError(f"{cls} is not a ScopeNode for {prop}")
                list_properties[prop.name] = prop
                list_properties_by_child[prop.child_node_type].append(prop)
        cls.__list_properties__ = frozendict(list_properties)
        cls.__list_properties_by_child__ = frozendict(list_properties_by_child)
        cls.__ancestor_properties__ = frozendict({p.name: p for p in props if p.ancestor})

        # register as concrete node class for node_type
        if node_type:
            cls.metatype = node_type
            if node_type in NODE_CLASS_BY_NODE_TYPE:
                raise ValueError(
                    f"node class conflict for {node_type}: {cls}, {NODE_CLASS_BY_NODE_TYPE[node_type]}"
                )
            NODE_CLASS_BY_NODE_TYPE[node_type] = cls
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

        parent_property = cls.__properties__.get("parent", None)
        if parent_property is None:
            raise ValueError(f"node {cls} has no parent property")
        cls.__parent_property__ = parent_property
        cls.__root__ = root
        cls.__is_in_module__ = in_module
        cls.__is_in_bench__ = in_bench

        return cls

    return decorate


NodeT = typing.TypeVar("NodeT", bound="Node")


def _node_ck_from_id_prop(prop: Property) -> property:
    """Get ck from id (read-only)."""

    def get(self: NodeT) -> UUID:
        return self.id

    def set(self: NodeT, value: UUID):
        raise NotImplementedError(f"cannot set computed property {prop!r}: {value!r}")

    return property(get, set)


def _node_ancestor_prop(prop: Property) -> property:
    """Computed ancestor property for Node instances."""

    if prop.is_ancestor_nearest:

        def get_nearest(self: NodeT) -> Optional[NodeT]:
            parent = self if prop.is_ancestor_self else self.parent
            while parent is not None:
                if parent.metatype == prop.ancestor:
                    return parent
                parent = parent.parent
            return None

        get = get_nearest
    else:

        def get_farthest(self: NodeT) -> Optional[NodeT]:
            parent = self if prop.is_ancestor_self else self.parent
            farthest = None
            while parent is not None:
                if parent.metatype == prop.ancestor:
                    farthest = parent
                parent = parent.parent
            return farthest

        get = get_farthest

    def set(self: NodeT, value: NodeT):
        raise NotImplementedError(f"cannot set computed property {prop!r}: {value!r}")

    return property(get, set)


def _node_ancestor_id_prop(prop: Property) -> property:
    """Computed ancestor/parent id property for Node classes."""

    def get(self: NodeT) -> Optional[UUID]:
        ancestor = getattr(self, prop.name)
        if ancestor is None:
            return None
        return ancestor.id

    def set(self: NodeT, value: Optional[UUID]):
        raise NotImplementedError(f"cannot set computed property {prop!r}: {value!r}")

    return property(get, set)


def _sort_nested_ordered_list(root_ck: UUID, nodes: list[NodeT]) -> list[NodeT]:
    """
    Sort a list of ordered, hierarchical nodes.
    Each node is ordered within its 'parent' (by 'order_key'). Start at the root.
    """
    ordered = []

    nodes_by_parent_ck: dict[UUID, list[NodeT]] = defaultdict(list)
    for node in nodes:
        nodes_by_parent_ck[node.parent.ck].append(node)

    def _walk_dfs(parent_ck: UUID):
        children = nodes_by_parent_ck.get(parent_ck, None)
        if children:
            children.sort(key=lambda n: n.order_key or BIGGEST_INTEGER)
            for child in children:
                ordered.append(child)
                _walk_dfs(child.ck)

    _walk_dfs(root_ck)

    if len(ordered) != len(nodes):
        missing_nodes = [n for n in nodes if n not in ordered]
        assert not missing_nodes, f"missing {len(missing_nodes)} nodes {missing_nodes} in {ordered}"
    return ordered


class _NodeChange(enum.IntFlag):
    """The kind of reactive change effect to trigger in a node."""

    Ignore = 0
    UpdateLists = 2**0
    Detach = 2**1
    Attach = 2**2
    Tach = Detach | Attach
    Full = UpdateLists | Detach | Attach


_NC = _NodeChange


@dataclass
class _InterpChange:
    """
    The effect of a change in nodes.
    TODO @Performance: optimize change effects (batch, lazy/mark dirty?, reduce impact radius)
    """

    prev_session: Optional["Session"]
    prev_status: Optional[NS]
    level: _NC
    affected_node_types: set[NodeType] | None
    ancestors: list["Node"] | None
    affected: list["Node"] | None

    @staticmethod
    def _collect(
        from_parent: Optional["Node"],
        to_parent: Optional["Node"],
        changed: list["Node"],
        level: _NC,
    ) -> "_InterpChange":
        """Collects nodes affected by a change in the given children."""
        assert changed, f"cannot create update on {to_parent!r} without changed nodes"
        assert from_parent or to_parent, f"cannot create update on {changed!r} without parent"

        # collect ancestors to update their affected node lists
        affected_node_types = set([n.metatype for n in changed])
        ancestors = []
        if level >= _NC.UpdateLists:
            parent = from_parent
            while parent is not None:
                ancestors.append(parent)
                parent = parent.parent
            parent = to_parent
            while parent is not None:
                ancestors.append(parent)
                parent = parent.parent

        # collect nodes to reinterp following attach/detach
        if level & (_NC.Detach | _NC.Attach):
            affected_nodes: list[Node] | None = ancestors[:]
            for child in changed:
                affected_nodes.append(child)
                if isinstance(child, ScopeNode):
                    affected_nodes.extend(
                        child._local_root_tree.get_descendants(
                            child.ck, recursive=True, include_self=False
                        )
                    )
            # filter out interp types
            affected_nodes = [n for n in affected_nodes if n.metatype not in INTERP_NODE_TYPES]
        else:
            affected_nodes = None

        return _InterpChange(
            prev_session=to_parent._session if to_parent else None,
            prev_status=to_parent._status if to_parent else None,
            affected_node_types=affected_node_types,
            ancestors=ancestors,
            affected=affected_nodes,
            level=level,
        )

    def _effect(self, level: _NC | None = None) -> None:
        """Applies the effect of a trigger to update the affected nodes."""
        if level & _NC.UpdateLists:
            for ancestor in self.ancestors:
                for prop in ancestor.__list_properties__.values():
                    if prop.child_node_type in self.affected_node_types:
                        getattr(ancestor, prop.name)._update(ancestor)

        if level & _NC.Detach:
            for _node in self.affected:
                if _node._session and _node._status == NS.ACTIVE:
                    _node._deactivate_self()
                    _node._detached_self()
            for _node in self.affected:
                _node._clear_self(_node.scope)

        if level & _NC.Attach:
            for _node in self.affected:
                _node._index_self()
            for _node in self.affected:
                _node._interp_self(
                    _node.scope, on_issue=_node.scope._on_issue if _node.scope else on_issue_raise
                )
                if self.prev_session and self.prev_status == NS.ACTIVE:
                    _node._attached_self()
                    _node._activate_self(self.prev_session)


class NodeListBase(abc.ABC, Collection, typing.Generic[NodeT]):
    """
    Base node list for custom implementation (right now just for database).
    """

    def __init__(self, parent: "ScopeNode", property: Property):
        self._parent = parent
        self._property = property

    def __repr__(self):
        return f"<{self.__class__.__name__} {self._parent.path}->{self._property.name}: {self}>"

    def _update(self, scope: "ScopeNode"):
        """Recomputes the list from the given scope."""
        raise NotImplementedError

    def create(self, *args, _append: bool = True, **kwargs) -> NodeT:
        """Creates a new node in the list."""
        if len(args) == 1 and isinstance(args[0], Node):
            raise ValueError(f"cannot create {args[0]!r}, use append for existing nodes")
        node_cls = NODE_CLASS_BY_NODE_TYPE[self._property.child_node_type]
        # set new node status to source to prevent activation before it's appended
        if hasattr(node_cls, "new"):
            node = node_cls.new(*args, **kwargs, for_parent=self._parent, _status=NS.SOURCE)
        else:
            node = node_cls(*args, **kwargs, _status=NS.SOURCE)
        if _append:
            self.append(node)
        return node

    def create_many(self, *nodes: Collection[typing.Any | dict]) -> list[NodeT]:
        """Creates a new node in the list."""
        created = []
        for n in flatten(nodes):
            if isinstance(n, dict):
                node = self.create(**n, _append=False)
            elif isinstance(n, tuple):
                node = self.create(*n, _append=False)
            else:
                node = self.create(n, _append=False)
            created.append(node)
        self.extend(*created)
        return created

    def append(self, node: NodeT, _create: bool = True, _trigger: _NC = _NC.Full) -> None:
        """
        Attaches a child node to a parent through a list. This is for users adding nodes.
        A node may be 'append'-ed to a list at most once,
         but may exist in multiple lists (through _init_from collection).
        """
        raise NotImplementedError

    def extend(
        self,
        *nodes: Collection[NodeT],
        _create: bool = True,
        _trigger: _NC = _NC.Full,
    ):
        """Attaches a list of child nodes to a parent. See append."""
        raise NotImplementedError

    def remove(self, node: NodeT, _delete: bool = True, _trigger: _NC = _NC.Full):
        """Removes a child node from a parent. See append for reverse."""
        raise NotImplementedError

    def clear(self, _delete: bool = True, _trigger: _NC = _NC.Full):
        """Removes all child nodes from a parent. See append for reverse."""
        raise NotImplementedError

    def set(self, nodes: Collection[NodeT], _trigger: _NC = _NC.Full):
        """Replaces all child nodes of a parent."""
        self.clear(_trigger=_NC.Ignore)
        self.extend(*nodes, _trigger=_trigger)

    def get(self, some_id: str) -> Optional[NodeT]:
        """Gets a node by some id (as determined by the logic of the list)."""
        raise NotImplementedError

    def index(self, node: NodeT) -> int:
        """Gets the index of a node in the list."""
        raise NotImplementedError


class NodeList(NodeListBase[NodeT]):
    """
    A list of node descendants for a parent's property.
    This is the primary way of adding, removing and accessing inline node relations.
    """

    def __init__(self, parent: "ScopeNode", property: Property):
        super().__init__(parent, property)
        self._child_node_type: NodeType = property.child_node_type
        self._flags = property.children_flags
        self._nodes: list[NodeT] = []

    if DEBUG:
        # for debugger inspection
        nodes = property(lambda self: self._nodes)

    def __str__(self):
        return str(self._nodes)

    def _scope(self) -> dict[str, "Node"]:
        """Gets the visible scope for error reporting"""
        if self._flags & NRel.Named:
            return {n.py_ident: n for n in self._nodes}
        return {}

    def _ok_bounds(
        self, after: NodeT = None, before: NodeT = None
    ) -> tuple[Optional[str], Optional[str]]:
        """Gets the order key bounds after the given (default to last)."""
        assert self._flags & NRel.Ordered, f"cannot get order key for {self!r}"
        if after is not None:
            next_ok = nextn(
                n.order_key
                for n in self._nodes
                if n.order_key > after.order_key and n.parent == after.parent
            )
            return after.order_key, next_ok
        elif before is not None:
            last_ok = nextn(
                n.order_key
                for n in reversed(self._nodes)
                if n.order_key < before.order_key and n.parent == before.parent
            )
            return last_ok, before.order_key
        else:
            last_ok = nextn(
                (n.order_key for n in reversed(self._nodes) if n.parent == self._parent)
            )
            return last_ok, None

    def _update(self, scope: "ScopeNode"):
        # _children is effectively a computed property which is replaced wholesale,
        # we don't do diff updates to keep it simple with all the relation types.
        if self._flags & NRel.Cumulative:
            # all matching children of parent's descendants
            #  e.g. Module->Issue, File->Issue, ... -> all issues
            self._nodes = scope._local_root_tree.get_descendants(
                scope.ck, self._child_node_type, recursive=True, prefilter=False
            )
            assert not self._flags & NRel.Ordered, f"cannot order cumulative {self}"
        elif self._flags & NRel.Flat:
            # all matching descendants of matching children of parent
            #  e.g. Module->File, File->File, ... -> all files
            self._nodes = scope._local_root_tree.get_descendants(
                scope.ck, self._child_node_type, recursive=True, prefilter=True
            )
            if self._flags & NRel.Ordered:
                self._nodes = _sort_nested_ordered_list(self._parent.ck, self._nodes)
        else:
            # only matching children of parent
            self._nodes = scope._local_root_tree.get_descendants(
                scope.ck, self._child_node_type, recursive=False
            )
            if self._flags & NRel.Ordered:
                self._nodes.sort(key=lambda n: n.order_key or BIGGEST_INTEGER)

    def append(
        self,
        _node: NodeT,
        _create: bool = True,
        after: NodeT = None,
        before: NodeT = None,
        _trigger: _NC = _NC.Full,
    ) -> list[NodeT]:
        assert isinstance(_node, Node), f"cannot append {_node!r} to {self!r}"
        if _node.parent is not None:
            raise ValueError(f"cannot attach {_node!r} to {self!r}: attached to {_node.parent!r}")

        # assign ids if newly attached to the module (ids are derived from ck + module)
        if not _node.attached and self._parent.attached:
            module_id = self._parent.module.id
            for n in _node._walk_rec():
                if n.id is None:
                    n._assign_id(module_id)
        change = _InterpChange._collect(None, self._parent, [_node], _trigger)
        # update parent after updating ids (the above walks tree, which is changed here)
        _node.parent = self._parent
        # validate node now that it has a parent (while in session)
        if self._parent._session is not None:
            _node._validate_self(_node.__tracked_properties__.keys(), on_invalid=on_invalid_raise)

        # index node into parent scope
        if isinstance(_node, ScopeNode) and _node._local_tree is not None:
            # subsume if previously detached (ignores out of line nodes)
            added = _node._local_tree.get_descendants(_node.ck, recursive=True, include_self=True)
            _node._local_tree.update(_node)  # parent changed
            self._parent._import_scope_tree(_node)
            _node._local_tree = None
        else:  # or just add
            added = [_node]
            self._parent._local_root_tree.add(_node)

        # register node scope
        if (
            self._flags & NRel.Scoped
            and _node.name
            and (not self._flags & NRel.Flat or _node.parent == self._parent)
        ):
            self._parent._add_node_to_scope(_node)

        # assign order key to ordered nodes
        if self._flags & NRel.Ordered and _node.order_key is None:
            _node.order_key = generate_key_between(*self._ok_bounds(after, before))
        # update affected nodes
        if _trigger:
            # and update every affected node (to list/interp as needed)
            change._effect(_trigger)
            assert _node in self._nodes, f"node {_node!r} not in {self!r}"

        # 'create' node in session if it's attached
        if _create and self._parent._session and self._parent.attached:
            self._parent._session.create(*added)
        # temporarily hoisted records may no longer be in tree, so return our added nodes
        return added

    def extend(
        self,
        *nodes: NodeT,
        _create: bool = True,
        after: NodeT = None,
        before: NodeT = None,
        _trigger: _NC = _NC.Full,
    ):
        nodes = flatten(*nodes)
        if not nodes:
            return

        # pre-assign order keys since we don't trigger between appends (meaning last_ok is wrong)
        if self._flags & NRel.Ordered:
            oks = generate_n_keys_between(*self._ok_bounds(after, before), n=len(nodes))
            for node, ok in zip(nodes, oks):
                node.order_key = ok

        # as in append but batched: append, trigger, create
        #  (can we merge them somehow to simplify)?
        change = _InterpChange._collect(None, self._parent, nodes, _trigger)
        change._effect(_trigger & ~_NC.Attach)
        added = []
        for node in nodes:
            added.extend(self.append(node, _create=False, _trigger=_NC.Ignore))
        change._effect(_trigger & ~_NC.Detach)
        if _trigger & _NC.UpdateLists:
            assert all(n in self._nodes for n in nodes), f"nodes {nodes} not in {self!r}"
        if _create and self._parent._session and self._parent.attached:
            self._parent._session.create(*added)

    def remove(self, _node: NodeT, _delete: bool = True, _trigger: _NC = _NC.Full):
        change = _InterpChange._collect(self._parent, None, [_node], _trigger)
        if _delete and self._parent._session:
            self._parent.session.delete(_node)
        self._parent._local_root_tree.remove(_node)
        _node.parent = None
        change._effect(_trigger)
        if _trigger & _NC.UpdateLists:
            assert _node not in self._nodes, f"node {_node!r} still in {self!r}"

    def clear(self, _delete: bool = True, _trigger: _NC = _NC.Full):
        if not self._nodes:
            return
        change = _InterpChange._collect(self._parent, None, self._nodes, _trigger)
        removed = list(self._nodes)
        for _node in removed:
            self.remove(_node, _delete=_delete, _trigger=_NC.Ignore)
        change._effect(_trigger)
        if _trigger & _NC.UpdateLists:
            assert not self._nodes, f"{self!r} is not empty"

    def get(self, some_id: str) -> Optional[NodeT]:
        if not (self._flags & NRel.Keyed) and not (self._flags & NRel.Named):
            raise ValueError(f"cannot get {some_id!r} from {self!r}")
        for child in self._nodes:
            if (self._flags & NRel.Keyed and child.key == some_id) or (
                self._flags & NRel.Named and (child.name == some_id or child.py_ident == some_id)
            ):
                return child
        return None

    def index(self, node: NodeT) -> int:
        return self._nodes.index(node)

    def __bool__(self):
        return bool(self._nodes)

    def __contains__(self, obj: object) -> bool:
        # special case to unwrap key (e.g. for tagging/tag objects)
        if self._flags & NRel.Keyed and hasattr(obj, "key"):
            obj = obj.key
        if isinstance(obj, str) and (self._flags & NRel.Keyed or self._flags & NRel.Named):
            return self.get(obj) is not None
        elif isinstance(obj, Node):
            if obj.metatype != self._property.child_node_type:
                raise TypeError(f"{self!r} cannot contain {obj!r}")
            return obj in self._nodes
        else:
            return False

    def __getitem__(self, item: int | slice | str) -> NodeT | list[NodeT]:
        if isinstance(item, int):
            return self._nodes[item]
        elif isinstance(item, slice):
            return self._nodes[item]
        elif isinstance(item, str):
            return self.get(item)
        else:
            raise TypeError(f"invalid index for {self!r}: {item} ({type(item)})")

    def __getattr__(self, item):
        if item.startswith("_"):
            return super().__getattr__(item)
        node = self.get(item)
        if node is None:
            raise AttributeError(f"no node '{item}' in {self!r}")
        return node

    def __iter__(self) -> Iterator[NodeT]:
        yield from self._nodes

    def __len__(self) -> int:
        return len(self._nodes)

    def __eq__(self, other: object) -> bool:
        if isinstance(other, NodeList):
            return self._nodes == other._nodes
        elif isinstance(other, list):
            return self._nodes == other
        else:
            return False


def _make_self_method(
    method: ComponentMethod,
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


def _make_inner_dunder_method(method: ComponentMethod):
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
    __reserved_properties__: ClassVar[set[int | str]] = set()
    __is_indexed_in_os__: ClassVar[bool] = False  # stored in local OS (only for logs really)

    _status: NodeStatus = struct_runtime(default=None)

    def __post_init__(self):
        if self._status is None:
            from bench.language.session import _active_session

            # not sure if this is totally right... where do we get :StructScope?
            self._status = NS.INTERP if _active_session.get() else NS.SOURCE
        self._init_self()

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
        # in session copy reference keys from references if set :NodeReferences
        for prop in self.__reference_properties__.values():
            ref = getattr(self, prop.name)
            if isinstance(ref, Node):
                self.__dict__[prop.reference_key.name] = ref.ck

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
            ref_key_value = getattr(self, prop.reference_key.name)
            if ref_key_value is not None:
                resolved = scope.resolve(ref_key_value)
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

        if LOCAL and not _should_validate():
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
    _init_self = _make_self_method(ComponentMethod.init, _init_inner)
    _clear_self = _make_self_method(ComponentMethod.clear, _clear_inner, NS.INTERP, NS.SOURCE)
    _interp_self = _make_self_method(ComponentMethod.interp, _interp_inner, NS.SOURCE, NS.INTERP)
    _visit_self = _make_self_method(ComponentMethod.visit, _visit_inner)
    _validate_self = _make_self_method(ComponentMethod.validate, _validate_inner)

    def _walk_self(self) -> typing.Iterable["Struct"]:
        yield self
        for prop in self.__struct_properties__.values():
            value = getattr(self, prop.name)
            if isinstance(value, list):
                for item in value:
                    yield from item._walk_self()
            elif value is not None:
                yield from value._walk_self()

    @staticmethod
    def _make_rec_method(method: ComponentMethod, wraps):
        """Creates method that calls _method_self for all contained structs"""

        @functools.wraps(wraps)
        def rec_method(self: "Node", *args, **kwargs):
            # just walk self, every struct can only appear once
            for descendant in self._walk_self():
                getattr(descendant, method.self)(*args, **kwargs)

        rec_method.__name__ = method.rec
        return rec_method

    _clear_rec = _make_rec_method(ComponentMethod.clear, _clear_self)
    _interp_rec = _make_rec_method(ComponentMethod.interp, _interp_self)
    _visit_rec = _make_rec_method(ComponentMethod.visit, _visit_self)


@node_component
class Node(Struct):
    """
    A node in a Bench module tree - basically struct + identity, so it can relate nodes.
    A node has a per-version unique id (id) and a constant identifier key (ck).
    The id is derived from the module id, so it's only assigned when the node is attached.
    """

    metatype: ClassVar[NodeType]  # type discriminator is field 0 if needed?
    __static_components__: ClassVar[tuple[type["Node"], ...]] = []
    __dynamic_components__: ClassVar[tuple[type["Node"], ...]] = ()
    __static_passthrough__: ClassVar[tuple[tuple[str, _Passthrough]]] = ()

    __properties__: ClassVar[dict[str, Property]] = {}
    __own_properties__: ClassVar[dict[str, Property]] = {}
    __properties_by_id__: ClassVar[dict[int, Property]] = {}
    __ancestor_properties__: ClassVar[dict[str, Property]] = {}
    __list_properties__: ClassVar[dict[str, Property]] = {}
    __list_properties_by_child__: ClassVar[dict[NodeType, list[Property]]] = defaultdict(list)
    __tracked_properties__: ClassVar[dict[str, Property]] = {}
    __internal_properties__: ClassVar[dict[str, Property]] = {}
    __reference_properties__: ClassVar[dict[str, Property]] = {}
    __struct_properties__: ClassVar[dict[str, Property]] = {}
    __stored_properties__: ClassVar[dict[str, Property]] = {}
    __reserved_properties__: ClassVar[set[int | str]] = set()
    __parent_property__: ClassVar[Property] = None

    __has_scope__: ClassVar[bool] = False  # can have node children
    __is_in_module__: ClassVar[bool] = UNSET  # part of a Module
    __is_in_bench__: ClassVar[bool] = UNSET  # part of a Bench
    __root__: ClassVar[NodeType | None] = UNSET
    __is_stored__: ClassVar[bool] = False  # stored in PG (runtime or local)
    __is_stored_custom__: ClassVar[bool] = False  # custom PG storage logic (for records)
    __is_indexed_in_os__: ClassVar[bool] = False  # stored in local OS
    __is_local__: ClassVar[bool] = False  # stored in Bench-local DB (instead of global Bench DB)

    # 1-9: reserved for node identity
    id: UUID = struct_internal(2, default=None, require=True, protect=True, reflect=True)
    # NOTE: ck/module/bench only exist if __is_in_module__/__is_in_bench__ :MagicNodeProps
    ck: UUID = struct_internal(3, default=None, require=True, protect=True, reflect=True)
    parent: Optional["Node"] = node_parent(4)
    module: Optional["Module"] = node_ancestor(
        5, NodeType.MODULE, store=True, wire=True, index_in_pg=True
    )
    bench: Optional["Bench"] = node_ancestor(6, NodeType.BENCH, store=False, wire=True)
    # prototype/template: Optional["Node"] = node_template(7)

    # 10-29: reserved for node tracking
    revision: int = struct_internal(10, default=0, require=True, protect=True, reflect=True)
    created_at: datetime = struct_internal(
        11, default=None, require=True, protect=True, reflect=True
    )
    updated_at: datetime = struct_internal(
        12, default=None, require=True, protect=True, reflect=True
    )
    deleted_at: datetime = struct_internal(13, default=None, protect=True, reflect=True)
    archived_at: datetime = struct_internal(14, default=None, protect=True, reflect=True)
    last_edited_at: datetime = struct_internal(
        15, default=None, require=True, protect=True, reflect=True
    )
    # only scope nodes can have 'inner' changes
    # last_changed_at: datetime = struct_internal(16, default=None, reflect=True)
    # created_by: ... = struct_internal(17, default=None, reflect=True)
    # last_edited_by: ... = struct_internal(18, default=None, reflect=True)
    # last_changed_by: ... = struct_internal(19, default=None, reflect=True)
    # policies: ... = struct_internal(20, default=None, struct_t=StructType.POLICY)

    # 30+ for 'user' node/struct properties
    # <... defined in concrete type ...>

    _session: Optional["Session"] = struct_runtime(default=None)
    _status: NodeStatus = struct_runtime(default=None)
    _track: NodeTrackingLevel = struct_runtime(default=NodeTrackingLevel.FULL)
    _new: bool = struct_runtime(default=False)

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
        """Identifier for dynamic components"""
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
        assert module_id, f"cannot assign id to {self} without a module id"
        assert self.id is None, f"cannot assign id to {self} twice"
        assert self.ck is not None, f"cannot assign id to {self} without ck"
        self.id = get_node_id(module_id, self.ck)

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
            if prop.child_node_type:
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
                if prop.reference_key:  # update reference key  :NodeReferences
                    reference_key_value = value.ck if value is not None else None
                    self.__dict__[prop.reference_key.name] = reference_key_value
                    if self.attached:
                        self._session.update(self, [key])
                        self._updated_self((prop.reference_key.name,))
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
        # prefer components own methods
        for component in self._components:
            if component is self.__class__ or component is Node:
                continue
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
        # attribute be property, method, or just plain value
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

    def _walk_structs(self) -> typing.Iterable["Struct"]:
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

    _call_inner = _make_inner_dunder_method(ComponentMethod.call)
    _iter_inner = _make_inner_dunder_method(ComponentMethod.iter)
    _aiter_inner = _make_inner_dunder_method(ComponentMethod.aiter)
    _len_inner = _make_inner_dunder_method(ComponentMethod.len)
    _getitem_inner = _make_inner_dunder_method(ComponentMethod.getitem)

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
        existing_lists: dict[str, typing.Any] | None = None
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
            self._components, ComponentMethod.init, self._instance_cache_key
        ):
            meth(self)

        # keep manually set node lists if passed in
        if existing_lists:
            changed_nodes: list[NodeT] = []
            was_interp = self._status >= NS.INTERP
            detach_trigger = _NC.UpdateLists | _NC.Detach if was_interp else _NC.UpdateLists
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
    _index_self = _make_self_method(ComponentMethod.index, _index_inner, NS.SOURCE, NS.INDEX)
    _clear_self = _make_self_method(ComponentMethod.clear, _clear_inner, NS.INTERP, NS.SOURCE)
    _interp_self = _make_self_method(ComponentMethod.interp, _interp_inner, NS.SOURCE, NS.INTERP)
    _activate_self = _make_self_method(
        ComponentMethod.activate, _activate_inner, NS.INTERP, NS.ACTIVE
    )
    _deactivate_self = _make_self_method(
        ComponentMethod.deactivate, _deactivate_inner, NS.ACTIVE, NS.INTERP
    )
    _attached_self = _make_self_method(ComponentMethod.attached, _attached_inner)
    _detached_self = _make_self_method(ComponentMethod.detached, _detached_inner)
    _updated_self = _make_self_method(ComponentMethod.updated, _updated_inner)

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

    @property
    def attached(self) -> bool:
        return self.parent is not None and self.module is not None

    @property
    def scope(self) -> Optional["ScopeNode"]:
        return self.parent

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

    @property
    def logger(self) -> Logger:
        return self.session._log


def _make_rec_method(
    method: ComponentMethod, wraps, custom_kwargs: Callable[["Node"], dict] = None
):
    """Creates method that calls _method_self for self and all descendants"""

    @functools.wraps(wraps)
    def rec_method(self: "ScopeNode", *args, **kwargs):
        # tree has only host and inlined nodes, so this ignores out-of-line descendants (like records)
        descendants = self._local_root_tree.get_descendants(self.ck, recursive=True)
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
    last_changed_at: datetime = struct_internal(16, default=None, reflect=True)
    issues: NodeList["Issue"] = node_children(NodeType.ISSUE, NRel.Cumulative)
    _scopes_by_name: dict[str, "ScopeNode"] = struct_runtime(default_factory=dict)
    _names_by_ident: dict[str, str] = struct_runtime(default_factory=dict)
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
            _InterpChange._collect(self.parent, self.parent, [self], _NC.Full)._effect(_NC.Full)

    _clear_rec = _make_rec_method(
        ComponentMethod.clear, Node._clear_self, custom_kwargs=lambda n: dict(scope=n.scope)
    )
    _index_rec = _make_rec_method(ComponentMethod.index, Node._index_self)
    _interp_rec = _make_rec_method(
        ComponentMethod.interp,
        Node._interp_self,
        custom_kwargs=lambda n: dict(
            scope=n.scope, on_issue=n.scope._on_issue if n.scope else on_issue_raise
        ),
    )
    _visit_rec = _make_rec_method(ComponentMethod.visit, Node._visit_self)
    _validate_rec = _make_rec_method(
        ComponentMethod.validate,
        Node._validate_self,
        custom_kwargs=lambda n: dict(
            properties=n.__tracked_properties__.keys(), on_invalid=on_invalid_raise
        ),
    )
    _activate_rec = _make_rec_method(ComponentMethod.activate, Node._activate_self)
    _deactivate_rec = _make_rec_method(ComponentMethod.deactivate, Node._deactivate_self)

    def _get_scope(self, name: str, by: Optional[LookupBy]) -> Union["ScopeNode", None]:
        if by is None and name in self._scopes_by_name or by == LookupBy.Name:
            return self._scopes_by_name.get(name)
        if by is None and name in self._names_by_ident or by == LookupBy.PyIdent:
            if name in self._names_by_ident:
                name = self._names_by_ident[name]
                return self._scopes_by_name.get(name)
        return None

    def _find_scope(self, name: str, by: Optional[LookupBy]) -> Union["ScopeNode", None]:
        scope = self._get_scope(name, by)
        if scope is not None:
            # check that we're not resolving something from an out-of-sync cache
            assert scope.attached == self.attached, f"{scope!r} isn't in the same tree as {self!r}"
            return scope
        if self.parent is not None:
            return self.parent._find_scope(name, by=by)
        return None

    def _update_lists(self, scope: "ScopeNode"):
        for prop in self.__list_properties__.values():
            getattr(self, prop.name)._update(scope)

    def _clear_inner(self, scope: Optional["ScopeNode"] = None):
        self._scopes_by_name = {}
        self._names_by_ident = {}

    def _index_inner(self) -> None:
        for prop in self.__list_properties__.values():
            if prop.children_flags & NRel.Scoped:
                for child in getattr(self, prop.name):
                    if child.name and (not prop.children_flags & NRel.Flat or child.parent == self):
                        self._add_node_to_scope(child)

    def _walk_rec(self) -> Collection["Node"]:
        return self._local_root_tree.get_descendants(self.ck, recursive=True, include_self=True)

    def _add_node_to_scope(self, node: Node) -> None:
        """
        Adds a child node into this scope. Idempotent for the same node.
        """
        if node.name in self._scopes_by_name or node.py_ident in self._names_by_ident:
            if node.py_ident in self._names_by_ident:
                existing = self._scopes_by_name[self._names_by_ident[node.py_ident]]
            else:
                existing = self._scopes_by_name[node.name]
            if existing.id != node.id:
                self._on_issue(type=IssueType.AMBIGUOUS_DEFINITION, subject=node, path=node.path)
        else:
            self._scopes_by_name[node.name] = node
            self._names_by_ident[node.py_ident] = node.name

    def _import_scope_tree(self, scope: "ScopeNode") -> None:
        """Adds the given tree into this scope."""
        assert scope._local_tree is not None, f"no local tree to import {scope!r} into {self!r}"
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
        by: Optional[LookupBy] = None,
        node_t: NodeType | StatementType | typing.Type[NodeT] | None = None,
    ) -> NodeT | None:
        """
        Lookup the symbol either by path or id. If path is a string, it can be
        it can be a name (lookup upwards) or a full relative/absolute path.
        """
        if isinstance(path, UUID):
            if self._local_tree is not None:
                return self._local_tree.get(path)
            else:
                return self._local_root_scope.lookup(path, by=by, node_t=node_t)

        if isinstance(path, str):
            path = parse_node_path(path)
        if path.path == ".":
            return self._find_scope(path.name, by)
        elif path.path.startswith("."):
            path = NodePath(path.path[1:], path.name)
        parts = path.path.split(".", 2)
        if len(parts) > 1:
            first_part, inner_part = parts[0], NodePath(parts[1], path.name)
        else:
            first_part, inner_part = parts[0], path.name
        scope = self._find_scope(first_part, by=by)
        if scope is None:
            return None
        return scope.lookup(inner_part, node_t=node_t, by=by)

    def resolve(
        self,
        path: Union["NodePath", UUID, str],
        by: Optional[LookupBy] = None,
        node_t: typing.Type[NodeT] | None = None,
    ) -> NodeT:
        result = self.lookup(path, by=by, node_t=node_t)
        if result is None:
            raise LookupError(f"{path} not found in {self!r}")
        return result

    def _get_visible_scopes(self) -> dict[str, "ScopeNode"]:
        """Returns the scopes visible from this node."""
        scopes = {**self._scopes_by_name}
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
        if issue not in issue.subject.issues:  # dedup
            issue.subject.issues.append(issue, _trigger=_NC.UpdateLists)

    @property
    def errors(self) -> list["Issue"]:
        if self.issues is None:
            return []
        return [i for i in self.issues or [] if i.kind == IssueKind.ERROR]

    @property
    def self_errors(self):
        return [i for i in self.errors or [] if i.parent == self]


@node(NodeType.BENCH, in_module=False)
class Bench(ScopeNode):
    """
    A Bench contains everything a young and growing AI needs to learn and grow.
    """

    parent: None = node_parent(4)
    policies: Optional[list["Policy"]] = struct_internal(
        20, default_factory=list, struct_t=StructType.POLICY
    )
    name: str = struct_internal(30)
    slug: str = struct_internal(31, protect=True)
    description: str = struct_internal(32, default=None)
    organization: Optional["Organization"] = struct_internal(
        33, protect=True, array=False, references=NodeType.ORGANIZATION
    )
    user: Optional["User"] = struct_internal(
        34, protect=True, array=False, references=NodeType.USER
    )

    # *per* environment stuff (will be moved into Environment or such later)
    head = struct_internal(40, protect=True, array=False, references=NodeType.MODULE)
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

    worker_sets: NodeList["WorkerSet"] = node_children(NodeType.WORKER_SET, NRel.Flat)

    # versions: NodeList["Module"] = node_children(NodeType.MODULE, NRel.Remote)

    @property
    def owner(self) -> Union["Organization", "User"]:
        return self.organization or self.user

    @property
    def attached(self) -> bool:
        return True  # always "attached"


@dataclass
class ModuleChange:
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
    def touched(self) -> typing.Iterable[Node]:
        return chain(self.added, self.updated, self.removed)

    @staticmethod
    def empty() -> "ModuleChange":
        return ModuleChange([], [], [], [], [])


@node(NodeType.MODULE, passthrough=(("files", _Passthrough.Full),))
class Module(ScopeNode):
    """A module is a semi-isolated version of a Bench, containing the actual files and so on."""

    parent: Bench = node_parent(4, NodeType.BENCH)
    policies: Optional[list["Policy"]] = struct_internal(
        20, default_factory=list, struct_t=StructType.POLICY
    )
    is_main: bool = struct_internal(
        31, protect=True, default=False, store=False
    )  # main environment?
    is_snapshot: bool = struct_internal(32, protect=True, default=False)  # snapshot or head?

    files: NodeList["File"] = node_children(NodeType.FILE, NRel.Flat | NRel.Named | NRel.Scoped)
    dependencies: dict[str, "Module"] = struct_runtime(default_factory=dict)
    builtins: list["File"] = struct_runtime(default_factory=list)

    _lookup_cache: dict[str, NodeT] = struct_runtime(default_factory=dict)
    _source: Optional[NodeTree] = struct_runtime(default=None)

    def __str__(self):
        if self.issues:
            issue_strs = []
            for k in (IssueKind.ERROR, IssueKind.WARNING, IssueKind.NOTICE):
                issues_of_kind = [i for i in self.issues if i.kind == k]
                if issues_of_kind:
                    issue_strs.append(f"{len(issues_of_kind)} {k.name.lower()}s")
            issues_str = f", {', '.join(issue_strs)}"
        else:
            issues_str = ""
        return f"{self.name} ({len(self.files)} files{issues_str})"

    def __repr__(self):
        return f"<Module {str(self)}>"

    @property
    def name(self):
        return self.parent.name

    @property
    def pg_name(self) -> str:
        return self.parent.pg_name

    @property
    def os_name(self) -> str:
        return self.parent.os_name

    @property
    def _tree(self) -> NodeTree:
        return self._local_tree

    @property
    def _nodes(self) -> Collection[Node]:
        return self._tree.nodes_by_ck.values()

    @property
    def attached(self) -> bool:
        return True  # always "attached"

    @property
    def path(self) -> str:
        return self.py_ident

    @property
    def py_ident(self) -> str:
        return to_pyidentifier(self.name, IdentifierType.PATH)

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
        by: Optional[LookupBy] = None,
        node_t: NodeType | typing.Type[NodeT] | None = None,
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
            resolved = ScopeNode.lookup(self, path, node_t=node_t, by=by)
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
                resolved = dependency.lookup(sub_path, node_t=node_t, by=by)

        if not resolved:
            return resolved
        assert resolved.attached, f"resolved {path} to detached {resolved!r} (index out of sync?)"
        # cache result
        if self.committed:
            self._lookup_cache[path] = resolved
        return resolved

    def _get_visible_scopes(self) -> dict[str, "ScopeNode"]:
        scopes = {**self._scopes_by_name}
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

    def _apply_edits(
        self, edits: list[EditData], old_source: NodeTree | None = None
    ) -> ModuleChange:
        """
        Applies the given external edits to the module.
        TODO @Performance @UX: :HotReload patch edits directly?
        """
        assert self._source is not None, f"cannot apply edits to {self!r} without source"

        if not edits:
            return ModuleChange.empty()

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
            try:
                self._source.apply_edit(edit)
            except ValueError as e:
                if edit.node_type not in INTERP_NODE_TYPES:
                    raise ValueError(f"failed to apply edit {edit!r} to {self!r}") from e
                # interp errors are fine here since e.g. a deleted issue's parent may have disappeared
                #  (we could filter that, but it's easier not to, the edits are explicit for clients)

    def _compute_change(
        self,
        source_edits: list[EditData],
        old_source: NodeTree,
    ) -> ModuleChange:
        """Computes the change between the old and new module state."""
        new_nodes: dict[UUID, Node] = self.module._tree.nodes_by_ck
        added = []
        updated = []
        for n in new_nodes.values():
            if n.ck in old_source.nodes_by_ck:
                if (
                    n.metatype not in INTERP_NODE_TYPES
                    and n.revision != old_source.nodes_by_ck[n.ck].revision
                ):
                    updated.append(n)
            else:
                added.append(n)
        removed = [n for n in old_source.nodes_by_ck.values() if n.ck not in new_nodes]

        # gather interp edits (delete from old, create in new)
        old_editor = NodeTreeEditor(old_source, self._bench_id, self.id)
        for node in removed:
            # :InterpEditFilter
            if node.metatype in INTERP_NODE_TYPES:
                if node.ck not in old_source.nodes_by_ck:
                    # need to investigate
                    logger.warning(f"node {node!r} not found in old source for {self!r}")
                    continue
                # recover parent info from old source
                old_editor.delete(old_source.nodes_by_ck[node.ck])
        new_editor = NodeTreeEditor(self._source, self._bench_id, self.id)
        for node in added:
            if node.metatype in INTERP_NODE_TYPES:
                new_editor.create(node)

        return ModuleChange(
            source_edits=source_edits,
            interp_edits=new_editor.edits + old_editor.edits,
            added=added,
            updated=updated,
            removed=removed,
        )

    @staticmethod
    def make(source: list["SomeNodeData"]) -> "Module":
        """Create an interpreted Module from a source module node tree."""
        from bench.proto import wiring

        source = [wiring.unwrap_some_node(s) for s in source]
        source = NodeTree(source)
        module = wiring.unpack_node_inline(source, parent=None, exclude=INTERP_NODE_TYPES)
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


_BENCH_TYPES_BY_NAME: dict[str, type[Node | Struct | enum.Enum]] = {}
BENCH_TYPES: frozenset[type[Node | Struct | enum.Enum]] = frozenset()
NODE_TYPES: frozenset[type[Node]] = frozenset()
STRUCT_TYPES: frozenset[type[Struct]] = frozenset()


def _complete_bench_setup():
    """Finalize setup of all language constructs after everything is imported."""
    global BENCH_TYPES
    global NODE_TYPES
    global STRUCT_TYPES
    from bench.language import const

    # populate known types
    for bench_t in chain(NODE_CLASS_BY_NODE_TYPE.values(), STRUCT_CLASS_BY_STRUCT_TYPE.values()):
        _BENCH_TYPES_BY_NAME[bench_t.__name__] = bench_t
    for maybe_bench_t in const.__dict__.values():
        if isinstance(maybe_bench_t, type) and issubclass(maybe_bench_t, enum.Enum):
            _BENCH_TYPES_BY_NAME[maybe_bench_t.__name__] = maybe_bench_t
    BENCH_TYPES = frozenset(_BENCH_TYPES_BY_NAME.values())
    for node_t in NodeType:
        BENCH_CLASS_BY_TYPE[node_t] = NODE_CLASS_BY_NODE_TYPE[node_t]
    for struct_t in StructType:
        BENCH_CLASS_BY_TYPE[struct_t] = STRUCT_CLASS_BY_STRUCT_TYPE[struct_t]
    NODE_TYPES = frozenset(NODE_CLASS_BY_NODE_TYPE.values())
    STRUCT_TYPES = frozenset(STRUCT_CLASS_BY_STRUCT_TYPE.values())

    # misc finalization on properties
    for cls in chain(get_subclasses(Node), get_subclasses(Struct)):
        is_node = issubclass(cls, Node)
        for name, prop in cls.__properties__.items():
            prop: Property
            # determine final storage type
            prop.finalize_type()

            # set reflected properties
            if prop.is_reflected:
                setattr(cls, name, prop)
                prop._as_field  # noqa ensure the reflected field works (and cache it)

            # check deferred/encrypted properties
            if prop.is_deferred and not prop.is_stored:
                raise ValueError(f"{prop!r} cannot be deferred and not stored on {cls!r}")
            if not is_node and (prop.is_deferred or prop.is_encrypted):
                raise ValueError(f"{prop!r} cannot be deferred or encrypted on {cls!r}")

            # check py_type matches struct type as defined
            if (
                not prop.is_tree_relation
                and not prop.is_node_reference
                and isinstance(prop.py_type_raw, type)
                and issubclass(prop.py_type_raw, Struct)
            ):
                if not prop.struct_type:
                    raise ValueError(
                        f"cannot store {prop!r} as {prop.py_type_raw!r} (missing struct_type)"
                    )
                if STRUCT_CLASS_BY_STRUCT_TYPE[prop.struct_type] is not prop.py_type_raw:
                    raise ValueError(f"{prop!r} {prop.struct_type} != {prop.py_type_raw}")

        # set stored properties now that storage info is determined
        cls.__stored_properties__ = frozendict(
            {p.name: p for p in cls.__properties__.values() if p.is_stored is True}
        )

    # determine node ancestry (is in module/bench)
    #  (to check if it was set consistently - we need to set this manually in @node
    #   because we can only walk parent types after finalization)
    def _has_module_ancestor(node_type: NodeType) -> bool:
        if node_type == NodeType.MODULE:
            return True
        for parent_type in NODE_CLASS_BY_NODE_TYPE[node_type].__parent_property__.parents:
            if parent_type == NodeType.MODULE:
                return True
            if parent_type != node_type:
                return _has_module_ancestor(parent_type)
        return False

    for node_cls in NODE_CLASS_BY_NODE_TYPE.values():
        if node_cls.__root__ is None or node_cls.__root__ != NodeType.BENCH:
            in_bench = False
            in_module = False
        else:  # check if node is in module
            in_bench = True
            in_module = _has_module_ancestor(node_cls.metatype)
        if in_bench != node_cls.__is_in_bench__ or in_module != node_cls.__is_in_module__:
            raise ValueError(
                f"{node_cls!r} parent types are inconsistent: root={node_cls.__root__} implies in_bench={in_bench} and in_module={in_module}, but got in_bench={node_cls.__is_in_bench__} and in_module={node_cls.__is_in_module__}"
            )
    in_bench_types = [t.metatype for t in NODE_CLASS_BY_NODE_TYPE.values() if t.__is_in_bench__]
    in_module_types = [t.metatype for t in NODE_CLASS_BY_NODE_TYPE.values() if t.__is_in_module__]
    assert set(IN_BENCH_NODE_TYPES) == set(in_bench_types), f"IN_BENCH_NODE_TYPES inconsistent"
    assert set(IN_MODULE_NODE_TYPES) == set(in_module_types), f"IN_MODULE_NODE_TYPES inconsistent"

    # check that all enum types are valid proto-able enums
    for struct_t in chain(STRUCT_CLASS_BY_STRUCT_TYPE.values(), NODE_CLASS_BY_NODE_TYPE.values()):
        for prop in struct_t.__properties__.values():
            if prop.is_enum and not issubclass(
                prop.py_type_stripped, (ProtoStrEnum, enum.IntEnum, enum.IntFlag)
            ):
                raise ValueError(f"{prop!r} is not a valid proto enum")
