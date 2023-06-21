from __future__ import annotations

import enum
import random
import string
import typing
import uuid
from dataclasses import field
from typing import Any, Optional
from uuid import UUID

from asgiref.sync import async_to_sync

from bench.bench.core import HasCrud, ModuleNode, Scope, Session, Symbol, node
from bench.bench.expect import IsExpectable
from bench.bench.query import Query, Sort
from bench.bench.type import Field, HasType, TypeFlag, TypeTag, instantiate_py_value_flat, map_value
from bench.utils.func import describe_type
from bench.utils.proxy import unproxy_value
from bench.utils.utils import required_field


class DatasetBackend(enum.StrEnum):
    OPENSEARCH = "os"


def new_dataset_backend_id():
    """Gets a random alphabetic key as a persistent key."""
    return "".join(random.choices(string.ascii_letters, k=DATASET_BACKEND_KEY_LENGTH))


class DatasetViewLayout(enum.StrEnum):
    """The layout of a dataset view."""

    TABLE = "table"


@node
class Record(ModuleNode, HasCrud):
    id: UUID = field(default_factory=uuid.uuid4)
    data: typing.Any = field(default_factory=dict)
    order_key: str = None
    _instantiated: bool = True

    def __str__(self):
        return f"{self.order_key} {describe_type(self.data)}"

    def __repr__(self):
        return f"<Record {self}>"

    @property
    def parent_id(self):
        return self.parent.id

    @property
    def keys(self):
        return self.data.keys

    def __getitem__(self, item: str):
        try:
            return self.data[item]
        except KeyError:
            raise KeyError(f"missing key '{item}' (available: {list(self.data.keys())})")

    def __setitem__(self, key, value):
        self.data[key] = value

    def __getattr__(self, item):
        try:
            return self.data[item]
        except KeyError:
            raise KeyError(f"missing key '{item}' (available: {list(self.data.keys())})")

    def __setattr__(self, key, value):
        if key in self.__dict__:
            super().__setattr__(key, value)
        else:
            self[key] = value


@node
class DatasetView(ModuleNode, HasCrud):
    name: str = None
    layout: Optional[DatasetViewLayout] = DatasetViewLayout.TABLE
    query: Optional[Query] = None
    sort: Optional[list[Sort]] = None
    order_key: str = field(default_factory=uuid.uuid4)
    fields: Optional[list[DatasetViewField]] = None


@node
class DatasetViewField(ModuleNode):
    field: UUID | Field = required_field()
    order_key: Optional[str] = None


DEFAULT_VIEW = DatasetView(name="default")


@node
class Dataset(Symbol, HasType, IsExpectable):
    description: Optional[str] = None
    tag: TypeTag = TypeTag.STRUCT
    flags: TypeFlag = TypeFlag.IsArray
    length: Optional[int] = None
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

    def clear(self):
        self.session.tracer.dataset_clear(self)

    def append(self, record: Record = None, **data):
        """Appends a record to the dataset."""
        if record is not None:
            if data:
                raise ValueError("cannot pass both record and data")
            data = record.data
        # TODO @UX: order records when inserted in code
        data = unproxy_value(data)  # remove source proxy if any
        record = Record(_id=uuid.uuid4(), parent=self, data=data)
        self.session.tracer.dataset_append(self, record)

    def extend(self, records: typing.Iterable[Record | dict]):
        """Extends the dataset with the given records."""
        datas = [  # remove source proxy if any
            unproxy_value(record.data) if isinstance(record, Record) else unproxy_value(record)
            for record in records
        ]
        records = [Record(_id=uuid.uuid4(), parent=self, data=data) for data in datas]
        self.session.tracer.dataset_extend(self, records)

    async def asearch(self, query: Query, sort: list[Sort] = None, limit: int = None) -> Dataset:
        """Searches this dataset remotely."""
        self.session.tracer.dataset_search(self, query, sort)
        raise NotImplementedError

    def search(self, query: Query, sort: list[Sort] = None, limit: int = None) -> Dataset:
        """Searches this dataset remotely."""
        return async_to_sync(self.asearch)(query, sort)

    def __len__(self):
        return self.length

    def __iter__(self):
        # nocheckin: remote datasets
        raise NotImplementedError

    def __aiter__(self):
        # nocheckin: remote datasets
        raise NotImplementedError


@node
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

    def _interp(self, scope: Scope) -> None:
        HasType._interp(self, scope)

    def _onread(self, key: str) -> None:
        pass

    def _onwrite(self, key: str) -> None:
        self.session.tracer.value_update(self, key)

    def instantiate_in(self, session: "Session") -> None:
        if self._instantiated:
            self.value = self._raw_value()
        # proxy
        self.value = map_value(
            value=self.value,
            type=self,
            map_k=lambda f: (f.typed_key, f.ident),
            map_v=instantiate_py_value_flat,
            ignore_outer_map=True,
        )
        self._instantiated = True

    def _raw_value(self) -> dict:
        if not self._instantiated:
            return self.value

    def __getattr__(self, item):
        if item in self._PROPERTIES:
            return self.__dict__[item]
        elif item in self.value:
            return self.value[item]
        else:
            raise AttributeError(item)

    def __setattr__(self, key, value):
        if key in self._PROPERTIES:
            super().__setattr__(key, value)
        else:
            self.value[key] = value


DATASET_BACKEND_KEY_LENGTH = 16
MAX_VERSIONED_RECORDS_TOTAL = 64_000
MAX_VERSIONED_RECORDS_PER_DATASET = 8_000
