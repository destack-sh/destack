from typing import Any, cast, override

from bench.language.bench import Bench
from bench.language.const import NodeType, StructType
from bench.language.field import TypeInfo
from bench.language.node import (
    BenchNode,
    Node,
    NodeReference,
    NodeReferenceBase,
    Struct,
    node_,
    struct_,
)
from bench.language.property import (
    Property,
    p_node_parent,
    p_regular,
    p_value_packed,
    p_value_runtime,
)
from bench.language.validation import TITLE_CONSTRAINT, ValidationHandler
from bench.language.value import HasValues
from bench.proto.wire import SecretData, SecretReferenceData

# pyright: reportIncompatibleVariableOverride=false


@node_(NodeType.SECRET)
class Secret(BenchNode[SecretData], HasValues):
    """A secret value."""

    parent: Bench | None = p_node_parent(4, NodeType.BENCH, is_system=True)

    # meta
    title: str = p_regular(33, constraint=TITLE_CONSTRAINT)

    # content
    value_type: TypeInfo = p_regular(40, struct=StructType.TYPE_INFO)
    value_packed: Any = p_value_packed(41, secret=True)
    value = p_value_runtime(41, typ=lambda self: cast(Secret, self).value_type)


@struct_(StructType.SECRET_REFERENCE)
class SecretReference(
    Struct[SecretReferenceData],
    NodeReferenceBase[Secret, SecretData, "SecretReference", SecretReferenceData],
):
    """
    A reference to a Secret. Extends NodeReference with secret-specific metadata.
    """  # :RichReferences

    # ...NodeReferenceBase[30-39]

    title: str = p_regular(43, constraint=TITLE_CONSTRAINT)
    value_type: TypeInfo | None = p_regular(44, struct=StructType.TYPE_INFO)

    def _validate_component(self, properties: tuple[Property, ...], invalid: ValidationHandler):
        if self.type != NodeType.SECRET:
            invalid(
                "type", f"referenced node must be Secret, got {self.type}", (SecretReference.type,)
            )

    @override
    @staticmethod
    def from_node(node: Node) -> "SecretReference":
        node_ref = NodeReference.from_node(node)
        secret_ref = SecretReference._clone_ref(SecretReference, node_ref)
        for prop in SecretReference.__declared_properties__.values():
            if hasattr(secret_ref, prop.name):
                setattr(secret_ref, prop.name, getattr(node, prop.name))
        return secret_ref

    @override
    @staticmethod
    def from_node_data(node_data: SecretData) -> SecretReferenceData:
        node_ref = NodeReference.from_node_data(node_data)
        secret_ref = SecretReference._clone_ref(SecretReferenceData, node_ref)
        for prop in SecretReference.__declared_properties__.values():
            if hasattr(secret_ref, prop.name):
                setattr(secret_ref, prop.name, getattr(node_data, prop.name))
        return secret_ref

    @override
    @staticmethod
    def from_node_as_data(node: Secret) -> SecretReferenceData:
        node_ref = NodeReference.from_node_as_data(node)
        secret_ref = SecretReference._clone_ref(SecretReferenceData, node_ref)
        for prop in SecretReference.__declared_properties__.values():
            if hasattr(secret_ref, prop.name):
                setattr(secret_ref, prop.name, getattr(node, prop.name))
        return secret_ref
