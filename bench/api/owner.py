from datetime import datetime
from typing import TYPE_CHECKING, Annotated

from strawberry import lazy
from strawberry_django_plus import gql
from strawberry_django_plus.relay import GlobalID

if TYPE_CHECKING:
    from bench.api.auth import AccessToken
    from bench.api.project import Project


@gql.interface
class Owner:
    id: GlobalID
    slug: str
    name: str
    created_at: datetime
    updated_at: datetime
    projects: gql.relay.Connection[Annotated["Project", lazy(".project")]]
    access_tokens: gql.relay.Connection[Annotated["AccessToken", lazy(".auth")]]
