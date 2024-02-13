from collections import defaultdict
from datetime import datetime
from typing import TYPE_CHECKING, Collection, Optional, Self, Union, NamedTuple, Any
from uuid import UUID

from bitarray import bitarray

from bench.language.const import (
    ACCESS_CLASSES,
    IN_BENCH_NODE_TYPES,
    NODE_TYPES,
    PUBLIC_NODE_TYPES,
    ROOT_NODE_TYPES,
    SUB_PACKAGE_NODE_TYPES,
    UNSET,
    AccessKind,
    AccessMode,
    AccessType,
    BadgeType,
    BenchError,
    BenchType,
    EditType,
    NodeType,
    PolicyEffect,
    ReadType,
    RunType,
    StructType,
)
from bench.language.graph import NodeDataGraph, NodeGraph, NodeList
from bench.language.node import (
    _COMPLETED_SETUP,
    ANCESTOR_NODE_TYPES,
    CHILD_NODE_TYPES,
    NODE_CLASS_BY_TYPE,
    Node,
    Property,
    Struct,
    _on_completing_setup,
    iter_properties,
    node,
    on_warning_raise,
    p_child,
    p_internal,
    p_parent,
    p_regular,
    p_runtime,
    p_system,
    struct,
)
from bench.language.notice import NoticeHandler
from bench.language.text import RichText
from bench.language.user import Membership, User
from bench.language.validation import ValidationError
from bench.proto.wire import AnyNodeData, EditData, NodeReferenceData
from bench.utils.casing import IdentifierType
from bench.utils.func import IdEnum, bytetuple, to_uuid

if TYPE_CHECKING:
    from bench.language import (
        Bench,
        Block,
        Client,
        Expression,
        NodeReference,
        Organization,
        Package,
    )

# the node types that can have 'policies' applied to them
#  (not delegated node types, which delegate via subject)
LEGISLATIVE_NODE_TYPES: bytetuple[NodeType] = bytetuple(
    NodeType.BENCH,
    NodeType.ENVIRONMENT,
    NodeType.BRANCH,
    NodeType.PACKAGE,
    NodeType.SPACE,
    NodeType.BLOCK,
)


@_on_completing_setup
def _check_legislative_types():
    actual_legislative_node_types: bytetuple[NodeType] = bytetuple(
        tuple(nt for nt in NODE_TYPES if "policies" in NODE_CLASS_BY_TYPE[nt].__properties__)
    )
    assert actual_legislative_node_types.bits == LEGISLATIVE_NODE_TYPES.bits


# 'owner' refers to both the root node and any user with root-level access to that root node.
# This is effectively the 'root user' who can do anything with descendants of the root node*.
#  (unless a system policy says otherwise)
Owner = Union["User", "Organization", "Bench"]


@node(NodeType.BADGE)
class Badge(Node):
    """
    Attach a badge to a node with an inline definition.
    A badge's policies are delegated to the 'holder' (any client presenting its secrets).
    The delegated policies apply at the parent scope OR given scopes (which must be below parent's).
    """

    parent: Union["Package", "Block"] = p_parent(4, NodeType.PACKAGE, NodeType.BLOCK)
    type: BadgeType = p_regular(30)
    name: Optional[str] = p_regular(31)
    delegated_policies: list["Policy"] = p_regular(32, array=True, struct=StructType.POLICY)
    expires_at: Optional[datetime] = p_regular(33, default=None)
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
    The delegated policies apply to all descendant's accesses.
    """

    parent: Union["Block", "Membership"] = p_parent(4, NodeType.BLOCK, NodeType.MEMBERSHIP)
    type: "Block" = p_regular(30, array=False, require=True, references=NodeType.BLOCK)


@node(NodeType.IDENTITY)
class Identity(Node):
    """
    Attach an identity to a block, member or user (only the user itself can do that).
    Identity policies are delegated to the parent and its descendants.
    The delegated policies apply to all descendant's accesses.
    """

    parent: Union["Block", "Membership", "User"] = p_parent(
        4, NodeType.BLOCK, NodeType.MEMBERSHIP, NodeType.USER
    )
    type: "Block" = p_regular(30, array=False, require=True, references=NodeType.BLOCK)

    roles: NodeList["Role"] = p_child(NodeType.ROLE)


@struct(StructType.READ_OPTIONS)
class ReadOptions(Struct):
    """
    Fine-grained options to a read request.
    This is an addition to primary options (like the filter for a search or aggregation).
    """

    # relations
    ancestor_types: list[NodeType] = p_regular(30, default_factory=list)
    descendant_types: list[NodeType] = p_regular(31, default_factory=list)
    related_properties: list[Property] = p_regular(
        32, require=False, default_factory=list, array=True, struct=StructType.PROPERTY_REFERENCE
    )

    # properties (include/exclude relative to default OR select specific properties)
    include_properties: list[Property] = p_regular(
        40, require=False, default_factory=list, array=True, struct=StructType.PROPERTY_REFERENCE
    )
    exclude_properties: list[Property] = p_regular(
        41, require=False, default_factory=list, array=True, struct=StructType.PROPERTY_REFERENCE
    )
    select_properties: list[Property] = p_regular(
        42, require=False, default_factory=list, array=True, struct=StructType.PROPERTY_REFERENCE
    )

    # filters (global + per type)
    global_filter: Optional["Expression"] = p_regular(
        50, require=False, default=None, struct=StructType.EXPRESSION
    )
    # (by type is runtime only since we can't / don't need to serialize maps yet)
    _filter_by_type: dict[NodeType, "Expression"] | None = p_runtime(default=None)

    def __content_str__(self) -> str:
        content_parts = []
        for key, prop in self.__declared_properties__.items():
            value = getattr(self, key)
            if value:
                content_parts.append(f"{prop.name}={value}")
        if content_parts:
            return ", ".join(content_parts)
        else:
            return "<default>"

    def copy(self) -> "ReadOptions":
        return ReadOptions(
            ancestor_types=list(self.ancestor_types),
            descendant_types=list(self.descendant_types),
            related_properties=list(self.related_properties),
            include_properties=list(self.include_properties),
            exclude_properties=list(self.exclude_properties),
            global_filter=self.global_filter,
            _filter_by_type=self._filter_by_type,
        )

    def related(self, node_type: NodeType) -> list[Property] | tuple[Property, ...]:
        return tuple(p for p in self.related_properties if p.type == node_type)

    def select(self, node_type: NodeType) -> list[Property] | tuple[Property, ...]:
        from bench.sql.engine import DEFAULT_SELECTED_PROPERTIES

        if self.select_properties:
            # select specific properties
            return tuple(p for p in self.select_properties if p.type == node_type)
        else:
            # select default properties +/- include/exclude
            properties = DEFAULT_SELECTED_PROPERTIES.get(node_type, ())
            if self.include_properties:
                properties = properties + tuple(
                    p for p in self.include_properties if p.type == node_type
                )
            if self.exclude_properties:
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
        return ReadOptions()


@struct(StructType.POLICY, identifier=IdentifierType.VARIABLE)
class Policy(Struct):
    """
    A policy regulating access to nodes within its scope.
    The scope is determined by where its attached, but may be further restricted using 'scopes'.

    The basics of access control:
     1. An access is DENYed implicitly unless explicitly and completely ALLOWed.
     2. Policies are attached directly to nodes or via delegates (badges, roles, identities, ...).
       a. Policies are scoped to the node their definition or
       b. Delegate is attached to (or less as specified).
     3. Policies are evaluated in order up the node graph, first match decides*.
        (This means you can read a sub block but not its parent.)
     4. Every identity/role/... applicable to a subject is evaluated separately and *any* allow wins.

     * Conceptually, we do 'ray trace' up the graph for every node, but actually doing that for every request
        is prohibitively expensive. Instead, we 'rasterize' an 'access matrix' and use 'zones' as a shortcut.

    """

    name: Optional[str] = p_regular(30, default=None)
    text: Optional["RichText"] = p_regular(31, default=None, struct=StructType.RICH_TEXT)
    rules: list["PolicyRule"] = p_regular(32, default_factory=list, struct=StructType.POLICY_RULE)
    scopes: list["Block"] | None = p_regular(
        33, default=None, require=False, array=True, references=NodeType.BLOCK
    )

    def __content_str__(self) -> str:
        if self.scopes:
            scopes_str = ", ".join(repr(s) for s in self.scopes)
        else:
            scopes_str = "<scope>"
        return f"{self.name or '<unnamed>'} (at {scopes_str}, {len(self.rules)} rules)"

    def append(self, *rules: "PolicyRule") -> "Self":
        self.rules.extend(rules)
        return self


@struct(StructType.POLICY_RULE)
class PolicyRule(Struct):
    """
    A rule in a policy: [subject] + can/cannot [verb] + [object] [if condition].
    Subject, verb and object are ORed, in-group conditions are ANDed.
     (where None/empty -> wildcard, any value -> filter)
    """

    name: Optional[str] = p_regular(30, default=None)
    text: Optional["RichText"] = p_regular(31, default=None, struct=StructType.RICH_TEXT)

    # subject
    # if subject is delegated then it always matches if this rule is present
    #  (and no other subject filters make sense)
    subject_is_delegated: Optional[bool] = p_regular(40, default=None)
    subject_is_authenticated: Optional[bool] = p_regular(41, default=None)
    subject_is_staff: Optional[bool] = p_regular(42, default=None)
    # member/owner is evaluated relative to the object
    subject_is_member: Optional[bool] = p_regular(43, default=None)
    subject_is_owner: Optional[bool] = p_regular(44, default=None)
    # subject_users, subject_groups, subject_identities, subject_roles, ...

    # verb
    effect: PolicyEffect = p_regular(60, default=PolicyEffect.DENY)
    verbs: list[AccessType] | None = p_regular(61, default=None)
    verb_kinds: list[AccessKind] | None = p_regular(62, default=None)
    _verb_mask: bitarray | None = p_runtime(default=None)

    # object (if unset it's a wildcard, except for _properties_is_<...>)
    object_node_types: Optional[list[NodeType]] = p_regular(80, default=None)
    _object_node_types_mask: bitarray | None = p_runtime(default=None)
    object_properties: list[Property] | None = p_regular(
        81, require=False, default=None, array=True, struct=StructType.PROPERTY_REFERENCE
    )
    object_properties_is_system: Optional[bool] = p_regular(82, default=None)
    object_properties_is_sensitive: Optional[bool] = p_regular(83, default=None)
    object_properties_is_kernel: Optional[bool] = p_regular(84, default=None)
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
            if value is not None:
                subject_str_parts.append(f"{subject_key[8:]}={value}")
        if subject_str_parts:
            subject_str = f"[{'&'.join(subject_str_parts)}]"
        else:
            subject_str = "*"

        verb_str_parts = []
        if self.verbs:
            verb_str_parts.extend(verb.bench_name for verb in self.verbs)
        if self.verb_kinds:
            verb_str_parts.extend(verb_kind.bench_name for verb_kind in self.verb_kinds)
        if verb_str_parts:
            verb_str = f"{'|'.join(verb_str_parts)}"
        else:
            verb_str = "*"

        if self.object_node_types:
            object_type_str_parts = tuple(t.bench_name for t in self.object_node_types)
        else:
            object_type_str_parts = []
        if self.object_properties:
            object_prop_str_parts = [
                f"{p.type.bench_name}.{p.name}" for p in self.object_properties
            ]
        else:
            object_prop_str_parts = []
        for object_properties_key in (
            "object_properties_is_system",
            "object_properties_is_sensitive",
            "object_properties_is_kernel",
        ):
            value = getattr(self, object_properties_key)
            if value is not None:
                object_prop_str_parts.append(f"{object_properties_key[7:]}={value}")

        object_type_str = "|".join(object_type_str_parts) if object_type_str_parts else "*"
        object_prop_str = "&".join(object_prop_str_parts) if object_prop_str_parts else "*"
        object_str = f"{object_type_str}.[{object_prop_str}]"

        return f"{self.name or '<unnamed>'} {self.effect.bench_name} {subject_str} {verb_str} {object_str}"

    def _clear_inner(self, scope: Optional["Node"] = None):
        self._verb_mask = None
        self._object_node_types_mask = None
        if self._object_properties_masks is not None:
            self._object_properties_masks.clear()

    def _interp_inner(self, scope: "Node", on_notice: "NoticeHandler"):
        self._update_verb_mask()
        self._update_object_mask()

    def _update_object_mask(self):
        self._object_node_types_mask = _enums_to_mask(self.object_node_types, NodeType)
        self._object_properties_masks = {}
        all_properties = self.object_properties or ()
        if self.object_properties_is_system is not None:
            all_properties = list(all_properties)
            for prop in iter_properties(*(self.object_node_types or NODE_TYPES)):
                if prop.has_id and prop.is_system == self.object_properties_is_system:
                    all_properties.append(prop)
        if self.object_properties_is_sensitive is not None:
            all_properties = list(all_properties)
            for prop in iter_properties(*(self.object_node_types or NODE_TYPES)):
                if prop.has_id and prop.is_sensitive == self.object_properties_is_sensitive:
                    all_properties.append(prop)
        if self.object_properties_is_kernel is not None:
            all_properties = list(all_properties)
            for prop in iter_properties(*(self.object_node_types or NODE_TYPES)):
                if prop.has_id and prop.is_kernel == self.object_properties_is_kernel:
                    all_properties.append(prop)
        for prop in all_properties:
            if prop.type not in self._object_properties_masks:
                self._object_properties_masks[prop.type] = bitarray(
                    prop.component.__max_property_ord__ + 1
                )
            self._object_properties_masks[prop.type][prop.ord] = True

    def _update_verb_mask(self):
        self._verb_mask = bitarray(AccessType.get_max_ord())
        if not self.verbs and not self.verb_kinds:
            self._verb_mask[AccessType.get_min_ord() : AccessType.get_max_ord()] = True
        else:
            for verb in self.verbs or ():
                self._verb_mask[verb.ord] = True
            for verb_kind in self.verb_kinds or ():
                self._verb_mask[verb_kind.from_ord : verb_kind.to_ord + 1] = True

    def matches_subject(self, subject: "Subject", object_owner: Owner) -> bool:
        if not self.subject_is_delegated:
            # subject always matches (by definition) if the policy is delegated
            if (
                self.subject_is_authenticated is not None
                and self.subject_is_authenticated != subject.is_authenticated
            ):
                return False
            if self.subject_is_member is not None and self.subject_is_member != (
                subject.memberships and object_owner not in subject.owned
            ):
                return False
            if self.subject_is_owner is not None and self.subject_is_owner != (
                subject.owned and object_owner not in subject.owned
            ):
                return False
            if self.subject_is_staff and not subject.is_staff:
                return False
        return True  # no mismatch -> match

    def matches_verb(self, verb: AccessType) -> bool:
        return self._verb_mask[verb.ord]

    #
    # Builder-style methods
    #

    def allow(self, *verbs: AccessType | AccessKind) -> "Self":
        self.effect = PolicyEffect.ALLOW
        self.verbs = [verb for verb in verbs if isinstance(verb, ACCESS_CLASSES)]
        self.verb_kinds = [verb for verb in verbs if isinstance(verb, AccessKind)]
        assert len(self.verbs) + len(self.verb_kinds) == len(verbs), f"invalid verbs {verbs!r}"
        if _COMPLETED_SETUP:
            self._update_verb_mask()
        return self

    def deny(self, *verbs: AccessType | AccessKind) -> "Self":
        self.effect = PolicyEffect.DENY
        self.verbs = [verb for verb in verbs if isinstance(verb, ACCESS_CLASSES)]
        self.verb_kinds = [verb for verb in verbs if isinstance(verb, AccessKind)]
        assert len(self.verbs) + len(self.verb_kinds) == len(verbs), f"invalid verbs {verbs!r}"
        if _COMPLETED_SETUP:
            self._update_verb_mask()
        return self

    def subject(
        self,
        is_authenticated: bool | None = None,
        is_member: bool | None = None,
        is_owner: bool | None = None,
        is_staff: bool | None = None,
    ) -> "Self":
        self.subject_is_authenticated = is_authenticated
        self.subject_is_member = is_member
        self.subject_is_owner = is_owner
        self.subject_is_staff = is_staff
        return self

    def object(
        self,
        node_types: tuple[BenchType, ...] = (),
        properties: tuple[Property, ...] = (),
        properties_is_system: Optional[bool] = None,
        properties_is_sensitive: Optional[bool] = None,
        properties_is_kernel: Optional[bool] = None,
    ) -> "Self":
        if (
            self.effect == PolicyEffect.ALLOW
            and not properties
            and properties_is_system is None
            and properties_is_sensitive is None
        ):
            # prevent footgun: default to only non-sensitive if allow and not specified
            properties_is_sensitive = False
        self.object_node_types = node_types  # type: ignore
        self.object_properties = properties  # type: ignore
        self.object_properties_is_system = properties_is_system
        self.object_properties_is_sensitive = properties_is_sensitive
        self.object_properties_is_kernel = properties_is_kernel
        if _COMPLETED_SETUP:
            self._update_object_mask()
        return self


@struct(StructType.SUBJECT)
class Subject(Struct):
    """
    The <whoever/whatever> issuing a request. Unknown/ignored attributes are unset.
    (We unset various combinations of attributes to evaluate the access of acting subjects independently.)
    """

    is_authenticated: Optional[bool] = p_system(30, default=None)
    is_staff: Optional[bool] = p_system(31, default=None)
    is_system: Optional[bool] = p_system(32, default=None)
    # (Client isn't a separate subject but useful to know)
    client: Optional["Client"] = p_system(
        40, default=None, require=False, array=False, references=NodeType.CLIENT
    )
    user: Optional["User"] = p_system(
        41, default=None, require=False, array=False, references=NodeType.USER
    )
    identity: Optional["Identity"] = p_system(
        42, default=None, require=False, array=False, references=NodeType.IDENTITY
    )
    badge: Optional["Badge"] = p_system(
        43, default=None, require=False, array=False, references=NodeType.BADGE
    )
    owned: list[Owner] = p_system(
        44,
        array=True,
        require=False,
        references=(NodeType.USER, NodeType.ORGANIZATION, NodeType.BENCH),
    )
    memberships: list[Union["Bench", "Organization"]] = p_system(
        45, require=False, array=True, references=NodeType.MEMBERSHIP
    )
    roles: list["Role"] = p_system(46, require=False, array=True, references=NodeType.ROLE)

    def split_into_acting_subjects(self, graph: NodeDataGraph) -> tuple["Subject", ...]:
        """
        Split into different subjects that may have different access and are relevant in the given graph.
         (The graph is assumed to contain all relevant owners!).
        Basically, acting subject X in "subject is acting as X" (where X may have different access).
        """

        applicable_principals: list[Subject] = [Subject(is_authenticated=False)]  # anonymous
        if self.is_authenticated:
            applicable_principals.append(Subject(is_authenticated=True))
        if self.is_staff:
            applicable_principals.append(Subject(is_staff=True))
        if self.user:
            applicable_principals.append(Subject(user=self.user))
        if self.identity:
            applicable_principals.append(Subject(identity=self.identity))
        if self.badge:
            applicable_principals.append(Subject(badge=self.badge))
        for owner in self.owned or ():
            if str(owner.id) in graph:
                applicable_principals.append(Subject(owned=[owner]))
        for membership in self.memberships or ():
            if str(membership.parent_id) in graph:
                applicable_principals.append(Subject(memberships=[membership]))
        for role in self.roles or ():
            if role.parent_type == NodeType.BLOCK:
                if str(role.parent_id) in graph:
                    applicable_principals.append(Subject(roles=[role]))
            elif role.parent_type == NodeType.MEMBERSHIP:
                if str(role.parent.parent_id) in graph:
                    applicable_principals.append(Subject(roles=[role]))
            else:
                raise BenchError(f"unexpected parent to {role!r}")

        assert len(applicable_principals) > 0, f"no applicable principals in {self!r}"
        return tuple(applicable_principals)

    def __content_str__(self):
        str_parts = []
        for prop in Subject.__declared_properties__.values():
            value = getattr(self, prop.name)
            if value:
                if isinstance(value, bool):
                    str_parts.append(prop.name)
                else:
                    str_parts.append(f"{prop.name}={value}")
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

    id: int = p_system(2, require=True)
    parent_id: int | None = p_system(4)
    scope_id: str = p_system(30)
    _scope: Optional[AnyNodeData] = p_runtime(default=None)
    identity_id: int = p_system(31)
    _identity: Optional[Identity] = p_runtime(default=None)
    rules: list[PolicyRule] = p_system(32, array=True, struct=StructType.POLICY_RULE)

    def __content_str__(self) -> str:
        return f"{self.id} for {self._identity or self.identity_id} in {self.scope_id} ({len(self.rules)} rules)"


@struct(StructType.ACCESS_MATRIX)
class AccessMatrix(Struct):
    """The materialized access matrix generated for a specific subject to quickly evaluate access for objects."""

    subject: Subject = p_system(30, require=True, struct=StructType.SUBJECT)
    identities: list[Subject] = p_system(32, array=True, struct=StructType.SUBJECT)
    scope_zones: list[AccessZone] = p_system(33, array=True, struct=StructType.ACCESS_ZONE)
    base_zones: list[AccessZone] = p_system(34, array=True, struct=StructType.ACCESS_ZONE)

    # quick access to the zone (id = index)
    _lowest_zone_by_scope: dict[tuple[int, str], AccessZone] = p_runtime(default_factory=dict)
    _base_zone_by_root: dict[tuple[int, str], AccessZone] = p_runtime(default_factory=dict)

    def __content_str__(self) -> str:
        return f"for {self.subject} ({len(self.identities)} identities, {len(self.scope_zones)} node zones, {len(self.base_zones)} base zones)"


@struct(StructType.ACCESS)
class Access(Struct):
    """
    An evaluated access on some objects as part of a larger Request (by the same subject).
    As in PolicyRule, if the decision is Deny, the object_properties are the denied ones.
    (And if object_properties is unset, it applies to all properties.)
    """

    mode: AccessMode = p_system(30, require=True)
    decision: PolicyEffect = p_system(31, require=True)
    verb: AccessType = p_system(32, require=True)
    object_type: BenchType = p_system(33, require=True)
    object_properties: list[Property] | None = p_system(
        34, require=False, array=True, default=None, struct=StructType.PROPERTY_REFERENCE
    )
    trace: Optional["AccessTrace"] = p_system(
        35, require=False, array=False, struct=StructType.ACCESS_TRACE
    )

    # arguments
    # roots, read_options, ...

    def __content_str__(self) -> str:
        if self.object_properties:
            object_properties_str = "|".join(p.name for p in self.object_properties)
            object_str = f"{self.object_type.bench_name} [{object_properties_str}]"
        else:
            object_str = f"{self.object_type.bench_name} [*]"
        return f"{self.decision.bench_name} {self.verb.bench_name} {object_str}"


@struct(StructType.ACCESS_TRACE)
class AccessTrace(Struct):
    """The trace of evaluating Access."""

    matched_rules: list[PolicyRule] = p_system(30, array=True, struct=StructType.POLICY_RULE)


@struct(StructType.REQUEST)
class Request(Struct):
    """
    A request with multiple accesses.
    TODO @Feature @Security: store, query and watch access log
    """

    subject: Subject = p_system(30, require=True, struct=StructType.SUBJECT)
    decision: PolicyEffect = p_system(31, require=True)
    accesses: list[Access] = p_system(32, array=True, require=True, struct=StructType.REQUEST)

    # scope/context
    # bench, package, space, user, ...
    transaction_id: Optional[UUID] = p_system(40, require=False, default=None)

    def __content_str__(self) -> str:
        return f"{self.decision.bench_name} [{self.subject}]: ({', '.join(str(r) for r in self.accesses)})"


def _enums_to_mask(values: list[IdEnum], cls: type[IdEnum]) -> bitarray:
    """Set the given values in a mask. No values == all values == wildcard!"""
    mask = bitarray(cls.get_max_ord() + 1)
    if not values:
        mask.setall(True)
    else:
        for value in values:
            mask[value.ord] = True
    return mask


def _properties_to_mask(values: list[Property], node_type: NodeType) -> bitarray:
    """Set the given properties in a mask. No values == all values == wildcard!"""
    mask = bitarray(NODE_CLASS_BY_TYPE[node_type].__max_property_ord__)
    if not values:
        mask.setall(True)
    else:
        for value in values:
            mask[value.ord] = True
    return mask


def _mask_to_properties(mask: bitarray, node_type: NodeType) -> tuple[Property, ...]:
    """Convert a mask to a list of properties. Empty mask == all properties == wildcard!"""
    node_cls = NODE_CLASS_BY_TYPE[node_type]
    if mask == node_cls.__properties_mask__:
        return ()
    properties = node_cls._unmask_properties(mask)
    return properties


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
        evaluation: Access | Request,
        cause: Exception | None = None,
    ):
        super().__init__(repr(evaluation), cause)
        self.evaluation = evaluation
        self.cause = cause


SYSTEM_POLICIES: tuple[Policy, ...] = (
    # NOTE: all policies (incl. these base policies) and their rules are evaluated in order
    Policy("SystemProtection").append(
        PolicyRule(
            "CannotAccessKernelProperties",
            text=RichText.plain("Kernel properties are inaccessible outside the system itself."),
        )
        .deny()
        .object(properties_is_kernel=True),
        PolicyRule(
            "CannotUpdateSystemProperties",
            text=RichText.plain(
                "System properties are not directly editable, only through special methods or relevant accesses."
            ),
        )
        .deny(EditType.UPDATE)
        .object(properties_is_system=True),
        PolicyRule(
            "CannotCreateRootNodesDirectly",
            text=RichText.plain(
                "Root nodes (Bench, Organization, User) must be created through special methods."
            ),
        )
        .deny(EditType.CREATE, EditType.UPSERT)
        .object(node_types=ROOT_NODE_TYPES.tuple),
        PolicyRule(
            "CannotEditHandles",
            text=RichText.plain(
                "Handles (like usernames) must be edited through special methods."
                # (explicitly deny this since handles are owned by the root)
            ),
        )
        .deny(AccessKind.EDIT)
        .object(node_types=(NodeType.HANDLE,)),
        PolicyRule(
            "CannotUpsertLegislativeNodes",
            text=RichText.plain(
                "Nodes that define their own policies cannot be upserted to prevent ambiguities in evaluation."
                # (we could do it, but it would be confusing and tedious)
            ),
        )
        .deny(EditType.UPSERT)
        .object(node_types=LEGISLATIVE_NODE_TYPES.tuple),
    ),
    Policy("OwnerAccess").append(
        PolicyRule(
            "OwnerCanDoAnything",
            text=RichText.plain(
                "Anyone identified as the owner of a node can always do everything."
            ),
        )
        .subject(is_owner=True)
        .allow(),
    ),
    Policy("StaffAccess").append(
        PolicyRule(
            "StaffCanReadAnything",
            text=RichText.plain("During the beta, staff users can access anything."),
        )
        .subject(is_staff=True)
        .allow(AccessKind.READ),
    ),
    Policy("MemberAccess").append(
        PolicyRule(
            "MemberCanReadBench",
            text=RichText.plain(
                "Every member of your Bench/Organization can read its non-sensitive properties."
            ),
        )
        .subject(is_member=True)
        .allow(AccessKind.READ)
        .object(
            node_types=tuple(nt for nt in IN_BENCH_NODE_TYPES if nt not in SUB_PACKAGE_NODE_TYPES),
            properties_is_sensitive=False,
        )
    ),
    Policy("AuthenticatedAccess").append(
        PolicyRule(
            "AuthenticatedCanReadPublic",
            text=RichText.plain(
                "Authenticated users can read public nodes like User, Organization, Bench, etc.."
            ),
        )
        .subject(is_authenticated=True)
        .allow(AccessKind.READ)
        .object(node_types=PUBLIC_NODE_TYPES.tuple, properties_is_sensitive=False),
    ),
    Policy("AnonymousAccess").append(
        PolicyRule(
            "AnonCanReadHandle",
            text=RichText.plain(
                "Everyone (incl. anonymous users) can read Handles (to create an account)."
            ),
        )
        .subject(is_authenticated=False)
        .allow(AccessKind.READ)
        .object((NodeType.HANDLE,))
    ),
)


@_on_completing_setup
def _interp_system_policies():
    for policy in SYSTEM_POLICIES:
        policy._interp_rec(None, on_warning_raise)


def adapt_read_options(
    subject: Subject, root_node_type: NodeType, options: ReadOptions
) -> ReadOptions:
    """
    Adapt read options based on the access to pre-filter as feasible while enabling the complete post-read check.
    Does NOT fully evaluate access yet, but avoids loading data that will be denied anyway.
    """

    from bench.language.bench import Bench

    options: ReadOptions = options.copy()

    # query ancestors up to root
    for ancestor_type in ANCESTOR_NODE_TYPES[root_node_type]:
        if ancestor_type not in options.ancestor_types:
            options.ancestor_types.append(ancestor_type)

    # if root >: Bench, also load related actual owner (User/Organization)
    root_node_cls = NODE_CLASS_BY_TYPE[root_node_type]
    if NodeType.BENCH in root_node_cls.__roots__:
        options.related_properties.append((Bench.owner,))

    # TODO @Performance @Security: also pre-filter read options for owner?

    return options


def get_edited_scopes(edits: list[EditData]) -> dict[UUID, "NodeReference"]:
    from bench.language import NodeReference
    from bench.proto import wire, wiring

    edited_scopes_ptr: dict[UUID, "NodeReference"] = {}
    for edit in edits:
        node = wiring.unwrap_some_node(edit.node)
        node_cls = NODE_CLASS_BY_TYPE[node.metatype]
        if edit.type == wire.EditType.CREATE or edit.type == wire.EditType.UPSERT:
            # scope is parent since we don't know this node yet
            if node.parent_ptr is not None:
                if node.parent_ptr.id not in edited_scopes_ptr:
                    ptr: "NodeReference" = wiring.unpack_struct(node.parent_ptr)
                    edited_scopes_ptr[ptr.id] = ptr
            elif node_cls.__roots__:
                raise ValidationError(node, f"can't create orphan")
        else:
            # scope is the edited node itself
            if node.id not in edited_scopes_ptr:
                ptr: "NodeReference" = wiring.unpack_struct(NodeReference.from_node_data(node))
                edited_scopes_ptr[ptr.id] = ptr
    return edited_scopes_ptr


def generate_access_matrix(
    subject: Subject,
    graph: NodeDataGraph,
    base_policies: tuple[Policy, ...] = SYSTEM_POLICIES,
    root_owner: Owner | None = None,
    unpacked_graph: NodeGraph | None = None,
) -> AccessMatrix:
    """Generates an access matrix to quickly evaluate access for a specific subject."""

    from bench.proto import wiring

    roots = graph.find_roots()
    identities = subject.split_into_acting_subjects(graph)
    scope_zones: list[AccessZone] = []
    base_zones: list[AccessZone] = []
    matrix = AccessMatrix(
        subject=subject, scope_zones=scope_zones, base_zones=base_zones, identities=identities
    )
    applied_policies_by_node_id: dict[str, list[Policy]] = defaultdict(list)

    def _assign_access_zones(
        current_node: AnyNodeData, owner: Owner, parent_zones_by_identity: tuple[int | None, ...]
    ):
        """Generates any new applicable access zones downstream from the node for all identities."""

        # if this node defines new policies, apply them to their scope
        if getattr(current_node, "policies", None):
            if unpacked_graph:
                new_policies: list[Policy] = unpacked_graph.get(to_uuid(current_node.id)).policies
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
        current_zones_by_identity = parent_zones_by_identity
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
                        id=len(scope_zones),
                        parent_id=parent_zones_by_identity[identity_id],
                        scope_id=current_node.id,
                        _scope=current_node,
                        identity_id=identity_id,
                        _identity=identity,
                        rules=applicable_rules,
                    )
                    scope_zones.append(zone)
                    # update parent zones for the next level
                    if current_zones_by_identity is parent_zones_by_identity:  # to list to modify
                        current_zones_by_identity = list(parent_zones_by_identity)
                        current_zones_by_identity[identity_id] = zone.id
        if current_zones_by_identity is not parent_zones_by_identity:  # back to tuple if modified
            current_zones_by_identity = tuple(current_zones_by_identity)

        # update 'lowest zone' shortcuts (per identity)
        for identity_id in range(len(identities)):
            zone_id = current_zones_by_identity[identity_id]
            zone = scope_zones[zone_id] if zone_id is not None else None
            matrix._lowest_zone_by_scope[(identity_id, current_node.id)] = zone

        # descend into children
        #  (even if they don't have any legislative nodes since we want the runtime-only zone mapping)
        current_type: NodeType = wiring.unpack_enum(NodeType, current_node.metatype)
        for child_type in CHILD_NODE_TYPES[current_type]:
            for child_node in graph.iter_descendants(current_node, child_type):
                _assign_access_zones(child_node, owner, current_zones_by_identity)

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
                id=len(base_zones),
                scope_id=root.id,
                _scope=root,
                parent_id=None,
                identity_id=identity_id,
                _identity=identity,
                rules=base_rules,
            )
            base_zones.append(base_zone)
            matrix._base_zone_by_root[(identity_id, root.id)] = base_zone

        # add nested zones if there are any legislative nodes down here
        _assign_access_zones(root, owner, root_zones_by_identity)

    return matrix


class _EvalCacheKey(NamedTuple):
    object_node_type: NodeType
    # assumes object_properties are equivalent for every object_node_type in request
    root_id: str
    start_scope_id: int | None
    identity_id: int


def evaluate_access(
    *,
    mode: AccessMode,
    matrix: AccessMatrix,
    verb: AccessType,
    object_node_type: NodeType,
    object_properties: bitarray,
    root_id: str,
    scope_id: str | None,
    trace: bool = False,
    cache: dict[_EvalCacheKey, bitarray] | None = None,
) -> tuple[bitarray, Access | None, bool]:
    """
    Evaluates Access for the given object type and properties in that scope.
    Returns the *allowed* properties and the Access.
    If passing a cache, caches evals per identity and Access is only created if the request is new..
    """

    # the granted 'allow' mask for properties across identities
    composite_allowed_properties = bitarray(len(object_properties))
    object_node_cls = NODE_CLASS_BY_TYPE[object_node_type]
    matched_rules: list[PolicyRule] = [] if trace else None
    num_cached_identities = 0

    # check the zones for each identity (separately)
    for identity_id in range(len(matrix.identities)):
        base_zone: AccessZone = matrix._base_zone_by_root[(identity_id, root_id)]
        start_scope_zone: AccessZone | None = matrix._lowest_zone_by_scope[(identity_id, scope_id)]

        # check cache
        cache_key = _EvalCacheKey(
            object_node_type=object_node_type,
            root_id=root_id,
            start_scope_id=start_scope_zone.id if start_scope_zone is not None else None,
            identity_id=identity_id,
        )
        if cache is not None:
            allowed_properties = cache.get(cache_key)
        else:
            allowed_properties = None

        if allowed_properties is None:  # not cached
            # first check the base zones, then walk the zones starting from the lowest
            allowed_properties = bitarray(len(object_properties))  # for this identity
            unset_properties = object_properties  # the unmatched properties so far
            current_zone = base_zone
            while unset_properties.any():
                for rule in current_zone.rules:
                    if (
                        rule.matches_verb(verb)
                        and rule._object_node_types_mask[object_node_type.ord]
                    ):
                        if trace:
                            matched_rules.append(rule)
                        if rule.effect == PolicyEffect.ALLOW:
                            rule_properties_mask = rule._object_properties_masks.get(
                                object_node_type, object_node_cls.__properties_mask__
                            )
                            allowed_properties |= rule_properties_mask & unset_properties
                        else:
                            rule_properties_mask = rule._object_properties_masks.get(
                                object_node_type, bitarray(object_node_cls.__max_property_ord__ + 1)
                            )
                            allowed_properties &= ~(rule_properties_mask & unset_properties)
                        unset_properties = unset_properties & ~rule_properties_mask
                        if not unset_properties.any():
                            break  # nothing can change anymore

                # advance to the next zone
                if current_zone.id == base_zone.id:
                    if start_scope_zone is None:
                        break  # no scope zones
                    current_zone = start_scope_zone
                elif current_zone.parent_id is not None:
                    current_zone = matrix.scope_zones[current_zone.parent_id]
                else:
                    break  # reached the top

        # accumulate (OR) allowed properties across identities
        composite_allowed_properties |= allowed_properties
        if composite_allowed_properties == object_properties:
            break  # already fully allowed

    # sum into decision
    if mode == AccessMode.ADAPTIVE:  # if any property was allowed -> access is allowed
        if composite_allowed_properties.any():
            decision = PolicyEffect.ALLOW
        else:
            decision = PolicyEffect.DENY
    elif mode == AccessMode.ATOMIC:  # if any property was rejected -> access is denied
        if composite_allowed_properties == object_properties:
            decision = PolicyEffect.ALLOW
        else:
            decision = PolicyEffect.DENY
    else:
        raise ValueError(f"unexpected mode {mode!r}")
    if decision == PolicyEffect.ALLOW:
        access_properties = composite_allowed_properties
    else:
        access_properties = ~composite_allowed_properties & object_properties

    if num_cached_identities < len(matrix.identities):
        access = Access(
            mode=mode,
            decision=decision,
            verb=verb,
            object_type=object_node_type,
            object_properties=_mask_to_properties(access_properties, object_node_type),
            trace=AccessTrace(matched_rules=matched_rules) if trace else None,
        )
        was_cached = False
    else:
        access = None
        was_cached = True
    return composite_allowed_properties, access, was_cached


def evaluate_and_adapt_read(
    matrix: AccessMatrix,
    graph: NodeDataGraph,
    *,
    adapt_nodes_in_place: bool,
    required_nodes: Collection[NodeReferenceData] | None = None,
    trace: bool = False,
) -> tuple[Request, Collection[AnyNodeData]]:
    """
    Evaluate *and* adapt access to all nodes in the given graph, pruning nodes & properties as needed.
     -> unlike for other accesses, we don't outright reject GET reads, you just get less (or zero) data.
    In case a node was completely denied but its children weren't, we include a Skip node in the result.
    If no overall owner is given, the owners (i.e. actual roots) must be in the graph.
    Assumes that all policies are valid.
    """
    from bench.proto import wire, wiring

    visible_nodes: list[AnyNodeData] = []
    accesses: list[Access] = []
    skips: dict[str, wire.SkipData] = {}
    cache: dict[Any, Any] = {}

    # adapt & filter nodes
    verb = ReadType.GET  # same for all?
    for node in graph.nodes:
        object_node_type: NodeType = wiring.unpack_enum(NodeType, node.metatype)
        object_node_cls = NODE_CLASS_BY_TYPE[object_node_type]
        object_properties: bitarray = object_node_cls.__properties_mask__

        root = graph.get_root(node)  # a bit inefficient?
        adapted_properties, access, was_cached = evaluate_access(
            matrix=matrix,
            verb=verb,
            object_node_type=object_node_type,
            object_properties=object_properties,
            root_id=root.id,
            scope_id=node.id,
            mode=AccessMode.ADAPTIVE,
            trace=trace,
            cache=cache,
        )
        if not was_cached:
            accesses.append(access)
        if access.decision == PolicyEffect.DENY:
            skips[node.id] = UNSET  # mark as skipped
        elif adapted_properties == object_properties:
            visible_nodes.append(node)
        else:
            node_cls = NODE_CLASS_BY_TYPE[object_node_type]
            # prune node properties to only allowed ones
            if not adapt_nodes_in_place:
                # TODO @Performance: avoid copying properties that we'll prune anyway
                node = wiring.copy_struct_data(node)
            pruned_properties = object_properties & (object_properties ^ adapted_properties)
            for pruned_prop_ord in pruned_properties.search(True):
                prop = node_cls.__properties_in_order__[pruned_prop_ord]
                setattr(node, prop.name, None)
            visible_nodes.append(node)

    # add any required skipped nodes back in (as Skips)
    for node in visible_nodes:
        if node.parent_ptr is not None and skips.get(node.parent_ptr.id, None) is UNSET:
            skip = wire.SkipData(
                metatype=wire.NodeType.SKIP,
                id=node.id,
                ck=getattr(node, "ck", None),
                parent_ptr=node.parent_ptr,
                revision=node.revision,
                order_key=getattr(node, "order_key", None),
                type=node.metatype,
            )
            skips[node.parent_ptr.id] = skip
            visible_nodes.append(skip)

    if required_nodes and any(n.id in skips for n in required_nodes):
        decision = PolicyEffect.DENY
    else:
        decision = PolicyEffect.ALLOW
    access = Request(subject=matrix.subject, accesses=accesses, decision=decision)
    return access, visible_nodes


def evaluate_edit(
    matrix: AccessMatrix, graph: NodeDataGraph, edits: Collection[EditData], *, trace: bool = False
) -> Request:
    """
    Evaluates whether the given policies (base and in graph) allow the given edits.
    Assumes that all policies are valid, and that all relevant scopes are in the graph.
    """
    from bench.proto import wiring

    accesses: list[Access] = []
    # when creating nested nodes in one transaction, the graph only knows about their 'root',
    #  so we remember the scopes for the new nodes to know which zone to use
    new_node_scopes_by_child_id: dict[str, str] | None = None
    for edit in edits:
        access_type: AccessType = wiring.unpack_enum(EditType, edit.type)
        node_type: NodeType = wiring.unpack_enum(NodeType, edit.node_type)
        node_cls = NODE_CLASS_BY_TYPE[node_type]
        node = wiring.unwrap_some_node(edit.node)

        # figure out the scope to evaluate the edit in
        if node_cls.__roots__:
            # regular non-root node: scope = parent if creating, else scope = node
            if edit.type in (EditType.CREATE, EditType.UPSERT):
                if new_node_scopes_by_child_id is None:
                    new_node_scopes_by_child_id = {}
                scope_id = node.parent_ptr.id
                while scope_id in new_node_scopes_by_child_id:
                    scope_id = new_node_scopes_by_child_id[scope_id]
                new_node_scopes_by_child_id[node.id] = scope_id
            else:
                scope_id = node.id
            scope = graph.get(scope_id)
            assert scope is not None, f"scope {scope_id} for {edit!r} not in {graph!r}"
            root = graph.get_root(scope)
        else:
            if edit.type in (EditType.CREATE, EditType.UPSERT):
                # there's a system rule against creating roots, but would need special logic to enforce it
                #  (because root would be node itself, which isn't in the matrix as we expect)
                return Request(
                    decision=PolicyEffect.DENY, subject=matrix.subject, accesses=accesses
                )
            else:
                scope = node
                root = node

        # and evaluate it
        object_properties = (
            _ints_to_mask(edit.properties, node_cls.__max_property_ord__ + 1)
            & node_cls.__properties_mask__
        )
        _, access, _ = evaluate_access(
            matrix=matrix,
            verb=access_type,
            object_node_type=node_type,
            object_properties=object_properties,
            root_id=root.id,
            scope_id=scope.id,
            mode=AccessMode.ATOMIC,
            trace=trace,
        )
        accesses.append(access)
        if access.decision == PolicyEffect.DENY:
            # implicit or explicit deny for access -> deny entire request
            return Request(decision=PolicyEffect.DENY, subject=matrix.subject, accesses=accesses)

    # at this point no implicit or explicit denies have happened -> allow
    return Request(decision=PolicyEffect.ALLOW, subject=matrix.subject, accesses=accesses)


def evaluate_run(
    matrix: AccessMatrix,
    run_type: RunType,
    node: "Block",
    *,
    trace: bool = False,
) -> Request:
    """
    Evaluates whether the given policies (base and in graph) allow the given run access.
    Assumes that all policies are valid.
    """
    node_cls = NODE_CLASS_BY_TYPE[node.metatype]
    _, access, _ = evaluate_access(
        matrix=matrix,
        verb=run_type,
        object_node_type=node.metatype,
        object_properties=node_cls.__properties_mask__,
        scope_id=str(node.id),
        root_id=str(node.bench.id),
        mode=AccessMode.ATOMIC,
        trace=trace,
    )
    return Request(decision=access.decision, subject=matrix.subject, accesses=[access])
