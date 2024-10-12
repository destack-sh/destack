import datetime
import struct
from collections import defaultdict
from dataclasses import dataclass
from itertools import chain
from typing import (
    Any,
    Collection,
    Mapping,
    Union,
    assert_never,
    cast,
)
from uuid import UUID

import psycopg
import pytz
from bitarray import bitarray
from google.protobuf.duration_pb2 import Duration
from google.protobuf.message import Message as ProtoMessage
from google.protobuf.timestamp_pb2 import Timestamp
from opentelemetry import trace
from psycopg import sql
from psycopg.types.json import Jsonb

from bench.language import Block, ConditionalOp, NodeReference, Property
from bench.language.bench import Bench
from bench.language.connection import ChannelIncapableError
from bench.language.const import (
    CASCADING_EDIT_TYPES,
    NODE_TYPES,
    SUB_PACKAGE_NODE_TYPES,
    EditOperationType,
    EditType,
    EnumType,
    LiteralOp,
    NodeType,
    PrimitiveType,
    QueryType,
    ReferenceKind,
    SortOp,
)
from bench.language.expression import C, Expression, ExpressionOps
from bench.language.graph import NodeDataGraph
from bench.language.node import NODE_CLASS_BY_TYPE, UNSET, BenchNode, Node
from bench.language.query import (
    DEFAULT_READ_OPTIONS,
    FILTER_NOT_DELETED,
    QueryBuilder,
)
from bench.language.setup import (
    DESCENDANT_NODE_TYPES_IN_STORE,
    HAS_CHILD_NODE_TYPES,
    NODE_CLASSES,
    PARENT_NODE_TYPES,
)
from bench.language.value import (
    pack_builtin_object_data,
    pack_proto_date,
    pack_proto_json,
    pack_proto_time,
    unpack_builtin_object_data,
    unpack_proto_date,
    unpack_proto_json,
    unpack_proto_time,
    unpack_value_data,
)
from bench.proto import wire, wiring
from bench.proto.wire import (
    AnyNodeData,
    Date,
    EditData,
    FileReferenceData,
    GraphScopeData,
    NodeReferenceData,
    SecretReferenceData,
    TimeOfDay,
)
from bench.proto.wiring import PROTO_CLASS_BY_TYPE
from bench.sql import schema
from bench.sql.client import GLOBAL_PG_CRYPTO_KEY
from bench.sql.core import (
    ALL_EXTENSIONS,
    DEFAULT_GLOBAL_TABLES,
    DEFAULT_LOCAL_TABLES,
    GLOBAL_EXTENSIONS,
    LOCAL_EXTENSIONS,
    PG_CONDITIONAL_OP_BY_BENCH,
    POSTGRES_SORT_OP_BY_BENCH,
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
from bench.sql.engine import (
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
    pg_insert,
    pg_select,
    pg_update_variable,
    pg_upsert,
    sql_node_to_sql,
    sqlident,
    sqljoin,
    sqlstr,
)
from bench.utils.env import IS_DEV
from bench.utils.func import bittuple, describe_type, group_by, to_uuid
from bench.utils.string import Casing, to_casing
from bench.utils.time import timedelta_from_isoformat
from bench.utils.uuidt import UUIDT


@dataclass(slots=True)
class BenchContext(SqlContext):
    """A context for a SQL statement."""

    bench: Bench | None

    def get_crypto_key(self, obj: "Table | Column") -> str | None:
        """Gets the crypto key for the given table."""

        table = obj if isinstance(obj, Table) else obj.table
        node_type = NODE_TYPE_BY_TABLE_NAME[table.name]
        if node_type in SUB_PACKAGE_NODE_TYPES:
            assert self.bench is not None, f"no bench for crypto key for {table!r}"
            return self.bench.encryption_key
        else:
            return GLOBAL_PG_CRYPTO_KEY


GLOBAL_CONTEXT = BenchContext(bench=None)
BENCH_TABLE_PREFIX = "bench_"
BENCH_RECORD_TABLE_PREFIX = "bench_record_"


def _get_node_table_name(node_type: NodeType) -> str:
    return f"{BENCH_TABLE_PREFIX}{node_type.name.lower().replace('_', '')}"


def _get_record_table_name(block: Block) -> str:
    return f"{BENCH_RECORD_TABLE_PREFIX}{block.tk}"


def map_node_class_to_table(node: type[Node]) -> Table:
    """Maps a node type into its Table schema."""
    table_name = _get_node_table_name(node.metatype)
    columns: list[Column] = []
    properties = list(node.__properties__.values())
    properties.sort(key=lambda p: p.id or -1)

    # extra constraints/indexes
    constraints: list[Constraint] = []
    indexes: list[Index] = []
    for uniqued_columns in node.__extra_uniques__:
        uniqued_columns = tuple(sorted(uniqued_columns))  # for consistency
        index_name = f"bench_idx_{'_'.join(uniqued_columns)}"
        index = Index(index_name, type=IndexType.BTREE, is_unique=True, columns=uniqued_columns)
        constraint = Constraint(
            index.inner_name,
            type=ConstraintType.UNIQUE,
            columns=uniqued_columns,
            index=index.inner_name,
        )
        indexes.append(index)
        constraints.append(constraint)
    for index in node.__extra_indexes__:
        index_name = f"bench_idx_{'_'.join(index)}"
        extra_index = Index(index_name, type=IndexType.BTREE, is_unique=False, columns=index)
        indexes.append(extra_index)

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
            elif isinstance(prop.default, int):
                column.default = str(prop.default)
            elif isinstance(prop.default, str):
                column.default = f"'{prop.default}'::character varying"
            else:
                raise TypeError(f"unexpected default in {prop!r}: {prop.default!r}")
        # FKs
        is_local = node.__is_local__ or any(
            NODE_CLASS_BY_TYPE[n].__is_local__ for n in prop.reference_nodes or ()
        )
        if (
            (prop.reference_kind == ReferenceKind.NODE_PARENT or prop.reference_force_fk)
            and not prop.is_list  # foreign keys must be scalar
            and prop.reference_nodes
            and (not is_local or node.metatype == prop.reference_nodes[0])
        ):
            assert len(prop.reference_nodes) == 1, f"stored prop {prop!r} has multiple references"
            column.is_foreign_key_to = _get_node_table_name(prop.reference_nodes[0])
            if prop.reference_kind in (ReferenceKind.NODE_PARENT, ReferenceKind.NODE_ANCESTOR):
                column.on_delete = CascadeAction.CASCADE
            else:
                column.on_delete = CascadeAction.SET_NULL

        if prop.is_indexed_in_pg or prop.is_unique:
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

    table = Table(
        _source=node.metatype.id,
        name=table_name,
        columns=tuple(columns),
        constraints=tuple(constraints),
        indexes=tuple(indexes),
    )
    return table


def map_database_block_to_table(block: Block) -> Table:
    table_name = _get_record_table_name(block)
    columns: list[Column] = []
    constraints: list[Constraint] = []
    indexes: list[Index] = []

    # nocheckin

    return Table(
        _source=block.tk,
        name=table_name,
        columns=tuple(columns),
        constraints=tuple(constraints),
        indexes=tuple(indexes),
    )


def _compile_expression_ref(
    node: Union[type[Node], Block],
    expr: Expression,
) -> SqlNode:
    if expr.property is not None:
        return sqlident(expr.property.name)
    else:
        raise TypeError(f"unexpected expression ref: {expr!r}")


def _pg_lower_conditional(node: Union[type[Node], Block], cond: Expression) -> Expression:
    """'Lowers' a conditional expression to a form that can be compiled to SQL."""
    prop = cond.property

    # translate general pointer queries into underlying id/ck queries
    if prop is not None and prop.reference_kind is not None:
        assert prop.reference_stored_ids, f"unexpected stored ids: {prop!r}"
        is_list = cond.op == ConditionalOp.IN or cond.op == ConditionalOp.NOT_IN
        id_prop = prop.reference_stored_ids[0]
        id_clause = C(op=cond.op, property=id_prop)
        if cond.value_packed is not None:
            id_clause.value = cond.value.id if not is_list else [r.id for r in cond.value]
        if (
            prop.reference_stored_metas
            and "ck" in prop.reference_stored_metas
            and prop.reference_stored_metas["ck"] is not id_prop
        ):
            ck_prop = prop.reference_stored_metas["ck"]
            ck_clause = C(op=cond.op, property=ck_prop)
            if cond.value_packed is not None:
                ck_clause.value = cond.value.ck if not is_list else [r.ck for r in cond.value]
            joint_clause = C(ConditionalOp.OR, clauses=[id_clause, ck_clause])
            return joint_clause
        else:
            return id_clause
    else:
        return cond


def _pg_compile_conditional(
    node: Union[type[Node], Block],
    cond: Expression,
) -> SqlNode:
    cond = _pg_lower_conditional(node, cond)
    if cond.op == LiteralOp.TRUE:
        return sqlstr("TRUE")
    elif cond.op == LiteralOp.FALSE:
        return sqlstr("FALSE")
    elif cond.op == LiteralOp.NONE:
        return sqlstr("NULL")
    elif cond.op in ExpressionOps.COND_COMPOUND and cond.op in PG_CONDITIONAL_OP_BY_BENCH:
        clauses = [_pg_compile_conditional(node, c) for c in cond.clauses or ()]
        if not clauses:
            # and/or/not <nothing> are all TRUE :EmptyCompoundConditional
            return sqlstr("TRUE")
        clause = SqlCompound(op=PG_CONDITIONAL_OP_BY_BENCH[cond.op], operands=clauses)
        return clause
    elif (
        cond.op in ExpressionOps.COND_COMPARISON or cond.op in ExpressionOps.COND_STRING
    ) and cond.op in PG_CONDITIONAL_OP_BY_BENCH:
        left = _compile_expression_ref(node, cond)

        if cond.op == ConditionalOp.IN:
            # map IN to = ANY() construct (IN/NOT IN doesn't work in psycopg)
            # see https://www.psycopg.org/psycopg3/docs/basic/from_pg2.html#you-cannot-use-in-s-with-a-tuple
            value = list(cond.value) if not isinstance(cond.value, list) else cond.value
            right = sqlstr("ANY({})").format(sql.Literal(value))
            return SqlComparison(left=left, op=PostgresConditionalOp.EQ, right=right)
        elif cond.op == ConditionalOp.NOT_IN:
            # and map NOT IN to != ALL() construct (see above)
            value = list(cond.value) if not isinstance(cond.value, list) else cond.value
            right = sqlstr("ALL({})").format(sql.Literal(value))
            return SqlComparison(left=left, op=PostgresConditionalOp.NEQ, right=right)
        elif cond.op == ConditionalOp.STARTS_WITH:
            right = sqlstr("{} || '%'").format(sql.Literal(cond.value))
        elif cond.op == ConditionalOp.ENDS_WITH:
            right = sqlstr("'%s' || {}").format(sql.Literal(cond.value))
        else:
            assert cond.value is not None, f"cannot compare {cond!r} with None"
            right = sql.Literal(cond.value)

        clause = SqlComparison(left=left, op=PG_CONDITIONAL_OP_BY_BENCH[cond.op], right=right)

        # coerce (x != y) -> (x != y or x IS NULL) if x is nullable
        if cond.op == ConditionalOp.NOT_EQUALS and (
            cond.property is not None and not cond.property.is_required
        ):
            null_clause = SqlUnary(
                op=PostgresConditionalOp.IS_NULL, left=_compile_expression_ref(node, cond)
            )
            clause = SqlCompound(op=PostgresConditionalOp.OR, operands=[clause, null_clause])

        return clause
    elif cond.op in ExpressionOps.COND_EXISTENCE:
        clause = SqlUnary(
            left=_compile_expression_ref(node, cond), op=PG_CONDITIONAL_OP_BY_BENCH[cond.op]
        )
        return clause
    raise ChannelIncapableError("postgres", expression=cond, reason="unsupported conditional")


def _pg_compile_sort(node: Union[type[Node], Block], sort: Expression) -> sql.Composed:
    field_ref = _compile_expression_ref(node, sort)
    sort_op = POSTGRES_SORT_OP_BY_BENCH[cast(SortOp, sort.op)]
    return sqlstr("{} {}").format(sql_node_to_sql(field_ref), sqlstr(sort_op))


def _pg_compile_sorts(
    node: Union[type[Node], Block], sorts: Collection[Expression]
) -> sql.Composed:
    return sqljoin(", ", (_pg_compile_sort(node, sort) for sort in sorts))


def _pack_object_data_prop_scalar(prop: Property, value: Any) -> SqlPrimitive:
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
    elif prop.primitive_type == PrimitiveType.INTERVAL:
        return value.ToTimedelta()
    else:
        return value


def _pack_object_data_prop(prop: Property, value: Any) -> SqlPrimitive:
    """Packs the value of a BuiltinObject property for storage in Postgres."""
    if value is None:
        return None
    elif not prop.is_list:
        return _pack_object_data_prop_scalar(prop, value)
    else:
        return [_pack_object_data_prop_scalar(prop, v) for v in value]


def _pack_object_value_prop_scalar(prop: Property, value: Any) -> SqlPrimitive:
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
    elif prop.primitive_type == PrimitiveType.INTERVAL:
        return timedelta_from_isoformat(value)
    else:
        return value


def _pack_object_value_prop(prop: Property, value: Any) -> SqlPrimitive:
    """Packs the JSON-value-packed value of a BuiltinObject for storage in Postgres."""
    if value is None:
        return None
    elif not prop.is_list:
        return _pack_object_value_prop_scalar(prop, value)
    else:
        return [_pack_object_value_prop_scalar(prop, v) for v in value]


def _unpack_object_data_prop_scalar(prop: Property, value: Any, into: Any | None = None) -> Any:
    """Unpacks the value of a BuiltinObject property from Postgres."""
    if value is None:
        return None
    elif prop.reference_struct:
        return unpack_builtin_object_data(value, into=into)
    elif prop.primitive_type == PrimitiveType.UUID:
        return str(value)
    elif prop.primitive_type == PrimitiveType.JSON:
        return pack_proto_json(value)
    elif prop.primitive_type == PrimitiveType.DATETIME:
        ts = Timestamp()
        ts.FromDatetime(value)
        return ts
    elif prop.primitive_type == PrimitiveType.DATE:
        return pack_proto_date(value)
    elif prop.primitive_type == PrimitiveType.TIME:
        return pack_proto_time(value)
    elif prop.primitive_type == PrimitiveType.INTERVAL:
        dur = Duration()
        dur.FromTimedelta(value)
        return dur
    else:
        return value


def _pg_pack_node_reference_into_row(
    prop: Property | Any,
    row: dict[str, Any],
    value: NodeReferenceData | Collection[NodeReferenceData] | None,
) -> None:
    """
    'Unravels' a wired pointer into (one or more) stored columns as needed :StoredPointers
    NOTE :Cleanup: pg_pack_node_reference/pg_unpack_node_reference are way too much code
    """
    if prop.reference_is_rich:
        # stored as struct (jsonb)
        assert prop.reference_wired_ptr is not None, f"no wired/stored ptr for {prop!r}"
        prop = prop.reference_wired_ptr
        row[prop.name] = _pack_object_data_prop(prop, value)
        return
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
        for meta_key, meta_prop in prop.reference_stored_metas.items():
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
            if reference is not None and reference.node_type in stored_prop.reference_nodes:
                row[stored_prop.name] = reference.id
            else:
                row[stored_prop.name] = None
        # additional pointer metadata
        for meta_key, meta_prop in prop.reference_stored_metas.items():
            if value is None:
                row[meta_prop.name] = None
            else:
                row[meta_prop.name] = getattr(value, meta_key) or None


def _pg_unpack_node_reference_from_row(prop: Property, row: RowOut, node: AnyNodeData) -> None:
    """
    'Ravels' a wired pointer from (one or more) stored columns :StoredPointers
    """

    if prop.reference_is_rich:
        # stored as struct (jsonb)
        assert prop.reference_wired_ptr is not None, f"no wired/stored ptr for {prop!r}"
        prop = prop.reference_wired_ptr
        value = row.get(prop.name)
        if value is not None:
            ptr = _unpack_object_data_prop_scalar(prop, value)
            wired_name = wiring.get_rich_reference_prop_name(prop, ptr)
            getattr(node, wired_name).CopyFrom(ptr)
        return

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
                ptr.metatype = wire.ObjectType.OBJECT_TYPE_NODE_REFERENCE
                ptr.id = str(id)
                ptr.node_type = cast(list[wire.NodeType], stored_prop.reference_nodes)[0]
        # additional pointer metadata
        for i, ptr in enumerate(ptrs):
            for meta_key, meta_prop in prop.reference_stored_metas.items():
                extra_value = cast(list, row.get(meta_prop.name))[i]
                if extra_value is None:
                    continue
                elif meta_prop.primitive_type == PrimitiveType.UUID:
                    extra_value = str(extra_value)
                elif meta_prop.enum_type == EnumType.NODE_TYPE:
                    extra_value = NodeType(extra_value)
                else:
                    raise RuntimeError(f"unexpected meta prop type: {meta_prop!r}")
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
                    metatype=wire.ObjectType.OBJECT_TYPE_NODE_REFERENCE,
                    id=str(value),
                    # if this is a heterogeneous ck pointer, type will be overwritten from extras
                    node_type=cast(list[wire.NodeType], stored_prop.reference_nodes)[0],
                )
                break
        else:
            return

        # additional pointer metadata
        for meta_key, meta_prop in prop.reference_stored_metas.items():
            extra_value = row.get(meta_prop.name)
            if extra_value is None:
                continue
            elif meta_prop.primitive_type == PrimitiveType.UUID:
                extra_value = str(extra_value)
            elif meta_prop.enum_type == EnumType.NODE_TYPE:
                extra_value = NodeType(extra_value)
            else:
                raise RuntimeError(f"unexpected meta prop type: {meta_prop!r}")
            setattr(ptr, meta_key, extra_value)
        if not ptr.ck:
            ptr.ck = ptr.id
        if bench_id and not ptr.bench_id:
            ptr.bench_id = bench_id
            if ptr.base_ck:
                ptr.base_bench_id = ptr.bench_id
        getattr(node, prop.reference_wired_ptr.name).CopyFrom(ptr)


def _pg_pack_node_data_row(node: AnyNodeData) -> dict[str, SqlPrimitive]:
    """Packs a node's data into a row for the respective table."""
    node_cls = NODE_CLASS_BY_TYPE[wiring.unpack_enum(NodeType, node.metatype)]
    try:
        row: dict[str, SqlPrimitive] = {}
        for name, prop in node_cls.__wired_properties__.items():
            if prop.reference_source is None:
                # regular non-ref property
                if prop.is_optional_scalar and not node.HasField(name):
                    value = None
                else:
                    value = getattr(node, name)
                row[name] = _pack_object_data_prop(prop, value)
            else:
                # unravel stored node reference :StoredPointers
                wired_name = cast(Property, prop.reference_source.reference_wired_ptr).name
                if prop.is_optional_scalar and not node.HasField(wired_name):
                    value = None
                else:
                    if prop.reference_is_rich:
                        wired_name = node.WhichOneof(wired_name)
                    value = getattr(node, wired_name)
                _pg_pack_node_reference_into_row(prop.reference_source, row, value)
        return row
    except (AttributeError, TypeError, ValueError, KeyError) as e:
        raise ValueError(f"could not pack row {node_cls.metatype.name}: {struct!r}") from e


def _pg_unpack_node_data_row(node_cls: type[Node], row: Mapping[str, Any]) -> AnyNodeData:
    """Unpacks a node's data from a row from the respective table."""
    try:
        proto_cls = PROTO_CLASS_BY_TYPE[node_cls.metatype]
        obj_data = cast(
            AnyNodeData, proto_cls(metatype=wiring.pack_enum(NodeType, node_cls.metatype))
        )  # type: ignore
        for name, prop in node_cls.__wired_properties__.items():
            if prop.reference_source is not None:
                # ravel stored node reference :StoredPointers
                _pg_unpack_node_reference_from_row(prop.reference_source, row, obj_data)
                continue

            # regular non-ref property
            value = row.get(name)
            if value is None:
                continue
            if not prop.is_list:  # scalar
                packed_value = _unpack_object_data_prop_scalar(prop, value)
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
                        _ = _unpack_object_data_prop_scalar(prop, item, into=packed_item)
                else:
                    for item in value:
                        packed_item = _unpack_object_data_prop_scalar(prop, item)
                        packed_value.append(packed_item)

        return obj_data
    except (AttributeError, TypeError, ValueError, KeyError) as e:
        row_str = repr(row) if IS_DEV else describe_type(row)
        raise ValueError(f"could not unpack row {node_cls.metatype.name}: {row_str}") from e


def _combine_filter(*, include_deleted: bool, filter: Expression | None) -> Expression | None:
    if include_deleted:
        return filter
    else:
        if filter is None:
            return FILTER_NOT_DELETED
        else:
            return filter & FILTER_NOT_DELETED


@_trace_pg_span
async def pg_graph_select(
    *, cur: psycopg.AsyncCursor, ctx: SqlContext, query: QueryBuilder
) -> list[AnyNodeData]:
    """Selects the nodes from the graph matching the given query."""
    # nocheckin: pg_graph_select for Records
    # compile
    options = query._select or DEFAULT_READ_OPTIONS
    node_cls = NODE_CLASS_BY_TYPE[query._node_type]
    node_table = TABLE_BY_NODE_TYPE[query._node_type]
    columns = [
        node_table._columns_by_name[prop.name] for prop in options.get_properties(query._node_type)
    ]
    assert any(c.is_primary_key for c in columns), f"no primary key selected in {columns!r}"
    filter = _combine_filter(include_deleted=query._include_deleted, filter=query._filter)
    where = _pg_compile_conditional(node_cls, filter) if filter is not None else None
    order_by = _pg_compile_sorts(node_cls, query._sort) if query._sort else None

    # select
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
    nodes_data = [_pg_unpack_node_data_row(node_cls, row) for row in rows]
    return nodes_data


@_trace_pg_span
async def pg_graph_count(*, cur: psycopg.AsyncCursor, ctx: SqlContext, query: QueryBuilder) -> int:
    """Counts the nodes from the graph matching the given query."""
    # nocheckin: pg_graph_count for Records
    # compile
    node_cls = NODE_CLASS_BY_TYPE[query._node_type]
    node_table = TABLE_BY_NODE_TYPE[query._node_type]
    filter = _combine_filter(include_deleted=query._include_deleted, filter=query._filter)
    where = _pg_compile_conditional(node_cls, filter) if filter is not None else None

    # count
    return await pg_count(cur=cur, ctx=ctx, table=node_table, where=where)


@_trace_pg_span
async def pg_graph_exists(
    *, cur: psycopg.AsyncCursor, ctx: SqlContext, query: QueryBuilder
) -> bool:
    """Checks if nodes from the graph matching the given query exist."""
    # nocheckin: pg_graph_exists (also for Records)
    ...


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
        and t in TABLE_BY_NODE_TYPE
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
                op=ConditionalOp.IN,
                property=child_cls.__parent_property__.reference_stored_ids[0],
                value=parent_ids,
            )
            if extra_filter is not None:
                parent_filter = parent_filter & extra_filter

            # collect children
            child_table = TABLE_BY_NODE_TYPE[child_type]
            assert child_table, f"no table for {child_cls!r}"
            assert child_table._primary_key, f"no primary key for {child_cls!r}"
            parent_where = _pg_compile_conditional(child_cls, parent_filter)
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
                    metatype=wire.ObjectType.OBJECT_TYPE_NODE_REFERENCE,
                    id=str(child_row["id"]),
                    node_type=cast(wire.NodeType, child_type),
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
    query: QueryBuilder,
    visited_graph: NodeDataGraph,
) -> None:
    """
    Gets the 'root' nodes from a Query (QueryBuilder.roots) and recursively reads up/down the graph.
    Also performs any additional joins needed for the query.
    """
    # NOTE :Performance: we could read all package contents with package_id=x if we know it's a package query.
    assert query._roots, "no roots to select"
    roots = [r._to_data() for r in query._roots]

    # get roots
    root_nodes: list[AnyNodeData]
    if isinstance(roots[0], (NodeReferenceData, FileReferenceData, SecretReferenceData)):
        # select roots
        roots_ids = [node.id for node in roots]
        root_filter = C(ConditionalOp.IN, property=Node.id, value=roots_ids)
        root_query = QueryBuilder(
            QueryType.GET,
            node_type=query._node_type,
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
                parents_filter = C(ConditionalOp.IN, property=Node.id, value=node_ids)
                parents_query = QueryBuilder(
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
            extra_filter=FILTER_NOT_DELETED if not query.include_deleted else None,
        )
        descendant_node_ptrs_by_type = group_by(descendant_node_ptrs, lambda ptr: ptr.node_type)
        for wire_node_type, node_ptrs in descendant_node_ptrs_by_type.items():
            node_type = NodeType(wire_node_type)
            children_filter = C(
                ConditionalOp.IN, property=Node.id, value=[ptr.id for ptr in node_ptrs]
            )
            children_query = QueryBuilder(
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
    query: QueryBuilder,
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
            wiring.unpack_object(
                NodeReference._ref_data_from_node_data(r), expect=NodeReference, supergraph=None
            )
            for r in roots
        ]
        get_query = QueryBuilder(
            QueryType.GET,
            query._node_type,
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

    assert edits[0].node_ptr is not None, f"no node ptr for {edits[0]!r}"
    batch_node_cls = NODE_CLASS_BY_TYPE[wiring.unpack_enum(NodeType, edits[0].node_ptr.node_type)]
    batch_updated_properties: bitarray = bitarray(batch_node_cls.__max_property_ord__ + 1)
    batch: list[EditData] = []
    all_cascaded_edits: list[EditData] = []

    # batch operations by kind and edit type
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
        if (
            next_edit is not None
            and next_edit.type == prev_edit.type
            and next_edit.node_ptr is not None
            and next_edit.node_ptr.node_type == prev_edit.node_ptr.node_type
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
            batch=batch,
            updated_properties=updated_properties,
        )

        # start new batch if needed
        if next_edit is not None:
            batch_node_cls = NODE_CLASS_BY_TYPE[
                wiring.unpack_enum(NodeType, next_edit.node_ptr.node_type)
            ]
            batch_updated_properties = bitarray(batch_node_cls.__max_property_ord__ + 1)
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
    removed_at_by_root_node_id: dict[str, datetime.datetime] = {}
    if edit_type == EditType.RESTORE:
        # only cascade to nodes that were removed at the exact same time
        removed_dts = []
        for root_edit in batch:
            assert root_edit.HasField("old_edited_at"), f"no old edited at for {root_edit!r}"
            removed_at = root_edit.old_edited_at.ToDatetime(tzinfo=pytz.utc)
            removed_dts.append(removed_at)
            removed_at_by_root_node_id[cast(str, root_edit.node_ptr.id)] = removed_at
        extra_filter = C(op=ConditionalOp.IN, property=Node.deleted_at, value=removed_dts)
    elif edit_type == EditType.ERASE:
        # cascade to all
        extra_filter = None
    else:
        # only cascade to visible
        extra_filter = FILTER_NOT_DELETED

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
                type=cast(wire.EditType, edit_type),
                node_ptr=node_ptr,
                edited_at=root_edit.edited_at,
                epoch=root_edit.epoch,
                scope=root_edit.scope,  # should always be the same
                category=root_edit.category,
                context=root_edit.context,
            )
            if root_edit.HasField("subject_ptr"):
                cascaded_edit.subject_ptr.CopyFrom(root_edit.subject_ptr)
            all_cascaded_edits.append(cascaded_edit)
            root_edit_by_cascaded_node_id[cast(str, node_ptr.id)] = root_edit

    # batch operations by edit kind and node type
    cascaded_edits_by_type = group_by(all_cascaded_edits, lambda edit: edit.node_ptr.node_type)
    for descendant_node_type, cascaded_edits in cascaded_edits_by_type.items():
        _ = await _pg_edit_batch(
            cur=cur,
            ctx=ctx,
            edit_type=edit_type,
            node_type=cast(NodeType, descendant_node_type),
            batch=cascaded_edits,
            updated_properties=(),
        )
        nodes_ids = [edit.node_ptr.id for edit in cascaded_edits]
        descendant_query = QueryBuilder(
            QueryType.GET,
            node_type=NodeType(descendant_node_type),
            filter=C(ConditionalOp.IN, property=Node.id, value=nodes_ids),
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
    batch: list[EditData],
    updated_properties: tuple[Property, ...],  # across batch
) -> None:
    """Applies a batch of edits to the graph (without cascading)."""
    trace.get_current_span().set_attributes(
        {"edit_type": edit_type.bench_name, "edits": len(batch)}
    )

    node_cls = NODE_CLASS_BY_TYPE[node_type]
    node_table = TABLE_BY_NODE_TYPE[node_type]
    assert node_table is not None, f"no table for {node_cls!r}"
    assert node_table._primary_key is not None, f"no primary key for {node_cls!r}: {node_table!r}"

    if edit_type in (EditType.CREATE, EditType.UPSERT):
        nodes = []
        rows = []
        for edit in batch:
            assert edit.epoch is not None, f"no epoch for {edit!r}"
            assert edit.HasField("node_data"), f"no node_data for {edit!r}"
            node = wiring.unwrap_some_node(edit.node_data)
            nodes.append(node)
            # inline implicit metadata
            row: dict[str, SqlPrimitive] = _pg_pack_node_data_row(node)
            row["created_at"] = row["updated_at"] = edit.edited_at.ToDatetime(tzinfo=pytz.utc)
            if "created_epoch" in node_cls.__properties__:
                row["created_epoch"] = row["updated_epoch"] = edit.epoch
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
        if issubclass(node_cls, BenchNode):
            implicit_properties.append(node_cls.updated_epoch)
        if edit_type in (EditType.DELETE, EditType.RESTORE):
            implicit_properties.append(node_cls.deleted_at)
        dynamic_columns: list[Column] = [node_table._primary_key]
        for prop in chain(implicit_properties, updated_properties):
            if prop.is_node_reference:
                dynamic_columns.extend(
                    node_table._columns_by_name[p.name] for p in prop.reference_stored_props or ()
                )
            else:
                dynamic_columns.append(node_table._columns_by_name[prop.name])

        # collect dynamic values
        dynamic_values: list[RowIn] = []
        for edit in batch:
            assert edit.node_ptr is not None, f"no node ptr for {edit!r}"
            assert edit.epoch is not None, f"no epoch for {edit!r}"
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
                if not prop.is_node_reference:
                    # regular non-ref property
                    value = _pack_object_value_prop(prop, new_value_packed)
                    row[prop.name] = value
                else:
                    # unravel stored node reference :StoredPointers
                    wired_name = cast(Property, prop.reference_wired_ptr).name
                    if new_value_packed is None:
                        _pg_pack_node_reference_into_row(prop, row, None)
                    elif prop.reference_is_rich:
                        row[wired_name] = Jsonb(new_value_packed)
                    else:
                        new_value = unpack_value_data(
                            new_value_packed, prop.type_info, wrap_primitive=False
                        )
                        _pg_pack_node_reference_into_row(prop, row, new_value)  # type: ignore

            # implicit properties
            edited_at = edit.edited_at.ToDatetime()
            row["updated_at"] = edited_at
            if "updated_epoch" in node_cls.__properties__:
                row["updated_epoch"] = edit.epoch
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
# General table registry
# (this needs to run after setup)
#

TABLE_BY_NODE_TYPE: dict[NodeType, Table] = {
    # read previously generated tables in schema.py
    node_type: getattr(schema, f"{to_casing(node_type.name, Casing.ALL_CAPS)}_TABLE")
    for node_type in NODE_TYPES
    if hasattr(schema, f"{to_casing(node_type.name, Casing.ALL_CAPS)}_TABLE")
}
NODE_TYPE_BY_TABLE_NAME: dict[str, NodeType] = {
    table.name: node_type for node_type, table in TABLE_BY_NODE_TYPE.items()
}
NODE_TABLES: tuple[Table, ...] = tuple(TABLE_BY_NODE_TYPE.values())
GLOBAL_TABLES: tuple[Table, ...] = DEFAULT_GLOBAL_TABLES + tuple(
    TABLE_BY_NODE_TYPE[node.metatype]
    for node in NODE_CLASSES
    if not node.__is_local__
    and node.__is_stored__
    and not node.__is_stored_custom__
    and node.metatype in TABLE_BY_NODE_TYPE
)
LOCAL_TABLES: tuple[Table, ...] = DEFAULT_LOCAL_TABLES + tuple(
    TABLE_BY_NODE_TYPE[node.metatype]
    for node in NODE_CLASSES
    if node.__is_local__
    and node.__is_stored__
    and not node.__is_stored_custom__
    and node.metatype in TABLE_BY_NODE_TYPE
)
ALL_TABLES: tuple[Table, ...] = (
    *GLOBAL_TABLES,
    *(t for t in LOCAL_TABLES if not any(t.name == g.name for g in GLOBAL_TABLES)),
)
GLOBAL_SCHEMA = Schema(GLOBAL_EXTENSIONS, GLOBAL_TABLES)
LOCAL_SCHEMA = Schema(LOCAL_EXTENSIONS, LOCAL_TABLES)
OMNI_SCHEMA = Schema(ALL_EXTENSIONS, ALL_TABLES)
