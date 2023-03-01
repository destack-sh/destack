from typing import TYPE_CHECKING, Annotated

from asgiref.sync import async_to_sync
from channels.auth import logout as channels_logout
from django.core.exceptions import PermissionDenied
from strawberry import lazy
from strawberry.types import Info
from strawberry_django_plus import gql
from strawberry_django_plus.gql import auto
from strawberry_django_plus.types import OperationInfo

from bench import models
from bench.api.auth import CanViewProject
from bench.api.owner import Owner
from bench.api.util import safe_mutation

if TYPE_CHECKING:
    from bench.api.organization import Organization
    from bench.api.project import Project


@gql.django.type(models.User)
class User(gql.relay.Node, Owner):
    username: auto
    email: auto
    created_at: auto
    updated_at: auto
    completed_signup: auto
    bot: auto
    organizations: gql.relay.Connection[
        Annotated["Organization", lazy(".organization")]
    ] = gql.django.connection()
    projects: gql.relay.Connection[Annotated["Project", lazy(".project")]] = gql.django.connection(
        directives=[CanViewProject()]
    )

    @gql.django.field(only=["first_name"])
    def name(self) -> str:
        return self.first_name

    @gql.django.field(only=["owner_slug_id"])
    def slug(self, info) -> str:
        return self.owner_slug_id


@gql.input
class UserCompleteSignupInput(gql.NodeInput):
    username: str
    full_name: str


@gql.type
class UserMutation:
    @safe_mutation(atomic=True)
    def complete_signup(self, info, input: UserCompleteSignupInput) -> User | OperationInfo:
        user = models.User.objects.get(id=input.id.node_id)
        requesting_user = info.context.request.scope["user"]
        if not requesting_user.is_authenticated or user.id != requesting_user.id:
            raise PermissionDenied("can only complete signup for yourself")
        user.change_username(input.username)
        user.first_name = input.full_name
        user.completed_signup = True
        user.save()
        return user

    @safe_mutation
    def logout(self, info: Info) -> None | OperationInfo:
        if not info.context.request.scope["user"].is_authenticated:
            raise PermissionDenied("can only logout when logged in")
        async_to_sync(channels_logout)(info.context.request.scope)
        return None
