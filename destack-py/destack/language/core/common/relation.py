from typing import (
    TYPE_CHECKING,
    Optional,
    assert_never,
)

from destack.language.registry import (
    NODE_CLASS_BY_TYPE,
    STRUCT_CLASS_BY_TYPE,
    TRAIT_CLASS_BY_TYPE,
)
from destack.proto import NodeReferenceProto, PropertyReferenceProto, ScopeProto
from destack.utils.uuid import UUID

from ..builtin import (
    BuiltinObjectBase,
    Enum,
    EnumType,
    Node,
    NodeType,
    PrimitiveType,
    PropertyDeclaration,
    Region,
    StructBase,
    StructFrozen,
    StructType,
    Trait,
    TraitType,
    builtin_enum,
    builtin_struct,
    property_,
)

if TYPE_CHECKING:
    from destack.language import (
        CustomEntityDefinition,
        CustomProperty,
        Node,
        NodeBase,
        PropertyDefinition,
    )

# pyright: reportIncompatibleVariableOverride=false

type_ = type


@builtin_struct(StructType.SCOPE, frozen=True)
class Scope(StructFrozen[ScopeProto]):
    """The scope in the Space graph."""

    region: Optional[Region] = property_(31, is_repr=True)
    space_id: Optional[UUID] = property_(32, is_repr=True)


@builtin_enum(EnumType.RELATION_TYPE)
class RelationType(Enum):
    BUILTIN_NODE = 1
    CUSTOM_NODE = 2
    TRAIT = 3
    # MULTI?


@builtin_struct(StructType.RELATION_REFERENCE, frozen=True)
class RelationReference(StructFrozen):
    """Reference to a Node relation (builtin, custom or trait, i.e. a "relation")."""

    type: RelationType = property_(30, is_repr=True)
    node_type: Optional[NodeType] = property_(31, is_repr=True)
    definition: Optional["CustomEntityDefinition"] = property_(32, is_repr=True)
    trait_type: Optional[TraitType] = property_(33, is_repr=True)
    if TYPE_CHECKING:
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
            return TRAIT_CLASS_BY_TYPE.get(self.trait_type)
        else:
            assert_never(self.type)

    def resolve_property(self, name: str) -> "PropertyDeclaration | None":
        """Resolve a Property in this relation."""
        object_cls = self.object_cls
        if object_cls is None:
            raise ValueError(f"could not resolve {self!r}")
        return object_cls.__properties__.get(name)

    def resolve_property_or_error(self, name: str) -> "PropertyDeclaration":
        """Resolve a Property in this relation (error if not found)."""
        resolved = self.resolve_property(name)
        if resolved is None:
            raise LookupError(f"could not find property {name!r} in {self!r}")
        return resolved

    @classmethod
    def of(cls, base: "NodeType | type[NodeBase] | CustomEntityDefinition") -> "RelationReference":
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


@builtin_enum(EnumType.OBJECT_TYPE)
class ObjectType(Enum):
    BUILTIN_NODE = 1
    CUSTOM_NODE = 2
    TRAIT = 3
    BUILTIN_STRUCT = 5
    # MULTI?


@builtin_struct(StructType.OBJECT_REFERENCE, frozen=True)
class ObjectReference(StructFrozen):
    """Reference to an object "type" (builtin, custom or trait)."""

    type: ObjectType = property_(30, is_repr=True)
    node_type: Optional[NodeType] = property_(31, is_repr=True)
    trait_type: Optional[TraitType] = property_(32, is_repr=True)
    struct_type: Optional[StructType] = property_(33, is_repr=True)
    definition: Optional["CustomEntityDefinition"] = property_(40, is_repr=True)
    if TYPE_CHECKING:
        definition_ptr: Optional["NodeReference"] = None

    @property
    def object_cls(self) -> type_[BuiltinObjectBase] | None:
        if self.type == ObjectType.BUILTIN_NODE:
            assert self.node_type is not None, f"no node_type for {self!r}"
            return NODE_CLASS_BY_TYPE.get(self.node_type)
        elif self.type == ObjectType.CUSTOM_NODE:
            assert self.definition is not None, f"no definition for {self!r}"
            return NODE_CLASS_BY_TYPE.get(NodeType.CUSTOM_ENTITY)
        elif self.type == ObjectType.TRAIT:
            assert self.trait_type is not None, f"no trait_type for {self!r}"
            return TRAIT_CLASS_BY_TYPE.get(self.trait_type)
        elif self.type == ObjectType.BUILTIN_STRUCT:
            assert self.struct_type is not None, f"no struct_type for {self!r}"
            return STRUCT_CLASS_BY_TYPE.get(self.struct_type)
        else:
            assert_never(self.type)

    def resolve_property(self, name: str) -> "PropertyDeclaration | None":
        """Resolve a Property in this relation."""
        object_cls = self.object_cls
        if object_cls is None:
            raise ValueError(f"could not resolve {self!r}")
        return object_cls.__properties__.get(name)

    def resolve_property_or_error(self, name: str) -> "PropertyDeclaration":
        """Resolve a Property in this relation (error if not found)."""
        resolved = self.resolve_property(name)
        if resolved is None:
            raise LookupError(f"could not find property {name!r} in {self!r}")
        return resolved

    @classmethod
    def of(
        cls, base: "NodeType | type[NodeBase] | CustomEntityDefinition | type[StructBase]"
    ) -> "ObjectReference":
        if isinstance(base, NodeType):
            return ObjectReference(type=ObjectType.BUILTIN_NODE, node_type=base)
        elif isinstance(base, StructType):
            return ObjectReference(type=ObjectType.BUILTIN_STRUCT, struct_type=base)
        elif isinstance(base, type):
            if issubclass(base, Node):
                return ObjectReference(type=ObjectType.BUILTIN_NODE, node_type=base.metatype)
            elif issubclass(base, Trait):
                return ObjectReference(type=ObjectType.TRAIT, trait_type=base.metatype)
            elif issubclass(base, StructBase):
                return ObjectReference(type=ObjectType.BUILTIN_STRUCT, struct_type=base.metatype)
            else:
                raise ValueError(f"invalid object reference type: {base!r}")
        elif isinstance(base, Node):
            return ObjectReference(
                type=ObjectType.CUSTOM_NODE,
                node_type=NodeType.CUSTOM_ENTITY,
                definition=base,
            )
        else:
            assert_never(base)


@builtin_enum(EnumType.PROPERTY_REFERENCE_TYPE)
class PropertyReferenceType(Enum):
    """The type of a property reference."""

    BUILTIN = 1
    CUSTOM = 2


@builtin_struct(StructType.PROPERTY_REFERENCE, frozen=True)
class PropertyReference(StructFrozen[PropertyReferenceProto]):
    """
    A reference to a builtin object's Property.
    """

    type: PropertyReferenceType = property_(30, is_repr=True)
    node_type: NodeType | None = property_(31, is_repr=True)
    trait_type: TraitType | None = property_(32, is_repr=True)
    struct_type: StructType | None = property_(33, is_repr=True)
    id: int | None = property_(
        35,
        is_repr=True,
        primitive_type=PrimitiveType.INT32,
        description="id of the builtin Property",
    )
    custom_property: "CustomProperty | None" = property_(
        36,
        is_repr=True,
        description="custom Property of a custom Node or Struct",
    )

    def resolve_or_error(self) -> "PropertyDefinition | CustomProperty":
        """Resolves the property reference to a Property."""
        resolved = self.resolve()
        if resolved is None:
            raise ValueError(f"could not resolve {self!r}")
        return resolved

    def resolve(self) -> "PropertyDefinition | CustomProperty | None":
        """Resolves the property reference to a Property."""
        if self.type == PropertyReferenceType.BUILTIN:
            if (node_type := self.node_type) is not None:
                object_cls = NODE_CLASS_BY_TYPE.get(node_type)
            elif (struct_type := self.struct_type) is not None:
                object_cls = STRUCT_CLASS_BY_TYPE.get(struct_type)
            elif (trait_type := self.trait_type) is not None:
                object_cls = TRAIT_CLASS_BY_TYPE.get(trait_type)
            else:
                object_cls = None
            object_cls = object_cls or Node
            assert self.id is not None, f"no id for {self!r}"
            prop = object_cls.__properties_by_id__.get(self.id)
            return prop.definition if prop is not None else None
        elif self.type == PropertyReferenceType.CUSTOM:
            prop = self.custom_property
            return prop
        else:
            return None

    @staticmethod
    def of(
        base: "PropertyDeclaration | PropertyDefinition | CustomProperty",
    ) -> "PropertyReference":
        from destack.language.core import CustomProperty

        if isinstance(base, PropertyDeclaration):
            return base.to_ref()
        elif isinstance(base, PropertyDefinition):
            raise ValueError(f"cannot convert {base!r} to a PropertyReference")
        elif isinstance(base, CustomProperty):
            return PropertyReference(
                type=PropertyReferenceType.CUSTOM,
                node_type=NodeType.CUSTOM_ENTITY,
                custom_property=base,
            )
        else:
            assert_never(base)


@builtin_struct(StructType.NODE_REFERENCE, frozen=True)
class NodeReference(StructFrozen[NodeReferenceProto]):
    """
    A reference to a Node (builtin or custom).
    """

    node_type: NodeType = property_(31, is_repr=True)
    id: UUID = property_(32, is_repr=True)
    space_id: Optional[UUID] = property_(34, is_repr=True)
    definition_id: Optional[UUID] = property_(35, is_repr=True)
    # area? external_id?
