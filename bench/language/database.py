import enum
import typing
from copy import deepcopy
from typing import Optional
from uuid import UUID

import psycopg
import structlog
from psycopg import sql

from bench.language.const import (
    UNSET,
    ConditionalOp,
    NodeType,
    new_dynamic_node_key,
)
from bench.language.expression import C, Expression, coerce_conditional
from bench.language.node import (
    Node,
    NodeList,
    NodeListBase,
    NodeStatus,
    NRel,
    Property,
    ScopeNode,
    _Passthrough,
    node,
    node_component,
    p_child,
    p_internal,
    p_parent,
    p_runtime,
)
from bench.language.notice import NoticeHandler
from bench.language.query import StoreEngine, QueryBase
from bench.language.value import HasValue
from bench.sql.core import RECORD_EPHEMERAL_TABLE, PrimitiveType, Table
from bench.utils.dt import utcnow_with_tz
from bench.utils.func import describe_type
from bench.proto.wire import RecordData

if typing.TYPE_CHECKING:
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
class Record(HasValue, Node):
    """A record in a database. The containing table is usually a real Postgres table."""

    # :RecordSchema
    parent: "Block" = p_parent(4, NodeType.BLOCK)
    value_packed: typing.Any | None = p_internal(
        30,
        default_factory=dict,
        copy=deepcopy,
        primitive_type=PrimitiveType.JSON,
        ignore_conflicts=True,
    )

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


class RecordStoreEngine(StoreEngine[Record, RecordData]):
    def __init__(self, cur: psycopg.AsyncCursor):
        self._cur = cur

    @property
    def _combined_filter(self) -> Expression:
        """Combines the custom with the default filter for this database."""
        return Expression.and_if_set(
            self._filter,
            C(ConditionalOp.EQUALS, field_key="block_key", value=self._database.dynamic_key),
            ~C(ConditionalOp.EXISTS, field_key="deleted_at"),
        )

    async def _fetch(self, *, _no_flush: bool = False) -> list[Record]:
        """Fetches the result set for this query."""
        from bench.proto import wiring
        from bench.sql.engine import pg_select_records_data

        session = self._database.session
        if not _no_flush and session._local_edits:
            await session.flush(local_only=True)  # first flush any local edits
        records, cursors, start_cursor = await pg_select_records_data(
            cur=pg_cursor,
            database=self._database,
            where=where,
            sort=self._sort,
            after=after,
            first=first,
            skip=self._skip,
        )
        records: list[Record] = []
        for record_data in fetched.records:
            record: Record = wiring.unpack_node(record_data, self._database, session)
            record._track_self(session)
            records.append(record)

        logger.debug("record.query.done", query=self, results=len(records))
        return records

    async def count(self, filter: Expression = None, *, _no_flush: bool = False, **kwargs) -> int:
        """Returns the number of results. May refine the query."""
        from bench.sql.engine import compile_pg_conditional, pg_count

        assert not self._first and not self._skip and not self._sort, "cannot count with limits"
        filter = coerce_conditional(self._database, filter, kwargs)

        if not _no_flush and self._database.session._local_edits:
            await self._database.session.flush(local_only=True)

        where = compile_pg_conditional(self._database, filter & self._combined_filter)
        return await pg_count(
            cur=await self._database._get_pg_cursor(), table=self._database._table, where=where
        )

    async def exists(self, filter: Expression = None, *, _no_flush: bool = False, **kwargs) -> bool:
        """Whether any results exist. May refine the query."""
        from bench.sql.engine import compile_pg_conditional, pg_exists

        filter = coerce_conditional(self._database, filter, kwargs, return_none_if_empty=True)

        assert not self._first and not self._skip and not self._sort, "cannot exists with limits"
        if filter is None and self._cached_records is not None:
            return bool(self._cached_records)

        if self._database.session._local_edits:
            await self._database.session.flush(local_only=True)

        if filter is None:
            filter = self._combined_filter
        else:
            filter = filter & self._combined_filter
        where = compile_pg_conditional(self._database, filter)
        return await pg_exists(
            cur=await self._database._get_pg_cursor(), table=self._database._table, where=where
        )

    async def update(self, **value) -> int:
        """Updates all results with the given values."""
        from bench.language.value import check_type, pack_value
        from bench.sql.engine import compile_pg_conditional, pg_update_static, pg_wrap_record_value

        if not value:
            raise ValueError(f"no values given to update {self!r}")

        session = self._database.session
        if session._local_edits:
            await session.flush(local_only=True)

        # 'serialize' values (probably need a better way here to retain some native types?)
        check_type(value, self._database)
        value = pack_value(
            value, self._database, ignore_outer=True, map_k=lambda f: (f.py_ident, f.storage_key)
        )
        value = pg_wrap_record_value(self._database, value)
        # update values alongside
        now = utcnow_with_tz()
        value["revision"] = sql.SQL("revision + 1")
        value["updated_at"] = now
        updated_rows = await pg_update_static(
            cur=await self._database._get_pg_cursor(),
            table=self._database._table,
            where=compile_pg_conditional(self._database, self._combined_filter),
            static_value=value,
            returning=[self._database._table._columns_by_name["id"]],
        )
        updated_records_ids = {r["id"] for r in updated_rows}
        session._records_changed(self._database, updated_records_ids)  # mark for OS sync
        return len(updated_rows)

    async def delete(self) -> int:
        """Deletes all results."""
        from bench.sql.engine import compile_pg_conditional, pg_delete

        self._require_engine(StoreEngineType.LOCAL_STORE)
        assert not self._engine, "cannot delete with forced query engine"
        session = self._database.session
        if session._local_edits:
            await session.flush(local_only=True)
        where = Expression.and_if_set(
            self._filter,
            C(ConditionalOp.EQUALS, field_key="block_key", value=self._database.dynamic_key),
        )
        deleted_rows = await pg_delete(
            cur=await self._database._get_pg_cursor(),
            table=self._database._table,
            where=compile_pg_conditional(self._database, where),
            returning=[self._database._table._columns_by_name["id"]],
        )
        deleted_records_ids = {r["id"] for r in deleted_rows}
        session._records_changed(self._database, deleted_records_ids)  # mark for OS sync
        return len(deleted_rows)


class RelationType(enum.StrEnum):
    OneToOne = "OneToOne"
    OneToMany = "OneToMany"
    ManyToMany = "ManyToMany"
    ManyToOne = "ManyToOne"


class RecordList(NodeListBase[Record], QueryBase[Record]):
    """A NodeList for remote records."""

    def __init__(self, parent: "ScopeNode", property: Property):
        NodeListBase[Record].__init__(self, parent, property)
        QueryBase.__init__(self, node_type=NodeType.RECORD, base=parent, cache=False)

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

    def _clear_inner(self, scope: Optional["ScopeNode"] = None) -> None:
        self._table = None

    def _interp_inner(self, scope: "ScopeNode", on_notice: "NoticeHandler") -> None:
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

    # maybe these node methods should also go into passthrough?

    def _iter_inner(self):
        return iter(self.records)

    def _aiter_inner(self):
        return aiter(self.records)

    def _len_inner(self):
        return len(self.records)

    def _getitem_inner(self, item):
        return self.records[item]
