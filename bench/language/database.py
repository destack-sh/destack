import enum
import typing
from copy import deepcopy
from typing import Optional
from uuid import UUID

import psycopg
import structlog
from asgiref.sync import async_to_sync
from psycopg import sql

from bench.language.const import UNSET, ConditionalOp, NodeType, QueryEngine, new_dynamic_node_key
from bench.language.expression import C, Expression, ExpressionOps, coerce_conditional, coerce_sort
from bench.language.notice import NoticeHandler
from bench.language.node import (
    _NC,
    NS,
    Node,
    NodeList,
    NodeListBase,
    NRel,
    Property,
    ScopeNode,
    _Passthrough,
    node,
    node_children,
    node_component,
    node_parent,
    struct_internal,
    struct_property,
    struct_runtime,
)
from bench.language.value import HasValue
from bench.sql.core import RECORD_EPHEMERAL_TABLE, PrimitiveType, Table
from bench.utils.dt import utcnow_with_tz
from bench.utils.func import _auto_async_to_sync, describe_type

if typing.TYPE_CHECKING:
    from bench.language import Block, Query
    from bench.proto.wire import RecordData  # noqa: F401

logger = structlog.get_logger(__name__)

LOCAL_RECORD_CACHE_LIMIT = 2048
RECORD_UNSPECIFIED_BATCH_SIZE = 500


@node(
    NodeType.RECORD,
    passthrough=(("value", _Passthrough.Full),),
    stored_custom=True,
    index_in_os=True,
    local=True,
)
class Record(HasValue, Node):
    """A record in a database. The containing table is usually a real Postgres table."""

    # :RecordSchema
    parent: "Block" = node_parent(4, NodeType.BLOCK)
    value_packed: typing.Any | None = struct_property(
        30,
        default_factory=dict,
        copy=deepcopy,
        primitive_type=PrimitiveType.JSON,
        ignore_conflicts_with=(HasValue,),
    )

    # could also have Record.secret_value_packed as in Block (no materialization needed?)

    @staticmethod
    def new(
        *args,
        for_parent: "Block" = None,
        _status: NS = None,
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
        if for_parent and for_parent.attached:
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


_RecordFetchResult = typing.NamedTuple(
    "_RecordFetchResult",
    [
        ("records", list["RecordData"]),
        ("cursors", list[str]),
        (
            "start_cursor",
            str | None,
        ),  # special because it encodes 'include me' (after is exclusive)
        ("total", int | None),
        ("engine", QueryEngine),
    ],
)


class RecordQuery:
    # TODO @Cleanup: merge RecordQuery into NodeQuery
    def __init__(
        self,
        database: "HasDatabase",
        filter: Expression | None = None,
        sort: list[Expression] = None,
        first: int = None,
        skip: int = None,
        engine: Optional[QueryEngine] = None,
        cache: bool = True,
    ):
        self._database = database
        self._filter = filter
        self._sort = sort
        self._first = first
        self._skip = skip
        self._engine = engine
        self._cache = cache
        self._cached_records: list[Record] | None = None
        self._cached_cursors: list[str] | None = None

    def __str__(self):
        args_strs = []
        for k in ("filter", "sort", "first", "skip"):
            v = getattr(self, f"_{k}")
            if k == "query":
                v = f"({v})" if v is not None else None
            if v is not None:
                args_strs.append(f"{k}={v}")
        return f"{self._database} {', '.join(args_strs)}"

    def __repr__(self):
        return f"<RecordQuery {self}>"

    @property
    def _combined_filter(self) -> Expression:
        """Combines the custom with the default filter for this database."""
        return Expression.and_if_set(
            self._filter,
            C(ConditionalOp.EQUALS, field_key="block_key", value=self._database.dynamic_key),
            ~C(ConditionalOp.EXISTS, field_key="deleted_at"),
        )

    def copy(self):
        """Clones the query (the properties are immutable)."""
        return RecordQuery(
            database=self._database,
            filter=self._filter,
            sort=self._sort,
            first=self._first,
            skip=self._skip,
            engine=self._engine,
            # cache is not copied on purpose as it shouldn't propagate
        )

    def _interp_self(self, scope: "ScopeNode", on_notice: "NoticeHandler") -> None:
        """
        Interprets the Bench parts of the query.
        Convenient for using RecordQuery with just deserialized parts,
         but of course RecordQuery isn't a node or struct or such.
        """
        if self._filter is not None:
            self._filter._interp_rec(scope, on_notice)
        if self._sort is not None:
            for sort in self._sort:
                sort._interp_rec(scope, on_notice)

    def _invalidate(self):
        self._cached_records = None
        self._cached_cursors = None

    @property
    def _required_engine(self) -> Optional[QueryEngine]:
        """The engine required to execute this query."""
        if self._filter is not None:
            ops = self._filter._collect_ops()
            if ConditionalOp.NEAR in ops:
                return QueryEngine.LOCAL_OPENSEARCH
        return None

    @property
    def _recommended_engine(self) -> QueryEngine:
        if self._filter is not None:
            ops = self._filter._collect_ops()
            if ops & ExpressionOps.COND_SCORED:
                return QueryEngine.LOCAL_OPENSEARCH
        return QueryEngine.LOCAL_POSTGRES

    def _require_engine(self, engine: QueryEngine) -> None:
        if self._required_engine and self._required_engine != engine:
            raise ValueError(f"cannot use {engine} with {self!r}")
        if self._engine and self._engine != engine:
            raise ValueError(f"cannot force {engine} with {self!r}")

    async def __aiter__(self):
        if self._cached_records is None:
            return iter(await self._fetch())
        return iter(self._cached_records)

    @_auto_async_to_sync
    async def tolist(self) -> list[Record]:
        if self._cached_records is None:
            return await self._fetch()
        return self._cached_records

    def __iter__(self):
        if self._cached_records is None:
            return iter(async_to_sync(self._fetch)())
        return iter(self._cached_records)

    def __len__(self):
        if self._cached_records is not None:
            return len(self._cached_records)
        return self.count()

    def using(self, engine: QueryEngine) -> "RecordQuery":
        """Forces use of a query engine."""
        copy = self.copy()
        copy._engine = engine
        return copy

    @_auto_async_to_sync
    async def get(self, filter: Expression = None, **kwargs) -> Record:
        """Returns the unique result matching the query (errors otherwise)."""
        filter = coerce_conditional(self._database, filter, kwargs)
        results = await self.filter(filter).tolist()
        if len(results) == 1:
            return results[0]
        else:
            raise ValueError(
                f"expected 1 result from {self!r} (filter={filter!r}), got {len(results)}: {results!r}"
            )

    def filter(self, filter: Expression = None, **kwargs) -> "RecordQuery":
        """Adds a filter clause to the query."""
        filter = coerce_conditional(self._database, filter, kwargs)
        copy = self.copy()
        copy._filter = filter & self._filter if self._filter is not None else filter
        return copy

    def sort(
        self, sort: list[Expression | str] | str | Expression = None, *args: str
    ) -> "RecordQuery":
        """Sorts the query results by the given sort criteria."""
        copy = self.copy()
        sort = coerce_sort(self._database, sort, args)
        copy._sort = sort
        return copy

    def first(self, count: int) -> "RecordQuery":
        """Returns the first N results."""
        copy = self.copy()
        copy._first = count
        return copy

    def skip(self, count: int) -> "RecordQuery":
        """Skips the first N results."""
        copy = self.copy()
        copy._skip = count
        return copy

    def __getitem__(self, item: slice | int) -> typing.Union["RecordQuery", Record]:
        if isinstance(item, slice):
            if item.stop is None:
                return self.skip(item.start or 0)
            elif item.start is not None:
                return self.skip(item.start).first(item.stop - item.start)
            else:
                return self.first(item.stop)
        elif isinstance(item, int):
            if self._cached_records is None:
                records = async_to_sync(self._fetch)()
            else:
                records = self._cached_records
            if item < 0:
                item += len(records)
            if item >= len(records):
                raise IndexError(f"index {item} out of range for {self!r} (got {len(self)})")
            return records[item]
        else:
            raise TypeError(f"expected slice or index into {self!r}, got {type(item)}: {item}")

    async def _fetch(self, *, _no_flush: bool = False) -> list[Record]:
        """Fetches the result set for this query."""
        from bench.proto import wiring

        session = self._database.session
        if not _no_flush and session._local_edits:
            await session.flush_local()  # first flush any local edits
        fetched = await self._do_fetch(pg_cursor=await self._database._get_pg_cursor())
        records: list[Record] = []
        for record_data in fetched.records:
            record: Record = wiring.unpack_node(record_data, self._database, session)
            record._track_self(session)
            records.append(record)

        if self._cache:
            self._cached_records = records
        logger.debug("record.query.done", query=self, results=len(records))
        return records

    async def _do_fetch(
        self, pg_cursor: psycopg.AsyncCursor | None, count: bool = False, after: str = None
    ) -> _RecordFetchResult:
        """Actually fetches the raw record results from some engine."""
        from bench.os.engine import os_search
        from bench.sql.engine import compile_pg_conditional, pg_count, pg_select_records_data

        where = self._combined_filter
        first = self._first or LOCAL_RECORD_CACHE_LIMIT
        target_engine = self._engine or self._recommended_engine or QueryEngine.LOCAL_POSTGRES

        logger.debug("record.query", query=self, where=where, limit=first, engine=target_engine)
        if target_engine == QueryEngine.LOCAL_OPENSEARCH:
            os_results = await os_search(
                os_name=self._database.package.os_name,
                metatype=NodeType.RECORD,
                filter=where,
                limit=first,
                skip=self._skip,
                after=after,
                count=count,
                sort=self._sort,
            )
            return _RecordFetchResult(
                os_results.as_records(),
                os_results.cursors,
                os_results.start_cursor,
                os_results.total,
                target_engine,
            )
        elif target_engine == QueryEngine.LOCAL_POSTGRES:
            assert pg_cursor is not None, f"missing pg_cursor for {self!r}"
            records, cursors, start_cursor = await pg_select_records_data(
                cur=pg_cursor,
                database=self._database,
                where=where,
                sort=self._sort,
                after=after,
                first=first,
                skip=self._skip,
            )
            if count:
                count = await pg_count(
                    cur=pg_cursor,
                    table=self._database._table,
                    where=compile_pg_conditional(self._database, where),
                )
            else:
                count = None
            return _RecordFetchResult(records, cursors, start_cursor, count, target_engine)
        else:
            raise ValueError(f"unexpected query engine {target_engine}")

    @_auto_async_to_sync
    async def count(self, filter: Expression = None, *, _no_flush: bool = False, **kwargs) -> int:
        """Returns the number of results. May refine the query."""
        from bench.sql.engine import compile_pg_conditional, pg_count

        self._require_engine(QueryEngine.LOCAL_POSTGRES)
        assert not self._first and not self._skip and not self._sort, "cannot count with limits"
        filter = coerce_conditional(self._database, filter, kwargs)

        if not _no_flush and self._database.session._local_edits:
            await self._database.session.flush_local()

        where = compile_pg_conditional(self._database, filter & self._combined_filter)
        return await pg_count(
            cur=await self._database._get_pg_cursor(), table=self._database._table, where=where
        )

    @_auto_async_to_sync
    async def exists(self, filter: Expression = None, *, _no_flush: bool = False, **kwargs) -> bool:
        """Whether any results exist. May refine the query."""
        from bench.sql.engine import compile_pg_conditional, pg_exists

        filter = coerce_conditional(self._database, filter, kwargs, return_none_if_empty=True)

        self._require_engine(QueryEngine.LOCAL_POSTGRES)
        assert not self._first and not self._skip and not self._sort, "cannot exists with limits"
        if filter is None and self._cached_records is not None:
            return bool(self._cached_records)

        if self._database.session._local_edits:
            await self._database.session.flush_local()

        if filter is None:
            filter = self._combined_filter
        else:
            filter = filter & self._combined_filter
        where = compile_pg_conditional(self._database, filter)
        return await pg_exists(
            cur=await self._database._get_pg_cursor(), table=self._database._table, where=where
        )

    @_auto_async_to_sync
    async def update(self, **value) -> int:
        """Updates all results with the given values."""
        from bench.language.value import check_type, pack_value
        from bench.sql.engine import compile_pg_conditional, pg_update_static, pg_wrap_record_value

        self._require_engine(QueryEngine.LOCAL_POSTGRES)
        if not value:
            raise ValueError(f"no values given to update {self!r}")

        session = self._database.session
        if session._local_edits:
            await session.flush_local()

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
        value["last_edited_at"] = now
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

    @_auto_async_to_sync
    async def delete(self) -> int:
        """Deletes all results."""
        from bench.sql.engine import compile_pg_conditional, pg_delete

        self._require_engine(QueryEngine.LOCAL_POSTGRES)
        assert not self._engine, "cannot delete with forced query engine"
        session = self._database.session
        if session._local_edits:
            await session.flush_local()
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


class RecordList(NodeListBase[Record], RecordQuery):
    """A NodeList for remote records."""

    def __init__(self, parent: "ScopeNode", property: Property):
        NodeListBase[Record].__init__(self, parent, property)
        RecordQuery.__init__(self, parent, cache=False)  # don't cache the root list

    def __str__(self):
        return f"from {self._parent._table.name}"

    def append(self, node: Record, _create: bool = True, _trigger: _NC = _NC.Full):
        assert isinstance(node, Record), f"cannot append {node!r} to {self!r}"
        node.parent = self._parent
        if node.id is None and self._parent.attached:
            node._assign_id(self._parent.package.id)
        # 'create' node
        if _create and not self._parent.attached:
            raise RuntimeError(f"cannot create {node!r} in detached {self!r}")
        # update affected nodes
        if _trigger:
            node._attached_self()
            if self._parent._session:
                node._track_self(self._parent._session)
        # 'create' node in session
        if _create and self._parent._session and self._parent.attached:
            self._parent.session.create(node)

    def extend(self, *nodes: Record, _create: bool = True, _trigger: _NC = _NC.Full) -> None:
        for record in nodes:
            self.append(record, _create=False, _trigger=_NC.Ignore)
        # 'create' nodes
        if _create and not self._parent.attached:
            raise RuntimeError(f"cannot create {nodes!r} in detached {self!r}")
        # update affected nodes
        if _trigger:
            for n in nodes:
                n._attached_self()
            if self._parent._session:
                for n in nodes:
                    n._track_self(self._parent._session)
        # 'create' nodes in session
        if _create and self._parent._session and self._parent.attached:
            self._parent.session.create(*nodes)

    def remove(self, node: Record, _delete: bool = True, _trigger: _NC = _NC.Full) -> None:
        if _delete:
            if self._parent._session:
                self._parent._session.delete(self, node)
            if not self._parent.attached:
                self._parent._root_tree.remove(node)
        node.parent = None

    @_auto_async_to_sync
    async def clear(self, _delete: bool = True, _trigger: _NC = _NC.Full) -> None:
        if _delete:
            await RecordQuery.filter(self).delete()

    def __contains__(self, obj: object) -> bool:
        return False  # lookup by id?

    def get(self, conditional: Expression = None, **kwargs) -> Record:
        return RecordQuery.get(self, conditional, **kwargs)

    #
    # Extra methods for record queries/expressions
    #

    def __len__(self):
        return RecordQuery.__len__(self)

    def __iter__(self):
        return RecordQuery.__iter__(self)

    def __aiter__(self):
        return RecordQuery.__aiter__(self)


@node_component
class HasDatabase(Node):
    dynamic_key: str | None = struct_internal(UNSET, default=None)
    queries: NodeList["Query"] = node_children(
        NodeType.QUERY, NRel.NAMED | NRel.SCOPED | NRel.ORDERED
    )
    records: RecordList[Record] = node_children(
        NodeType.RECORD, NRel.STORED_CUSTOM, custom_list=RecordList
    )
    _table: Optional[Table] = struct_runtime(default=None)

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

    async def _get_pg_cursor(self) -> psycopg.AsyncCursor:
        return await self.session.pg_cursor_to_local(package=self.package)

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
