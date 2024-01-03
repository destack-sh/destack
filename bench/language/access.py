from datetime import datetime
from typing import TYPE_CHECKING, Optional
from uuid import UUID

from bench.language.const import ActionKind, BenchType, NodeType, PolicyEffect, StructType
from bench.language.module import Bench, Node, Struct, node, node_parent, struct, struct_internal

if TYPE_CHECKING:
    from bench.language import Expression, Field, User


@struct(StructType.POLICY)
class Policy(Struct):
    """A policy regulating access to resources, usually within this scope."""

    name: Optional[str] = struct_internal(30, default=None)
    rules: list["PolicyRule"] = struct_internal(
        31, default_factory=list, struct_t=StructType.POLICY_RULE
    )


@struct(StructType.POLICY_RULE)
class PolicyRule(Struct):
    """A rule in a policy of the form subject + verb + object [if condition]."""

    # subject
    subject_authenticated: bool = struct_internal(30, default=False)
    subject_users: Optional[list["User"]] = struct_internal(31, references=NodeType.USER)
    # principal_groups, principal_roles, ...
    # nocheckin: ensure policy serializes correctly (PolicyRuleData.verb etc. is str??)
    # verb
    effect: PolicyEffect = struct_internal(40, default=PolicyEffect.ALLOW)
    verb: Optional[list[ActionKind]] = struct_internal(41, default=None)
    # object
    object_type: Optional[BenchType] = struct_internal(50, default=None)
    object_nodes: list[Node] | None = struct_internal(51, references=tuple(NodeType))
    object_fields: list["Field"] | None = struct_internal(52, references=NodeType.FIELD)
    # [condition]
    condition: Optional["Expression"] = struct_internal(
        60, default=None, struct_t=StructType.EXPRESSION
    )


@node(NodeType.BADGE, detached=True)
class Badge(Node):
    """A badge for a non-member to access parts of this Bench (via web or programmatically)."""

    parent: Bench = node_parent(4, NodeType.BENCH)
    archived_at: datetime = struct_internal(14, default=None, reflect=True)
    name: Optional[str] = struct_internal(30, default=None)
    policy: Policy = struct_internal(31, struct_t=StructType.POLICY)
    # sharing link
    link_enabled: bool = struct_internal(40, default=False)
    link_token: Optional[UUID] = struct_internal(41, default=None, unique=True)
    link_password_digest: Optional[str] = struct_internal(42, default=None, encrypt=True)
    # access token
    secret_enabled: bool = struct_internal(50, default=False)
    secret_value: Optional[str] = struct_internal(52, default=None, encrypt=True, defer=True)
    secret_value_digest: Optional[str] = struct_internal(51, default=None, encrypt=True)
