from collections import defaultdict
from datetime import datetime
from typing import TYPE_CHECKING, Collection, NamedTuple, Optional, Self, Union
from uuid import UUID

from bench.language.const import (
    ACTION_KINDS,
    IN_BENCH_NODE_TYPES,
    SUB_PACKAGE_NODE_TYPES,
    ActionKind,
    ActionType,
    BadgeType,
    BenchError,
    BenchType,
    NodeType,
    PolicyEffect,
    ReadType,
    RunType,
    StructType,
)
from bench.language.node import (
    NODE_CLASS_BY_TYPE,
    Node,
    Package,
    Property,
    Struct,
    node,
    node_parent,
    struct,
    struct_internal,
    struct_runtime,
)
from bench.language.tree import NodeDataTree
from bench.language.user import Membership
from bench.proto.wire import AnyNodeData, EditData
from bench.utils.casing import IdentifierType

if TYPE_CHECKING:
    from bench.language import (
        Bench,
        Block,
        Client,
        Expression,
        Organization,
        PropertyReference,
        User,
    )


@struct(StructType.POLICY, identifier=IdentifierType.VARIABLE)
class Policy(Struct):
    """
    A policy regulating access to nodes within its scope.
    The scope is determined by where its attached, but may be further restricted using 'scopes'.

    The basics of access control:
     1. An action is denied implicitly if it's explicitly and fully allowed.
     2. Policies are attached directly to nodes or via delegates (badges, roles, identities, ...).
       a. Policies are scoped to the node their definition or
       b. delegate is attached to (or less as specified).
     3. Policies are evaluated in order up the node tree, first match decides.
        (This means you can read a sub block but not its parent.)
     4. Every applicable identity/role/... is evaluated separately and *any* allow wins.

     * 'owner' = User with owner-level access to the resp. root node (Bench/User/Organization/...).
        This is effectively the 'root user' who can do anything. Be careful.
    """

    name: Optional[str] = struct_internal(30, default=None)
    rules: list["PolicyRule"] = struct_internal(
        31, default_factory=list, struct=StructType.POLICY_RULE
    )
    hidden: bool = struct_internal(
        32, default=False, description="Hide this policy and its effects."
    )
    scopes: list["Block"] | None = struct_internal(
        33, default=None, require=False, array=True, references=NodeType.BLOCK
    )

    def __content_str__(self) -> str:
        return f"{self.name or '<unnamed>'} ({len(self.rules)} rules, {'hidden' if self.hidden else 'visible'})"

    def append(self, *rules: "PolicyRule") -> "Self":
        self.rules.extend(rules)
        return self


@struct(StructType.POLICY_RULE)
class PolicyRule(Struct):
    """
    A rule in a policy: <subject> + can/cannot <verb> + <object> [if condition].
    If set, subject/verb/object are ORed together, i.e. any overlap is a match.
    """

    # subject (if unset it's a wildcard)
    subject_is_delegated: bool = struct_internal(30, default=False)
    # if subject is delegated then it always matches if this rule is present
    #  (and no other subject filters make sense)
    subject_is_authenticated: bool = struct_internal(31, default=False)
    subject_is_member: bool = struct_internal(32, default=False)
    subject_is_owner: bool = struct_internal(33, default=False)
    subject_is_staff: bool = struct_internal(34, default=False)
    # subject_users, subject_groups, subject_identities, subject_roles, ...

    # verb
    effect: PolicyEffect = struct_internal(50, default=PolicyEffect.DENY)
    verbs: list[ActionType] | None = struct_internal(51, default=None)
    verb_kinds: list[ActionKind] | None = struct_internal(52, default=None)

    # object (if unset it's a wildcard)
    object_types: Optional[list[BenchType]] = struct_internal(70, default=None)
    object_is_sensitive: bool = struct_internal(71, default=False)
    object_is_system: bool = struct_internal(72, default=False)

    # object_properties, object_nodes, object_fields, ...

    # [condition]
    # condition: Expression ...

    def __content_str__(self) -> str:
        subject_str_parts = []
        for subject_key in (
            "subject_is_delegated",
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

    def matches_subject(self, subject: "RequestSubject", object: "RequestObject") -> bool:
        if not self.subject_is_delegated:  # subject always matches if it's delegated
            if self.subject_is_authenticated and not subject.is_authenticated:
                return False
            if self.subject_is_owner and object.owner not in subject.ownerships:
                return False
            if self.subject_is_staff and not subject.is_staff:
                return False
        return True  # no mismatch -> match

    def matches_verb(self, verb: ActionType) -> bool:
        if self.verbs and verb not in self.verbs:
            return False
        if self.verb_kinds and verb.kind not in self.verb_kinds:
            return False
        return True  # no mismatch -> match

    def matches_object(self, object: "RequestObject") -> bool:
        if self.object_types and object.type not in self.object_types:
            return False
        if self.object_is_sensitive and not object.is_sensitive:
            return False
        if self.object_is_system and not object.is_system:
            return False
        return True  # no mismatch -> match

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
    """
    Attach a badge to a node with an inline definition.
    A badge's policies are delegated to the 'holder' (any client presenting its secrets).
    The delegated policies apply at the parent scope OR given scopes (which must be below parent's).
    """

    parent: Union[Package, "Block"] = node_parent(4, NodeType.PACKAGE, NodeType.BLOCK)
    type: BadgeType = struct_internal(30)
    name: Optional[str] = struct_internal(31)
    delegated_policies: list[Policy] = struct_internal(32, array=True, struct=StructType.POLICY)
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
    """
    Attach a role to a block or member.
    Role policies are delegated to the parent and its descendants.
    The delegated policies apply to all descendant's actions.
    """

    parent: Union["Block", "Membership"] = node_parent(4, NodeType.BLOCK, NodeType.MEMBERSHIP)
    type: "Block" = struct_internal(30, array=False, require=True, references=NodeType.BLOCK)


@node(NodeType.IDENTITY)
class Identity(Node):
    """
    Attach an identity to a block or member.
    Identity policies are delegated to the parent and its descendants.
    The delegated policies apply to all descendant's actions.
    """

    parent: Union["Block", "Membership"] = node_parent(4, NodeType.BLOCK, NodeType.MEMBERSHIP)
    type: "Block" = struct_internal(30, array=False, require=True, references=NodeType.BLOCK)


@struct(StructType.READ_OPTIONS)
class ReadOptions(Struct):
    """
    Fine-grained options to a read request.
    This is an addition to primary options (like the filter for a search or aggregation).
    """

    # relations
    ancestor_types: list[NodeType] | None = struct_internal(30, default=None)
    descendant_types: list[NodeType] | None = struct_internal(31, default=None)
    related_properties: list["PropertyReference"] | None = struct_internal(
        32, default=None, struct=StructType.PROPERTY_REFERENCE
    )

    # selects
    include_sensitive: bool = struct_internal(40, default=False)
    # (runtime only since we can't / don't need to serialize maps yet)
    _select_properties_by_type: dict[NodeType, list[Property]] | None = struct_runtime(default=None)

    # filters
    global_filter: Optional["Expression"] = struct_internal(
        50, default=None, require=False, struct=StructType.EXPRESSION
    )
    # (see above for why runtime only)
    _filter_by_type: dict[NodeType, "Expression"] | None = struct_runtime(default=None)

    def combined_filter(
        self, node_type: NodeType, filter: Optional["Expression"] = None
    ) -> "Expression":
        from bench.sql.engine import DEFAULT_GLOBAL_FILTER

        type_filter = (
            self._filter_by_type.get(node_type) if self._filter_by_type is not None else None
        )
        global_filter = (
            self.global_filter if self.global_filter is not None else DEFAULT_GLOBAL_FILTER
        )
        if filter is None:
            if type_filter is None:
                return global_filter
            else:
                return global_filter & type_filter
        else:
            if type_filter is None:
                return global_filter & filter
            else:
                return global_filter & type_filter & filter

    def selected_properties(self, node_type: NodeType) -> list[Property]:
        from bench.sql.engine import SELECT_ALL_PROPERTIES, SELECT_DEFAULT_PROPERTIES

        if self._select_properties_by_type is not None:
            return self._select_properties_by_type.get(node_type, ())
        elif self.include_sensitive:
            return SELECT_ALL_PROPERTIES.get(node_type, ())
        else:
            return SELECT_DEFAULT_PROPERTIES.get(node_type, ())

    @staticmethod
    def default():
        from bench.sql.engine import DEFAULT_GLOBAL_FILTER

        return ReadOptions(
            ancestor_types=None,
            descendant_types=None,
            include_sensitive=False,
            global_filter=DEFAULT_GLOBAL_FILTER,
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
    ownerships: list[Union["User", "Organization", "Bench"]] = struct_internal(
        32,
        require=True,
        array=True,
        references=(NodeType.USER, NodeType.ORGANIZATION, NodeType.BENCH),
    )
    # memberships/roles/...?
    client: Optional["Client"] = struct_internal(
        35, default=None, require=False, array=False, references=NodeType.CLIENT
    )
    user: Optional["User"] = struct_internal(
        36, default=None, require=False, array=False, references=NodeType.USER
    )
    identity: Optional["Identity"] = struct_internal(
        37, default=None, require=False, array=False, references=NodeType.IDENTITY
    )
    badge: Optional["Badge"] = struct_internal(
        38, default=None, require=False, array=False, references=NodeType.BADGE
    )

    # groups, identities, roles, ...

    def split_into_available_subjects(self) -> tuple["RequestSubject", ...]:
        """
        Split into different subjects that may have different access.
        Basically, this is every 'identity' where we could say "acting as X".
        """

        applicable_principals: list[RequestSubject] = []
        if self.is_authenticated:
            applicable_principals.append(RequestSubject(is_authenticated=True))
        if self.is_staff:
            applicable_principals.append(RequestSubject(is_staff=True))
        if self.client:
            applicable_principals.append(RequestSubject(client=self.client))
        if self.user:
            applicable_principals.append(RequestSubject(user=self.user))
        if self.identity:
            applicable_principals.append(RequestSubject(identity=self.identity))
        if self.badge:
            applicable_principals.append(RequestSubject(badge=self.badge))
        return tuple(applicable_principals)

    def __content_str__(self):
        str_parts = []
        for prop in RequestSubject.__declared_properties__.values():
            value = getattr(self, prop.name)
            if value:
                if isinstance(value, bool):
                    str_parts.append(prop.name)
                else:
                    str_parts.append(repr(value))
        if str_parts:
            return f"{', '.join(str_parts)}"
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
        str_parts = []
        for object_key in ("is_sensitive", "is_system"):
            value = getattr(self, object_key)
            if value:
                if isinstance(value, bool):
                    str_parts.append(object_key)
                else:
                    str_parts.append(f"{object_key}={value}")
        if str_parts:
            return f"{self.type.bench_name} [{', '.join(str_parts)}]"
        else:
            return self.type.bench_name


@struct(StructType.REQUEST_EVALUATION)
class RequestEvaluation(Struct):
    """The result of evaluating a single request."""

    subject: RequestSubject = struct_internal(30, require=True, struct=StructType.REQUEST_SUBJECT)
    verb: ActionType = struct_internal(31, require=True)
    object: RequestObject = struct_internal(32, require=True, struct=StructType.REQUEST_OBJECT)
    deciding_rule: PolicyRule | None = struct_internal(
        33, default=None, struct=StructType.POLICY_RULE
    )
    decision: PolicyEffect = struct_internal(34, require=True)

    def __content_str__(self) -> str:
        return f"{self.decision} {self.subject} {self.verb.bench_name} {self.object} @ {self.deciding_rule}"

    @property
    def is_implicit(self) -> bool:
        return self.deciding_rule is None and self.decision == PolicyEffect.DENY


@struct(StructType.ACTION_EVALUATION)
class ActionEvaluation(Struct):
    """The result of evaluating an action."""

    request_evaluations: list[RequestEvaluation] = struct_internal(
        31, array=True, require=True, struct=StructType.REQUEST_EVALUATION
    )
    deciding_evaluation: RequestEvaluation | None = struct_internal(
        32, default=None, require=False, struct=StructType.REQUEST_EVALUATION
    )
    decision: PolicyEffect = struct_internal(33, require=True)

    def __content_str__(self) -> str:
        return f"{self.action} @ {self.request_evaluations}"

    @property
    def is_implicit(self) -> bool:
        return self.deciding_evaluation is None and self.decision == PolicyEffect.DENY


class AccessError(BenchError, ValueError):
    def __init__(
        self,
        evaluation: RequestEvaluation | ActionEvaluation,
        cause: Exception | None = None,
    ):
        super().__init__(f"denied: {evaluation}", cause)
        self.evaluation = evaluation
        self.cause = cause


def adapt_access_pre_read(
    subject: RequestSubject,
    options: ReadOptions,
    base_policies: Collection[Policy] = SYSTEM_POLICIES,
) -> ReadOptions:
    """
    Adapt read options based on the action to pre-filter as feasible while enabling the complete post-read check.
    Does NOT fully evaluate access yet, but avoids loading data that will be denied anyway.
    """

    # query ancestors up to root

    # if root == Bench, also load related actual owner (User/Organization)

    return options  # nocheckin


_EvalZoneKey = NamedTuple(
    "_EvalKey",
    [
        ("action", ActionType),
        ("scope", UUID),
        ("node_type", int),
        ("is_sensitive", bool),
        ("policies", int),
    ],
)


def evaluate_request(
    subject: RequestSubject,
    identities: list[RequestSubject] | tuple[RequestSubject, ...],
    verb: ActionType,
    object: RequestObject,
    policies: Collection[Policy],
) -> RequestEvaluation:
    # first pre-filter by verb/object
    applicable_rules: tuple[PolicyRule, ...] = tuple(
        rule
        for policy in policies
        for rule in policy.rules
        if rule.matches_verb(verb) and rule.matches_object(object)
    )
    # then evaluate by subject
    first_deny: PolicyRule | None = None
    for identity in identities:
        for rule in applicable_rules:
            if rule.matches_subject(identity, object):
                if rule.effect == PolicyEffect.ALLOW:
                    return RequestEvaluation(
                        subject=subject,
                        verb=verb,
                        object=object,
                        deciding_rule=rule,
                        decision=rule.effect,
                    )
                elif first_deny is None:
                    first_deny = rule
                    # continue, any allow wins

    # implicit deny if no applicable rules
    return RequestEvaluation(
        subject=subject,
        verb=verb,
        object=object,
        deciding_rule=first_deny,
        decision=PolicyEffect.DENY,
    )


def adapt_access_post_read(
    subject: RequestSubject,
    tree: NodeDataTree,
    options: ReadOptions,
    base_policies: list[Policy] | tuple[Policy, ...] = SYSTEM_POLICIES,
    root_owner: Optional[Union["User", "Organization", "Bench"]] = None,
) -> tuple[ActionEvaluation, Collection[AnyNodeData]]:
    """
    Adapt and evaluate access to all nodes in the given according to the given options.
    If no overall owner is given, the owners (i.e. actual roots) must be in the tree.
    Assumes that all policies are valid.
    """
    from bench.proto import wiring

    # 1. collect all policies into scopes
    #    (we use the id() of a policy set to cache evaluation result)
    owner_by_node_id: dict[str, Union["User", "Organization", "Bench"]] = {}
    region_by_node_id: dict[str, str] = {}
    policies_by_node_id: dict[str, list[Policy] | tuple[Policy, ...]] = defaultdict(list)
    roots = tree.find_roots()
    for root in roots:
        # figure out owner
        root_type = wiring.unpack_enum(NodeType, root.metatype)
        root_cls = NODE_CLASS_BY_TYPE[root_type]
        if root_cls.__roots__:
            if root_owner is None:
                raise ValueError(f"root requires an owner: {root}")
            owner = root_owner
        else:
            owner = root
        # nocheckin accumulate policies properly
        policies_by_node_id[root.id] = base_policies
        owner_by_node_id[root.id] = owner
        region_by_node_id[root.id] = root.id
        for descendant in tree.iter_descendants(root, recursive=True):
            descendant: AnyNodeData
            owner_by_node_id[descendant.id] = owner
            policies_by_node_id[descendant.id] = base_policies
            region_by_node_id[root.id] = root.id

    # 2. evaluate all requests to get at least one allow per principal
    identities = subject.split_into_available_subjects()
    eval_cache: dict[_EvalZoneKey, RequestEvaluation] = {}
    visible_nodes: list[AnyNodeData] = []
    for node in tree.nodes:
        node_type = node.metatype
        owner = owner_by_node_id[node.id]
        policies = policies_by_node_id[node.id]
        eval_key = _EvalZoneKey(ReadType.GET, owner.id, node_type, id(policies))
        if eval_key in eval_cache:
            evaluation = eval_cache[eval_key]
        else:
            object = RequestObject(
                type=node_type, owner=owner, include_sensitive=options.include_sensitive
            )
            evaluation = evaluate_request(subject, identities, eval_key.action, object, policies)
            eval_cache[eval_key] = evaluation
        if evaluation.decision == PolicyEffect.ALLOW:
            visible_nodes.append(node)

    action_evaluation = ActionEvaluation(
        request_evaluations=list(eval_cache.values()),
        deciding_evaluation=None,
        decision=PolicyEffect.ALLOW,  # nocheckin ???
    )
    return action_evaluation, visible_nodes


def evaluate_edits(
    base_policies: Collection[Policy], edits: Collection[EditData]
) -> ActionEvaluation:
    """
    Evaluates whether the given policies (base and in tree) allow the given edits.
    Assumes that all policies are valid.
    """
    raise NotImplementedError


def evaluate_run(
    base_policies: Collection[Policy], run_type: RunType, block: "Block"
) -> ActionEvaluation:
    """
    Evaluates whether the given policies (base and in tree) allow the given run action.
    Assumes that all policies are valid.
    """
    raise NotImplementedError
