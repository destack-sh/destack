from typing import TYPE_CHECKING, Annotated

from strawberry import auto, lazy
from strawberry_django_plus import gql

from bench import models

if TYPE_CHECKING:
    from bench.api.project import Project
    from bench.api.user import User


@gql.django.type(models.Organization)
class Organization(gql.relay.Node):
    name: auto
    created_at: auto
    updated_at: auto
    members: list[Annotated["User", lazy(".user")]]
    projects: list[Annotated["Project", lazy(".project")]]

    @gql.django.field(only=["owner_slug_id"])
    def slug(self, info) -> str:
        return self.owner_slug_id
