from collections import defaultdict
from datetime import datetime
from typing import TYPE_CHECKING, Collection, Optional, Self, Union
from uuid import UUID

from bitarray import bitarray

from bench.language.const import (
    ACTION_KINDS,
    IN_BENCH_NODE_TYPES,
    NODE_TYPES,
    SUB_PACKAGE_NODE_TYPES,
    ActionKind,
    ActionType,
    BadgeType,
    BenchError,
    BenchType,
    EditType,
    NodeType,
    PolicyEffect,
    RunType,
    StructType,
    ReadType,
)
from bench.language.node import (
    ANCESTOR_NODE_TYPES,
    NODE_CLASS_BY_TYPE,
    Node,
    Package,
    Property,
    Struct,
    iter_properties,
    node,
    p_internal,
    p_parent,
    p_runtime,
    p_tracked,
    struct,
    CHILD_NODE_TYPES,
)
from bench.language.notice import NoticeHandler
from bench.language.text import RichText
from bench.language.tree import NodeDataTree, NodeTree
from bench.language.user import Membership, Organization, User
from bench.proto.wire import AnyNodeData, EditData
from bench.utils.casing import IdentifierType
from bench.utils.func import to_uuid, IdStrEnum, bytetuple

if TYPE_CHECKING:
    from bench.language import Bench, Block, Client, Expression, Property, ScopeNode

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

    parent: Union[Package, "Block"] = p_parent(4, NodeType.PACKAGE, NodeType.BLOCK)
    type: BadgeType = p_tracked(30)
    name: Optional[str] = p_tracked(31)
    delegated_policies: list["Policy"] = p_tracked(32, array=True, struct=StructType.POLICY)
    expires_at: Optional[datetime] = p_tracked(33, default=None)
    # sharing link badge
    link_token: Optional[UUID] = p_internal(40, unique=True, default=None)
    link_password: Optional[str] = p_internal(
        41, default=None, encrypt=True, defer=True, sensitive=True
    )
    link_password_hash: Optional[str] = p_internal(
        42, default=None, encrypt=True, defer=True, sensitive=True
    )
    # access key badge
    key_value: Optional[str] = p_internal(
        50, default=None, encrypt=True, defer=True, sensitive=True
    )
    key_value_hash: Optional[str] = p_internal(
        51, default=None, encrypt=True, defer=True, sensitive=True
    )


@node(NodeType.ROLE)
class Role(Node):
    """
    Attach a role to a block or member.
    Role policies are delegated to the parent and its descendants.
    The delegated policies apply to all descendant's actions.
    """

    parent: Union["Block", "Membership"] = p_parent(4, NodeType.BLOCK, NodeType.MEMBERSHIP)
    type: "Block" = p_tracked(30, array=False, require=True, references=NodeType.BLOCK)


@node(NodeType.IDENTITY)
class Identity(Node):
    """
    Attach an identity to a block or member.
    Identity policies are delegated to the parent and its descendants.
    The delegated policies apply to all descendant's actions.
    """

    parent: Union["Block", "Membership"] = p_parent(4, NodeType.BLOCK, NodeType.MEMBERSHIP)
    type: "Block" = p_tracked(30, array=False, require=True, references=NodeType.BLOCK)


@struct(StructType.READ_OPTIONS)
class ReadOptions(Struct):
    """
    Fine-grained options to a read request.
    This is an addition to primary options (like the filter for a search or aggregation).
    """

    # relations
    ancestor_types: list[NodeType] | None = p_tracked(30, default=None)
    descendant_types: list[NodeType] | None = p_tracked(31, default=None)
    related_properties: list[Property] | None = p_tracked(
        32, require=False, default=None, array=True, struct=StructType.PROPERTY_REFERENCE
    )

    # properties (include/exclude relative to default)
    include_properties: list[Property] | None = p_tracked(
        40, require=False, default=None, array=True, struct=StructType.PROPERTY_REFERENCE
    )
    exclude_properties: list[Property] | None = p_tracked(
        41, require=False, default=None, array=True, struct=StructType.PROPERTY_REFERENCE
    )

    # filters (global + per type)
    global_filter: Optional["Expression"] = p_tracked(
        50, require=False, default=None, struct=StructType.EXPRESSION
    )
    # (by type is runtime only since we can't / don't need to serialize maps yet)
    _filter_by_type: dict[NodeType, "Expression"] | None = p_runtime(default=None)

    def copy(self) -> "ReadOptions":
        _copy_list_maybe = lambda x: list(x) if x is not None else None

        return ReadOptions(
            ancestor_types=_copy_list_maybe(self.ancestor_types),
            descendant_types=_copy_list_maybe(self.descendant_types),
            related_properties=_copy_list_maybe(self.related_properties),
            include_properties=_copy_list_maybe(self.include_properties),
            exclude_properties=_copy_list_maybe(self.exclude_properties),
            global_filter=self.global_filter,
            _filter_by_type=self._filter_by_type,
        )

    def related(self, node_type: NodeType) -> list[Property] | tuple[Property, ...]:
        return tuple(p.resolved_property for p in self.related_properties if p.type == node_type)

    def select(self, node_type: NodeType) -> list[Property] | tuple[Property, ...]:
        from bench.sql.engine import DEFAULT_SELECTED_PROPERTIES

        # a bit ugly but the tuples are very small
        properties = DEFAULT_SELECTED_PROPERTIES.get(node_type, ())
        if self.include_properties is not None and any(
            p.type == node_type for p in self.include_properties
        ):
            properties = properties + tuple(
                p for p in self.include_properties if p.type == node_type
            )
        if self.exclude_properties is not None and any(
            p.type == node_type for p in self.exclude_properties
        ):
            properties = tuple(
                p for p in properties if not any(e.id == p.id for e in self.exclude_properties)
            )
        return properties

    def filter(self, node_type: NodeType, filter: Optional["Expression"] = None) -> "Expression":
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
        return ReadOptions(ancestor_types=None, descendant_types=None, global_filter=None)


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

    name: Optional[str] = p_tracked(30, default=None)
    text: Optional["RichText"] = p_tracked(31, default=None, struct=StructType.RICH_TEXT)
    rules: list["PolicyRule"] = p_tracked(32, default_factory=list, struct=StructType.POLICY_RULE)
    hidden: bool = p_tracked(33, default=False, description="Hide this policy and its effects.")
    scopes: list["Block"] | None = p_tracked(
        34, default=None, require=False, array=True, references=NodeType.BLOCK
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

    name: Optional[str] = p_tracked(30, default=None)
    text: Optional["RichText"] = p_tracked(31, default=None, struct=StructType.RICH_TEXT)

    # subject (if unset it's a wildcard)
    # if subject is delegated then it always matches if this rule is present
    #  (and no other subject filters make sense)
    subject_is_delegated: bool = p_tracked(40, default=False)
    subject_is_authenticated: bool = p_tracked(41, default=False)
    subject_is_staff: bool = p_tracked(42, default=False)
    # member/owner relative to the object
    subject_is_member: bool = p_tracked(43, default=False)
    subject_is_owner: bool = p_tracked(44, default=False)
    # subject_users, subject_groups, subject_identities, subject_roles, ...

    # verb
    effect: PolicyEffect = p_tracked(60, default=PolicyEffect.DENY)
    verbs: list[ActionType] | None = p_tracked(61, default=None)
    verb_kinds: list[ActionKind] | None = p_tracked(62, default=None)
    _verb_mask: bitarray | None = p_runtime(default=None)

    # object (if unset it's a wildcard)
    object_node_types: Optional[list[NodeType]] = p_tracked(80, default=None)
    _object_node_types_mask: bitarray | None = p_runtime(default=None)
    object_properties: list[Property] | None = p_tracked(
        81, require=False, default=None, array=True, struct=StructType.PROPERTY_REFERENCE
    )
    _object_properties_masks: dict[NodeType, bitarray] | None = p_runtime(default=None)

    # object_properties, object_nodes, object_fields, ...

    # [condition]
    # condition: Expression ...

    def __content_str__(self) -> str:
        subject_str_parts = []
        for subject_key in (
            "subject_is_delegated",
            "subject_is_authenticated",
            "subject_is_staff",
            "subject_is_member",
            "subject_is_owner",
        ):
            value = getattr(self, subject_key)
            if value:
                subject_str_parts.append(f"{subject_key[8:]}={value}")
        if subject_str_parts:
            subject_str = f"[{', '.join(subject_str_parts)}]"
        else:
            subject_str = "*"

        verb_str_parts = []
        if self.verbs:
            verb_str_parts.extend(verb.bench_name for verb in self.verbs)
        if self.verb_kinds:
            verb_str_parts.extend(verb_kind.bench_name for verb_kind in self.verb_kinds)
        if verb_str_parts:
            verb_str = f"{', '.join(verb_str_parts)}"
        else:
            verb_str = "*"

        if self.object_node_types:
            object_type_str_parts = tuple(t.bench_name for t in self.object_node_types)
        else:
            object_type_str_parts = ("*",)
        if self.object_properties:
            object_prop_str_parts = tuple(
                f"{p.type.bench_name}.{p.name}" for p in self.object_properties
            )
        else:
            object_prop_str_parts = ("*",)
        object_type_str = ", ".join(object_type_str_parts) if object_type_str_parts else "*"
        object_prop_str = ", ".join(object_prop_str_parts) if object_prop_str_parts else "*"
        object_str = f"[{object_type_str}].[{object_prop_str}]"

        return f"{self.effect} {subject_str} {verb_str} {object_str}"

    def _clear_inner(self, scope: Optional["ScopeNode"] = None):
        self._verb_mask = None
        self._object_node_types_mask = None
        if self._object_properties_masks is not None:
            self._object_properties_masks.clear()

    def _interp_inner(self, scope: "ScopeNode", on_notice: "NoticeHandler"):
        self._update_verb_mask()
        self._update_object_mask()

    def _update_object_mask(self):
        self._object_node_types_mask = _enums_to_mask(self.object_node_types, NodeType)

        self._object_properties_masks = {}
        if self.object_properties:
            for prop in self.object_properties:
                if prop.type not in self._object_properties_masks:
                    self._object_properties_masks[prop.type] = bitarray(
                        prop.component.__max_property_id__ + 1
                    )
                self._object_properties_masks[prop.type][prop.id] = True

    def _update_verb_mask(self):
        self._verb_mask = bitarray(ActionType.get_max_id())
        if not self.verbs and not self.verb_kinds:
            self._verb_mask[ActionType.get_min_id() : ActionType.get_max_id()] = True
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

    def matches_verb(self, verb: ActionType) -> bool:
        return self._verb_mask[verb.id]

    def matches_object(self, object: "RequestObject") -> bool:
        if not self._object_node_types_mask[object.node_type.id]:
            return False
        if self._object_properties_masks is not None:
            properties_mask = self._object_properties_masks.get(object.node_type)
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
    """The <whoever/whatever> issuing a request. Unknown attributes are uninitialized."""

    is_authenticated: bool = p_internal(30)
    is_staff: bool = p_internal(31, default=False)
    ownerships: list[Owner] = p_internal(
        32,
        require=True,
        array=True,
        references=(NodeType.USER, NodeType.ORGANIZATION, NodeType.BENCH),
    )
    # memberships/roles/...?
    client: Optional["Client"] = p_internal(
        35, default=None, require=False, array=False, references=NodeType.CLIENT
    )
    user: Optional["User"] = p_internal(
        36, default=None, require=False, array=False, references=NodeType.USER
    )
    identity: Optional["Identity"] = p_internal(
        37, default=None, require=False, array=False, references=NodeType.IDENTITY
    )
    badge: Optional["Badge"] = p_internal(
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


@struct(StructType.ACCESS_ZONE)
class AccessZone(Struct):
    """
    The pre-filtered access rules for a given identity.
    Clients use this to indicate access rights, but - obviously - only our copy is binding.
    """

    id: int = p_internal(2, require=True)
    parent_id: int | None = p_internal(4)
    scope_id: str = p_internal(30)
    identity_id: int = p_internal(31)
    rules: list[PolicyRule] = p_internal(32, array=True, struct=StructType.POLICY_RULE)


@struct(StructType.ACCESS_MATRIX)
class AccessMatrix(Struct):
    """The materialized access matrix generated for a specific subject to quickly evaluate access for objects."""

    subject: RequestSubject = p_internal(30, require=True, struct=StructType.REQUEST_SUBJECT)
    identities: list[RequestSubject] = p_internal(32, array=True, struct=StructType.REQUEST_SUBJECT)
    zones: list[AccessZone] = p_internal(33, array=True, struct=StructType.ACCESS_ZONE)
    base_zones: list[PolicyRule] = p_internal(34, array=True, struct=StructType.POLICY_RULE)

    # quick access to the zone (id = index)
    _lowest_zone_by_scope: dict[tuple[int, str], AccessZone] = p_runtime(default_factory=dict)
    _base_zone_by_root: dict[str, AccessZone] = p_runtime(default_factory=dict)


@struct(StructType.REQUEST)
class Request(Struct):
    """The result of evaluating a single request."""

    verb: ActionType = p_internal(31, require=True)
    object_type: BenchType = p_internal(32, require=True)
    object_properties: list[Property] | None = p_internal(
        33, require=False, array=True, default=None, struct=StructType.PROPERTY_REFERENCE
    )
    decision: PolicyEffect = p_internal(34, require=True)

    def __content_str__(self) -> str:
        object_str_parts = []
        for object_key in ("properties", "owner"):
            value = getattr(self, object_key)
            if value:
                if isinstance(value, bool):
                    object_str_parts.append(object_key)
                else:
                    object_str_parts.append(f"{object_key}={value}")
        if object_str_parts:
            object_str = f"{self.object_type.bench_name} [{', '.join(object_str_parts)}]"
        else:
            object_str = self.object_type.bench_name
        return f"{self.decision} {self.verb.bench_name} {object_str}"


@struct(StructType.ACTION)
class Action(Struct):
    """
    The result of evaluating an action (multiple requests).
    TODO @Feature @Security: store, query and watch action log
    """

    subject: RequestSubject = p_internal(30, require=True, struct=StructType.REQUEST_SUBJECT)
    requests: list[Request] = p_internal(31, array=True, require=True, struct=StructType.REQUEST)
    decision: PolicyEffect = p_internal(32, require=True)

    # context
    bench: Optional["Bench"] = p_internal(
        40, default=None, require=False, array=False, references=NodeType.BENCH
    )
    package: Optional[Package] = p_internal(
        41, default=None, require=False, array=False, references=NodeType.PACKAGE
    )
    user: Optional["User"] = p_internal(
        42, default=None, require=False, array=False, references=NodeType.USER
    )

    def __content_str__(self) -> str:
        return f"{self.decision} {self.subject} ({', '.join(str(r) for r in self.requests)})"


def _enums_to_mask(values: list[IdStrEnum], cls: type[IdStrEnum]) -> bitarray:
    """Set the given values in a mask. No values == all values == wildcard!"""
    mask = bitarray(cls.get_max_id() + 1)
    if not values:
        mask.setall(True)
    else:
        for value in values:
            mask[value.id] = True
        return mask


def _properties_to_mask(values: list[Property], node_type: NodeType) -> bitarray:
    """Set the given properties in a mask. No values == all values == wildcard!"""
    mask = bitarray(NODE_CLASS_BY_TYPE[node_type].__max_property_id__)
    if not values:
        mask.setall(True)
    else:
        for value in values:
            mask[value.id] = True
        return mask


def _ints_to_mask(values: list[int], length: int) -> bitarray:
    mask = bitarray(length)
    if not values:
        mask.setall(True)
    else:
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
    Policy("SystemProtection").append(
        PolicyRule("CannotEditSystemProperties")
        .deny(EditType.UPDATE)
        .object(
            properties=tuple(
                p for p in iter_properties(*NODE_TYPES) if p.is_system and p.id is not None
            )
        )
    ),
    Policy("OwnerAccess").append(
        PolicyRule("OwnerCanDoAnything").subject(is_owner=True).allow(*ACTION_KINDS),
    ),
    Policy("MemberAccess").append(
        # meta = nodes belonging to the bench, outside  package
        PolicyRule("MemberCanReadBenchMeta")
        .subject(is_member=True)
        .allow(ActionKind.READ)
        .object(
            node_types=tuple(nt for nt in IN_BENCH_NODE_TYPES if nt not in SUB_PACKAGE_NODE_TYPES)
        )
    ),
    Policy("AuthenticatedAccess").append(
        PolicyRule("AuthenticatedCanReadUserInfo")
        .subject(is_authenticated=True)
        .allow(ActionKind.READ)
        .object(
            node_types=(NodeType.USER,),
            properties=tuple(
                p for p in User.__properties__.values() if not p.is_sensitive and p.id
            ),
        ),
        PolicyRule("AuthenticatedCanReadOrganizationInfo")
        .subject(is_authenticated=True)
        .allow(ActionKind.READ)
        .object(
            node_types=(NodeType.ORGANIZATION,),
            properties=tuple(
                p for p in Organization.__properties__.values() if not p.is_sensitive and p.id
            ),
        ),
    ),
    Policy("AnonymousAccess").append(
        PolicyRule("AnonCanReadHandle").subject().allow(ActionKind.READ).object((NodeType.HANDLE,))
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
        options.related_properties.append(Bench.user, Bench.organization)

    # TODO @Performance @Security: also pre-filter read options for owner?

    return options


# the node types that can have 'policies' applied to them
#  (not delegated node types, which delegate via subject)
LEGISLATIVE_NODE_TYPES: bytetuple[NodeType] = bytetuple(
    NodeType.BENCH, NodeType.PACKAGE, NodeType.SPACE, NodeType.BLOCK
)


def generate_access_matrix(
    subject: RequestSubject,
    tree: NodeDataTree,
    base_policies: tuple[Policy, ...] = SYSTEM_POLICIES,
    root_owner: Owner | None = None,
    unpacked_tree: NodeTree | None = None,
) -> AccessMatrix:
    """Generates an access matrix for the given subject."""

    from bench.proto import wiring

    identities = subject.split_into_acting_subjects()
    zones: list[AccessZone] = []
    base_zones: list[AccessZone] = []
    matrix = AccessMatrix(
        subject=subject, zones=zones, base_zones=base_zones, identities=identities
    )
    roots = tree.find_roots()
    applied_policies_by_node_id: dict[str, list[Policy]] = defaultdict(list)

    def _assign_access_zones(
        current_node: AnyNodeData, owner: Owner, parent_zones_by_identity: tuple[int | None, ...]
    ):
        """Generates any new applicable access zones downstream from the node."""

        # if this node defines new policies, apply them to their scope
        if getattr(current_node, "policies", None):
            if unpacked_tree:
                new_policies: list[Policy] = unpacked_tree.get(to_uuid(current_node.id)).policies
            else:
                new_policies: list[Policy] = [
                    wiring.unpack_struct_interp(p) for p in current_node.policies
                ]
            for policy in new_policies:
                if policy.scopes:
                    for scope in policy.scopes:
                        # we don't check if scope <= current here, but we don't need to
                        #  (it's validated in Policy and ancestor policies were already considered)
                        applied_policies_by_node_id[str(scope.id)].append(policy)
                else:
                    applied_policies_by_node_id[current_node.id].append(policy)

        # gather all the policy rules that apply in this context (per identity)
        applied_policies = applied_policies_by_node_id.get(current_node.id, ())
        if applied_policies:
            for identity_id, identity in enumerate(identities):
                applicable_rules = tuple(
                    rule
                    for policy in applied_policies
                    for rule in policy.rules
                    if rule.matches_subject(identity, owner)
                )
                if applicable_rules:
                    # we got a new zone with different roles down here
                    zone = AccessZone(
                        id=len(zones),
                        parent_id=parent_zones_by_identity[identity_id],
                        scope_id=current_node.id,
                        identity_id=identity_id,
                        rules=applicable_rules,
                    )
                    zones.append(zone)
                    # update parent zones for the next level
                    parent_zones_by_identity = tuple(parent_zones_by_identity)
                    parent_zones_by_identity[identity_id] = zone.id

        # update 'lowest zone' shortcuts
        for identity_id in range(len(identities)):
            zone = zones[parent_zones_by_identity[identity_id]]
            matrix._lowest_zone_by_scope[(identity_id, current_node.id)] = zone

        # descend into children
        current_type: NodeType = wiring.unpack_enum(NodeType, current_node.metatype)
        for child_type in CHILD_NODE_TYPES[current_type]:
            for child_node in tree.iter_descendants(current_node, child_type):
                _assign_access_zones(child_node, owner, parent_zones_by_identity)

    # start at root
    root_zones_by_identity = tuple(None for _ in identities)
    for root in roots:
        # figure out owner
        root_type = wiring.unpack_enum(NodeType, root.metatype)
        root_cls = NODE_CLASS_BY_TYPE[root_type]
        if root_cls.__roots__:  # not an actual root
            if root_owner is None:
                raise ValueError(f"no owner for root {root!r}")
            owner = root_owner
        else:
            owner = root

        # base zones are checked before all others (typically for system policies)
        #  but are specific to each root (=owner)
        for identity_id, identity in enumerate(identities):
            base_rules = tuple(
                rule
                for policy in base_policies
                for rule in policy.rules
                if rule.matches_subject(identity, owner)
            )
            base_zone = AccessZone(
                scope_id=root.id,
                parent_id=None,
                identity_id=identity_id,
                rules=base_rules,
            )
            base_zones.append(base_zone)
            matrix._base_zone_by_root[root.id] = base_zone

        # add nested zones if there are any legislative nodes down here
        _assign_access_zones(root, owner, root_zones_by_identity)

    return matrix


_ALL_PROPERTIES_BY_TYPE: dict[NodeType, bitarray] = ...


def evaluate_request(
    *,
    matrix: AccessMatrix,
    verb: ActionType,
    object_node_type: NodeType,
    object_properties: bitarray,
    root_id: str,
    scope_id: str,
) -> bitarray:
    """
    Evaluates the action against the granted access, masking rejected properties if possible.
    Technically, the requests are for (verb, node_type, property), so we use the first match for each.
    """

    # the granted 'allow' mask for properties
    composite_allowed_properties = bitarray(len(object_properties))

    # check the zones for each identity (separately)
    for identity_id in range(len(matrix.identities)):
        allowed_properties = bitarray(len(object_properties))  # for this identity
        unset_properties = object_properties  # the unmatched properties so far

        # first check the base zones, then walk the zones starting from the lowest
        base_zone: AccessZone = matrix._base_zone_by_root[root_id]
        start_zone: AccessZone = matrix._lowest_zone_by_scope[(identity_id, scope_id)]
        current_zone = base_zone
        while unset_properties:
            for rule in current_zone.rules:
                if rule.matches_verb(verb) and object_node_type in rule._object_node_types_mask:
                    rule_properties_mask = rule._object_properties_masks.get(
                        object_node_type, _ALL_PROPERTIES_BY_TYPE[object_node_type]
                    )
                    if rule.effect == PolicyEffect.ALLOW:
                        allowed_properties |= rule_properties_mask & unset_properties
                    else:
                        allowed_properties &= ~(rule_properties_mask & unset_properties)
                    unset_properties = unset_properties & ~rule_properties_mask

                    if not unset_properties:
                        break  # nothing can change anymore

            if current_zone.id == base_zone.id:
                current_zone = start_zone
            elif start_zone.id is not None:
                current_zone = matrix.zones[current_zone.parent_id]
            else:
                break  # reached the top

        composite_allowed_properties |= allowed_properties
        if composite_allowed_properties == object_properties:
            break  # all allowed

    return composite_allowed_properties


def evaluate_atomic_request(
    *,
    matrix: AccessMatrix,
    verb: ActionType,
    object_node_type: NodeType,
    object_properties: bitarray,
    root_id: str,
    scope_id: str,
) -> PolicyEffect:
    """
    Evaluates a single 'atomic' request (atomic because if any property is rejected, all are).
    """

    allowed_properties = evaluate_request(
        matrix=matrix,
        verb=verb,
        object_node_type=object_node_type,
        object_properties=object_properties,
        root_id=root_id,
        scope_id=scope_id,
    )
    if allowed_properties == object_properties:
        return PolicyEffect.ALLOW
    else:
        return PolicyEffect.DENY


def evaluate_and_adapt_read(
    matrix: AccessMatrix, tree: NodeDataTree, *, adapt_nodes_in_place: bool = False
) -> tuple[Action, Collection[AnyNodeData]]:
    """
    Evaluate *and* adapt access to all nodes in the given tree, pruning nodes & properties as needed.
     -> unlike for other actions, we don't outright reject GET reads, you just get less (or zero) data.
    In case a node was completely denied but its children weren't, we include a Skip node in the result.
    If no overall owner is given, the owners (i.e. actual roots) must be in the tree.
    Assumes that all policies are valid.
    """
    from bench.proto import wire, wiring

    visible_nodes: list[AnyNodeData] = []
    skipped_nodes_ids: set[str] = set()

    # adapt & filter nodes
    verb = ReadType.GET  # same for all?
    for node in tree.nodes:
        object_node_type: NodeType = wiring.unpack_enum(NodeType, node.metatype)
        object_properties: bitarray = _ALL_PROPERTIES_BY_TYPE[object_node_type]

        root = tree.get_root(node)  # a bit inefficient?
        # nocheckin: cache this per zone!
        adapted_properties, deciding_rule = evaluate_request(
            matrix=matrix,
            verb=verb,
            object_node_type=object_node_type,
            object_properties=object_properties,
            root_id=root.id,
            node_id=node.id,
        )
        if not adapted_properties:
            skipped_nodes_ids.add(node.id)
            continue
        elif adapted_properties == object_properties:
            visible_nodes.append(node)
        else:
            node_cls = NODE_CLASS_BY_TYPE[object_node_type]
            # prune the properties
            if not adapt_nodes_in_place:
                # TODO @Performance: avoid copying properties that we'll prune anyway
                node = wiring.copy_struct_data(node)
            for pruned_prop_id in object_properties ^ adapted_properties:
                prop_name = node_cls.__properties_name_by_id__[pruned_prop_id]
                setattr(node, prop_name, None)

    # add any required skipped nodes back in (as Skips)
    for node in visible_nodes:
        if node.parent_ptr is not None and node.parent_ptr.id in skipped_nodes_ids:
            skip = wire.SkipData(
                metatype=wire.NodeType.SKIP,
                id=node.id,
                ck=getattr(node, "ck", None),
                parent_ptr=node.parent_ptr,
                revision=node.revision,
                order_key=getattr(node, "order_key", None),
                type=node.metatype,
            )
            skipped_nodes_ids.remove(node.parent_ptr.id)  # add only once
            visible_nodes.append(skip)


def evaluate_edit(matrix: AccessMatrix, tree: NodeDataTree, edits: Collection[EditData]) -> Action:
    """
    Evaluates whether the given policies (base and in tree) allow the given edits.
    Assumes that all policies are valid.
    """
    from bench.proto import wiring

    requests: list[Request] = []
    new_node_scopes_by_child_id: dict[str, str] | None = None
    for edit in edits:
        action_type: ActionType = wiring.unpack_enum(EditType, edit.action)
        node_type: NodeType = wiring.unpack_enum(NodeType, edit.node_type)
        node_cls = NODE_CLASS_BY_TYPE[node_type]
        node = wiring.unwrap_some_node(edit.node)
        properties = tuple(node_cls.__properties_by_id__[prop_id] for prop_id in edit.properties)

        # when creating nested nodes in one transaction, the tree only knows about the 'root',
        #  so we remember the actual scopes for the new nodes to know which zone to use
        if edit.type == EditType.CREATE or edit.type == EditType.UPSERT:
            if new_node_scopes_by_child_id is None:
                new_node_scopes_by_child_id = {}
            scope_id = node.parent_ptr.id or node.id
            while scope_id in new_node_scopes_by_child_id:
                scope_id = new_node_scopes_by_child_id[scope_id]
            new_node_scopes_by_child_id[node.id] = scope_id
        else:
            scope_id = node.id
        root = tree.get_root(tree.get(scope_id))

        object_properties = _ints_to_mask(edit.properties, node_cls.__max_property_id__)
        decision, deciding_rule = evaluate_atomic_request(
            matrix=matrix,
            verb=action_type,
            object_node_type=node_type,
            object_properties=object_properties,
            scope_id=scope_id,
            root_id=root.id,
        )
        request = Request(
            verb=action_type, object=object, decision=decision, deciding_rule=deciding_rule
        )
        if decision == PolicyEffect.DENY:
            return Action(
                subject=matrix.subject,
                requests=requests,
                deciding_request=request,
                decision=PolicyEffect.DENY,
            )
        requests.append(request)

    # This isn't implicit approve since we already implicitly denied per request above,
    #  so the constituent requests must have been explicit approved.
    return Action(subject=matrix.subject, request_evaluations=requests, decision=PolicyEffect.ALLOW)


def check_edit(matrix: AccessMatrix, tree: NodeDataTree, edits: Collection[EditData]) -> Action:
    evaluation = evaluate_edit(matrix, tree, edits)
    if evaluation.decision == PolicyEffect.DENY:
        raise AccessError(evaluation)
    return evaluation


def evaluate_run(
    matrix: AccessMatrix, tree: NodeDataTree, run_type: RunType, block: "Block"
) -> Action:
    """
    Evaluates whether the given policies (base and in tree) allow the given run action.
    Assumes that all policies are valid.
    """
    decision, deciding_rule = evaluate_atomic_request(
        matrix=matrix,
        verb=run_type,
        object_node_type=block.metatype,
        object_properties=_ALL_PROPERTIES_BY_TYPE[block.metatype],
        scope_id=str(block.id),
        root_id=str(block.bench.id),
    )
    return Action(
        subject=matrix.subject,
        requests=[
            Request(verb=run_type, object=object, decision=decision, deciding_rule=deciding_rule)
        ],
        decision=decision,
    )


def check_run(
    matrix: AccessMatrix, tree: NodeDataTree, run_type: RunType, block: "Block"
) -> Action:
    action = evaluate_run(matrix, tree, run_type, block)
    if action.decision == PolicyEffect.DENY:
        raise AccessError(action)
    return action
