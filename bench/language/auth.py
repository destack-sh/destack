from datetime import datetime
from typing import TYPE_CHECKING, Optional
from uuid import UUID

from bench.language.const import (
    ActionKind,
    BadgeType,
    BenchType,
    NodeType,
    PolicyEffect,
    StructType,
)
from bench.language.module import Bench, Node, Struct, node, node_parent, struct, struct_internal

if TYPE_CHECKING:
    from bench.language import Expression, Field, User


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


@struct(StructType.POLICY_RULE)
class PolicyRule(Struct):
    """A rule in a policy: <subject> + can/cannot <verb> + <object> [if condition]."""

    # subject
    subject_authenticated: bool = struct_internal(30, default=False)
    subject_users: Optional[list["User"]] = struct_internal(
        31, array=True, references=NodeType.USER
    )
    # subject_groups, subject_roles, ...
    # verb
    effect: PolicyEffect = struct_internal(40)
    verb: Optional[list[ActionKind]] = struct_internal(41)
    # object
    object_types: Optional[list[BenchType]] = struct_internal(50, default=None)
    object_nodes: list[Node] | None = struct_internal(51, array=True, references=tuple(NodeType))
    object_fields: list["Field"] | None = struct_internal(52, array=True, references=NodeType.FIELD)
    # [condition]
    condition: Optional["Expression"] = struct_internal(
        60, default=None, struct_t=StructType.EXPRESSION
    )


@node(NodeType.BADGE, in_module=False)
class Badge(Node):
    """A badge for a non-member to access parts of this Bench (via web or programmatically)."""

    parent: Bench = node_parent(4, NodeType.BENCH)
    type: BadgeType = struct_internal(30)
    name: Optional[str] = struct_internal(31)
    policy: Policy = struct_internal(32, struct_t=StructType.POLICY)
    expires_at: Optional[datetime] = struct_internal(33, default=None)
    # sharing link badge
    link_token: Optional[UUID] = struct_internal(40, default=None, unique=True)
    link_password: Optional[str] = struct_internal(41, default=None, encrypt=True, defer=True)
    link_password_digest: Optional[str] = struct_internal(42, default=None, encrypt=True)
    # access key badge
    key_value: Optional[str] = struct_internal(50, default=None, encrypt=True, defer=True)
    key_value_digest: Optional[str] = struct_internal(51, default=None, encrypt=True)
