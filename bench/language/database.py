import inspect
import typing
from typing import Any, Optional
from uuid import UUID

import structlog

from bench.language.const import (
    DATABASE_VERSIONED_RECORD_LIMIT,
    MNT,
    DatabaseViewLayout,
    new_dynamic_node_key,
)
from bench.language.field import Field
from bench.language.module import (
    _NC,
    NS,
    Module,
    Node,
    NodeList,
    NodeListBase,
    NodeProperty,
    NRel,
    ScopeNode,
    _ChangeEffect,
    _Passthrough,
    nchildren,
    ninternal,
    node,
    node_component,
    nparent,
    nproperty,
)
from bench.language.query import Query, Sort
from bench.language.search import ElementT, Search
from bench.language.validation import enum_validator
from bench.language.value import HasValue
from bench.utils.func import describe_type
from bench.utils.utils import DotList, flatten_list

if typing.TYPE_CHECKING:
    from bench.language import Statement
    from bench.language.wire import RecordData

logger = structlog.get_logger(__name__)


@node(mnt=MNT.RECORD, passthrough=(("value", _Passthrough.Full),))
class Record(HasValue, Node):
    parent: "Statement" = nparent(MNT.STATEMENT)

    @staticmethod
    def new(*args, for_parent: "Statement" = None, _status: NS = None, **kwargs) -> "Record":
        from bench.language.packer import check_type

        if not for_parent and args:
            raise TypeError(f"cannot create record with args {args} without for_parent")

        value = {**kwargs}
        if for_parent and for_parent.attached:
            # inline args and check type for instant feedback
            for field, arg in zip(for_parent.resolved_fields, args):
                value[field.name] = arg
            check_type(value, for_parent)
        return Record(value=value, _status=_status)

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


@node(mnt=MNT.DATABASE_VIEW)
class DatabaseView(ScopeNode):
    parent: "Statement" = nparent(MNT.STATEMENT)
    name: str | None = nproperty(default=None)
    layout: DatabaseViewLayout = nproperty(
        default=DatabaseViewLayout.TABLE, validate=enum_validator(DatabaseViewLayout)
    )
    query: Optional[Query] = nproperty(default=None)
    sort: Optional[list[Sort]] = nproperty(default=None)
    fields: Optional[list["DatabaseViewField"]] = nchildren(MNT.DATABASE_VIEW_FIELD)

    def __str__(self):
        return f"{self.parent.path}:{self.name} ({self.layout})"

    def __repr__(self):
        return f"<DatabaseView {self}>"

    @property
    def path(self) -> str:
        return f"{self.parent.path}.{self.name}"


@node(mnt=MNT.DATABASE_VIEW_FIELD)
class DatabaseViewField(Node):
    field: UUID | Field = nproperty()
    order_key: str | None = ninternal(default=None)


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
        databases: list["HasDatabase"],
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
        record = wire.unpack_node_flat(record_data, parent, None)
        if self.module._session:
            record._activate_self(self.module._session)
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


LOCAL_RECORD_CACHE_LIMIT = DATABASE_VERSIONED_RECORD_LIMIT


class RecordList(NodeListBase[Record], RecordSearch):
    """
    Implements NodeList protocol for remote records with a local cache.
    TODO @UX @Performance: turn record list into proper hybrid list (:BE-352)
     use local list for everything but search (for now) (in ~small databases only)
     need to fetch initial state on first read, use cache in RecordSearch, watermarks, etc.
    """

    def __init__(self, parent: "ScopeNode", property: NodeProperty):
        super().__init__(parent, property)
        # TODO @Broken: init RecordList.super(RecordSearch) (need module for that.. where?)
        self._cached_records_by_ck: dict[UUID, Record] | None = None
        if parent._new:
            # right now we only use the cache for new databases to avoid cache complexity (see above)
            self._cached_records_by_ck = {}

    def __str__(self):
        if self._cached_records_by_ck is None:
            return "remote"  # can't really say anything useful here
        else:
            return str(self._cached_records_by_ck.values())

    def _update(self, scope: "ScopeNode"):
        pass  # nothing to do, not part of regular tree

    def append(self, node: Record, _create: bool = True, _trigger: _NC = _NC.Full) -> None:
        assert isinstance(node, Record), f"cannot append {node!r} to {self!r}"
        node.parent = self._parent
        if node.id is None and self._parent.attached:
            node._assign_id(self._parent.module.id)
        # update cache
        if self._cached_records_by_ck is not None:
            self._cached_records_by_ck[node.ck] = node  # not quite right, see :BE-352
        # 'create' node
        if _create:
            if self._parent._session:
                self._parent.session.tracer.node_create_preflight(node)
            if not self._parent.attached:
                # detached record nodes are temporarily hoisted into inline tree :TempRecordTree
                self._parent._local_root_tree.add(node)
        # update affected nodes
        if _trigger:
            _ChangeEffect._collect(None, self._parent, [node], _trigger)._effect(_trigger)
        # 'create' node (for real)
        if _create and self._parent._session and self._parent.attached:
            self._parent.session.tracer.node_create(node)

    def extend(self, *nodes: Record, _create: bool = True, _trigger: _NC = _NC.Full) -> None:
        nodes = flatten_list(nodes)
        for record in nodes:
            self.append(record, _create=False, _trigger=_NC.Ignore)
        # 'create' nodes
        if _create:
            if self._parent._session:
                self._parent.session.tracer.node_create_preflight(*nodes)
            if not self._parent.attached:
                self._parent._local_root_tree.add_many(nodes)  # :TempRecordTree
        # update affected nodes
        if _trigger:
            _ChangeEffect._collect(None, self._parent, nodes, _trigger)._effect(_trigger)
        # 'create' nodes (for real)
        if _create and self._parent._session and self._parent.attached:
            self._parent.session.tracer.node_create(*nodes)

    def remove(self, node: Record, _delete: bool = True, _trigger: _NC = _NC.Full) -> None:
        if _delete:
            if self._parent._session:
                self._parent._session.tracer.node_delete(self, node)
            if not self._parent.attached:
                self._parent._local_root_tree.remove(node)
        node.parent = None
        if self._cached_records_by_ck is not None and node.ck in self._cached_records_by_ck:
            del self._cached_records_by_ck[node.ck]

    def clear(self, _delete: bool = True, _trigger: _NC = _NC.Full) -> None:
        if _delete:
            if self._parent._session:
                self._parent._session.tracer.node_truncate(self._parent, MNT.RECORD)
            if not self._parent.attached:
                self._parent._local_root_tree.truncate(self._parent, MNT.RECORD)
        if self._cached_records_by_ck is not None:
            self._cached_records_by_ck.clear()

    def __getitem__(self, item: slice):
        raise NotImplementedError(f"index into {self!r} not supported")

    def __contains__(self, obj: object) -> bool:
        return False  # lookup by id?

    #
    # Extra methods for records
    #

    def search(
        self, query: Optional[Query] = None, sort: list[Sort] = None, limit: int = None
    ) -> "RecordSearch":
        """Searches this database remotely."""
        return RecordSearch(self._parent.module, [self._parent], query, sort, limit)

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
        if self._cached_records_by_ck is not None:
            return len(self._cached_records_by_ck)
        else:
            return self.search().count()

    def __iter__(self):
        if self._cached_records_by_ck is not None:
            return iter(self._cached_records_by_ck.values())
        else:
            return iter(self.search())

    def __aiter__(self):
        return aiter(self.search())


@node_component
class HasDatabase(Node):
    # note that HasDatabase doesn't feel like component like the others (HasCode, HasText, etc.)
    #  but it would also be weird to have it not be a component now.
    views: NodeList["DatabaseView"] = nchildren(MNT.DATABASE_VIEW, NRel.Named | NRel.Ordered)
    records: NodeList["Record"] = nchildren(MNT.RECORD, NRel.Remote, custom_list=RecordList)

    def _init_inner(self):
        # this runs before HasFields because of the ordering in
        #  (which is necessary because HasFields also sets key)
        if self.key is None:
            self.key = self._derive_key()

    @staticmethod
    def _derive_key(instance: "HasDatabase") -> str | None:
        if instance.versioned:
            if instance.id is not None:
                return new_dynamic_node_key(instance.id)
            else:
                return None
        else:
            return new_dynamic_node_key(instance.ck)

    # maybe these should also go into passthrough?

    def _iter_inner(self):
        return iter(self.records)

    def _aiter_inner(self):
        return aiter(self.records)

    def _len_inner(self):
        return len(self.records)
