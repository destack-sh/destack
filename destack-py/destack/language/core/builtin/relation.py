from typing import (
    TYPE_CHECKING,
    Optional,
    Union,
    assert_never,
    cast,
)

from destack.language.registry import (
    NODE_CLASS_BY_TYPE,
    STRUCT_CLASS_BY_TYPE,
    TRAIT_CLASS_BY_TYPE,
)
from destack.proto import NodeReferenceProto, PropertyReferenceProto
from destack.utils.uuid import UUID

from .common import EnumType, NodeType, PrimitiveType, StoreType
from .enum import Enum, builtin_enum
from .object import BuiltinObject
from .property import PropertyDeclaration, builtin_property
from .struct import Struct, StructFrozen, StructType, builtin_struct
from .trait import Trait, TraitType

if TYPE_CHECKING:
    from destack.language import (
        CustomEntityDefinition,
        CustomEventDefinition,
        CustomProperty,
        CustomStructDefinition,
        CustomTraitDefinition,
        Node,
        PropertyDefinition,
    )

# pyright: reportIncompatibleVariableOverride=false

type_ = type


@builtin_enum(EnumType.NODE_DEFINITION_TYPE)
class NodeDefinitionType(Enum):
    BUILTIN = 1
    CUSTOM = 2


@builtin_struct(StructType.NODE_DEFINITION_REFERENCE, frozen=True)
class NodeDefinitionReference(StructFrozen):
    """Reference to a Node definition."""

    type: NodeDefinitionType = builtin_property(100, is_repr=True)
    node_type: NodeType = builtin_property(101, is_repr=True)
    definition: Union[
        "CustomEntityDefinition",
        "CustomEventDefinition",
        None,
    ] = builtin_property(105, is_repr=True)
    if TYPE_CHECKING:
        definition_ptr: Optional["NodeReference"] = None

    @property
    def is_multi(self) -> bool:
        """Whether this definition references multiple Node definitions"""
        if self.type == NodeDefinitionType.BUILTIN:
            assert self.node_type is not None, f"no node_type for {self!r}"
            node_cls = NODE_CLASS_BY_TYPE[self.node_type]
            return TraitType.EXTENSIBLE in node_cls.__traits__ and bool(node_cls.__extended_by__)
        elif self.type == NodeDefinitionType.CUSTOM:
            raise NotImplementedError(f"unexpected node definition reference: {self!r}")
        else:
            assert_never(self.type)

    @property
    def object_cls(self) -> type_[BuiltinObject] | None:
        return NODE_CLASS_BY_TYPE.get(self.node_type)

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
        cls, base: "NodeType | type[Node] | CustomEntityDefinition"
    ) -> "NodeDefinitionReference":
        from .node import Node

        if isinstance(base, NodeType):
            return NodeDefinitionReference(type=NodeDefinitionType.BUILTIN, node_type=base)
        elif isinstance(base, type):
            if issubclass(base, Node):
                return NodeDefinitionReference(
                    type=NodeDefinitionType.BUILTIN, node_type=base.metatype
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
    def object_cls(self) -> type_[BuiltinObject] | None:
        if self.type == ObjectDefinitionType.BUILTIN_NODE:
            assert self.node_type is not None, f"no node_type for {self!r}"
            return NODE_CLASS_BY_TYPE.get(self.node_type)
        elif self.type == ObjectDefinitionType.CUSTOM_NODE:
            raise NotImplementedError(f"unexpected object definition reference: {self!r}")
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
        cls,
        base: Union[
            "NodeType",
            "type[Node]",
            "type[Trait]",
            "type[Struct]",
            "CustomEntityDefinition",
            "CustomEventDefinition",
            "CustomTraitDefinition",
        ],
    ) -> "ObjectDefinitionReference":
        from .entity import CustomEntityDefinition, CustomTraitDefinition
        from .event import CustomEventDefinition
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
                    type=ObjectDefinitionType.BUILTIN_NODE,
                    node_type=cast(NodeType, base.metatype),
                )
            elif issubclass(base, Trait):
                return ObjectDefinitionReference(
                    type=ObjectDefinitionType.BUILTIN_TRAIT,
                    trait_type=cast(TraitType, base.metatype),
                )
            elif issubclass(base, Struct):
                return ObjectDefinitionReference(
                    type=ObjectDefinitionType.BUILTIN_STRUCT,
                    struct_type=base.metatype,
                )
            else:
                raise ValueError(f"invalid object reference type: {base!r}")
        elif isinstance(
            base, (CustomEntityDefinition, CustomEventDefinition, CustomTraitDefinition)
        ):
            raise NotImplementedError(f"unexpected object definition reference: {base!r}")
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

    def resolve_or_error(self) -> "PropertyDefinition | CustomProperty":
        """Resolves the property reference to a Property."""
        resolved = self.resolve()
        if resolved is None:
            raise ValueError(f"could not resolve {self!r}")
        return resolved

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
            assert base.parent_ptr is not None, f"no parent for {base!r}"
            return PropertyReference(
                type=PropertyReferenceType.CUSTOM,
                node_type=base.parent_ptr.type,
                custom_property=base,
            )
        else:
            assert_never(base)


@builtin_struct(StructType.NODE_REFERENCE, frozen=True)
class NodeReference(StructFrozen[NodeReferenceProto]):
    """
    A reference to a Node (builtin or custom).
    """

    # identity
    type: NodeType = builtin_property(
        100,
        is_repr=True,
        description="The type of the Node.",
    )
    id: UUID = builtin_property(
        101,
        is_repr=True,
        description="The unique id of the Node.",
    )
    definition_id: Optional[UUID] = builtin_property(
        102,
        is_repr=True,
        description="The unique id of the custom Node definition.",
    )
    snapshot_id: Optional[UUID] = builtin_property(
        103,
        is_repr=True,
        description="The id of the Snapshot the Node belonged to.",
    )
    # snapshot_time/epoch/...?

    # location
    space_id: Optional[UUID] = builtin_property(
        110,
        is_repr=True,
        description="The id of the Space the Node belonged to.",
    )
    store_type: Optional[StoreType] = builtin_property(
        111,
        is_repr=True,
        description="The type of the Store the Node belonged to.",
    )

    # external?
    # external_id?
