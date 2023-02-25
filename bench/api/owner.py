from datetime import datetime
from typing import TYPE_CHECKING, Annotated

from strawberry import lazy
from strawberry_django_plus import gql
from strawberry_django_plus.relay import GlobalID

if TYPE_CHECKING:
    from bench.api.project import Project


@gql.interface
class Owner:
    id: GlobalID
    slug: str
    name: str
    created_at: datetime
    updated_at: datetime
    projects: list[Annotated["Project", lazy(".project")]]
