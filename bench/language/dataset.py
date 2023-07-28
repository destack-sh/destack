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

from bench.language.const import DatasetViewLayout, StatementType, TypeFlag, TypeTag
from bench.language.core import (
    HasCrud,
    HasSession,
    Module,
    ModuleNode,
    Scope,
    Session,
    Statement,
    node,
)
from bench.language.query import Query, Sort
from bench.language.search import ElementT, Search
from bench.language.tag import HasTags
from bench.language.type import Field, HasType, instantiate_py_value, strip_py_value
from bench.utils.func import describe_type, did_you_mean_str
from bench.utils.proxy import proxy_value, unproxy_value
from bench.utils.utils import DotList, required_field

if typing.TYPE_CHECKING:
    from bench.language.wire import RecordData

logger = structlog.get_logger(__name__)


def new_dataset_key():
    """Gets a random alphabetic key as a persistent key."""
    return "".join(random.choices(string.ascii_letters, k=DATASET_BACKEND_KEY_LENGTH))


@node(tracked=["order_key", "value"])
class Record(ModuleNode, HasSession, HasCrud):
    id: UUID = field(default_factory=uuid.uuid4)
    parent: "Dataset" = required_field()
    value: typing.Any = field(default_factory=dict)
    order_key: str = None
    _instantiated: bool = True

    def __str__(self):
        return f"{self.parent.path}:{self.id} {describe_type(self.value)}"

    def __repr__(self):
        return f"<Record {self}>"

    @property
    def parent_id(self):
        return self.parent.id

    @property
    def keys(self):
        return self.value.keys

    def activate_in(self, session: "Session") -> None:
        if self._instantiated:
            self.value = self._raw_value()
            self._instantiated = False
        # proxy
        self.value = instantiate_py_value(
            self.value, self.parent, ignore_array=True, ignore_outer_map=True
        )
        self.value = proxy_value(self.value, onread=self._onread, onwrite=self._onwrite)
        self._instantiated = True
        super().activate_in(session)

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

    def __contains__(self, item: str):
        return item in self.value

    def __getitem__(self, item: str):
        val = self.value.get(item)
        if val is not None or self.parent.has_field(item):
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


DEFAULT_QUERY = None
DEFAULT_SORT = None


@node(tracked=["name", "layout", "query", "sort", "order_key"])
class DatasetView(ModuleNode, HasSession, HasCrud):
    parent: "Dataset" = required_field()
    name: str = None
    layout: Optional[DatasetViewLayout] = DatasetViewLayout.TABLE
    query: Optional[Query] = None
    sort: Optional[list[Sort]] = None
    order_key: str = field(default_factory=uuid.uuid4)
    fields: Optional[list["DatasetViewField"]] = None

    def __str__(self):
        return f"{self.parent.path}:{self.name} ({self.layout})"

    def __repr__(self):
        return f"<DatasetView {self}>"

    @property
    def path(self) -> str:
        return f"{self.parent.path}.{self.name}"


@node
class DatasetViewField(ModuleNode):
    field: UUID | Field = required_field()
    order_key: Optional[str] = None


@node(tracked=["description", "versioned"])
class Dataset(HasType, HasTags, Search["RecordData", Record], Statement):
    type: StatementType = StatementType.DATASET
    description: Optional[str] = None
    tag: TypeTag = TypeTag.STRUCT
    flags: TypeFlag = TypeFlag.IsArray
    versioned: bool = True
    views: Optional[list[DatasetView]] = None
    key: str = field(default_factory=new_dataset_key)

    def _clear(self) -> None:
        HasType._clear(self)
        HasTags._clear(self)
        Statement._clear(self)

    def _interp(self, scope: Scope) -> None:
        HasType._interp(self, scope)
        HasTags._interp(self, scope)

    def view_by_name(self, name: str) -> DatasetView:
        view = first((view for view in self.views if view.name == name), None)
        if view is None:
            raise KeyError(f"no view named '{name}' in {self}")
        return view

    def clear(self):
        self.session.tracer.dataset_clear(self)

    def append(self, record: Record | dict = None, **value):
        """Appends a record to the dataset."""
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

    def map(
        self,
        func: typing.Union["MapFunction", "BatchMapFunction"],
        batch_size: Optional[int] = None,
    ):
        """Maps the dataset with the given function."""
        self.search().map(func, batch_size)

    async def amap(
        self,
        func: typing.Union["AmapFunction", "BatchAmapFunction"],
        batch_size: Optional[int] = None,
    ):
        """Maps the dataset with the given async function."""
        await self.search().amap(func, batch_size)

    def search(
        self, query: Optional[Query] = None, sort: list[Sort] = None, limit: int = None
    ) -> "RecordSearch":
        """Searches this dataset remotely."""
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

    def __len__(self):
        return self.count()

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
    """A search over records (of a dataset)."""

    def __init__(
        self,
        module: Module,
        datasets: list[Dataset],
        query: Query,
        sort: list[Sort],
        limit: Optional[int],
    ):
        super().__init__(module, query, sort, limit)
        if not isinstance(datasets, list):
            raise TypeError(f"datasets must be a list (not {type(datasets)}): {datasets}")
        self.module = module
        self.datasets = datasets

    def __str__(self):
        return f"{self.datasets} {self._query or '<no query>'} {self._sort or '<no sort>'} limit={self._limit or '<no limit>'}"

    def __repr__(self):
        return f"<RecordSearch {self}>"

    async def _do_search(
        self, after: list[Any] = None, limit: Optional[int] = None, count: bool = False
    ):
        from bench.msg import NMessage
        from bench.msg.core import request
        from bench.msg.messages import NMessageType, RepSearchRecordPayload, ReqSearchRecordPayload

        batch_limit = min(self.RESULT_BATCH_SIZE, limit or self._limit or self.RESULT_BATCH_SIZE)
        if self.datasets is not None:
            statement_ids = [dataset.id for dataset in self.datasets]
            keys = [dataset.key for dataset in self.datasets]
        else:
            statement_ids = None
            keys = None
        rep: NMessage[RepSearchRecordPayload] = await request(
            NMessageType.SEARCH_RECORD,
            ReqSearchRecordPayload(
                module_id=self.module.id,
                statement_ids=statement_ids,
                keys=keys,
                query=self._query,
                sort=self._sort,
                after=after,
                limit=batch_limit,
                count=count,
            ),
            reply_t=RepSearchRecordPayload,
        )
        if rep.p.error:
            raise RuntimeError(f"{self} failed (after={after}, limit={limit}): {rep.p.error}")
        return rep

    def _unpack_element_data(self, element_data: "RecordData") -> ElementT:
        from bench.language import wire

        parent = self.module._statements_by_id[element_data.parent_id]
        element = wire.unpack_node_flat(element_data, parent, self.module.session)
        element._instantiated = False
        element.activate_in(self.module.session)
        return element

    def filter(self, query: Query) -> "RecordSearch":
        combined_query = Query.and_if_set(self._query, query)
        return RecordSearch(self.module, self.datasets, combined_query, self._sort, self._limit)

    def sort(self, sort: list[Sort] | Sort) -> "RecordSearch":
        sort = [sort] if isinstance(sort, Sort) else sort
        return RecordSearch(self.module, self.datasets, self._query, sort, self._limit)

    def limit(self, limit: int) -> "RecordSearch":
        return RecordSearch(self.module, self.datasets, self._query, self._sort, limit)

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


@node(tracked=["description", "value"])
class Value(HasType, HasTags, Statement):
    type: StatementType = StatementType.VALUE
    description: Optional[str] = None
    tag: TypeTag = TypeTag.STRUCT
    flags: TypeFlag = TypeFlag.Zero
    value: Any = field(default_factory=dict)
    _instantiated: bool = True

    def _clear(self) -> None:
        Statement._clear(self)
        HasType._clear(self)
        HasTags._clear(self)
        self.deactivate()

    def _interp(self, scope: Scope) -> None:
        HasType._interp(self, scope)
        HasTags._interp(self, scope)

    def _onread(self, key: str) -> None:
        pass

    def _onwrite(self, key: str) -> None:
        self.session.tracer.value_update(self, key)

    def activate_in(self, session: "Session") -> None:
        if self._instantiated:
            self.value = self._raw_value()
            self._instantiated = False
        # proxy
        self.value = instantiate_py_value(
            self.value, self, ignore_array=True, ignore_outer_map=True
        )
        self.value = proxy_value(self.value, onread=self._onread, onwrite=self._onwrite)
        self._instantiated = True
        super().activate_in(session)

    def deactivate(self) -> None:
        super().deactivate()
        if self._instantiated:
            self.value = self._raw_value()
            self._instantiated = False

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
        elif not isinstance(item, str):
            raise TypeError(f"cannot index {self} with {type(item)}")
        else:
            candidates = {
                **{f: f for f in self._PROPERTIES},
                **{f.py_ident: f for f in self.parent.fields},
            }
            did_you_mean = did_you_mean_str(candidates, item)
            raise AttributeError(
                f"{self} has no field {item} ({did_you_mean}, available: {self.fields})"
            )

    def __setattr__(self, key, value):
        if key in self._PROPERTIES:
            super().__setattr__(key, value)
        else:
            self.value[key] = value

    def __getitem__(self, item):
        if item in self.value:
            return self.value[item]
        elif self.has_field(item):
            return None
        elif not isinstance(item, str):
            raise TypeError(f"cannot index {self} with {type(item)}")
        else:
            candidates = {f.py_ident: f for f in self.parent.fields}
            did_you_mean = did_you_mean_str(candidates, item)
            raise AttributeError(
                f"{self} has no field {item} ({did_you_mean}, available: {self.fields})"
            )

    def __iter__(self):
        return iter(self.value)


DATASET_BACKEND_KEY_LENGTH = 16
MAX_VERSIONED_RECORDS_TOTAL = 64_000
MAX_VERSIONED_RECORDS_PER_DATASET = 8_000
