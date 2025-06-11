from typing import (
    TYPE_CHECKING,
    Optional,
    Union,
    assert_never,
)

import structlog
from fastuuid import UUID
from opentelemetry import trace

from destack.language.registry import (
    NODE_CLASS_BY_TYPE,
    STRUCT_CLASS_BY_TYPE,
    TRAIT_CLASS_BY_TRAIT,
)
from destack.pb2 import NodeReferenceData, PropertyReferenceData, ScopeData

from ..builtin import (
    BuiltinEnum,
    BuiltinObjectBase,
    EnumType,
    Node,
    NodeType,
    Property,
    Region,
    StructFrozen,
    StructType,
    Trait,
    TraitType,
    enum_,
    property_,
    struct_,
)

if TYPE_CHECKING:
    from destack.language import CustomEntityDefinition, Field, Node, NodeBase

# pyright: reportIncompatibleVariableOverride=false

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)
type_ = type


@struct_(StructType.SCOPE, frozen=True)
class Scope(StructFrozen[ScopeData]):
    """The scope in the Space graph."""

    region: Optional[Region] = property_(31, is_repr=True)
    space_id: Optional[UUID] = property_(32, is_repr=True)


@enum_(EnumType.RELATION_TYPE)
class RelationType(BuiltinEnum):
    BUILTIN_NODE = 1
    CUSTOM_NODE = 2
    TRAIT = 3
    # MULTI?


@struct_(StructType.RELATION_REFERENCE, frozen=True)
class RelationReference(StructFrozen):
    """Reference to a Node "type" (builtin or custom, i.e. a "relation")."""

    type: RelationType = property_(30, is_repr=True)
    node_type: Optional[NodeType] = property_(31, is_repr=True)
    definition: Optional["CustomEntityDefinition"] = property_(32, is_repr=True)
    trait_type: Optional[TraitType] = property_(33, is_repr=True)
    if TYPE_CHECKING:
        definition_id: Optional[UUID] = None
        definition_ptr: Optional["NodeReference"] = None

    @property
    def is_single(self) -> bool:
        return self.type in (RelationType.BUILTIN_NODE, RelationType.CUSTOM_NODE)

    @property
    def is_multi(self) -> bool:
        return self.type == RelationType.TRAIT

    @property
    def object_cls(self) -> type_[BuiltinObjectBase] | None:
        if self.type == RelationType.BUILTIN_NODE:
            assert self.node_type is not None, f"no node_type for {self!r}"
            return NODE_CLASS_BY_TYPE.get(self.node_type)
        elif self.type == RelationType.CUSTOM_NODE:
            assert self.definition is not None, f"no definition for {self!r}"
            return NODE_CLASS_BY_TYPE.get(NodeType.CUSTOM_ENTITY)
        elif self.type == RelationType.TRAIT:
            assert self.trait_type is not None, f"no trait_type for {self!r}"
            return TRAIT_CLASS_BY_TRAIT.get(self.trait_type)
        else:
            assert_never(self.type)

    def resolve_property(self, name: str) -> "Property | None":
        """Resolve a Property in this relation."""
        object_cls = self.object_cls
        if object_cls is None:
            raise ValueError(f"could not resolve {self!r}")
        return object_cls.__properties__.get(name)

    def resolve_property_or_error(self, name: str) -> "Property":
        """Resolve a Property in this relation (error if not found)."""
        resolved = self.resolve_property(name)
        if resolved is None:
            raise LookupError(f"could not find property {name!r} in {self!r}")
        return resolved


def relation_ref(base: "NodeType | type[NodeBase] | CustomEntityDefinition") -> RelationReference:
    if isinstance(base, NodeType):
        return RelationReference(type=RelationType.BUILTIN_NODE, node_type=base)
    elif isinstance(base, type):
        if issubclass(base, Node):
            return RelationReference(type=RelationType.BUILTIN_NODE, node_type=base.metatype)
        elif issubclass(base, Trait):
            return RelationReference(type=RelationType.TRAIT, trait_type=base.metatype)
        else:
            raise ValueError(f"invalid relation reference type: {base!r}")
    elif isinstance(base, Node):
        return RelationReference(
            type=RelationType.CUSTOM_NODE,
            node_type=NodeType.CUSTOM_ENTITY,
            definition=base,
        )
    else:
        assert_never(base)


@enum_(EnumType.ATTRIBUTE_TYPE)
class AttributeType(BuiltinEnum):
    PROPERTY = 1
    FIELD = 2


@struct_(StructType.ATTRIBUTE_REFERENCE, frozen=True)
class AttributeReference(StructFrozen):
    """Reference to a Field or Property."""

    type: AttributeType = property_(30, is_repr=True)
    prop: Optional["Property"] = property_(31, is_repr=True)
    field: Optional["Field"] = property_(32, is_repr=True)
    if TYPE_CHECKING:
        prop_ptr: Optional["PropertyReference"] = None
        field_id: Optional[UUID] = None
        field_ptr: Optional["NodeReference"] = None


AttributeReferenceIn = Union["Field", "Property", "AttributeReference"]


def attribute_ref(attribute: AttributeReferenceIn) -> AttributeReference:
    if isinstance(attribute, Property):
        return AttributeReference(type=AttributeType.PROPERTY, prop=attribute)
    elif isinstance(attribute, Node):
        return AttributeReference(type=AttributeType.FIELD, field=attribute)
    elif isinstance(attribute, AttributeReference):
        return attribute
    else:
        assert_never(attribute)


@enum_(EnumType.PROPERTY_REFERENCE_TYPE)
class PropertyReferenceType(BuiltinEnum):
    """The type of a property reference."""

    NODE = 1
    TRAIT = 2
    STRUCT = 3


@struct_(StructType.PROPERTY_REFERENCE, frozen=True)
class PropertyReference(StructFrozen[PropertyReferenceData]):
    """
    A reference to a builtin object's Property.
    If type is unset, this refers to a base property in one of the base BuiltinObject types.
    """

    type: PropertyReferenceType = property_(30, is_repr=True)
    node_type: NodeType | None = property_(31, is_repr=True)
    trait_type: TraitType | None = property_(32, is_repr=True)
    struct_type: StructType | None = property_(33, is_repr=True)
    id: int = property_(35, is_repr=True)

    @property
    def object_cls(self) -> type_[BuiltinObjectBase] | None:
        if self.type == PropertyReferenceType.NODE:
            assert self.node_type is not None, f"no node_type for {self!r}"
            return NODE_CLASS_BY_TYPE.get(self.node_type)
        elif self.type == PropertyReferenceType.TRAIT:
            assert self.trait_type is not None, f"no trait_type for {self!r}"
            return TRAIT_CLASS_BY_TRAIT.get(self.trait_type)
        elif self.struct_type is not None:
            assert self.struct_type is not None, f"no struct_type for {self!r}"
            return STRUCT_CLASS_BY_TYPE.get(self.struct_type)
        else:
            return None

    def resolve_or_error(self) -> Property:
        """Resolves the property reference to a Property."""
        resolved = self.resolve()
        if resolved is None:
            raise ValueError(f"could not resolve {self!r}")
        return resolved

    def resolve(self) -> Property | None:
        """Resolves the property reference to a Property."""
        object_cls = self.object_cls
        prop = (object_cls or Node).__properties_by_id__.get(self.id)
        return prop


@struct_(StructType.NODE_REFERENCE, frozen=True)
class NodeReference(StructFrozen[NodeReferenceData]):
    """
    A reference to a Node.
    """

    node_type: NodeType = property_(31, is_repr=True)
    id: UUID = property_(32, is_repr=True)
    space_id: Optional[UUID] = property_(34, is_repr=True)
    definition_id: Optional[UUID] = property_(35, is_repr=True)
    # area? external_id?
