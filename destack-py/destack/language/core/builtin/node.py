from typing import (
    TYPE_CHECKING,
    Any,
    ClassVar,
    Optional,
    Self,
    cast,
    dataclass_transform,
)

from destack.language.registry import (
    NODE_CLASS_BY_TYPE,
    NODE_DEFINITION_REFERENCE_BY_CLASS,
    NODE_TYPE_BY_CLASS,
)
from destack.proto import AnyNodeProto
from destack.utils.func import get_superclasses
from destack.utils.uuid import UUID

from .common import NodeType, RoleType, StoreType, TraitType
from .const import UNSET
from .object import BuiltinObject, _process_object_cls
from .property import (
    _PROPERTY_SPECIFIERS,
    PropertyDeclaration,
    _resolve_trait_type,
    builtin_property,
    builtin_property_parent,
    builtin_property_runtime,
)
from .trait import (
    IndexIn,
)

if TYPE_CHECKING:
    from destack.language import (
        Condition,
        ExpressionIn,
        Graph,
        JoinIn,
        Node,
        NodeDefinition,
        NodeReference,
        Query,
        QueryConnection,
        Session,
        Sort,
        Supergraph,
    )

# pyright: reportIncompatibleVariableOverride=false

type_ = type


@dataclass_transform(kw_only_default=True, field_specifiers=_PROPERTY_SPECIFIERS)
def builtin_node(
    node_type: NodeType | None,
    root_type: NodeType | None = NodeType.SPACE,
    frozen: bool = False,
    index: tuple[IndexIn, ...] = (),
    event_types: tuple[NodeType, ...] = (),
    is_abstract: bool = False,
):
    """Register a class as a concrete node for the given node type."""

    # default index for nodes with parents
    if root_type:
        index = (*index, IndexIn(columns=("parent_id",), cover=("id",)))

    def decorate(cls: type) -> type:
        nonlocal frozen
        assert cls.__name__ == "Node" or issubclass(cls, Node), f"{cls.__name__} is not a Node"

        # base types
        traits: list[TraitType] = []
        base_traits: list[TraitType] = []
        inherits: list[NodeType] = []
        for base in cls.__bases__:
            if trait := _resolve_trait_type(base.__name__):
                if trait not in base_traits:
                    base_traits.append(trait)
        for superclass in get_superclasses(cls):
            if trait := _resolve_trait_type(superclass.__name__):
                if trait not in traits:
                    traits.append(trait)
            elif isinstance(base_type := getattr(superclass, "metatype", None), NodeType):
                if base_type not in inherits:
                    inherits.append(base_type)
        cls.__is_trait__ = False  # override Trait.__is_trait__
        cls.__traits__ = tuple(reversed(traits))
        cls.__base_traits__ = tuple(reversed(base_traits))
        cls.__inherits__ = tuple(reversed(inherits))
        cls.__base_type__ = cls.__inherits__[-1] if cls.__inherits__ else None
        cls.__is_abstract__ = is_abstract

        # event types
        all_event_types: list[NodeType] = []
        cls.__base_event_types__ = tuple(event_types)
        for base in cls.__bases__:
            if (
                any(b.__name__ == "Node" for b in base.__bases__)
                and issubclass(base, Node)
                and base.__base_event_types__
            ):
                for event_type in base.__base_event_types__:
                    if event_type not in all_event_types:
                        all_event_types.append(event_type)
        cls.__event_types__ = tuple(all_event_types)

        # abstract nodes cannot extend non-abstract nodes
        if is_abstract and cls.__bases__ and not cls.__bases__[0].__is_abstract__:
            raise ValueError(
                f"{cls.__name__} is abstract but extends non-abstract {cls.__bases__[0].__name__}"
            )
        if NodeType.EVENT in inherits:
            frozen = True  # Events are frozen by default

        cls, _ = _process_object_cls(
            cls=cast(type["Node"], cls),
            object_type=node_type,
            is_concrete=node_type is not None,
            is_node=True,
            is_root_node=root_type is None,
            is_frozen=frozen,
            is_abstract=is_abstract,
            traits=cls.__traits__,
            inherits=cls.__inherits__,
        )
        cls.__indexes__ = index

        # register
        if node_type is not None:
            cls.metatype = node_type
            NODE_CLASS_BY_TYPE[node_type] = cls
            NODE_TYPE_BY_CLASS[cls] = node_type
            NODE_CLASS_BY_TYPE[node_type] = cls

        # parent/root
        parent_property = cls.__properties__.get("parent", None)
        assert parent_property is not None, f"missing parent property for {node_type}"
        cls.__parent_property__ = parent_property
        cls.__root_type__ = root_type

        return cls

    return decorate


@builtin_node(node_type=NodeType.NODE, root_type=None, is_abstract=True)
class Node[NodeProtoT: AnyNodeProto](BuiltinObject[NodeProtoT]):
    """
    A Node with Properties and a persistent identity.
    """

    metatype: ClassVar[NodeType]
    __is_node__: ClassVar[bool] = True

    __definition__: ClassVar["NodeDefinition"]

    """Whether this class is an actual Node (not a Trait)."""
    __is_node__: ClassVar[bool] = True
    """Whether this class is a Trait (not a Node)."""
    __is_trait__: ClassVar[bool] = False  # override Trait.__is_trait__
    """Indexes for this Node."""
    __indexes__: ClassVar[tuple[IndexIn, ...]] = ()

    """Whether this class is abstract (not concrete)."""
    __is_abstract__: ClassVar[bool] = False
    """The base type this Node extends (directly)."""
    __base_type__: ClassVar[NodeType | None] = None
    """Nodes that extend this Node type (directly)."""
    __extended_by__: ClassVar[tuple[NodeType, ...]] = ()
    """Nodes that this Node extends (directly and indirectly)."""
    __inherits__: ClassVar[tuple[NodeType, ...]] = ()
    """Nodes that extend this Node type (directly and indirectly)."""
    __inherited_by__: ClassVar[tuple[NodeType, ...]] = ()
    """Traits directly inherited by this Node (directly)."""
    __base_traits__: ClassVar[tuple[TraitType, ...]] = ()
    """Traits directly and indirectly inherited by this Node (directly and indirectly)."""
    __traits__: ClassVar[tuple[TraitType, ...]] = ()
    """The main StoreTypes this Node is primarily stored in."""
    __primary_store_types__: ClassVar[tuple[StoreType, ...]] = ()

    """The root ancestor type of this Node type (if any)."""
    __root_type__: ClassVar[NodeType | None] = None
    """The parent type of this Node type (directly)."""
    __parent_property__: ClassVar[PropertyDeclaration] = UNSET
    """The parent classes of this Node type (directly)."""
    __parent_classes__: ClassVar[tuple[type["Node"], ...]] = ()
    """The parent types of this Node type (directly)."""
    __parent_types__: ClassVar[tuple[NodeType, ...]] = ()
    """The child types of this Node type (directly)."""
    __child_types__: ClassVar[tuple[NodeType, ...]] = ()
    """The ancestor types of this Node type (directly and indirectly)."""
    __ancestor_types__: ClassVar[tuple[NodeType, ...]] = ()
    """The descendant types of this Node type (directly and indirectly)."""
    __descendant_types__: ClassVar[tuple[NodeType, ...]] = ()

    """The base event types of this Node type (directly)."""
    __base_event_types__: ClassVar[tuple[NodeType, ...]] = ()
    """The event types of this Node type (directly and indirectly)."""
    __event_types__: ClassVar[tuple[NodeType, ...]] = ()

    # 1-20: node identity
    # Node.metatype: 1
    id: UUID = builtin_property(2, is_managed=True, is_eq=False, can_write=RoleType.SYSTEM)
    parent: Optional["Node"] = builtin_property_parent()
    # Spatial.space: 5
    # IsExtensible.definition: 6
    # IsExtensible.base_type: 7
    # Entity.[*]: 10-20
    if TYPE_CHECKING:
        parent_ptr: Optional[NodeReference] = None

    _session: "Session" = builtin_property_runtime()
    _supergraph: "Supergraph" = builtin_property_runtime()
    _graph: "Graph" = builtin_property_runtime(default=None)
    _connection: "QueryConnection | None" = builtin_property_runtime(default=None)
    _ref: "Optional[NodeReference]" = builtin_property_runtime(default=None)
    _is_new: bool = builtin_property_runtime(default=False)
    _is_attached: bool = builtin_property_runtime(default=False)

    # 20-40: node tracking
    # IsTracked.created_at/created_by/updated_at/updated_by: 20-23
    # IsArchivable.archived_at: 24
    # IsDeletable.deleted_at: 25
    # IsCustomizable.custom_values: 26
    # IsOrdered.order_key: 27
    # IsOwnable.owned_by: 28
    # ...managed_by/controlled_by?

    # 40-100: more internal properties
    # ...

    # 100+ for general properties
    # ...

    def __eq__(self, other: Any):
        """Equals the Node's identity."""
        return type(self) is type(other) and (self.id == other.id)

    def __hash__(self):
        """Hash the Node's identity."""
        return self.id.int

    @property
    def path(self) -> str:
        raise NotImplementedError  # generated

    def __to_ref__(self) -> "NodeReference":
        """Gets a reference to this Node."""
        raise NotImplementedError  # generated

    def to_ref(self) -> "NodeReference":
        """Gets a reference to this Node."""
        if self._ref is None:
            self._ref = self.__to_ref__()
        return self._ref

    @classmethod
    def from_value(
        cls,
        _object_value: dict,
        _session: "Session | None" = None,
        _supergraph: "Supergraph | None" = None,
        _graph: "Graph | None" = None,
        _connection: "QueryConnection | None" = None,
    ) -> Self:
        node_type = NodeType(_object_value["1"])
        node_cls = NODE_CLASS_BY_TYPE[node_type]
        node = node_cls.from_value(
            _object_value,
            _session=_session,
            _supergraph=_supergraph,
            _graph=_graph,
            _connection=_connection,
        )
        return cast(Self, node)

    @classmethod
    def get(
        cls: type["Self"],
        where: Optional["Condition"] = None,
        *,
        name: str | None = None,
        join: Optional["JoinIn"] = None,
        **subqueries: "Query",
    ) -> "Query[Self]":  # type: ignore
        """Make a get Query for this Node/Trait type."""
        from ..common.query import Join, Query, QueryType, to_subqueries

        query = Query(
            type=QueryType.NODE,
            definition=NODE_DEFINITION_REFERENCE_BY_CLASS[cls],
            name=name or cls.metatype.camel_name,
            join=Join.of(join) if join is not None else None,
            where=where,
            subqueries=to_subqueries(subqueries),
            limit=3,
        )
        return query  # type: ignore

    @classmethod
    def search(
        cls: type["Self"],
        where: Optional["Condition"] = None,
        *,
        name: str | None = None,
        join: Optional["JoinIn"] = None,
        having: Optional["Condition"] = None,
        sort: Optional[list["Sort"]] = None,
        group_by: Optional[list["ExpressionIn"]] = None,
        limit: Optional[int] = None,
        offset: Optional[int] = None,
        **subqueries: "Query",
    ) -> "Query[Self]":  # type: ignore
        """Make a search Query for this Node/Trait type."""
        from ..common.query import Expression, Join, Query, QueryType, to_subqueries

        query = Query(
            type=QueryType.NODE if not group_by else QueryType.GROUPED_NODE,
            definition=NODE_DEFINITION_REFERENCE_BY_CLASS[cls],
            name=name or cls.metatype.camel_name,
            join=Join.of(join) if join is not None else None,
            where=where,
            having=having,
            group_by=[Expression.of(expr) for expr in group_by or ()],
            sort=sort or [],
            limit=limit,
            offset=offset,
            subqueries=to_subqueries(subqueries),
        )
        return query  # type: ignore

    @classmethod
    def exists(
        cls: type["Self"],
        where: Optional["Condition"] = None,
        *,
        name: str | None = None,
        join: Optional["JoinIn"] = None,
    ) -> "Query[Self]":  # type: ignore
        """Make a count Query for this Node/Trait type."""
        from ..common.query import (
            Aggregation,
            AggregationType,
            Join,
            Query,
            QueryType,
        )

        query = Query(
            type=QueryType.SCALAR,
            definition=NODE_DEFINITION_REFERENCE_BY_CLASS[cls],
            name=name or cls.metatype.camel_name,
            join=Join.of(join) if join is not None else None,
            where=where,
            aggregation=Aggregation(type=AggregationType.EXISTS),
        )
        return query  # type: ignore

    @classmethod
    def count(
        cls: type["Self"],
        where: Optional["Condition"] = None,
        *,
        name: str | None = None,
        join: Optional["JoinIn"] = None,
        sort: Optional[list["Sort"]] = None,
        group_by: Optional[list["ExpressionIn"]] = None,
        having: Optional["Condition"] = None,
    ) -> "Query[Self]":  # type: ignore
        """Make a min Query for this Node/Trait type."""
        from ..common.query import Aggregation, AggregationType, Expression, Join, Query, QueryType

        query = Query(
            type=QueryType.SCALAR if not group_by else QueryType.GROUPED_SCALAR,
            definition=NODE_DEFINITION_REFERENCE_BY_CLASS[cls],
            name=name or cls.metatype.camel_name,
            join=Join.of(join) if join is not None else None,
            where=where,
            having=having,
            group_by=[Expression.of(expr) for expr in group_by or ()],
            aggregation=Aggregation(type=AggregationType.COUNT),
            sort=sort or [],
        )
        return query  # type: ignore

    @classmethod
    def min(
        cls: type["Self"],
        expression: "ExpressionIn",
        *,
        name: str | None = None,
        join: Optional["JoinIn"] = None,
        where: Optional["Condition"] = None,
        having: Optional["Condition"] = None,
        group_by: Optional[list["ExpressionIn"]] = None,
        sort: Optional[list["Sort"]] = None,
    ) -> "Query[Self]":  # type: ignore
        from ..common.query import Aggregation, AggregationType, Expression, Join, Query, QueryType

        query = Query(
            type=QueryType.SCALAR if not group_by else QueryType.GROUPED_SCALAR,
            definition=NODE_DEFINITION_REFERENCE_BY_CLASS[cls],
            name=name or cls.metatype.camel_name,
            join=Join.of(join) if join is not None else None,
            where=where,
            having=having,
            group_by=[Expression.of(expr) for expr in group_by or ()],
            aggregation=Aggregation(type=AggregationType.MIN, expression=Expression.of(expression)),
            sort=sort or [],
        )
        return query  # type: ignore

    @classmethod
    def max(
        cls: type["Self"],
        expression: "ExpressionIn",
        *,
        name: str | None = None,
        join: Optional["JoinIn"] = None,
        where: Optional["Condition"] = None,
        having: Optional["Condition"] = None,
        group_by: Optional[list["ExpressionIn"]] = None,
        sort: Optional[list["Sort"]] = None,
    ) -> "Query[Self]":  # type: ignore
        """Make an average Query for this Node/Trait type."""
        from ..common.query import Aggregation, AggregationType, Expression, Join, Query, QueryType

        query = Query(
            type=QueryType.SCALAR if not group_by else QueryType.GROUPED_SCALAR,
            definition=NODE_DEFINITION_REFERENCE_BY_CLASS[cls],
            name=name or cls.metatype.camel_name,
            join=Join.of(join) if join is not None else None,
            where=where,
            having=having,
            group_by=[Expression.of(expr) for expr in group_by or ()],
            aggregation=Aggregation(type=AggregationType.MAX, expression=Expression.of(expression)),
            sort=sort or [],
        )
        return query  # type: ignore

    @classmethod
    def sum(
        cls: type["Self"],
        expression: "ExpressionIn",
        *,
        name: str | None = None,
        join: Optional["JoinIn"] = None,
        where: Optional["Condition"] = None,
        having: Optional["Condition"] = None,
        group_by: Optional[list["ExpressionIn"]] = None,
        sort: Optional[list["Sort"]] = None,
    ) -> "Query[Self]":  # type: ignore
        """Make an average Query for this Node/Trait type."""
        from ..common.query import Aggregation, AggregationType, Expression, Join, Query, QueryType

        query = Query(
            type=QueryType.SCALAR if not group_by else QueryType.GROUPED_SCALAR,
            definition=NODE_DEFINITION_REFERENCE_BY_CLASS[cls],
            name=name or cls.metatype.camel_name,
            join=Join.of(join) if join is not None else None,
            where=where,
            having=having,
            group_by=[Expression.of(expr) for expr in group_by or ()],
            aggregation=Aggregation(type=AggregationType.SUM, expression=Expression.of(expression)),
            sort=sort or [],
        )
        return query  # type: ignore
