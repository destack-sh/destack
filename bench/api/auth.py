import abc
import dataclasses
import functools
from typing import Any, Callable, Iterable, Self, Union, cast

import strawberry
import structlog
from django.core.exceptions import PermissionDenied
from django.db.models import Model, Q, QuerySet
from graphql import GraphQLResolveInfo
from social_django.strategy import DjangoStrategy
from strawberry import Private
from strawberry.channels import StrawberryChannelsContext
from strawberry.schema_directive import Location
from strawberry.types import Info
from strawberry_django_plus import field, permissions
from strawberry_django_plus.directives import SchemaDirectiveHelper
from strawberry_django_plus.permissions import (
    AuthDirective,
    _user_ensured_attr,
    get_user_or_anonymous,
    init_checker,
    is_perm_safe,
    running_checks,
    set_perm_safe,
)
from strawberry_django_plus.relay import Connection
from strawberry_django_plus.types import OperationInfo
from strawberry_django_plus.utils import aio, resolvers

from bench import models
from bench.models import Organization, User

logger = structlog.get_logger(__name__)


@dataclasses.dataclass
class HasCustomPermDirective(AuthDirective, abc.ABC):
    at_root: bool

    def resolve(
        self,
        helper: SchemaDirectiveHelper,
        _next: Callable,
        root: Any,
        info: GraphQLResolveInfo,
        *args,
        **kwargs,
    ):
        # exactly like AuthDirective.resolve, but get user from channels context
        context = cast(StrawberryChannelsContext, info.context)
        resolver = functools.partial(_next, root, info, *args, **kwargs)

        user = cast(User, context.request.scope["user"]._wrapped)
        if not getattr(context, _user_ensured_attr, False):
            return aio.resolve(
                cast(User, get_user_or_anonymous(user)),
                functools.partial(
                    self.resolve_for_user,
                    helper,
                    resolver,
                    root,
                    info,
                    **kwargs,
                ),
                info=info,
            )

        return self.resolve_for_user(
            helper,
            resolver,
            root,
            info,
            cast(User, user),
            **kwargs,
        )

    def get_cache(
        self,
        info: GraphQLResolveInfo,
        user: User,
    ) -> dict[Union[Self, tuple[Self, Any]], bool]:
        cache_key = f"_{self.__class__.__name__}_cache"

        cache = getattr(user, cache_key, None)
        if cache is not None:
            return cache

        cache = {}
        setattr(user, cache_key, cache)
        return cache

    def resolve_for_user(
        self,
        helper: SchemaDirectiveHelper,
        resolver: Callable,
        root: Any,
        info: GraphQLResolveInfo,
        user: User,
        **kwargs,
    ):
        cache = self.get_cache(info, user)
        if self.at_root and root is not None:
            has_perm = cache.get(self)
            if has_perm is None:
                has_perm = self.has_perm_safe(root, info, user, root)
                cache[self] = has_perm
            return self.resolve_retval(helper, root, info, resolver, has_perm)
        else:  # retval
            init_checker(self)  # type:ignore

            ret = resolver()
            if not ret or isinstance(ret, OperationInfo):
                return ret  # just return that

            # Avoid is_awaitable as much as we can
            if not isinstance(ret, (list, Model, QuerySet)) and aio.is_awaitable(ret, info=info):
                return aio.resolve_async(
                    ret,
                    functools.partial(self.resolve_has_perm, helper, root, info, user),
                )
            return self.resolve_has_perm(helper, root, info, user, ret)

    def resolve_has_perm(
        self,
        helper: SchemaDirectiveHelper,
        root: Any,
        info: GraphQLResolveInfo,
        user: User,
        obj: Any,
    ) -> Any:
        if is_perm_safe():
            return self.resolve_retval(helper, root, info, obj, True)

        if not obj or isinstance(obj, OperationInfo):
            return obj  # just return that

        if isinstance(obj, Iterable):
            # not needed so far, see _resolve_iterable_perms_safe once necessary
            raise NotImplementedError
        elif isinstance(obj, Connection):
            obj_key = f"{info.field_name}_{obj.page_info.start_cursor}_{obj.page_info.end_cursor}"
        else:
            obj_key = obj  # models are hashable

        cache = self.get_cache(info, user)
        has_perm = cache.get(obj_key)
        if has_perm is None:
            has_perm = self.has_perm_safe(root, info, user, obj)
            cache[obj_key] = has_perm
        return self.resolve_retval(helper, root, info, obj, has_perm)

    def has_perm_safe(self, root: Any, info: GraphQLResolveInfo, user: User, obj: Any) -> bool:
        """Checks whether the user has the permission for the given object. Async safe."""
        raise NotImplementedError

    def filter_for_user(self, qs: QuerySet, info: Info, user: User) -> QuerySet:
        """Filters the queryset for access by the given user."""
        raise NotImplementedError


# patch strawberry_django_plus.permissions.filter_with_perms
# to use our custom auth directives (filter_with_perms is hardcoded into strawberry_django_plus)
def filter_with_perms(qs: QuerySet, info: Info) -> QuerySet:
    checks = running_checks.get()
    if not checks:
        return qs

    # do not do anything is results are cached, the target is the retval
    if qs._result_cache is not None:  # type:ignore
        set_perm_safe(False)
        return qs

    user = cast(User, info.context.request.scope["user"]._wrapped)
    for check in checks:
        if not isinstance(check, HasCustomPermDirective):
            raise ValueError("filter_with_perms only supports HasCustomPermDirective")
        qs = check.filter_for_user(qs, info, user)

    set_perm_safe(True)
    return qs


# TODO @Cleanup: no monkey-patching strawberry_django_plus filter_with_perms
filter_with_perms.__patched__ = True
permissions.filter_with_perms = filter_with_perms
# also patch in import sites
field.filter_with_perms = filter_with_perms
resolvers.filter_with_perms = filter_with_perms


def can_view_project(user: User, obj: Any) -> bool:
    """Whether the given user can view the project-related object."""
    if user.is_authenticated and user.is_staff:
        return True
    # TODO @Performance: prefetch project or check condition entirely in SQL
    # normalize to project instance
    if isinstance(obj, (models.File, models.Statement, models.Execution)):
        obj = obj.project_version.project
    elif isinstance(obj, models.ProjectVersion):
        obj = obj.project
    elif isinstance(obj, Connection):
        return all(can_view_project(user, edge.node) for edge in obj.edges)
    elif isinstance(obj, Iterable):
        return all(can_view_project(user, item) for item in obj)
    if not isinstance(obj, models.Project):
        raise ValueError(f"CanViewProject cannot be used on {obj}")
    # is public or user is owner or user is member of owning organization
    is_org_member = (
        obj.organization_id is not None and obj.organization.members.filter(id=user.id).exists()
    )
    is_owner = obj.user_id is not None and obj.user_id == user.id
    return obj.visibility == models.ProjectVisibility.PUBLIC or is_owner or is_org_member


def can_write_project(user: User, obj: Any) -> bool:
    """Whether the given user can write to the project-related object."""
    if user.is_authenticated and user.is_staff:
        return True
    if user.is_anonymous:
        return False
    # TODO @Performance: prefetch project or check condition entirely in SQL
    # normalize to project instance
    if isinstance(obj, (models.File, models.Statement, models.Execution)):
        obj = obj.project_version.project
    elif isinstance(obj, models.ProjectVersion):
        obj = obj.project
    if not isinstance(obj, models.Project):
        raise ValueError(f"CanWriteProject cannot be used on {obj}")
    is_org_member = (
        obj.organization_id is not None and obj.organization.members.filter(id=user.id).exists()
    )
    is_owner = obj.user_id is not None and obj.user_id == user.id
    return is_owner or is_org_member


def check_can_view_project(info: Info, obj: Any) -> None:
    """Raises a PermissionDenied error if the user cannot view the given object."""
    user = cast(User, info.context.request.scope["user"]._wrapped)
    if not can_view_project(user, obj):
        raise PermissionDenied("User cannot view this.")


def check_can_write_project(info: Info, obj: Any) -> None:
    """Raises a PermissionDenied error if the user cannot write to the given object."""
    user = cast(User, info.context.request.scope["user"]._wrapped)
    if not can_write_project(user, obj):
        raise PermissionDenied("User cannot write to this.")


@strawberry.schema_directive(
    locations=[Location.FIELD_DEFINITION],
    description="Can only be resolved by user with project view permission",
)
class CanViewProject(HasCustomPermDirective):
    at_root: bool = dataclasses.field(default=True)
    message: Private[str] = dataclasses.field(default="User cannot view this.")

    @resolvers.async_safe
    def has_perm_safe(self, root: Any, info: GraphQLResolveInfo, user: User, obj: Any) -> bool:
        return can_view_project(user, obj)

    def filter_for_user(self, qs: QuerySet, info: Info, user: User) -> QuerySet:
        if user.is_authenticated and user.is_staff:
            return qs
        if not user.is_authenticated:
            filters = (Q(visibility=models.ProjectVisibility.PUBLIC),)
        else:
            # is public or user is owner or user is member of owning organization
            filters = (
                Q(visibility=models.ProjectVisibility.PUBLIC),
                Q(user=user),
                Q(organization__members=user),
            )
        # normalize filters to project instance (prefix with paths)
        if issubclass(qs.model, (models.File, models.Statement, models.Execution)):
            prefix = "project_version__project__"
        elif issubclass(qs.model, models.ProjectVersion):
            prefix = "project__"
        else:
            prefix = ""
        if prefix:
            filters = [Q(**{prefix + f"{k}": v}) for f in filters for k, v in f.children]
        combined_filter = Q(*filters, _connector=Q.OR)
        qs = qs.filter(combined_filter)
        # distinct because of organization joins
        # there's probably better way to do this
        return qs.distinct()


@strawberry.schema_directive(
    locations=[Location.FIELD_DEFINITION],
    description="Can only be resolved by user with project write permission",
)
class CanWriteProject(CanViewProject):
    at_root: bool = dataclasses.field(default=True)
    message: Private[str] = dataclasses.field(default="User cannot write to this.")

    @resolvers.async_safe
    def has_perm_safe(self, root: Any, info: GraphQLResolveInfo, user: User, obj: Any) -> bool:
        return can_write_project(user, obj)


def can_write_organization(user: User, obj: "Organization") -> bool:
    if user.is_authenticated and user.is_staff:
        return True
    return obj.members.filter(id=user.id).exists()


def check_can_write_organization(info: Info, obj: "Organization") -> None:
    user = cast(User, info.context.request.scope["user"]._wrapped)
    if not can_write_organization(user, obj):
        raise PermissionDenied("User cannot write to this.")


def can_write_user(user: User, obj: User) -> bool:
    if user.is_authenticated and user.is_staff:
        return True
    return user.id == obj.id


def check_can_write_user(info: Info, obj: User) -> None:
    user = cast(User, info.context.request.scope["user"]._wrapped)
    if not can_write_user(user, obj):
        raise PermissionDenied("User cannot write to this.")


@strawberry.schema_directive(
    locations=[Location.FIELD_DEFINITION],
    description="Can only be resolved by user with organization write permission",
)
class CanWriteOrganization(HasCustomPermDirective):
    at_root: bool = dataclasses.field(default=True)
    message: Private[str] = dataclasses.field(default="User cannot write to this.")

    @resolvers.async_safe
    def has_perm_safe(self, root: Any, info: GraphQLResolveInfo, user: User, obj: Any) -> bool:
        return can_write_organization(user, obj)

    def filter_for_user(self, qs: QuerySet, info: Info, user: User) -> QuerySet:
        if not user.is_authenticated:
            return qs.none()
        return qs.filter(organization__members=user)


@strawberry.schema_directive(
    locations=[Location.FIELD_DEFINITION],
    description="Can only be resolved by user with user write permission",
)
class CanWriteUser(HasCustomPermDirective):
    at_root: bool = dataclasses.field(default=True)
    message: Private[str] = dataclasses.field(default="User cannot write to this.")

    @resolvers.async_safe
    def has_perm_safe(self, root: Any, info: GraphQLResolveInfo, user: User, obj: Any) -> bool:
        return can_write_user(user, obj)

    def filter_for_user(self, qs: QuerySet, info: Info, user: User) -> QuerySet:
        if not user.is_authenticated:
            return qs.none()
        return qs.filter(user__id=user.id)


def social_create_user(strategy: DjangoStrategy, details, backend, user=None, *args, **kwargs):
    if user:
        return {"is_new": False}

    username = details.get("username")
    email = details["email"][0] if isinstance(details["email"], (list, tuple)) else details["email"]
    full_name = (
        details.get("fullname")
        or f"{details.get('first_name') or ''} {details.get('last_name') or ''}".strip()
        or details.get("username")
    )
    # incomplete signup, need to set more properties (like username)
    user = User.objects.create_user(username, email, full_name, completed_signup=False)
    strategy.session_set("backend", backend.name)

    logger.info("social_create_user", user=user)
    return {"is_new": True, "user": user}


def is_owner_or_member(user: User, owner: Union["User", "Organization"]) -> bool:
    return owner.id == user.id or (
        user.id is not None
        and isinstance(owner, Organization)
        and owner.members.filter(id=user.id).exists()
    )
