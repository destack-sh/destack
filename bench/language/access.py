from collections import defaultdict
from datetime import datetime
from itertools import chain
from typing import (
    TYPE_CHECKING,
    Any,
    Collection,
    NamedTuple,
    Optional,
    Self,
    Union,
    cast,
)
from uuid import UUID

import structlog
from bitarray import bitarray
from opentelemetry import trace

from bench.language.bench import Server
from bench.language.const import (
    ACCESS_CLASSES,
    IN_BENCH_NODE_TYPES,
    NODE_TYPES,
    PUBLIC_NODE_TYPES,
    ROOT_NODE_TYPES,
    SUB_PACKAGE_NODE_TYPES,
    AccessKind,
    AccessMode,
    AccessType,
    BenchError,
    EditType,
    NodeType,
    ObjectType,
    PolicyEffect,
    ReadType,
    StructType,
    UseType,
)
from bench.language.graph import NodeDataGraph, NodeGraph, NodeList, NodeSuperGraph
from bench.language.node import (
    NODE_CLASS_BY_TYPE,
    InlineStruct,
    NodeReference,
    SourceNode,
    Struct,
    node_,
    struct_,
)
from bench.language.property import (
    Property,
    p_internal,
    p_node_children,
    p_node_parent,
    p_regular,
    p_runtime,
    p_system,
)
from bench.language.setup import (
    ANCESTOR_NODE_TYPES,
    CHILD_NODE_TYPES,
    _on_completing_setup,
)
from bench.language.text import Text
from bench.language.user import Membership, User
from bench.language.validation import NAME_CONSTRAINT, ValidationError
from bench.proto.wire import AnyNodeData, EditData, NodeReferenceData
from bench.utils.func import IdEnum, bittuple

if TYPE_CHECKING:
    from bench.language import Bench, Block, Client, Organization, Package, ReadOptions

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)

# the node types that can have 'policies' applied to them
#  (not delegated node types, which delegate via subject)
LEGISLATIVE_NODE_TYPES: bittuple[NodeType] = bittuple(
    NodeType.BENCH,
    NodeType.ENVIRONMENT,
    NodeType.BRANCH,
    NodeType.PACKAGE,
    NodeType.SPACE,
    NodeType.BLOCK,
)


@_on_completing_setup
def _check_legislative_types():
    actual_legislative_node_types: bittuple[NodeType] = bittuple(
        *tuple(nt for nt in NODE_TYPES if "policies" in NODE_CLASS_BY_TYPE[nt].__properties__)
    )
    assert actual_legislative_node_types.bits == LEGISLATIVE_NODE_TYPES.bits


# 'owner' refers to both the root node and any user with root-level access to that root node.
# This is effectively the 'root user' who can do anything with descendants of the root node*.
#  (unless a system policy says otherwise)
Ownable = Union["User", "Organization", "Bench"]
OWNABLE_NODE_TYPES: bittuple[NodeType] = bittuple(
    NodeType.USER, NodeType.ORGANIZATION, NodeType.BENCH
)


@node_(NodeType.BADGE)
class Badge(SourceNode):
    """
    Attach a badge to a node with an inline definition.
    A badge's policies are delegated to the 'holder' (any client presenting its secrets).
    The delegated policies apply at the parent scope OR given scopes (which must be below parent's).
    """

    parent: Union["Package", "Block", None] = p_node_parent(4, NodeType.PACKAGE, NodeType.BLOCK)  # type: ignore
    name: str = p_regular(31, constraint=NAME_CONSTRAINT)
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


@node_(NodeType.ROLE)
class Role(SourceNode):
    """
    Attach a role to a block or member.
    Role policies are delegated to the parent and its descendants.
    The delegated policies apply to all descendant's accesses.
    """

    parent: Union["Block", "Membership", None] = p_node_parent(  # type: ignore
        4, NodeType.BLOCK, NodeType.MEMBERSHIP
    )
    type: "Block" = p_regular(30, array=False, require=True, references=NodeType.BLOCK)


@node_(NodeType.IDENTITY)
class Identity(SourceNode):
    """
    Attach an identity to a block, member or user (only the user itself can do that).
    Identity policies are delegated to the parent and its descendants.
    The delegated policies apply to all descendant's accesses.
    """

    parent: Union["Block", "Membership", "User", None] = p_node_parent(  # type: ignore
        4, NodeType.BLOCK, NodeType.MEMBERSHIP, NodeType.USER
    )
    type: "Block" = p_regular(30, array=False, require=True, references=NodeType.BLOCK)

    roles: NodeList["Role"] = p_node_children(NodeType.ROLE)


@struct_(StructType.POLICY)
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

    name: str = p_regular(30, constraint=NAME_CONSTRAINT)
    text: Optional["Text"] = p_regular(31, default=None, struct=StructType.TEXT)
    rules: list["PolicyRule"] = p_regular(32, array=True, struct=StructType.POLICY_RULE)
    scopes: list["Block"] = p_regular(33, require=False, array=True, references=NodeType.BLOCK)

    def __content_str__(self) -> str:
        scopes = self.scopes
        if scopes:
            scopes_str = ", ".join(repr(s) for s in scopes)
            scopes_str = f" at {scopes_str}"
        else:
            scopes_str = "<unscoped>"
        return f"{self.name or '<unnamed>'} ({len(self.rules)} rules, {scopes_str})"

    def append(self, *rules: "PolicyRule") -> "Self":
        self.rules.extend(rules)
        return self


@struct_(StructType.POLICY_RULE)
class PolicyRule(Struct):
    """
    A rule in a policy: [subject] + [effect] [verb] + [object] [if condition].
    Subject, verb and object are ORed, in-group conditions are ANDed.
     (where None/empty -> wildcard, any value -> filter)
    """

    name: str = p_regular(30, constraint=NAME_CONSTRAINT)
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
        subject_str = f"[{'&'.join(subject_str_parts)}]" if subject_str_parts else "*"

        verb_str_parts = []
        if self.verbs:
            verb_str_parts.extend(verb.bench_name for verb in self.verbs)
        if self.verb_kinds:
            verb_str_parts.extend(verb_kind.bench_name for verb_kind in self.verb_kinds)
        verb_str = f"{'|'.join(verb_str_parts)}" if verb_str_parts else "*"

        if self.object_node_types:
            object_type_str_parts = tuple(t.bench_name for t in self.object_node_types)
        else:
            object_type_str_parts = []
        if self.object_properties:
            object_prop_str_parts = [
                f"{cast(ObjectType, p.type).bench_name}.{p.name}" for p in self.object_properties
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

    def _update_component(self, properties: tuple[Property, ...]):
        self._update_masks()

    def _update_masks(self):
        # object mask
        self._object_node_types_mask = _enums_to_mask(self.object_node_types, NodeType)  # type: ignore
        self._object_properties_masks = {}
        all_properties = self.object_properties
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
                self._object_properties_masks[cast(NodeType, prop.type)] = bitarray(
                    prop.component.__max_property_ord__ + 1
                )
            self._object_properties_masks[cast(NodeType, prop.type)][prop.ord] = True

        # verb mask
        self._verb_mask = bitarray(AccessType.get_max_ord())  # type: ignore
        if not self.verbs and not self.verb_kinds:
            self._verb_mask[AccessType.get_min_ord() : AccessType.get_max_ord()] = True  # type: ignore
        else:
            for verb in self.verbs or ():
                assert type(verb) is AccessType, f"unexpected verb {verb!r}"
                self._verb_mask[verb.ord] = True
            for verb_kind in self.verb_kinds or ():
                self._verb_mask[verb_kind.from_ord : verb_kind.to_ord + 1] = True

    def matches_subject(self, subject: "Subject", root_id: UUID) -> bool:
        if self.subject_is_delegated:
            # subject always matches (by definition) if the policy is delegated
            return True
        if (
            self.subject_is_authenticated is not None
            and self.subject_is_authenticated != subject.is_authenticated
        ):
            return False
        if self.subject_is_owner is not None and self.subject_is_owner != (
            subject.owned and any(node.id == root_id for node in subject.owned)
        ):
            return False
        if self.subject_is_staff and not subject.is_staff:  # noqa: SIM103
            return False
        # no mismatch -> match
        return True

    def matches_verb(self, verb: AccessType) -> bool:
        assert self._verb_mask is not None, f"verb mask not ready in {self!r}"
        assert type(verb) is AccessType, f"unexpected verb {verb!r}"
        return cast(bool, self._verb_mask[verb.ord])

    #
    # Builder-style methods
    #

    def allow(self, *verbs: AccessType | AccessKind) -> "Self":
        self.effect = PolicyEffect.ALLOW
        self.verbs = [verb.to(AccessType) for verb in verbs if isinstance(verb, ACCESS_CLASSES)]
        self.verb_kinds = [verb for verb in verbs if isinstance(verb, AccessKind)]
        assert len(self.verbs) + len(self.verb_kinds) == len(verbs), f"invalid verbs {verbs!r}"
        self._update_masks()
        return self

    def deny(self, *verbs: AccessType | AccessKind) -> "Self":
        self.effect = PolicyEffect.DENY
        self.verbs = [verb.to(AccessType) for verb in verbs if isinstance(verb, ACCESS_CLASSES)]
        self.verb_kinds = [verb for verb in verbs if isinstance(verb, AccessKind)]
        assert len(self.verbs) + len(self.verb_kinds) == len(verbs), f"invalid verbs {verbs!r}"
        self._update_masks()
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
        self._update_masks()
        return self


@struct_(StructType.SUBJECT)
class Subject(Struct):
    """
    The <whoever/whatever> issuing a request. Unknown/ignored attributes are unset.
    (We unset various combinations of attributes to evaluate the access of acting subjects independently.)
    """

    # flags
    is_authenticated: Optional[bool] = p_system(30, default=None)
    is_staff: Optional[bool] = p_system(31, default=None)
    is_system: Optional[bool] = p_system(32, default=None)
    # (Client isn't a separate subject but useful to know)

    # who
    client: Optional["Client"] = p_system(
        40, default=None, require=False, array=False, references=NodeType.CLIENT
    )
    user: Optional["User"] = p_system(
        41, default=None, require=False, array=False, references=NodeType.USER
    )
    server: Optional["Server"] = p_system(
        42, default=None, require=False, array=False, references=NodeType.SERVER
    )
    if TYPE_CHECKING:
        client_ptr: Optional[NodeReference] = None
        user_ptr: Optional[NodeReference] = None
        server_ptr: Optional[NodeReference] = None

    # accessories
    identity: Optional["Identity"] = p_system(
        50, default=None, require=False, array=False, references=NodeType.IDENTITY
    )
    badges: list["Badge"] = p_system(51, require=False, array=True, references=NodeType.BADGE)
    owned: list[Ownable] = p_system(
        52, array=True, require=False, references=OWNABLE_NODE_TYPES.tuple
    )
    memberships: list[Union["Bench", "Organization"]] = p_system(
        53, require=False, array=True, references=NodeType.MEMBERSHIP
    )
    roles: list["Role"] = p_system(54, require=False, array=True, references=NodeType.ROLE)

    def split_into_acting_subjects(self, graph: NodeDataGraph) -> tuple["Subject", ...]:
        """
        Split into different subjects that may have different access and are relevant in the given graph.
         (The graph is assumed to contain all relevant owners!).
        Basically, acting subject X in "subject is acting as X" (where X may have different access).
        """

        subjects: list[Subject] = [Subject(is_authenticated=False)]  # anonymous
        if self.is_authenticated:
            subjects.append(Subject(is_authenticated=True))
        if self.is_staff:
            subjects.append(Subject(is_staff=True))
        if self.user:
            subjects.append(Subject(user=self.user, _supergraph=self._supergraph))
        if self.identity:
            subjects.append(Subject(identity=self.identity, _supergraph=self._supergraph))
        if self.badges:
            for badge in self.badges:
                subjects.append(Subject(badges=[badge], _supergraph=self._supergraph))
        for owner in self.owned or ():
            if str(owner.id) in graph:
                subjects.append(Subject(owned=[owner], _supergraph=self._supergraph))
        for membership in self.memberships or ():
            if str(membership.parent_id) in graph:
                subjects.append(Subject(memberships=[membership], _supergraph=self._supergraph))
        for role in self.roles or ():
            role_parent = role.parent
            assert role_parent, f"role {role!r} has no parent"
            if role_parent.metatype == NodeType.BLOCK:
                if str(role.parent_id) in graph:
                    subjects.append(Subject(roles=[role], _supergraph=self._supergraph))
            elif role_parent.metatype == NodeType.MEMBERSHIP:
                if str(role_parent.parent_id) in graph:
                    subjects.append(Subject(roles=[role], _supergraph=self._supergraph))
            else:
                raise BenchError(f"unexpected parent to {role!r}")

        assert len(subjects) > 0, f"no applicable principals in {self!r}"
        return tuple(subjects)

    def __content_str__(self):
        content_parts = []
        if self.is_authenticated:
            content_parts.append("is_authenticated")
        if self.is_staff:
            content_parts.append("is_staff")
        if self.client:
            content_parts.append(f"client={self.client}")
        elif self.user:
            content_parts.append(f"user={self.user}")
        elif self.server:
            content_parts.append(f"server={self.server}")
        if self.badges:
            content_parts.append(f"badges={len(self.badges)}")
        if self.identity:
            content_parts.append(f"identity={self.identity}")
        return ", ".join(content_parts)


@struct_(StructType.ACCESS_ZONE)
class AccessZone(Struct):
    """
    The pre-filtered access rules for a given identity.
    Clients use this to indicate access rights, but - obviously - only our copy is binding.
    """

    scope_id: str = p_system(30)
    _scope: Optional[AnyNodeData] = p_runtime(default=None)
    identity_id: int = p_system(31)
    _identity: Optional[Subject] = p_runtime(default=None)
    rules: list[PolicyRule] = p_system(32, array=True, struct=StructType.POLICY_RULE)

    def __content_str__(self) -> str:
        return f"{(self._identity or self.identity_id)!r} in {self._scope or self.scope_id}: {len(self.rules)} rules"


@struct_(StructType.ACCESS_MATRIX)
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


@struct_(StructType.ACCESS, inline=True)
class Access(InlineStruct):
    """
    An evaluated access on some objects as part of a larger request (by the same subject).
    """

    mode: AccessMode = p_system(30, require=True)
    decision: PolicyEffect = p_system(31, require=True)
    verb: AccessType = p_system(32, require=True)
    node_type: NodeType = p_system(33, require=True)
    allowed_properties: list[Property] = p_system(
        34, require=False, array=True, struct=StructType.PROPERTY_REFERENCE
    )

    # arguments
    # roots, read_options, ...

    def __content_str__(self) -> str:
        if self.allowed_properties:
            object_properties_str = "|".join(p.name for p in self.allowed_properties)
            object_str = f"{self.node_type.bench_name} [{object_properties_str}]"
        else:
            object_str = f"{self.node_type.bench_name} [*]"
        return f"{self.decision.bench_name} {self.verb.bench_name} {object_str}"


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
        access: Access | Collection[Access],
        cause: Exception | None = None,
    ):
        super().__init__(repr(access), cause)
        self.evaluation = access
        self.cause = cause


_SYSTEM_POLICIES: list[Policy] = []


@_on_completing_setup
def _setup_system_policies():
    system_policies = (
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
                text=Text.plain("System properties may only be edited through designated methods."),
            )
            .deny(EditType.UPDATE)
            .object(properties_is_system=True),
            PolicyRule(
                name="CannotCreateOrDeleteSystemNodesDirectly",
                text=Text.plain("System nodes must be managed through designated methods."),
            )
            .deny(
                EditType.CREATE,
                EditType.UPSERT,
                EditType.DELETE,
                EditType.RESTORE,
                EditType.ARCHIVE,
                EditType.UNARCHIVE,
                EditType.ERASE,
            )
            .object(node_types=(*ROOT_NODE_TYPES.tuple, NodeType.CLIENT)),
            PolicyRule(
                name="CannotEditHandles",
                text=Text.plain("Handles (like usernames) must be edited through special methods."),
            )
            .deny(AccessKind.EDIT)
            .object(node_types=(NodeType.HANDLE,)),
            PolicyRule(
                name="CannotUpsertLegislativeNodes",
                # (to prevent ambiguities in evaluation - we could do it, but it would be confusing)
                text=Text.plain("Nodes that define their own policies cannot be upserted."),
            )
            .deny(EditType.UPSERT)
            .object(node_types=LEGISLATIVE_NODE_TYPES.tuple),
        ),
        Policy(name="OwnerAccess").append(
            PolicyRule(
                name="OwnerCanDoAnything",
                text=Text.plain("Owners of a node can do anything (unless otherwise prohibited)."),
            )
            .subject(is_owner=True)
            .allow(),
        ),
        Policy(name="StaffAccess").append(
            PolicyRule(
                name="StaffCanReadAnythingDuringEA",
                text=Text.plain("During early access, staff users can access anything."),
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
                node_types=tuple(
                    nt for nt in IN_BENCH_NODE_TYPES if nt not in SUB_PACKAGE_NODE_TYPES
                ),
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
                text=Text.plain("Everyone can read Handles (so they can create an account)."),
            )
            .subject(is_authenticated=False)
            .allow(AccessKind.READ)
            .object((NodeType.HANDLE,))
        ),
    )
    _SYSTEM_POLICIES.extend(system_policies)


@tracer.start_as_current_span("access.generate_access_matrix")
def generate_access_matrix(
    subject: Subject,
    graph: NodeDataGraph,
    supergraph: NodeSuperGraph | None,
    base_policies: Collection[Policy] | None = None,
    unpacked_graph: NodeGraph | None = None,
) -> AccessMatrix:
    """Generates an access matrix to quickly evaluate access for a specific subject."""

    from bench.proto import wiring

    if base_policies is None:
        base_policies = _SYSTEM_POLICIES

    # TODO :Performance :Architecture: figure out better way of checking access than access matrices
    #  Current approach is a bit unwiedly, hard to update incrementally and not very efficient.

    roots = graph.find_roots()
    identities = subject.split_into_acting_subjects(graph)
    matrix = AccessMatrix(subject=subject, identities=list(identities))
    extra_policies_by_node_id: dict[str, list[Policy]] = defaultdict(list)

    def _assign_access_zones(
        node_data: AnyNodeData, root_id: UUID, parent_zones_by_identity: dict[int, int]
    ):
        """Generates any new applicable access zones downstream from the node for all identities."""

        node_type: NodeType = wiring.unpack_enum(NodeType, node_data.metatype)

        # if this node defines new policies, apply them to their scope
        if getattr(node_data, "policies", None):
            if unpacked_graph:
                node = unpacked_graph.get(UUID(node_data.id))
                new_policies: list[Policy] = getattr(node, "policies")
            else:
                new_policies: list[Policy] = [
                    wiring.unpack_object_validate(p, supergraph=supergraph)
                    for p in getattr(node_data, "policies")
                ]
            for policy in new_policies:
                if policy.scopes:
                    for scope in policy.scopes:
                        # we don't check if scope <= current here, but we don't need to
                        #  (it's validated in Policy and ancestor policies were already considered)
                        extra_policies_by_node_id[str(scope.id)].append(policy)
                else:
                    extra_policies_by_node_id[node_data.id].append(policy)

        # gather all the additional policy rules that apply in this scope (per identity)
        extra_policies = extra_policies_by_node_id.get(node_data.id, ())
        new_zones_by_identity: dict[int, int] | None = None
        if extra_policies:
            for identity in identities:
                applicable_rules = [
                    rule
                    for policy in extra_policies
                    for rule in policy.rules
                    if rule.matches_subject(identity, root_id)
                ]
                if applicable_rules:
                    # new rules, so make new zone
                    zone = AccessZone(
                        parent_id=parent_zones_by_identity.get(identity.id),
                        scope_id=node_data.id,
                        _scope=node_data,
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
            if zone_id is not None:
                zone = matrix._scoped_zones_by_id[zone_id]
                matrix._lowest_zone_by_scope[(identity.id, node_data.id)] = zone

        # descend into children
        #  (even if they don't have any legislative nodes since we want the runtime-only zone mapping)
        for child_type in CHILD_NODE_TYPES[node_type]:
            for child_node in graph.iter_descendants(node_data, child_type):
                _assign_access_zones(child_node, root_id, new_zones_by_identity)

    # start at root
    root_zones_by_identity = {}
    for root in roots:
        # figure out owner
        root_type = wiring.unpack_enum(NodeType, root.metatype)
        root_cls = NODE_CLASS_BY_TYPE[root_type]
        root_id = UUID(root.id)
        assert not root_cls.__roots__, f"unexpected non-root root: {root!r}"

        # base zones are checked before all others (typically for system policies)
        #  but are specific to each root (=owner)
        for identity in identities:
            base_rules = [
                rule
                for policy in base_policies
                for rule in policy.rules
                if rule.matches_subject(identity, root_id)
            ]
            base_zone = AccessZone(
                parent_id=None,
                scope_id=root.id,
                _scope=root,
                identity_id=identity.id,
                _identity=identity,
                rules=base_rules,
            )
            matrix.base_zones.append(base_zone)
            matrix._base_zone_by_root[(identity.id, root.id)] = base_zone

        # add nested zones if there are any legislative nodes down here
        _assign_access_zones(root, root_id, root_zones_by_identity)

    logger.trace("access.generate_access_matrix", subject=subject, span="current")
    return matrix


class _AccessCacheKey(NamedTuple):
    object_node_type: NodeType
    root_id: str
    start_scope_id: int | None
    identity_id: int


def evaluate_access(
    *,
    mode: AccessMode,
    matrix: AccessMatrix,
    verb: AccessType,
    node_type: NodeType,
    wanted_properties: bitarray,
    root_id: str,
    scope_id: str | None,
    cache: dict[_AccessCacheKey, bitarray] | None,
) -> tuple[PolicyEffect, bitarray]:
    """
    Evaluates Access for the given object type and properties in that scope.
    Returns the decision and the allowed properties (all or subset if any).
    NOTE: assumes verb & object_properties are equivalent for every object_node_type in request
    """

    # NOTE :Performance: we can probably cache evaluate_access more aggressively
    #  (e.g. cache the entire result, not just per identity+start zone)

    verb = verb.to(AccessType)
    composite_allowed_properties = bitarray(len(wanted_properties))
    node_cls = NODE_CLASS_BY_TYPE[node_type]
    matched_rules: list[PolicyRule] = []

    # check the zones for each identity (separately)
    for identity in matrix.identities:
        base_zone: AccessZone = matrix._base_zone_by_root[(identity.id, root_id)]
        if scope_id is not None:
            start_scoped_zone = matrix._lowest_zone_by_scope.get((identity.id, scope_id))
        else:
            start_scoped_zone = None

        # check cache
        cache_key = _AccessCacheKey(
            object_node_type=node_type,
            root_id=root_id,
            start_scope_id=start_scoped_zone.id if start_scoped_zone is not None else None,
            identity_id=identity.id,
        )
        allowed_properties = cache.get(cache_key) if cache is not None else None

        if allowed_properties is None:  # not cached
            # first check the base zones, then walk the zones starting from the lowest
            allowed_properties = bitarray(len(wanted_properties))  # for this identity
            unset_properties = wanted_properties  # the unmatched properties so far
            current_zone = base_zone
            while unset_properties.any():
                for rule in current_zone.rules:
                    if (
                        rule.matches_verb(verb)
                        and rule._object_node_types_mask is not None
                        and rule._object_node_types_mask[node_type.ord]
                    ):
                        if trace:
                            matched_rules.append(rule)
                        if rule.effect == PolicyEffect.ALLOW:
                            rule_properties_mask = rule._object_properties_masks.get(
                                node_type, node_cls.__properties_mask_set__
                            )
                            allowed_properties |= rule_properties_mask & unset_properties
                        else:
                            rule_properties_mask = rule._object_properties_masks.get(
                                node_type, node_cls.__properties_mask_unset__
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
                elif current_zone.parent is not None:
                    current_zone = matrix.scoped_zones[cast(int, current_zone.parent.id)]
                else:
                    break  # reached the top
            if cache is not None:
                cache[cache_key] = allowed_properties

        # accumulate (OR) allowed properties across identities
        composite_allowed_properties |= allowed_properties
        if composite_allowed_properties == wanted_properties:
            break  # already fully allowed

    # sum into decision
    if mode == AccessMode.ADAPTIVE:  # adaptive  = if any property was allowed -> access is allowed
        decision = PolicyEffect.ALLOW if composite_allowed_properties.any() else PolicyEffect.DENY
    elif mode == AccessMode.ATOMIC:  # atomic = if any property was rejected -> access is denied
        if composite_allowed_properties == wanted_properties:
            decision = PolicyEffect.ALLOW
        else:
            decision = PolicyEffect.DENY
    else:
        raise ValueError(f"unexpected mode {mode!r}")
    return decision, composite_allowed_properties


def adapt_read_options(
    subject: Subject, root_node_type: NodeType, options: "ReadOptions"
) -> "ReadOptions":
    """
    Adapt read options based on the access to pre-filter as feasible while enabling the complete post-read check.
    Does NOT fully evaluate access yet, but avoids loading data that will be denied anyway.
    """

    options = options.copy()

    # query ancestors up to root
    for ancestor_type in ANCESTOR_NODE_TYPES[root_node_type]:
        if ancestor_type not in options.ancestor_types:
            options.ancestor_types.append(ancestor_type)

    # NOTE :Performance: select only properties required to evaluate edit (id/policies/...?)
    # NOTE :Performance :Security: also pre-filter read options for owner?

    return options


@tracer.start_as_current_span("access.evaluate_and_adapt_read")
def evaluate_and_adapt_read(
    matrix: AccessMatrix,
    graph: NodeDataGraph,
    root_node_type: NodeType,
    options: "ReadOptions",
    *,
    required_nodes: Collection[NodeReferenceData] | None = None,
) -> tuple[PolicyEffect, Collection[Access], Collection[AnyNodeData]]:
    """
    Evaluate *and* adapt access to all nodes in the given graph, pruning nodes & properties as needed.
     -> unlike for other accesses, we don't outright reject GET reads, you just get less (or zero) data.
    Node types not in the original (unadapted) options are pruned (e.g. when querying Records, we adapt
      options to load Record->Block->...->Package->Branch->Bench to evaluate, but we only want Records)

    In case a node was completely denied but its children weren't, we include a Skip node in the result.
    If no overall owner is given, the owners (i.e. actual roots) must be in the graph.

    NOTE: assumes that all policies are valid.
    NOTE: nodes are returned in pre-order (parents before children).
    """
    from bench.proto import wire

    trace.get_current_span().set_attribute("nodes", len(graph))
    requested_node_types = bittuple(root_node_type, *options.all_node_types)
    requested_nodes_preorder: list[AnyNodeData] = []
    visible_nodes: list[AnyNodeData] = []
    allowed_properties_by_node_id: dict[str, bitarray] = {}
    skipped: set[str] = set()
    cache: dict[Any, Any] = {}
    required_nodes_ids: set[str] = {str(n.id) for n in required_nodes or ()}

    # evaluate access per node
    with tracer.start_as_current_span("access.evaluate_read", attributes={"nodes": len(graph)}):
        for root in graph.find_roots():
            descendants = graph.collect_descendants(root, recursive=True)
            for node in chain((root,), descendants):
                # evaluate access
                node_type = NodeType(node.metatype)
                node_properties: bitarray = NODE_CLASS_BY_TYPE[node_type].__properties_mask_set__
                decision, allowed_properties = evaluate_access(
                    matrix=matrix,
                    verb=ReadType.GET,  # same for all?
                    node_type=node_type,
                    wanted_properties=node_properties,
                    root_id=root.id,
                    scope_id=node.id,
                    mode=AccessMode.ADAPTIVE,
                    cache=cache,
                )
                if decision != PolicyEffect.DENY:
                    allowed_properties_by_node_id[node.id] = allowed_properties
                elif node.id in required_nodes_ids:
                    node_cls = NODE_CLASS_BY_TYPE[node_type]
                    access = Access(
                        mode=AccessMode.ADAPTIVE,
                        decision=decision,
                        verb=ReadType.GET,
                        node_type=node_type,
                        allowed_properties=list(node_cls._unmask_properties(allowed_properties)),
                    )
                    return decision, (access,), ()
                if node_type in requested_node_types:
                    requested_nodes_preorder.append(node)

    # adapt & filter nodes
    with tracer.start_as_current_span("access.adapt_read", attributes={"nodes": len(graph)}):
        for node in requested_nodes_preorder:
            node_cls = NODE_CLASS_BY_TYPE[cast(NodeType, node.metatype)]
            allowed_properties = allowed_properties_by_node_id.get(node.id)
            node_type = cast(NodeType, node.metatype)
            node_properties: bitarray = NODE_CLASS_BY_TYPE[node_type].__properties_mask_set__
            if allowed_properties is None:
                # skip (will be added in as skip if needed below)
                skipped.add(node.id)
            elif allowed_properties == node_properties:
                # add as is (with all properties)
                visible_nodes.append(node)
            else:
                # add, but prune node properties to only allowed ones
                node_copy = type(node)(metatype=node.metatype)
                for prop_ord in allowed_properties.search(True):
                    prop = node_cls.__properties_in_order__[prop_ord]
                    if prop.reference_wired_ptr is not None:
                        prop = prop.reference_wired_ptr
                    setattr(node_copy, prop.name, getattr(node, prop.name))
                visible_nodes.append(node_copy)

    # add any required skipped nodes back in (as Skips)
    skips: dict[str, wire.SkipData] = {}
    for node in visible_nodes:
        if node.parent_ptr is not None and node.parent_ptr.id in skipped:
            if node.parent_ptr.id in skips:
                continue
            skip = wire.SkipData(
                metatype=wire.ObjectType.SKIP,
                id=node.id,
                parent_ptr=node.parent_ptr,
                revision=node.revision,
                order_key=getattr(node, "order_key", None),
                reference_ptr=NodeReference.from_node_data(node),
            )
            skips[cast(str, node.parent_ptr.id)] = skip
    visible_nodes.extend(skips.values())

    # if we got here none of the required nodes were denied (above)
    return PolicyEffect.ALLOW, (), visible_nodes


@tracer.start_as_current_span("access.evaluate_edit")
def evaluate_edit(
    matrix: AccessMatrix,
    graph: NodeDataGraph,
    edits: Collection[EditData],
) -> tuple[PolicyEffect, Collection[Access]]:
    """
    Evaluates whether the given policies (base and in graph) allow the given edits.
    Assumes that all policies are valid, and that all relevant scopes are in the graph.
    TODO :Broken :Security: verify equivalent 'use' access for the edits
     (e.g. create Run with base=Block <=> run Block, create Signal with base=Block <=> emit Block)
    """
    from bench.language.transaction import unpack_node_delta
    from bench.proto import wiring

    # when creating nested nodes in one transaction, the graph only knows about their 'root',
    #  so we remember the scopes for the new nodes to know which zone to use
    new_node_scopes_by_child_id: dict[str, str] = {}
    for edit in edits:
        access_type: AccessType = wiring.unpack_enum(EditType, edit.type)
        node_type: NodeType = wiring.unpack_enum(NodeType, edit.node_ptr.type)
        node_cls = NODE_CLASS_BY_TYPE[node_type]
        node_id = edit.node_ptr.id
        assert node_id is not None, f"no node id for {edit!r}"

        # figure out the scope to evaluate what in
        if node_cls.__roots__:
            # regular non-root node: scope = parent if creating, else scope = node :NodeEditScope
            if edit.type in (EditType.CREATE, EditType.UPSERT):
                assert edit.new_node_packed, f"no new node for {edit!r}"
                node_data = unpack_node_delta(
                    edit.new_node_packed, node_type=node_type, only=(node_cls.__parent_property__,)
                )
                assert node_data.parent_ptr, f"no parent for {edit!r}"
                scope_id = cast(str, node_data.parent_ptr.id)
                while scope_id in new_node_scopes_by_child_id:
                    scope_id = new_node_scopes_by_child_id[scope_id]
                new_node_scopes_by_child_id[node_id] = scope_id
            else:
                scope_id = new_node_scopes_by_child_id.get(node_id, node_id)
            scope = graph.get(scope_id)
            assert scope is not None, f"scope {scope_id} for {edit!r} not in {graph!r}"
            root = graph.get_root(scope)
        else:
            if edit.type in (EditType.CREATE, EditType.UPSERT):
                # there's a system rule against creating roots, but would need special logic to enforce it
                #  (because root would be a node itself, which isn't in the matrix as we expect)
                return PolicyEffect.DENY, ()
            scope = root = graph.get(node_id)
        try:
            object_properties = node_cls._mask_properties_ids(edit.properties)
            if not object_properties.any():  # if nothing specified, default to all
                object_properties = node_cls.__properties_mask_set__
        except LookupError as e:
            raise ValidationError(edit, "invalid properties") from e

        if scope is None or root is None:
            raise ValidationError(edit, f"scope {node_id} not in {graph!r}")
        # and evaluate it
        decision, allowed_properties = evaluate_access(
            matrix=matrix,
            verb=access_type,
            node_type=node_type,
            wanted_properties=object_properties,
            root_id=root.id,
            scope_id=scope.id,
            mode=AccessMode.ATOMIC,
            cache=None,
        )
        if decision == PolicyEffect.DENY:
            # implicit or explicit deny for access -> deny entire request
            access = Access(
                mode=AccessMode.ATOMIC,
                decision=PolicyEffect.DENY,
                verb=access_type,
                node_type=node_type,
                allowed_properties=list(node_cls._unmask_properties(allowed_properties)),
            )
            return PolicyEffect.DENY, (access,)

    # at this point no implicit or explicit denies have happened -> explicit allow
    return PolicyEffect.ALLOW, ()


@tracer.start_as_current_span("access.evaluate_use")
def evaluate_use(
    matrix: AccessMatrix,
    use_type: UseType,
    node: "Block",
) -> tuple[PolicyEffect, Collection[Access]]:
    """
    Evaluates whether the given policies (base and in graph) allow the given run access.
    Assumes that all policies are valid.
    """
    node_cls = NODE_CLASS_BY_TYPE[node.metatype]
    decision, allowed_properties = evaluate_access(
        matrix=matrix,
        verb=use_type,
        node_type=node.metatype,
        wanted_properties=node_cls.__properties_mask_set__,
        scope_id=str(node.id),
        root_id=str(node.bench.id),
        mode=AccessMode.ATOMIC,
        cache=None,
    )
    access = Access(
        mode=AccessMode.ATOMIC,
        decision=decision,
        verb=use_type,
        node_type=node.metatype,
        allowed_properties=list(node_cls._unmask_properties(allowed_properties)),
    )
    return decision, [access]
