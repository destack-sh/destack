import inspect
import typing
from typing import Any, Optional
from uuid import UUID

import structlog

from bench.language.const import MNT, DatabaseViewLayout, new_dynamic_node_key
from bench.language.field import Field
from bench.language.module import (
    Module,
    ModuleNode,
    ScopeNode,
    nchildren,
    ninternal,
    node,
    node_component,
    nparent,
    nproperty,
)
from bench.language.query import Query, Sort
from bench.language.search import ElementT, Search
from bench.language.value import HasValue
from bench.utils.func import describe_type, did_you_mean_str
from bench.utils.proxy import unproxy_value
from bench.utils.utils import DotList

if typing.TYPE_CHECKING:
    from bench.language import Statement
    from bench.language.wire import RecordData

logger = structlog.get_logger(__name__)


@node(mnt=MNT.Record)
class Record(HasValue, ModuleNode):
    parent: "Statement" = nparent(MNT.Statement)

    @staticmethod
    def _coerce_from(value: Any = None, *args, **kwargs) -> "Record":
        if isinstance(value, dict):
            value = unproxy_value(value)
        return Record(value=value, *args, **kwargs)

    def __str__(self):
        return f"{self.parent.path}:{self.id} {describe_type(self.value)}"

    def __repr__(self):
        return f"<Record {self}>"

    @property
    def parent_id(self):
        return self.parent.id

    @property
    def _type_of_value(self):
        return self.parent

    @property
    def keys(self):
        return self.value.keys

    def __contains__(self, item: str):
        return item in self.value

    def __getitem__(self, item: str):
        val = self.value.get(item)
        if val is not None or item in self.parent.fields:
            return val
        elif not isinstance(item, str):
            raise TypeError(f"cannot index {repr(self)} with {type(item)}")
        else:
            candidates = {f.py_ident: f for f in self.parent.fields}
            did_you_mean = did_you_mean_str(candidates, item)
            raise KeyError(
                f"{self} has no field '{item}' ({did_you_mean}, available: {list(self.parent.fields)})"
            )

    def __setitem__(self, key, value):
        self.value[key] = value

    def __getattr__(self, item):
        if item in self._PROPERTIES:
            return super().__getattr__(item)
        else:
            return self[item]

    def __setattr__(self, key, value):
        if key in self._PROPERTIES:
            super().__setattr__(key, value)
        else:
            self.value[key] = value


@node(mnt=MNT.DatabaseView)
class DatabaseView(ScopeNode):
    parent: "Statement" = nparent(MNT.Statement)
    name: str | None = nproperty(default=None)
    layout: DatabaseViewLayout = nproperty(default=DatabaseViewLayout.TABLE)
    query: Optional[Query] = nproperty(default=None)
    sort: Optional[list[Sort]] = nproperty(default=None)
    fields: Optional[list["DatabaseViewField"]] = nchildren(MNT.DatabaseViewField)

    def __str__(self):
        return f"{self.parent.path}:{self.name} ({self.layout})"

    def __repr__(self):
        return f"<DatabaseView {self}>"

    @property
    def path(self) -> str:
        return f"{self.parent.path}.{self.name}"


@node(mnt=MNT.DatabaseViewField)
class DatabaseViewField(ModuleNode):
    field: UUID | Field = nproperty()
    order_key: str | None = ninternal(default=None)


@node_component(dynamic=True)
class HasDatabase(ModuleNode, Search["RecordData", Record]):
    # note that HasDatabase doesn't feel like component like the others (HasCode, HasText, etc.)
    #  but it would also be weird to have it not be a component now.

    def _init(self):
        # nocheckin: ensure this takes effect if HasFields is also a component
        if self.key is None:
            if self.versioned:
                if self.id is not None:
                    self.key = new_dynamic_node_key(self.id)
                else:
                    self.key = None
            else:
                self.key = new_dynamic_node_key(self.ck)

    def clear(self):
        self.records.clear(self)

    def append(self, record: Record | dict = None, **value) -> Record:
        """Appends a record to the database."""
        if record is not None:
            if value:
                raise ValueError("cannot pass both record and data")
            if isinstance(record, dict):
                value = record
            elif isinstance(record, Record):
                value = record.value
            else:
                raise TypeError(f"cannot append {type(record)} to {self}")
        value = unproxy_value(value)  # remove source proxy if any
        record = Record(parent=self, value=value)
        self.session.tracer.database_append(self, record)
        return record

    def extend(self, records: typing.Iterable[Record | dict]) -> None:
        """Extends the database with the given records."""
        values = [  # remove source proxy if any
            unproxy_value(record.value) if isinstance(record, Record) else unproxy_value(record)
            for record in records
        ]
        records = [Record(parent=self, value=value) for value in values]
        self.session.tracer.node_create(self, records)

    def search(
        self, query: Optional[Query] = None, sort: list[Sort] = None, limit: int = None
    ) -> "RecordSearch":
        """Searches this database remotely."""
        return RecordSearch(self.module, [self], query, sort, limit)

    def __getitem__(self, item: slice):
        if isinstance(item, slice):
            return self.search(limit=item.stop)
        else:
            raise TypeError(f"index into {self} must be slice (not {type(item)})")

    def filter(self, query: Query) -> "RecordSearch":
        if not isinstance(query, Query):
            raise TypeError(f"cannot filter by {type(query)}")
        return self.search(query=query)

    def sort(self, sort: list[Sort] | Sort) -> "RecordSearch":
        if isinstance(sort, Sort):
            sort = [sort]
        return self.search(sort=sort)

    def limit(self, limit: int) -> "RecordSearch":
        return self.search(limit=limit)

    def __iter__(self):
        return iter(self.search())

    def __aiter__(self):
        return aiter(self.search())


MapFunction = typing.Callable[[Record], typing.Union[Record, dict]]
BatchMapFunction = typing.Callable[[list[Record]], list[typing.Union[Record, dict]]]
AmapFunction = typing.Callable[[Record], typing.Awaitable[typing.Union[Record, dict]]]
BatchAmapFunction = typing.Callable[
    [list[Record]], typing.Awaitable[list[typing.Union[Record, dict]]]
]


class RecordSearch(Search["RecordData", Record]):
    """A search over records (of a database)."""

    def __init__(
        self,
        module: Module,
        databases: list[HasDatabase],
        query: Query,
        sort: list[Sort],
        limit: Optional[int],
    ):
        super().__init__(module, query, sort, limit)
        if not isinstance(databases, list):
            raise TypeError(f"databases must be a list (not {type(databases)}): {databases}")
        self.module = module
        self.databases = databases

    def __str__(self):
        return f"{self.databases} {self._query or '<no query>'} {self._sort or '<no sort>'} limit={self._limit or '<no limit>'}"

    def __repr__(self):
        return f"<RecordSearch {self}>"

    async def _do_search(self, after: str = None, limit: Optional[int] = None, count: bool = False):
        from bench.msg import NMessage
        from bench.msg.core import request
        from bench.msg.messages import (
            NMessageType,
            RepSearchRecordsPayload,
            ReqSearchRecordsPayload,
        )

        await self.module.session._do_search_preflight(self)
        batch_limit = min(self.RESULT_BATCH_SIZE, limit or self._limit or self.RESULT_BATCH_SIZE)
        if self.databases is not None:
            statement_keys = [database.key for database in self.databases]
        else:
            statement_keys = None
        rep: NMessage[RepSearchRecordsPayload] = await request(
            NMessageType.SEARCH_RECORDS,
            ReqSearchRecordsPayload(
                module_id=self.module.id,
                statement_keys=statement_keys,
                query=self._query,
                sort=self._sort,
                after=after,
                limit=batch_limit,
                count=count,
            ),
            timeout=5,
            reply_t=RepSearchRecordsPayload,
        )
        if rep.p.error:
            raise RuntimeError(f"{self} failed (after={after}, limit={limit}): {rep.p.error}")
        return rep

    def _unpack_element_data(self, record_data: "RecordData") -> ElementT:
        from bench.language import wire

        parent = self.module._tree.get(record_data.parent_id)
        if parent is None:
            raise RuntimeError(
                f"parent statement {record_data.parent_id} of{record_data.id} not found"
            )
        record = wire.unpack_node_flat(record_data, parent, self.module.session)
        record._instantiated = False
        record._activate_rec(self.module.session)
        return record

    def filter(self, query: Query) -> "RecordSearch":
        combined_query = Query.and_if_set(self._query, query)
        return RecordSearch(self.module, self.databases, combined_query, self._sort, self._limit)

    def sort(self, sort: list[Sort] | Sort) -> "RecordSearch":
        sort = [sort] if isinstance(sort, Sort) else sort
        return RecordSearch(self.module, self.databases, self._query, sort, self._limit)

    def limit(self, limit: int) -> "RecordSearch":
        return RecordSearch(self.module, self.databases, self._query, self._sort, limit)

    async def avalues(self, field: str) -> list[Any]:
        """Returns the values of the given field for all records."""
        return [getattr(record, field) async for record in self]

    def values(self, field: str) -> list[Any]:
        """Returns the values of the given field for all records."""
        return [getattr(record, field) for record in self]

    async def avalues_map(self, func: AmapFunction | MapFunction) -> list[Any]:
        is_async = inspect.iscoroutinefunction(func)
        if is_async:
            return [await func(record) async for record in self]
        else:
            return [func(record) async for record in self]

    def values_map(self, func: MapFunction) -> list[Any]:
        return [func(record) for record in self]

    def map(self, func: MapFunction | BatchMapFunction, batch_size: Optional[int] = None) -> int:
        """Maps the filtered records with the given function."""
        batch: DotList[Record] = DotList() if batch_size is not None else None
        num_mapped = 0
        for record in self:
            if batch_size is None:
                self._map_single_ret(record, func(record))
            else:
                batch.append(record)
                if len(batch) >= batch_size:
                    self._map_batch_ret(batch, func(batch))
                    batch = DotList()
            num_mapped += 1
        if batch_size is not None and batch:
            self._map_batch_ret(batch, func(batch))
        return num_mapped

    async def amap(
        self, func: AmapFunction | BatchAmapFunction, batch_size: Optional[int] = None
    ) -> int:
        """Maps the filtered records with the given async function."""
        batch: list[Record] = []
        num_mapped = 0
        async for record in self:
            if batch_size is None:
                self._map_single_ret(record, await func(record))
            else:
                batch.append(record)
                if len(batch) >= batch_size:
                    self._map_batch_ret(batch, await func(batch))
                    batch = []
            num_mapped += 1
        if batch_size is not None and batch:
            self._map_batch_ret(batch, await func(batch))
        return num_mapped

    def _map_single_ret(self, record: Record, ret: Record) -> None:
        if isinstance(ret, dict):
            for key, value in ret.items():
                record[key] = value
        elif isinstance(ret, Record):
            pass  # already tracked
        elif ret is not None:
            raise TypeError(f"map function returned {ret!r} instead of None or dict")

    def _map_batch_ret(self, records: list[Record], ret: list[Any] | dict[str, list[Any]]) -> None:
        if isinstance(ret, list):
            if len(ret) != len(records):
                raise TypeError(
                    f"batch map function returned {len(ret)} records instead of {len(records)}"
                )
            for record, ret in zip(records, ret):
                if isinstance(ret, dict):
                    for key, value in ret.items():
                        record[key] = value
                elif isinstance(ret, Record):
                    pass  # already tracked
                elif ret is not None:
                    raise TypeError(f"batch map function returned {ret!r} instead of None or dict")
        elif isinstance(ret, dict):
            for i, record in enumerate(records):
                for key, value in ret.items():
                    record[key] = value[i]
        else:
            raise TypeError(f"batch map function returned {ret} instead of list or dict of lists")
