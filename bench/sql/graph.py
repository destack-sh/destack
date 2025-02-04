import datetime
import struct
from collections import defaultdict
from dataclasses import dataclass
from itertools import chain
from typing import (
    Any,
    Collection,
    Mapping,
    Sequence,
    assert_never,
    cast,
    override,
)
from uuid import UUID

import psycopg
import pytz
from bitarray import bitarray
from google.protobuf.duration_pb2 import Duration
from google.protobuf.message import Message as ProtoMessage
from google.protobuf.struct_pb2 import Value as ProtoValue
from google.protobuf.timestamp_pb2 import Timestamp
from more_itertools import first
from opentelemetry import trace
from psycopg import sql
from psycopg.types.json import Jsonb
from pydantic import JsonValue

from bench import pb2
from bench.language import (
    CASCADING_EDIT_TYPES,
    DESCENDANT_NODE_TYPES_IN_STORE,
    HAS_CHILD_NODE_TYPES,
    NODE_CLASS_BY_TYPE,
    NODE_CLASSES,
    NODE_TYPES,
    PARENT_NODE_TYPES,
    PROPERTY_META_KEY_BY_TYPE,
    UNSET,
    Bench,
    Block,
    C,
    ConditionalType,
    EditOperationType,
    EditType,
    EngineIncapableError,
    EnumType,
    Expression,
    ExpressionTypes,
    Field,
    FieldType,
    LiteralType,
    Node,
    NodeArea,
    NodeDataGraph,
    NodeReference,
    NodeType,
    PrimitiveType,
    PrimitiveValue,
    Property,
    PropertyReferenceType,
    Query,
    QueryType,
    Record,
    ReferenceKind,
    SelectOptions,
    SortType,
    TypeKind,
    bittuple,
    get_default_query_filter,
    pack_builtin_object_data,
    pack_proto_date,
    pack_proto_json,
    pack_proto_time,
    pack_value,
    unpack_builtin_object_data,
    unpack_proto_date,
    unpack_proto_json,
    unpack_proto_json_struct,
    unpack_proto_time,
    unpack_value,
    unpack_value_data,
)
from bench.pb2 import (
    AnyNodeData,
    Date,
    EditData,
    GraphScopeData,
    NodeReferenceData,
    TimeOfDay,
)
from bench.proto import wiring
from bench.proto.wiring import PROTO_CLASS_BY_TYPE
from bench.utils.env import IS_DEV
from bench.utils.func import describe_type, group_by, to_uuid
from bench.utils.string import Casing, to_casing
from bench.utils.time import timedelta_from_isoformat
from bench.utils.uuidt import UUIDT

from . import schema
from .client import GLOBAL_PG_CRYPTO_KEY
from .core import (
    DEFAULT_GLOBAL_TABLES,
    DEFAULT_LOCAL_TABLES,
    DEFAULT_REGIONAL_TABLES,
    GLOBAL_EXTENSIONS,
    LOCAL_EXTENSIONS,
    PG_CONDITIONAL_OP_BY_BENCH,
    POSTGRES_SORT_OP_BY_BENCH,
    REGIONAL_EXTENSIONS,
    CascadeAction,
    Column,
    Constraint,
    ConstraintType,
    Index,
    IndexType,
    PostgresConditionalOp,
    Schema,
    SqlPrimitive,
    Table,
)
from .engine import (
    RowIn,
    RowOut,
    SqlComparison,
    SqlCompound,
    SqlContext,
    SqlNode,
    SqlUnary,
    _trace_pg_span,
    pg_count,
    pg_delete,
    pg_exists,
    pg_insert,
    pg_select,
    pg_update_variable,
    pg_upsert,
    sql_node_to_sql,
    sqlident,
    sqljoin,
    sqlstr,
)

GLOBAL_CONTEXT = SqlContext()
BENCH_TABLE_PREFIX = "bench_"
BENCH_RECORD_TABLE_PREFIX = "bench_record_"
BENCH_RECORD_VALUE_PREFIX = "value_"


@dataclass(slots=True)
class BenchSqlContext(SqlContext):
    """Host context for SQL operations (with custom databases)."""

    bench: Bench

    @override
    def get_crypto_key(self, obj: Table | Column) -> str | None:
        table = obj.table
        node_type = BUILTIN_NODE_BY_TABLE_NAME.get(table.name)
        if node_type is None:
            return None
        node_cls = NODE_CLASS_BY_TYPE[node_type]
        if node_cls.__is_in_package__:
            return self.bench.encryption_key
        else:
            return GLOBAL_PG_CRYPTO_KEY


#
# Mapping
#


def get_node_table_name(node_type: NodeType) -> str:
    return f"{BENCH_TABLE_PREFIX}{node_type.name.lower()}"


def get_record_table_name(block: Block) -> str:
    return f"{BENCH_RECORD_TABLE_PREFIX}{block.tk}"


def get_record_field_name(field: Field) -> str:
    return f"{BENCH_RECORD_VALUE_PREFIX}{field.tk}{field.identity_key}"


def map_builtin_object_to_table(
    node: type[Node], properties: list[Property] | None = None
) -> Table:
    """Maps a node type into its builtin Table schema."""
    table_name = get_node_table_name(node.metatype)
    columns: list[Column] = []
    constraints: list[Constraint] = []
    indexes: list[Index] = []
    properties = properties or list(node.__properties__.values())
    properties.sort(key=lambda p: p.id or -1)

    # map properties to columns, add per-column indices
    for prop in properties:
        if not prop.is_stored:
            continue
        assert prop.primitive_type is not None, f"no primitive type for {prop!r}"
        column = Column(
            name=prop.name,
            type=prop.primitive_type,
            is_array=prop.is_list,
            is_nullable=not prop.is_required,
            is_encrypted=prop.is_encrypted,
            is_primary_key=prop.name == "id",
            is_unique=prop.is_unique,
            _source=prop.id,
        )
        # default
        if prop.default_sql is not UNSET:
            column.default = prop.default_sql
        elif prop.default is not UNSET and prop.default is not None:
            if isinstance(prop.default, bool):
                column.default = "false" if prop.default is False else "true"
            elif isinstance(prop.default, (int, float)):
                column.default = str(prop.default)
            elif isinstance(prop.default, str):
                column.default = f"'{prop.default}'::character varying"
            else:
                raise TypeError(f"unexpected default in {prop!r}: {prop.default!r}")
        # FKs
        reference_nodes = (
            prop.reference_nodes or () if prop.reference_nodes != "any" else NODE_TYPES.tuple
        )
        if (
            (prop.reference_kind == ReferenceKind.NODE_PARENT or prop.reference_force_fk)
            and not prop.is_list  # foreign keys must be scalar
            and reference_nodes
            and all(r.area == node.__area__ for r in reference_nodes)
        ):
            assert len(reference_nodes) == 1, f"stored prop {prop!r} has multiple references"
            column.is_foreign_key_to = get_node_table_name(reference_nodes[0])
            if prop.reference_kind in (ReferenceKind.NODE_PARENT, ReferenceKind.NODE_ANCESTOR):
                column.on_delete = CascadeAction.CASCADE
            else:
                column.on_delete = CascadeAction.SET_NULL

        if prop.is_indexed or prop.is_unique:
            index = Index(
                f"bench_idx_{prop.name}",
                type=IndexType.BTREE,
                columns=(column.name,),
                is_unique=prop.is_unique,
                _source=prop.id,
            )
            indexes.append(index)
            if prop.is_unique:
                constraint = Constraint(
                    index.inner_name,  # must be the same as the index name (postgres will rename otherwise)
                    type=ConstraintType.UNIQUE,
                    columns=(column.name,),
                    index=index.inner_name,
                    _source=prop.id,
                )
                constraints.append(constraint)
        columns.append(column)

    # extras
    for index in node.__indexes__:
        assert not index.name or not index.name.startswith(
            "bench_"
        ), f"index shouldn't include prefix: {index!r}"
        extra_index = Index.from_index_in(
            f"bench_idx_{index.name or '_'.join(index.columns)}", index
        )
        indexes.append(extra_index)

    table = Table(
        _source=node.metatype.id,
        name=table_name,
        columns=tuple(columns),
        constraints=tuple(constraints),
        indexes=tuple(indexes),
    )
    return table


def map_database_block_to_table(block: Block, old_table: Table | None) -> Table:
    """
    Maps a DatabaseBlock to its corresponding custom Record Table.
    If a previous table is passed in, all its constructs will exist in the new table
     (if they are not already present in the new table).
    """
    base_table = map_builtin_object_to_table(
        Record,
        # all stored Record properties except value, which we unfurl into columns
        properties=[p for p in Record.__stored_properties__.values() if not p.is_value_packed],
    )
    table_name = get_record_table_name(block)
    columns: list[Column] = [column.clone() for column in base_table.columns]
    constraints: list[Constraint] = [constraint.clone() for constraint in base_table.constraints]
    indexes: list[Index] = [index.clone() for index in base_table.indexes]

    # map fields into columns
    for field in block.fields:
        if field.kind == TypeKind.PRIMITIVE:
            assert field.primitive_type is not None, f"no primitive type for {field!r}"
            primitive_type = field.primitive_type
        elif field.kind == TypeKind.NODE or field.kind == TypeKind.BASED_NODE:
            # NOTE :Architecture: unravel custom field node refs like in builtin objects?
            primitive_type = PrimitiveType.JSON
        elif field.kind == TypeKind.STRUCT:
            primitive_type = PrimitiveType.JSON
        elif field.kind == TypeKind.ENUM:
            primitive_type = PrimitiveType.INT16
        elif field.kind == TypeKind.BASED_NODE:
            primitive_type = PrimitiveType.JSON
        elif field.kind == TypeKind.UNION:
            continue  # not stored directly
        else:
            raise TypeError(f"cannot store field in {block!r}: {field!r}")

        column = Column(
            name=get_record_field_name(field),
            type=primitive_type,
            is_array=field.is_list,
            is_nullable=True,  # NOTE :Incomplete: support field constraints in database
            is_primary_key=False,
            _source=field.tk,
            _field=field,
        )
        columns.append(column)

    # keep old columns
    if old_table is not None:
        columns_by_name: dict[str, Column] = {c.name: c for c in columns}
        for old_column in old_table.columns:
            if old_column.name not in columns_by_name:
                columns.append(old_column.clone())

    return Table(
        _source=block.tk,
        _block=block,
        name=table_name,
        columns=tuple(columns),
        constraints=tuple(constraints),
        indexes=tuple(indexes),
    )


#
# Compilation
#


def _compile_expression_ref(
    node_type: NodeType,
    node_cls: type[Node],
    node_table: Table,
    block: Block | None,
    expr: Expression,
) -> SqlNode:
    prop = expr.property
    if prop is not None:
        return sqlident(prop.name)
    field = expr.field
    if field is not None:
        if field.parent is not block:  # must belong to the block (for now?)
            raise RuntimeError(f"unexpected field {field!r} for {block!r} in {expr!r}")
        column = node_table.get_column(field)
        return sqlident(column.name)
    raise RuntimeError(f"unexpected expression ref: {expr!r}")


def _pg_lower_conditional(
    node_type: NodeType,
    node_cls: type[Node],
    node_table: Table,
    block: Block | None,
    cond: Expression,
) -> Expression:
    """'Lowers' a conditional expression to a form that can be compiled to SQL."""
    prop = cond.property

    # translate general pointer queries into underlying id/ck queries
    if prop is not None and prop.reference_kind is not None:
        assert prop.reference_stored_ids, f"unexpected stored ids: {prop!r}"
        is_list = cond.type == ConditionalType.IN or cond.type == ConditionalType.NOT_IN
        id_prop = prop.reference_stored_ids[0]
        id_clause = C(op=cond.type, property=id_prop)
        if cond.value_packed is not None:
            id_clause.value = cond.value.id if not is_list else [r.id for r in cond.value]
        if (
            prop.reference_stored_metas
            and PropertyReferenceType.CK in prop.reference_stored_metas
            and prop.reference_stored_metas[PropertyReferenceType.CK] is not id_prop
        ):
            ck_prop = prop.reference_stored_metas[PropertyReferenceType.CK]
            ck_clause = C(op=cond.type, property=ck_prop)
            if cond.value_packed is not None:
                ck_clause.value = cond.value.ck if not is_list else [r.ck for r in cond.value]
            joint_clause = C(ConditionalType.OR, clauses=[id_clause, ck_clause])
            return joint_clause
        else:
            return id_clause
    else:
        return cond


def _pg_compile_conditional(
    node_type: NodeType,
    node_cls: type[Node],
    node_table: Table,
    block: Block | None,
    cond: Expression,
) -> SqlNode:
    cond = _pg_lower_conditional(node_type, node_cls, node_table, block, cond)
    if cond.type == LiteralType.TRUE:
        return sqlstr("TRUE")
    elif cond.type == LiteralType.FALSE:
        return sqlstr("FALSE")
    elif cond.type == LiteralType.NONE:
        return sqlstr("NULL")
    elif cond.type in ExpressionTypes.COMPOUND and cond.type in PG_CONDITIONAL_OP_BY_BENCH:
        clauses = [
            _pg_compile_conditional(node_type, node_cls, node_table, block, c)
            for c in cond.clauses or ()
        ]
        if not clauses:
            # and/or/not <nothing> are all TRUE :EmptyCompoundConditional
            return sqlstr("TRUE")
        clause = SqlCompound(op=PG_CONDITIONAL_OP_BY_BENCH[cond.type], operands=clauses)
        return clause
    elif (
        cond.type in ExpressionTypes.COMPOUND
        or cond.type in ExpressionTypes.EXACT
        or cond.type in ExpressionTypes.RANGE
        or cond.type in ExpressionTypes.STRING
    ) and cond.type in PG_CONDITIONAL_OP_BY_BENCH:
        left = _compile_expression_ref(node_type, node_cls, node_table, block, cond)

        if cond.type == ConditionalType.IN:
            # map IN to = ANY() construct (IN/NOT IN doesn't work in psycopg)
            # see https://www.psycopg.org/psycopg3/docs/basic/from_pg2.html#you-cannot-use-in-s-with-a-tuple
            value = list(cond.value) if not isinstance(cond.value, list) else cond.value
            right = sqlstr("ANY({})").format(sql.Literal(value))
            return SqlComparison(left=left, op=PostgresConditionalOp.EQ, right=right)
        elif cond.type == ConditionalType.NOT_IN:
            # and map NOT IN to != ALL() construct (see above)
            value = list(cond.value) if not isinstance(cond.value, list) else cond.value
            right = sqlstr("ALL({})").format(sql.Literal(value))
            return SqlComparison(left=left, op=PostgresConditionalOp.NEQ, right=right)
        elif cond.type == ConditionalType.MATCHES:
            # wrap a % in the value with %
            right = sqlstr("'%' || {} || '%'").format(sql.Literal(cond.value))
        elif cond.type == ConditionalType.STARTS_WITH:
            right = sqlstr("{} || '%'").format(sql.Literal(cond.value))
        elif cond.type == ConditionalType.ENDS_WITH:
            right = sqlstr("'%' || {}").format(sql.Literal(cond.value))
        else:
            assert cond.value is not None, f"cannot compare {cond!r} with None"
            right = sql.Literal(cond.value)

        clause = SqlComparison(left=left, op=PG_CONDITIONAL_OP_BY_BENCH[cond.type], right=right)

        # coerce (x != y) -> (x != y or x IS NULL) if x is nullable
        if cond.type == ConditionalType.NOT_EQUALS and (
            cond.property is not None and not cond.property.is_required
        ):
            null_clause = SqlUnary(
                op=PostgresConditionalOp.IS_NULL,
                left=_compile_expression_ref(node_type, node_cls, node_table, block, cond),
            )
            clause = SqlCompound(op=PostgresConditionalOp.OR, operands=[clause, null_clause])

        return clause
    elif cond.type in ExpressionTypes.EXISTENCE:
        clause = SqlUnary(
            left=_compile_expression_ref(node_type, node_cls, node_table, block, cond),
            op=PG_CONDITIONAL_OP_BY_BENCH[cond.type],
        )
        return clause
    raise EngineIncapableError("postgres", expression=cond, reason="unsupported conditional")


def _pg_compile_sort(
    node_type: NodeType,
    node_cls: type[Node],
    node_table: Table,
    block: Block | None,
    sort: Expression,
) -> SqlNode:
    field_ref = _compile_expression_ref(node_type, node_cls, node_table, block, sort)
    sort_op = POSTGRES_SORT_OP_BY_BENCH[cast(SortType, sort.type)]
    return sqlstr("{} {}").format(sql_node_to_sql(field_ref), sqlstr(sort_op))


def _pg_compile_sorts(
    node_type: NodeType,
    node_cls: type[Node],
    node_table: Table,
    block: Block | None,
    sorts: list[Expression],
) -> sql.Composed:
    return sqljoin(
        ", ", (_pg_compile_sort(node_type, node_cls, node_table, block, s) for s in sorts)
    )


#
# Packing/unpacking
#


def _pack_builtin_object_data_prop_scalar(prop: Property, value: Any) -> SqlPrimitive:
    """Packs the value of a BuiltinObject property for storage in Postgres."""
    if value is None:
        return None
    elif prop.is_struct:
        value = pack_builtin_object_data(value)
        return Jsonb(value)  # type: ignore
    elif prop.primitive_type == PrimitiveType.UUID:
        return to_uuid(value)
    elif prop.primitive_type == PrimitiveType.JSON:
        return Jsonb(unpack_proto_json(value))  # type: ignore
    elif prop.primitive_type == PrimitiveType.DATETIME:
        return cast(Timestamp, value).ToDatetime()
    elif prop.primitive_type == PrimitiveType.DATE:
        return unpack_proto_date(cast(Date, value))
    elif prop.primitive_type == PrimitiveType.TIME:
        return unpack_proto_time(cast(TimeOfDay, value))
    elif prop.primitive_type == PrimitiveType.DURATION:
        return value.ToTimedelta()
    else:
        return value


def _pack_builtin_object_data_prop(prop: Property, value: Any) -> SqlPrimitive:
    """Packs the value of a BuiltinObject property for storage in Postgres."""
    if value is None:
        return None
    elif not prop.is_list:
        return _pack_builtin_object_data_prop_scalar(prop, value)
    else:
        return [_pack_builtin_object_data_prop_scalar(prop, v) for v in value]


def _pack_builtin_object_value_prop_scalar(prop: Property, value: Any) -> SqlPrimitive:
    """Packs the JSON-value-packed value of a BuiltinObject for storage in Postgres."""
    if prop.is_struct:
        return Jsonb(value)
    elif prop.primitive_type == PrimitiveType.UUID:
        return to_uuid(value)
    elif prop.primitive_type == PrimitiveType.JSON:
        return Jsonb(value)
    elif prop.primitive_type == PrimitiveType.DATETIME:
        return datetime.datetime.fromisoformat(value)
    elif prop.primitive_type == PrimitiveType.DATE:
        return datetime.date.fromisoformat(value)
    elif prop.primitive_type == PrimitiveType.TIME:
        return datetime.time.fromisoformat(value)
    elif prop.primitive_type == PrimitiveType.DURATION:
        return timedelta_from_isoformat(value)
    else:
        return value


def _pack_builtin_object_value_prop(prop: Property, value: Any) -> SqlPrimitive:
    """Packs the JSON-value-packed value of a BuiltinObject for storage in Postgres."""
    if value is None:
        return None
    elif not prop.is_list:
        return _pack_builtin_object_value_prop_scalar(prop, value)
    else:
        return [_pack_builtin_object_value_prop_scalar(prop, v) for v in value]


def _unpack_builtin_object_data_prop_scalar(
    prop: Property, value_packed: Any, into: Any | None = None
) -> Any:
    """Unpacks the value of a BuiltinObject property from Postgres."""
    if value_packed is None:
        return None
    elif prop.reference_struct:
        return unpack_builtin_object_data(value_packed, into=into)
    elif prop.primitive_type == PrimitiveType.UUID:
        return str(value_packed)
    elif prop.primitive_type == PrimitiveType.JSON:
        return pack_proto_json(value_packed)
    elif prop.primitive_type == PrimitiveType.DATETIME:
        ts = Timestamp()
        ts.FromDatetime(value_packed)
        return ts
    elif prop.primitive_type == PrimitiveType.DATE:
        return pack_proto_date(value_packed)
    elif prop.primitive_type == PrimitiveType.TIME:
        return pack_proto_time(value_packed)
    elif prop.primitive_type == PrimitiveType.DURATION:
        dur = Duration()
        dur.FromTimedelta(value_packed)
        return dur
    else:
        return value_packed


def _pack_field_value(field: Field, value: JsonValue) -> SqlPrimitive:
    """Packs the JSON-value-packed value of a Field for storage in Postgres."""
    if value is None:
        return None
    elif field.primitive_type == PrimitiveType.JSON or field.kind in (
        TypeKind.NODE,
        TypeKind.BASED_NODE,
        TypeKind.STRUCT,
        TypeKind.CUSTOM_OBJECT,
        TypeKind.PARTIAL_OBJECT,
    ):
        if field.is_list:
            return [Jsonb(v) for v in value]  # type: ignore
        else:
            return Jsonb(value)
    elif field.kind == TypeKind.PRIMITIVE or field.kind == TypeKind.ENUM:
        return cast(PrimitiveValue, unpack_value(value, field))
    else:
        raise RuntimeError(f"unexpected field kind: {field!r}")


def _unpack_field_value(field: Field, value_packed: Any) -> JsonValue:
    """Unpacks the JSON-value-packed value of a Field from Postgres."""
    if value_packed is None:
        return None
    elif field.primitive_type == PrimitiveType.JSON or field.kind in (
        TypeKind.NODE,
        TypeKind.BASED_NODE,
        TypeKind.STRUCT,
        TypeKind.CUSTOM_OBJECT,
        TypeKind.PARTIAL_OBJECT,
    ):
        return value_packed
    elif field.kind == TypeKind.PRIMITIVE or field.kind == TypeKind.ENUM:
        return pack_value(value_packed, field)
    else:
        raise RuntimeError(f"unexpected field kind: {field!r}")


def _pg_pack_node_reference_into_row(
    prop: Property | Any,
    row: dict[str, Any],
    value: NodeReferenceData | Collection[NodeReferenceData] | None,
) -> None:
    """
    'Unravels' a wired pointer into (one or more) stored columns as needed :StoredPointers
    NOTE :Cleanup: pg_pack_node_reference/pg_unpack_node_reference are way too much code
    """
    # unravel reference
    assert prop.reference_stored_ids is not None, f"no stored ids for {prop!r}"
    assert prop.reference_stored_ids_by_type is not None, f"no stored ids for {prop!r}"
    assert prop.reference_stored_metas is not None, f"no stored extras for {prop!r}"
    if prop.is_list:  # list reference
        # map to references list
        references = () if value is None else cast(list[NodeReferenceData], value)
        # pointer id/ck
        for stored_prop in prop.reference_stored_ids:
            row[stored_prop.name] = []
        for ref in references:
            stored_prop = prop.reference_stored_ids_by_type[cast(NodeType, ref.node_type)]
            row[stored_prop.name].append(ref.id)
        # additional pointer metadata
        for meta_type, meta_prop in prop.reference_stored_metas.items():
            meta_key = PROPERTY_META_KEY_BY_TYPE[meta_type]
            row[meta_prop.name] = [getattr(v, meta_key) or None for v in references]
    else:  # single reference
        # map to single reference
        if value is None:
            reference = None
        elif type(value) is NodeReferenceData:
            reference = value
        else:
            raise ValueError(f"expected single reference, got {value!r}")
        # pointer id/ck
        for stored_prop in prop.reference_stored_ids:
            assert stored_prop.reference_nodes is not None, f"no reference nodes: {stored_prop!r}"
            if reference is not None:
                row[stored_prop.name] = reference.id
            else:
                row[stored_prop.name] = None
        # additional pointer metadata
        for meta_type, meta_prop in prop.reference_stored_metas.items():
            if value is None:
                row[meta_prop.name] = None
            else:
                meta_key = PROPERTY_META_KEY_BY_TYPE[meta_type]
                row[meta_prop.name] = getattr(value, meta_key) or None


def _pg_unpack_node_reference_from_row(prop: Property, row: RowOut, node: AnyNodeData) -> None:
    """
    'Ravels' a wired pointer from (one or more) stored columns :StoredPointers
    """

    # get bench id
    bench_id = row.get("id") if node.metatype == NodeType.BENCH else row.get("bench_id")
    if bench_id is not None:
        bench_id = str(bench_id)

    assert prop.reference_stored_ids is not None, f"no stored ids for {prop!r}"
    assert prop.reference_stored_metas is not None, f"no stored extras for {prop!r}"
    assert prop.reference_wired_ptr is not None, f"no wired ptr for {prop!r}"
    if prop.is_list:  # list reference
        # can only be a a set of id props + a single ck prop
        ptrs = getattr(node, prop.reference_wired_ptr.name)
        # pointer id/cks
        for stored_prop in prop.reference_stored_ids:
            ids = cast(list[UUID] | None, row.get(stored_prop.name))
            for id in ids or ():
                ptr: NodeReferenceData = ptrs.add()
                ptr.metatype = pb2.ObjectType.OBJECT_TYPE_NODE_REFERENCE
                ptr.id = str(id)
                ptr.node_type = cast(list[pb2.NodeType], stored_prop.reference_nodes)[0]
        # additional pointer metadata
        for i, ptr in enumerate(ptrs):
            for meta_type, meta_prop in prop.reference_stored_metas.items():
                extra_value = cast(list, row.get(meta_prop.name))[i]
                if extra_value is None:
                    continue
                elif meta_prop.primitive_type == PrimitiveType.UUID:
                    extra_value = str(extra_value)
                elif meta_prop.enum_type == EnumType.NODE_TYPE:
                    extra_value = NodeType(extra_value)
                else:
                    raise RuntimeError(f"unexpected meta prop type: {meta_prop!r}")
                meta_key = PROPERTY_META_KEY_BY_TYPE[meta_type]
                setattr(ptr, meta_key, extra_value)
            if not ptr.ck:
                ptr.ck = ptr.id
            if bench_id and not ptr.bench_id:
                ptr.bench_id = bench_id
                if ptr.base_ck:
                    ptr.base_bench_id = ptr.bench_id
    else:  # single reference
        # pointer id/ck
        for stored_prop in prop.reference_stored_ids:
            value = cast(UUID | None, row.get(stored_prop.name))
            if value is not None:
                ptr = NodeReferenceData(
                    metatype=pb2.ObjectType.OBJECT_TYPE_NODE_REFERENCE,
                    id=str(value),
                    # if this is a heterogeneous ck pointer, type will be overwritten from extras
                    node_type=cast(list[pb2.NodeType], stored_prop.reference_nodes)[0],
                )
                break
        else:
            return

        # additional pointer metadata
        for meta_type, meta_prop in prop.reference_stored_metas.items():
            extra_value = row.get(meta_prop.name)
            if extra_value is None:
                continue
            elif meta_prop.primitive_type == PrimitiveType.UUID:
                extra_value = str(extra_value)
            elif meta_prop.enum_type == EnumType.NODE_TYPE:
                extra_value = NodeType(cast(int, extra_value))
            else:
                raise RuntimeError(f"unexpected meta prop type: {meta_prop!r}")
            meta_key = PROPERTY_META_KEY_BY_TYPE[meta_type]
            setattr(ptr, meta_key, extra_value)
        if not ptr.ck:
            ptr.ck = ptr.id
        if bench_id and not ptr.bench_id:
            ptr.bench_id = bench_id
            if ptr.base_ck:
                ptr.base_bench_id = ptr.bench_id
        getattr(node, prop.reference_wired_ptr.name).CopyFrom(ptr)


def _pg_pack_node_data_row(
    node_type: NodeType,
    node_cls: type[Node],
    node_table: Table,
    block: Block | None,
    node: AnyNodeData,
) -> dict[str, SqlPrimitive]:
    """Packs a node's data into a row for the respective table."""
    try:
        row: dict[str, SqlPrimitive] = {}

        # wired properties
        for name, prop in node_cls.__wired_properties__.items():
            if prop.is_value_packed and node_cls.__is_stored_value_unraveled__:
                continue  # value is stored in unraveled columns
            elif prop.reference_source is None:
                # regular non-ref property
                if prop.is_optional_scalar and not node.HasField(name):
                    value = None
                else:
                    value = getattr(node, name)
                row[name] = _pack_builtin_object_data_prop(prop, value)
            else:
                # unravel stored node reference :StoredPointers
                wired_name = cast(Property, prop.reference_source.reference_wired_ptr).name
                if prop.is_optional_scalar and not node.HasField(wired_name):
                    value = None
                else:
                    value = getattr(node, wired_name)
                _pg_pack_node_reference_into_row(prop.reference_source, row, value)

        # unravel value-packed fields
        if node_cls.__is_stored_value_unraveled__ and block is not None:
            value_runtime_prop = first(node_cls.__value_runtime_properties__.values())
            value_packed_prop = value_runtime_prop.value_packed_ptr
            assert type(value_packed_prop) is Property, f"unexpected packed: {value_packed_prop!r}"
            value_packed_any: ProtoValue | None = getattr(node, value_packed_prop.name)
            value_packed = (
                unpack_proto_json_struct(value_packed_any.struct_value)
                if value_packed_any is not None
                else None
            )
            for field in block.fields:
                if field.type != FieldType.MEMBER:
                    continue
                column = node_table.get_column(field)
                if value_packed is not None:
                    field_value = value_packed.get(field.storage_key)
                else:
                    field_value = None
                row[column.name] = _pack_field_value(field, field_value)

        return row
    except (AttributeError, TypeError, ValueError, KeyError) as e:
        raise ValueError(f"could not pack row {node_cls.metatype.name}: {struct!r}") from e


def _pg_unpack_node_data_row(
    node_type: NodeType,
    node_cls: type[Node],
    node_table: Table,
    block: Block | None,
    selected_fields: Sequence[Field],
    row: Mapping[str, Any],
) -> AnyNodeData:
    """Unpacks a node's data from a row from the respective table."""
    try:
        proto_cls = PROTO_CLASS_BY_TYPE[node_cls.metatype]
        obj_data = cast(
            AnyNodeData, proto_cls(metatype=wiring.pack_enum(NodeType, node_cls.metatype))
        )  # type: ignore

        # wired properties
        for name, prop in node_cls.__wired_properties__.items():
            if prop.is_value_packed and node_cls.__is_stored_value_unraveled__:
                continue  # value is stored in unraveled columns
            if prop.reference_source is not None:
                # ravel stored node reference :StoredPointers
                _pg_unpack_node_reference_from_row(prop.reference_source, row, obj_data)
                continue
            # regular non-ref property
            value = row.get(name)
            if value is None:
                continue
            if not prop.is_list:  # scalar
                packed_value = _unpack_builtin_object_data_prop_scalar(prop, value)
                if isinstance(packed_value, ProtoMessage):
                    getattr(obj_data, name).CopyFrom(packed_value)
                elif prop.is_struct:
                    assert (
                        value is None
                    ), f"unexpected non-proto struct value for {prop!r}: {value!r}"
                    obj_data.ClearField(name)
                else:
                    setattr(obj_data, name, packed_value)
            elif len(value) > 0:  # list
                packed_value = getattr(obj_data, name)
                if prop.is_struct:
                    for item in value:
                        packed_item = packed_value.add()
                        _ = _unpack_builtin_object_data_prop_scalar(prop, item, into=packed_item)
                else:
                    for item in value:
                        packed_item = _unpack_builtin_object_data_prop_scalar(prop, item)
                        packed_value.append(packed_item)

        # ravel value-packed fields
        if node_cls.__is_stored_value_unraveled__ and block is not None:
            value_runtime_prop = first(node_cls.__value_runtime_properties__.values())
            value_packed_prop = value_runtime_prop.value_packed_ptr
            assert type(value_packed_prop) is Property, f"unexpected packed: {value_packed_prop!r}"
            value_packed_any: ProtoValue = getattr(obj_data, value_packed_prop.name)
            value_packed = value_packed_any.struct_value
            for field in selected_fields:
                column = node_table.get_column(field)
                field_value = row.get(column.name)
                field_value_packed = _unpack_field_value(field, field_value)
                value_packed.__setitem__(field.storage_key, field_value_packed)

        return obj_data
    except (AttributeError, TypeError, ValueError, KeyError) as e:
        row_str = repr(row) if IS_DEV else describe_type(row)
        raise ValueError(f"could not unpack row {node_cls.metatype.name}: {row_str}") from e


#
# Graph API
#


def _combine_filter(*, include_deleted: bool, filter: Expression | None) -> Expression | None:
    if include_deleted:
        return filter
    else:
        if filter is None:
            return get_default_query_filter()
        else:
            return filter & get_default_query_filter()


@_trace_pg_span
async def pg_graph_select(
    *, cur: psycopg.AsyncCursor, ctx: SqlContext, query: Query
) -> list[AnyNodeData]:
    """
    Selects the nodes from the graph matching the given query.
    Only the given node type is selected, no joins are performed (up/down or sideways).
    """
    # compile
    select = query._select or SelectOptions.default()
    node_type = query._node_type
    node_cls = NODE_CLASS_BY_TYPE[node_type]
    block = query._base_block
    selected_properties = select.get_selected_properties(node_type)
    if node_type in BUILTIN_TABLE_BY_NODE_TYPE:
        node_table = BUILTIN_TABLE_BY_NODE_TYPE[node_type]
        columns = [node_table.get_column(prop.name) for prop in selected_properties]
        selected_fields: Sequence[Field] = []
    else:
        node_table, block = ctx.get_custom_table(query.block)
        columns = [
            node_table.get_column(prop.name)
            for prop in selected_properties
            if prop.name in node_table._columns_by_name
        ]
        selected_fields = select.get_selected_fields(block)
        for field in selected_fields:
            if field.type != FieldType.MEMBER:
                continue
            column = node_table.get_column(field)
            columns.append(column)
    filter = _combine_filter(include_deleted=query._include_deleted, filter=query._filter)
    where = (
        _pg_compile_conditional(node_type, node_cls, node_table, block, filter)
        if filter is not None
        else None
    )
    order_by = (
        _pg_compile_sorts(node_type, node_cls, node_table, block, query._sort)
        if query._sort
        else None
    )

    # select
    assert any(c.is_primary_key for c in columns), f"no primary key selected in {columns!r}"
    rows = await pg_select(
        cur=cur,
        ctx=ctx,
        table=node_table,
        columns=columns,
        where=where,
        order_by=order_by,
        first=query._first,
        skip=query._skip,
    )
    nodes_data = [
        _pg_unpack_node_data_row(
            node_type=node_type,
            node_cls=node_cls,
            node_table=node_table,
            block=block,
            selected_fields=selected_fields,
            row=row,
        )
        for row in rows
    ]
    return nodes_data


@_trace_pg_span
async def pg_graph_count(*, cur: psycopg.AsyncCursor, ctx: SqlContext, query: Query) -> int:
    """Counts the nodes from the graph matching the given query. Ignores pagination parameters."""
    # compile
    node_type = query._node_type
    node_cls = NODE_CLASS_BY_TYPE[node_type]
    block = query._base_block
    if node_type in BUILTIN_TABLE_BY_NODE_TYPE:
        node_table = BUILTIN_TABLE_BY_NODE_TYPE[node_type]
    else:
        node_table, _ = ctx.get_custom_table(query.block)
    filter = _combine_filter(include_deleted=query._include_deleted, filter=query._filter)
    where = (
        _pg_compile_conditional(node_type, node_cls, node_table, block, filter)
        if filter is not None
        else None
    )

    # count
    count = await pg_count(cur=cur, ctx=ctx, table=node_table, where=where)
    return count


@_trace_pg_span
async def pg_graph_exists(*, cur: psycopg.AsyncCursor, ctx: SqlContext, query: Query) -> bool:
    """Checks if nodes from the graph matching the given query exist."""
    # compile
    node_type = query._node_type
    node_cls = NODE_CLASS_BY_TYPE[node_type]
    block = query._base_block
    if node_type in BUILTIN_TABLE_BY_NODE_TYPE:
        node_table = BUILTIN_TABLE_BY_NODE_TYPE[node_type]
    else:
        node_table, _ = ctx.get_custom_table(query.block)
    filter = _combine_filter(include_deleted=query._include_deleted, filter=query._filter)
    where = (
        _pg_compile_conditional(node_type, node_cls, node_table, block, filter)
        if filter is not None
        else None
    )

    # exists
    exists = await pg_exists(cur=cur, ctx=ctx, table=node_table, where=where)
    return exists


@_trace_pg_span
async def _pg_graph_walk_down(
    *,
    cur: psycopg.AsyncCursor,
    ctx: SqlContext,
    roots: list[NodeReferenceData]
    | list[AnyNodeData]
    | tuple[NodeReferenceData, ...]
    | tuple[AnyNodeData, ...],
    descendant_types: Collection[NodeType],
    extra_filter: Expression | None,
) -> tuple[list[NodeReferenceData], dict[str, list[NodeReferenceData]]]:
    """
    Gets node pointers to all descendants down from the roots matching the filter.
    NOTE :Incomplete: for now graph walk down only works with builtin tables, not custom tables
    TODO :Performance: walk graph down in SQL only (no roundtrip recursion)
     (the result of this walk is usually cached, but not for cascading edits)
    """
    if not roots:
        return [], {}
    if not isinstance(roots[0], NodeReferenceData):
        roots_ptrs = [
            NodeReference._ref_data_from_node_data(cast(AnyNodeData, node)) for node in roots
        ]
    else:
        roots_ptrs = cast(list[NodeReferenceData], roots)

    # filter to descendant types that have a table with parents
    descendant_types = [
        t
        for t in descendant_types
        if NODE_CLASS_BY_TYPE[t].__parent_property__.reference_stored_ids
        and t in BUILTIN_TABLE_BY_NODE_TYPE
    ]

    # descend
    all_descendants: list[NodeReferenceData] = []
    root_id_by_node_id = {cast(str, node.id): cast(str, node.id) for node in roots_ptrs}
    all_descendants_by_root_id: dict[str, list[NodeReferenceData]] = defaultdict(list)
    current_parents = roots_ptrs
    while current_parents:
        next_parents: list[NodeReferenceData] = []
        # traverse all direct children of plausible types
        for child_type in descendant_types:
            child_cls = NODE_CLASS_BY_TYPE[child_type]
            assert child_cls.__parent_property__.reference_stored_ids

            # collect possible parents
            parent_ids: list[str] = []
            for parent in current_parents:
                if NodeType(parent.node_type) in PARENT_NODE_TYPES[child_type]:
                    parent_ids.append(cast(str, parent.id))
            if not parent_ids:
                continue  # nothing to do
            parent_filter = C(
                op=ConditionalType.IN,
                property=child_cls.__parent_property__.reference_stored_ids[0],
                value=parent_ids,
            )
            if extra_filter is not None:
                parent_filter = parent_filter & extra_filter

            # collect children
            child_table = BUILTIN_TABLE_BY_NODE_TYPE[child_type]
            assert child_table, f"no table for {child_cls!r}"
            assert child_table._primary_key, f"no primary key for {child_cls!r}"
            parent_where = _pg_compile_conditional(
                child_type, child_cls, child_table, None, parent_filter
            )
            children_rows = await pg_select(
                cur=cur,
                ctx=ctx,
                table=child_table,
                columns=(
                    child_table._columns_by_name["id"],
                    child_table._columns_by_name["parent_id"],
                ),
                where=parent_where,
            )
            for child_row in children_rows:
                child_ptr = NodeReferenceData(
                    metatype=pb2.ObjectType.OBJECT_TYPE_NODE_REFERENCE,
                    id=str(child_row["id"]),
                    node_type=cast(pb2.NodeType, child_type),
                )
                next_parents.append(child_ptr)
                all_descendants.append(child_ptr)
                # remember root
                root_id = root_id_by_node_id.get(str(child_row["parent_id"]))
                assert root_id is not None, f"no root id for {child_row!r}"
                root_id_by_node_id[cast(str, child_ptr.id)] = root_id
                all_descendants_by_root_id[root_id].append(child_ptr)

        current_parents = next_parents

    return all_descendants, all_descendants_by_root_id


@_trace_pg_span
async def pg_graph_get(
    *,
    cur: psycopg.AsyncCursor,
    ctx: SqlContext,
    query: Query,
    visited_graph: NodeDataGraph,
) -> None:
    """
    Gets the 'root' nodes from a Query (Query.roots) and recursively reads up/down the graph.
    Also performs any additional joins needed for the query.
    """
    # NOTE :Performance: we could read all package contents with package_id=x if we know it's a package query.
    assert query._roots, "no roots to select"
    roots = [r._to_data() for r in query._roots]

    # get roots
    root_nodes: list[AnyNodeData]
    if isinstance(roots[0], NodeReferenceData):
        # select roots
        roots_ids = [node.id for node in roots]
        root_filter = C(ConditionalType.IN, property=Node.id, value=roots_ids)
        root_query = Query(
            QueryType.GET,
            node_type=query._node_type,
            base_block=query._base_block,
            filter=root_filter,
            select=query._select,
            include_deleted=query.include_deleted,
        )
        root_nodes = await pg_graph_select(cur=cur, ctx=ctx, query=root_query)
    else:  # already got nodes
        root_nodes = cast(list[AnyNodeData], list(roots))
    for node in root_nodes:
        visited_graph.add(node)

    # select ancestors (recursively)
    # (since this is usually a straight, short walk we just select up step by step)
    if query._ancestor_types:
        current_parents = root_nodes
        to_select_by_type: dict[NodeType, list[str]] = defaultdict(list)
        while current_parents:
            to_select_by_type.clear()

            # traverse unseen parents to select next
            for node in current_parents:
                if (
                    node.parent_ptr is not None
                    and node.parent_ptr.id is not None
                    and node.parent_ptr.node_type in query._ancestor_types
                    and node.parent_ptr.id not in visited_graph
                ):
                    parent_type = NodeType(node.parent_ptr.node_type)
                    to_select_by_type[parent_type].append(node.parent_ptr.id)

            # select next parents
            next_parents = []
            for node_type, node_ids in to_select_by_type.items():
                parents_filter = C(ConditionalType.IN, property=Node.id, value=node_ids)
                parents_query = Query(
                    QueryType.SEARCH,
                    node_type,
                    filter=parents_filter,
                    select=query._select,
                    include_deleted=query.include_deleted,
                )
                new_parents = await pg_graph_select(cur=cur, ctx=ctx, query=parents_query)
                next_parents.extend(new_parents)
                for node in new_parents:
                    visited_graph.add(node)
            current_parents = next_parents

    # select descendants (recursively)
    # (since this may be a long wide search down, we first collect the pointers, then select by type)
    if query._descendant_types:
        descendant_node_ptrs, _ = await _pg_graph_walk_down(
            cur=cur,
            ctx=ctx,
            roots=root_nodes,
            descendant_types=query._descendant_types,
            extra_filter=get_default_query_filter() if not query.include_deleted else None,
        )
        descendant_node_ptrs_by_type = group_by(descendant_node_ptrs, lambda ptr: ptr.node_type)
        for wire_node_type, node_ptrs in descendant_node_ptrs_by_type.items():
            node_type = NodeType(wire_node_type)
            children_filter = C(
                ConditionalType.IN, property=Node.id, value=[ptr.id for ptr in node_ptrs]
            )
            children_query = Query(
                QueryType.SEARCH,
                node_type,
                filter=children_filter,
                select=query._select,
                include_deleted=query.include_deleted,
            )
            new_children = await pg_graph_select(cur=cur, ctx=ctx, query=children_query)
            for node in new_children:
                visited_graph.add(node)


@_trace_pg_span
async def pg_graph_search(
    cur: psycopg.AsyncCursor,
    ctx: SqlContext,
    scope: GraphScopeData,
    query: Query,
    count: bool,
) -> tuple[list[AnyNodeData], NodeDataGraph, int | None]:
    """
    Search for roots matching the filter and then get the graph up/down/joined from there.
    """
    node_types = tuple(query.all_node_types)
    if count:
        total = await pg_graph_count(cur=cur, ctx=ctx, query=query)
    else:
        total = None
    if query._ancestor_types or query._descendant_types:
        # split into two passes if we have other nodes to fetch
        roots = await pg_graph_select(cur=cur, ctx=ctx, query=query)
        visited_graph = NodeDataGraph(scope, node_types)
        if not roots:
            return roots, visited_graph, total
        roots_ptrs = [
            wiring.unpack_builtin_object(
                NodeReference._ref_data_from_node_data(r), expect=NodeReference, supergraph=None
            )
            for r in roots
        ]
        get_query = Query(
            QueryType.GET,
            query._node_type,
            base_block=query._base_block,
            roots=roots_ptrs,
            ancestor_types=query._ancestor_types,
            descendant_types=query._descendant_types,
            select=query._select,
            include_deleted=query.include_deleted,
        )
        _ = await pg_graph_get(cur=cur, ctx=ctx, query=get_query, visited_graph=visited_graph)
        return roots, visited_graph, total
    else:
        # otherwise just select in one go
        roots = await pg_graph_select(cur=cur, ctx=ctx, query=query)
        graph = NodeDataGraph(scope, node_types, nodes=roots)
        return roots, graph, total


# TODO :Performance!: use psycopg3/postgres pipelining to batch edits


@_trace_pg_span
async def pg_graph_edit(
    *,
    cur: psycopg.AsyncCursor,
    ctx: SqlContext,
    edits: list[EditData] | tuple[EditData, ...],
    cascade: bittuple[EditType] = CASCADING_EDIT_TYPES,
) -> list[EditData]:
    """Apply graph edits, cascading as needed. Returns the cascaded edits."""
    if not edits:
        return []

    def _get_node_table(edit: EditData):
        if edit.node_ptr.node_type in BUILTIN_TABLE_BY_NODE_TYPE:
            return BUILTIN_TABLE_BY_NODE_TYPE[edit.node_ptr.node_type], None
        else:
            assert edit.node_ptr.base_ck, f"no base ck for custom table in {edit!r}"
            table, block = ctx.get_custom_table(UUID(edit.node_ptr.base_ck))
            return table, block

    assert edits[0].node_ptr is not None, f"no node ptr for {edits[0]!r}"
    batch_node_cls = NODE_CLASS_BY_TYPE[wiring.unpack_enum(NodeType, edits[0].node_ptr.node_type)]
    batch_updated_properties: bitarray = bitarray(batch_node_cls.__max_property_ord__ + 1)
    batch: list[EditData] = []
    batch_node_table, batch_block = _get_node_table(edits[0])
    all_cascaded_edits: list[EditData] = []

    # batch operations by table and edit type
    for i, prev_edit in enumerate(edits):
        next_edit = edits[i + 1] if i + 1 < len(edits) else None
        assert prev_edit.node_ptr is not None, f"no node ptr for {prev_edit!r}"
        batch.append(prev_edit)

        # accumulate updated properties
        for op in prev_edit.operations:
            prop_id = int(op.path[0])
            prop_ord = batch_node_cls.__properties_by_id__[prop_id].ord
            batch_updated_properties[prop_ord] = True

        # continue batch if same edit + node types
        next_node_table, next_block = (
            _get_node_table(next_edit) if next_edit is not None else (batch_node_table, batch_block)
        )
        if (
            next_edit is not None
            and next_edit.type == prev_edit.type
            and next_edit.node_ptr is not None
            and next_edit.node_ptr.node_type == prev_edit.node_ptr.node_type
            and next_node_table == batch_node_table
        ):
            continue

        # flush batch (at end or next is different)
        edit_type: EditType = wiring.unpack_enum(EditType, prev_edit.type)
        node_type = wiring.unpack_enum(NodeType, prev_edit.node_ptr.node_type)
        del prev_edit  # for clarity

        # cascade edits down
        # NOTE :Performance: sometimes we don't need to cascade down removes in PG
        #  (for instance in Host we the edited graph may be loaded, so we could do this in memory)
        if edit_type in cascade and node_type in HAS_CHILD_NODE_TYPES:
            cascaded_edits = await _pg_edit_cascade(
                cur=cur, ctx=ctx, edit_type=edit_type, node_type=node_type, batch=batch
            )
            all_cascaded_edits.extend(cascaded_edits)

        # write edits to pg
        updated_properties = batch_node_cls._unmask_properties(batch_updated_properties)
        _ = await _pg_edit_batch(
            cur=cur,
            ctx=ctx,
            edit_type=edit_type,
            node_type=cast(NodeType, node_type),
            node_table=batch_node_table,
            block=batch_block,
            batch=batch,
            updated_properties=updated_properties,
        )

        # start new batch if needed
        if next_edit is not None:
            batch_node_cls = NODE_CLASS_BY_TYPE[
                wiring.unpack_enum(NodeType, next_edit.node_ptr.node_type)
            ]
            batch_updated_properties = bitarray(batch_node_cls.__max_property_ord__ + 1)
            batch_node_table, batch_block = next_node_table, next_block
            batch.clear()

    return all_cascaded_edits


@_trace_pg_span
async def _pg_edit_cascade(
    *,
    cur: psycopg.AsyncCursor,
    ctx: SqlContext,
    edit_type: EditType,
    node_type: NodeType,
    batch: list[EditData],
) -> list[EditData]:
    """Cascades a batch of edits to the relevant descendants of the node."""

    # figure out which nodes to cascade to
    root_nodes = tuple(root_edit.node_ptr for root_edit in batch)
    root_edit_by_root_node_id = {cast(str, root_edit.node_ptr.id): root_edit for root_edit in batch}
    if edit_type == EditType.RESTORE:
        # only cascade to nodes that were removed at the exact same time
        removed_dts = []
        for root_edit in batch:
            assert root_edit.HasField("old_edited_at"), f"no old edited at for {root_edit!r}"
            removed_at = root_edit.old_edited_at.ToDatetime(tzinfo=pytz.utc)
            removed_dts.append(removed_at)
        extra_filter = C(op=ConditionalType.IN, property=Node.deleted_at, value=removed_dts)
    elif edit_type == EditType.ERASE:
        # cascade to all
        extra_filter = None
    else:
        # only cascade to visible
        extra_filter = get_default_query_filter()

    # select cascaded nodes from graph
    _, cascaded_nodes_by_root_id = await _pg_graph_walk_down(
        cur=cur,
        ctx=ctx,
        roots=root_nodes,
        # only descend to node types in the same store
        descendant_types=DESCENDANT_NODE_TYPES_IN_STORE[node_type],
        extra_filter=extra_filter,
    )

    # turn into cascaded edits with source root for exact context
    all_cascaded_edits: list[EditData] = []
    root_edit_by_cascaded_node_id: dict[str, EditData] = {}
    for root_id, node_ptrs in cascaded_nodes_by_root_id.items():
        root_edit = root_edit_by_root_node_id[root_id]
        for node_ptr in node_ptrs:
            cascaded_edit = EditData(
                id=str(UUIDT()),
                type=cast(pb2.EditType, edit_type),
                node_ptr=node_ptr,
                edited_at=root_edit.edited_at,
                scope=root_edit.scope,  # should always be the same
                category=root_edit.category,
                context=root_edit.context,
            )
            if root_edit.HasField("subject_ptr"):
                cascaded_edit.subject_ptr.CopyFrom(root_edit.subject_ptr)
            all_cascaded_edits.append(cascaded_edit)
            root_edit_by_cascaded_node_id[cast(str, node_ptr.id)] = root_edit

    # batch operations by edit kind and node table
    cascaded_edits_by_type = group_by(all_cascaded_edits, lambda edit: edit.node_ptr.node_type)
    for descendant_node_type, cascaded_edits in cascaded_edits_by_type.items():
        node_table = BUILTIN_TABLE_BY_NODE_TYPE[NodeType(descendant_node_type)]
        _ = await _pg_edit_batch(
            cur=cur,
            ctx=ctx,
            edit_type=edit_type,
            node_type=cast(NodeType, descendant_node_type),
            node_table=node_table,
            block=None,
            batch=cascaded_edits,
            updated_properties=(),
        )
        nodes_ids = [edit.node_ptr.id for edit in cascaded_edits]
        descendant_query = Query(
            QueryType.GET,
            node_type=NodeType(descendant_node_type),
            filter=C(ConditionalType.IN, property=Node.id, value=nodes_ids),
            include_deleted=True,
        )
        nodes = await pg_graph_select(cur=cur, ctx=ctx, query=descendant_query)
        nodes_by_id = {node.id: node for node in nodes}

        # and assign new/old node to edit now that we have the full data :EditData
        for cascaded_edit in cascaded_edits:
            node = nodes_by_id[cascaded_edit.node_ptr.id]
            cascaded_edit.node_data.CopyFrom(wiring.wrap_some_node(node))

    return all_cascaded_edits


@_trace_pg_span
async def _pg_edit_batch(
    *,
    cur: psycopg.AsyncCursor,
    ctx: SqlContext,
    edit_type: EditType,
    node_type: NodeType,
    node_table: Table,
    block: Block | None,
    batch: list[EditData],
    updated_properties: tuple[Property, ...],  # across batch
) -> None:
    """Applies a batch of edits to the graph (without cascading)."""
    trace.get_current_span().set_attributes(
        {"edit_type": edit_type.bench_name, "edits": len(batch)}
    )

    node_cls = NODE_CLASS_BY_TYPE[node_type]
    assert node_table._primary_key is not None, f"no primary key for {node_cls!r}: {node_table!r}"

    if edit_type in (EditType.CREATE, EditType.UPSERT):
        nodes = []
        rows = []
        for edit in batch:
            assert edit.HasField("node_data"), f"no node_data for {edit!r}"
            node = wiring.unwrap_some_node(edit.node_data)
            nodes.append(node)
            # inline implicit metadata
            row: dict[str, SqlPrimitive] = _pg_pack_node_data_row(
                node_type=node_type,
                node_cls=node_cls,
                node_table=node_table,
                block=node_table._block,
                node=node,
            )
            row["created_at"] = row["updated_at"] = edit.edited_at.ToDatetime(tzinfo=pytz.utc)
            subject_ptr = edit.subject_ptr if edit.HasField("subject_ptr") else None
            _pg_pack_node_reference_into_row(node_cls.get_property("created_by"), row, subject_ptr)
            _pg_pack_node_reference_into_row(node_cls.get_property("updated_by"), row, subject_ptr)
            rows.append(row)

        if edit_type == EditType.CREATE:
            _ = await pg_insert(cur=cur, ctx=ctx, table=node_table, rows=rows)
        else:
            rows = await pg_upsert(
                cur=cur,
                ctx=ctx,
                table=node_table,
                rows=rows,
                conflict_columns=(node_table._primary_key,),
                static_columns=tuple(c for c in node_table.columns if c != node_table._primary_key),
            )
    elif edit_type in (EditType.UPDATE, EditType.MOVE, EditType.DELETE, EditType.RESTORE):
        # collect dynamic columns (incl. implicit metadata)
        implicit_properties: list[Property | Any] = [
            node_cls.updated_at,
            node_cls.get_property("updated_by"),
        ]
        if edit_type in (EditType.DELETE, EditType.RESTORE):
            implicit_properties.append(node_cls.deleted_at)
        dynamic_columns: list[Column] = [node_table._primary_key]
        dynamic_fields: list[Field] = (
            [f for f in block.fields if f.type == FieldType.MEMBER] if block is not None else []
        )
        for prop in chain(implicit_properties, updated_properties):
            if prop.is_value_packed and node_cls.__is_stored_value_unraveled__:  # unravel value
                assert block is not None, f"no block for {prop!r}"
                for field in dynamic_fields:
                    dynamic_columns.append(node_table.get_column(field))
            elif prop.is_node_reference:
                dynamic_columns.extend(
                    node_table._columns_by_name[p.name] for p in prop.reference_stored_props or ()
                )
            else:
                dynamic_columns.append(node_table._columns_by_name[prop.name])

        # collect dynamic values
        dynamic_values: list[RowIn] = []
        for edit in batch:
            assert edit.node_ptr is not None, f"no node ptr for {edit!r}"
            assert edit.HasField("edited_at"), f"no edited_at for {edit!r}"
            row: dict[str, SqlPrimitive] = {"id": edit.node_ptr.id}

            # apply update operations
            for op in edit.operations:
                # we only support top-level set/clear operations
                assert (
                    op.type == EditOperationType.SET or op.type == EditOperationType.CLEAR
                ), f"cannot perform non-set operation: ${op!r} in {edit!r}"
                assert len(op.path) == 1, f"cannot update hierarchically: {op!r} in {edit!r}"
                prop_id = int(op.path[0])
                prop = node_cls.__properties_by_id__.get(prop_id)
                assert prop is not None, f"no property {prop_id!r} in {node_cls!r} for {edit!r}"

                # apply set
                if op.type == EditOperationType.CLEAR or not op.HasField("new_value_packed"):
                    new_value_packed = [] if prop.is_list else None
                else:
                    new_value_packed = unpack_proto_json(op.new_value_packed)
                if prop.is_value_packed and node_cls.__is_stored_value_unraveled__:
                    assert (
                        type(new_value_packed) is dict
                    ), f"unexpected packed: {new_value_packed!r} in {op!r}"
                    # unravel value
                    for field in dynamic_fields:
                        field_value = new_value_packed.get(field.key)
                        column = node_table.get_column(field)
                        row[column.name] = _pack_field_value(field, field_value)
                elif not prop.is_node_reference:
                    # regular non-ref property
                    value = _pack_builtin_object_value_prop(prop, new_value_packed)
                    row[prop.name] = value
                else:
                    # unravel stored node reference :StoredPointers
                    if new_value_packed is None:
                        _pg_pack_node_reference_into_row(prop, row, None)
                    else:
                        new_value = unpack_value_data(new_value_packed, prop.type_info)
                        _pg_pack_node_reference_into_row(prop, row, new_value)  # type: ignore

            # implicit properties
            edited_at = edit.edited_at.ToDatetime()
            row["updated_at"] = edited_at
            subject_ptr = edit.subject_ptr if edit.HasField("subject_ptr") else None
            _pg_pack_node_reference_into_row(node_cls.get_property("updated_by"), row, subject_ptr)
            if edit_type == EditType.DELETE:
                row["deleted_at"] = edited_at
            elif edit_type == EditType.RESTORE:
                row["deleted_at"] = None
            dynamic_values.append(row)

        # actually update
        _ = await pg_update_variable(
            cur=cur,
            ctx=ctx,
            table=node_table,
            dynamic_columns=dynamic_columns,
            dynamic_values=dynamic_values,
        )
    elif edit_type == EditType.ERASE:
        nodes_ids = [edit.node_ptr.id for edit in batch]
        where = SqlComparison(
            left=sqlident("id"),
            op=PostgresConditionalOp.EQ,
            right=sqlstr("ANY({})").format(sql.Literal(nodes_ids)),
        )
        _ = await pg_delete(cur=cur, ctx=ctx, table=node_table, where=where)
    else:
        assert_never(edit_type)


#
# Builtin table registry
#

BUILTIN_TABLE_BY_NODE_TYPE: dict[NodeType, Table] = {
    # read previously generated tables in schema.py
    node_type: getattr(schema, f"{to_casing(node_type.name, Casing.ALL_CAPS)}_TABLE")
    for node_type in NODE_TYPES
    if hasattr(schema, f"{to_casing(node_type.name, Casing.ALL_CAPS)}_TABLE")
}
BUILTIN_NODE_BY_TABLE_NAME: dict[str, NodeType] = {
    table.name: node_type for node_type, table in BUILTIN_TABLE_BY_NODE_TYPE.items()
}
BUILTIN_NODE_TABLES: tuple[Table, ...] = tuple(BUILTIN_TABLE_BY_NODE_TYPE.values())

BUILTIN_GLOBAL_TABLES: tuple[Table, ...] = DEFAULT_GLOBAL_TABLES + tuple(
    BUILTIN_TABLE_BY_NODE_TYPE[node.metatype]
    for node in NODE_CLASSES
    if node.__area__ == NodeArea.GLOBAL and node.metatype in BUILTIN_TABLE_BY_NODE_TYPE
)
BUILTIN_REGIONAL_TABLES: tuple[Table, ...] = DEFAULT_REGIONAL_TABLES + tuple(
    BUILTIN_TABLE_BY_NODE_TYPE[node.metatype]
    for node in NODE_CLASSES
    if node.__area__ == NodeArea.REGIONAL and node.metatype in BUILTIN_TABLE_BY_NODE_TYPE
)
BUILTIN_LOCAL_TABLES: tuple[Table, ...] = DEFAULT_LOCAL_TABLES + tuple(
    BUILTIN_TABLE_BY_NODE_TYPE[node.metatype]
    for node in NODE_CLASSES
    if node.__area__ == NodeArea.LOCAL and node.metatype in BUILTIN_TABLE_BY_NODE_TYPE
)
BUILTIN_TABLES_BY_AREA: dict[NodeArea, tuple[Table, ...]] = {
    NodeArea.GLOBAL: BUILTIN_GLOBAL_TABLES,
    NodeArea.REGIONAL: BUILTIN_REGIONAL_TABLES,
    NodeArea.LOCAL: BUILTIN_LOCAL_TABLES,
}

BUILTIN_GLOBAL_SCHEMA = Schema(GLOBAL_EXTENSIONS, BUILTIN_GLOBAL_TABLES)
BUILTIN_REGIONAL_SCHEMA = Schema(REGIONAL_EXTENSIONS, BUILTIN_REGIONAL_TABLES)
BUILTIN_LOCAL_SCHEMA = Schema(LOCAL_EXTENSIONS, BUILTIN_LOCAL_TABLES)
BUILTIN_SCHEMA_BY_AREA: dict[NodeArea, Schema] = {
    NodeArea.GLOBAL: BUILTIN_GLOBAL_SCHEMA,
    NodeArea.REGIONAL: BUILTIN_REGIONAL_SCHEMA,
    NodeArea.LOCAL: BUILTIN_LOCAL_SCHEMA,
}
