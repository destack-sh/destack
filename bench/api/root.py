import os
import typing
from typing import TYPE_CHECKING, List, Optional, Union

import strawberry
import strawberry_django
from django.contrib.auth.models import AnonymousUser
from graphql import GraphQLError, NoSchemaIntrospectionCustomRule
from strawberry.extensions import AddValidationRules, Extension, ParserCache, QueryDepthLimiter
from strawberry.relay import GlobalID
from strawberry.types import ExecutionContext, Info
from strawberry_django.optimizer import DjangoOptimizerExtension

from bench import models
from bench.api.auth import HasModuleAccess, IsOwner
from bench.api.bench import Bench, BenchMutation, BenchVisibility
from bench.api.organization import Organization, OrganizationMutation
from bench.api.sentry import SentryPerformanceExtension
from bench.api.token import AccessToken, AccessTokenMutation
from bench.api.user import User, UserFilter, UserMutation
from bench.api.utils import HasCrud, get_user_from_info
from bench.models import OwnerSlug
from bench.settings import DEBUG, TEST
from bench.utils.utils import sentry_capture

if TYPE_CHECKING:
    pass

PyType = typing.Type


@strawberry.type
class SystemInfo:
    version: str
    git_commit: str


SYSTEM_INFO = SystemInfo(
    version=os.environ["VERSION"], git_commit=os.environ.get("GIT_COMMIT", "dev")
)


def get_me(self, info: Info) -> Optional[User]:
    # unwrap because we need the actual object but channels.auth gives us a UserLazyObject
    user = get_user_from_info(info)
    if isinstance(user, AnonymousUser):
        return None
    return user


def get_user_or_organization_by_slug(
    self, info: Info, slug: str
) -> Optional[Union[User, Organization]]:
    try:
        slug = OwnerSlug.objects.get(slug=slug)
        return slug.owner
    except OwnerSlug.DoesNotExist:
        return None


def get_module_by_tag(bench_id: GlobalID, tag: str):
    try:
        return models.Module.objects.get_by_tag(bench_id.node_id, tag)
    except models.Module.DoesNotExist:
        return None


def get_module_by_slug(owner: str, bench: str, tag: str):
    try:
        return models.Module.objects.get_by_slug(owner, bench, tag)
    except models.Module.DoesNotExist:
        return None


def get_bench_by_slug(owner: str, bench: str):
    try:
        return models.Bench.objects.get_by_slug(owner, bench)
    except models.Bench.DoesNotExist:
        return None


def get_user_by_slug(slug: str):
    try:
        return models.User.objects.get_by_slug(slug)
    except models.User.DoesNotExist:
        return None


def get_organization_by_slug(organization: str):
    try:
        return models.Organization.objects.get_by_slug(organization)
    except models.Organization.DoesNotExist:
        return None


def get_featured_benches(self) -> typing.Iterable[Bench]:
    # just return symbolx benches for now
    return models.Bench.objects.filter(
        organization__owner_slug_id="symbolx", visibility=BenchVisibility.PUBLIC
    )


@strawberry.type
class Query:
    system_info: SystemInfo = strawberry_django.field(resolver=lambda: SYSTEM_INFO)

    # users/orgs
    me: Optional[User] = strawberry_django.field(resolver=get_me)
    user: Optional[User] = strawberry_django.node()
    users: strawberry_django.relay.ListConnectionWithTotalCount[
        User
    ] = strawberry_django.connection(filters=UserFilter)
    organization: Optional[Organization] = strawberry_django.node()
    owner_by_slug: Optional[Union[User, Organization]] = strawberry_django.field(
        resolver=get_user_or_organization_by_slug
    )
    access_token: Optional[AccessToken] = strawberry_django.node(
        extensions=[IsOwner(map=lambda t: t.owner)]
    )

    # bench
    bench: Optional[Bench] = strawberry_django.node(extensions=[HasModuleAccess()])
    bench_by_slug: Optional[Bench] = strawberry_django.field(
        resolver=get_bench_by_slug, extensions=[HasModuleAccess()]
    )
    featured_benches: strawberry_django.relay.ListConnectionWithTotalCount[
        Bench
    ] = strawberry_django.connection(resolver=get_featured_benches)


@strawberry.type
class Mutation(
    UserMutation,
    OrganizationMutation,
    AccessTokenMutation,
    BenchMutation,
):
    pass


default_extensions: list[Union[PyType[Extension], Extension]] = [
    DjangoOptimizerExtension,
    QueryDepthLimiter(max_depth=10),
    SentryPerformanceExtension,
]
prod_extensions: list[Union[PyType[Extension], Extension]] = [
    ParserCache(),
    AddValidationRules([NoSchemaIntrospectionCustomRule]),
]
if DEBUG or TEST:
    extensions = default_extensions
else:
    extensions = default_extensions + prod_extensions


class SentryCaptureSchema(strawberry.Schema):
    def process_errors(
        self, errors: List[GraphQLError], execution_context: Optional[ExecutionContext] = None
    ) -> None:
        for error in errors:
            if error.original_error:
                sentry_capture(error.original_error)
        super().process_errors(errors, execution_context)


schema = SentryCaptureSchema(Query, Mutation, extensions=extensions, types=[HasCrud])
