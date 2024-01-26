from datetime import datetime
from typing import TYPE_CHECKING, Optional, Union
from uuid import UUID

from bench.language.const import (
    ActionKind,
    BadgeType,
    BenchType,
    NodeType,
    PolicyEffect,
    StructType,
    BenchError,
)
from bench.language.expression import PropertyReference
from bench.language.node import (
    Node,
    Struct,
    node,
    node_parent,
    struct,
    struct_internal,
    struct_runtime,
    Package,
)

if TYPE_CHECKING:
    from bench.language import Expression, User, Block


@struct(StructType.POLICY)
class Policy(Struct):
    """A policy regulating access to nodes within its scope."""

    name: Optional[str] = struct_internal(30, default=None)
    rules: list["PolicyRule"] = struct_internal(
        31, default_factory=list, struct_t=StructType.POLICY_RULE
    )
    hidden: bool = struct_internal(
        32, default=False, description="Hide this policy and its effects from the denied."
    )

    def __content_str__(self) -> str:
        return f"{self.name or '<unnamed>'} ({len(self.rules)} rules, {'hidden' if self.hidden else 'visible'})"


@struct(StructType.POLICY_RULE)
class PolicyRule(Struct):
    """A rule in a policy: <subject> + can/cannot <verb> + <object> [if condition]."""

    # subject
    subject_is_system: bool = struct_runtime(
        default=False
    )  # not stored because only we can be system
    subject_is_authenticated: bool = struct_internal(31, default=False)
    subject_is_owner: bool = struct_internal(32, default=False)
    subject_users: Optional[list["User"]] = struct_internal(
        33, require=False, array=True, references=NodeType.USER
    )
    # subject_groups, subject_identities, subject_roles, ...

    # verb
    effect: PolicyEffect = struct_internal(40)
    verb: Optional[list[ActionKind]] = struct_internal(41)

    # object
    object_types: Optional[list[BenchType]] = struct_internal(50, default=None)
    object_nodes: list[Node] | None = struct_internal(
        51, require=False, array=True, references=tuple(NodeType)
    )
    object_properties: list[PropertyReference] | None = struct_internal(
        52, require=False, array=True, struct_t=StructType.PROPERTY_REFERENCE
    )
    # object_fields: list["Field"] | None = struct_internal(
    #     53, require=False, array=True, references=NodeType.FIELD
    # )

    # [condition]
    condition: Optional["Expression"] = struct_internal(
        60, default=None, struct_t=StructType.EXPRESSION
    )


@struct(StructType.CONTEXT)
class Context(Struct):
    """The context of a request for evaluating a policy."""

    # subject
    subject_is_system: bool = struct_internal(30, default=False)
    subject_is_authenticated: bool = struct_internal(31, default=False)
    subject_is_owner: bool = struct_internal(32, default=False)
    subject_user: Optional["User"] = struct_internal(
        33, array=False, require=False, references=NodeType.USER
    )

    # verb
    verbs: list[ActionKind] = struct_internal(41, default_factory=list)

    # object
    object_types: list[BenchType] = struct_internal(50, default_factory=list)
    object_nodes: list[Node] | None = struct_internal(
        51, array=True, require=False, default_factory=list, references=tuple(NodeType)
    )
    object_properties: list[PropertyReference] | None = struct_internal(
        52, default_factory=list, struct_t=StructType.PROPERTY_REFERENCE
    )


@node(NodeType.BADGE)
class Badge(Node):
    """A badge for an unknown identity to access (parts of) this Bench."""

    parent: Union[Package, "Block"] = node_parent(4, NodeType.PACKAGE, NodeType.BLOCK)
    type: BadgeType = struct_internal(30)
    name: Optional[str] = struct_internal(31)
    policy: Policy = struct_internal(32, struct_t=StructType.POLICY)
    expires_at: Optional[datetime] = struct_internal(33, default=None)
    # sharing link badge
    link_token: Optional[UUID] = struct_internal(40, default=None)
    link_password: Optional[str] = struct_internal(41, default=None, encrypt=True, defer=True)
    link_password_digest: Optional[str] = struct_internal(42, default=None, encrypt=True)
    # access key badge
    key_value: Optional[str] = struct_internal(50, default=None, encrypt=True, defer=True)
    key_value_digest: Optional[str] = struct_internal(51, default=None, encrypt=True)


class AuthError(BenchError, ValueError):
    def __init__(
        self,
        node: Node,
        action: ActionKind,
        context: Context | None = None,
        cause: Exception | None = None,
    ):
        super().__init__(f"{node!r}: {action.value}")
        self.node = node
        self.action = action
        self.cause = cause
        self.context = context
