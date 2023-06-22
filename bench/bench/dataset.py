from __future__ import annotations

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
from bench.bench.core import HasCrud, HasSession, ModuleNode, Scope, Session, Symbol, node
from bench.bench.expect import IsExpectable
from bench.bench.query import Query, Sort
from bench.bench.type import (
    Field,
    HasType,
    instantiate_py_value_flat,
    map_value,
    strip_py_value_flat,
)
from bench.utils.func import describe_type
from bench.utils.proxy import proxy_value, unproxy_value
from bench.utils.utils import required_field

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
        self.value = map_value(
            value=self.value,
            type=self.parent,
            map_k=lambda f: (f.typed_key, f.ident),
            map_v=instantiate_py_value_flat,
            ignore_outer_map=True,
            ignore_array=True,
        )
        self.value = proxy_value(self.value, onread=self._onread, onwrite=self._onwrite)
        self._instantiated = True

    def _raw_value(self) -> dict:
        if not self._instantiated:
            return self.value
        else:
            return map_value(
                value=self.value,
                type=self.parent,
                map_k=lambda f: (f.ident, f.typed_key),
                map_v=strip_py_value_flat,
                ignore_outer_map=True,
                ignore_array=True,
            )

    def _onread(self, key: Optional[str]):
        pass

    def _onwrite(self, key: Optional[str]):
        self.session.tracer.dataset_update(self.parent, self, key)

    def __getitem__(self, item: str):
        try:
            return self.value[item]
        except KeyError:
            raise KeyError(f"{self} has no field '{item}' (available: {list(self.value.keys())})")

    def __setitem__(self, key, value):
        self.value[key] = value

    def __getattr__(self, item):
        try:
            return self.value[item]
        except KeyError:
            raise KeyError(f"{self} has no field '{item}' (available: {list(self.value.keys())})")

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
class Dataset(Symbol, HasType, IsExpectable):
    description: Optional[str] = None
    tag: TypeTag = TypeTag.STRUCT
    flags: TypeFlag = TypeFlag.IsArray
    versioned: bool = True
    views: Optional[list[DatasetView]] = None
    backend: DatasetBackend = DatasetBackend.OPENSEARCH
    backend_id: str = field(default_factory=new_dataset_backend_id)

    def _clear(self) -> None:
        Symbol._clear(self)
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
    ) -> SearchResult:
        """Searches this dataset remotely."""
        self.session.tracer.dataset_search(self, query, sort)
        return SearchResult(self, query, sort, limit)

    def __len__(self):
        return len(self.search(limit=0))

    def __iter__(self):
        return iter(self.search())

    async def __aiter__(self):
        return aiter(self.search())


SEARCH_RESULT_BATCH_SIZE = 100

MapFunction = typing.Callable[[Record], typing.Union[Record, dict]]
BatchMapFunction = typing.Callable[[list[Record]], list[typing.Union[Record, dict]]]
AmapFunction = typing.Callable[[Record], typing.Awaitable[typing.Union[Record, dict]]]
BatchAmapFunction = typing.Callable[
    [list[Record]], typing.Awaitable[list[typing.Union[Record, dict]]]
]


class SearchResult:
    def __init__(self, dataset: Dataset, query: Query, sort: list[Sort], limit: Optional[int]):
        self.dataset = dataset
        self.query = query
        self.sort = sort
        self.limit = limit
        # cache
        self._total: Optional[int] = None

    def __str__(self):
        return f"{self.dataset} {self.query or '<no query>'} {self.sort or '<no sort>'} limit={self.limit or '<no limit>'}"

    def __repr__(self):
        return f"<SearchResult {self}>"

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

        batch_limit = min(SEARCH_RESULT_BATCH_SIZE, limit or self.limit or SEARCH_RESULT_BATCH_SIZE)
        rep: NMessage[RepSearchDatasetPayload] = await request(
            NMessageType.REQUEST_SEARCH_DATASET,
            ReqSearchDatasetPayload(
                module_id=self.dataset.module.id,
                statement_id=self.dataset.id,
                backend_id=self.dataset.backend_id,
                query=self.query,
                sort=self.sort,
                after=after,
                limit=batch_limit,
                count=count,
            ),
            reply_t=RepSearchDatasetPayload,
        )
        return rep

    def __iter__(self) -> typing.Iterator[Record]:
        from bench.bench import wire

        after = None
        while True:
            rep = self.dataset.session.async_to_sync(self._do_search)(after=after)
            if len(rep.payload.records) == 0:
                break
            for record_data in rep.payload.records:
                record = wire.unpack_node_flat(record_data, self.dataset, self.dataset.session)
                record._instantiated = False
                record.instantiate_in(self.dataset.session)
                yield record
            after = rep.payload.last_sort_key

    async def __aiter__(self) -> typing.AsyncIterator[Record]:
        """Iterates over the records of the search result (batched)."""
        from bench.bench import wire

        after = None
        while True:
            rep = await self._do_search(after=after)
            if len(rep.payload.records) == 0:
                break
            for record_data in rep.payload.records:
                record = wire.unpack_node_flat(record_data, self.dataset, self.dataset.session)
                record._instantiated = False
                record.instantiate_in(self.dataset.session)
                yield record
            after = rep.payload.after

    def __len__(self) -> int:
        if self._total is not None:
            return self._total
        rep = self.dataset.session.async_to_sync(self._do_search)(limit=0, count=True)
        self._total = rep.payload.total

    def map(self, func: MapFunction | BatchMapFunction, batch_size: Optional[int] = None):
        """Maps the filtered records with the given function."""
        raise NotImplementedError

    def amap(self, func: AmapFunction | BatchMapFunction, batch_size: Optional[int] = None):
        """Maps the filtered records with the given async function."""
        raise NotImplementedError


@node(tracked=["description", "value"])
class Value(Symbol, HasType, IsExpectable):
    description: Optional[str] = None
    tag: TypeTag = TypeTag.STRUCT
    flags: TypeFlag = TypeFlag.Zero
    value: Any = field(default_factory=dict)
    _instantiated: bool = True

    @property
    def keys(self):
        return self.value.keys()

    def _clear(self) -> None:
        HasType._clear(self)
        if self._instantiated:
            self.value = self._raw_value()

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
        self.value = map_value(
            value=self.value,
            type=self,
            map_k=lambda f: (f.typed_key, f.ident),
            map_v=instantiate_py_value_flat,
            ignore_outer_map=True,
        )
        self.value = proxy_value(self.value, onread=self._onread, onwrite=self._onwrite)
        self._instantiated = True

    def _raw_value(self) -> dict:
        if not self._instantiated:
            return self.value
        else:
            return map_value(
                value=self.value,
                type=self,
                map_k=lambda f: (f.ident, f.typed_key),
                map_v=strip_py_value_flat,
                ignore_outer_map=True,
            )

    def __getattr__(self, item):
        if item in self._PROPERTIES:
            return self.__dict__[item]
        elif item in self.value:
            return self.value[item]
        else:
            raise AttributeError(f"{self} has no field {item} (available: {self.keys()})")

    def __setattr__(self, key, value):
        if key in self._PROPERTIES:
            super().__setattr__(key, value)
        else:
            self.value[key] = value


DATASET_BACKEND_KEY_LENGTH = 16
MAX_VERSIONED_RECORDS_TOTAL = 64_000
MAX_VERSIONED_RECORDS_PER_DATASET = 8_000
