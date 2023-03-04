from typing import TYPE_CHECKING, Annotated, cast

from django.core.exceptions import PermissionDenied
from strawberry import auto, lazy
from strawberry_django_plus import gql
from strawberry_django_plus.types import OperationInfo

from bench import models
from bench.api.auth import (
    CanViewProject,
    CanWriteOrganization,
    can_write_organization,
    check_can_write_organization,
)
from bench.api.owner import AccessTokenFilter, Owner
from bench.api.util import safe_mutation
from bench.models import OrganizationMembership

if TYPE_CHECKING:
    from bench.api.project import Project
    from bench.api.token import AccessToken
    from bench.api.user import User


@gql.django.type(models.Organization)
class Organization(gql.relay.Node, Owner):
    name: auto
    created_at: auto
    updated_at: auto
    description: auto
    members: gql.relay.Connection[Annotated["User", lazy(".user")]] = gql.django.connection()
    projects: gql.relay.Connection[Annotated["Project", lazy(".project")]] = gql.django.connection(
        directives=[CanViewProject(at_root=False)]
    )
    access_tokens: gql.relay.Connection[
        Annotated["AccessToken", lazy(".token")]
    ] = gql.django.connection(
        filters=AccessTokenFilter, directives=[CanWriteOrganization(at_root=False)]
    )

    @gql.field
    def can_view_full(self, info: OperationInfo):
        user = cast(models.User, info.context.request.scope["user"]._wrapped)
        return can_write_organization(user, self)

    @gql.field
    def can_write(self, info: OperationInfo):
        user = cast(models.User, info.context.request.scope["user"]._wrapped)
        return can_write_organization(user, self)

    @gql.django.field(only=["owner_slug_id"])
    def slug(self, info) -> str:
        return self.owner_slug_id


@gql.input
class OrganizationCreateInput(gql.NodeInput):
    name: str
    slug: str


@gql.input
class OrganizationUpdateInput(gql.NodeInput):
    name: str
    description: str


@gql.input
class OrganizationRenameInput(gql.NodeInput):
    slug: str


@gql.type
class OrganizationMutation:
    @safe_mutation(atomic=True)
    def create_organization(
        self, info, input: OrganizationCreateInput
    ) -> Organization | OperationInfo:
        user = info.context.request.scope["user"]._wrapped
        if not user.is_authenticated:
            raise PermissionDenied("you must be logged in to create an organization")
        organization = models.Organization.objects.create(
            name=input.name, owner=user, owner_slug_id=input.slug
        )
        user.join_organization(organization, OrganizationMembership.Level.Owner)
        return organization

    @safe_mutation
    def update_organization(
        self, info, input: OrganizationUpdateInput
    ) -> Organization | OperationInfo:
        organization = models.Organization.objects.get(id=input.id.node_id)
        check_can_write_organization(info, organization)
        organization.name = input.name
        organization.description = input.description
        organization.save()
        return organization
