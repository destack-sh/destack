import enum
import typing
from typing import Optional
from uuid import UUID

import structlog

from bench.language.builtin import _auto_async_to_sync
from bench.language.const import MNT, QueryEngine, SessionAccessLevel, new_dynamic_node_key
from bench.language.expression import C, Conditional, Sort, coerce_conditional
from bench.language.module import (
    _NC,
    NS,
    Node,
    NodeList,
    NodeListBase,
    NRel,
    Property,
    ScopeNode,
    _ChangeEffect,
    _Passthrough,
    bruntime,
    nchildren,
    node,
    node_component,
    nparent,
)
from bench.language.value import HasValue
from bench.search.core import DocumentType
from bench.sql.core import EPHEMERAL_RECORD_TABLE, Table
from bench.utils.func import describe_type
from bench.utils.utils import flatten

if typing.TYPE_CHECKING:
    from bench.language import ConditionalOp, Field, Session, Statement, View

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


class RecordBaseQuery:
    def __init__(
        self,
        database: "HasDatabase",
        query: Conditional | None = None,
        sort: list[Sort] = None,
        include: list["Field"] = None,
        select: list["Field"] = None,
        distinct: list["Field"] = None,
        first: int = None,
        skip: int = None,
        engine: Optional[QueryEngine] = None,
    ):
        self._database = database
        self._query = query
        self._sort = sort
        self._include = include
        self._select = select
        self._distinct = distinct
        self._first = first
        self._skip = skip
        self._engine = engine
        self._result_cache: list[Record] | None = None

    def __str__(self):
        args_strs = []
        for k in ("query", "sort", "include", "select", "distinct", "first", "skip"):
            v = getattr(self, f"_{k}")
            if k == "query":
                v = f"({v})" if v is not None else None
            if v is not None:
                args_strs.append(f"{k}={v}")
        return f"{self._database} {', '.join(args_strs)}"

    def __repr__(self):
        return f"<RecordQuery {self}>"

    def copy(self):
        """Clones the query (the properties are immutable)."""
        return RecordBaseQuery(
            database=self._database,
            query=self._query,
            sort=self._sort,
            include=self._include,
            select=self._select,
            distinct=self._distinct,
            first=self._first,
            skip=self._skip,
            engine=self._engine,
        )

    def using(self, engine: QueryEngine) -> "RecordBaseQuery":
        """Forces use of a query engine."""
        copy = self.copy()
        copy._engine = engine
        return copy

    def _invalidate(self):
        self._result_cache = None

    async def _fetch(self, session: "Session") -> list[Record]:
        """Fetches the result set for this query."""
        from bench.language import wire
        from bench.search.engine import compile_os_search, os_search
        from bench.sql.engine import pg_select_records

        # nocheckin: 8. reroute record query through local DB if possible
        # force flush and index if there are any pending database edits
        #  (or previous edits that were already flushed but didn't refresh the index)
        if session._editor.edits or session._past_commits:
            # nocheckin: turn this into a local PG flush only (only schema apply, not commit)
            await session.commit()

        where = Conditional.and_if_set(
            self._query,
            C(ConditionalOp.EQUALS, "statement_key", value=self._database.key)
            & ~C(ConditionalOp.EXISTS, "deleted_at"),
        )
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
            search = compile_os_search(
                type=DocumentType.RECORD, query=where, limit=first, count=False, sort=self._sort
            )
            os_results = await os_search(self._database.module.os_name, search)
            records_data = os_results.as_records()
        elif target_engine == QueryEngine.POSTGRES:
            records_data = await pg_select_records(
                cur=session.pg_cursor,
                database=self._database,
                where=where,
                sort=self._sort,
                first=first,
                skip=self._skip,
            )
        else:
            raise ValueError(f"unexpected query engine {target_engine}")

        # turn into records
        records: list[Record] = []
        for record_data in records_data:
            record: Record = wire.unpack_node_flat(record_data, self._database, session)
            record._activate_self(session)
            records.append(record)

        self._result_cache = records
        logger.debug("record.query.done", query=self, results=len(records))
        return records

    async def __aiter__(self):
        if self._result_cache is None:
            await self._fetch(self._database.session)
        return iter(self._result_cache)

    @_auto_async_to_sync
    async def tolist(self) -> list[Record]:
        if self._result_cache is None:
            await self._fetch(self._database.session)
        return self._result_cache

    def __iter__(self):
        if self._result_cache is None:
            self._database.session.async_to_sync(self._fetch)(self._database.session)
        return iter(self._result_cache)

    def __len__(self):
        if self._result_cache is not None:
            return len(self._result_cache)
        return self.count()

    @_auto_async_to_sync
    async def get(self, query: Conditional = None, **kwargs) -> Record:
        """Returns the unique result matching the query (errors otherwise)."""
        query = coerce_conditional(self._database, query, kwargs)
        results = await self.filter(query).tolist()
        if len(results) == 1:
            return results[0]
        else:
            raise ValueError(f"expected 1 result from {self!r}, got {len(results)}: {results}")

    def filter(self, query: Conditional = None, **kwargs) -> "RecordBaseQuery":
        """Adds a filter clause to the query."""
        query = coerce_conditional(self._database, query, kwargs)
        copy = self.copy()
        copy._query = query & self._query if self._query else query
        return copy

    def sort(self, sort: list[Sort] | Sort) -> "RecordBaseQuery":
        """Sorts the query results by the given sort criteria."""
        copy = self.copy()
        if not isinstance(sort, list):
            sort = [sort]
        copy._sort = sort
        return copy

    def select(self, *fields: "Field") -> "RecordBaseQuery":
        """Selects only the given fields in the results."""
        raise NotImplementedError("not yet supported")

    def include(self, *fields: "Field") -> "RecordBaseQuery":
        """Includes the given related fields in the results."""
        raise NotImplementedError("not yet supported")

    def distinct(self, *fields: "Field") -> "RecordBaseQuery":
        """Returns only distinct results."""
        raise NotImplementedError("not yet supported")

    def first(self, count: int) -> "RecordBaseQuery":
        """Returns the first N results."""
        copy = self.copy()
        copy._first = count
        return copy

    def skip(self, count: int) -> "RecordBaseQuery":
        """Skips the first N results."""
        copy = self.copy()
        copy._skip = count
        return copy

    def __getitem__(self, item: slice | int) -> typing.Union["RecordBaseQuery", Record]:
        if isinstance(item, slice):
            if item.stop is None:
                return self.skip(item.start or 0)
            elif item.start is not None:
                return self.skip(item.start).first(item.stop - item.start)
            else:
                return self.first(item.stop)
        elif isinstance(item, int):
            if self._result_cache is None:
                self._database.session.async_to_sync(self._fetch)(self._database.session)
            if item < 0:
                item += len(self._result_cache)
            if item >= len(self._result_cache):
                raise IndexError(f"index {item} out of range for {self!r} (got {len(self)})")
            return self._result_cache[item]
        else:
            raise TypeError(f"expected slice or index into {self!r}, got {type(item)}: {item}")

    @_auto_async_to_sync
    async def count(self) -> int:
        """Returns the number of results."""
        assert not self._first and not self._skip and not self._sort, "cannot count with limits"
        if self._result_cache is not None:
            return len(self._result_cache)
        raise NotImplementedError("nocheckin")

    @_auto_async_to_sync
    async def update(self, **kwargs) -> int:
        """Updates all results with the given values."""

        self._database.session.check_access(SessionAccessLevel.Update)
        # return the full updated values to update in OS
        raise NotImplementedError("not yet supported")

    @_auto_async_to_sync
    async def delete(self) -> int:
        """Deletes all results."""
        self._database.session.check_access(SessionAccessLevel.Delete)
        # return the ids to delete in OS
        raise NotImplementedError("not yet supported")

    def group_by(self, *fields: "Field") -> "RecordBaseQuery":
        """Groups the results by the given fields."""
        raise NotImplementedError


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

    def filter(self, query: Conditional) -> "RecordBaseQuery":
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


class RecordList(NodeListBase[Record], RecordBaseQuery):
    """A NodeList for remote records."""

    def __init__(self, parent: "ScopeNode", property: Property):
        NodeListBase[Record].__init__(self, parent, property)
        RecordBaseQuery.__init__(self, parent)

    def __str__(self):
        return f"from {self._parent._table.name}"

    def _update(self, scope: "ScopeNode"):
        pass  # nothing to do, not part of regular tree

    def append(self, node: Record, _create: bool = True, _trigger: _NC = _NC.Full) -> None:
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
        if _trigger:
            _ChangeEffect._collect(None, self._parent, [node], _trigger)._effect(_trigger)
        # 'create' node in session
        if _create and self._parent._session and self._parent.attached:
            self._parent.session._tracer.node_create(node)

    def extend(self, *nodes: Record, _create: bool = True, _trigger: _NC = _NC.Full) -> None:
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
            _ChangeEffect._collect(None, self._parent, nodes, _trigger)._effect(_trigger)
        # 'create' nodes in session
        if _create and self._parent._session and self._parent.attached:
            self._parent.session._tracer.node_create(*nodes)

    def remove(self, node: Record, _delete: bool = True, _trigger: _NC = _NC.Full) -> None:
        if _delete:
            if self._parent._session:
                self._parent._session._tracer.node_delete(self, node)
            if not self._parent.attached:
                self._parent._local_root_tree.remove(node)
        node.parent = None

    def clear(self, _delete: bool = True, _trigger: _NC = _NC.Full) -> None:
        if _delete:
            if self._parent._session:
                self._parent._session._tracer.node_truncate(self._parent, MNT.RECORD)
            if not self._parent.attached:
                self._parent._local_root_tree.truncate(self._parent, MNT.RECORD)

    def __contains__(self, obj: object) -> bool:
        return False  # lookup by id?

    def get(self, conditional: Conditional = None, **kwargs) -> Record:
        return RecordBaseQuery.get(self, conditional, **kwargs)

    #
    # Extra methods for record queries/expressions
    #

    def __len__(self):
        return RecordBaseQuery.__len__(self)

    def __iter__(self):
        return RecordBaseQuery.__iter__(self)

    def __aiter__(self):
        return RecordBaseQuery.__aiter__(self)

    def __getitem__(self, item: slice):
        return RecordBaseQuery.__getitem__(self, item)


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

    def _interp_inner(self, scope: "ScopeNode") -> None:
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
