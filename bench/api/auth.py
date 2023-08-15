import random
from typing import TYPE_CHECKING, Any, Iterable, Union
from uuid import UUID

import structlog
from django.core.exceptions import PermissionDenied
from social_django.strategy import DjangoStrategy
from strawberry import relay
from strawberry.types import Info

from bench import models
from bench.api.utils import get_user_from_info
from bench.models import Notification, Organization, User, UserStatus
from bench.models.notification import create_notifications_on_signup
from bench.models.owner import OwnerSlug, slugify

if TYPE_CHECKING:
    from bench.models.organization import OrganizationMembershipLevel

logger = structlog.get_logger(__name__)


def normalize_to_project(obj) -> models.Project:
    """Normalizes the given object to a project."""
    # TODO @Performance: prefetch project or check condition entirely in SQL
    if isinstance(obj, relay.Connection):
        return normalize_to_project(obj.edges[0].node)
    elif isinstance(obj, Iterable):
        return normalize_to_project(obj[0])
    elif isinstance(obj, (models.Field, models.Tagging)):
        obj = obj.statement.project_version.project
    elif isinstance(obj, (models.File, models.Statement, models.Run)):
        obj = obj.project_version.project
    elif isinstance(
        obj,
        (models.ProjectVersion, models.RemoteObject, models.Secret, models.Session),
    ):
        obj = obj.project
    elif not isinstance(obj, models.Project):
        raise ValueError(f"CanViewProject cannot be used on {obj}")
    return obj


def can_read_project(user: User, obj: Any) -> bool:
    """Whether the given user can view the project-related object."""
    if user.is_authenticated and user.is_staff:
        return True
    obj = normalize_to_project(obj)
    # is public or user is owner or user is member (any level) of owning organization
    is_org_member = (
        obj.organization_id is not None
        and obj.organization.memberships.filter(user_id=user.id).exists()
    )
    is_owner = obj.user_id is not None and obj.user_id == user.id
    return obj.visibility == models.ProjectVisibility.PUBLIC or is_owner or is_org_member


def can_write_project(user: User, obj: Any) -> bool:
    """Whether the given user can write to the project-related object."""
    if user.is_authenticated and user.is_staff:
        return True
    if user.is_anonymous:
        return False
    obj = normalize_to_project(obj)
    from bench.models import OrganizationMembershipLevel  # avoid circular import

    is_org_member = (
        obj.organization_id is not None
        and obj.organization.memberships.filter(
            user_id=user.id, level__gte=OrganizationMembershipLevel.Member
        ).exists()
    )
    is_owner = obj.user_id is not None and obj.user_id == user.id
    return is_owner or is_org_member


def check_can_read_project(info: Info, obj: Any) -> None:
    """Raises a PermissionDenied error if the user cannot view the given object."""
    user = get_user_from_info(info)
    if not can_read_project(user, obj):
        raise PermissionDenied("User cannot view this.")


def check_can_view_project_by_id(
    user: models.User, project_version_id: UUID = None, project_id: UUID = None
):
    if not project_version_id and not project_id:
        raise ValueError("must set project_version_id or project_id")

    if project_version_id is None:
        project = models.Project.objects.prefetch_related("user", "organization").get(id=project_id)
    else:
        project_version = (
            models.ProjectVersion.objects.all()
            .prefetch_related("project", "project__user", "project__organization")
            .get(id=project_version_id)
        )
        project = project_version.project
        if project_id is not None and project_id != project.id:
            raise ValueError("project_id must match project_version_id")
    if not can_read_project(user, project):
        raise PermissionDenied("You don't have permission to view this project.")


def check_can_write_project(info: Info, obj: Any) -> None:
    """Raises a PermissionDenied error if the user cannot write to the given object."""
    user = get_user_from_info(info)
    if not can_write_project(user, obj):
        raise PermissionDenied("User cannot write to this.")


def check_can_write_project_by_id(user: models.User, project_version_id: UUID):
    project_version = (
        models.ProjectVersion.objects.all()
        .prefetch_related("project", "project__user", "project__organization")
        .get(id=project_version_id)
    )
    if not can_write_project(user, project_version.project):
        raise PermissionDenied("You don't have permission to write to this project.")


def is_owner_or_member(
    user: User, owner: Union["User", "Organization"], level: "OrganizationMembershipLevel" = None
) -> bool:
    from bench.models import OrganizationMembershipLevel  # avoid circular import

    return owner.id == user.id or (
        user.id is not None
        and isinstance(owner, Organization)
        and owner.memberships.filter(
            user_id=user.id, level__gte=level or OrganizationMembershipLevel.Member
        ).exists()
    )


def can_view_full_organization(user: User, obj: "Organization") -> bool:
    if user.is_authenticated and user.is_staff:
        return True
    return obj.members.filter(id=user.id).exists()


def check_can_view_full_organization(info: Info, obj: "Organization") -> None:
    user = get_user_from_info(info)
    if not can_view_full_organization(user, obj):
        raise PermissionDenied("User cannot view this.")


def can_write_organization(user: User, obj: "Organization") -> bool:
    from bench.models import OrganizationMembershipLevel  # avoid circular import

    if user.is_authenticated and user.is_staff:
        return True
    return obj.memberships.filter(
        user_id=user.id, level__gte=OrganizationMembershipLevel.Administrator
    ).exists()


def check_can_write_organization(info: Info, obj: "Organization") -> None:
    user = get_user_from_info(info)
    if not can_write_organization(user, obj):
        raise PermissionDenied("User cannot write to this.")


def can_write_user(user: User, obj: Any) -> bool:
    if user.is_authenticated and user.is_staff:
        return True
    if isinstance(obj, Notification):
        return user.id == obj.user_id
    elif isinstance(obj, User):
        return user.id == obj.id
    else:
        raise ValueError(f"can_write_user cannot be used on {obj}")


def check_can_write_user(info: Info, obj: User) -> None:
    user = get_user_from_info(info)
    if not can_write_user(user, obj):
        raise PermissionDenied("User cannot write to this.")


def social_create_user(strategy: DjangoStrategy, details, backend, user=None, *args, **kwargs):
    if user:
        return {"is_new": False}

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
