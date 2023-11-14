import enum
import typing
from typing import Optional
from uuid import UUID

import structlog

from bench.language.const import DATABASE_VERSIONED_RECORD_LIMIT, MNT, new_dynamic_node_key
from bench.language.expression import C, Conditional, ConditionalOp, Sort
from bench.language.module import (
    _NC,
    NS,
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
)
from bench.language.value import HasValue
from bench.search.client import os_client
from bench.search.core import DocumentType
from bench.utils.func import describe_type
from bench.utils.utils import flatten_list

if typing.TYPE_CHECKING:
    from bench.language import Field, Session, Statement, View

logger = structlog.get_logger(__name__)

LOCAL_RECORD_CACHE_LIMIT = DATABASE_VERSIONED_RECORD_LIMIT
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
    ):
        self._database = database
        self._query = query
        self._sort = sort
        self._include = include
        self._select = select
        self._distinct = distinct
        self._first = first
        self._skip = skip
        self._result_cache: list[Record] | None = None

    def __str__(self):
        args_strs = []
        for k in ("query", "sort", "include", "select", "distinct", "first", "skip"):
            v = getattr(self, f"_{k}")
            if v is not None:
                args_strs.append(f"{k}={v}")
        return f"{self._database} {', '.join(args_strs)}"

    def __repr__(self):
        return f"<RecordQuery {self}>"

    def deepcopy(self):
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
        )

    async def _execute(self, session: "Session"):
        from bench.language import wire
        from bench.search import mirror
        from bench.search.mapping import prepare_os_query

        # force flush and index if there are any pending database edits
        #  (or previous edits that were already flushed but didn't refresh the index)
        # TODO @Performance: force flush module for record search only if needed by query
        if session._editor.edits or session._past_commits:
            await session.acommit(refresh_index=True)

        # TODO @Broken: iterate through all records if query has no limit?
        query = Conditional.and_if_set(
            self._query, C(ConditionalOp.EQUALS, "statement_key", value=self._database.key)
        )
        limit = self._first or LOCAL_RECORD_CACHE_LIMIT
        search = prepare_os_query(
            type=DocumentType.RECORD,
            project_version_id=None,
            query=query,
            limit=limit + 1,
            count=True,
            sort=self._sort,
        )
        os_name = self._database.module.os_name  # may be different from current session's module
        logger.debug("record.query", query=self, actual=query, limit=limit, os_name=os_name)
        os_results = await os_client.search(index=os_name, body=search)
        results = []
        record_mirror = mirror._packers_by_mirror[mirror.Record]
        has_more = len(os_results["hits"]["hits"]) > limit
        total = os_results["hits"]["total"]["value"]
        for hit in os_results["hits"]["hits"][:limit]:
            record_doc = mirror.Record.from_dict(hit["_source"], hit["_id"])
            record_data = record_mirror.pack(record_doc)
            record = wire.unpack_node_flat(record_data, self._database, session)
            record._activate_self(session)
            results.append(record)
        self._result_cache = results
        logger.debug(
            "record.query.done", query=self, results=len(results), total=total, has_more=has_more
        )

    async def __aiter__(self):
        if self._result_cache is None:
            await self._execute(self._database.session)
        return iter(self._result_cache)

    async def atolist(self) -> list[Record]:
        if self._result_cache is None:
            await self._execute(self._database.session)
        return self._result_cache

    def __iter__(self):
        if self._result_cache is None:
            self._database.session.async_to_sync(self._execute)(self._database.session)
        return iter(self._result_cache)

    def tolist(self) -> list[Record]:
        if self._result_cache is None:
            self._database.session.async_to_sync(self._execute)(self._database.session)
        return self._result_cache

    def __len__(self):
        # maybe just issue count query?
        if self._result_cache is None:
            self._database.session.async_to_sync(self._execute)(self._database.session)
        return len(self._result_cache)

    def filter(self, query: Conditional) -> "RecordBaseQuery":
        """Adds a filter clause to the query."""
        copy = self.deepcopy()
        copy._query = query & self._query if self._query else query
        return copy

    def sort(self, sort: list[Sort] | Sort) -> "RecordBaseQuery":
        """Sorts the query results by the given sort criteria."""
        copy = self.deepcopy()
        if isinstance(sort, Sort):
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
        copy = self.deepcopy()
        copy._first = count
        return copy

    def skip(self, count: int) -> "RecordBaseQuery":
        """Skips the first N results."""
        copy = self.deepcopy()
        copy._skip = count
        return copy

    def group_by(self, *fields: "Field") -> "RecordBaseQuery":
        """Groups the results by the given fields."""
        raise NotImplementedError

    def update(self, **kwargs) -> int:
        """Updates all results with the given values."""
        raise NotImplementedError("not yet supported")

    def delete(self) -> int:
        """Deletes all results."""
        raise NotImplementedError("not yet supported")


class RecordWriteQuery(RecordBaseQuery):
    """
    Updates or deletes records in a result set.
    """

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
        last: int = None,
        update: dict["Field", typing.Any] = None,
        delete: bool = False,
    ):
        super().__init__(
            database=database,
            query=query,
            sort=sort,
            include=include,
            select=select,
            distinct=distinct,
            first=first,
            skip=skip,
            last=last,
        )
        self._update = update
        self._delete = delete

    async def _execute(self, session: "Session"):
        raise NotImplementedError("not yet supported")


class RecordSingleAggregationQuery(RecordBaseQuery):
    """
    Aggregate results into a single value.
    """

    pass  # (not yet supported)


class RecordGroupAggregationQuery(RecordBaseQuery):
    """
    Bucket and aggregate results.
    """

    pass  # (not yet supported)


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
    """
    Implements NodeList protocol for remote records with a local cache.
    TODO @UX @Performance: turn record list into proper hybrid list? (:BE-352)
    """

    def __init__(self, parent: "ScopeNode", property: NodeProperty):
        NodeListBase[Record].__init__(self, parent, property)
        RecordBaseQuery.__init__(self, parent)
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
                self._parent.session._tracer.node_create_preflight(node)
            if not self._parent.attached:
                # detached record nodes are temporarily hoisted into inline tree :TempRecordTree
                self._parent._local_root_tree.add(node)
        # update affected nodes
        if _trigger:
            _ChangeEffect._collect(None, self._parent, [node], _trigger)._effect(_trigger)
        # 'create' node (for real)
        if _create and self._parent._session and self._parent.attached:
            self._parent.session._tracer.node_create(node)

    def extend(self, *nodes: Record, _create: bool = True, _trigger: _NC = _NC.Full) -> None:
        nodes = flatten_list(nodes)
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
        # 'create' nodes (for real)
        if _create and self._parent._session and self._parent.attached:
            self._parent.session._tracer.node_create(*nodes)

    def remove(self, node: Record, _delete: bool = True, _trigger: _NC = _NC.Full) -> None:
        if _delete:
            if self._parent._session:
                self._parent._session._tracer.node_delete(self, node)
            if not self._parent.attached:
                self._parent._local_root_tree.remove(node)
        node.parent = None
        if self._cached_records_by_ck is not None and node.ck in self._cached_records_by_ck:
            del self._cached_records_by_ck[node.ck]

    def clear(self, _delete: bool = True, _trigger: _NC = _NC.Full) -> None:
        if _delete:
            if self._parent._session:
                self._parent._session._tracer.node_truncate(self._parent, MNT.RECORD)
            if not self._parent.attached:
                self._parent._local_root_tree.truncate(self._parent, MNT.RECORD)
        if self._cached_records_by_ck is not None:
            self._cached_records_by_ck.clear()

    def __getitem__(self, item: slice):
        raise NotImplementedError(f"index into {self!r} not yet supported")

    def __contains__(self, obj: object) -> bool:
        return False  # lookup by id?

    #
    # Extra methods for record queries/expressions
    #

    def __len__(self):
        if self._cached_records_by_ck is not None:
            return len(self._cached_records_by_ck)
        return RecordBaseQuery.__len__(self)

    def __iter__(self):
        if self._cached_records_by_ck is not None:
            return iter(self._cached_records_by_ck.values())
        return RecordBaseQuery.__iter__(self)

    def __aiter__(self):
        if self._cached_records_by_ck is not None:
            return iter(self._cached_records_by_ck.values())
        return RecordBaseQuery.__aiter__(self)


@node_component
class HasDatabase(Node):
    ephemeral: bool = ninternal(default=True)
    views: NodeList["View"] = nchildren(MNT.VIEW, NRel.Named | NRel.Ordered)
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

    # maybe these node methods should also go into passthrough?

    def _iter_inner(self):
        return iter(self.records)

    def _aiter_inner(self):
        return aiter(self.records)

    def _len_inner(self):
        return len(self.records)
