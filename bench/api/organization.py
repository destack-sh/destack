from typing import TYPE_CHECKING, Annotated

from strawberry import auto, lazy
from strawberry_django_plus import gql

from bench import models
from bench.api.auth import CanViewProject, CanWriteOrganization
from bench.api.owner import Owner

if TYPE_CHECKING:
    from bench.api.auth import AccessToken
    from bench.api.project import Project
    from bench.api.user import User


@gql.django.type(models.Organization)
class Organization(gql.relay.Node, Owner):
    name: auto
    created_at: auto
    updated_at: auto
    members: gql.relay.Connection[Annotated["User", lazy(".user")]] = gql.django.connection()
    projects: gql.relay.Connection[Annotated["Project", lazy(".project")]] = gql.django.connection(
        directives=[CanViewProject(at_root=False)]
    )
    access_tokens: gql.relay.Connection[
        Annotated["AccessToken", lazy(".auth")]
    ] = gql.django.connection(directives=[CanWriteOrganization(at_root=False)])

    @gql.django.field(only=["owner_slug_id"])
    def slug(self, info) -> str:
        return self.owner_slug_id
