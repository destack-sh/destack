from __future__ import annotations

import inspect
import random
import string
import typing
import uuid
from dataclasses import field
from typing import Any, Optional
from uuid import UUID

import structlog
from more_itertools import first

from bench.bench.const import DatasetBackend, DatasetViewLayout, TypeFlag, TypeTag
from bench.bench.core import HasCrud, HasSession, ModuleNode, Scope, Session, Statement, node
from bench.bench.expect import IsExpectable
from bench.bench.query import Query, Sort
from bench.bench.type import Field, HasType, instantiate_py_value, strip_py_value
from bench.utils.func import describe_type
from bench.utils.proxy import proxy_value, unproxy_value
from bench.utils.utils import DotDictList, required_field

logger = structlog.get_logger(__name__)


def new_dataset_backend_id():
    """Gets a random alphabetic key as a persistent key."""
    return "".join(random.choices(string.ascii_letters, k=DATASET_BACKEND_KEY_LENGTH))


@node(tracked=["order_key", "value"])
class Record(ModuleNode, HasSession, HasCrud):
    id: UUID = field(default_factory=uuid.uuid4)
    parent: Dataset = required_field()
    value: typing.Any = field(default_factory=dict)
    order_key: str = None
    _instantiated: bool = True

    def __str__(self):
        return f"{self.parent}:{self.order_key or '<unordered>'} {describe_type(self.value)}"

    def __repr__(self):
        return f"<Record {self}>"

    @property
    def parent_id(self):
        return self.parent.id

    @property
    def keys(self):
        return self.value.keys

    def instantiate_in(self, session: "Session") -> None:
        if self._instantiated:
            self.value = self._raw_value()
            self._instantiated = False
        # proxy
        self.value = instantiate_py_value(
            self.value, self.parent, ignore_array=True, ignore_outer_map=True
        )
        self.value = proxy_value(self.value, onread=self._onread, onwrite=self._onwrite)
        self._instantiated = True

    def _raw_value(self) -> dict:
        if not self._instantiated:
            return self.value
        else:
            value = unproxy_value(self.value)
            return strip_py_value(value, self.parent, ignore_array=True, ignore_outer_map=True)

    def _onread(self, key: Optional[str]):
        pass

    def _onwrite(self, key: Optional[str]):
        # TODO @Performance: writing an entire update on every change is obviously inefficient
        self.session.tracer.dataset_update(self.parent, self, key)

    def __getitem__(self, item: str):
        val = self.value.get(item)
        if val is not None or self.parent.has_field(item):
            return val
        else:
            raise KeyError(f"{self} has no field '{item}' (available: {list(self.parent.fields)})")

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


DEFAULT_QUERY = None
DEFAULT_SORT = None


@node(tracked=["name", "layout", "query", "sort", "order_key"])
class DatasetView(ModuleNode, HasSession, HasCrud):
    name: str = None
    layout: Optional[DatasetViewLayout] = DatasetViewLayout.TABLE
    query: Optional[Query] = None
    sort: Optional[list[Sort]] = None
    order_key: str = field(default_factory=uuid.uuid4)
    fields: Optional[list[DatasetViewField]] = None


DEFAULT_VIEW = DatasetView()


@node
class DatasetViewField(ModuleNode):
    field: UUID | Field = required_field()
    order_key: Optional[str] = None


@node(tracked=["description", "versioned"])
class Dataset(HasType, IsExpectable, Statement):
    description: Optional[str] = None
    tag: TypeTag = TypeTag.STRUCT
    flags: TypeFlag = TypeFlag.IsArray
    versioned: bool = True
    views: Optional[list[DatasetView]] = None
    backend: DatasetBackend = DatasetBackend.OPENSEARCH
    backend_id: str = field(default_factory=new_dataset_backend_id)

    def _clear(self) -> None:
        Statement._clear(self)
        HasType._clear(self)

    def _interp(self, scope: Scope) -> None:
        HasType._interp(self, scope)

    @property
    def default_view(self) -> DatasetView:
        return self.views[0] if self.views else DEFAULT_VIEW

    def view_by_name(self, name: str) -> DatasetView:
        view = first((view for view in self.views if view.name == name), None)
        if view is None:
            raise KeyError(f"no view named '{name}' in {self}")
        return view

    def clear(self):
        self.session.tracer.dataset_clear(self)

    def append(self, record: Record = None, **value):
        """Appends a record to the dataset."""
        if record is not None:
            if value:
                raise ValueError("cannot pass both record and data")
            value = record.value
        # TODO @UX: order records when inserted in code
        value = unproxy_value(value)  # remove source proxy if any
        record = Record(id=uuid.uuid4(), parent=self, value=value)
        self.session.tracer.dataset_append(self, record)

    def extend(self, records: typing.Iterable[Record | dict]):
        """Extends the dataset with the given records."""
        values = [  # remove source proxy if any
            unproxy_value(record.value) if isinstance(record, Record) else unproxy_value(record)
            for record in records
        ]
        records = [Record(id=uuid.uuid4(), parent=self, value=value) for value in values]
        self.session.tracer.dataset_extend(self, records)

    def map(self, func: MapFunction | BatchMapFunction, batch_size: Optional[int] = None):
        """Maps the dataset with the given function."""
        self.search().map(func, batch_size)

    async def amap(self, func: AmapFunction | BatchAmapFunction, batch_size: Optional[int] = None):
        """Maps the dataset with the given async function."""
        await self.search().amap(func, batch_size)

    def search(
        self, query: Optional[Query] = None, sort: list[Sort] = None, limit: int = None
    ) -> Search:
        """Searches this dataset remotely."""
        return Search(self, query, sort, limit)

    def __getitem__(self, item: slice):
        if isinstance(item, slice):
            return self.search(limit=item.stop)
        else:
            raise TypeError(f"index into {self} must be slice (not {type(item)})")

    def filter(self, query: Query) -> Search:
        return self.search(query=query)

    def sort(self, sort: list[Sort] | Sort) -> Search:
        if isinstance(sort, Sort):
            sort = [sort]
        return self.search(sort=sort)

    def limit(self, limit: int) -> Search:
        return self.search(limit=limit)

    def __len__(self):
        return len(self.search(limit=0))

    def __iter__(self):
        return iter(self.search())

    async def __aiter__(self):
        return aiter(self.search())


SEARCH_RESULT_BATCH_SIZE = 400

MapFunction = typing.Callable[[Record], typing.Union[Record, dict]]
BatchMapFunction = typing.Callable[[list[Record]], list[typing.Union[Record, dict]]]
AmapFunction = typing.Callable[[Record], typing.Awaitable[typing.Union[Record, dict]]]
BatchAmapFunction = typing.Callable[
    [list[Record]], typing.Awaitable[list[typing.Union[Record, dict]]]
]


class Search:
    """A search over a dataset."""

    def __init__(self, dataset: Dataset, query: Query, sort: list[Sort], limit: Optional[int]):
        self.dataset = dataset
        self._query = query
        self._sort = sort
        self._limit = limit
        # cache
        self._total: Optional[int] = None

    def __str__(self):
        return f"{self.dataset} {self._query or '<no query>'} {self._sort or '<no sort>'} limit={self._limit or '<no limit>'}"

    def __repr__(self):
        return f"<Search {self}>"

    def filter(self, query: Query) -> Search:
        return Search(self.dataset, self._query.filter(query), self._sort, self._limit)

    def sort(self, sort: list[Sort] | Sort) -> Search:
        if isinstance(sort, Sort):
            sort = [sort]
        return Search(self.dataset, self._query, sort, self._limit)

    def limit(self, limit: int) -> Search:
        return Search(self.dataset, self._query, self._sort, limit)

    async def _do_search(
        self, after: list[Any] = None, limit: Optional[int] = None, count: bool = False
    ):
        from bench.msg import NMessage
        from bench.msg.core import request
        from bench.msg.messages import (
            NMessageType,
            RepSearchDatasetPayload,
            ReqSearchDatasetPayload,
        )

        batch_limit = min(
            SEARCH_RESULT_BATCH_SIZE, limit or self._limit or SEARCH_RESULT_BATCH_SIZE
        )
        rep: NMessage[RepSearchDatasetPayload] = await request(
            NMessageType.REQUEST_SEARCH_DATASET,
            ReqSearchDatasetPayload(
                module_id=self.dataset.module.id,
                statement_id=self.dataset.id,
                backend_id=self.dataset.backend_id,
                query=self._query,
                sort=self._sort,
                after=after,
                limit=batch_limit,
                count=count,
            ),
            reply_t=RepSearchDatasetPayload,
        )
        if rep.p.error:
            raise RuntimeError(f"{self} failed (after={after}, limit={limit}): {rep.p.error}")
        return rep

    def __iter__(self) -> typing.Iterator[Record]:
        yield from self._iter(batched=False)

    def batched(self) -> typing.Iterator[list[Record]]:
        yield from self._iter(batched=True)

    def _iter(self, batched: bool):
        from bench.bench import wire

        after = None
        remaining_limit = self._limit
        while remaining_limit is None or remaining_limit > 0:
            rep = self.dataset.session.async_to_sync(self._do_search)(
                after=after, limit=remaining_limit
            )
            if len(rep.payload.records) == 0:
                break
            records = DotDictList() if batched else None
            for record_data in rep.payload.records:
                record = wire.unpack_node_flat(record_data, self.dataset, self.dataset.session)
                record._instantiated = False
                record.instantiate_in(self.dataset.session)
                if batched:
                    records.append(record)
                else:
                    yield record
            if batched:
                yield records
            after = rep.payload.last_sort_key
            if remaining_limit is not None:
                remaining_limit -= len(rep.payload.records)

    async def abatched(self) -> typing.AsyncIterator[list[Record]]:
        async for batch in self._aiter(batched=True):
            yield batch

    async def __aiter__(self) -> typing.AsyncIterator[Record]:
        """Iterates over the records of the search result (batched)."""
        async for record in self._aiter(batched=False):
            yield record

    async def _aiter(self, batched: bool) -> typing.AsyncIterator[Record]:
        from bench.bench import wire

        after = None
        remaining_limit = self._limit
        while remaining_limit is None or remaining_limit > 0:
            rep = await self._do_search(after=after, limit=remaining_limit)
            if len(rep.payload.records) == 0:
                break
            records = DotDictList() if batched else None
            for record_data in rep.payload.records:
                record = wire.unpack_node_flat(record_data, self.dataset, self.dataset.session)
                record._instantiated = False
                record.instantiate_in(self.dataset.session)
                if batched:
                    records.append(record)
                else:
                    yield record
            if batched:
                yield records
            after = rep.payload.last_sort_key
            if remaining_limit is not None:
                remaining_limit -= len(rep.payload.records)

    def __len__(self) -> int:
        return self.count()

    async def afirst(self) -> Optional[Record]:
        """Returns the first record of the search result."""
        async for record in self.limit(1):
            return record
        return None

    def first(self) -> Optional[Record]:
        """Returns the first record of the search result."""
        return self.dataset.session.async_to_sync(self.afirst)()

    async def atolist(self) -> list[Record]:
        """Returns the search result as a list."""
        return [record async for record in self]

    def tolist(self) -> list[Record]:
        """Returns the search result as a list."""
        return list(self)

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

    def count(self):
        if self._total is not None:
            return self._total
        rep = self.dataset.session.async_to_sync(self._do_search)(limit=0, count=True)
        self._total = rep.payload.total
        return self._total

    def map(self, func: MapFunction | BatchMapFunction, batch_size: Optional[int] = None):
        """Maps the filtered records with the given function."""
        batch: DotDictList[Record] = DotDictList() if batch_size is not None else None
        for record in self:
            if batch_size is None:
                self._map_single_ret(record, func(record))
            else:
                batch.append(record)
                if len(batch) >= batch_size:
                    self._map_batch_ret(batch, func(batch))
                    batch = DotDictList()
        if batch_size is not None and batch:
            self._map_batch_ret(batch, func(batch))

    async def amap(self, func: AmapFunction | BatchAmapFunction, batch_size: Optional[int] = None):
        """Maps the filtered records with the given async function."""
        batch: list[Record] = []
        async for record in self:
            if batch_size is None:
                self._map_single_ret(record, await func(record))
            else:
                batch.append(record)
                if len(batch) >= batch_size:
                    self._map_batch_ret(batch, await func(batch))
                    batch = []
        if batch_size is not None and batch:
            self._map_batch_ret(batch, await func(batch))

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


@node(tracked=["description", "value"])
class Value(HasType, IsExpectable, Statement):
    description: Optional[str] = None
    tag: TypeTag = TypeTag.STRUCT
    flags: TypeFlag = TypeFlag.Zero
    value: Any = field(default_factory=dict)
    _instantiated: bool = True

    def _clear(self) -> None:
        HasType._clear(self)
        if self._instantiated:
            self.value = self._raw_value()
            self._instantiated = False

    def _interp(self, scope: Scope) -> None:
        HasType._interp(self, scope)

    def _onread(self, key: str) -> None:
        pass

    def _onwrite(self, key: str) -> None:
        self.session.tracer.value_update(self, key)

    def instantiate_in(self, session: "Session") -> None:
        super().instantiate_in(session)
        if self._instantiated:
            self.value = self._raw_value()
            self._instantiated = False
        # proxy
        self.value = instantiate_py_value(
            self.value, self, ignore_array=True, ignore_outer_map=True
        )
        self.value = proxy_value(self.value, onread=self._onread, onwrite=self._onwrite)
        self._instantiated = True

    def _raw_value(self) -> dict:
        if not self._instantiated:
            return self.value
        else:
            return strip_py_value(self.value, self, ignore_array=True, ignore_outer_map=True)

    def __getattr__(self, item):
        if item in self._PROPERTIES:
            return self.__dict__[item]
        elif item in self.value:
            return self.value[item]
        elif self.has_field(item):
            return None
        else:
            raise AttributeError(f"{self} has no field {item} (available: {self.fields})")

    def __setattr__(self, key, value):
        if key in self._PROPERTIES:
            super().__setattr__(key, value)
        else:
            self.value[key] = value


DATASET_BACKEND_KEY_LENGTH = 16
MAX_VERSIONED_RECORDS_TOTAL = 64_000
MAX_VERSIONED_RECORDS_PER_DATASET = 8_000
