from datetime import datetime
from typing import TYPE_CHECKING, Optional, Union, Collection, Self
from uuid import UUID

from bench.language.const import (
    ActionType,
    BadgeType,
    BenchError,
    BenchType,
    NodeType,
    PolicyEffect,
    StructType,
    ActionKind,
    ReadType,
    ACTION_KINDS,
    IN_BENCH_NODE_TYPES,
    SUB_PACKAGE_NODE_TYPES,
)
from bench.language.node import (
    Node,
    Package,
    Struct,
    node,
    node_parent,
    struct,
    struct_internal,
    struct_runtime,
    Property,
)
from bench.language.tree import NodeDataTree, NodeTree
from bench.language.user import Membership
from bench.proto.wire import EditData
from bench.utils.casing import IdentifierType

if TYPE_CHECKING:
    from bench.language import (
        Block,
        Expression,
        User,
        Client,
        Worker,
        Organization,
        Bench,
        PropertyReference,
    )


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


@struct(StructType.POLICY, identifier=IdentifierType.VARIABLE)
class Policy(Struct):
    """A policy regulating access to nodes within its scope."""

    name: Optional[str] = struct_internal(30, default=None)
    rules: list["PolicyRule"] = struct_internal(
        31, default_factory=list, struct=StructType.POLICY_RULE
    )
    hidden: bool = struct_internal(
        32, default=False, description="Hide this policy and its effects."
    )

    def append(self, *rules: "PolicyRule") -> "Self":
        self.rules.extend(rules)
        return self

    def __content_str__(self) -> str:
        return f"{self.name or '<unnamed>'} ({len(self.rules)} rules, {'hidden' if self.hidden else 'visible'})"


@struct(StructType.POLICY_RULE)
class PolicyRule(Struct):
    """
    A rule in a policy: <subject> + can/cannot <verb> + <object> [if condition].
    If set, subject/verb/object are ORed together, i.e. any overlap is a match.
    """

    # subject (if not set it's a wildcard)
    subject_is_authenticated: Optional[bool] = struct_internal(30, default=None)
    subject_is_member: Optional[bool] = struct_internal(31, default=None)
    subject_is_owner: Optional[bool] = struct_internal(32, default=None)
    subject_is_staff: Optional[bool] = struct_internal(33, default=None)
    # subject_users, subject_groups, subject_identities, subject_roles, ...

    # verb
    effect: PolicyEffect = struct_internal(50, default=PolicyEffect.DENY)
    verbs: list[ActionType] | None = struct_internal(51, default=None)
    verb_kinds: list[ActionKind] | None = struct_internal(52, default=None)

    # object (if not set it's a wildcard)
    object_types: Optional[list[BenchType]] = struct_internal(70, default=None)
    object_is_sensitive: Optional[bool] = struct_internal(71, default=None)
    object_is_system: Optional[bool] = struct_internal(72, default=None)

    # object_properties, object_nodes, object_fields, ...

    # [condition]
    # condition: Expression ...

    def __content_str__(self) -> str:
        subject_str_parts = []
        for subject_key in (
            "subject_is_authenticated",
            "subject_is_member",
            "subject_is_owner",
            "subject_is_staff",
        ):
            value = getattr(self, subject_key)
            if value is not None:
                subject_str_parts.append(f"{subject_key[8:]}={value}")
        if subject_str_parts:
            subject_str = f"[{', '.join(subject_str_parts)}]"
        else:
            subject_str = "*"

        verb_str_parts = []
        if self.verbs is not None:
            verb_str_parts.extend(verb.bench_name for verb in self.verbs)
        if self.verb_kinds is not None:
            verb_str_parts.extend(verb_kind.bench_name for verb_kind in self.verb_kinds)
        if verb_str_parts:
            verb_str = f"{', '.join(verb_str_parts)}"
        else:
            verb_str = "*"

        object_str_parts = []
        if self.object_types is not None:
            object_str_parts.extend(object_type.bench_name for object_type in self.object_types)
        for object_key in ("object_is_sensitive", "object_is_system"):
            value = getattr(self, object_key)
            if value is not None:
                object_str_parts.append(f"{object_key[7:]}={value}")
        if object_str_parts:
            object_str = f"[{', '.join(object_str_parts)}]"
        else:
            object_str = "*"

        return f"{self.effect} {subject_str} {verb_str} {object_str}"

    def matches(self, request: "Request") -> bool:
        """Checks if this rule matches (i.e. applies to) the given request."""

        # subject
        if self.subject_is_authenticated and not request.subject.is_authenticated:
            return False
        if self.subject_is_owner and request.object.owner not in request.subject.ownerships:
            return False
        if self.subject_is_member and request.subject.user not in request.subject.memberships:
            return False
        if self.subject_is_staff and not request.subject.is_staff:
            return False

        # verb
        if self.verbs and request.verb not in self.verbs:
            return False
        if self.verb_kinds and request.verb.kind not in self.verb_kinds:
            return False

        # object
        if self.object_types and request.object.type not in self.object_types:
            return False
        if self.object_is_sensitive and not request.object.is_sensitive:
            return False
        if self.object_is_system and not request.object.is_system:
            return False

        # no mismatch -> match
        return True

    #
    # Builder-style methods
    #

    def allow(self, *verbs: ActionType | ActionKind) -> "Self":
        self.effect = PolicyEffect.ALLOW
        self.verbs = [verb for verb in verbs if isinstance(verb, ActionType)]
        self.verb_kinds = [verb for verb in verbs if isinstance(verb, ActionKind)]
        return self

    def deny(self, *verbs: ActionType | ActionKind) -> "Self":
        self.effect = PolicyEffect.DENY
        self.verbs = [verb for verb in verbs if isinstance(verb, ActionType)]
        self.verb_kinds = [verb for verb in verbs if isinstance(verb, ActionKind)]
        return self

    def subject(
        self,
        is_authenticated: bool = None,
        is_member: bool = None,
        is_owner: bool = None,
        is_staff: bool = None,
    ) -> "Self":
        self.subject_is_authenticated = is_authenticated
        self.subject_is_member = is_member
        self.subject_is_owner = is_owner
        self.subject_is_staff = is_staff
        return self

    def object(
        self, *types: BenchType, is_sensitive: bool = None, is_system: bool = None
    ) -> "Self":
        self.object_types = list(types)
        self.object_is_sensitive = is_sensitive
        self.object_is_system = is_system
        return self


@node(NodeType.BADGE)
class Badge(Node):
    """Attach a badge to a node with an inline definition. Assumed by the 'holder', applies at the parent."""

    parent: Union[Package, "Block"] = node_parent(4, NodeType.PACKAGE, NodeType.BLOCK)
    type: BadgeType = struct_internal(30)
    name: Optional[str] = struct_internal(31)
    assumed_policies: list[Policy] = struct_internal(32, array=True, struct=StructType.POLICY)
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
    """Attach an identity to a block or member. Assumed by the parent, applies globally."""

    parent: Union["Block", "Membership"] = node_parent(4, NodeType.BLOCK, NodeType.MEMBERSHIP)
    type: "Block" = struct_internal(30, array=False, require=True, references=NodeType.BLOCK)


@struct(StructType.READ_OPTIONS)
class ReadOptions(Struct):
    """Load configuration for a read request."""

    ancestor_types: list[NodeType] | None = struct_internal(30, default=None)
    descendant_types: list[NodeType] | None = struct_internal(31, default=None)
    related_properties: list["PropertyReference"] | None = struct_internal(
        32, default=None, struct=StructType.PROPERTY_REFERENCE
    )

    include_sensitive: bool = struct_internal(40, default=False)

    global_filter: "Expression" = struct_internal(50, require=False, struct=StructType.EXPRESSION)

    # runtime only
    # (runtime only since we can't (and don't need to) serialize maps yet)
    filter_by_type: dict[NodeType, "Expression"] | None = struct_runtime(default=None)
    select_properties_by_type: dict[NodeType, list[Property]] | None = struct_runtime(default=None)

    def combined_filter(
        self, node_type: NodeType, filter: Optional["Expression"] = None
    ) -> "Expression":
        type_filter = self.filter_by_type.get(node_type)
        if filter is None:
            if type_filter is None:
                return self.global_filter
            else:
                return self.global_filter & type_filter
        else:
            if type_filter is None:
                return self.global_filter & filter
            else:
                return self.global_filter & type_filter & filter

    @staticmethod
    def default():
        from bench.sql.engine import GLOBAL_FILTER_DEFAULT

        return ReadOptions(
            ancestor_types=None,
            descendant_types=None,
            include_sensitive=False,
            global_filter=GLOBAL_FILTER_DEFAULT,
            filter_by_type=None,
        )


SYSTEM_POLICIES: tuple[Policy, ...] = (
    # order matters!
    Policy("OnlySystemCanEditSystem").append(
        PolicyRule().deny(ActionKind.EDIT).object(is_system=True)
    ),
    Policy("OwnerCanDoAnything").append(
        PolicyRule().subject(is_owner=True).allow(*ACTION_KINDS),
    ),
    Policy("MemberCanReadGlobal").append(
        PolicyRule()
        .subject(is_member=True)
        .allow(ActionKind.READ)
        # global = above package
        .object(*tuple(nt for nt in IN_BENCH_NODE_TYPES if nt not in SUB_PACKAGE_NODE_TYPES))
    ),
    Policy("AnyoneCanReadHandle").append(
        PolicyRule().subject().allow(ActionKind.READ).object(NodeType.HANDLE)
    ),
    Policy("AuthenticatedCanReadPublic").append(
        PolicyRule()
        .subject(is_authenticated=True)
        .allow(ActionKind.READ)
        .object(NodeType.USER, NodeType.ORGANIZATION, is_sensitive=False)
    ),
)


@struct(StructType.REQUEST_SUBJECT)
class RequestSubject(Struct):
    """The principal issuing a request. Unknown attributes are uninitialized."""

    is_authenticated: bool = struct_internal(30)
    is_staff: bool = struct_internal(31, default=False)
    memberships: list["Membership"] = struct_internal(
        32, require=True, array=True, references=NodeType.MEMBERSHIP
    )
    ownerships: list[Union["User", "Organization", "Bench"]] = struct_internal(
        33,
        require=True,
        array=True,
        references=(NodeType.USER, NodeType.ORGANIZATION, NodeType.BENCH),
    )
    client: Optional["Client"] = struct_internal(
        34, default=None, require=False, array=False, references=NodeType.CLIENT
    )
    user: Optional["User"] = struct_internal(
        35, default=None, require=False, array=False, references=NodeType.USER
    )
    worker: Optional["Worker"] = struct_internal(
        36, default=None, require=False, array=False, references=NodeType.WORKER
    )
    badge: Optional["Badge"] = struct_internal(
        37, default=None, require=False, array=False, references=NodeType.BADGE
    )

    # groups, identities, roles, ...

    def __content_str__(self):
        subject_str_parts = []
        for subject_key in (
            "client",
            "user",
            "badge",
            "is_authenticated",
            "is_owner",
            "is_member",
            "is_staff",
        ):
            value = getattr(self, subject_key)
            if value:
                if isinstance(value, bool):
                    subject_str_parts.append(subject_key)
                else:
                    subject_str_parts.append(repr(value))
        if subject_str_parts:
            return f"{', '.join(subject_str_parts)}"
        else:
            return "<anonymous>"


@struct(StructType.REQUEST_OBJECT)
class RequestObject(Struct):
    """
    The object of a request. Often refers to a summary of actual objects with identical properties.
    As with RequestSubject, unknown attributes are uninitialized.
    """

    type: BenchType = struct_internal(30)
    is_sensitive: bool = struct_internal(31, default=False)
    is_system: bool = struct_internal(32, default=False)
    owner: Union["User", "Organization", "Bench", None] = struct_internal(
        33, default=None, require=False, array=False, references=NodeType.BENCH
    )

    # properties, bases, node, fields, ...

    def __content_str__(self):
        object_str_parts = []
        for object_key in ("is_sensitive", "is_system"):
            value = getattr(self, object_key)
            if value:
                if isinstance(value, bool):
                    object_str_parts.append(object_key)
                else:
                    object_str_parts.append(f"{object_key}={value}")
        if object_str_parts:
            object_str = f" [{', '.join(object_str_parts)}]"
        else:
            object_str = ""
        return f"{self.type.bench_name}{object_str}"


@struct(StructType.REQUEST)
class Request(Struct):
    """The context of a single request."""

    subject: RequestSubject = struct_internal(30, require=True, struct=StructType.REQUEST_SUBJECT)
    verb: ActionType = struct_internal(31)
    object: RequestObject = struct_internal(32, require=True, struct=StructType.REQUEST_OBJECT)

    def __content_str__(self) -> str:
        return f"{self.subject} {self.verb.bench_name} {self.object}"

    @staticmethod
    def from_read_options(
        options: "ReadOptions", subject: RequestSubject
    ) -> list["Request"] | tuple["Request", ...]:
        """Map the read options to a list of requests."""
        requests: list[Request] = []
        is_sensitive = options.include_sensitive or False
        for type in options.ancestor_types or ():
            object = RequestObject(type=type, is_sensitive=is_sensitive)
            request = Request(subject=subject, verb=ReadType.LIST, object=object)
            requests.append(request)
        for type in options.descendant_types or ():
            object = RequestObject(type=type, is_sensitive=is_sensitive)
            request = Request(subject=subject, verb=ReadType.LIST, object=object)
            requests.append(request)
        return requests


@struct(StructType.ACTION)
class Action(Struct):
    """A set of sub-requests by the same subject."""

    subject: RequestSubject = struct_internal(30, require=True, struct=StructType.REQUEST_SUBJECT)
    requests: list[Request] = struct_internal(
        31, require=True, array=True, struct=StructType.REQUEST
    )

    def __content_str__(self) -> str:
        requests_str_parts = tuple(
            f"{request.verb.bench_name} {request.object}" for request in self.requests
        )
        requests_str = f"{', '.join(requests_str_parts)}"
        return f"{self.subject} {requests_str}"


@struct(StructType.REQUEST_EVALUATION)
class RequestEvaluation(Struct):
    """The result of evaluating a single request."""

    request: Request | None = struct_internal(30, default=None, struct=StructType.REQUEST)
    deciding_rule: PolicyRule | None = struct_internal(
        31, default=None, struct=StructType.POLICY_RULE
    )
    decision: PolicyEffect = struct_internal(32, require=True)

    def __content_str__(self) -> str:
        return f"{self.decision} {self.request} @ {self.deciding_rule}"


@struct(StructType.ACTION_EVALUATION)
class ActionEvaluation(Struct):
    """The result of evaluating an action."""

    action: Action | None = struct_internal(30, default=None, struct=StructType.ACTION)
    request_evaluations: list[RequestEvaluation] = struct_internal(
        31, array=True, require=True, struct=StructType.REQUEST_EVALUATION
    )
    deciding_evaluation: RequestEvaluation | None = struct_internal(
        32, default=None, require=False, struct=StructType.REQUEST_EVALUATION
    )
    decision: PolicyEffect = struct_internal(33, require=True)

    def __content_str__(self) -> str:
        return f"{self.action} @ {self.request_evaluations}"


class AccessError(BenchError, ValueError):
    def __init__(
        self,
        evaluation: RequestEvaluation | ActionEvaluation,
        cause: Exception | None = None,
    ):
        super().__init__(f"denied: {evaluation}", cause)
        self.evaluation = evaluation
        self.cause = cause


def evaluate_request(request: Request, policies: Collection[Policy]) -> RequestEvaluation:
    """Evaluate a single request against a list of policies (in order!)."""
    for policy in policies:
        for rule in policy.rules:
            if rule.matches(request):
                return RequestEvaluation(request=request, deciding_rule=rule, decision=rule.effect)

    # default: implicit deny
    return RequestEvaluation(request=request, decision=PolicyEffect.DENY)


def evaluate_action(action: Action, policies: Collection[Policy]) -> ActionEvaluation:
    """Evaluate an action against a list of policies (in order!)."""
    request_evaluations: list[RequestEvaluation] = []
    for request in action.requests:
        evaluation = evaluate_request(request, policies)
        request_evaluations.append(evaluation)
        if evaluation.decision == PolicyEffect.DENY:
            return ActionEvaluation(
                action=action,
                request_evaluations=request_evaluations,
                deciding_evaluation=evaluation,
                decision=PolicyEffect.DENY,
            )

    # all requests are allowed (no explicit or implicit deny)
    return ActionEvaluation(
        action=action,
        request_evaluations=request_evaluations,
        deciding_evaluation=None,
        decision=PolicyEffect.ALLOW,
    )


def adapt_access_pre_read(
    subject: RequestSubject,
    base_requests: Collection[Request],
    base_policies: Collection[Policy],
    options: ReadOptions,
) -> ReadOptions:
    """Adapt read options based on the action to pre-filter as feasible and enable the post-read check."""
    return options  # nocheckin ???


def adapt_access_post_read(
    subject: RequestSubject,
    base_requests: Collection[Request],
    base_policies: Collection[Policy],
    tree: NodeTree | NodeDataTree,
    options: ReadOptions,
) -> NodeTree | NodeDataTree:
    raise NotImplementedError


def check_access_edit(
    subject: RequestSubject,
    base_policies: Collection[Policy],
    edits: Collection[EditData],
    tree: NodeTree | NodeDataTree,
) -> NodeTree | NodeDataTree:
    raise NotImplementedError
