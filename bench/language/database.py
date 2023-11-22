import enum
import typing
from typing import Optional
from uuid import UUID

import psycopg
import structlog
from psycopg import sql
from psycopg.types.json import Jsonb

from bench.language.builtin import _auto_async_to_sync
from bench.language.const import (
    MNT,
    ConditionalOp,
    QueryEngine,
    SessionAccessLevel,
    new_dynamic_node_key,
)
from bench.language.expression import (
    TYPE_DISCRIMINATOR_KEY,
    C,
    Conditional,
    Sort,
    coerce_conditional,
)
from bench.language.module import (
    _NC,
    NS,
    Node,
    NodeList,
    NodeListBase,
    NRel,
    Property,
    ScopeNode,
    _Passthrough,
    bruntime,
    nchildren,
    node,
    node_component,
    nparent,
)
from bench.language.validation import ValidationHandler
from bench.language.value import HasValue
from bench.search.core import DocumentType
from bench.sql.core import EPHEMERAL_RECORD_TABLE, Table
from bench.utils.dt import utcnow_with_tz
from bench.utils.func import describe_type
from bench.utils.utils import flatten

if typing.TYPE_CHECKING:
    from bench.language import Field, Session, Statement, View
    from bench.language.wire import RecordData

logger = structlog.get_logger(__name__)

LOCAL_RECORD_CACHE_LIMIT = 2048
RECORD_UNSPECIFIED_BATCH_SIZE = 500


@node(mnt=MNT.RECORD, passthrough=(("value", _Passthrough.Full),))
class Record(HasValue, Node):
    parent: "Statement" = nparent(MNT.STATEMENT)

    @staticmethod
    def new(
        *args,
        for_parent: "Statement" = None,
        _status: NS = None,
        _id: UUID = None,
        _ck: UUID = None,
        **kwargs,
    ) -> "Record":
        from bench.language.packer import check_type

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
            for field, arg in zip(for_parent.resolved_fields, args):
                value[field.name] = arg
            check_type(value, for_parent)
        return Record(value=value, _status=_status, **id_kwargs)

    def __str__(self):
        self_str = f"{self.id} {describe_type(self.value) or '<empty>'}"
        if self.parent is None:
            return f"<detached>:{self_str}"
        else:
            return f"{self.parent.path}:{self_str}"

    def __repr__(self):
        return f"<Record {self}>"

    @property
    def parent_id(self):
        return self.parent.id

    @property
    def _type_of_value(self) -> Optional["Statement"]:
        return self.parent

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
        ("total", int | None),
        ("engine", QueryEngine),
    ],
)


class RecordQuery:
    def __init__(
        self,
        database: "HasDatabase",
        filter: Conditional | None = None,
        sort: list[Sort] = None,
        include: list["Field"] = None,
        select: list["Field"] = None,
        distinct: list["Field"] = None,
        first: int = None,
        skip: int = None,
        engine: Optional[QueryEngine] = None,
        cache: bool = True,
    ):
        self._database = database
        self._filter = filter
        self._sort = sort
        self._include = include
        self._select = select
        self._distinct = distinct
        self._first = first
        self._skip = skip
        self._engine = engine
        self._cache = cache
        self._cached_records: list[Record] | None = None
        self._cached_cursors: list[str] | None = None

    def __str__(self):
        args_strs = []
        for k in ("filter", "sort", "include", "select", "distinct", "first", "skip"):
            v = getattr(self, f"_{k}")
            if k == "query":
                v = f"({v})" if v is not None else None
            if v is not None:
                args_strs.append(f"{k}={v}")
        return f"{self._database} {', '.join(args_strs)}"

    def __repr__(self):
        return f"<RecordQuery {self}>"

    @property
    def _combined_filter(self) -> Conditional:
        """Combines the custom with the default filter"""
        return Conditional.and_if_set(
            self._filter,
            C(ConditionalOp.EQUALS, "statement_key", value=self._database.key),
            ~C(ConditionalOp.EXISTS, "deleted_at"),
        )

    def copy(self):
        """Clones the query (the properties are immutable)."""
        return RecordQuery(
            database=self._database,
            filter=self._filter,
            sort=self._sort,
            include=self._include,
            select=self._select,
            distinct=self._distinct,
            first=self._first,
            skip=self._skip,
            engine=self._engine,
            # cache is deliberately not copied
        )

    def _interp_self(self, scope: "ScopeNode", on_issue: "ValidationHandler") -> None:
        """
        Interprets the Bench parts of the query.
        Convenient for using RecordQuery with just deserialized parts,
         but of course RecordQuery isn't a node or struct or such.
        """
        if self._filter is not None:
            self._filter._interp_self(scope, on_issue)
        if self._sort is not None:
            for sort in self._sort:
                sort._interp_self(scope, on_issue)

    def _invalidate(self):
        self._cached_records = None
        self._cached_cursors = None

    async def _do_fetch(
        self, pg_cursor: psycopg.AsyncCursor | None, count: bool = False, after: str = None
    ) -> _RecordFetchResult:
        """Actually fetches the raw record results from some engine."""
        from bench.search.engine import os_search
        from bench.sql.engine import compile_pg_conditional, pg_count, pg_select_records

        where = self._combined_filter
        first = self._first or LOCAL_RECORD_CACHE_LIMIT
        # prefer sql engine if possible, except for scored queries
        required_engine = None
        if where.is_scored:
            required_engine = QueryEngine.OPENSEARCH
        if required_engine and self._engine and self._engine != required_engine:
            raise ValueError(f"cannot use {self._engine} with {self!r}")
        target_engine = self._engine or required_engine or QueryEngine.POSTGRES

        logger.debug("record.query", query=self, where=where, limit=first, engine=target_engine)
        if target_engine == QueryEngine.OPENSEARCH:
            os_results = await os_search(
                os_name=self._database.module.os_name,
                type=DocumentType.RECORD,
                filter=where,
                limit=first,
                skip=self._skip,
                after=after,
                count=count,
                sort=self._sort,
            )
            return _RecordFetchResult(
                os_results.as_records(), os_results.cursors, os_results.total, target_engine
            )
        elif target_engine == QueryEngine.POSTGRES:
            assert pg_cursor is not None, f"missing pg_cursor for {self!r}"
            records, cursors = await pg_select_records(
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
            return _RecordFetchResult(records, cursors, count, target_engine)
        else:
            raise ValueError(f"unexpected query engine {target_engine}")

    async def _fetch(self, session: "Session") -> list[Record]:
        """Fetches the result set for this query."""
        from bench.language import wire

        if session._tracer._local_edits:
            await session.flush_local()  # first flush any local edits
        fetched = await self._do_fetch(pg_cursor=session.pg_cursor)
        records: list[Record] = []
        for record_data in fetched.records:
            record: Record = wire.unpack_node_flat(record_data, self._database, session)
            record._activate_self(session)
            records.append(record)

        if self._cache:
            self._cached_records = records
        logger.debug("record.query.done", query=self, results=len(records))
        return records

    async def __aiter__(self):
        if self._cached_records is None:
            return iter(await self._fetch(self._database.session))
        return iter(self._cached_records)

    @_auto_async_to_sync
    async def tolist(self) -> list[Record]:
        if self._cached_records is None:
            return await self._fetch(self._database.session)
        return self._cached_records

    def __iter__(self):
        if self._cached_records is None:
            return iter(self._database.session.async_to_sync(self._fetch)(self._database.session))
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
    async def get(self, query: Conditional = None, **kwargs) -> Record:
        """Returns the unique result matching the query (errors otherwise)."""
        query = coerce_conditional(self._database, query, kwargs)
        results = await self.filter(query).tolist()
        if len(results) == 1:
            return results[0]
        else:
            raise ValueError(f"expected 1 result from {self!r}, got {len(results)}: {results}")

    def filter(self, query: Conditional = None, **kwargs) -> "RecordQuery":
        """Adds a filter clause to the query."""
        query = coerce_conditional(self._database, query, kwargs)
        copy = self.copy()
        copy._filter = query & self._filter if self._filter else query
        return copy

    def sort(self, sort: list[Sort] | Sort) -> "RecordQuery":
        """Sorts the query results by the given sort criteria."""
        copy = self.copy()
        if not isinstance(sort, list):
            sort = [sort]
        copy._sort = sort
        return copy

    def select(self, *fields: "Field") -> "RecordQuery":
        """Selects only the given fields in the results."""
        raise NotImplementedError("not yet supported")

    def include(self, *fields: "Field") -> "RecordQuery":
        """Includes the given related fields in the results."""
        raise NotImplementedError("not yet supported")

    def distinct(self, *fields: "Field") -> "RecordQuery":
        """Returns only distinct results."""
        raise NotImplementedError("not yet supported")

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
                self._database.session.async_to_sync(self._fetch)(self._database.session)
            if item < 0:
                item += len(self._cached_records)
            if item >= len(self._cached_records):
                raise IndexError(f"index {item} out of range for {self!r} (got {len(self)})")
            return self._cached_records[item]
        else:
            raise TypeError(f"expected slice or index into {self!r}, got {type(item)}: {item}")

    @_auto_async_to_sync
    async def count(self) -> int:
        """Returns the number of results."""
        assert not self._first and not self._skip and not self._sort, "cannot count with limits"
        if self._cached_records is not None:
            return len(self._cached_records)
        from bench.sql.engine import compile_pg_conditional, pg_count

        if self._database.session._tracer._local_edits:
            await self._database.session.flush_local()

        where = compile_pg_conditional(self._database, self._combined_filter)
        return await pg_count(
            cur=self._database.session.pg_cursor, table=self._database._table, where=where
        )

    @_auto_async_to_sync
    async def update(self, **values) -> int:
        """Updates all results with the given values."""
        from bench.language.packer import check_type, pack_value
        from bench.sql.engine import compile_pg_conditional, pg_update_static

        session = self._database.session
        session.check_access(SessionAccessLevel.Update)
        if session._tracer._local_edits:
            await session.flush_local()

        # 'serialize' values (probably need a better way here to retain some native types?)
        check_type(values, self._database)
        values = pack_value(
            values, self._database, ignore_outer=True, map_k=lambda f: (f.py_ident, f._typed_key)
        )
        if TYPE_DISCRIMINATOR_KEY in values:  # not stored in database (implicit in statement_key)
            del values[TYPE_DISCRIMINATOR_KEY]
        # update values alongside :LocalRecordCru
        if self._database.ephemeral:  # set within generic 'value' JSONB column
            values = {"value": sql.SQL("value || {}").format(sql.Literal(Jsonb(values)))}
        now = utcnow_with_tz()
        values["revision"] = sql.SQL("revision + 1")
        values["updated_at"] = now
        values["last_edited_at"] = now
        updated_rows = await pg_update_static(
            cur=session.pg_cursor,
            table=self._database._table,
            where=compile_pg_conditional(self._database, self._combined_filter),
            values=values,
            returning=[self._database._table.columns_by_name["id"]],
        )
        updated_records_ids = {r["id"] for r in updated_rows}
        session._tracer._records_changed(self._database, updated_records_ids)  # mark for OS sync
        return len(updated_rows)

    @_auto_async_to_sync
    async def delete(self) -> int:
        """Deletes all results."""
        from bench.sql.engine import compile_pg_conditional, pg_delete

        session = self._database.session
        session.check_access(SessionAccessLevel.Delete)
        if session._tracer._local_edits:
            await session.flush_local()
        where = Conditional.and_if_set(
            self._filter, C(ConditionalOp.EQUALS, "statement_key", value=self._database.key)
        )
        deleted_rows = await pg_delete(
            cur=session.pg_cursor,
            table=self._database._table,
            where=compile_pg_conditional(self._database, where),
            returning=[self._database._table.columns_by_name["id"]],
        )
        deleted_records_ids = {r["id"] for r in deleted_rows}
        session._tracer._records_changed(self._database, deleted_records_ids)  # mark for OS sync
        return len(deleted_rows)


class RelationType(enum.StrEnum):
    OneToOne = "OneToOne"
    OneToMany = "OneToMany"
    ManyToMany = "ManyToMany"
    ManyToOne = "ManyToOne"


class RecordRelation:
    """
    A related (sub-)value in a record.
    """

    def __init__(
        self, parent: "Record", field: "Field", type: RelationType, reverse_field: "Field" = None
    ):
        self._parent = parent
        self._field = field
        self._type = type
        self._reverse_field: Optional["Field"] = reverse_field
        self._loaded = False

    def clear(self) -> None:
        raise NotImplementedError


class RecordRelationToOne(RecordRelation):
    """
    The one side of a one-to-one or many-to-one relation.
    """

    def __init__(
        self, parent: "Record", field: "Field", type: RelationType, reverse_field: "Field" = None
    ):
        super().__init__(parent, field, type, reverse_field)
        self.value: Optional[Record] = None

    def set(self, record: Optional["Record"]) -> None:
        raise NotImplementedError


class RecordRelationToMany(RecordRelation):
    """
    The many side of a one-to-many or many-to-many relation.
    """

    def __init__(
        self, parent: "Record", field: "Field", type: RelationType, reverse_field: "Field" = None
    ):
        super().__init__(parent, field, type, reverse_field)
        self.value: list[Record] = []

    def filter(self, query: Conditional) -> "RecordQuery":
        raise NotImplementedError

    def create(self, **kwargs) -> "Record":
        raise NotImplementedError

    def append(self, record: "Record") -> None:
        raise NotImplementedError

    def extend(self, *records: "Record") -> None:
        raise NotImplementedError

    def remove(self, record: "Record") -> None:
        raise NotImplementedError

    def count(self) -> int:
        raise NotImplementedError

    def set(self, records: list["Record"]) -> None:
        raise NotImplementedError

    def __len__(self) -> int:
        return self.count()


class RecordList(NodeListBase[Record], RecordQuery):
    """A NodeList for remote records."""

    def __init__(self, parent: "ScopeNode", property: Property):
        NodeListBase[Record].__init__(self, parent, property)
        RecordQuery.__init__(self, parent, cache=False)  # don't cache the root list

    def __str__(self):
        return f"from {self._parent._table.name}"

    def _update(self, scope: "ScopeNode"):
        pass  # nothing to do, not part of regular tree

    def append(self, node: Record, _create: bool = True, _trigger: _NC = _NC.Tach):
        assert isinstance(node, Record), f"cannot append {node!r} to {self!r}"
        node.parent = self._parent
        if node.id is None and self._parent.attached:
            node._assign_id(self._parent.module.id)
        # 'create' node
        if _create:
            if self._parent._session:
                self._parent.session._tracer.node_create_preflight(node)
            if not self._parent.attached:
                # detached record nodes are temporarily hoisted into inline tree :TempRecordTree
                self._parent._local_root_tree.add(node)
        # update affected nodes
        if _trigger:  # manually trigger ChangeEffect
            node._attached_inner()
        # 'create' node in session
        if _create and self._parent._session and self._parent.attached:
            self._parent.session._tracer.node_create(node)

    def extend(self, *nodes: Record, _create: bool = True, _trigger: _NC = _NC.Tach) -> None:
        nodes = flatten(nodes)
        for record in nodes:
            self.append(record, _create=False, _trigger=_NC.Ignore)
        # 'create' nodes
        if _create:
            if self._parent._session:
                self._parent.session._tracer.node_create_preflight(*nodes)
            if not self._parent.attached:
                self._parent._local_root_tree.add_many(nodes)  # :TempRecordTree
        # update affected nodes
        if _trigger:
            for n in nodes:
                n._attached_inner()
        # 'create' nodes in session
        if _create and self._parent._session and self._parent.attached:
            self._parent.session._tracer.node_create(*nodes)

    def remove(self, node: Record, _delete: bool = True, _trigger: _NC = _NC.Tach) -> None:
        if _delete:
            if self._parent._session:
                self._parent._session._tracer.node_delete(self, node)
            if not self._parent.attached:
                self._parent._local_root_tree.remove(node)
        node.parent = None

    @_auto_async_to_sync
    async def clear(self, _delete: bool = True, _trigger: _NC = _NC.Tach) -> None:
        if _delete:
            await RecordQuery.filter(self).delete()

    def __contains__(self, obj: object) -> bool:
        return False  # lookup by id?

    def get(self, conditional: Conditional = None, **kwargs) -> Record:
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

    def __getitem__(self, item: slice):
        return RecordQuery.__getitem__(self, item)


@node_component
class HasDatabase(Node):
    views: NodeList["View"] = nchildren(MNT.VIEW, NRel.Named | NRel.Ordered)
    records: NodeList[Record] = nchildren(MNT.RECORD, NRel.Remote, custom_list=RecordList)
    _table: Optional[Table] = bruntime(default=None)

    def _init_inner(self):
        # this runs before HasFields because of the ordering in
        #  (which is necessary because HasFields also sets key)
        if self.key is None:
            self.key = self._derive_key()

    def _clear_inner(self, scope: Optional["ScopeNode"]) -> None:
        self._table = None

    def _interp_inner(self, scope: "ScopeNode", on_issue: "ValidationHandler") -> None:
        from bench.sql.engine import map_to_pg_table

        self._table = map_to_pg_table(self) if not self.ephemeral else EPHEMERAL_RECORD_TABLE

    @property
    def ephemeral(self) -> bool:
        # basically whether this should be 1:1 a real database table or just virtual
        return True  # nocheckin: 10. make database non-ephemeral by default

    @staticmethod
    def _derive_key(instance: "HasDatabase") -> str | None:
        if instance.versioned:
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
