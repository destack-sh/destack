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
    ActionKindSet,
)
from bench.language.node import (
    Node,
    Package,
    Struct,
    node,
    node_parent,
    struct,
    struct_internal,
)
from bench.language.user import Membership

if TYPE_CHECKING:
    from bench.language import Block, Expression


#
# Basics of access control:
#  0. Access is always implicitly denied if not otherwise specified.
#  1. Policies can be attached directly to nodes or via delegates (badges, roles, identities, ...).
#  2. Policies are scoped to the node their definition or delegate is attached to.
#  3. Policy rules are evaluated in order up the node tree, first match decides.
#  4. If multiple subject identities apply, any allow wins (i.e. the most permissive is granted).
#  5. The 'owner' is a user with owner-level access to the resp. root node (Bench/User/Organization/...).
#     This is effectively the 'root user' who can do anything.
#


@struct(StructType.POLICY)
class Policy(Struct):
    """A policy regulating access to nodes within its scope."""

    name: Optional[str] = struct_internal(30, default=None)
    rules: list["PolicyRule"] = struct_internal(
        31, default_factory=list, struct_t=StructType.POLICY_RULE
    )
    hidden: bool = struct_internal(
        32, default=False, description="Hide this policy and its effects."
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
    verb_effect: PolicyEffect = struct_internal(40)
    verb_sets: list[ActionKindSet] = struct_internal(41)

    # object
    object_types: Optional[list[BenchType]] = struct_internal(50, default=None)
    object_is_sensitive: bool = struct_internal(51, default=False)
    # object_properties, object_nodes, object_fields, ...

    # [condition]
    # condition: Expression ...


@struct(StructType.REQUEST_CONTEXT)
class RequestContext(Struct):
    """The context of a request for evaluating a policy."""

    # subject
    subject_is_owner: bool = struct_internal(30, default=False)
    subject_is_authenticated: bool = struct_internal(31, default=False)
    # subject_user, subject_groups, subject_identities, subject_roles, ...

    # verb
    verbs: list[ActionKind] = struct_internal(41, default_factory=list)

    # object
    object_types: list[BenchType] = struct_internal(50, default_factory=list)
    object_is_sensitive: bool = struct_internal(51, default=False)
    # object_properties, object_nodes, object_fields, ...

    # [condition]
    # ... any extra values for evaluating the condition


@node(NodeType.BADGE)
class Badge(Node):
    """Attach a badge to a node with an inline definition. Assumed by the 'holder', applies at the parent."""

    parent: Union[Package, "Block"] = node_parent(4, NodeType.PACKAGE, NodeType.BLOCK)
    type: BadgeType = struct_internal(30)
    name: Optional[str] = struct_internal(31)
    assumed_policies: list[Policy] = struct_internal(32, array=True, struct_t=StructType.POLICY)
    expires_at: Optional[datetime] = struct_internal(33, default=None)
    # sharing link badge
    link_token: Optional[UUID] = struct_internal(40, unique=True, default=None)
    link_password: Optional[str] = struct_internal(
        41, default=None, encrypt=True, defer=True, sensitive=True
    )
    link_password_hash: Optional[str] = struct_internal(
        42, default=None, encrypt=True, defer=True, sensitive=True
    )
    # access key badge
    key_value: Optional[str] = struct_internal(
        50, default=None, encrypt=True, defer=True, sensitive=True
    )
    key_value_hash: Optional[str] = struct_internal(
        51, default=None, encrypt=True, defer=True, sensitive=True
    )


@node(NodeType.ROLE)
class Role(Node):
    """Attach a role to a block or member. Assumed by the parent, applies globally."""

    parent: Union["Block", "Membership"] = node_parent(4, NodeType.BLOCK, NodeType.MEMBERSHIP)
    type: "Block" = struct_internal(30, array=False, require=True, references=NodeType.BLOCK)


@node(NodeType.IDENTITY)
class Identity(Node):
    """Attach and optionally define (inline) a block or member. Assumed by the parent, applies globally."""

    parent: Union["Block", "Membership"] = node_parent(4, NodeType.BLOCK, NodeType.MEMBERSHIP)
    type: "Block" = struct_internal(30, array=False, require=True, references=NodeType.BLOCK)


@struct(StructType.READ_OPTIONS)
class ReadOptions(Struct):
    """Basic read request for nodes with relations & properties."""

    ancestor_types: list[NodeType] | None = struct_internal(30, default=None)
    descendant_types: list[NodeType] | None = struct_internal(31, default=None)
    include_sensitive: bool = struct_internal(32, default=False)
    global_filter: Optional["Expression"] = struct_internal(
        35, default=None, struct_t=StructType.EXPRESSION
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
