from typing import TYPE_CHECKING, Annotated

from strawberry import lazy
from strawberry_django_plus import gql
from strawberry_django_plus.gql import auto

from bench import models

if TYPE_CHECKING:
    from bench.api.organization import Organization


@gql.django.type(models.User)
class User(gql.relay.Node):
    username: auto
    first_name: auto
    email: auto
    created_at: auto
    updated_at: auto
    organizations: list[Annotated["Organization", lazy(".organization")]]

    @gql.django.field(only=["owner_slug_id"])
    def slug(self, info) -> str:
        return self.owner_slug_id
