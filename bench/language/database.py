from typing import TYPE_CHECKING, Any, Optional
from uuid import UUID

import structlog

from bench.language.const import UNSET, ConditionalOp, NodeType, new_dynamic_node_key, AggregationOp
from bench.language.expression import C
from bench.language.node import (
    Node,
    NodeList,
    NodeListBase,
    NodeStatus,
    NRel,
    Property,
    _Passthrough,
    node,
    node_component,
    p_child,
    p_internal,
    p_parent,
    p_runtime,
    p_secret_value_packed,
    p_value_packed,
    p_value_runtime,
)
from bench.language.notice import NoticeHandler
from bench.language.query import (
    PostgresEngine,
    QueryBuilder,
    AggregateResult,
    StoreEngineIncapableError,
    PostgresConnection,
    FetchResult,
    FetchOptions,
)
from bench.language.value import HasValues
from bench.proto.wire import RecordData, AggregationData
from bench.sql.core import RECORD_EPHEMERAL_TABLE, Table
from bench.utils.func import describe_type

if TYPE_CHECKING:
    from bench.language import Block, Query

logger = structlog.get_logger(__name__)

LOCAL_RECORD_CACHE_LIMIT = 2048
RECORD_UNSPECIFIED_BATCH_SIZE = 500


@node(
    NodeType.RECORD,
    passthrough=(("value", _Passthrough.Full),),
    stored_custom=True,
    index_in_search=True,
    local=True,
)
class Record(HasValues):
    """A record in a database. The containing table is usually a real Postgres table."""

    # :RecordSchema
    parent: "Block" = p_parent(4, NodeType.BLOCK)
    value_packed: Any = p_value_packed(30)
    secret_value_packed = p_secret_value_packed(31)
    value = p_value_runtime(30, 31, type=4)

    # could also have Record.secret_value_packed as in Block (no materialization needed?)

    @staticmethod
    def new(
        *args,
        for_parent: "Block" = None,
        _status: NodeStatus = None,
        _id: UUID = None,
        _ck: UUID = None,
        **kwargs,
    ) -> "Record":
        from bench.language.value import check_type

        if not for_parent and args:
            raise TypeError(f"cannot create record with args {args} without for_parent")

        value = kwargs
        id_kwargs = {}
        if _id is not None:
            if not isinstance(_id, UUID):
                raise TypeError(f"invalid id {_id} ({type(_id)})")
            id_kwargs["id"] = _id
        if _ck is not None:
            if not isinstance(_ck, UUID):
                raise TypeError(f"invalid ck {_ck} ({type(_ck)})")
            id_kwargs["ck"] = _ck
        if for_parent and for_parent.is_attached:
            # inline args and check type for instant feedback
            for field, arg in zip(for_parent.fields, args):
                value[field.name] = arg
            check_type(value, for_parent)
        return Record(value=value, _status=_status, **id_kwargs)

    def __content_str__(self):
        return f"{describe_type(self.value) or '<empty>'}"

    @property
    def _type(self) -> Optional["Block"]:
        return self.parent._as_type

    @property
    def keys(self):
        return self.value.keys

    def __contains__(self, item: str):
        return item in self.value

    def __setitem__(self, key, value):
        self.value[key] = value


class RecordPostgresEngine(PostgresEngine[Record, RecordData]):
    pass


class RecordConnection(PostgresConnection[Record, RecordData]):
    async def fetch(
        self, query: "QueryBuilder[Record, RecordData]", options: FetchOptions
    ) -> FetchResult:
        from bench.sql.engine import (
            compile_pg_conditional_maybe,
            compile_pg_sorts,
            pg_select_records_data,
            pg_count,
        )
        from bench.language.expression import NodeReference

        filter = query._filter & C(
            ConditionalOp.EQUALS, field_key="block_key", value=query._base.dynamic_key
        )
        where = compile_pg_conditional_maybe(query._node_cls, filter)
        sort = compile_pg_sorts(query._sort)
        records, cursors, start_cursor = await pg_select_records_data(
            cur=self._cur,
            database=query._base,
            where=where,
            sort=sort,
            after=query._after,
            first=query._first,
            skip=query._skip,
        )
        roots = tuple(NodeReference.from_node_data(record) for record in records)

        if options.count:
            total = await pg_count(self._cur, query._node_cls.__table__, where)
        else:
            total = None

        return FetchResult(
            nodes=records, roots=roots, cursors=cursors, start_cursor=start_cursor, total=total
        )

    async def aggregate(self, query: "QueryBuilder[Record, RecordData]") -> AggregateResult:
        from bench.sql.engine import compile_pg_conditional_maybe, pg_count, pg_exists

        filter = query._filter & C(
            ConditionalOp.EQUALS, field_key="block_key", value=query._base.dynamic_key
        )
        if query._aggregation.op == AggregationOp.EXISTS:
            exists = await pg_exists(
                self._cur,
                query._node_cls.__table__,
                compile_pg_conditional_maybe(query._node_cls, filter),
            )
            return AggregateResult(AggregationData(exists=exists))
        elif query._aggregation.op == AggregationOp.COUNT:
            where = compile_pg_conditional_maybe(query._node_cls, filter)
            count = await pg_count(self._cur, query._node_cls.__table__, where)
            return AggregateResult(AggregationData(count=count))
        else:
            raise StoreEngineIncapableError(
                self, query, expr=query._aggregation, reason="unsupported"
            )


class RecordList(NodeListBase[Record], QueryBuilder[Record, RecordData]):
    """A NodeList for remote records."""

    def __init__(self, parent: "Node", property: Property):
        NodeListBase[Record].__init__(self, parent, property)
        QueryBuilder.__init__(self, node_type=NodeType.RECORD, base=parent, cache=False)

    def __str__(self):
        return f"from {self._parent._table.name}"

    def append(self, node: Record):
        assert isinstance(node, Record), f"cannot append {node!r} to {self!r}"
        node.parent = self._parent
        if node.id is None and self._parent.is_attached:
            node._assign_id(self._parent.package.id)
        if not self._parent.is_attached:
            raise RuntimeError(f"cannot create {node!r} in detached {self!r}")
        if self._parent._session and self._parent.is_attached:
            self._parent.session.create(node)

    def extend(self, *nodes: Record) -> None:
        for record in nodes:
            self.append(record)

    def remove(self, node: Record) -> None:
        if self._parent._session:
            self._parent._session.delete(self, node)
        if not self._parent.is_attached:
            self._parent._root_graph.remove(node)
        node.parent = None


@node_component
class HasDatabase(Node):
    dynamic_key: str | None = p_internal(UNSET, default=None)
    queries: NodeList["Query"] = p_child(NodeType.QUERY, NRel.NAMED | NRel.SCOPED | NRel.ORDERED)
    records: RecordList[Record] = p_child(
        NodeType.RECORD, NRel.STORED_CUSTOM, custom_list=RecordList
    )
    _table: Optional[Table] = p_runtime(default=None)

    def _init_inner(self):
        # this runs before HasFields because of the ordering in
        #  (which is necessary because HasFields also sets key)
        if self._is_new and self.dynamic_key is None:
            self.dynamic_key = self._derive_dynamic_key()

    def _clear_inner(self, scope: Optional["Node"] = None) -> None:
        self._table = None

    def _interp_inner(self, scope: "Node", on_notice: "NoticeHandler") -> None:
        from bench.sql.engine import map_database_to_pg_table

        if self.ephemeral:
            self._table = RECORD_EPHEMERAL_TABLE
        else:
            self._table = map_database_to_pg_table(self)

    @property
    def is_materialized(self):
        # should this database be a real database table
        return True

    @property
    def ephemeral(self) -> bool:
        return not self.is_materialized

    @staticmethod
    def _derive_dynamic_key(instance: "HasDatabase") -> str | None:
        if not instance.shared:
            if instance.id is not None:
                return new_dynamic_node_key(instance.id)
            else:
                return None
        else:
            return new_dynamic_node_key(instance.ck)
