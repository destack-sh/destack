from datetime import datetime
from typing import TYPE_CHECKING, Optional, Union
from uuid import UUID

from bench.language.const import (
    ActionKind,
    BadgeType,
    BenchError,
    BenchType,
    NodeType,
    PolicyEffect,
    StructType,
)
from bench.language.expression import PropertyReference
from bench.language.node import (
    Node,
    Package,
    Struct,
    node,
    node_parent,
    struct,
    struct_internal,
)

if TYPE_CHECKING:
    from bench.language import Block


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
    subject_is_owner: bool = struct_internal(30, default=False)
    subject_is_authenticated: bool = struct_internal(31, default=False)
    # subject_users, subject_groups, subject_identities, subject_roles, ...

    # verb
    effect: PolicyEffect = struct_internal(40)
    verb: list[ActionKind] = struct_internal(41)

    # object
    object_types: Optional[list[BenchType]] = struct_internal(50, default=None)
    object_properties: list[PropertyReference] | None = struct_internal(
        51, require=False, array=True, struct_t=StructType.PROPERTY_REFERENCE
    )
    # object_nodes, object_fields, ...

    # [condition]
    # condition: Optional["Expression"] = struct_internal(
    #     60, default=None, struct_t=StructType.EXPRESSION
    # )


@struct(StructType.REQUEST_CONTEXT)
class RequestContext(Struct):
    """The context of a request for evaluating a policy."""

    # subject
    subject_is_owner: bool = struct_internal(30, default=False)
    subject_is_authenticated: bool = struct_internal(31, default=False)
    subject_is_staff: bool = struct_internal(32, default=False)
    # subject_user, subject_groups, subject_identities, subject_roles, ...

    # verb
    verbs: list[ActionKind] = struct_internal(41, default_factory=list)

    # object
    object_types: list[BenchType] = struct_internal(50, default_factory=list)
    object_properties: list[PropertyReference] | None = struct_internal(
        51, default_factory=list, struct_t=StructType.PROPERTY_REFERENCE
    )
    # object_nodes, object_fields, ...

    # [condition]
    # ... any extra values for evaluating the condition


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
    link_password: Optional[str] = struct_internal(
        41, default=None, encrypt=True, defer=True, sensitive=True
    )
    link_password_digest: Optional[str] = struct_internal(
        42, default=None, encrypt=True, defer=True, sensitive=True
    )
    # access key badge
    key_value: Optional[str] = struct_internal(
        50, default=None, encrypt=True, defer=True, sensitive=True
    )
    key_value_digest: Optional[str] = struct_internal(
        51, default=None, encrypt=True, defer=True, sensitive=True
    )


class AccessError(BenchError, ValueError):
    def __init__(
        self,
        node: Node,
        action: ActionKind,
        context: RequestContext | None = None,
        cause: Exception | None = None,
    ):
        super().__init__(f"{node!r}: {action.value}")
        self.node = node
        self.action = action
        self.cause = cause
        self.context = context
