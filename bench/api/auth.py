import random
from dataclasses import dataclass
from typing import Optional, Union
from uuid import UUID

import structlog
from django.core.exceptions import PermissionDenied
from social_django.strategy import DjangoStrategy
from strawberry.types import Info

from bench import models
from bench.api.utils import get_header_from_scope, get_scope_from_info, get_user_from_info
from bench.models import (
    Organization,
    OrganizationRole,
    ProjectAccessLevel,
    ProjectMembership,
    User,
    UserStatus,
)
from bench.models.notification import create_notifications_on_signup
from bench.models.owner import OwnerSlug, slugify

logger = structlog.get_logger(__name__)

# default access level for new projects
DEFAULT_PROJECT_ACCESS_LEVEL_BY_ORGANIZATION_ROLE: dict[OrganizationRole, ProjectAccessLevel] = {
    OrganizationRole.Owner: ProjectAccessLevel.Admin,
    OrganizationRole.Manager: ProjectAccessLevel.Manage,
    OrganizationRole.Member: ProjectAccessLevel.Edit,
    OrganizationRole.Guest: ProjectAccessLevel.Read,
}


@dataclass
class ProjectAccessInfo:
    user: Optional[models.User]
    project: models.Project
    project_version: Optional[models.ProjectVersion]
    level: models.ProjectAccessLevel

    @staticmethod
    def zero(project: models.Project) -> "ProjectAccessInfo":
        return ProjectAccessInfo(
            user=None, project=project, project_version=None, level=models.ProjectAccessLevel.Zero
        )


def check_project_access(
    info: Info,
    project: UUID | models.Project | models.ProjectVersion,
    level: models.ProjectAccessLevel,
) -> ProjectAccessInfo:
    """Raises a PermissionDenied error if the user cannot view the given object."""
    access = has_project_access(info, project, level)
    if not access:
        raise PermissionDenied("User cannot do this.")
    return access


def check_module_node_access(
    info: Info, module_node: models.ModuleNode, level: models.ProjectAccessLevel
) -> ProjectAccessInfo:
    """Raises a PermissionDenied error if the user cannot view the given object."""
    access = has_module_node_access(info, module_node, level)
    if not access:
        raise PermissionDenied("User cannot do this.")
    return access


def has_project_access(
    info: Info,
    project: UUID | models.Project | models.ProjectVersion,
    level: models.ProjectAccessLevel,
) -> Optional[ProjectAccessInfo]:
    """Get project-level access info for the given user."""
    project_version = None
    if isinstance(project, UUID):
        project = models.Project.objects.get(id=project)
    elif isinstance(project, models.ProjectVersion):
        project = project.project
        project_version = project
    else:
        project = project

    user_access = get_user_access(info, project) or ProjectAccessInfo.zero(project)
    sharing_token_access = get_sharing_token_access(info, project) or ProjectAccessInfo.zero(
        project
    )

    # return the more permissive access level >= level if any
    access = max(user_access, sharing_token_access, key=lambda a: a.level)
    if access.level < level:
        return None
    access.project_version = project_version
    return access


def get_user_access(info: Info, project: models.Project) -> Optional[ProjectAccessInfo]:
    user = get_user_from_info(info)
    if not user.is_authenticated:
        return None
    if user.is_staff:
        return ProjectAccessInfo(user, project, None, models.ProjectAccessLevel.Admin)

    # check if user is owner
    if project.user_id == user.id:
        return ProjectAccessInfo(user, project, None, models.ProjectAccessLevel.Admin)

    # check if user is a member of the project
    project_membership: ProjectMembership = project.memberships.filter(user_id=user.id).first()
    if project_membership:
        return ProjectAccessInfo(user, project, None, project_membership.level)

    # if project belongs to an organization, check if user is a member of the organization
    if project.organization_id:
        org_membership = models.OrganizationMembership.objects.filter(
            organization_id=project.organization_id, user_id=user.id
        ).first()
        if org_membership:
            level = DEFAULT_PROJECT_ACCESS_LEVEL_BY_ORGANIZATION_ROLE[org_membership.level]
            return ProjectAccessInfo(user, project, None, level)

    return None


def get_sharing_token_access(info: Info, project: models.Project) -> Optional[ProjectAccessInfo]:
    scope = get_scope_from_info(info)
    sharing_token = get_header_from_scope(scope, "x-sharing-token")
    if not sharing_token:
        if project.visibility != models.ProjectVisibility.PUBLIC:
            return None
        return ProjectAccessInfo(None, project, None, models.ProjectAccessLevel.Read)
    sharing_token = UUID(sharing_token)
    if project.sharing_token != sharing_token:
        return None
    return ProjectAccessInfo(None, project, None, project.sharing_level)


def has_module_node_access(
    info: Info, node: models.ModuleNode, level: models.ProjectAccessLevel
) -> Optional[ProjectAccessInfo]:
    """
    Get node-level access info for the given user.
    Right now this is the same as project-level access.
    """
    # normalize to project
    root = node
    while root.parent:
        root = root.parent
    if isinstance(root, models.ProjectVersion):
        root = root.project
    if not isinstance(root, models.Project):
        raise ValueError(f"expected project root for {node}, got {root}")
    return has_project_access(info, root, level)


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


def can_write_organization(info: Info, obj: "Organization") -> bool:
    from bench.models import OrganizationRole  # avoid circular import

    user = get_user_from_info(info)
    if user.is_authenticated and user.is_staff:
        return True
    return obj.memberships.filter(user_id=user.id, level__gte=OrganizationRole.Manager).exists()


def check_can_write_organization(info: Info, obj: "Organization") -> None:
    if not can_write_organization(info, obj):
        raise PermissionDenied("User cannot write to this.")


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
