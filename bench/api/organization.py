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
    slug: auto
    created_at: auto
    updated_at: auto
    projects: list[Annotated["Project", lazy(".project")]]
    members: list[Annotated["User", lazy(".user")]]
