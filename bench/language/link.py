import abc
import enum
import functools
import re
from collections import defaultdict
from dataclasses import dataclass
from typing import (
    TYPE_CHECKING,
    Any,
    Collection,
    Generic,
    Iterator,
    NamedTuple,
    Optional,
    TypeVar,
    Union,
)
from uuid import UUID

from asgiref.sync import async_to_sync

from bench.language.const import (
    NS,
    BenchError,
    ConditionalOp,
    ExpressionOp,
    NodeStatus,
    NodeType,
    NRel,
    QueryEngine,
    SortOp,
    active_session,
)
from bench.language.validation import on_invalid_raise
from bench.proto.wire import AnyNodeData
from bench.utils.fractional import BIGGEST_INTEGER, generate_key_between, generate_n_keys_between
from bench.utils.func import _auto_async_to_sync, nextn

if TYPE_CHECKING:
    from bench.language import (
        Expression,
        Field,
        FieldPath,
        Node,
        NoticeType,
        Property,
        ReadOptions,
        ScopeNode,
        Session,
        TypeInfo,
    )

NodeT = TypeVar("NodeT", bound="Node")


class _NodeChange(enum.IntFlag):
    """The kind of reactive change effect to trigger in a node."""

    Ignore = 0
    Detach = 2**1
    Attach = 2**2
    Full = Detach | Attach


_NC = _NodeChange


def on_notice_raise(
    subject: "Node",
    type: "NoticeType",
    message: Optional[str] = None,
    path: Optional["FieldPath"] = None,
    properties: list["Property"] | None = None,
):
    from bench.language.notice import Notice, NoticeError

    notice = Notice.from_subject(
        subject=subject, type=type, message=message, path=path, properties=properties
    )
    raise NoticeError(notice)


@dataclass(slots=True)
class _InterpChange:
    """
    The effect of a change in nodes.
    TODO @Performance: optimize change effects (batch, lazy/mark dirty?, reduce impact radius)
    """

    prev_session: Optional["Session"]
    prev_status: Optional[NodeStatus]
    level: _NC
    affected: tuple["Node", ...] | None

    @staticmethod
    def _collect(
        from_parent: Optional["Node"],
        to_parent: Optional["Node"],
        changed: tuple["Node", ...],
        level: _NC,
    ) -> "_InterpChange":
        """Collects nodes affected by a change in the given children."""
        assert changed, f"cannot create update on {to_parent!r} without changed nodes"
        assert from_parent or to_parent, f"cannot create update on {changed!r} without parent"

        # collect nodes to reinterp following attach/detach
        if level & (_NC.Detach | _NC.Attach):
            affected_nodes = changed
        else:
            affected_nodes = None

        return _InterpChange(
            prev_session=to_parent._session if to_parent else None,
            prev_status=to_parent._status if to_parent else None,
            affected=affected_nodes,
            level=level,
        )

    def _effect(self, level: _NC | None = None) -> None:
        """Applies the effect of a trigger to update the affected nodes."""
        if level & _NC.Detach:
            for _node in self.affected:
                if _node._session is not None and _node._status == NS.TRACKED:
                    _node._untrack_self()
            for _node in self.affected:
                _node._clear_self(_node.scope)

        if level & _NC.Attach:
            for _node in self.affected:
                _node._interp_self(
                    _node.scope,
                    on_notice=_node.scope._on_notice if _node.scope else on_notice_raise,
                )
                if self.prev_session is not None and self.prev_status == NS.TRACKED:
                    _node._track_self(self.prev_session)


def _sort_nested_ordered_list(root_ck: UUID, nodes: list[NodeT]) -> list[NodeT]:
    """
    Sort a list of ordered, hierarchical nodes.
    Each node is ordered within its 'parent' (by 'order_key'). Start at the root.
    """
    ordered = []

    nodes_by_parent_ck: dict[UUID, list[NodeT]] = defaultdict(list)
    for node in nodes:
        nodes_by_parent_ck[node.parent.ck].append(node)

    def _walk_dfs(parent_ck: UUID):
        children = nodes_by_parent_ck.get(parent_ck, None)
        if children:
            children.sort(key=lambda n: n.order_key or BIGGEST_INTEGER)
            for child in children:
                ordered.append(child)
                _walk_dfs(child.ck)

    _walk_dfs(root_ck)

    if len(ordered) != len(nodes):
        missing_nodes = [n for n in nodes if n not in ordered]
        assert not missing_nodes, f"missing {len(missing_nodes)} nodes {missing_nodes} in {ordered}"
    return ordered


class NodeListBase(abc.ABC, Collection, Generic[NodeT]):
    """
    A list of node descendants for a parent's property.
    This is the primary way of adding, removing and accessing regular node relations.
    """

    __slots__ = ("_parent", "_property")

    def __init__(self, parent: "ScopeNode", property: "Property"):
        self._parent = parent
        self._property = property

    def __repr__(self):
        return f"<{self.__class__.__name__} {self._parent.path}.{self._property.name}: {self}>"

    def create(self, *args, _append: bool = True, **kwargs) -> NodeT:
        """Creates a new node in the list."""
        if len(args) == 1 and isinstance(args[0], Node):
            raise ValueError(f"cannot create {args[0]!r}, use append for existing nodes")
        from bench.language.node import NODE_CLASS_BY_TYPE

        node_cls = NODE_CLASS_BY_TYPE[self._property.reference_types[0]]
        # set new node status to source to prevent activation before it's appended
        if hasattr(node_cls, "new"):
            node = node_cls.new(*args, **kwargs, for_parent=self._parent, _status=NS.SOURCE)
        else:
            node = node_cls(*args, **kwargs, _status=NS.SOURCE)
        if _append:
            self.append(node)
        return node

    def create_many(self, *nodes: Collection[Any | dict]) -> list[NodeT]:
        """Creates a new node in the list."""
        created = []
        for n in nodes:
            if isinstance(n, dict):
                node = self.create(**n, _append=False)
            elif isinstance(n, tuple):
                node = self.create(*n, _append=False)
            else:
                node = self.create(n, _append=False)
            created.append(node)
        self.extend(*created)
        return created

    def append(self, node: NodeT, _create: bool = True, _trigger: _NC = _NC.Full) -> None:
        """
        Attaches a child node to a parent through a list. This is for users adding nodes.
        A node may be 'append'-ed to a list at most once,
         but may exist in multiple lists (through _init_from collection).
        """
        raise NotImplementedError

    def extend(
        self,
        *nodes: Collection[NodeT],
        _create: bool = True,
        _trigger: _NC = _NC.Full,
    ):
        """Attaches a list of child nodes to a parent. See append."""
        raise NotImplementedError

    def remove(self, node: NodeT, _delete: bool = True, _trigger: _NC = _NC.Full):
        """Removes a child node from a parent. See append for reverse."""
        raise NotImplementedError

    def clear(self, _delete: bool = True, _trigger: _NC = _NC.Full):
        """Removes all child nodes from a parent. See append for reverse."""
        raise NotImplementedError

    def set(self, nodes: Collection[NodeT], _trigger: _NC = _NC.Full):
        """Replaces all child nodes of a parent."""
        self.clear(_trigger=_NC.Ignore)
        self.extend(*nodes, _trigger=_trigger)

    def get(self, some_id: str) -> Optional[NodeT]:
        """Gets a node by some id (as determined by the logic of the list)."""
        raise NotImplementedError

    def index(self, node: NodeT) -> int:
        """Gets the index of a node in the list."""
        raise NotImplementedError


class NodeList(NodeListBase[NodeT]):
    __slots__ = ("_child_node_type", "_flags")

    def __init__(self, parent: "ScopeNode", property: "Property"):
        super().__init__(parent, property)
        assert len(property.reference_types) == 1, f"cannot have many child types: {property!r}"
        self._child_node_type: NodeType = property.reference_types[0]
        self._flags = property.children_flags

    def __str__(self):
        return str(self._nodes)

    @property
    def _nodes(self) -> tuple[NodeT, ...]:
        """Access the computed nodes"""
        return self._parent._root_tree.collect_descendants(
            self._parent, self._child_node_type, recursive=bool(self._flags & NRel.CUMULATIVE)
        )

    def _ok_bounds(
        self, after: NodeT = None, before: NodeT = None
    ) -> tuple[Optional[str], Optional[str]]:
        """Gets the order key bounds after the given (default to last)."""
        assert self._flags & NRel.ORDERED, f"cannot get order key for {self!r}"
        if after is not None:
            next_ok = nextn(
                n.order_key
                for n in self._nodes
                if n.order_key > after.order_key and n.parent == after.parent
            )
            return after.order_key, next_ok
        elif before is not None:
            last_ok = nextn(
                n.order_key
                for n in reversed(self._nodes)
                if n.order_key < before.order_key and n.parent == before.parent
            )
            return last_ok, before.order_key
        else:
            last_ok = nextn(
                (n.order_key for n in reversed(self._nodes) if n.parent == self._parent)
            )
            return last_ok, None

    def append(
        self,
        _node: NodeT,
        _create: bool = True,
        after: NodeT = None,
        before: NodeT = None,
        _trigger: _NC = _NC.Full,
    ) -> tuple[NodeT, ...]:
        assert isinstance(_node, Node), f"cannot append {_node!r} to {self!r}"
        if _node.parent is not None:
            raise ValueError(f"cannot attach {_node!r} to {self!r}: attached to {_node.parent!r}")

        # assign ids if newly attached to the package (ids are derived from ck + package)
        if not _node.attached and self._parent.attached:
            package_id = self._parent.package.id
            for n in _node._walk_rec():
                if n.id is None:
                    n._assign_id(package_id)
        change = _InterpChange._collect(None, self._parent, (_node,), _trigger)
        # update parent after updating ids (the above walks tree, which is changed here)
        _node.parent = self._parent
        # validate node now that it has a parent (while in session)
        if self._parent._session is not None:
            _node._validate_self(_node.__tracked_properties__.keys(), on_invalid=on_invalid_raise)

        # add node to parent tree
        if _node.__has_scope__ and _node._local_tree is not None:
            # subsume if previously detached (ignores out of line nodes)
            added = _node._local_tree.collect_descendants(_node, recursive=True)
            _node._local_tree.update(_node)  # parent changed
            self._parent._root_tree.add_tree(_node._local_tree)
            _node._local_tree = None
        else:
            added = (_node,)
            self._parent._root_tree.add(_node)

        # assign order key to ordered nodes
        if self._flags & NRel.ORDERED and _node.order_key is None:
            _node.order_key = generate_key_between(*self._ok_bounds(after, before))
        if _trigger:
            # and update every affected node (to list/interp as needed)
            change._effect(_trigger)

        # 'create' node in session if it's attached
        if _create and self._parent._session and self._parent.attached:
            self._parent._session.create(*added)

        return added

    def extend(
        self,
        *nodes: NodeT,
        _create: bool = True,
        after: NodeT = None,
        before: NodeT = None,
        _trigger: _NC = _NC.Full,
    ) -> None:
        if not nodes:
            return

        # pre-assign order keys since we don't trigger between appends (meaning last_ok is wrong)
        if self._flags & NRel.ORDERED:
            oks = generate_n_keys_between(*self._ok_bounds(after, before), n=len(nodes))
            for node, ok in zip(nodes, oks):
                node.order_key = ok

        # as in append but batched: append, trigger, create
        #  (can we merge them somehow to simplify)?
        change = _InterpChange._collect(None, self._parent, nodes, _trigger)
        change._effect(_trigger & ~_NC.Attach)
        added = []
        for node in nodes:
            added.extend(self.append(node, _create=False, _trigger=_NC.Ignore))
        change._effect(_trigger & ~_NC.Detach)
        if _create and self._parent._session and self._parent.attached:
            self._parent._session.create_many(*added)

    def remove(self, _node: NodeT, _delete: bool = True, _trigger: _NC = _NC.Full):
        change = _InterpChange._collect(self._parent, None, [_node], _trigger)
        if _delete and self._parent._session:
            self._parent.session.delete(_node)
        self._parent._root_tree.remove(_node)
        _node.parent = None
        change._effect(_trigger)

    def clear(self, _delete: bool = True, _trigger: _NC = _NC.Full):
        if not self._nodes:
            return
        removed = tuple(self._nodes)
        change = _InterpChange._collect(self._parent, None, removed, _trigger)
        for _node in removed:
            self.remove(_node, _delete=_delete, _trigger=_NC.Ignore)
        change._effect(_trigger)

    def get(self, some_id: str) -> Optional[NodeT]:
        if not (self._flags & NRel.KEYED) and not (self._flags & NRel.NAMED):
            raise ValueError(f"cannot get {some_id!r} from {self!r}")
        for child in self._nodes:
            if (self._flags & NRel.KEYED and child.key == some_id) or (
                self._flags & NRel.NAMED and (child.name == some_id or child.py_ident == some_id)
            ):
                return child
        return None

    def index(self, node: NodeT) -> int:
        return self._nodes.index(node)

    def __bool__(self):
        return len(self._nodes) > 0

    def __contains__(self, obj: object) -> bool:
        # special case to unwrap key (e.g. for tagging/tag objects)
        if self._flags & NRel.KEYED and hasattr(obj, "dynamic_key"):
            obj = obj.dynamic_key
        if isinstance(obj, str) and (self._flags & NRel.KEYED or self._flags & NRel.NAMED):
            return self.get(obj) is not None
        elif isinstance(obj, Node):
            if obj.metatype != self._property.reference_types[0]:
                raise TypeError(f"{self!r} cannot contain {obj!r}")
            return obj in self._nodes
        else:
            return False

    def __getitem__(self, item: int | slice | str) -> NodeT | list[NodeT]:
        if isinstance(item, int):
            return self._nodes[item]
        elif isinstance(item, slice):
            return self._nodes[item]
        elif isinstance(item, str):
            return self.get(item)
        else:
            raise TypeError(f"invalid index for {self!r}: {item} ({type(item)})")

    def __getattr__(self, item):
        if item.startswith("_"):
            return super().__getattribute__(item)
        node = self.get(item)
        if node is None:
            raise AttributeError(f"no node '{item}' in {self!r}")
        return node

    def __iter__(self) -> Iterator[NodeT]:
        yield from self._nodes

    def __len__(self) -> int:
        return len(self._nodes)

    def __eq__(self, other: object) -> bool:
        if isinstance(other, NodeList):
            return self._nodes == other._nodes
        elif isinstance(other, list):
            return self._nodes == other
        else:
            return False


def _require_expression_op(op: ExpressionOp):
    def decorator(func):
        @functools.wraps(func)
        def wrapper(self: "_TypeExpressionBase", *args, **kwargs):
            from bench.language.expression import _check_field_supports

            _check_field_supports(self._as_type, op)
            return func(self, *args, **kwargs)

        return wrapper

    return decorator


def _to_conditional(op: ConditionalOp, target: Union["Field", "Property"], value: Any = None):
    from bench.language.expression import C, Property

    if isinstance(target, Property):
        return C(op, field=None, property=target, value=value)
    else:
        return C(op, field=target, property=None, value=value)


def _to_sort(op: SortOp, target: Union["Field", "Property"]):
    from bench.language.expression import Property, S

    if isinstance(target, Property):
        return S(op, field=None, property=target)
    else:
        return S(op, field=target, property=None)


class _TypeExpressionBase:
    """
    Base for field-like expressions on a field-like class.
    We define this here to use it for Property and Field.
    """

    @property
    def _as_type(self) -> "TypeInfo":
        raise NotImplementedError(f"{self!r} does not implement type")

    def _coerce_value(self: "TypeInfo", value: Any) -> Any:
        return value

    # comparison

    @_require_expression_op(ConditionalOp.EQUALS)
    def equals(self, value: Any) -> "Expression":
        value = self._coerce_value(value)
        if value is None:
            return self.not_exists()
        return _to_conditional(ConditionalOp.EQUALS, self, value=value)

    @_require_expression_op(ConditionalOp.NOT_EQUALS)
    def not_equal(self, value: Any) -> "Expression":
        value = self._coerce_value(value)
        return _to_conditional(ConditionalOp.NOT_EQUALS, self, value=value)

    @_require_expression_op(ConditionalOp.GREATER_THAN)
    def greater_than(self, value: Any) -> "Expression":
        value = self._coerce_value(value)
        return _to_conditional(ConditionalOp.GREATER_THAN, self, value=value)

    @_require_expression_op(ConditionalOp.GREATER_THAN_OR_EQUALS)
    def greater_than_or_equals(self, value: Any) -> "Expression":
        value = self._coerce_value(value)
        return _to_conditional(ConditionalOp.GREATER_THAN_OR_EQUALS, self, value=value)

    @_require_expression_op(ConditionalOp.LESS_THAN)
    def less_than(self, value: Any) -> "Expression":
        value = self._coerce_value(value)
        return _to_conditional(ConditionalOp.LESS_THAN, self, value=value)

    @_require_expression_op(ConditionalOp.LESS_THAN_OR_EQUALS)
    def less_than_or_equals(self, value: Any) -> "Expression":
        value = self._coerce_value(value)
        return _to_conditional(ConditionalOp.LESS_THAN_OR_EQUALS, self, value=value)

    def __eq__(self, other):
        from bench.language.node import Node

        if isinstance(self, Node) and isinstance(other, Node):
            return Node.__eq__(self, other)  # imitate Field equality
        else:
            return self.equals(other)

    def __ne__(self, other):
        from bench.language.node import Node

        if isinstance(self, Node) and isinstance(other, Node):
            return Node.__ne__(self, other)
        else:
            return self.not_equal(other)

    __gt__ = greater_than
    __ge__ = greater_than_or_equals
    __lt__ = less_than
    __le__ = less_than_or_equals

    # string comparison

    @_require_expression_op(ConditionalOp.MATCHES)
    def matches(self, value: str) -> "Expression":
        return _to_conditional(ConditionalOp.MATCHES, self, value=value)

    @_require_expression_op(ConditionalOp.STARTS_WITH)
    def starts_with(self, value: str) -> "Expression":
        return _to_conditional(ConditionalOp.STARTS_WITH, self, value=value)

    @_require_expression_op(ConditionalOp.REGEX)
    def regex(self, value: str | re.Pattern) -> "Expression":
        if isinstance(value, re.Pattern):
            value = value.pattern
        return _to_conditional(ConditionalOp.REGEX, self, value=value)

    # containment

    @_require_expression_op(ConditionalOp.IN)
    def in_(self, *values: list[Any]) -> "Expression":
        values = [self._coerce_value(value) for value in values]
        return _to_conditional(ConditionalOp.IN, self, value=values)

    @_require_expression_op(ConditionalOp.NOT_IN)
    def not_in(self, *values: list[Any]) -> "Expression":
        values = [self._coerce_value(value) for value in values]
        return _to_conditional(ConditionalOp.NOT_IN, self, value=values)

    @_require_expression_op(ConditionalOp.CONTAINS)
    def contains(self, value: Any) -> "Expression":
        value = self._coerce_value(value)
        return _to_conditional(ConditionalOp.CONTAINS, self, value=value)

    @_require_expression_op(ConditionalOp.NOT_CONTAINS)
    def not_contains(self, value: Any) -> "Expression":
        value = self._coerce_value(value)
        return _to_conditional(ConditionalOp.NOT_CONTAINS, self, value=value)

    # existence

    @_require_expression_op(ConditionalOp.EXISTS)
    def exists(self) -> "Expression":
        return _to_conditional(ConditionalOp.EXISTS, self)

    @_require_expression_op(ConditionalOp.NOT_EXISTS)
    def not_exists(self) -> "Expression":
        return _to_conditional(ConditionalOp.NOT_EXISTS, self)

    # knn

    @_require_expression_op(ConditionalOp.NEAR)
    def near(self, value: list[float]) -> "Expression":
        return _to_conditional(ConditionalOp.NEAR, self, value=value)

    # sort

    @_require_expression_op(SortOp.ASCENDING)
    def asc(self) -> "Expression":
        return _to_sort(SortOp.ASCENDING, self)

    ascending = asc

    @_require_expression_op(SortOp.DESCENDING)
    def desc(self) -> "Expression":
        return _to_sort(SortOp.DESCENDING, self)

    descending = desc


FieldOrProperty = Union["Field", "Property"]
_NodeFetchResult = NamedTuple(
    "_NodeFetchResult",
    [
        ("nodes", list[AnyNodeData]),
        ("cursors", list[str]),
        ("start_cursor", str | None),
        ("total", int),
        ("engine", QueryEngine),
    ],
)


class QueryError(BenchError, ValueError):
    def __init__(self, query: "NodeQuery", cause: Exception | None = None):
        super().__init__(f"{query!r}: {query.filter!r}")
        self.query = query
        self.cause = cause


class NoNodeFoundError(QueryError):
    pass


class MultipleNodesFoundError(QueryError):
    pass


class NodeQuery(Generic[NodeT]):
    def __init__(
        self,
        node_type: NodeType | None,
        filter: Optional["Expression"] = None,
        sort: list["Expression"] | None = None,
        first: int | None = None,
        skip: int | None = None,
        engine: Optional[QueryEngine] = None,
        options: Optional["ReadOptions"] = None,
        cache: bool = True,
    ):
        from bench.language.node import NODE_CLASS_BY_TYPE, Node

        self._node_type = node_type
        self._node_cls = NODE_CLASS_BY_TYPE[node_type] if node_type else Node
        self._filter = filter
        self._sort = sort
        self._first = first
        self._skip = skip
        self._engine = engine
        self._options = options
        self._cache = cache
        self._cached_nodes: list[NodeT] | None = None
        self._cached_cursors: list[str] | None = None

    def __str__(self):
        args_strs = []
        for k in ("filter", "sort", "first", "skip"):
            v = getattr(self, f"_{k}", None)
            if k == "query":
                v = f"({v})" if v is not None else None
            if v is not None:
                args_strs.append(f"{k}={v}")
        return f"{self._node_type} {', '.join(args_strs)}"

    def __repr__(self):
        return f"<{self.__class__.__name__} {self}>"

    def copy(self):
        """Clones the query (the properties are immutable)."""
        return NodeQuery(
            node_type=self._node_type,
            filter=self._filter,
            sort=self._sort,
            first=self._first,
            skip=self._skip,
            engine=self._engine,
            options=self._options,
            # cache is not copied on purpose as it shouldn't propagate
        )

    def _invalidate(self):
        self._cached_records = None
        self._cached_cursors = None

    def _get_target_engine(self, *with_ops: "ExpressionOp") -> QueryEngine:
        from bench.language.expression import ExpressionOps

        node_cls = self._node_cls
        if node_cls.__is_local__:
            ops = self._filter._collect_ops() if self._filter else ()
            if with_ops:
                ops |= set(with_ops)
            if node_cls.__is_indexed_in_os__ and (
                ops & ExpressionOps.AGG_SCALAR or ops & ExpressionOps.AGG_BUCKET
            ):
                return QueryEngine.LOCAL_OPENSEARCH
            else:
                return QueryEngine.LOCAL_POSTGRES
        else:
            return QueryEngine.GLOBAL_POSTGRES

    async def __aiter__(self):
        if self._cached_nodes is None:
            return iter(await self._fetch())
        return iter(self._cached_nodes)

    @_auto_async_to_sync
    async def tolist(self) -> list[NodeT]:
        if self._cached_nodes is None:
            return await self._fetch()
        return self._cached_nodes

    def __iter__(self):
        if self._cached_nodes is None:
            return iter(async_to_sync(self._fetch)())
        return iter(self._cached_nodes)

    def __len__(self):
        if self._cached_nodes is not None:
            return len(self._cached_nodes)
        return self.count()

    @_auto_async_to_sync
    async def get(self, filter: "Expression" = None, **kwargs) -> NodeT:
        """Returns the unique result matching the query (errors otherwise)."""
        from bench.language.expression import coerce_conditional

        filter = coerce_conditional(self._node_cls, filter, kwargs)
        results = await self.filter(filter).tolist()
        if len(results) == 1:
            return results[0]
        elif len(results) == 0:
            raise NoNodeFoundError(self)
        else:
            raise MultipleNodesFoundError(self)

    def filter(self, filter: "Expression" = None, **kwargs) -> "NodeQuery[NodeT]":
        """Adds a filter clause to the query."""
        from bench.language.expression import coerce_conditional

        filter = coerce_conditional(self._node_cls, filter, kwargs)
        copy = self.copy()
        copy._filter = filter & self._filter if self._filter is not None else filter
        return copy

    def sort(
        self, sort: Union[list[Union["Expression", str]], str, "Expression"] = None, *args: str
    ) -> "NodeQuery[NodeT]":
        """Sorts the query results by the given sort criteria."""
        from bench.language.expression import coerce_sort

        copy = self.copy()
        sort = coerce_sort(self._node_cls, sort, args)
        copy._sort = sort
        return copy

    def first(self, count: int) -> "NodeQuery[NodeT]":
        """Returns the first N results."""
        copy = self.copy()
        copy._first = count
        return copy

    def skip(self, count: int) -> "NodeQuery[NodeT]":
        """Skips the first N results."""
        copy = self.copy()
        copy._skip = count
        return copy

    def include(self, *properties: FieldOrProperty) -> "NodeQuery":
        copy = self.copy()
        copy._options = copy._options.copy() if copy._options is not None else ReadOptions()
        if copy._options.include_properties is None:
            copy._options.include_properties = list(properties)
        else:
            copy._options.include_properties.extend(*properties)
        return copy

    def exclude(self, *properties: FieldOrProperty) -> "NodeQuery":
        copy = self.copy()
        copy._options = copy._options.copy() if copy._options is not None else ReadOptions()
        if copy._options.exclude_properties is None:
            copy._options.exclude_properties = list(properties)
        else:
            copy._options.exclude_properties.extend(*properties)
        return copy

    def related(self, *properties: FieldOrProperty) -> "NodeQuery":
        copy = self.copy()
        copy._options = copy._options.copy() if copy._options is not None else ReadOptions()
        if copy._options.related_properties is None:
            copy._options.related_properties = list(properties)
        else:
            copy._options.related_properties.extend(*properties)
        return copy

    def __getitem__(self, item: slice | int) -> Union["NodeQuery[NodeT]", NodeT]:
        if isinstance(item, slice):
            if item.stop is None:
                return self.skip(item.start or 0)
            elif item.start is not None:
                return self.skip(item.start).first(item.stop - item.start)
            else:
                return self.first(item.stop)
        elif isinstance(item, int):
            if self._cached_nodes is None:
                records = async_to_sync(self._fetch)()
            else:
                records = self._cached_nodes
            if item < 0:
                item += len(records)
            if item >= len(records):
                raise IndexError(f"index {item} out of range for {self!r} (got {len(self)})")
            return records[item]
        else:
            raise TypeError(f"expected slice or index into {self!r}, got {type(item)}: {item}")

    async def _fetch(self) -> list[NodeT]:
        from bench.proto import wiring

        session = active_session()
        fetched = await self._do_fetch(session)
        nodes: list[NodeT] = []
        for node_data in fetched.nodes:
            node = wiring.unpack_node(node_data, parent=None, session=session)
            node._track_self(session)
            nodes.append(node)

        if self._cache:
            self._cached_nodes = nodes
            self._cached_cursors = fetched.cursors
        return nodes

    async def _do_fetch(
        self, session: "Session", count: bool = False, after: str = None
    ) -> _NodeFetchResult:
        from bench.proto import wire, wiring
        from bench.sql.engine import compile_pg_conditional, pg_count, pg_select_nodes_data

        engine = self._get_target_engine()
        if engine == QueryEngine.LOCAL_POSTGRES or (
            engine == QueryEngine.GLOBAL_POSTGRES and session._global_pg_cursor
        ):
            cur = session._global_pg_cursor or session._local_pg_cursor
            nodes_data = await pg_select_nodes_data(
                cur=cur,
                node_type=self._node_type,
                filter=self._filter,
                sort=self._sort,
                first=self._first,
                skip=self._skip,
                after=after,
            )
            if count:
                count = await pg_count(
                    cur=cur,
                    table=self._node_cls.__table__,
                    where=compile_pg_conditional(self._node_cls, self._filter),
                )
            else:
                count = None
            return _NodeFetchResult(
                nodes=nodes_data.nodes,
                cursors=nodes_data.cursors,
                start_cursor=nodes_data.start_cursor,
                total=count,
                engine=engine,
            )
        elif engine == QueryEngine.GLOBAL_POSTGRES:  # request from host
            assert not count, "count not supported in host query"
            request = wire.SearchNodesRequest(
                node_type=wiring.pack_enum(NodeType, self._node_type),
                filter=wiring.pack_struct_maybe(self._filter),
                sort=[wiring.pack_struct(s) for s in self._sort] if self._sort else None,
                limit=self._first,
                after=after,
                count=count,
            )
            await session.host.search_nodes(request)
            # return [wiring.unwrap_some_node(n) for n in response.nodes]
            raise NotImplementedError("nocheckin: NodeQuery._do_fetch GLOBAL_POSTGRES")
        else:
            raise ValueError(f"unexpected query engine {engine}")

    @_auto_async_to_sync
    async def count(self, filter: "Expression" = None, **kwargs) -> int:
        """Returns the number of results. May refine the query."""
        from bench.language.expression import coerce_conditional
        from bench.proto import wire, wiring
        from bench.sql.engine import compile_pg_conditional, pg_count

        filter = coerce_conditional(self._node_cls, filter, kwargs, return_none_if_empty=True)
        engine = self._get_target_engine()
        session = active_session()
        if engine == QueryEngine.LOCAL_POSTGRES or (
            engine == QueryEngine.GLOBAL_POSTGRES and session._global_pg_cursor
        ):
            cur = session._global_pg_cursor or session._local_pg_cursor
            return await pg_count(
                cur=cur,
                table=self._node_cls.__table__,
                where=compile_pg_conditional(self._node_cls, filter),
            )
        elif engine == QueryEngine.GLOBAL_POSTGRES:  # request from host
            request = wire.AggregateNodesRequest(
                node_type=wiring.pack_enum(NodeType, self._node_type),
                filter=wiring.pack_struct_maybe(filter),
                limit=self._first,
                aggregation=wire.ExpressionData(op=wire.ExpressionOp.COUNT),
            )
            aggregation_data = await active_session()._host.aggregate_nodes(request)
            return int(aggregation_data.aggregation.scalar)
        else:
            raise ValueError(f"unexpected query engine {engine}")

    @_auto_async_to_sync
    async def exists(self, filter: "Expression" = None, **kwargs) -> bool:
        """Whether any results exist. May refine the query."""
        from bench.language.expression import coerce_conditional
        from bench.proto import wire, wiring
        from bench.sql.engine import compile_pg_conditional, pg_exists

        filter = coerce_conditional(self._node_cls, filter, kwargs, return_none_if_empty=True)
        engine = self._get_target_engine()
        session = active_session()
        if engine == QueryEngine.LOCAL_POSTGRES or (
            engine == QueryEngine.GLOBAL_POSTGRES and session._global_pg_cursor
        ):
            cur = session._global_pg_cursor or session._local_pg_cursor
            return await pg_exists(
                cur=cur,
                table=self._node_cls.__table__,
                where=compile_pg_conditional(self._node_cls, filter),
            )
        elif engine == QueryEngine.GLOBAL_POSTGRES:  # request remotely
            request = wire.AggregateNodesRequest(
                node_type=wiring.pack_enum(NodeType, self._node_type),
                filter=wiring.pack_struct_maybe(filter),
                aggregation=wire.ExpressionData(op=wire.ExpressionOp.EXISTS),
            )
            aggregation_data = await active_session()._host.aggregate_nodes(request)
            return aggregation_data.aggregation.exists
        else:
            raise ValueError(f"unexpected query engine {engine}")


class _NodeExpressionBase:
    @classmethod
    async def tolist(cls: type["Node"]) -> list[NodeT]:
        return await NodeQuery(node_type=cls.metatype).tolist()

    @classmethod
    def get(cls: type["Node"], conditional: "Expression" = None, **kwargs) -> "NodeT":
        return NodeQuery(node_type=cls.metatype).get(conditional, **kwargs)

    @classmethod
    def filter(cls: type["Node"], filter: "Expression" = None, **kwargs) -> "NodeQuery":
        return NodeQuery(node_type=cls.metatype).filter(filter, **kwargs)

    @classmethod
    def sort(cls: type["Node"], sort: "Expression" = None, *args: str) -> "NodeQuery":
        return NodeQuery(node_type=cls.metatype).sort(sort, *args)

    @classmethod
    def include(cls: type["Node"], *properties: FieldOrProperty) -> "NodeQuery":
        return NodeQuery(node_type=cls.metatype).include(*properties)

    @classmethod
    def exclude(cls: type["Node"], *properties: FieldOrProperty) -> "NodeQuery":
        return NodeQuery(node_type=cls.metatype).exclude(*properties)

    @classmethod
    def related(cls: type["Node"], *properties: FieldOrProperty) -> "NodeQuery":
        return NodeQuery(node_type=cls.metatype).related(*properties)

    @classmethod
    def first(cls: type["Node"], count: int) -> "NodeQuery":
        return NodeQuery(node_type=cls.metatype).first(count)

    @classmethod
    async def count(cls: type["Node"], filter: "Expression" = None, **kwargs) -> int:
        return NodeQuery(node_type=cls.metatype).count(filter, **kwargs)

    @classmethod
    async def exists(cls: type["Node"], filter: "Expression" = None, **kwargs) -> bool:
        return NodeQuery(node_type=cls.metatype).exists(filter, **kwargs)
