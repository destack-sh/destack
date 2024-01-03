import abc
import enum
import random
from dataclasses import dataclass
from typing import Any, Callable, Generic, Optional, TypeVar, Union
from uuid import UUID

import django.db.models
import structlog
from django.core.exceptions import PermissionDenied
from social_django.strategy import DjangoStrategy
from strawberry import relay
from strawberry.extensions import FieldExtension
from strawberry.extensions.field_extension import AsyncExtensionResolver, SyncExtensionResolver
from strawberry.type import StrawberryList, StrawberryOptional
from strawberry.types import Info
from strawberry.union import StrawberryUnion
from strawberry_django import django_resolver
from strawberry_django.fields.types import OperationInfo, OperationMessage

from bench import models
from bench.api.utils import get_param_from_info, get_user_from_info
from bench.models import (
    BenchMembership,
    ModuleAccessLevel,
    Organization,
    OrganizationRole,
    User,
    UserStatus,
)
from bench.models.notification import create_notifications_on_signup
from bench.models.owner import OwnerSlug, slugify
from bench.proto import wire

logger = structlog.get_logger(__name__)

# default access level for new benches
DEFAULT_PROJECT_ACCESS_LEVEL_BY_ORGANIZATION_ROLE: dict[OrganizationRole, ModuleAccessLevel] = {
    OrganizationRole.Owner: ModuleAccessLevel.Admin,
    OrganizationRole.Manager: ModuleAccessLevel.Manage,
    OrganizationRole.Member: ModuleAccessLevel.Edit,
    OrganizationRole.Guest: ModuleAccessLevel.Read,
}


@dataclass
class ModuleAccessInfo:
    user: Optional[models.User]
    bench: models.Bench
    bench_version: Optional[models.BenchVersion]
    level: models.ModuleAccessLevel

    @staticmethod
    def zero(bench: models.Bench) -> "ModuleAccessInfo":
        return ModuleAccessInfo(
            user=None, bench=bench, bench_version=None, level=models.ModuleAccessLevel.Zero
        )


def check_module_access(
    info: Info,
    bench: UUID | models.Bench | models.BenchVersion,
    level: models.ModuleAccessLevel,
) -> ModuleAccessInfo:
    """Raises a PermissionDenied error if the user cannot view the given object."""
    access = has_module_access(info, bench, level)
    if not access:
        raise PermissionDenied("User cannot do this.")
    return access


def check_module_node_access(
    info: Info, module_node: models.Node, level: models.ModuleAccessLevel
) -> ModuleAccessInfo:
    """Raises a PermissionDenied error if the user cannot view the given object."""
    access = has_module_node_access(info, module_node, level)
    if not access:
        raise PermissionDenied("User cannot do this.")
    return access


def has_module_access(
    info: Info,
    bench: UUID | models.Bench | models.BenchVersion,
    level: models.ModuleAccessLevel,
) -> Optional[ModuleAccessInfo]:
    """Get bench-level access info for the given user."""
    bench_version = None
    if isinstance(bench, UUID):
        bench = models.Bench.objects.get(id=bench)
    elif isinstance(bench, models.BenchVersion):
        bench_version = bench
        bench = bench.parent_bench
    else:
        bench = bench
    if not isinstance(bench, models.Bench):
        raise ValueError(f"{type(bench).__name__} {bench} is not a Project")

    granted_accesses = (
        get_user_access(info, bench),
        get_sharing_token_access(info, bench),
        get_default_bench_access(bench),
        ModuleAccessInfo.zero(bench),
    )
    # return the more permissive access level >= level if any
    access = max((a for a in granted_accesses if a is not None), key=lambda a: a.level)
    if access.level < level:
        return None
    access.bench_version = bench_version
    return access


def get_user_access(info: Info, bench: models.Bench) -> Optional[ModuleAccessInfo]:
    user = get_user_from_info(info)
    if not user.is_authenticated:
        return None
    if user.is_staff:
        return ModuleAccessInfo(user, bench, None, models.ModuleAccessLevel.Admin)

    # check if user is owner
    if bench.user_id == user.id:
        return ModuleAccessInfo(user, bench, None, models.ModuleAccessLevel.Admin)

    # check if user is a member of the bench
    bench_membership: BenchMembership = bench.memberships.filter(user_id=user.id).first()
    if bench_membership:
        return ModuleAccessInfo(user, bench, None, bench_membership.level)

    # if bench belongs to an organization, check if user is a member of the organization
    if bench.organization_id:
        org_membership = models.OrganizationMembership.objects.filter(
            organization_id=bench.organization_id, user_id=user.id
        ).first()
        if org_membership:
            level = DEFAULT_PROJECT_ACCESS_LEVEL_BY_ORGANIZATION_ROLE[org_membership.level]
            return ModuleAccessInfo(user, bench, None, level)

    return None


def get_sharing_token_access(info: Info, bench: models.Bench) -> Optional[ModuleAccessInfo]:
    if not bench.sharing_enabled:
        return None
    sharing_token = get_param_from_info(info, "x-sharing-token")
    if sharing_token is None:
        return None
    try:
        sharing_token = UUID(sharing_token)
    except ValueError:
        return None
    if bench.sharing_token != sharing_token:
        return None
    return ModuleAccessInfo(None, bench, None, bench.sharing_level)


def get_default_bench_access(bench: models.Bench) -> Optional[ModuleAccessInfo]:
    if bench.visibility == models.BenchVisibility.PUBLIC:
        return ModuleAccessInfo(None, bench, None, ModuleAccessLevel.Read)
    return None


def has_module_node_access(
    info: Info, node: models.Node | wire.RecordData, level: models.ModuleAccessLevel
) -> Optional[ModuleAccessInfo]:
    """
    Get node-level access info for the given user.
    Right now this is the same as bench-level access.
    """
    if isinstance(node, wire.RecordData):
        node = models.Statement._base_manager.get(id=node.parent_id)
    root = node
    while root.parent:
        root = root.parent
    if not isinstance(root, models.BenchVersion):
        raise ValueError(f"expected bench root for {node}, got {root}")
    return has_module_access(info, root, level)


def is_owner_or_member(
    user: User, owner: Union["User", "Organization"], level: "OrganizationRole" = None
) -> bool:
    from bench.models import OrganizationRole  # avoid circular import

    level = level or OrganizationRole.Member
    return owner.id == user.id or (
        user.id is not None
        and isinstance(owner, Organization)
        and owner.memberships.filter(user_id=user.id, level__gte=level).exists()
    )


def get_organization_role(info: Info, organization: "Organization") -> Optional["OrganizationRole"]:
    user = get_user_from_info(info)
    if not user.is_authenticated:
        return None
    if user.is_staff:
        return OrganizationRole.Owner
    return organization.memberships.filter(user_id=user.id).values_list("level", flat=True).first()


def can_write_organization(info: Info, obj: "Organization") -> bool:
    from bench.models import OrganizationRole  # avoid circular import

    role = get_organization_role(info, obj)
    return role and role >= OrganizationRole.Manager


def check_can_write_organization(info: Info, obj: "Organization") -> None:
    if not can_write_organization(info, obj):
        raise PermissionDenied("User cannot write to this.")


def check_organization_role(info: Info, obj: "Organization", level: "OrganizationRole") -> None:
    role = get_organization_role(info, obj)
    if not role or role < level:
        raise PermissionDenied("User cannot do to this.")


def can_write_user(info: Info, user: User) -> bool:
    self = get_user_from_info(info)
    if self.is_authenticated and self.is_staff:
        return True
    return self.id == user.id


def check_can_write_user(info: Info, user: User) -> None:
    if not can_write_user(info, user):
        raise PermissionDenied("User cannot write to this.")


# :SocialAuth pipeline
# see https://python-social-auth.readthedocs.io/en/latest/configuration/django.html
def social_create_user(strategy: DjangoStrategy, details, backend, user=None, *args, **kwargs):
    if user:
        return {"is_new": False, "user": user}

    username = slugify(details.get("username"))
    # if username is taken, add random suffix
    while OwnerSlug.objects.filter(slug=username).exists():
        username = f"{username}{random.randint(10, 99)}"

    email = details["email"][0] if isinstance(details["email"], (list, tuple)) else details["email"]
    full_name = (
        details.get("fullname")
        or f"{details.get('first_name') or ''} {details.get('last_name') or ''}".strip()
        or details.get("username")
    )
    # incomplete signup, need to set/confirm properties manually (name/username/description, etc.)
    user = User.objects.create_user(username, email, full_name, status=UserStatus.WAITLISTED)
    logger.info("social_create_user", user=user)
    strategy.session_set("backend", backend.name)

    create_notifications_on_signup(user)

    return {"is_new": True, "user": user}


_RetvalT = TypeVar("_RetvalT", bound=django.db.models.Model)


class CheckTarget(enum.StrEnum):
    """Fields to check for permission."""

    ROOT = "root"
    RETVAL = "retval"


class SimplePermissionExtension(FieldExtension, abc.ABC, Generic[_RetvalT]):
    """Check the root or return value of a field for permission."""

    def __init__(
        self,
        check: Callable[[Info, _RetvalT], Optional[bool]],
        target: CheckTarget = CheckTarget.RETVAL,
        fail_silently: bool = True,
        message: str = "User cannot do this.",
    ) -> None:
        super().__init__()
        self.check = check
        self.check_async = django_resolver(check)
        self.target = target
        self.fail_silently = fail_silently
        self.message = message

    @django_resolver(qs_hook=None)
    def resolve(
        self,
        next_: SyncExtensionResolver,
        source: Any,
        info: Info,
        **kwargs: dict[str, Any],
    ) -> Any:
        retval = next_(source, info, **kwargs)
        value = info.root_value if self.target == CheckTarget.ROOT else retval
        if value is None:
            return retval  # nothing to check
        try:
            if self.check(info, value) is False:
                raise PermissionDenied(self.message)
        except PermissionDenied as e:
            retval = self.handle_no_permission(e, info)
        return retval

    async def resolve_async(
        self,
        next_: AsyncExtensionResolver,
        source: Any,
        info: Info,
        **kwargs: dict[str, Any],
    ) -> Any:
        retval = await next_(source, info, **kwargs)
        value = info.root_value if self.target == CheckTarget.ROOT else retval
        if value is None:
            return retval  # nothing to check
        try:
            if await self.check_async(info, value) is False:
                raise PermissionDenied(self.message)
        except PermissionDenied as e:
            retval = await self.handle_no_permission(e, info)
        return retval

    def handle_no_permission(self, exception: BaseException, info: Info):
        # shamelessly borrowed from strawberry-graphql
        # https://github.com/strawberry-graphql/strawberry-graphql-django/blob/7508300d6cf6f3488ea652b6ff784e46ec98fb7b/strawberry_django/permissions.py#L354
        if not self.fail_silently:
            raise PermissionDenied(self.message) from exception

        ret_type = info.return_type

        if isinstance(ret_type, StrawberryOptional):
            ret_type = ret_type.of_type
            is_optional = True
        else:
            is_optional = False

        if isinstance(ret_type, StrawberryUnion):
            ret_types = []
            for type_ in ret_type.types:
                ret_types.append(ret_type)

                if not isinstance(type_, type):
                    continue

                if issubclass(type_, OperationInfo):
                    return type_(
                        messages=[
                            OperationMessage(
                                kind=OperationMessage.Kind.PERMISSION,
                                message=self.message,
                                field=info.field_name,
                            ),
                        ],
                    )

                if issubclass(type_, OperationMessage):
                    return type_(
                        kind=OperationMessage.Kind.PERMISSION,
                        message=self.message,
                        field=info.field_name,
                    )
        else:
            ret_types = [ret_type]

        if is_optional:
            return None

        if isinstance(ret_type, StrawberryList):
            return []

        # If it is a Connection, try to return an empty connection, but only if
        # it is the only possibility available...
        for ret_possibility in ret_types:
            if isinstance(ret_possibility, type) and issubclass(
                ret_possibility,
                relay.Connection,
            ):
                return []

        # In last case, raise an error
        raise PermissionDenied(self.message) from exception


_OtherT = TypeVar("_OtherT", bound=django.db.models.Model)


# noinspection PyPep8Naming
def HasModuleAccess(
    level: ModuleAccessLevel = ModuleAccessLevel.Read,
    target: CheckTarget = CheckTarget.RETVAL,
    map: Optional[Callable[[_OtherT], models.Bench]] = None,
):
    if map is None:
        return SimplePermissionExtension(
            check=lambda info, bench: check_module_access(info, bench, level) is not None,
            target=target,
        )
    else:
        return SimplePermissionExtension(
            check=lambda info, other: check_module_access(info, map(other), level) is not None,
            target=target,
        )


# noinspection PyPep8Naming
def HasOrganizationRole(
    level: OrganizationRole = OrganizationRole.Guest,
    target: CheckTarget = CheckTarget.RETVAL,
    map: Optional[Callable[[_OtherT], models.Organization]] = None,
):
    if map is None:
        return SimplePermissionExtension(
            check=lambda info, org: check_organization_role(info, org, level) is not None,
            target=target,
        )
    else:
        return SimplePermissionExtension(
            check=lambda info, other: check_organization_role(info, map(other), level) is not None,
            target=target,
        )


# noinspection PyPep8Naming
def IsUser(
    target: CheckTarget = CheckTarget.RETVAL, map: Optional[Callable[[_OtherT], models.User]] = None
):
    def _check_is_user(info: Info, user: models.User):
        requesting_user = get_user_from_info(info)
        if map is not None:
            user = map(user)
        return requesting_user.is_staff or requesting_user.id == user.id

    return SimplePermissionExtension(target=target, check=_check_is_user)


# Path: bench/api/record.py
def IsOwner(
    target: CheckTarget = CheckTarget.RETVAL,
    map: Optional[Callable[[_OtherT], models.AccessToken]] = None,
):
    def _check_is_owner(info: Info, owner: models.User | models.Organization):
        requesting_user = get_user_from_info(info)
        if map is not None:
            owner = map(owner)
        return is_owner_or_member(requesting_user, owner, level=OrganizationRole.Member)

    return SimplePermissionExtension(target=target, check=_check_is_owner)
