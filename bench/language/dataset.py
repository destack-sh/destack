import inspect
import itertools
import typing
import uuid
from dataclasses import field
from typing import Any, Optional
from uuid import UUID

import structlog
from more_itertools import first

from bench.language.basic import HasText
from bench.language.const import DatasetViewLayout, StatementType, TypeFlag, TypeTag
from bench.language.core import (
    MNT,
    HasCrud,
    HasSession,
    Module,
    ModuleNode,
    ModuleVisitor,
    Scope,
    Session,
    Statement,
    node,
)
from bench.language.query import Query, Sort
from bench.language.search import ElementT, Search
from bench.language.tag import HasTags
from bench.language.type import Field, HasType, map_value, pack_value, unpack_value
from bench.utils.func import describe_type, did_you_mean_str
from bench.utils.proxy import proxy_value, unproxy_value
from bench.utils.utils import DotList, required_field

if typing.TYPE_CHECKING:
    from bench.language.wire import RecordData

logger = structlog.get_logger(__name__)


@node(mnt=MNT.Record, tracked=["value"])
class Record(ModuleNode, HasSession, HasCrud):
    id: UUID = field(default_factory=uuid.uuid4)
    parent: "Dataset" = required_field()
    value: typing.Any = field(default_factory=dict)
    _instantiated: bool = True

    def __str__(self):
        return f"{self.parent.path}:{self.id} {describe_type(self.value)}"

    def __repr__(self):
        return f"<Record {self}>"

    def _visit(self, visitor: ModuleVisitor) -> None:
        pass

    @property
    def parent_id(self):
        return self.parent.id

    @property
    def keys(self):
        return self.value.keys

    def _activate_in(self, session: "Session") -> None:
        if self._instantiated:
            self.value = self._raw_value()
            self._instantiated = False
        # proxy
        self.value = unpack_value(self.value, self.parent, ignore_array=True, ignore_outer_map=True)
        self.value = proxy_value(self.value, onread=self._onread, onwrite=self._onwrite)
        self._instantiated = True
        super()._activate_in(session)

    def _raw_value(self) -> dict:
        """The raw/stripped value with field keys."""
        if not self._instantiated:
            return self.value
        else:
            value = unproxy_value(self.value)
            return pack_value(value, self.parent, ignore_array=True, ignore_outer_map=True)

    def _raw_named_value(self):
        """The raw/stripped value with field names."""
        return map_value(
            self._raw_value(),
            self.parent,
            ignore_array=True,
            ignore_outer_map=True,
            map_k=lambda f: (f.typed_key, f.py_ident),
        )

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


@node(mnt=MNT.DatasetView, tracked=["name", "layout", "query", "sort", "order_key"])
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


@node(mnt=MNT.DatasetViewField, tracked=["order_key"])
class DatasetViewField(ModuleNode, HasSession, HasCrud):
    field: UUID | Field = required_field()
    order_key: Optional[str] = None


@node(tracked=["text", "versioned"])
class Dataset(HasType, HasTags, HasText, Search["RecordData", Record], Statement):
    type: StatementType = StatementType.DATASET
    tag: TypeTag = TypeTag.STRUCT
    flags: TypeFlag = TypeFlag.IsArray
    versioned: bool = True
    views: Optional[list[DatasetView]] = None

    def _clear(self) -> None:
        Statement._clear(self)
        HasText._clear(self)
        HasType._clear(self)
        HasTags._clear(self)

    def _interp(self, scope: Scope) -> None:
        HasText._interp(self, scope)
        HasType._interp(self, scope)
        HasTags._interp(self, scope)

    def _visit(self, visitor: "ModuleVisitor") -> None:
        for n in itertools.chain(self.children, self.fields, self.tags):
            visitor.visit_child(n)

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
        record = Record(parent=self, value=value)
        self._notify_added(record)
        self.session.tracer.dataset_append(self, record)

    def extend(self, records: typing.Iterable[Record | dict]):
        """Extends the dataset with the given records."""
        values = [  # remove source proxy if any
            unproxy_value(record.value) if isinstance(record, Record) else unproxy_value(record)
            for record in records
        ]
        records = [Record(parent=self, value=value) for value in values]
        self._notify_added(*records)
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

    async def _do_search(self, after: str = None, limit: Optional[int] = None, count: bool = False):
        from bench.msg import NMessage
        from bench.msg.core import request
        from bench.msg.messages import (
            NMessageType,
            RepSearchRecordsPayload,
            ReqSearchRecordsPayload,
        )

        batch_limit = min(self.RESULT_BATCH_SIZE, limit or self._limit or self.RESULT_BATCH_SIZE)
        if self.datasets is not None:
            statement_ids = [dataset.id for dataset in self.datasets]
            statement_cks = [dataset.ck for dataset in self.datasets]
        else:
            statement_ids = None
            statement_cks = None
        rep: NMessage[RepSearchRecordsPayload] = await request(
            NMessageType.SEARCH_RECORDS,
            ReqSearchRecordsPayload(
                module_id=self.module.id,
                statement_ids=statement_ids,
                statement_cks=statement_cks,
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

        parent = self.module._nodes_by_id.get(record_data.parent_id)
        if parent is None:
            raise RuntimeError(
                f"parent statement {record_data.parent_id} of{record_data.id} not found"
            )
        record = wire.unpack_node_flat(record_data, parent, self.module.session)
        record._instantiated = False
        record._activate_in(self.module.session)
        return record

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


@node(tracked=["text", "value"])
class Variable(HasType, HasTags, HasText, Statement):
    type: StatementType = StatementType.VARIABLE
    text: Optional[str] = None
    tag: TypeTag = TypeTag.STRUCT
    flags: TypeFlag = TypeFlag.Zero
    value: Any = field(default_factory=dict)
    _instantiated: bool = False  # when should we set this?

    def _clear(self) -> None:
        Statement._clear(self)
        HasText._clear(self)
        HasType._clear(self)
        HasTags._clear(self)
        self._deactivate()

    def _interp(self, scope: Scope) -> None:
        HasText._interp(self, scope)
        HasType._interp(self, scope)
        HasTags._interp(self, scope)

    def _visit(self, visitor: "ModuleVisitor") -> None:
        for n in itertools.chain(self.children, self.fields, self.tags):
            visitor.visit_child(n)
        HasText._visit(self, visitor)

    def _onread(self, key: str) -> None:
        pass

    def _onwrite(self, key: str) -> None:
        self.session.tracer.variable_update(self, key)

    def _activate_in(self, session: "Session") -> None:
        if self._instantiated:
            self.value = self._raw_value()
            self._instantiated = False
        # proxy
        self.value = unpack_value(self.value, self, ignore_array=True, ignore_outer_map=True)
        self.value = proxy_value(self.value, onread=self._onread, onwrite=self._onwrite)
        self._instantiated = True
        super()._activate_in(session)

    def _deactivate(self) -> None:
        super()._deactivate()
        if self._instantiated:
            self.value = self._raw_value()
            self._instantiated = False

    def _raw_value(self) -> dict:
        """The raw/stripped value with field keys."""
        if not self._instantiated:
            return self.value
        else:
            return pack_value(self.value, self, ignore_array=True, ignore_outer_map=True)

    def _raw_named_value(self):
        """The raw/stripped value with field names."""
        return map_value(
            self._raw_value(),
            self,
            ignore_array=True,
            ignore_outer_map=True,
            map_k=lambda f: (f.typed_key, f.py_ident),
        )

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
                **{f.py_ident: f for f in self.fields},
            }
            raise AttributeError(
                f"{self} has no field {item} ({did_you_mean_str(candidates, item)}, available: {self.fields})"
            )

    def __setattr__(self, key, value):
        if key in self._PROPERTIES:
            super().__setattr__(key, value)
        else:
            assert self.value is not None, f"cannot set {key} on {self} without value"
            self.value[key] = value

    def __getitem__(self, item):
        if item in self.value:
            return self.value[item]
        elif self.has_field(item):
            return None
        elif not isinstance(item, str):
            raise TypeError(f"cannot index {self} with {type(item)}")
        else:
            candidates = {f.py_ident: f for f in self.fields}
            raise AttributeError(
                f"{self} has no field {item} ({did_you_mean_str(candidates, item)}, available: {self.fields})"
            )

    def __iter__(self):
        return iter(self.value)


DATASET_BACKEND_KEY_LENGTH = 16
MAX_VERSIONED_RECORDS_TOTAL = 64_000
MAX_VERSIONED_RECORDS_PER_DATASET = 8_000
