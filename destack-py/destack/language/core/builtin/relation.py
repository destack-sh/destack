from typing import (
    TYPE_CHECKING,
    Optional,
    Union,
    assert_never,
)

from destack.language.registry import (
    NODE_CLASS_BY_TYPE,
    STRUCT_CLASS_BY_TYPE,
    TRAIT_CLASS_BY_TYPE,
)
from destack.proto import NodeReferenceProto, PropertyReferenceProto
from destack.utils.uuid import UUID

from .common import EnumType, NodeType, PrimitiveType
from .enum import Enum, builtin_enum
from .object import BuiltinObjectBase
from .property import PropertyDeclaration, builtin_property
from .struct import StructBase, StructFrozen, StructType, builtin_struct
from .trait import Trait, TraitType

if TYPE_CHECKING:
    from destack.language import (
        CustomEntityDefinition,
        CustomEventDefinition,
        CustomProperty,
        CustomStructDefinition,
        CustomTraitDefinition,
        Node,
        NodeBase,
        PropertyDefinition,
    )

# pyright: reportIncompatibleVariableOverride=false

type_ = type


@builtin_enum(EnumType.NODE_DEFINITION_TYPE)
class NodeDefinitionType(Enum):
    BUILTIN_NODE = 1
    CUSTOM_NODE = 2
    BUILTIN_TRAIT = 3
    CUSTOM_TRAIT = 4


@builtin_struct(StructType.NODE_DEFINITION_REFERENCE, frozen=True)
class NodeDefinitionReference(StructFrozen):
    """Reference to a Node definition (builtin, custom or by trait)."""

    type: NodeDefinitionType = builtin_property(100, is_repr=True)
    node_type: Optional[NodeType] = builtin_property(101, is_repr=True)
    trait_type: Optional[TraitType] = builtin_property(102, is_repr=True)
    definition: Union[
        "CustomEntityDefinition",
        "CustomEventDefinition",
        "CustomTraitDefinition",
        None,
    ] = builtin_property(105, is_repr=True)
    if TYPE_CHECKING:
        definition_ptr: Optional["NodeReference"] = None

    @property
    def is_multi(self) -> bool:
        """Whether this definition references multiple Node definitions"""
        if self.type == NodeDefinitionType.BUILTIN_NODE:
            assert self.node_type is not None, f"no node_type for {self!r}"
            node_cls = NODE_CLASS_BY_TYPE[self.node_type]
            return TraitType.EXTENSIBLE in node_cls.__traits__
        elif self.type == NodeDefinitionType.CUSTOM_NODE:
            raise NotImplementedError(f"unexpected node definition reference: {self!r}")
        elif self.type in (NodeDefinitionType.BUILTIN_TRAIT, NodeDefinitionType.CUSTOM_TRAIT):
            return True
        else:
            assert_never(self.type)

    @property
    def object_cls(self) -> type_[BuiltinObjectBase] | None:
        if self.type == NodeDefinitionType.BUILTIN_NODE:
            assert self.node_type is not None, f"no node_type for {self!r}"
            return NODE_CLASS_BY_TYPE.get(self.node_type)
        elif self.type == NodeDefinitionType.CUSTOM_NODE:
            node_definition = self.definition
            assert node_definition is not None, f"no definition for {self!r}"
            return NODE_CLASS_BY_TYPE.get(NodeType.CUSTOM_ENTITY)
        elif self.type == NodeDefinitionType.BUILTIN_TRAIT:
            assert self.trait_type is not None, f"no trait_type for {self!r}"
            return TRAIT_CLASS_BY_TYPE.get(self.trait_type)
        elif self.type == NodeDefinitionType.CUSTOM_TRAIT:
            trait_definition = self.definition
            assert trait_definition is not None, f"no trait_definition for {self!r}"
            return TRAIT_CLASS_BY_TYPE.get(trait_definition.metatype)
        else:
            assert_never(self.type)

    def resolve_property(self, name: str) -> "PropertyDeclaration | None":
        """Resolve a Property in this definition."""
        object_cls = self.object_cls
        if object_cls is None:
            raise ValueError(f"could not resolve {self!r}")
        return object_cls.__properties__.get(name)

    def resolve_property_or_error(self, name: str) -> "PropertyDeclaration":
        """Resolve a Property in this definition (error if not found)."""
        resolved = self.resolve_property(name)
        if resolved is None:
            raise LookupError(f"could not find property {name!r} in {self!r}")
        return resolved

    @classmethod
    def of(
        cls, base: "NodeType | type[NodeBase] | CustomEntityDefinition"
    ) -> "NodeDefinitionReference":
        from .node import Node

        if isinstance(base, NodeType):
            return NodeDefinitionReference(type=NodeDefinitionType.BUILTIN_NODE, node_type=base)
        elif isinstance(base, type):
            if issubclass(base, Node):
                return NodeDefinitionReference(
                    type=NodeDefinitionType.BUILTIN_NODE, node_type=base.metatype
                )
            elif issubclass(base, Trait):
                return NodeDefinitionReference(
                    type=NodeDefinitionType.BUILTIN_TRAIT, trait_type=base.metatype
                )
            else:
                raise ValueError(f"invalid definition reference type: {base!r}")
        elif isinstance(base, Node):
            raise NotImplementedError(f"unexpected node definition reference: {base!r}")
        else:
            assert_never(base)


@builtin_enum(EnumType.OBJECT_DEFINITION_TYPE)
class ObjectDefinitionType(Enum):
    BUILTIN_NODE = 1
    CUSTOM_NODE = 2
    BUILTIN_TRAIT = 3
    CUSTOM_TRAIT = 4
    BUILTIN_STRUCT = 5
    CUSTOM_STRUCT = 6
    # BUILTIN_ENUM, CUSTOM_ENUM?


@builtin_struct(StructType.OBJECT_DEFINITION_REFERENCE, frozen=True)
class ObjectDefinitionReference(StructFrozen):
    """Reference to an object "type" (builtin, custom or trait)."""

    type: ObjectDefinitionType = builtin_property(100, is_repr=True)
    node_type: Optional[NodeType] = builtin_property(101, is_repr=True)
    trait_type: Optional[TraitType] = builtin_property(102, is_repr=True)
    struct_type: Optional[StructType] = builtin_property(103, is_repr=True)
    definition: Union[
        "CustomEntityDefinition",
        "CustomEventDefinition",
        "CustomTraitDefinition",
        "CustomStructDefinition",
        None,
    ] = builtin_property(105, is_repr=True)
    if TYPE_CHECKING:
        definition_ptr: Optional["NodeReference"] = None

    @property
    def object_cls(self) -> type_[BuiltinObjectBase] | None:
        if self.type == ObjectDefinitionType.BUILTIN_NODE:
            assert self.node_type is not None, f"no node_type for {self!r}"
            return NODE_CLASS_BY_TYPE.get(self.node_type)
        elif self.type == ObjectDefinitionType.CUSTOM_NODE:
            definition = self.definition
            assert definition is not None, f"no definition for {self!r}"
            if isinstance(definition, CustomEntityDefinition):
                return NODE_CLASS_BY_TYPE.get(NodeType.CUSTOM_ENTITY)
            elif isinstance(definition, CustomEventDefinition):
                return NODE_CLASS_BY_TYPE.get(NodeType.CUSTOM_EVENT)
            else:
                raise ValueError(f"unexpected object definition reference: {self!r}")
        elif self.type == ObjectDefinitionType.BUILTIN_TRAIT:
            assert self.trait_type is not None, f"no trait_type for {self!r}"
            return TRAIT_CLASS_BY_TYPE.get(self.trait_type)
        elif self.type == ObjectDefinitionType.BUILTIN_STRUCT:
            assert self.struct_type is not None, f"no struct_type for {self!r}"
            return STRUCT_CLASS_BY_TYPE.get(self.struct_type)
        elif (
            self.type == ObjectDefinitionType.CUSTOM_STRUCT
            or self.type == ObjectDefinitionType.CUSTOM_TRAIT
        ):
            raise NotImplementedError(f"unexpected object definition reference: {self!r}")
        else:
            assert_never(self.type)

    def resolve_property(self, name: str) -> "PropertyDeclaration | None":
        """Resolve a Property in this definition."""
        object_cls = self.object_cls
        if object_cls is None:
            raise ValueError(f"could not resolve {self!r}")
        return object_cls.__properties__.get(name)

    def resolve_property_or_error(self, name: str) -> "PropertyDeclaration":
        """Resolve a Property in this definition (error if not found)."""
        resolved = self.resolve_property(name)
        if resolved is None:
            raise LookupError(f"could not find property {name!r} in {self!r}")
        return resolved

    @classmethod
    def of(
        cls, base: "NodeType | type[NodeBase] | CustomEntityDefinition | type[StructBase]"
    ) -> "ObjectDefinitionReference":
        from .node import Node

        if isinstance(base, NodeType):
            return ObjectDefinitionReference(type=ObjectDefinitionType.BUILTIN_NODE, node_type=base)
        elif isinstance(base, StructType):
            return ObjectDefinitionReference(
                type=ObjectDefinitionType.BUILTIN_STRUCT, struct_type=base
            )
        elif isinstance(base, type):
            if issubclass(base, Node):
                return ObjectDefinitionReference(
                    type=ObjectDefinitionType.BUILTIN_NODE, node_type=base.metatype
                )
            elif issubclass(base, Trait):
                return ObjectDefinitionReference(
                    type=ObjectDefinitionType.BUILTIN_TRAIT, trait_type=base.metatype
                )
            elif issubclass(base, StructBase):
                return ObjectDefinitionReference(
                    type=ObjectDefinitionType.BUILTIN_STRUCT, struct_type=base.metatype
                )
            else:
                raise ValueError(f"invalid object reference type: {base!r}")
        elif isinstance(base, Node):
            return ObjectDefinitionReference(
                type=ObjectDefinitionType.CUSTOM_NODE,
                node_type=NodeType.CUSTOM_ENTITY,
                definition=base,
            )
        else:
            assert_never(base)


@builtin_enum(EnumType.STRUCT_DEFINITION_TYPE)
class StructDefinitionType(Enum):
    BUILTIN_STRUCT = 1
    CUSTOM_STRUCT = 2
    BUILTIN_ENUM = 3
    CUSTOM_ENUM = 4


@builtin_struct(StructType.STRUCT_DEFINITION_REFERENCE, frozen=True)
class StructDefinitionReference(StructFrozen):
    """Reference to a Struct definition (builtin, custom or by trait)."""

    type: StructDefinitionType = builtin_property(100, is_repr=True)
    struct_type: Optional[StructType] = builtin_property(101, is_repr=True)
    definition: "CustomStructDefinition" = builtin_property(105, is_repr=True)
    if TYPE_CHECKING:
        definition_ptr: Optional["NodeReference"] = None


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

    type: PropertyReferenceType = builtin_property(100, is_repr=True)
    node_type: NodeType | None = builtin_property(101, is_repr=True)
    trait_type: TraitType | None = builtin_property(102, is_repr=True)
    struct_type: StructType | None = builtin_property(103, is_repr=True)
    id: int | None = builtin_property(
        105,
        is_repr=True,
        primitive_type=PrimitiveType.INT32,
        description="id of the builtin Property",
    )
    custom_property: "CustomProperty | None" = builtin_property(
        106,
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

    type: NodeType = builtin_property(101, is_repr=True)
    id: UUID = builtin_property(102, is_repr=True)
    space_id: Optional[UUID] = builtin_property(103, is_repr=True)
    definition_id: Optional[UUID] = builtin_property(104, is_repr=True)
    # area? external_id?
