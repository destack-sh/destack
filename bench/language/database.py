import enum
import typing
from typing import Optional
from uuid import UUID

import structlog

from bench.language.const import DATABASE_VERSIONED_RECORD_LIMIT, MNT, new_dynamic_node_key
from bench.language.expression import Query, Sort
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
    node,
    node_component,
    nparent,
)
from bench.language.value import HasValue
from bench.utils.func import describe_type
from bench.utils.utils import flatten_list

if typing.TYPE_CHECKING:
    from bench.language import Field, Statement, View
LOCAL_RECORD_CACHE_LIMIT = DATABASE_VERSIONED_RECORD_LIMIT

logger = structlog.get_logger(__name__)


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


class RecordQuery:
    def __init__(
        self,
        database: "HasDatabase",
        query: Query | None = None,
        sort: list[Sort] = None,
        include: list["Field"] = None,
        select: list["Field"] = None,
    ):
        self._database = database
        self._query = query
        self._sort = sort
        self._include = include
        self._select = select

    # nocheckin design/scaffold

    def deepcopy(self):
        return RecordQuery(
            database=self._database,
            query=self._query,
            sort=self._sort,
            include=self._include,
            select=self._select,
        )

    async def _do_search_preflight(self, search: "Search") -> None:
        """FLush any relevant edits before searching."""
        # force flush and index if there are any pending database edits
        #  (or previous edits that were already flushed but didn't refresh the index)
        # TODO @Performance: force flush module for record search only if needed
        session = self._database.session
        if session._editor.edits or session._past_flushes:
            await session.acommit(optimistic=False, refresh_index=True)

    def filter(self, query: Query) -> "RecordQuery":
        copy = self.deepcopy()
        copy._query = query & self._query if self._query else query
        return copy

    def sort(self, sort: list[Sort] | Sort) -> "RecordQuery":
        copy = self.deepcopy()
        if isinstance(sort, Sort):
            sort = [sort]
        copy._sort = sort
        return copy

    def select(self, *fields: "Field") -> "RecordQuery":
        raise NotImplementedError

    def include(self, *fields: "Field") -> "RecordQuery":
        raise NotImplementedError

    def limit(self, limit: int) -> "RecordQuery":
        raise NotImplementedError

    def update(self, **kwargs) -> int:
        raise NotImplementedError

    def delete(self) -> int:
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


class RecordRelationToMany(RecordRelation):
    """
    The many side of a one-to-many or many-to-many relation.
    """

    def __init__(
        self, parent: "Record", field: "Field", type: RelationType, reverse_field: "Field" = None
    ):
        super().__init__(parent, field, type, reverse_field)
        self.value: list[Record] = []

    def filter(self, query: Query) -> "RecordQuery":
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

    def __len__(self) -> int:
        return self.count()


class RecordList(NodeListBase[Record], RecordQuery):
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
        raise NotImplementedError(f"index into {self!r} not yet supported")

    def __contains__(self, obj: object) -> bool:
        return False  # lookup by id?

    #
    # Extra methods for record queries/expressions
    #

    def __len__(self):
        if self._cached_records_by_ck is not None:
            return len(self._cached_records_by_ck)
        return len(self.query())

    def __iter__(self):
        if self._cached_records_by_ck is not None:
            return iter(self._cached_records_by_ck.values())

    def __aiter__(self):
        return aiter(self.search())


@node_component
class HasDatabase(Node):
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
