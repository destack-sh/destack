import abc
import functools
from collections import defaultdict
from datetime import datetime, timedelta
from itertools import chain
from typing import (
    TYPE_CHECKING,
    Any,
    Callable,
    ClassVar,
    Collection,
    Iterable,
    Mapping,
    NamedTuple,
    Optional,
    Self,
    Sequence,
    Union,
    cast,
    dataclass_transform,
    final,
    overload,
    override,
)
from uuid import UUID, uuid4

import structlog
from opentelemetry import trace

from bench import pb2
from bench.language.registry import (
    BUILTIN_OBJECT_CLASS_BY_TYPE,
    CHILD_NODE_TYPES,
    DESCENDANT_NODE_TYPES,
    HAS_CHILD_NODE_TYPES,
    NODE_CLASS_BY_NAME,
    NODE_CLASS_BY_TYPE,
)
from bench.pb2 import AnyNodeData, NodeReferenceData
from bench.utils.env import IS_DEV
from bench.utils.fractional import INTEGER_ZERO
from bench.utils.func import dualmethod, hash_stable
from bench.utils.string import Casing, to_casing, to_code_name
from bench.utils.utils import frozendict

from .const import (
    ACTIVE_SESSION,
    BASED_NODE_TYPES,
    BENCH_NODE_TYPES,
    EMPTY_DICT,
    IS_IN_USER_CODE,
    NODE_TYPES,
    PACKAGE_NODE_TYPES,
    UNSET,
    BuiltinEnum,
    FieldType,
    NodeArea,
    NodeType,
    ObjectType,
    QueryType,
    ReferenceKind,
    StructType,
    TypeKind,
    active_session,
    bittuple,
)
from .graph import NULL_SUPERGRAPH, NodeDataGraph, NodeGraph
from .list import attach_node
from .object import (
    EMPTY_SCOPE_DATA,
    BuiltinObject,
    FieldOrProperty,
    NodeTypeOrClass,
    _process_object_cls,
    _trace_edit_operation,
)
from .property import (
    _PROPERTY_SPECIFIERS,
    Property,
    p_internal,
    p_node_ancestor_with_self,
    p_node_parent,
    p_regular,
    p_runtime,
    p_subnode_packed,
    p_system,
)
from .struct import Struct, struct_
from .trait import SUBJECT_NODE_TYPES, IsBased, IsInstantiable, IsModal, Subject, TypeBaseNode
from .validation import on_invalid_raise

if TYPE_CHECKING:
    from bench.language import (
        Bench,
        Block,
        CustomObject,
        Expression,
        Field,
        GetConnection,
        Icon,
        NodeLink,
        NodeReference,
        Package,
        Page,
        Query,
        SearchConnection,
        Session,
        TextLine,
        Type,
    )

# pyright: reportIncompatibleVariableOverride=false

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


class IndexIn(NamedTuple):
    """Index to be turned into a SQL Index."""

    columns: tuple[str, ...]
    cover: tuple[str, ...] = ()
    is_unique: bool = False
    condition: str | None = None
    name: str | None = None


@dataclass_transform(kw_only_default=True, field_specifiers=_PROPERTY_SPECIFIERS)
def node_component_(
    node_type: NodeType | None = None,
    passthrough_set: str | tuple[str, ...] | None = None,
    passthrough_get: str | tuple[str, ...] | None = None,
    is_root: bool = False,
    is_variable_root: bool = False,
    is_final: bool = False,
    is_subtype: bool = False,
):
    """Mark a class as a node component (or concrete node for a NodeType)."""
    if isinstance(passthrough_get, str):
        passthrough_get = (passthrough_get,)
    if isinstance(passthrough_set, str):
        passthrough_set = (passthrough_set,)

    def decorate(cls: type["Node"]) -> type["Node"]:
        cls, properties = _process_object_cls(
            cls=cls,
            object_type=node_type,
            is_root=is_root,
            is_variable_root=is_variable_root,
            is_final=is_final,
            is_node=True,
        )
        cls.__passthrough_get__ = passthrough_get
        cls.__passthrough_set__ = passthrough_set

        # register node properties
        child_properties: dict[str, Property] = {}
        child_properties_by_type: dict[NodeType, list[Property]] = defaultdict(list)
        ancestor_properties: dict[str, Property] = {}
        for prop in properties.values():
            if prop.reference_kind == ReferenceKind.NODE_CHILDREN:
                if cls.__name__ != "Node" and not issubclass(cls, Node) and node_type is not None:
                    raise ValueError(f"{cls} is not a Node for {prop}")
                child_properties[prop.name] = prop
                assert isinstance(
                    prop.reference_nodes, tuple
                ), f"unexpected {prop.reference_nodes!r} for {prop!r}"
                child_properties_by_type[prop.reference_nodes[0]].append(prop)
            elif (
                prop.reference_kind == ReferenceKind.NODE_ANCESTOR
                or prop.reference_kind == ReferenceKind.NODE_ANCESTOR_OR_SELF
            ):
                ancestor_properties[prop.name] = prop
        cls.__node_child_properties__ = frozendict(child_properties)
        cls.__node_child_properties_by_type__ = frozendict(child_properties_by_type)
        cls.__node_ancestor_properties__ = frozendict(ancestor_properties)
        # subtypes for every final node base
        if is_final and not is_subtype:
            cls.__subclass_by_subtype__ = {}
            cls.__subtype_by_subclass__ = {}

        # register as concrete node class for node_type
        if node_type:
            cls.metatype = node_type
        if node_type and not is_subtype:
            if node_type in NODE_CLASS_BY_TYPE:
                raise ValueError(
                    f"node class conflict for {node_type}: {cls}, {NODE_CLASS_BY_TYPE[node_type]}"
                )
            NODE_CLASS_BY_TYPE[node_type] = cls
            NODE_CLASS_BY_NAME[cls.__name__] = cls

        return cls

    return decorate


@dataclass_transform(kw_only_default=True, field_specifiers=_PROPERTY_SPECIFIERS)
def node_(
    node_type: NodeType,
    passthrough_get: str | tuple[str, ...] | None = None,
    passthrough_set: str | tuple[str, ...] | None = None,
    stored: bool = True,
    stored_value_unraveled: bool = False,
    roots: tuple[NodeType, ...] = (NodeType.BENCH,),
    index: tuple[IndexIn, ...] = (),
    has_subtypes: bool = False,
):
    """Register a class as a concrete node for the given node type."""

    in_package = node_type in PACKAGE_NODE_TYPES
    in_bench = node_type in BENCH_NODE_TYPES

    # default index for nodes with parents
    if roots:
        index = (*index, IndexIn(columns=("parent_id",), cover=("id",)))

    def decorate(cls: type["Node"]) -> type["Node"]:
        cls = node_component_(
            node_type=node_type,
            passthrough_get=passthrough_get,
            passthrough_set=passthrough_set,
            is_root=len(roots) == 0,
            is_variable_root=len(roots) > 1,
            is_final=True,
        )(cls)
        cls.__is_stored__ = stored
        cls.__is_stored_value_unraveled__ = stored_value_unraveled

        if node_type.is_global:
            cls.__area__ = NodeArea.GLOBAL
        elif node_type.is_regional:
            cls.__area__ = NodeArea.REGIONAL
        elif node_type.is_local:
            cls.__area__ = NodeArea.LOCAL
        else:
            raise ValueError(f"unknown node store for {node_type}")

        cls.__indexes__ = index

        cls.__roots__ = bittuple(*roots, enum_cls=NodeType)
        cls.__is_in_package__ = in_package
        cls.__is_in_bench__ = in_bench

        if has_subtypes:
            cls.__has_subtypes__ = True
            cls.__subtype_base_property__ = cls.__properties__["type"]

        return cls

    return decorate


@dataclass_transform(kw_only_default=True, field_specifiers=_PROPERTY_SPECIFIERS)
def subnode_(
    subtype: BuiltinEnum,
    passthrough_get: str | tuple[str, ...] | None = None,
    passthrough_set: str | tuple[str, ...] | None = None,
):
    """
    Mark a class as a subtype class of an ancestor node class.
    """

    def decorate(cls: type["Node"]) -> type["Node"]:
        for c in cls.__mro__[::-1]:
            if getattr(c, "metatype", None):
                base_cls = cast(type["Node"], c)
                assert issubclass(base_cls, Node), f"expected Node, got {base_cls} for {cls}"
                break
        else:
            raise RuntimeError(f"no base class found for {cls}")
        cls = node_component_(
            node_type=base_cls.metatype,
            passthrough_get=passthrough_get,
            passthrough_set=passthrough_set,
            is_root=len(base_cls.__roots__) == 0,
            is_variable_root=len(base_cls.__roots__) > 1,
            is_final=True,
            is_subtype=True,
        )(cls)

        # cannot instantiate node subtypes directly (only through base class)
        def _fail_init(self, *args, **kwargs):
            raise TypeError(f"cannot instantiate node subtype {cls} directly")

        cls.__init__ = _fail_init

        # check
        if IS_DEV:
            # check name is good
            target_name = f"{to_casing(subtype.name, Casing.CAMEL)}{base_cls.__name__}"
            assert cls.__name__ == target_name, f"expected {target_name}, got {cls.__name__}"

            # check properties
            for prop in cls.__declared_properties__.values():
                if prop.reference_kind == ReferenceKind.NODE_CHILDREN:
                    raise ValueError(f"can't have children {prop!r} in subtype {cls}")
                if prop.id is not None and prop.id < 100:
                    raise ValueError(f"shouldn't have id < 100 {prop!r} in subtype {cls}")

        # register
        cls.__base_class__ = base_cls
        if subtype in base_cls.__subclass_by_subtype__:
            raise ValueError(f"subtype {subtype} already registered for {base_cls}")
        base_cls.__subclass_by_subtype__[subtype] = cls
        base_cls.__subtype_by_subclass__[cls] = subtype
        assert base_cls.__has_subtypes__, f"bad base type {cls!r}"
        assert base_cls.__subtype_base_property__ is not None, f"bad base type {cls!r}"
        cls.__subtype__ = subtype
        subtype_extra_properties = {
            p.name: p for p in cls.__properties__.values() if p.name not in base_cls.__properties__
        }
        cls.__subtype_extra_properties__ = frozendict(subtype_extra_properties)
        cls.__subtype_extra_original_properties__ = frozendict(
            {p.name: p for p in subtype_extra_properties.values() if p.reference_source is None}
        )

        # add subtype key to properties
        for prop in cls.__subtype_extra_properties__.values():
            if prop.subtype_key is None and type(prop.key) is str:
                prop.subtype_key = f"{subtype.value}.{prop.key}"

        return cls

    return decorate


@dataclass_transform(kw_only_default=True, field_specifiers=_PROPERTY_SPECIFIERS)
def timed_node_(
    node_type: NodeType,
    passthrough_get: str | tuple[str, ...] | None = None,
    passthrough_set: str | tuple[str, ...] | None = None,
    index: tuple[IndexIn, ...] = (),
    has_subtypes: bool = False,
):
    """Register a class as a concrete node for the given node type."""
    return node_(
        node_type=node_type,
        passthrough_get=passthrough_get,
        passthrough_set=passthrough_set,
        index=(*index, IndexIn(columns=("created_at",))),
        has_subtypes=has_subtypes,
    )


@node_component_()
class Node[NodeDataT: AnyNodeData](BuiltinObject[NodeDataT], abc.ABC):
    """
    A Node with properties and an identity.
    Conceptually, all Nodes live together happily in a single giant supergraph.
    In practice, there are multiple stores and we load smaller subgraphs at runtime.
    """

    # NOTE :Architecture!: we may need a better :NodeInheritance mechanism since some
    #  nodes have different subtypes with varying properties (e.g. View).
    # Mapping their union into database columns is annoying since there may be many,
    #  so maybe this has to wait until we have custom storage engines.

    metatype: ClassVar[NodeType]  # type: ignore

    # NOTE :Test: make id factories deterministic (incl. UUIDT? somehow)
    __id_factory__: ClassVar[Callable[[], UUID]] = uuid4

    __node_child_properties__: ClassVar[dict[str, Property]] = frozendict()
    __node_child_properties_by_type__: ClassVar[dict[NodeType, list[Property]]] = frozendict()
    __node_ancestor_properties__: ClassVar[dict[str, Property]] = frozendict()
    __subclass_by_subtype__: ClassVar[dict[BuiltinEnum, type["Node"]]] = frozendict()
    __subtype_by_subclass__: ClassVar[dict[type["Node"], BuiltinEnum]] = frozendict()
    __has_subtypes__: ClassVar[bool] = False
    __base_class__: ClassVar[type["Node"] | None] = None
    __subtype_base_property__: ClassVar["Property | None"] = None
    __subtype__: ClassVar[BuiltinEnum | None] = None
    __subtype_extra_properties__: ClassVar[dict[str, Property]] = frozendict()
    __subtype_extra_original_properties__: ClassVar[dict[str, Property]] = frozendict()

    __roots__: ClassVar[bittuple[NodeType]] = UNSET
    __is_struct__: ClassVar[bool] = False
    __is_node__: ClassVar[bool] = True
    __is_in_bench__: ClassVar[bool] = UNSET  # part of a Bench
    __is_in_package__: ClassVar[bool] = UNSET  # part of a Package
    __is_stored__: ClassVar[bool] = False  # stored in primary store (runtime or local)
    __is_stored_value_unraveled__: ClassVar[bool] = False  # custom storage logic (for records)
    __area__: ClassVar[NodeArea]
    __indexes__: ClassVar[tuple[IndexIn, ...]] = ()

    # 1-9: node identity
    # Node.metatype: 1
    id: UUID = p_system(2, default=None, require=True, autoset=True)
    # SourceNode.ck: 3
    parent: Optional["Node"] = p_node_parent(4)  # type: ignore
    if TYPE_CHECKING:
        parent_type: NodeType | None = None
        parent_id: Optional[UUID] = None
        parent_ck: Optional[UUID] = None
        parent_ptr: Optional[NodeReference] = None
    # BenchNode.bench: 5
    # PackageNode.package: 6
    # AuthNode.organization/team/user: 7-9

    # 10-29: node tracking
    created_at: datetime = p_system(10, default=None, require=True, autoset=True)
    created_by: Optional[Subject] = p_system(  # type: ignore (pyright is wrong, Subject is a type)
        11,
        default=None,
        require=False,
        array=False,
        autoset=True,
        references=SUBJECT_NODE_TYPES.tuple,
        same_bench=True,
        baseless=True,
        ckless=True,
    )
    updated_at: datetime = p_system(12, default=None, require=True, autoset=True)
    updated_by: Optional[Subject] = p_system(  # type: ignore (see above)
        13,
        default=None,
        require=False,
        array=False,
        autoset=True,
        references=SUBJECT_NODE_TYPES.tuple,
        same_bench=True,
        baseless=True,
        ckless=True,
    )
    archived_at: Optional[datetime] = p_system(14, default=None, autoset=True)
    deleted_at: Optional[datetime] = p_system(15, default=None, autoset=True)
    # IsTemplatable.template/template_at: 16
    # IsOwnable.owned_by: 17
    # IsClaimable.claimed_by: 18
    # ...managed_by?
    if TYPE_CHECKING:
        created_by_id: Optional[UUID] = None
        created_by_type: NodeType | None = None
        created_by_ptr: Optional[NodeReference] = None
        updated_by_id: Optional[UUID] = None
        updated_by_type: NodeType | None = None
        updated_by_ptr: Optional[NodeReference] = None
    # IsModal.mode: 20

    # NOTE :Architecture! :PolyViews: remove subnode_packed once we have :PolyViews
    subnode_packed: dict[str, dict[str, Any]] | None = p_subnode_packed(29)

    # 30-89 for general node/struct properties
    # ...

    _graph: "NodeGraph" = p_runtime(default=None)
    _link: "NodeLink | None" = p_runtime(default=None)
    _connection: "GetConnection | SearchConnection" = p_runtime(default=None)
    _is_new: bool = p_runtime(default=False)

    if TYPE_CHECKING:
        _skip_add_self: bool = False

    def __init__(
        self, *, _skip_add_self: bool = False, _skip_validate_self: bool = False, **kwargs
    ):
        # inject :Tracing context
        cls = type(self)
        if isinstance(self, IsModal):
            if not kwargs.get("mode"):
                session = ACTIVE_SESSION.get()
                if session is not None and session._runtime is not None:
                    kwargs["mode"] = session._runtime.active_mode

        # init object
        super().__init__(**kwargs, _skip_validate_self=True, _skip_extra_kwargs=True)

        # init node
        if isinstance(self, IsInstantiable):
            if self.ck is None:
                self.id = cls.__id_factory__()
                self.ck = self.id  # type: ignore
                self._is_new = True
            elif self.id is None:
                self.id = cls.__id_factory__()
                self._is_new = True
        elif self.id is None:
            self.id = cls.__id_factory__()
            self._is_new = True
        if self.created_at is None:
            if self._session is None and self.metatype == NodeType.SESSION:
                # 'bootstrap' session with itself
                self._session = cast("Session", self)
            assert self._session is not None, f"{self!r} is not in a session"
            now = self._session._oracle.utc()
            self.created_at = now
            self.updated_at = now

        # init graph (nodes must always be in a non-null supergraph & graph)
        assert self._supergraph is not NULL_SUPERGRAPH, f"no supergraph for {self!r}"
        if self._graph is not None:
            pass  # use given graph
        elif self.parent_ptr is not None:
            # use parents graph
            parent = self.parent
            assert (
                parent is not None
            ), f"parent for {type(self).__name__} not in {self._supergraph!r}: {self.parent_ptr!r}"
            if self.metatype in parent._graph.node_types:
                self._graph = parent._graph
            else:
                # have parent graph but it's not the right one :IsolatedGraph
                from bench.language.connection import capture

                self._graph = NodeGraph(
                    scope=parent._graph.scope,
                    node_types=(self.metatype, *DESCENDANT_NODE_TYPES[self.metatype]),
                    supergraph=self._supergraph,
                )
                self._supergraph.add_graph(self._graph)
                capture(self._graph)
        else:
            # no parent, create our own graph
            # if we're not in a graph, start a new one :IsolatedGraph
            from bench.language.connection import capture

            graph = NodeGraph(  # type: ignore
                scope=EMPTY_SCOPE_DATA,
                node_types=(self.metatype, *DESCENDANT_NODE_TYPES[self.metatype]),
                supergraph=self._supergraph,
            )
            self._graph = graph
            self._supergraph.add_graph(graph)
            capture(graph)
        if not _skip_add_self:
            self._graph.add(self)

        # init node lists (preserving existing)
        for name, prop in self.__node_child_properties__.items():
            assert prop.reference_list_type is not None
            node_list = prop.reference_list_type(self, prop)
            self.__dict__[name] = node_list
            if (existing := kwargs.get(name, UNSET)) is not UNSET:
                node_list.extend(*existing)

        # parse extraneous kwargs
        self._init_extra_kwargs(kwargs)

        # init session context
        if self._session is not None:
            if self._is_new and not _skip_validate_self:
                self._validate_self((), invalid=on_invalid_raise)
            self._track_self(self._session)

    @dualmethod
    def get_property(self, key: str) -> Property:  # type: ignore
        """Get a property by key from this instance."""
        prop = self._get_effective_cls().__properties__.get(key)
        if prop is None:
            raise ValueError(f"no property '{key}' in {self.__class__.__name__}")
        return prop

    @get_property.cls
    def get_property_cls(cls, key: str) -> Property:  # type: ignore  # noqa: N805
        """Get a property by key from the class."""
        prop = cls.__properties__.get(key)
        if prop is None:
            raise ValueError(f"no property '{key}' in {cls.__name__}")
        return prop

    @override
    def _get_effective_cls(self) -> type["Node"]:
        # extend BuiltinObject to point to subtype class if we have one
        if self.__has_subtypes__:
            subtype = self.__dict__["type"]
            subtype_cls = self.__subclass_by_subtype__.get(subtype)
            if subtype_cls is not None:
                return subtype_cls
        return type(self)

    @override
    def _init_extra_kwargs(self, kwargs: dict[str, Any]):
        # extend BuiltinObject to initialise subtype properties
        super()._init_extra_kwargs(kwargs)
        if self.__has_subtypes__:
            cls = self._get_effective_cls()
            subtype_key = str(self.__dict__["type"])
            if self.subnode_packed is None:
                subnode_packed = EMPTY_DICT
            else:
                subnode_packed = self.subnode_packed.get(subtype_key) or EMPTY_DICT
            for prop in cls.__subtype_extra_properties__.values():
                prop_value = kwargs.get(prop.name, UNSET)
                if prop_value is UNSET:
                    if prop.key in subnode_packed:
                        continue  # already set directly
                    if prop.default_factory is not None:
                        prop_value = prop.default_factory()
                    elif prop.default is not UNSET:
                        if prop.default is None:
                            continue  # skip optional None
                        prop_value = prop.default
                    elif not prop.is_required:
                        if prop_value.is_list:
                            prop_value = []
                        else:
                            continue  # skip optional None
                    else:
                        raise ValueError(f"missing required property: {prop!r}")
                self._do_set(prop.name, prop_value, track=False)

    def __default_content_str__(self) -> str:
        """Default __content_str__ for Nodes with all set properties (incl. subtypes)."""
        value_strs = []
        properties = self.__declared_properties__.values()
        if self.__has_subtypes__:
            subtype = self.__dict__["type"]
            subtype_cls = self.__subclass_by_subtype__.get(subtype)
            if subtype_cls is not None:
                properties = chain(properties, subtype_cls.__subtype_extra_properties__.values())
        for prop in properties:
            if (
                prop.id is UNSET
                or prop.id is None
                or prop.reference_kind == ReferenceKind.NODE_CHILDREN
                or prop.is_sensitive
                or prop.id < 30
                or prop.name in ("type", "name", "order_key")
            ):
                continue
            prop_value = getattr(self, prop.name)
            if (
                prop_value is None
                or (isinstance(prop_value, Sequence) and not prop_value)
                or (
                    not isinstance(prop.default, BuiltinObject)
                    and type(prop_value) is type(prop.default)
                    and prop_value == prop.default
                )
            ):
                continue
            elif prop.is_enum:
                if prop.is_list:
                    prop_value_str = "|".join(p.bench_name for p in prop_value)
                else:
                    prop_value_str = prop_value.bench_name  # type: ignore
            elif isinstance(prop_value, Node):
                prop_value_str = f"<{prop_value._ident_key} ...>"
            elif (
                isinstance(prop_value, (list, tuple))
                and prop_value
                and isinstance(prop_value[0], Node)
            ):
                prop_value_str = f"[{', '.join(f'<{node.ident_str} ...>' for node in prop_value)}]"
            else:
                prop_value_str = repr(prop_value)
            value_strs.append(f"{prop.name}={prop_value_str}")
        return ", ".join(value_strs)

    @final
    def __str__(self):  # type: ignore
        # override the default __str__ for nodes
        content_str = self.__content_str__()
        if content_str:
            content_str = f" ({content_str})"
        if self.deleted_at is not None:
            status_str = " [deleted]"
        elif self.archived_at is not None:
            status_str = " [archived]"
        else:
            status_str = ""
        return f"{self._ident_key}{content_str}{status_str}"

    @final
    def __repr__(self):  # type: ignore
        # override the default __repr__ for nodes
        if self.__has_subtypes__:
            subtype = self.__dict__["type"]
            return f"<{subtype.bench_name}{self.__class__.__name__} {self!s}>"
        else:
            return f"<{self.__class__.__name__} {self!s}>"

    @property
    def ck(self):
        return self.id

    @property
    def is_attached(self) -> bool:
        """
        Whether this Node is attached to a roots.
        TODO :Performance: Node.is_attached is very inefficient
        """
        if not self.__roots__.bits.any():
            return True  # always attached
        parent = self
        while parent is not None:
            if parent.metatype in self.__roots__:
                return True
            parent = parent.parent
        return False

    def iter_descendants(self, recursive: bool = False) -> Iterable["Node"]:
        """Iterate over all descendants of this node."""
        for child_type in CHILD_NODE_TYPES[self.metatype]:
            for child in self._graph.iter_descendants(self, child_type):
                yield child
                if recursive:
                    yield from child.iter_descendants(recursive=True)

    @override
    def clone(
        self,
        *,
        reset: bool = True,
        recursive: bool = True,
        detach: bool = False,
        _map: bool | dict[UUID, "Node"] = True,
        _ignore_definition: bool = False,
        _is_nested: bool = False,
        **kwargs,
    ) -> Self:
        from bench.language import Block

        # clone self
        copy_kwargs = self._clone_kwargs(reset=reset)
        if isinstance(self, IsModal):
            copy_kwargs["mode"] = self.mode  # keep mode
        copy_kwargs.update(kwargs)
        if detach and not reset:
            # put the node in a new graph to isolate (because same ids)
            copy_kwargs["_graph"] = NodeGraph(
                scope=self._graph.scope,
                node_types=self._graph.node_types,
                supergraph=active_session()._supergraph,
            )
        clone = self.__class__(**copy_kwargs, _is_new=True)

        # remember new identities
        if _map is True:
            _map = {self.id: clone}
        elif _map is not False:
            _map[self.id] = clone

        # clone blocks/definitions together
        if not _ignore_definition:
            clone_parent = clone.parent
            if clone_parent is None and not detach:
                clone_parent = self.parent
            if isinstance(self, Block):
                if isinstance(node := self.node, InlineNode) and node.definition_id == self.id:
                    assert clone_parent is not None, f"cannot clone detached {self!r}"
                    cloned_node = node.clone(
                        reset=reset,
                        recursive=True,
                        detach=True,
                        _map=_map,
                        _is_nested=True,
                        _ignore_definition=True,
                    )
                    cast(Block, clone).node_ptr = cloned_node.to_ref()
                    cloned_node.definition_ptr = clone.to_ref()
                    attach_node(cloned_node, clone_parent, clone_parent._graph, create=False)
            elif isinstance(self, InlineNode) and (definition := self.definition) is not None:
                assert clone_parent is not None, f"cannot clone detached {self!r}"
                cloned_node = definition.clone(
                    reset=reset,
                    recursive=True,
                    detach=True,
                    _map=_map,
                    _is_nested=True,
                    _ignore_definition=True,
                )
                cast(InlineNode, clone).definition_ptr = cloned_node.to_ref()
                cloned_node.node_ptr = clone.to_ref()
                attach_node(cloned_node, clone_parent, clone_parent._graph, create=False)

        # clone children and append to self (recursive)
        if recursive:
            for child_type in CHILD_NODE_TYPES[self.metatype]:
                for child in self._graph.iter_descendants(self, child_type):
                    child_clone = child.clone(
                        reset=reset,
                        recursive=True,
                        detach=True,
                        _map=_map,
                        _is_nested=True,
                        _ignore_definition=True,  # we're the parent, so we clone both
                    )
                    attach_node(child_clone, clone, clone._graph, create=False)  # re-attach

        # map new identities (at root)
        if not _is_nested and type(_map) is dict:
            for node in _map.values():
                node.replace_references(
                    _map,
                    exclude=(ReferenceKind.NODE_PARENT, ReferenceKind.NODE_CHILDREN),
                )

        # append to our parent to re-attach
        parent = self.parent
        if detach:
            clone.parent_ptr = None
        elif parent:
            parent.append(clone)
        return clone

    def __eq__(self, other: Any):
        """Equals the Node's identity."""
        return self is other or (type(self) is type(other) and (self.id == other.id))

    def _stable_hash(self):
        """Hash the Node's identity."""
        return hash_stable((self.metatype, self.id))

    # only define __hash__ for nodes since their id is constant
    __hash__ = _stable_hash  # type: ignore

    @override
    def equals(
        self, other: Self | Any, identity_map: Mapping[UUID, "NodeReference"] = EMPTY_DICT
    ) -> bool:
        if not super().equals(other, identity_map):
            return False
        if self.__has_subtypes__:
            # compare node subtype
            subtype = self.__dict__["type"]
            subtype_cls = self.__subclass_by_subtype__.get(subtype)
            if subtype_cls is not None:
                self_subnode_packed = self.__dict__.get("subnode_packed") or EMPTY_DICT
                self_subnode = self_subnode_packed.get(str(subtype)) or EMPTY_DICT
                other_subnode_packed = other.__dict__.get("subnode_packed") or EMPTY_DICT
                other_subnode = other_subnode_packed.get(str(subtype)) or EMPTY_DICT
                for prop in subtype_cls.__subtype_extra_original_properties__.values():
                    self_value = self_subnode.get(prop.name)
                    other_value = other_subnode.get(prop.name)
                    if prop._type_info is not None and not value_equals(
                        prop._type_info, self_value, other_value, identity_map
                    ):
                        return False
        return True

    @override
    def _do_get(self, key: str):
        """Called if an attribute doesn't exist in __dict__ / the usual places."""

        if self.__has_subtypes__:
            subtype = self.__dict__["type"]
            subtype_cls = self.__subclass_by_subtype__.get(subtype)
            if subtype_cls is not None:
                # short-circuit to subclass property or method if it exists
                subtype_attr = getattr(subtype_cls, key, None)
                if subtype_attr is not None:
                    if type(subtype_attr) is property and subtype_attr.fget is not None:
                        # it's a (Python) property
                        return subtype_attr.fget(self)
                    elif type(subtype_attr) is not Property and callable(subtype_attr):
                        # it's a method
                        return functools.partial(subtype_attr, self)

                # regular subtype property
                prop = subtype_cls.__properties__.get(key)
                if prop is not None:
                    subtype_key = str(subtype)
                    subnode_packed = self.__dict__.get("subnode_packed", EMPTY_DICT)
                    if subnode_packed is not None:
                        subnode_packed = subnode_packed.get(subtype_key)
                        if subnode_packed is not None:
                            subnode_value = subnode_packed.get(prop.key)
                            if subnode_value is not None:
                                return subnode_value
                    if prop.is_list:
                        return ()
                    else:
                        return None
                else:
                    # check subtype passthrough
                    if subtype_cls.__passthrough_get__ is not None and self._session is not None:
                        for passthrough_key in subtype_cls.__passthrough_get__:
                            target = getattr(self, passthrough_key, UNSET)
                            attr = getattr(target, key, UNSET)
                            if attr is not UNSET:
                                return attr

        # check passthrough (if in a session)
        if self.__passthrough_get__ is not None and self._session is not None:
            for passthrough_key in self.__passthrough_get__:
                target = getattr(self, passthrough_key, UNSET)
                attr = getattr(target, key, UNSET)
                if attr is not UNSET:
                    return attr

        # attribute error
        try:
            self_str = repr(self)
        except Exception:
            self_str = self.__class__.__name__
        raise AttributeError(f"{self_str} has no attribute '{key}'")

    @override
    def _do_set(self, key: str, new_value: Any, *, track: bool = True, validate: bool = False):
        """Sets *any* attribute on this builtin object."""
        if self.__has_subtypes__ and key not in self.__properties__:
            # set subtype property
            subtype = self.__dict__["type"]
            subtype_cls = self.__subclass_by_subtype__.get(subtype)
            if subtype_cls is not None:
                subtype_prop = getattr(subtype_cls, key, None)

                # set regular subtype property
                prop = subtype_cls.__properties__.get(key)
                if prop is not None:
                    # move value
                    if prop.reference_kind == ReferenceKind.STRUCT_CHILD:
                        if not prop.is_list:
                            if new_value is not None:
                                new_value = new_value._move_to(self, prop)
                        else:
                            new_value = [v._move_to(self, prop) for v in new_value]

                    # coerce & check type
                    if prop._type_info is not None and prop.reference_source is None:
                        if IS_IN_USER_CODE.get():
                            new_value = coerce_value(new_value, prop._type_info)
                            check_value(
                                new_value,
                                prop._type_info,
                                options=DEFAULT_CHECK_OPTIONS,
                                invalid=on_invalid_raise,
                            )
                        elif validate:
                            check_value(
                                new_value,
                                prop._type_info,
                                options=DEFAULT_CHECK_OPTIONS,
                                invalid=on_invalid_raise,
                            )

                    # set
                    # short-circuit to subclass 'property' if it exists
                    subtype_key = str(subtype)
                    if type(subtype_prop) is property and subtype_prop.fset is not None:
                        assert subtype_prop.fget is not None
                        if prop.is_value_runtime:
                            old_value = getattr(self, prop.value_packed_ptr.name)  # type: ignore
                        else:
                            old_value = subtype_prop.fget(self)
                        subtype_prop.fset(self, new_value)
                    elif self.subnode_packed is None:
                        old_value = None
                        # set directly
                        self.__dict__["subnode_packed"] = {subtype_key: {prop.key: new_value}}
                    elif subtype_key not in self.subnode_packed:
                        old_value = None
                        self.subnode_packed[subtype_key] = {prop.key: new_value}
                    else:
                        old_value = getattr(self, key)
                        self.subnode_packed[subtype_key][prop.key] = new_value

                    # track
                    if track:
                        if prop.is_value_runtime:
                            _trace_edit_operation(
                                self,
                                prop.value_packed_ptr,  # type: ignore
                                new_value=getattr(self, prop.value_packed_ptr.name),  # type: ignore
                                old_value=old_value,
                                subtype=subtype,
                            )
                        else:
                            _trace_edit_operation(
                                self,
                                prop,
                                new_value=new_value,
                                old_value=old_value,
                                subtype=subtype,
                            )

                    return  # success

        super()._do_set(key, new_value, track=track)

    if not TYPE_CHECKING:  # (see above in BuiltinObject)
        __getattr__ = _do_get
        __setattr__ = _do_set

    @property
    def connection(self):
        """The currently active connection (errors if none)"""
        assert self._connection is not None, f"no connection for {self!r}"
        return self._connection  # type: ignore (class definition "depends on itself" for some reason)

    @property
    def _is_live(self) -> bool:
        """Whether this Node is live."""
        return (
            self._connection is not None
            and self._connection.is_live
            and not self._connection._is_closed
        ) or (self._link is not None and self._link.is_active)

    @property
    def _data_graph(self) -> "NodeDataGraph":
        assert self._connection is not None, f"no connection for {self!r}"
        assert self._connection.result_data is not None, f"no data graph for {self!r}"
        return self._connection.result_data.graph

    @final
    def _track_self(self, session: "Session"):
        """Track this object in the given session."""
        if self._session is not None and self._session is not session:
            raise RuntimeError(f"{self!r} is already in {self._session!r}, not {session!r}")
        self._session = session

    @final
    def _untrack_self(self) -> None:
        """Stop tracking this object."""
        self._session = None

    @final
    def _walk_ancestors(self) -> Iterable["Node"]:
        current = self.parent
        while current is not None:
            yield current
            current = current.parent

    @final
    def _walk_descendants(self) -> Iterable["Node"]:
        yield self
        if self.metatype in HAS_CHILD_NODE_TYPES:
            yield from self._graph.get_descendants(self, recursive=True)

    @final
    def _track_rec(self, session: "Session"):
        for inner_node in self._walk_descendants():
            inner_node._track_self(session)

    @final
    def _untrack_rec(self):
        for inner_node in self._walk_descendants():
            inner_node._untrack_self()

    @property
    def _ident(self) -> Optional[str]:
        """The Bench identifier of this node (slug if exists, else name if exists)."""
        if "slug" in self.__properties__:
            slug = getattr(self, "slug")
            if slug:  # prefer slug as ident
                return slug
        if "name" in self.__properties__:
            return getattr(self, "name")
        if "title" in self.__properties__:
            title = getattr(self, "title")
            if type(title) is str:
                return title
            elif title is not None:
                return cast("TextLine", title).to_plain()
        return None

    @property
    def _path_key(self) -> str:
        """The Bench *path* identifier of this node (prefers bench_ident, ck/id filter otherwise)"""
        bench_ident = self._ident
        if bench_ident is not None:
            if self.metatype == NodeType.BENCH:
                return f"@{bench_ident}"
            else:
                return bench_ident
        else:
            return f"{self.metatype.bench_name}[id={self.id}]"

    @property
    def _ident_key(self) -> str:
        """Get the identifier string for this node."""
        ident_str = self.code_name
        if ident_str is None:
            ident_str = str(self.id)
        if self.__parent_property__ is not None:
            return self.absolute_path
        return ident_str

    @property
    def code_name(self) -> Optional[str]:
        """The python-compatible identifier of this node."""
        if "slug" in self.__properties__:
            slug = getattr(self, "slug", None)
            if slug:  # prefer slug as ident
                return slug
        if "name" in self.__properties__:
            name = getattr(self, "name", None)
            if name:
                return to_code_name(name)
        if "title" in self.__properties__:
            title = getattr(self, "title", None)
            if title is not None:
                if type(title) is str and title:
                    return to_code_name(title)
                else:
                    return to_code_name(cast("TextLine", title).to_plain())
        return None

    @property
    def absolute_path(self) -> str:
        if not self.__parent_types__:
            # this is a root node
            ident = self._ident
            assert ident is not None, f"no identifier for {self!r}"
            return ident
        else:
            # assemble path (like in Path.render)
            path_parts: list[str] = []
            current = self
            while current is not None:
                if current.metatype == NodeType.PACKAGE:
                    bench = cast("Bench | Package", current).bench
                    if bench is not None:
                        path_parts.append(f"{bench._path_key}:{current._path_key}")
                        break

                path_parts.append(current._path_key)
                next_parent = current.parent
                if next_parent is None and current.metatype in self.__roots__:
                    break  # reached the root
                current = next_parent
            else:
                path_parts.append("<detached>")
            return "/".join(reversed(path_parts))

    def to_ref(self) -> "NodeReference":
        """Gets a reference to this node. May be rich in subclasses."""
        return NodeReference._ref_from_node(self)

    def _to_ref_data(self) -> "NodeReferenceData":
        """Gets a data reference to this node. May be rich in subclasses."""
        return NodeReference._ref_from_node(self)._to_data()

    #
    # Lifecycle
    #

    @property
    def is_extant(self):
        return self.deleted_at is None and self.archived_at is None

    @property
    def is_active(self) -> bool:
        return self.deleted_at is None and self.archived_at is None

    @property
    def is_archived(self) -> bool:
        return self.archived_at is not None or (
            (parent := self.parent) is not None and parent.is_archived
        )

    @property
    def is_deleted(self) -> bool:
        return self.deleted_at is not None or (
            (parent := self.parent) is not None and parent.is_deleted
        )

    def archive(self, _now: datetime | None = None):
        """Archive this Node."""
        assert not self.archived_at, f"{self!r} is already archived"
        self.active_session._archive(self, _now=_now)

    def unarchive(self, _now: datetime | None = None):
        """Unarchive this Node."""
        assert self.archived_at, f"{self!r} is not archived"
        self.active_session._unarchive(self, _now=_now)

    def delete(self, _now: datetime | None = None):
        """Delete this Node."""
        assert not self.deleted_at, f"{self!r} is already deleted"
        self.active_session._delete(self, _now=_now)

    def restore(self, _now: datetime | None = None):
        """Restore this deleted Node from the trash."""
        assert self.deleted_at, f"{self!r} is not deleted"
        self.active_session._restore(self, _now=_now)

    def erase(self):
        """Wipe this Node from this cosmos forever."""
        self.active_session._erase(self)

    def move(self, to: "Node"):
        """Move this Node to a new parent."""
        to.append(self, move=True)

    def append[T: Node](self, child: T, move: bool = False) -> T:
        """Append a Node as a child of this Node."""
        child_prop = self.get_child_property(child.metatype)
        if child_prop is not None:
            child_list = getattr(self, child_prop.name)
            child_list.append(child, move=move)
        elif self.metatype in child.__parent_types__:
            if child.metatype in self._graph.node_types:
                graph = self._graph
            else:
                # have parent graph but it's not the right one :IsolatedGraph
                graph = NodeGraph(
                    scope=self._graph.scope,
                    node_types=(child.metatype, *DESCENDANT_NODE_TYPES[child.metatype]),
                    supergraph=self._supergraph,
                )
                self._supergraph.add_graph(graph)
            attach_node(child, self, move=move, graph=graph)
        else:
            raise ValueError(f"cannot append {child!r} to {self!r}")
        return child

    def extend(self, *children: "Node", move: bool = False):
        """Append multiple Nodes as children of this Node."""
        for child in children:
            self.append(child, move=move)

    def _move_to_graph(self, graph: NodeGraph):
        """Moves this Node and its descendants to a new graph."""
        moved = self._graph.get_descendants(self, recursive=True)  # type: ignore
        moved = (self, *moved)
        for n in moved:
            # add/update in new graph
            if n.id in graph._nodes_by_id:
                graph.update(n)
            else:
                graph.add(n)
            # remove from old graph
            if n._graph is not graph and n.id in n._graph._nodes_by_id:
                n._graph.remove(n)
            n._graph = graph
        return moved

    def _detach_rec(self):
        """Removes this node from the graph / supergraph."""
        self._graph.remove(self)  # type: ignore ("depends on itself")
        if len(self._graph) == 0:
            from bench.language.connection import uncapture

            # remove entire graph from supergraph if it was just this node (and its descendants)
            self._supergraph.remove_graph(self._graph)
            uncapture(self._graph)

    async def wait_until(self, condition: Callable[[Self], bool], timeout: timedelta | None = None):
        """Wait until the given condition is true."""
        runtime = active_session().runtime
        await runtime.wait_for(nodes=[self], condition=lambda: condition(self), timeout=timeout)

    @classmethod
    def get_child_property_or_error(cls, node_type: NodeType) -> Property:
        """Gets the child property for the given Node type."""
        prop = cls.get_child_property(node_type)
        if prop is None:
            raise ValueError(f"no child property for {node_type.bench_name} in {cls.__name__}")
        return prop

    @classmethod
    def get_child_property(cls, node_type: NodeType) -> Property | None:
        """Gets the child property for the given Node type, or None if not found."""
        for prop in cls.__node_child_properties__.values():
            if (
                prop.reference_nodes
                and prop.reference_nodes != "any"
                and node_type in prop.reference_nodes
            ):
                return prop
        return None

    @classmethod
    def partial_type(
        cls,
        type: int | None = None,
        *,
        base_type: "TypeBaseNode | None" = None,
        field_types: list[FieldType] | None = None,
    ) -> "Type":
        """Creates a Type object for a partial Node."""
        from bench.language.core import Type, TypeConstraint

        constraint = TypeConstraint(node_subtypes=[type]) if type is not None else None
        metatype = getattr(cls, "metatype", None)  # Node has no metatype

        if base_type is not None:
            return Type(
                kind=TypeKind.PARTIAL_OBJECT,
                base_type=base_type,
                base_field_types=field_types or [FieldType.MEMBER],
                property_field_types=field_types or [],
                bench_type=metatype,
                constraint=constraint,
            )
        else:
            field_types = field_types or []
            return Type(
                kind=TypeKind.PARTIAL_OBJECT,
                bench_type=metatype,
                base_field_types=field_types,
                property_field_types=field_types,
                constraint=constraint,
            )

    @classmethod
    def partial(
        cls,
        type: int | None = None,
        *,
        base_type: "TypeBaseNode | None" = None,
        field_types: list[FieldType] | None = None,
        **kwargs: Any,
    ) -> "CustomObject":
        """Creates a new partial Node of this type."""
        from .trait import IsBased
        from .value import coerce_custom_object_scalar

        if base_type is None and issubclass(cls, IsBased):
            base_type = cast(TypeBaseNode, cls.get_base_from_partial(kwargs))

        typ = cls.partial_type(type, base_type=base_type, field_types=field_types)
        if cls is not Node:
            kwargs["metatype"] = cls.metatype
        if type is not None:
            kwargs["type"] = type
        return coerce_custom_object_scalar(kwargs, typ)

    @classmethod
    def from_partial(cls, partial: "CustomObject", **kwargs) -> Self:
        """Creates a new full Node from a partial Node."""
        from .value import make_node_from_partial

        node = make_node_from_partial(partial, **kwargs)
        assert isinstance(node, cls), f"unexpected node {node!r} from partial {partial!r}"
        return node

    #
    # Querying
    #

    @classmethod
    def _query(cls) -> "Query[Self, NodeDataT]":
        from bench.language import Query

        return Query(type=QueryType.SEARCH, node_type=cls.metatype)

    @classmethod
    def where(cls, filter: Optional["Expression"] = None, **kwargs) -> "Query[Self, NodeDataT]":
        return cls._query().where(filter, **kwargs)

    @classmethod
    def order_by(
        cls, sort: "Optional[Expression] | str | Field | Property" = None, *args: str
    ) -> "Query[Self, NodeDataT]":
        return cls._query().order_by(sort, *args)

    @classmethod
    def include(cls, *properties: Property) -> "Query[Self, NodeDataT]":
        return cls._query().include(*properties)

    @classmethod
    def select(cls, *keys: FieldOrProperty) -> "Query[Self, NodeDataT]":
        return cls._query().select(*keys)

    @classmethod
    def select_all(cls) -> "Query[Self, NodeDataT]":
        return cls._query().select_all()

    @classmethod
    def deselect(cls, *properties: FieldOrProperty) -> "Query[Self, NodeDataT]":
        return cls._query().deselect(*properties)

    @classmethod
    def include_ancestors(cls, *node_types: NodeTypeOrClass) -> "Query[Self, NodeDataT]":
        return cls._query().include_ancestors(*node_types)

    @classmethod
    def include_descendants(cls, *node_types: NodeTypeOrClass) -> "Query[Self, NodeDataT]":
        return cls._query().include_descendants(*node_types)

    @overload
    @classmethod
    async def get(
        cls,
        filter: Optional["Expression | NodeReference | None"] = None,
        live: bool = False,
        **kwargs,
    ) -> Self: ...
    @overload
    @classmethod
    async def get(
        cls, filter: Sequence["NodeReference"], live: bool = False, **kwargs
    ) -> list[Self]: ...
    @classmethod
    async def get(
        cls,
        filter: Optional["Expression | NodeReference | Sequence[NodeReference] | None"] = None,
        live: bool = False,
        **kwargs,
    ) -> Self | list[Self]:
        return await cls._query().get(filter, live=live, **kwargs)

    @classmethod
    async def search(cls, filter: Optional["Expression"] = None, **kwargs) -> list[Self]:
        return await cls._query().search(filter, **kwargs)

    @classmethod
    def first(cls, count: int) -> "Query[Self, NodeDataT]":
        return cls._query().first(count)

    @classmethod
    async def count(cls, filter: Optional["Expression"] = None, **kwargs) -> int:
        return await cls._query().count(filter, **kwargs)

    @classmethod
    async def exists(cls, filter: Optional["Expression"] = None, **kwargs) -> bool:
        return await cls._query().exists(filter, **kwargs)


class NodeSubtypeStub[NodeT: Node]:
    """
    The stub for the virtual subclass of a Node for a specific subtype.
    Basically, we use this so we can have instantiate & instance check with subnodes,
     like with Database or TreeView (even when the actual subtype-class doesn't exist).
    """

    __slots__ = ("_name", "_node_cls", "_node_subtype", "_node_type")

    def __init__(self, cls: type[NodeT], type: NodeType, subtype: BuiltinEnum):
        self._node_cls = cls
        self._node_type = type
        self._node_subtype = subtype
        self._name = f"{to_casing(subtype.name, Casing.CAMEL)}{cls.__name__}"

    def __call__(self, **kwargs: Any) -> NodeT:
        return self._node_cls(type=self._node_subtype, **kwargs)


@node_component_()
class BenchNode[NodeDataT: AnyNodeData](Node[NodeDataT], abc.ABC):
    """A Node inside a Bench."""

    bench: "Bench | None" = p_node_ancestor_with_self(
        5, NodeType.BENCH, require=True, store=True, wire=True
    )
    if TYPE_CHECKING:
        bench_id: Optional[UUID] = None
        bench_ptr: Optional[NodeReference] = None

    @property
    def is_attached(self) -> bool:
        return self.parent_ptr is not None and self.bench is not None


@node_component_()
class PackageNode[NodeDataT: AnyNodeData](BenchNode[NodeDataT], abc.ABC):
    """A Node in a Package."""

    package: "Package | None" = p_node_ancestor_with_self(
        9, NodeType.PACKAGE, require=True, store=True, wire=True, is_bench_implicit=True
    )
    if TYPE_CHECKING:
        package_id: Optional[UUID] = None
        package_ptr: Optional[NodeReference] = None


@node_component_()
class InlineNode[NodeDataT: AnyNodeData](PackageNode[NodeDataT]):
    """A Node that can be defined 'inline' on a Block/Page."""

    parent: Union["Page", None] = p_node_parent(4, NodeType.PAGE)
    # name: 31
    # title: 32
    order_key: str | None = p_internal(33, default=INTEGER_ZERO, default_sql=None)
    icon: Optional["Icon"] = p_regular(
        34, default=None, require=False, array=False, struct=StructType.ICON
    )
    definition: "Block | None" = p_regular(
        35,
        require=False,
        array=False,
        references=NodeType.BLOCK,
        same_bench=True,
        description="The Block where this Node is 'defined'.",
    )
    if TYPE_CHECKING:
        definition_id: Optional[UUID] = None
        definition_ck: Optional[UUID] = None
        definition_ptr: Optional[NodeReference] = None

    @property
    def containing_page(self) -> "Page | None":
        """Gets the containing ancestor Page (if any)"""
        from bench.language import Page

        parent = self.parent
        while parent is not None:
            if isinstance(parent, Page):
                return parent
            parent = parent.parent
        return None

    @override
    def delete(self, _now: datetime | None = None):
        super().delete(_now=_now)
        # also delete defining Block (if any)
        if (
            (definition := self.definition) is not None
            and definition.node_id == self.id
            and not definition.is_deleted
        ):
            definition.delete(_now=_now)

    @override
    def restore(self, _now: datetime | None = None):
        super().restore(_now=_now)
        # also restore defining Block (if any)
        if (
            (definition := self.definition) is not None
            and definition.node_id == self.id
            and definition.is_deleted
        ):
            definition.restore(_now=_now)

    def to_block(self) -> "Block":
        """Wrap this Node in a *new* Block."""
        from bench.language import Block

        return Block.wrap(self)


#
# Utility types
#


@struct_(StructType.NODE_REFERENCE)
class NodeReference(Struct[NodeReferenceData]):
    """
    A plain reference to a Node.
    We include the Bench and 'ck' where available.
    Base = the Node is 'based' on (as in HasNodeBase).
    """

    node_type: NodeType = p_internal(30, require=True)
    id: UUID = p_internal(31)
    ck: Optional[UUID] = p_internal(32, default=None)
    bench_id: Optional[UUID] = p_internal(33, default=None)
    base_id: Optional[UUID] = p_internal(34, default=None)

    async def get(self) -> "Node":
        """Gets the Node referenced by this reference."""
        node_cls = NODE_CLASS_BY_TYPE[self.node_type]
        query = node_cls._query()
        query._include_memory = False
        return await query.get(self)

    @staticmethod
    def _clone_ref[T: NodeReference | Any](
        ref_cls: type[T], ref: "NodeReference | Any", **kwargs
    ) -> T:
        return ref_cls(
            id=ref.id,
            ck=ref.ck,
            node_type=ref.node_type,
            bench_id=ref.bench_id,
            base_id=ref.base_id,
            **kwargs,
        )

    def to_ref(self) -> "Self":
        """Gets the reference (noop for compatibility with Node.to_ref)."""
        return self

    def __content_str__(self):
        content_parts = []
        if self.id is not None:
            content_parts.append(f"id={self.id}")
        if self.ck is not None:
            if self.ck == self.id:
                content_parts.append("ck=id")
            else:
                content_parts.append(f"ck={self.ck}")
        if self.bench_id is not None:
            content_parts.append(f"bench_id={self.bench_id}")
        if self.base_id is not None:
            content_parts.append(f"base_id={self.base_id}")
        selector_str = ", ".join(content_parts)
        return f"{self.node_type.bench_name}:[{selector_str}]"

    @staticmethod
    def _ref_from_node(node: Node) -> "NodeReference":
        assert isinstance(node, Node), f"expected Node, got {node!r}"

        # bench
        bench_id: UUID | None = None
        if node.metatype == NodeType.BENCH:
            bench_id = node.id
        elif NodeType.BENCH in node.__roots__:
            bench_id = cast(BenchNode, node).bench_id
            if bench_id is None:
                # maybe just creating, try from context
                session = ACTIVE_SESSION.get()
                if session is not None and session.bench_id is not None:
                    bench_id = session.bench_id

        # base
        base_id: UUID | None = None
        if node.metatype in BASED_NODE_TYPES:
            base_id = cast(IsBased, node).base_id

        reference = NodeReference(
            node_type=node.metatype,
            id=node.id,
            ck=node.ck,
            bench_id=bench_id,
            base_id=base_id,
            _skip_validate_self=True,
        )
        return reference

    @staticmethod
    def _ref_data_from_node_data(node_data: AnyNodeData) -> "NodeReferenceData":
        node_cls = BUILTIN_OBJECT_CLASS_BY_TYPE[cast(ObjectType, node_data.metatype)]
        reference = NodeReferenceData(
            metatype=pb2.OBJECT_TYPE_NODE_REFERENCE,
            node_type=cast(pb2.NodeType, node_data.metatype),
            id=node_data.id,
            ck=getattr(node_data, "ck", node_data.id),
        )

        # bench
        if node_data.metatype == NodeType.BENCH:
            if node_data.id:
                reference.bench_id = node_data.id
        elif "bench" in node_cls.__properties__ and node_data.parent_ptr.bench_id:
            reference.bench_id = node_data.parent_ptr.bench_id
        # base
        if NodeType(node_data.metatype) in BASED_NODE_TYPES:
            base = cast("IsBased", node_cls).get_base_from_data(node_data)
            if base is not None:
                reference.base_id = base.id

        return reference


@node_(NodeType.SKIP, stored=False)
class Skip(Node):
    """A reference to another node in some graph that wasn't available for some reason (usually permissions)."""

    parent: Node = p_node_parent(4, *NODE_TYPES.tuple)
    reference: Optional[Node] = p_regular(30, array=False, references="any", require=True)
    order_key: str | None = p_internal(31, default=None)


@node_(NodeType.EMPTY, stored=False)
class Empty(Node):
    """An empty node."""

    parent: Node = p_node_parent(4, *NODE_TYPES.tuple)


# NOTE: import from .value later to avoid circular import
#  (but import at top level to avoid import in critical path)


from .value import (  # noqa: E402
    DEFAULT_CHECK_OPTIONS,
    check_value,
    coerce_value,
    value_equals,
)


def extract_name_id(name: str) -> Optional[int]:
    """Extracts the last (potentially multi-digit) characters as an integer."""
    for i in range(len(name), 0, -1):
        if not name[i - 1].isdigit():
            return None if i == len(name) else int(name[i:])
    return int(name)


def generate_node_name(
    metatype: NodeType, type: Optional[Any], siblings: Collection["Node"]
) -> str:
    """Generates a new name for the given node based on its siblings. :AutoNaming"""
    if metatype == NodeType.BLOCK or metatype == NodeType.VIEW or metatype == NodeType.ACTION:
        assert isinstance(type, BuiltinEnum), f"expected type for {metatype!r}, got {type!r}"
        base_name = to_casing(type.name, Casing.CAMEL)
        type_siblings = tuple(n for n in siblings if getattr(n, "type") == type)
    else:
        base_name = to_casing(metatype.name, Casing.CAMEL)
        type_siblings = tuple(n for n in siblings if n.metatype == metatype)

    if len(type_siblings) == 0:
        max_id = 0
    else:
        max_id = max((extract_name_id(getattr(n, "name")) or 0) for n in type_siblings)
    return f"{base_name}{max_id + 1}"


def patch_graph(*, old_graph: NodeGraph, new_graph: NodeGraph) -> None:
    """Patches the old graph *in place* from the new graph."""
    for existing_node in tuple(old_graph.nodes):
        if existing_node.id not in new_graph:
            # node removed: leave as is, remove from existing graph
            if existing_node in old_graph._nodes_by_id:
                old_graph.remove(existing_node)  # may be a child
            continue
        else:
            # node updated: patch in place
            patch_node = new_graph[existing_node.id]
            for prop in existing_node.__wired_properties__.values():
                if prop.is_computed:
                    continue  # ignore computed properties
                prop_value = getattr(existing_node, prop.name)
                existing_node._do_set(prop.name, prop_value, track=False)
    for patch_node in tuple(new_graph.nodes):
        if patch_node.id not in old_graph:
            # node added: add to existing graph
            if patch_node.id not in old_graph._nodes_by_id:
                old_graph.add(patch_node)
