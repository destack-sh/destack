from typing import TYPE_CHECKING, Annotated, Optional

import strawberry
import strawberry_django
from django.core.exceptions import PermissionDenied
from strawberry import auto, lazy, relay
from strawberry.relay import GlobalID
from strawberry_django.fields.types import OperationInfo

from bench import models
from bench.api.auth import can_write_organization, check_can_write_organization
from bench.api.owner import AccessTokenFilter, Owner
from bench.api.utils import get_user_from_info, safe_mutation

if TYPE_CHECKING:
    from bench.api.project import Project
    from bench.api.token import AccessToken
    from bench.api.user import User


@strawberry_django.filter(models.Organization)
class OrganizationFilter:
    slug_prefix: Optional[str]

    def filter(self, queryset):
        if self.slug_prefix:
            queryset = queryset.filter(slug__startswith=self.slug_prefix)
        return queryset


@strawberry_django.type(models.Organization)
class Organization(Owner, relay.Node):
    name: auto
    created_at: auto
    updated_at: auto
    description: auto
    memberships: strawberry_django.relay.ListConnectionWithTotalCount[
        "OrganizationMembership"
    ] = strawberry_django.connection()
    invites: strawberry_django.relay.ListConnectionWithTotalCount[
        "OrganizationInvite"
    ] = strawberry_django.connection()
    projects: strawberry_django.relay.ListConnectionWithTotalCount[
        Annotated["Project", lazy(".project")]
    ] = strawberry_django.connection(directives=[])
    access_tokens: strawberry_django.relay.ListConnectionWithTotalCount[
        Annotated["AccessToken", lazy(".token")]
    ] = strawberry_django.connection(filters=AccessTokenFilter, directives=[])

    @strawberry_django.field
    def can_view_full(self, info: OperationInfo) -> bool:
        user = get_user_from_info(info)
        return can_write_organization(user, self)

    @strawberry_django.field
    def can_write(self, info: OperationInfo) -> bool:
        user = get_user_from_info(info)
        return can_write_organization(user, self)

    @strawberry_django.field(only=["owner_slug_id"])
    def slug(self, info) -> str:
        return self.owner_slug_id


OrganizationMembershipLevel = strawberry.enum(models.OrganizationMembershipLevel)


@strawberry_django.type(models.OrganizationMembership)
class OrganizationMembership(relay.Node):
    organization: Organization
    user: Annotated["User", lazy(".user")]
    level: OrganizationMembershipLevel
    created_at: auto
    updated_at: auto


@strawberry_django.type(models.OrganizationInvite)
class OrganizationInvite(relay.Node):
    organization: Organization
    user: Optional[Annotated["User", lazy(".user")]]
    email: auto
    level: OrganizationMembershipLevel
    created_at: auto
    updated_at: auto
    email_sent_at: auto


@strawberry.input
class OrganizationCreateInput:
    name: str
    slug: str


@strawberry.input
class OrganizationUpdateInput(strawberry_django.NodeInput):
    name: str
    description: str


@strawberry.input
class OrganizationRenameInput(strawberry_django.NodeInput):
    slug: str


@strawberry.input
class OrganizationInviteInput(strawberry_django.NodeInput):
    emails: list[str]
    level: OrganizationMembershipLevel
    message: Optional[str] = None


@strawberry.input
class OrganizationUpdateMembershipInput(strawberry_django.NodeInput):
    user_id: GlobalID
    level: OrganizationMembershipLevel


@strawberry.input
class OrganizationRemoveMembershipInput(strawberry_django.NodeInput):
    user_id: GlobalID


@strawberry.type
class OrganizationMutation:
    @safe_mutation(atomic=True)
    def create_organization(
        self, info, input: OrganizationCreateInput
    ) -> Organization | OperationInfo:
        user = get_user_from_info(info)
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
                email, input.level, input.message, created_by=get_user_from_info(info)
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
