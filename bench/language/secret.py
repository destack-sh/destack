from typing import TYPE_CHECKING, Any, Union, cast, override

from bench.language.bench import AnonymousResourceNode
from bench.language.const import NodeType, ObjectKind, StructType
from bench.language.field import TypeInfo
from bench.language.node import (
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
from bench.proto.wire import SecretData, SecretReferenceData

if TYPE_CHECKING:
    from bench.language import Vault

# pyright: reportIncompatibleVariableOverride=false


@node_(NodeType.SECRET)
class Secret(AnonymousResourceNode[SecretData]):
    """A secret value."""

    parent: Union["Vault", None] = p_node_parent(4, NodeType.VAULT, is_system=True)

    # meta
    title: str = p_regular(33, constraint=TITLE_CONSTRAINT)

    # content
    value_type: TypeInfo = p_regular(40, struct=StructType.TYPE_INFO)
    value_packed: Any = p_value_packed(41, secret=True)
    value = p_value_runtime(
        41, kind=ObjectKind.MEMBER, typ=lambda self: cast(Secret, self).value_type
    )

    def to_ref(self) -> "SecretReference":
        """Gets a reference to this secret."""
        return SecretReference._ref_from_node(self)

    def _to_ref_data(self) -> SecretReferenceData:
        """Gets a data reference to this secret."""
        return SecretReference._ref_from_node(self)._to_data()


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

    def _validate_component(self, properties: tuple[Property, ...], invalid: ValidationHandler):
        if self.node_type != NodeType.SECRET:
            invalid(
                "type",
                f"referenced node must be Secret, got {self.node_type}",
                (SecretReference.node_type,),
            )

    @override
    @staticmethod
    def _ref_from_node(node: Secret) -> "SecretReference":
        node_ref = NodeReference._ref_from_node(node)
        return SecretReference._clone_ref(SecretReference, node_ref, title=node.title)

    @override
    @staticmethod
    def _ref_data_from_node_data(node_data: SecretData) -> SecretReferenceData:
        node_ref = NodeReference._ref_data_from_node_data(node_data)
        return SecretReference._clone_ref(SecretReferenceData, node_ref, title=node_data.title)
