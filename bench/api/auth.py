import abc
import dataclasses
import functools
from typing import Any, Callable, Iterable, Self, Union, cast

import strawberry
import structlog
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
from strawberry_django_plus.utils import aio, resolvers

from bench import models
from bench.models import User

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
            if not ret:
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
    ) -> User:
        if is_perm_safe():
            return self.resolve_retval(helper, root, info, obj, True)

        if isinstance(obj, Iterable):
            # not needed so far, see _resolve_iterable_perms_safe once necessary
            raise NotImplementedError

        cache = self.get_cache(info, user)
        has_perm = cache.get(obj)
        if has_perm is None:
            has_perm = self.has_perm_safe(root, info, user, obj)
            cache[obj] = has_perm
        return self.resolve_retval(helper, root, info, obj, has_perm)

    def has_perm_safe(self, root: Any, info: GraphQLResolveInfo, user: User, obj: Any) -> bool:
        """Checks whether the user has the permission for the given object. Async safe."""
        raise NotImplementedError

    def filter_for_user(self, qs: QuerySet, user: User) -> QuerySet:
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
        qs = check.filter_for_user(qs, user)

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
    if isinstance(obj, (models.File, models.Statement)):
        obj = obj.project_version.project
    elif isinstance(obj, models.ProjectVersion):
        obj = obj.project
    if not isinstance(obj, models.Project):
        raise ValueError(f"CanViewProject cannot be used on {obj}")
    # is public or user is owner or user is member of owning organization
    return (
        obj.visibility == models.ProjectVisibility.PUBLIC
        or obj.user_id == user.id
        or obj.organization_id is not None
        and obj.organization.members.filter(user=user).exists()
    )


def can_write_project(user: User, obj: Any) -> bool:
    """Whether the given user can write to the project-related object."""
    if user.is_authenticated and user.is_staff:
        return True
    if user.is_anonymous:
        return False
    # TODO @Performance: prefetch project or check condition entirely in SQL
    # normalize to project instance
    if isinstance(obj, (models.File, models.Statement)):
        obj = obj.project_version.project
    elif isinstance(obj, models.ProjectVersion):
        obj = obj.project
    if not isinstance(obj, models.Project):
        raise ValueError(f"CanWriteProject cannot be used on {obj}")
    return (
        obj.user_id == user.id
        or obj.organization_id is not None
        and obj.organization.members.filter(user=user).exists()
    )


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

    def filter_for_user(self, qs: QuerySet, user: User) -> QuerySet:
        if not user.is_authenticated:
            return qs.filter(visibility=models.ProjectVisibility.PUBLIC)
        if user.is_staff:
            return qs
        # is public or user is owner or user is member of owning organization
        return qs.filter(
            Q(visibility=models.ProjectVisibility.PUBLIC)
            | Q(user=user)
            | Q(organization__members=user),
        )


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
