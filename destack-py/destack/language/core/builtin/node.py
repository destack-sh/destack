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

from .common import Cson, EnumType, NodeType, StoreDomain, StoreKey, TraitType
from .const import UNSET
from .meta import TagDeclaration, builtin_method
from .object import BuiltinObject, ValueFactory, _process_object_cls
from .property import (
    _PROPERTY_SPECIFIERS,
    PropertyDeclaration,
    _resolve_trait_type,
    builtin_property,
    builtin_property_runtime,
)

if TYPE_CHECKING:
    from destack.language import (
        ActionDefinition,
        Condition,
        ConstraintDeclaration,
        ConstraintDefinition,
        ExpressionIn,
        Graph,
        IndexDeclaration,
        IndexDefinition,
        JoinIn,
        MethodDefinition,
        Node,
        NodeDefinition,
        NodeDefinitionReference,
        NodeReference,
        PermissionDeclaration,
        PermissionDefinition,
        Query,
        QueryConnection,
        Session,
        Sort,
        Space,
        Supergraph,
        TagDefinition,
    )

# pyright: reportIncompatibleVariableOverride=false

type_ = type


@dataclass_transform(kw_only_default=True, field_specifiers=_PROPERTY_SPECIFIERS)
def builtin_node(
    node_type: NodeType | None,
    *,
    frozen: bool = False,
    is_abstract: bool = False,
    is_extensible: bool = False,
    event_types: tuple[NodeType, ...] = (),
    enum_types: tuple[EnumType, ...] = (),
    expected_parent_types: tuple[NodeType, ...] = (),
    expected_child_types: tuple[NodeType, ...] = (),
    expected_ancestor_types: tuple[NodeType, ...] = (),
    expected_descendant_types: tuple[NodeType, ...] = (),
    indexes: tuple["IndexDeclaration", ...] = (),
    constraints: tuple["ConstraintDeclaration", ...] = (),
    permissions: tuple["PermissionDeclaration", ...] = (),
    tags: tuple["TagDeclaration", ...] = (),
):
    """Register a class as a concrete node for the given node type."""

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
        cls.__is_extensible__ = is_extensible

        # event types
        cls.__base_event_types__ = tuple(event_types)

        # abstract nodes cannot extend non-abstract nodes
        if is_abstract and cls.__bases__ and not cls.__bases__[0].__is_abstract__:
            raise ValueError(
                f"{cls.__name__} is abstract but extends non-abstract {cls.__bases__[0].__name__}"
            )
        if NodeType.EVENT in inherits:
            frozen = True  # Events are always frozen

        # expected types
        cls.__expected_parent_types__ = tuple(expected_parent_types)
        cls.__expected_child_types__ = tuple(expected_child_types)
        cls.__expected_ancestor_types__ = tuple(expected_ancestor_types)
        cls.__expected_descendant_types__ = tuple(expected_descendant_types)

        # process class
        is_entity = any(base.__name__ == "Entity" for base in cls.__bases__)
        cls, _ = _process_object_cls(
            cls=cast(type["Node"], cls),
            object_type=node_type,
            is_struct=False,
            is_concrete=node_type is not None,
            is_node=True,
            is_root_node=node_type == NodeType.SPACE,
            is_entity=is_entity,
            is_frozen=frozen,
            is_abstract=is_abstract,
            base_type=None,
            traits=cls.__traits__,
            inherits=cls.__inherits__,
        )
        if node_type is not None:
            cls.metatype = node_type

        # meta
        if indexes:
            from .definition import IndexDefinition

            cls.__indexes__ = tuple(
                IndexDefinition.from_declaration(cls, index) for index in indexes
            )
        if constraints:
            from .definition import ConstraintDefinition

            cls.__constraints__ = tuple(
                ConstraintDefinition.from_declaration(cls, constraint) for constraint in constraints
            )
        if permissions:
            from .definition import PermissionDefinition

            cls.__permissions__ = tuple(
                PermissionDefinition.from_declaration(permission) for permission in permissions
            )
        if tags:
            from .definition import TagDefinition

            cls.__tags__ = tuple(TagDefinition.from_declaration(tag) for tag in tags)

        # register
        if node_type is not None:
            NODE_CLASS_BY_TYPE[node_type] = cls
            NODE_TYPE_BY_CLASS[cls] = node_type
            NODE_CLASS_BY_TYPE[node_type] = cls

        # parent/root
        parent_property = cls.__properties__.get("parent", None)
        cls.__parent_property__ = parent_property

        return cls

    return decorate


@builtin_node(
    node_type=NodeType.NODE,
    is_abstract=True,
    tags=(
        TagDeclaration(id=1, name="identity", description="Node identity"),
        TagDeclaration(id=2, name="tracking", description="Node tracking"),
    ),
)
class Node[NodeProtoT: AnyNodeProto](BuiltinObject[NodeProtoT]):
    """
    A Node with some Properties and a persistent identity (its id).
    Nodes always belong to a Space and are thus identifiable by their (space_id, id) tuple.
    """

    """The specific metatype of this Node."""
    metatype: ClassVar[NodeType]
    """The definition this Node is an instance of."""
    __definition__: ClassVar["NodeDefinition"]
    """The reference to the definition this Node is an instance of."""
    __definition_reference__: ClassVar["NodeDefinitionReference"]

    # flags
    """Whether this class is an actual Node (not a Trait)."""
    __is_node__: ClassVar[bool] = True
    """Whether this class is a Trait (not a Node)."""
    __is_trait__: ClassVar[bool] = False  # override Trait.__is_trait__ in subclasses
    """Whether this class is abstract (not concrete)."""
    __is_abstract__: ClassVar[bool] = False
    """Whether this class is extensible (can be extended by custom Nodes)."""
    __is_extensible__: ClassVar[bool] = False

    # inheritance
    """The base type this Node extends (directly)."""
    __base_type__: ClassVar[NodeType | None] = None
    """Nodes that extend this Node (directly)."""
    __extended_by__: ClassVar[tuple[NodeType, ...]] = ()
    """Nodes that this Node extends (directly and indirectly)."""
    __inherits__: ClassVar[tuple[NodeType, ...]] = ()
    """Nodes that extend this Node (directly and indirectly)."""
    __inherited_by__: ClassVar[tuple[NodeType, ...]] = ()
    """Traits directly inherited by this Node (directly)."""
    __base_traits__: ClassVar[tuple[TraitType, ...]] = ()
    """Traits directly and indirectly inherited by this Node (directly and indirectly)."""
    __traits__: ClassVar[tuple[TraitType, ...]] = ()

    # content
    """The indexes defined for this Node."""
    __indexes__: ClassVar[tuple["IndexDefinition", ...]] = ()
    """The constraints defined for this Node."""
    __constraints__: ClassVar[tuple["ConstraintDefinition", ...]] = ()
    """The permissions defined for this Node."""
    __permissions__: ClassVar[tuple["PermissionDefinition", ...]] = ()
    """The methods defined for this Node."""
    __methods__: ClassVar[tuple["MethodDefinition", ...]] = ()
    """The actions defined for this Node."""
    __actions__: ClassVar[tuple["ActionDefinition", ...]] = ()
    """The tags defined for this Node."""
    __tags__: ClassVar[tuple["TagDefinition", ...]] = ()

    # event
    """The base event types of this Node (directly)."""
    __base_event_types__: ClassVar[tuple[NodeType, ...]] = ()
    """The event types of this Node (directly and indirectly)."""
    __event_types__: ClassVar[tuple[NodeType, ...]] = ()

    # enum
    """The base enum types of this Node (directly)."""
    __base_enum_types__: ClassVar[tuple[EnumType, ...]] = ()
    """The enum types of this Node (directly and indirectly)."""
    __enum_types__: ClassVar[tuple[EnumType, ...]] = ()

    # tree
    """The parent type of this Node (directly)."""
    __parent_property__: ClassVar[PropertyDeclaration | None] = None
    """The parent classes of this Node (directly)."""
    __parent_classes__: ClassVar[tuple[type["Node"], ...]] = ()
    """The parent types of this Node (directly)."""
    __parent_types__: ClassVar[tuple[NodeType, ...]] = ()
    """The child types of this Node (directly)."""
    __child_types__: ClassVar[tuple[NodeType, ...]] = ()
    """The ancestor types of this Node (directly and indirectly)."""
    __ancestor_types__: ClassVar[tuple[NodeType, ...]] = ()
    """The descendant types of this Node (directly and indirectly)."""
    __descendant_types__: ClassVar[tuple[NodeType, ...]] = ()
    """The expected parent types of this Node (any of)."""

    # expected tree
    __expected_parent_types__: ClassVar[tuple[NodeType, ...]] = ()
    """The expected child types of this Node (any of)."""
    __expected_child_types__: ClassVar[tuple[NodeType, ...]] = ()
    """The expected ancestor types of this Node (any of)."""
    __expected_ancestor_types__: ClassVar[tuple[NodeType, ...]] = ()
    """The expected descendant types of this Node (any of)."""
    __expected_descendant_types__: ClassVar[tuple[NodeType, ...]] = ()

    # store
    """The main Stores this Node is primarily stored in."""
    __primary_store_keys__: ClassVar[tuple[StoreKey, ...]] = ()
    """The domain of this Node (Entity or Event)."""
    __store_domain__: ClassVar[StoreDomain | None] = None

    # 1-20: node identity
    # Node.metatype: 1
    id: UUID = builtin_property(
        2,
        is_internal=True,
        is_eq=False,
        is_readonly=True,
        description="The universally unique identifier of this Node.",
        tags=("identity",),
    )
    space: "Space" = builtin_property(
        5,
        is_internal=True,
        is_readonly=True,
        default_factory=ValueFactory.SPACE,
        description="The Space this Node is in.",
        tags=("identity",),
    )
    if TYPE_CHECKING:
        space_ptr: NodeReference = UNSET

    # 100+ for general properties
    # ...

    """The current Session this Node is in."""
    _session: "Session" = builtin_property_runtime()
    """The Supergraph this Node is part of."""
    _supergraph: "Supergraph" = builtin_property_runtime()
    """The specific Graph this Node is part of."""
    _graph: "Graph" = builtin_property_runtime(default=None)
    """The QueryConnection this Node is from (if any)."""
    _connection: "QueryConnection | None" = builtin_property_runtime(default=None)
    """The cached reference to this Node."""
    _ref: "Optional[NodeReference]" = builtin_property_runtime(default=None)
    """Whether this Node is new."""
    _is_new: bool = builtin_property_runtime(default=False)

    def __eq__(self, other: Any):
        """Equals the Node's identity."""
        return type(self) is type(other) and (self.id == other.id)

    def __hash__(self):
        """Hash the Node's identity."""
        return self.id.int

    @property
    @builtin_method(1)
    def path(self) -> str:
        """The human readable path of this Node."""
        raise NotImplementedError  # generated

    def __to_ref__(self) -> "NodeReference":
        """Gets a reference to this Node."""
        raise NotImplementedError  # generated

    @builtin_method(2)
    def to_ref(self) -> "NodeReference":
        """Gets a reference to this Node."""
        if self._ref is None:
            self._ref = self.__to_ref__()
        return self._ref

    @classmethod
    def from_cson(
        cls,
        _object_cson: "Cson",
        _session: "Session | None" = None,
        _supergraph: "Supergraph | None" = None,
        _graph: "Graph | None" = None,
        _connection: "QueryConnection | None" = None,
    ) -> Self:
        node_type = NodeType(_object_cson["1"])
        node_cls = NODE_CLASS_BY_TYPE[node_type]
        node = node_cls.from_cson(
            _object_cson,
            _session=_session,
            _supergraph=_supergraph,
            _graph=_graph,
            _connection=_connection,
        )
        return cast(Self, node)

    @classmethod
    @builtin_method(60)
    def get(
        cls: type["Self"],
        where: Optional["Condition"] = None,
        *,
        name: str | None = None,
        join: Optional["JoinIn"] = None,
        include_deleted: bool = False,
        **subqueries: "Query",
    ) -> "Query[Self]":  # type: ignore
        """Make a get Query for this Node."""
        from ..common.query import Join, Query, QueryType, to_subqueries

        assert cls.__store_domain__ is not None, f"no store domain for {cls.__name__}"
        query = Query(
            type=QueryType.NODE,
            domain=cls.__store_domain__,
            definition=NODE_DEFINITION_REFERENCE_BY_CLASS[cls],
            name=name or cls.metatype.camel_name,
            join=Join.of(join) if join is not None else None,
            where=where,
            include_deleted=include_deleted,
            subqueries=to_subqueries(subqueries),
        )
        return query  # type: ignore

    @classmethod
    @builtin_method(61)
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
        include_deleted: bool = False,
        **subqueries: "Query",
    ) -> "Query[Self]":  # type: ignore
        """Make a search Query for this Node."""
        from ..common.query import Expression, Join, Query, QueryType, to_subqueries

        assert cls.__store_domain__ is not None, f"no store domain for {cls.__name__}"
        query = Query(
            type=QueryType.NODE if not group_by else QueryType.GROUPED_NODE,
            domain=cls.__store_domain__,
            definition=NODE_DEFINITION_REFERENCE_BY_CLASS[cls],
            name=name or cls.metatype.camel_name,
            join=Join.of(join) if join is not None else None,
            where=where,
            having=having,
            group_by=[Expression.of(expr) for expr in group_by or ()],
            sort=sort or [],
            limit=limit,
            offset=offset,
            include_deleted=include_deleted,
            subqueries=to_subqueries(subqueries),
        )
        return query  # type: ignore

    @classmethod
    @builtin_method(62)
    def exists(
        cls: type["Self"],
        where: Optional["Condition"] = None,
        *,
        name: str | None = None,
        join: Optional["JoinIn"] = None,
        include_deleted: bool = False,
    ) -> "Query[Self]":  # type: ignore
        """Make a count Query for this Node."""
        from ..common.query import (
            Aggregation,
            AggregationType,
            Join,
            Query,
            QueryType,
        )

        assert cls.__store_domain__ is not None, f"no store domain for {cls.__name__}"
        query = Query(
            type=QueryType.SCALAR,
            domain=cls.__store_domain__,
            definition=NODE_DEFINITION_REFERENCE_BY_CLASS[cls],
            name=name or cls.metatype.camel_name,
            join=Join.of(join) if join is not None else None,
            where=where,
            include_deleted=include_deleted,
            aggregation=Aggregation(type=AggregationType.EXISTS),
        )
        return query  # type: ignore

    @classmethod
    @builtin_method(63)
    def count(
        cls: type["Self"],
        where: Optional["Condition"] = None,
        *,
        name: str | None = None,
        join: Optional["JoinIn"] = None,
        sort: Optional[list["Sort"]] = None,
        group_by: Optional[list["ExpressionIn"]] = None,
        having: Optional["Condition"] = None,
        include_deleted: bool = False,
    ) -> "Query[Self]":  # type: ignore
        """Make a min Query for this Node."""
        from ..common.query import Aggregation, AggregationType, Expression, Join, Query, QueryType

        assert cls.__store_domain__ is not None, f"no store domain for {cls.__name__}"
        query = Query(
            type=QueryType.SCALAR if not group_by else QueryType.GROUPED_SCALAR,
            domain=cls.__store_domain__,
            definition=NODE_DEFINITION_REFERENCE_BY_CLASS[cls],
            name=name or cls.metatype.camel_name,
            join=Join.of(join) if join is not None else None,
            where=where,
            having=having,
            group_by=[Expression.of(expr) for expr in group_by or ()],
            aggregation=Aggregation(type=AggregationType.COUNT),
            include_deleted=include_deleted,
            sort=sort or [],
        )
        return query  # type: ignore

    @classmethod
    @builtin_method(64)
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
        include_deleted: bool = False,
    ) -> "Query[Self]":  # type: ignore
        from ..common.query import Aggregation, AggregationType, Expression, Join, Query, QueryType

        assert cls.__store_domain__ is not None, f"no store domain for {cls.__name__}"
        query = Query(
            type=QueryType.SCALAR if not group_by else QueryType.GROUPED_SCALAR,
            domain=cls.__store_domain__,
            definition=NODE_DEFINITION_REFERENCE_BY_CLASS[cls],
            name=name or cls.metatype.camel_name,
            join=Join.of(join) if join is not None else None,
            where=where,
            having=having,
            group_by=[Expression.of(expr) for expr in group_by or ()],
            aggregation=Aggregation(type=AggregationType.MIN, expression=Expression.of(expression)),
            sort=sort or [],
            include_deleted=include_deleted,
        )
        return query  # type: ignore

    @classmethod
    @builtin_method(65)
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
        include_deleted: bool = False,
    ) -> "Query[Self]":  # type: ignore
        """Make an average Query for this Node."""
        from ..common.query import Aggregation, AggregationType, Expression, Join, Query, QueryType

        assert cls.__store_domain__ is not None, f"no store domain for {cls.__name__}"
        query = Query(
            type=QueryType.SCALAR if not group_by else QueryType.GROUPED_SCALAR,
            domain=cls.__store_domain__,
            definition=NODE_DEFINITION_REFERENCE_BY_CLASS[cls],
            name=name or cls.metatype.camel_name,
            join=Join.of(join) if join is not None else None,
            where=where,
            having=having,
            group_by=[Expression.of(expr) for expr in group_by or ()],
            aggregation=Aggregation(type=AggregationType.MAX, expression=Expression.of(expression)),
            sort=sort or [],
            include_deleted=include_deleted,
        )
        return query  # type: ignore

    @classmethod
    @builtin_method(66)
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
        include_deleted: bool = False,
    ) -> "Query[Self]":  # type: ignore
        """Make an average Query for this Node."""
        from ..common.query import Aggregation, AggregationType, Expression, Join, Query, QueryType

        assert cls.__store_domain__ is not None, f"no store domain for {cls.__name__}"
        query = Query(
            type=QueryType.SCALAR if not group_by else QueryType.GROUPED_SCALAR,
            domain=cls.__store_domain__,
            definition=NODE_DEFINITION_REFERENCE_BY_CLASS[cls],
            name=name or cls.metatype.camel_name,
            join=Join.of(join) if join is not None else None,
            where=where,
            having=having,
            group_by=[Expression.of(expr) for expr in group_by or ()],
            aggregation=Aggregation(type=AggregationType.SUM, expression=Expression.of(expression)),
            sort=sort or [],
            include_deleted=include_deleted,
        )
        return query  # type: ignore
