from collections import defaultdict
from datetime import datetime
from typing import TYPE_CHECKING, Any, Collection, NamedTuple, Optional, Self, Union
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
    BenchError,
    ConditionalOp,
    EditType,
    NodeType,
    ObjectType,
    PolicyEffect,
    ReadType,
    StructType,
    UseType,
)
from bench.language.expression import C, Expression, NodeReference
from bench.language.graph import NodeDataGraph, NodeGraph, NodeList
from bench.language.node import NODE_CLASS_BY_TYPE, Node, Struct, _on_completing_setup, node, struct
from bench.language.notice import NoticeHandler
from bench.language.property import (
    Property,
    p_internal,
    p_node_child,
    p_node_parent,
    p_regular,
    p_runtime,
    p_system,
)
from bench.language.setup import (
    _COMPLETED_SETUP,
    ANCESTOR_NODE_TYPES,
    CHILD_NODE_TYPES,
    NODE_CLASSES,
)
from bench.language.text import Text
from bench.language.user import Membership, User
from bench.language.validation import ValidationError, validate_name
from bench.proto.wire import AnyNodeData, EditData, NodeReferenceData
from bench.utils.func import IdEnum, bytetuple, to_uuid

if TYPE_CHECKING:
    from bench.language import Bench, Block, Client, Organization, Package

# default (read) access options
FILTER_DEFAULT: Expression = C(ConditionalOp.AND, clauses=[])
SELECT_DEFAULT_PROPERTIES: dict[NodeType, tuple[Property, ...]] = {}
SELECT_ALL_PROPERTIES: dict[NodeType, tuple[Property, ...]] = {}


@_on_completing_setup
def _populate_default_access():
    FILTER_DEFAULT.clauses = [
        C(ConditionalOp.NOT_EXISTS, property=Node.deleted_at),
        C(ConditionalOp.NOT_EXISTS, property=Node.archived_at),
    ]
    for node_t in NODE_CLASSES:
        SELECT_DEFAULT_PROPERTIES[node_t.metatype] = tuple(
            prop for prop in node_t.__stored_properties__.values() if not prop.is_deferred
        )
        SELECT_ALL_PROPERTIES[node_t.metatype] = tuple(node_t.__stored_properties__.values())


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
        *tuple(nt for nt in NODE_TYPES if "policies" in NODE_CLASS_BY_TYPE[nt].__properties__)
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

    parent: Union["Package", "Block"] = p_node_parent(4, NodeType.PACKAGE, NodeType.BLOCK)
    name: str = p_regular(31, validate=validate_name)
    delegated_policies: list["Policy"] = p_regular(32, array=True, struct=StructType.POLICY)
    expires_at: Optional[datetime] = p_regular(33, default=None)
    key: Optional[str] = p_internal(
        40, unique=True, default=None, encrypt=True, defer=True, sensitive=True
    )
    key_hash: Optional[str] = p_internal(
        41, unique=True, default=None, encrypt=True, defer=True, sensitive=True
    )
    password: Optional[str] = p_internal(42, default=None, encrypt=True, defer=True, sensitive=True)
    password_hash: Optional[str] = p_internal(
        43, default=None, encrypt=True, defer=True, sensitive=True
    )


@node(NodeType.ROLE)
class Role(Node):
    """
    Attach a role to a block or member.
    Role policies are delegated to the parent and its descendants.
    The delegated policies apply to all descendant's accesses.
    """

    parent: Union["Block", "Membership"] = p_node_parent(4, NodeType.BLOCK, NodeType.MEMBERSHIP)
    type: "Block" = p_regular(30, array=False, require=True, references=NodeType.BLOCK)


@node(NodeType.IDENTITY)
class Identity(Node):
    """
    Attach an identity to a block, member or user (only the user itself can do that).
    Identity policies are delegated to the parent and its descendants.
    The delegated policies apply to all descendant's accesses.
    """

    parent: Union["Block", "Membership", "User"] = p_node_parent(
        4, NodeType.BLOCK, NodeType.MEMBERSHIP, NodeType.USER
    )
    type: "Block" = p_regular(30, array=False, require=True, references=NodeType.BLOCK)

    roles: NodeList["Role"] = p_node_child(NodeType.ROLE)


@struct(StructType.READ_OPTIONS, inline=True)
class ReadOptions(Struct):
    """
    Fine-grained options to a read request.
    This is an addition to primary options (like the filter for a search or aggregation).
    """

    # relations
    ancestor_types: list[NodeType] = p_regular(31, array=True, require=False)
    descendant_types: list[NodeType] = p_regular(32, array=True, require=False)
    related_properties: list[Property] = p_regular(
        33, require=False, array=True, struct=StructType.PROPERTY_REFERENCE
    )

    # properties (include/exclude relative to default OR select specific properties)
    include_properties: list[Property] = p_regular(
        40, require=False, array=True, struct=StructType.PROPERTY_REFERENCE
    )
    exclude_properties: list[Property] = p_regular(
        41, require=False, array=True, struct=StructType.PROPERTY_REFERENCE
    )
    select_properties: list[Property] = p_regular(
        42, require=False, array=True, struct=StructType.PROPERTY_REFERENCE
    )
    select_all_properties: bool = p_regular(43, default=False)

    # filters (simplified for now)
    include_hidden: bool = p_regular(50, default=False)

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
            select_properties=list(self.select_properties),
            select_all_properties=self.select_all_properties,
            include_hidden=self.include_hidden,
        )

    def related(self, node_type: NodeType) -> list[Property] | tuple[Property, ...]:
        return tuple(p for p in self.related_properties if p.type == node_type)

    def select(self, node_type: NodeType) -> list[Property] | tuple[Property, ...]:
        if self.select_all_properties:
            return SELECT_ALL_PROPERTIES[node_type]
        elif self.select_properties:
            # select specific properties
            return tuple(p for p in self.select_properties if p.type == node_type)
        else:
            # select default properties +/- include/exclude
            properties = SELECT_DEFAULT_PROPERTIES[node_type]
            if self.include_properties:
                properties = properties + tuple(
                    p for p in self.include_properties if p.type == node_type
                )
            if self.exclude_properties:
                properties = tuple(
                    p for p in properties if not any(e.id == p.id for e in self.exclude_properties)
                )
            return properties

    def filter(
        self, node_type: NodeType, custom_filter: Optional["Expression"] = None
    ) -> "Expression":
        filter = C(ConditionalOp.TRUE) if self.include_hidden else FILTER_DEFAULT
        if custom_filter is not None:
            filter &= custom_filter
        return filter

    @staticmethod
    def default():
        """Read default: exclude soft delete & archived, select all non-deferred properties."""
        return ReadOptions()

    @staticmethod
    def all():
        """Read all: include everything, select all properties."""
        return ReadOptions(include_hidden=True, select_all_properties=True)


@struct(StructType.POLICY)
class Policy(Struct):
    """
    A policy regulating access to nodes within its scope.
    The scope is determined by where its attached, but may be further restricted using 'scopes'.

    The basics of access control:
     1. Access is DENIED implicitly unless explicitly and fully ALLOWED.
     2. Policies are either attached
        a. directly to nodes (scoped to the node)
        b. or to a delegate (full scope or less as specified)
     3. Policies are evaluated in order up the node graph, first match decides*.
        (This means you can read a sub block but not its parent.)
     4. Every identity/role/... applicable to a subject is evaluated separately and *any* allow wins.

     * Conceptually, we do 'ray trace' up the graph for every node/property/...,
        but actually doing that for every request is prohibitively expensive.
        Instead, we 'rasterize' an 'access matrix' and use 'zones' as a shortcut.
    """

    name: str = p_regular(30, validate=validate_name)
    text: Optional["Text"] = p_regular(31, default=None, struct=StructType.TEXT)
    rules: list["PolicyRule"] = p_regular(32, array=True, struct=StructType.POLICY_RULE)
    scopes: list["Block"] = p_regular(33, require=False, array=True, references=NodeType.BLOCK)

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

    name: str = p_regular(30, validate=validate_name)
    text: Optional["Text"] = p_regular(31, default=None, struct=StructType.TEXT)

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
    verbs: list[AccessType] = p_regular(61, array=True)
    verb_kinds: list[AccessKind] = p_regular(62, array=True)
    _verb_mask: bitarray | None = p_runtime(default=None)

    # object (if unset it's a wildcard, except for _properties_is_<...>)
    object_node_types: Optional[list[NodeType]] = p_regular(80, array=True)
    _object_node_types_mask: bitarray | None = p_runtime(default=None)
    object_properties: list[Property] = p_regular(
        81, require=False, array=True, struct=StructType.PROPERTY_REFERENCE
    )
    object_properties_is_system: Optional[bool] = p_regular(82, default=None)
    object_properties_is_sensitive: Optional[bool] = p_regular(83, default=None)
    object_properties_is_kernel: Optional[bool] = p_regular(84, default=None)
    _object_properties_masks: dict[NodeType, bitarray] = p_runtime(default=None)

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
        if (
            self.object_properties_is_system is not None
            or self.object_properties_is_sensitive is not None
            or self.object_properties_is_kernel is not None
        ):
            all_properties = list(all_properties)
            for node_type in self.object_node_types or NODE_TYPES:
                node_cls = NODE_CLASS_BY_TYPE[node_type]
                for prop in node_cls.__runtime_properties__.values():
                    if prop.has_id and (
                        prop.is_system == self.object_properties_is_system
                        or prop.is_sensitive == self.object_properties_is_sensitive
                        or prop.is_kernel == self.object_properties_is_kernel
                    ):
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
                assert type(verb) is AccessType, f"unexpected verb {verb!r}"
                self._verb_mask[verb.ord] = True
            for verb_kind in self.verb_kinds or ():
                self._verb_mask[verb_kind.from_ord : verb_kind.to_ord + 1] = True

    def matches_subject(self, subject: "Subject", object_owner: Owner) -> bool:
        # subject always matches (by definition) if the policy is delegated
        if not self.subject_is_delegated:
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
        assert type(verb) is AccessType, f"unexpected verb {verb!r}"
        return self._verb_mask[verb.ord]

    #
    # Builder-style methods
    #

    def allow(self, *verbs: AccessType | AccessKind) -> "Self":
        self.effect = PolicyEffect.ALLOW
        self.verbs = [verb.to(AccessType) for verb in verbs if isinstance(verb, ACCESS_CLASSES)]
        self.verb_kinds = [verb for verb in verbs if isinstance(verb, AccessKind)]
        assert len(self.verbs) + len(self.verb_kinds) == len(verbs), f"invalid verbs {verbs!r}"
        if _COMPLETED_SETUP:
            self._update_verb_mask()
        return self

    def deny(self, *verbs: AccessType | AccessKind) -> "Self":
        self.effect = PolicyEffect.DENY
        self.verbs = [verb.to(AccessType) for verb in verbs if isinstance(verb, ACCESS_CLASSES)]
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
        node_types: tuple[ObjectType, ...] = (),
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
    badges: list["Badge"] = p_system(43, require=False, array=True, references=NodeType.BADGE)
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
        if self.badges:
            for badge in self.badges:
                applicable_principals.append(Subject(badges=[badge]))
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

    scope_id: str = p_system(30)
    _scope: Optional[AnyNodeData] = p_runtime(default=None)
    identity_id: int = p_system(31)
    _identity: Optional[Identity] = p_runtime(default=None)
    rules: list[PolicyRule] = p_system(32, array=True, struct=StructType.POLICY_RULE)

    def __content_str__(self) -> str:
        return f"{(self._identity or self.identity_id)!r} in {self._scope or self.scope_id}: {len(self.rules)} rules"


@struct(StructType.ACCESS_MATRIX)
class AccessMatrix(Struct):
    """The materialized access matrix generated for a specific subject to quickly evaluate access for objects."""

    subject: Subject = p_system(30, require=True, struct=StructType.SUBJECT)
    identities: list[Subject] = p_system(32, array=True, struct=StructType.SUBJECT)
    scoped_zones: list[AccessZone] = p_system(33, array=True, struct=StructType.ACCESS_ZONE)
    base_zones: list[AccessZone] = p_system(34, array=True, struct=StructType.ACCESS_ZONE)

    # quick access to the zone (id = index)
    _scoped_zones_by_id: dict[int, AccessZone] = p_runtime(default_factory=dict)
    _lowest_zone_by_scope: dict[tuple[int, str], AccessZone] = p_runtime(default_factory=dict)
    _base_zone_by_root: dict[tuple[int, str], AccessZone] = p_runtime(default_factory=dict)

    def __content_str__(self) -> str:
        return f"{self.subject!r}: {len(self.identities)} identities, {len(self.scoped_zones)} scoped zones, {len(self.base_zones)} base zones"


@struct(StructType.ACCESS, inline=True)
class Access(Struct):
    """
    An evaluated access on some objects as part of a larger Request (by the same subject).
    As in PolicyRule, if the decision is Deny, the object_properties are the denied ones.
    (And if object_properties is unset, it applies to all properties.)
    """

    mode: AccessMode = p_system(30, require=True)
    decision: PolicyEffect = p_system(31, require=True)
    verb: AccessType = p_system(32, require=True)
    object_type: ObjectType = p_system(33, require=True)
    object_properties: list[Property] = p_system(
        34, require=False, array=True, struct=StructType.PROPERTY_REFERENCE
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
    """A request comprising multiple Accesses."""

    subject: Subject = p_system(30, require=True, struct=StructType.SUBJECT)
    decision: PolicyEffect = p_system(31, require=True)
    accesses: list[Access] = p_system(32, array=True, require=True, struct=StructType.ACCESS)

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
    Policy(name="SystemProtection").append(
        PolicyRule(
            name="CannotAccessKernelProperties",
            text=Text.plain("Kernel properties are inaccessible outside of the system."),
        )
        .deny()
        .object(properties_is_kernel=True),
        PolicyRule(
            name="CannotUpdateSystemProperties",
            text=Text.plain("System properties must be edited through designated methods."),
        )
        .deny(EditType.UPDATE)
        .object(properties_is_system=True),
        PolicyRule(
            name="CannotCreateOrDeleteSystemNodesDirectly",
            text=Text.plain("System nodes existence must be managed through special methods."),
        )
        .deny(
            EditType.CREATE,
            EditType.UPSERT,
            EditType.SOFT_DELETE,
            EditType.RESTORE,
            EditType.ARCHIVE,
            EditType.UNARCHIVE,
            EditType.DELETE,
        )
        .object(node_types=(*ROOT_NODE_TYPES.tuple, NodeType.CLIENT)),
        PolicyRule(
            name="CannotEditHandles",
            text=Text.plain(
                "Handles (like usernames) must be edited through special methods."
                # (explicitly deny this since handles are owned by the root via OwnerAccess)
            ),
        )
        .deny(AccessKind.EDIT)
        .object(node_types=(NodeType.HANDLE,)),
        PolicyRule(
            name="CannotUpsertLegislativeNodes",
            text=Text.plain(
                "Nodes that define their own policies cannot be upserted to prevent ambiguities in evaluation."
                # (we could do it, but it would be confusing and tedious)
            ),
        )
        .deny(EditType.UPSERT)
        .object(node_types=LEGISLATIVE_NODE_TYPES.tuple),
    ),
    Policy(name="OwnerAccess").append(
        PolicyRule(
            name="OwnerCanDoAnything",
            text=Text.plain("Anyone identified as the owner of a node can always do everything."),
        )
        .subject(is_owner=True)
        .allow(),
    ),
    Policy(name="StaffAccess").append(
        PolicyRule(
            name="StaffCanReadAnythingDuringBeta",
            text=Text.plain("During the beta, staff users can access anything."),
        )
        .subject(is_staff=True)
        .allow(AccessKind.READ),
    ),
    Policy(name="MemberAccess").append(
        PolicyRule(
            name="MemberCanReadBench",
            text=Text.plain(
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
    Policy(name="AuthenticatedAccess").append(
        PolicyRule(
            name="AuthenticatedCanReadPublic",
            text=Text.plain(
                "Authenticated users can read public nodes like User, Organization, Bench, etc.."
            ),
        )
        .subject(is_authenticated=True)
        .allow(AccessKind.READ)
        .object(node_types=PUBLIC_NODE_TYPES.tuple, properties_is_sensitive=False),
    ),
    Policy(name="AnonymousAccess").append(
        PolicyRule(
            name="AnonCanReadHandle",
            text=Text.plain(
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
    from bench.language.notice import on_warning_raise

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

    options = options.copy()

    # query ancestors up to root
    for ancestor_type in ANCESTOR_NODE_TYPES[root_node_type]:
        if ancestor_type not in options.ancestor_types:
            options.ancestor_types.append(ancestor_type)

    # if root >: Bench, also load related actual owner (User/Organization)
    root_node_cls = NODE_CLASS_BY_TYPE[root_node_type]
    if NodeType.BENCH in root_node_cls.__roots__:
        options.related_properties.append((Bench.owner,))

    # TODO :Performance :Security: also pre-filter read options for owner?

    return options


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
    matrix = AccessMatrix(subject=subject, identities=identities)
    applied_policies_by_node_id: dict[str, list[Policy]] = defaultdict(list)

    def _assign_access_zones(
        current_node: AnyNodeData, owner: Owner, parent_zones_by_identity: dict[int, int]
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
        new_zones_by_identity: dict[int, int | None] | None = None
        if applied_policies:
            for identity in identities:
                applicable_rules = tuple(
                    rule
                    for policy in applied_policies
                    for rule in policy.rules
                    if rule.matches_subject(identity, owner)
                )
                if applicable_rules:
                    # we got a new zone with different roles down here
                    zone = AccessZone(
                        parent_id=parent_zones_by_identity.get(identity.id),
                        scope_id=current_node.id,
                        _scope=current_node,
                        identity_id=identity.id,
                        _identity=identity,
                        rules=applicable_rules,
                    )
                    matrix.scoped_zones.append(zone)
                    matrix._scoped_zones_by_id[zone.id] = zone
                    # update parent zones for the next level
                    if new_zones_by_identity is None:
                        new_zones_by_identity = {}
                    new_zones_by_identity[identity.id] = zone.id
        if new_zones_by_identity is None:
            new_zones_by_identity = parent_zones_by_identity
        else:  # merge update
            for identity_id, zone_id in parent_zones_by_identity.items():
                if identity_id not in new_zones_by_identity:
                    new_zones_by_identity[identity_id] = zone_id

        # update 'lowest zone' shortcuts (per identity)
        for identity in identities:
            zone_id = new_zones_by_identity.get(identity.id)
            zone = matrix._scoped_zones_by_id[zone_id] if zone_id is not None else None
            matrix._lowest_zone_by_scope[(identity.id, current_node.id)] = zone

        # descend into children
        #  (even if they don't have any legislative nodes since we want the runtime-only zone mapping)
        current_type: NodeType = wiring.unpack_enum(NodeType, current_node.metatype)
        for child_type in CHILD_NODE_TYPES[current_type]:
            for child_node in graph.iter_descendants(current_node, child_type):
                _assign_access_zones(child_node, owner, new_zones_by_identity)

    # start at root
    root_zones_by_identity = {}
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
        for identity in identities:
            base_rules = tuple(
                rule
                for policy in base_policies
                for rule in policy.rules
                if rule.matches_subject(identity, owner)
            )
            base_zone = AccessZone(
                scope_id=root.id,
                _scope=root,
                parent_id=None,
                identity_id=identity.id,
                _identity=identity,
                rules=base_rules,
            )
            matrix.base_zones.append(base_zone)
            matrix._base_zone_by_root[(identity.id, root.id)] = base_zone

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
    verb = verb.to(AccessType)
    composite_allowed_properties = bitarray(len(object_properties))
    object_node_cls = NODE_CLASS_BY_TYPE[object_node_type]
    matched_rules: list[PolicyRule] = [] if trace else None
    num_cached_identities = 0

    # check the zones for each identity (separately)
    for identity in matrix.identities:
        base_zone: AccessZone = matrix._base_zone_by_root[(identity.id, root_id)]
        start_scoped_zone: AccessZone | None = matrix._lowest_zone_by_scope[(identity.id, scope_id)]

        # check cache
        cache_key = _EvalCacheKey(
            object_node_type=object_node_type,
            root_id=root_id,
            start_scope_id=start_scoped_zone.id if start_scoped_zone is not None else None,
            identity_id=identity.id,
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
                                object_node_type, object_node_cls.__properties_mask_set__
                            )
                            allowed_properties |= rule_properties_mask & unset_properties
                        else:
                            rule_properties_mask = rule._object_properties_masks.get(
                                object_node_type, object_node_cls.__properties_mask_unset__
                            )
                            allowed_properties &= ~(rule_properties_mask & unset_properties)
                        unset_properties = unset_properties & ~rule_properties_mask
                        if not unset_properties.any():
                            break  # nothing can change anymore

                # advance to the next zone
                if current_zone.id == base_zone.id:
                    if start_scoped_zone is None:
                        break  # no scope zones
                    current_zone = start_scoped_zone
                elif current_zone.parent.id is not None:
                    current_zone = matrix.scoped_zones[current_zone.parent_id]
                else:
                    break  # reached the top

        # accumulate (OR) allowed properties across identities
        composite_allowed_properties |= allowed_properties
        if composite_allowed_properties == object_properties:
            break  # already fully allowed

    # sum into decision
    if mode == AccessMode.ADAPTIVE:  # adaptive  = if any property was allowed -> access is allowed
        if composite_allowed_properties.any():
            decision = PolicyEffect.ALLOW
        else:
            decision = PolicyEffect.DENY
    elif mode == AccessMode.ATOMIC:  # atomic = if any property was rejected -> access is denied
        if composite_allowed_properties == object_properties:
            decision = PolicyEffect.ALLOW
        else:
            decision = PolicyEffect.DENY
    else:
        raise ValueError(f"unexpected mode {mode!r}")
    if decision == PolicyEffect.ALLOW:
        decided_properties = composite_allowed_properties
    elif decision == PolicyEffect.DENY:  # denied properties are inverse of allowed
        decided_properties = ~composite_allowed_properties & object_properties
    else:
        raise ValueError(f"unexpected decision {decision!r}")
    if num_cached_identities < len(matrix.identities):
        if decided_properties.all():
            object_properties = ()
        else:
            object_properties = object_node_cls._unmask_properties(decided_properties)
        access = Access(
            mode=mode,
            decision=decision,
            verb=verb,
            object_type=object_node_type,
            object_properties=object_properties,
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

    NOTE: assumes that all policies are valid.
    NOTE: nodes are returned in pre-order (parents before children).
    """
    from bench.proto import wire, wiring

    visible_nodes: list[AnyNodeData] = []
    accesses: list[Access] = []
    skips: dict[str, wire.SkipData] = {}
    cache: dict[Any, Any] = {}

    # adapt & filter nodes

    def _adapt_descendants(root: AnyNodeData, n: AnyNodeData) -> None:
        # evaluate access
        object_node_type: NodeType = wiring.unpack_enum(NodeType, n.metatype)
        object_node_cls = NODE_CLASS_BY_TYPE[object_node_type]
        object_properties: bitarray = object_node_cls.__properties_mask_set__
        allowed_properties, access, was_cached = evaluate_access(
            matrix=matrix,
            verb=ReadType.GET,  # same for all?
            object_node_type=object_node_type,
            object_properties=object_properties,
            root_id=root.id,
            scope_id=n.id,
            mode=AccessMode.ADAPTIVE,
            trace=trace,
            cache=cache,
        )
        if not was_cached:
            accesses.append(access)

        # apply decision (skip or adapt)
        if access.decision == PolicyEffect.DENY:
            skips[n.id] = UNSET  # mark as skipped
        elif allowed_properties == object_properties:
            visible_nodes.append(n)
        else:
            node_cls = NODE_CLASS_BY_TYPE[object_node_type]
            # prune node properties to only allowed ones
            if not adapt_nodes_in_place:
                # TODO :Performance: avoid copying properties that we'll prune anyway
                n = wiring.copy_data(n)
            pruned_properties = object_properties & (object_properties ^ allowed_properties)
            for pruned_prop_ord in pruned_properties.search(True):
                prop = node_cls.__properties_in_order__[pruned_prop_ord]
                setattr(n, prop.name, None)
            n.metatype = object_node_type  # always keep metatype
            visible_nodes.append(n)

        # traverse children
        for child in graph.iter_descendants(n):
            _adapt_descendants(root, child)

    for root in graph.find_roots():
        _adapt_descendants(root, root)

    # add any required skipped nodes back in (as Skips)
    for n in visible_nodes:
        if n.parent_ptr is not None and skips.get(n.parent_ptr.id, None) is UNSET:
            skip = wire.SkipData(
                metatype=wire.NodeType.SKIP,
                id=n.id,
                ck=getattr(n, "ck", None),
                parent_ptr=n.parent_ptr,
                revision=n.revision,
                order_key=getattr(n, "order_key", None),
                reference_ptr=NodeReference.from_node_data(n)
            )
            skips[n.parent_ptr.id] = skip
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
    TODO :Broken :Security: verify equivalent 'use' access for the edits
     (e.g. create Run with base=Block <=> run Block, create Signal with base=Block <=> emit Block)
    """
    from bench.proto import wiring

    accesses: list[Access] = []
    cache: dict[Any, Any] = {}
    # when creating nested nodes in one transaction, the graph only knows about their 'root',
    #  so we remember the scopes for the new nodes to know which zone to use
    new_node_scopes_by_child_id: dict[str, str] = {}
    for edit in edits:
        access_type: AccessType = wiring.unpack_enum(EditType, edit.type)
        node_type: NodeType = wiring.unpack_enum(NodeType, edit.node_type)
        node_cls = NODE_CLASS_BY_TYPE[node_type]
        node = wiring.unwrap_some_node(edit.node)

        # figure out the scope to evaluate what in
        if node_cls.__roots__:
            # regular non-root node: scope = parent if creating, else scope = node :NodeEditScope
            if edit.type in (EditType.CREATE, EditType.UPSERT):
                scope_id = node.parent_ptr.id
                while scope_id in new_node_scopes_by_child_id:
                    scope_id = new_node_scopes_by_child_id[scope_id]
                new_node_scopes_by_child_id[node.id] = scope_id
            else:
                if node.id in new_node_scopes_by_child_id:
                    scope_id = new_node_scopes_by_child_id[node.id]
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
                scope = root = graph.get(node.id)
        try:
            object_properties = node_cls._mask_properties_ids(edit.properties)
            if not object_properties.any():  # if nothing specified, default to all
                object_properties = node_cls.__properties_mask_set__
        except LookupError as e:
            raise ValidationError(edit, "invalid properties") from e

        # and evaluate it
        _, access, was_cached = evaluate_access(
            matrix=matrix,
            verb=access_type,
            object_node_type=node_type,
            object_properties=object_properties,
            root_id=root.id,
            scope_id=scope.id,
            mode=AccessMode.ATOMIC,
            trace=trace,
            cache=cache,
        )
        if not was_cached:
            accesses.append(access)
        if access.decision == PolicyEffect.DENY:
            # implicit or explicit deny for access -> deny entire request
            return Request(decision=PolicyEffect.DENY, subject=matrix.subject, accesses=accesses)

    # at this point no implicit or explicit denies have happened -> explicit allow
    return Request(decision=PolicyEffect.ALLOW, subject=matrix.subject, accesses=accesses)


def evaluate_use(
    matrix: AccessMatrix,
    use_type: UseType,
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
        verb=use_type,
        object_node_type=node.metatype,
        object_properties=node_cls.__properties_mask_set__,
        scope_id=str(node.id),
        root_id=str(node.bench.id),
        mode=AccessMode.ATOMIC,
        trace=trace,
    )
    return Request(decision=access.decision, subject=matrix.subject, accesses=[access])
