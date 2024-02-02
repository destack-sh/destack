from datetime import datetime
from typing import TYPE_CHECKING, Collection, Optional, Self, Union
from uuid import UUID

from bitarray import bitarray

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
    RunType,
    StructType,
    NODE_TYPES,
    EditType,
)
from bench.language.node import (
    Node,
    Package,
    Property,
    Struct,
    node,
    node_parent,
    struct,
    struct_internal,
    struct_runtime,
    iter_properties,
    ANCESTOR_NODE_TYPES,
    NODE_CLASS_BY_TYPE,
    DESCENDANT_NODE_TYPES,
)
from bench.language.notice import NoticeHandler
from bench.language.tree import NodeDataTree, NodeTree
from bench.language.user import Membership
from bench.proto.core import ProtoStrEnum
from bench.proto.wire import AnyNodeData, EditData
from bench.utils.casing import IdentifierType

if TYPE_CHECKING:
    from bench.language import (
        Bench,
        Block,
        Client,
        Expression,
        PropertyReference,
        User,
        Space,
        ScopeNode,
    )

Owner = Union["User", "Organization", "Bench"]


# 'owner' refers to both the root node and any user with root-level access to that root node.
# This is effectively the 'root user' who can do anything with descendants of the root node*.
#  (unless a system policy says otherwise)


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
    delegated_policies: list["Policy"] = struct_internal(32, array=True, struct=StructType.POLICY)
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
    related_types_via: list["PropertyReference"] | None = struct_internal(
        32, default=None, struct=StructType.PROPERTY_REFERENCE
    )

    # properties (include/exclude relative to default)
    include_properties: list[PropertyReference] | None = struct_internal(
        40, default=None, array=True, struct=StructType.PROPERTY_REFERENCE
    )
    exclude_properties: list[PropertyReference] | None = struct_internal(
        41, default=None, array=True, struct=StructType.PROPERTY_REFERENCE
    )

    # filters (global + per type)
    global_filter: Optional["Expression"] = struct_internal(
        50, default=None, require=False, struct=StructType.EXPRESSION
    )
    # (by type is runtime only since we can't / don't need to serialize maps yet)
    _filter_by_type: dict[NodeType, "Expression"] | None = struct_runtime(default=None)

    def copy(self) -> "ReadOptions":
        return ReadOptions(
            ancestor_types=self.ancestor_types,
            descendant_types=self.descendant_types,
            related_types_via=self.related_types_via,
            include_properties=self.include_properties,
            exclude_properties=self.exclude_properties,
            global_filter=self.global_filter,
            _filter_by_type=self._filter_by_type,
        )

    def related_properties(self, node_type: NodeType) -> list[Property] | tuple[Property, ...]:
        return tuple(p.resolved_property for p in self.related_types_via if p.type == node_type)

    def selected_properties(self, node_type: NodeType) -> list[Property] | tuple[Property, ...]:
        from bench.sql.engine import DEFAULT_SELECTED_PROPERTIES

        # a bit ugly but the tuples are very small
        properties = DEFAULT_SELECTED_PROPERTIES.get(node_type, ())
        if self.include_properties is not None and any(
            p.type == node_type for p in self.include_properties
        ):
            properties = properties + tuple(
                p.resolved_property for p in self.include_properties if p.type == node_type
            )
        if self.exclude_properties is not None and any(
            p.type == node_type for p in self.exclude_properties
        ):
            properties = tuple(
                p for p in properties if not any(e.id == p.id for e in self.exclude_properties)
            )
        return properties

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

    @staticmethod
    def default():
        from bench.sql.engine import DEFAULT_GLOBAL_FILTER

        return ReadOptions(
            ancestor_types=None,
            descendant_types=None,
            include_sensitive=False,
            global_filter=DEFAULT_GLOBAL_FILTER,
        )


@struct(StructType.POLICY, identifier=IdentifierType.VARIABLE)
class Policy(Struct):
    """
    A policy regulating access to nodes within its scope.
    The scope is determined by where its attached, but may be further restricted using 'scopes'.

    The basics of access control:
     1. An action is DENYed implicitly unless explicitly and completely ALLOWed.
     2. Policies are attached directly to nodes or via delegates (badges, roles, identities, ...).
       a. Policies are scoped to the node their definition or
       b. Delegate is attached to (or less as specified).
     3. Policies are evaluated in order up the node tree, first match decides*.
        (This means you can read a sub block but not its parent.)
     4. Every identity/role/... applicable to a subject is evaluated separately and *any* allow wins.

     * Conceptually, we do 'ray trace' up the tree for every node, but actually doing it for every request
        is prohibitively expensive. Instead, we 'rasterize' an 'access matrix' and use that as a shortcut.

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
    # if subject is delegated then it always matches if this rule is present
    #  (and no other subject filters make sense)
    subject_is_delegated: bool = struct_internal(30, default=False)
    subject_is_authenticated: bool = struct_internal(31, default=False)
    subject_is_staff: bool = struct_internal(32, default=False)
    # member/owner relative to the object
    subject_is_member: bool = struct_internal(33, default=False)
    subject_is_owner: bool = struct_internal(34, default=False)
    # subject_users, subject_groups, subject_identities, subject_roles, ...

    # verb
    effect: PolicyEffect = struct_internal(50, default=PolicyEffect.DENY)
    verbs: list[ActionType] | None = struct_internal(51, default=None)
    verb_kinds: list[ActionKind] | None = struct_internal(52, default=None)
    _verb_mask: bitarray | None = struct_runtime(default=None)

    # object (if unset it's a wildcard)
    object_node_types: Optional[list[NodeType]] = struct_internal(70, default=None)
    _object_node_types_mask: bitarray | None = struct_runtime(default=None)
    object_properties: list[PropertyReference] | None = struct_internal(
        73, default=None, struct=StructType.PROPERTY_REFERENCE
    )
    _object_properties_mask_by_node_type: dict[NodeType, bitarray] | None = struct_runtime(
        default=None
    )

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
        if self.object_node_types is not None:
            object_str_parts.extend(
                object_type.bench_name for object_type in self.object_node_types
            )
        if self.object_properties is not None:
            object_str_parts.extend(
                "." + ",".join(prop._resolved_property.name for prop in self.object_properties)
            )
        else:
            object_str_parts.append(".[*]")
        if object_str_parts:
            object_str = f"[{', '.join(object_str_parts)}]"
        else:
            object_str = "*"

        return f"{self.effect} {subject_str} {verb_str} {object_str}"

    def _clear_inner(self, scope: Optional["ScopeNode"] = None):
        self._verb_mask = None
        self._object_node_types_mask = None
        if self._object_properties_mask_by_node_type is not None:
            self._object_properties_mask_by_node_type.clear()

    def _interp_inner(self, scope: "ScopeNode", on_notice: "NoticeHandler"):
        self._update_verb_mask()
        self._update_object_mask()

    def _update_object_mask(self):
        self._object_node_types_mask = _enums_to_mask(self.object_node_types)
        if self.object_properties:
            self._object_properties_mask_by_node_type = {}
            for prop in self.object_properties:
                if prop.type not in self._object_properties_mask_by_node_type:
                    self._object_properties_mask_by_node_type[prop.type] = bitarray()
                self._object_properties_mask_by_node_type[prop.type][prop.id] = True

    def _update_verb_mask(self):
        self._verb_mask = bitarray()
        if not self.verbs and not self.verb_kinds:
            self._verb_mask[ActionType.__MIN_ID__ : ActionType.__MAX_ID__] = True
        else:
            for verb in self.verbs or ():
                self._verb_mask[verb.id] = True
            for verb_kind in self.verb_kinds or ():
                self._verb_mask[verb_kind.from_id : verb_kind.to_id + 1] = True

    def matches_subject(self, subject: "RequestSubject", object_owner: Owner) -> bool:
        if not self.subject_is_delegated:  # subject always matches if it's delegated
            if self.subject_is_authenticated and not subject.is_authenticated:
                return False
            if self.subject_is_owner and object_owner not in subject.ownerships:
                return False
            if self.subject_is_staff and not subject.is_staff:
                return False
        return True  # no mismatch -> match

    @property
    def is_verb_wildcard(self) -> bool:
        return not self.verbs and not self.verb_kinds

    def matches_verb(self, verb: ActionType) -> bool:
        if self._verb_mask:
            return self._verb_mask[verb.id]
        return True

    @property
    def is_object_wildcard(self) -> bool:
        return not self.object_node_types and not self.object_properties

    def matches_object(self, object: "RequestObject") -> bool:
        if (
            self._object_node_types_mask is not None
            and not self._object_node_types_mask[object.node_type.id]
        ):
            return False
        if self._object_properties_mask_by_node_type is not None:
            properties_mask = self._object_properties_mask_by_node_type.get(object.node_type)
            if (
                properties_mask is not None
                and object._properties_mask is not None
                and not properties_mask & object._properties_mask
            ):
                return False  # any overlap is a match
        return True

    #
    # Builder-style methods
    #

    def allow(self, *verbs: ActionType | ActionKind) -> "Self":
        self.effect = PolicyEffect.ALLOW
        self.verbs = [verb for verb in verbs if isinstance(verb, ActionType)]
        self.verb_kinds = [verb for verb in verbs if isinstance(verb, ActionKind)]
        self._update_verb_mask()
        return self

    def deny(self, *verbs: ActionType | ActionKind) -> "Self":
        self.effect = PolicyEffect.DENY
        self.verbs = [verb for verb in verbs if isinstance(verb, ActionType)]
        self.verb_kinds = [verb for verb in verbs if isinstance(verb, ActionKind)]
        self._update_verb_mask()
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
        self, node_types: tuple[BenchType, ...] = (), properties: tuple[Property, ...] = ()
    ) -> "Self":
        self.object_node_types = node_types  # type: ignore
        self.object_properties = properties  # type: ignore
        self._update_object_mask()
        return self


@struct(StructType.REQUEST_SUBJECT)
class RequestSubject(Struct):
    """The principal issuing a request. Unknown attributes are uninitialized."""

    is_authenticated: bool = struct_internal(30)
    is_staff: bool = struct_internal(31, default=False)
    ownerships: list[Owner] = struct_internal(
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

    def split_into_acting_subjects(self) -> tuple["RequestSubject", ...]:
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
    The object of a request. Often refers to multiple actual objects with shared attributes.
    """

    node_type: BenchType = struct_internal(30)
    properties: list[PropertyReference] | None = struct_internal(
        31, default=None, struct=StructType.PROPERTY_REFERENCE
    )
    owner: Optional[Owner] = struct_internal(
        32,
        default=None,
        require=False,
        references=(NodeType.USER, NodeType.ORGANIZATION, NodeType.BENCH),
    )
    _properties_mask: bitarray | None = struct_runtime(default=None)

    # properties, bases, node, fields, ...

    def __content_str__(self):
        str_parts = []
        for object_key in ("properties", "owner"):
            value = getattr(self, object_key)
            if value:
                if isinstance(value, bool):
                    str_parts.append(object_key)
                else:
                    str_parts.append(f"{object_key}={value}")
        if str_parts:
            return f"{self.node_type.bench_name} [{', '.join(str_parts)}]"
        else:
            return self.node_type.bench_name

    def _clear_inner(self, scope: Optional["ScopeNode"] = None):
        self._properties_mask = None

    def _interp_inner(self, scope: "ScopeNode", on_notice: "NoticeHandler"):
        if self.properties:
            self._properties_mask = _enums_to_mask(self.properties)


@struct(StructType.RULE_EVALUATION)
class RuleEvaluation(Struct):
    """The result of evaluating access to a tree for a subject."""

    subject: RequestSubject = struct_internal(30, require=True, struct=StructType.REQUEST_SUBJECT)
    matched_rule: PolicyRule = struct_internal(31, default=None, struct=StructType.POLICY_RULE)
    allowed_verbs: list[ActionType] | None = struct_internal(32, default=None)
    denied_verbs: list[ActionType] | None = struct_internal(33, default=None)
    _allow_verb_mask: bitarray | None = struct_runtime(default=None)
    _deny_verb_mask: bitarray | None = struct_runtime(default=None)


@struct(StructType.ACCESS_ZONE)
class AccessZone(Struct):
    """
    Materialized access grant to a region of the tree (<=scope) for objects matching the criteria.
    The closest matching zone 'up' for a verb+object determines access.
    Because zones are specific to object criteria, there may be multiple overlapping zones for a single scope.
    """

    id: int = struct_internal(2, require=True)
    scope: Union["Bench", "Package", "Block", "Space"] = struct_internal(30, require=True)
    _parent: Optional["AccessZone"] = struct_runtime(default=None)

    allowed_verbs: list[ActionType] = struct_internal(40)
    _allow_verb_mask: bitarray | None = struct_runtime(default=None)

    object_node_types: list[NodeType] | None = struct_internal(50, default=None)
    _object_node_types_mask: bitarray | None = struct_runtime(default=None)
    object_properties: list[PropertyReference] | None = struct_internal(
        53, default=None, struct=StructType.PROPERTY_REFERENCE
    )
    _object_properties_mask_by_node_type: dict[NodeType, bitarray] | None = struct_runtime(
        default=None
    )

    def __content_str__(self) -> str:
        if self.scope is None:
            scope_str = self.scope_ptr
        else:
            scope_str = self.scope.path

        allowed_verbs_str = ", ".join(v.bench_name for v in self.allowed_verbs)

        object_str_parts = []
        if self.object_node_types is not None:
            object_str_parts.extend(
                object_type.bench_name for object_type in self.object_node_types
            )
        for object_key in ("object_is_sensitive", "object_is_system"):
            value = getattr(self, object_key)
            if value is not None:
                object_str_parts.append(f"{object_key[7:]}={value}")
        if object_str_parts:
            object_str = f"[{', '.join(object_str_parts)}]"
        else:
            object_str = "*"

        return f"{self.id}: {PolicyEffect.ALLOW} {allowed_verbs_str} {object_str} in {scope_str}"

    def _clear_inner(self, scope: Optional["ScopeNode"] = None):
        self._allow_verb_mask = None
        self._object_node_types_mask = None
        if self._object_properties_mask_by_node_type is not None:
            self._object_properties_mask_by_node_type.clear()

    def _interp_inner(self, scope: "ScopeNode", on_notice: "NoticeHandler"):
        self._update_verb_mask()
        self._update_object_mask()

    def _update_verb_mask(self):
        self._allow_verb_mask = _enums_to_mask(self.allowed_verbs)

    def _update_object_mask(self):
        self._object_node_types_mask = _enums_to_mask(self.object_node_types)
        if self.object_properties:
            self._object_properties_mask_by_node_type = {}
            for prop in self.object_properties:
                if prop.type not in self._object_properties_mask_by_node_type:
                    self._object_properties_mask_by_node_type[prop.type] = bitarray()
                self._object_properties_mask_by_node_type[prop.type][prop.id] = True

    @property
    def is_verb_wildcard(self) -> bool:
        return not self.allowed_verbs

    def allows_verb(self, verb: ActionType) -> bool:
        """Whether the zone allows the verb."""
        # verb must be in mask
        if self._allow_verb_mask is not None and not self._allow_verb_mask[verb.id]:
            return False
        return True

    @property
    def is_object_wildcard(self) -> bool:
        return not self.object_node_types and not self.object_properties

    def includes_object(self, object: "RequestObject") -> bool:
        """Whether the zone includes the object."""
        # node type must match
        if (
            self._object_node_types_mask is not None
            and not self._object_node_types_mask[object.node_type.id]
        ):
            return False
        # all object properties must match
        if self._object_properties_mask_by_node_type is not None:
            properties_mask = self._object_properties_mask_by_node_type.get(object.node_type)
            if properties_mask is not None:
                if object._properties_mask is None:
                    return False
                # all bits of object's properties_mask must be set in our properties_mask
                if (properties_mask & object._properties_mask) != object._properties_mask:
                    return False
        return True  # no mismatch -> match


@struct(StructType.ACCESS_MATRIX)
class AccessMatrix(Struct):
    """The materialized access matrix to quickly and easily evaluate access for a subject."""

    # id should match the index into zones list
    subject: RequestSubject = struct_internal(30, require=True, struct=StructType.REQUEST_SUBJECT)
    zones: list[AccessZone] = struct_internal(31, array=True, struct=StructType.ACCESS_ZONE)
    _zones_by_scope_id: dict[str, tuple[int, ...]] = struct_runtime(default_factory=dict)
    _applied_policies_by_scope_id: dict[str, list[Policy]] = struct_runtime(default_factory=dict)


@struct(StructType.REQUEST)
class Request(Struct):
    """The result of evaluating a single request."""

    subject: RequestSubject = struct_internal(30, require=True, struct=StructType.REQUEST_SUBJECT)
    verb: ActionType = struct_internal(31, require=True)
    object: RequestObject = struct_internal(32, require=True, struct=StructType.REQUEST_OBJECT)
    decision: PolicyEffect = struct_internal(34, require=True)

    def __content_str__(self) -> str:
        return f"{self.decision} {self.subject} {self.verb.bench_name} {self.object}"

    def to_action(self) -> "Action":
        return Action(
            subject=self.subject,
            request_evaluations=[self],
            deciding_evaluation=self,
            decision=self.decision,
        )


@struct(StructType.ACTION)
class Action(Struct):
    """
    The result of evaluating an action (multiple requests).
    TODO @Feature @Security: store, query and watch action log
    """

    subject: RequestSubject = struct_internal(30, require=True, struct=StructType.REQUEST_SUBJECT)
    request_evaluations: list[Request] = struct_internal(
        31, array=True, require=True, struct=StructType.REQUEST
    )
    deciding_evaluation: Request | None = struct_internal(
        32, default=None, require=False, struct=StructType.REQUEST
    )
    decision: PolicyEffect = struct_internal(33, require=True)

    def __content_str__(self) -> str:
        return f"{self.decision} {self.subject} ({', '.join(str(r) for r in self.request_evaluations)})"


@struct(StructType.ACTION_TRACE)
class ActionTrace(Struct):
    """A detailed on-demand trace of the access evaluation for an action."""

    pass


def _enums_to_mask(values: list[ProtoStrEnum | PropertyReference | Property]) -> bitarray:
    mask = bitarray()
    mask.setall(False)
    for value in values:
        mask[value.id] = True
    return mask


def _ints_to_mask(values: list[int]) -> bitarray:
    mask = bitarray()
    mask.setall(False)
    for value in values:
        mask[value] = True
    return mask


class AccessError(BenchError, ValueError):
    def __init__(
        self,
        evaluation: Request | Action,
        cause: Exception | None = None,
    ):
        super().__init__(f"denied: {evaluation}", cause)
        self.evaluation = evaluation
        self.cause = cause


SYSTEM_POLICIES: tuple[Policy, ...] = (
    # order matters!
    Policy("OnlySystemCanEditSystem").append(
        PolicyRule()
        .deny(EditType.UPDATE)
        .object(properties=tuple(p for p in iter_properties(*NODE_TYPES) if p.is_system))
    ),
    Policy("OwnerCanDoAnything").append(
        PolicyRule().subject(is_owner=True).allow(*ACTION_KINDS),
    ),
    Policy("MemberCanReadBenchGlobal").append(
        # global = above package
        PolicyRule()
        .subject(is_member=True)
        .allow(ActionKind.READ)
        .object(
            node_types=tuple(nt for nt in IN_BENCH_NODE_TYPES if nt not in SUB_PACKAGE_NODE_TYPES)
        )
    ),
    Policy("AuthenticatedCanReadUsersAndOrgs").append(
        PolicyRule()
        .subject(is_authenticated=True)
        .allow(ActionKind.READ)
        .object(
            node_types=(NodeType.USER, NodeType.ORGANIZATION),
            properties=tuple(
                p
                for p in iter_properties(NodeType.USER, NodeType.ORGANIZATION)
                if not p.is_sensitive
            ),
        )
    ),
    Policy("AnyoneCanReadHandle").append(
        PolicyRule().subject().allow(ActionKind.READ).object((NodeType.HANDLE,))
    ),
)


def adapt_read_options(
    subject: RequestSubject, root_node_type: NodeType, options: ReadOptions
) -> ReadOptions:
    """
    Adapt read options based on the action to pre-filter as feasible while enabling the complete post-read check.
    Does NOT fully evaluate access yet, but avoids loading data that will be denied anyway.
    """

    options: ReadOptions = options.copy()

    # query ancestors up to root
    for ancestor_type in ANCESTOR_NODE_TYPES[root_node_type]:
        if ancestor_type not in options.ancestor_types:
            options.ancestor_types.append(ancestor_type)

    # if root >: Bench, also load related actual owner (User/Organization)
    root_node_cls = NODE_CLASS_BY_TYPE[root_node_type]
    if NodeType.BENCH in root_node_cls.__roots__:
        options.related_types_via.append(Bench.user.to_ref, Bench.organization.to_ref)

    # TODO @Performance @Security: also pre-filter for owner?

    return options


LEGISLATIVE_NODE_TYPES: tuple[NodeType, ...] = tuple(
    NodeType.BENCH, NodeType.PACKAGE, NodeType.SPACE, NodeType.BLOCK
)


def _build_access_zones(
    current_scope: AnyNodeData,
    parent_zones: tuple[int, ...],
    identities: tuple[RequestSubject, ...],
    tree: NodeDataTree,
    owner: Owner,
    matrix: AccessMatrix,
) -> None:
    from bench.proto import wiring

    if current_scope.policies:
        pass  # nocheckin: add any new policies from current node to applied_policies_by_scope_id

    # adjust & split zones using most permissive identity
    new_policies = matrix._applied_policies_by_scope_id.get(current_scope.id, ())
    if new_policies:
        new_rules = tuple(rule for policy in new_policies for rule in policy.rules)
        for identity in identities:
            for rule in new_rules:
                ...

    current_zones = ...

    # descend tree
    current_node_type = wiring.unpack_enum(NodeType, current_scope.metatype)
    for nt in DESCENDANT_NODE_TYPES.get(current_scope.metatype, ()):
        if nt in LEGISLATIVE_NODE_TYPES:
            # build more access zones if nodes could have different policies
            for child in tree.iter_descendants(current_scope, nt):
                matrix._zones_by_scope_id[child.id] = current_zones
        else:
            for child in tree.iter_descendants(current_scope, nt, recursive=True):
                matrix._zones_by_scope_id[child.id] = current_zones


def materialize_access_matrix(
    subject: RequestSubject,
    tree: NodeDataTree,
    base_policies: list[Policy] | tuple[Policy, ...] = SYSTEM_POLICIES,
    root_owner: Optional[Owner] = None,
) -> AccessMatrix:
    """
    Builds the access matrix for the given subject from scratch.
    The (block) definition for the subject's roles and identities must be in the tree.
    To evaluate ownership and membership, the tree must include the top-level root or specify an owner.
    """
    from bench.proto import wiring

    identities: tuple[RequestSubject, ...] = subject.split_into_acting_subjects()
    matrix = AccessMatrix(subject=subject, zones=[])
    roots = tree.find_roots()
    for root in roots:
        root_type = wiring.unpack_enum(NodeType, root.metatype)
        root_cls = NODE_CLASS_BY_TYPE[root_type]
        if not root_cls.__roots__:
            owner = root
        elif root_owner is None:
            raise ValueError(f"root {root} has no owner")
        else:
            owner = root_owner

        # root zone starts with no allowed verbs, object is everything (i.e. *=blank)
        root_zone = AccessZone(id=len(matrix.zones), scope=root)
        matrix.zones.append(root_zone)
        start_index = len(matrix.zones) - 1
        _build_access_zones(
            current_scope=root,
            parent_zones=(root_zone.id,),
            identities=identities,
            tree=tree,
            owner=owner,
            matrix=matrix,
        )

        # apply base policies to all zones

        for new_zone in matrix.zones[start_index:]:
            pass

    return matrix


def _find_matching_zone(
    matrix: AccessMatrix, verb: ActionType, object: RequestObject, scope_id: str
) -> Optional[AccessZone]:
    zone_ids = matrix._zones_by_scope_id.get(scope_id)
    for zone_id in zone_ids:
        zone = matrix.zones[zone_id]
        if zone.allows_verb(verb) and zone.includes_object(object):
            return zone
    return None


def _evaluate_request_in_matrix(
    matrix: AccessMatrix, verb: ActionType, object: RequestObject, scope_id: str
) -> Request:
    zone = _find_matching_zone(matrix, verb, object, scope_id)
    if zone:
        return Request(
            subject=matrix.subject, verb=verb, object=object, decision=PolicyEffect.ALLOW
        )
    # implicit deny
    return Request(subject=matrix.subject, verb=verb, object=object, decision=PolicyEffect.DENY)


def evaluate_read(
    access: AccessMatrix, tree: NodeTree | NodeDataTree
) -> tuple[Action, Collection[AnyNodeData]]:
    """
    Evaluate and *adapt* access to all nodes in the given tree, pruning nodes & properties as needed.
     -> unlike for other actions, we don't outright reject GET reads, you just get less (or zero) data.
    If no overall owner is given, the owners (i.e. actual roots) must be in the tree.
    Assumes that all policies are valid.
    """
    visible_nodes: list[AnyNodeData] = []


def evaluate_edit(
    matrix: AccessMatrix, tree: NodeTree | NodeDataTree, edits: Collection[EditData]
) -> Action:
    """
    Evaluates whether the given policies (base and in tree) allow the given edits.
    Assumes that all policies are valid.
    """
    from bench.proto import wiring

    request_evaluations: list[Request] = []
    new_node_scopes_by_child_id: dict[str, str] | None = None
    for edit in edits:
        action_type: ActionType = wiring.unpack_enum(EditType, edit.action)
        node_type: NodeType = wiring.unpack_enum(NodeType, edit.node_type)
        node_cls = NODE_CLASS_BY_TYPE[node_type]
        node = wiring.unwrap_some_node(edit.node)
        properties = tuple(node_cls.__properties_by_id__[prop_id] for prop_id in edit.properties)
        object = RequestObject(
            node_type=node_type,
            properties=properties,
            _properties_mask=_ints_to_mask(edit.properties),
        )

        # When creating multiple nodes in one transaction the tree only knows about the 'root',
        #  so we remember the actual scopes for the new nodes to know which zone to use.
        if edit.type == EditType.CREATE or edit.type == EditType.UPSERT:
            if new_node_scopes_by_child_id is None:
                new_node_scopes_by_child_id = {}
            scope_id = node.parent_ptr.id or node.id
            while scope_id in new_node_scopes_by_child_id:
                scope_id = new_node_scopes_by_child_id[scope_id]
            new_node_scopes_by_child_id[node.id] = scope_id
        else:
            scope_id = node.id

        evaluation = _evaluate_request_in_matrix(matrix, action_type, object, scope_id)
        if evaluation.decision == PolicyEffect.DENY:
            return Action(
                subject=matrix.subject,
                request_evaluations=request_evaluations,
                deciding_evaluation=evaluation,
                decision=PolicyEffect.DENY,
            )
        request_evaluations.append(evaluation)

    # This isn't implicit approve since we already implicitly denied per request above,
    #  so the constituent requests must have been explicit approved.
    return Action(
        subject=matrix.subject, request_evaluations=request_evaluations, decision=PolicyEffect.ALLOW
    )


def check_edit(
    matrix: AccessMatrix, tree: NodeTree | NodeDataTree, edits: Collection[EditData]
) -> Action:
    evaluation = evaluate_edit(matrix, tree, edits)
    if evaluation.decision == PolicyEffect.DENY:
        raise AccessError(evaluation)
    return evaluation


def evaluate_run(
    matrix: AccessMatrix, tree: NodeTree, run_type: RunType, block: "Block"
) -> Request:
    """
    Evaluates whether the given policies (base and in tree) allow the given run action.
    Assumes that all policies are valid.
    """
    object = RequestObject(node_type=block.type)
    evaluation = _evaluate_request_in_matrix(matrix, run_type, object, str(block.id))
    return evaluation


def check_run(matrix: AccessMatrix, tree: NodeTree, run_type: RunType, block: "Block") -> Request:
    evaluation = evaluate_run(matrix, tree, run_type, block)
    if evaluation.decision == PolicyEffect.DENY:
        raise AccessError(evaluation)
    return evaluation
