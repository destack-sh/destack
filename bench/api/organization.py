from typing import TYPE_CHECKING, Annotated, Optional, cast

from django.core.exceptions import PermissionDenied
from strawberry import auto, lazy
from strawberry_django_plus import gql
from strawberry_django_plus.relay import GlobalID
from strawberry_django_plus.types import OperationInfo

from bench import models
from bench.api.auth import (
    CanViewProject,
    CanWriteOrganization,
    can_write_organization,
    check_can_write_organization,
)
from bench.api.owner import AccessTokenFilter, Owner
from bench.api.utils import safe_mutation

if TYPE_CHECKING:
    from bench.api.project import Project
    from bench.api.token import AccessToken
    from bench.api.user import User


@gql.django.filter(models.Organization)
class OrganizationFilter:
    slug_prefix: Optional[str]

    def filter(self, queryset):
        if self.slug_prefix:
            queryset = queryset.filter(slug__startswith=self.slug_prefix)
        return queryset


@gql.django.type(models.Organization)
class Organization(gql.relay.Node, Owner):
    name: auto
    created_at: auto
    updated_at: auto
    description: auto
    members: gql.relay.Connection[Annotated["User", lazy(".user")]] = gql.django.connection()
    memberships: gql.relay.Connection["OrganizationMembership"] = gql.django.connection()
    invites: gql.relay.Connection["OrganizationInvite"] = gql.django.connection()
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


OrganizationMembershipLevel = gql.enum(models.OrganizationMembershipLevel)


@gql.django.type(models.OrganizationMembership)
class OrganizationMembership(gql.Node):
    organization: Organization
    user: Annotated["User", lazy(".user")]
    level: OrganizationMembershipLevel
    created_at: auto
    updated_at: auto


@gql.django.type(models.OrganizationInvite)
class OrganizationInvite(gql.Node):
    organization: Organization
    user: Optional[Annotated["User", lazy(".user")]]
    email: auto
    level: OrganizationMembershipLevel
    created_at: auto
    updated_at: auto
    email_sent_at: auto


@gql.input
class OrganizationCreateInput:
    name: str
    slug: str


@gql.input
class OrganizationUpdateInput(gql.NodeInput):
    name: str
    description: str


@gql.input
class OrganizationRenameInput(gql.NodeInput):
    slug: str


@gql.input
class OrganizationInviteInput(gql.NodeInput):
    emails: list[str]
    level: OrganizationMembershipLevel
    message: Optional[str] = None


@gql.input
class OrganizationUpdateMembershipInput(gql.NodeInput):
    user_id: GlobalID
    level: OrganizationMembershipLevel


@gql.input
class OrganizationRemoveMembershipInput(gql.NodeInput):
    user_id: GlobalID


@gql.type
class OrganizationMutation:
    @safe_mutation(atomic=True)
    def create_organization(
        self, info, input: OrganizationCreateInput
    ) -> Organization | OperationInfo:
        user = info.context.request.scope["user"]._wrapped
        if not user.is_authenticated:
            raise PermissionDenied("you must be logged in to create an organization")
        organization = models.Organization.objects.create_organization(
            name=input.name, slug=input.slug
        )
        user.join_organization(organization, OrganizationMembershipLevel.Owner)
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

    @safe_mutation(atomic=True)
    def create_organization_invites(
        self, info, input: OrganizationInviteInput
    ) -> Organization | OperationInfo:
        organization = models.Organization.objects.get(id=input.id.node_id)
        check_can_write_organization(info, organization)
        for email in input.emails:
            invite = organization.create_invite(
                email, input.level, input.message, created_by=info.context.request.scope["user"]
            )
            invite.full_clean()
        return organization

    @safe_mutation
    def cancel_organization_invite(self, info, id: GlobalID) -> Organization | OperationInfo:
        invite = models.OrganizationInvite.objects.get(id=id.node_id)
        check_can_write_organization(info, invite.organization)
        invite.delete()
        return invite.organization

    @safe_mutation
    def update_organization_membership(
        self, info, input: OrganizationUpdateMembershipInput
    ) -> OrganizationMembership | OperationInfo:
        organization = models.Organization.objects.get(id=input.id.node_id)
        check_can_write_organization(info, organization)
        membership = organization.memberships.get(user_id=input.user_id.node_id)
        membership.level = input.level
        membership.save()
        return membership

    @safe_mutation
    def remove_organization_membership(
        self, info, input: OrganizationRemoveMembershipInput
    ) -> Organization | OperationInfo:
        organization = models.Organization.objects.get(id=input.id.node_id)
        check_can_write_organization(info, organization)
        membership = organization.memberships.get(user_id=input.user_id.node_id)
        membership.delete()
        return organization
