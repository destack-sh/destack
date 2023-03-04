from datetime import datetime
from typing import TYPE_CHECKING, Annotated, Optional

from django.db.models import Q
from strawberry import lazy
from strawberry_django_plus import gql
from strawberry_django_plus.relay import GlobalID

from bench import models

if TYPE_CHECKING:
    from bench.api.project import Project
    from bench.api.token import AccessToken


@gql.django.filter(models.AccessToken)
class AccessTokenFilter:
    include_inactive: Optional[bool] = False

    def filter(self, queryset):
        if not self.include_inactive:
            queryset = queryset.filter(Q(revoked_at__isnull=True)).filter(
                Q(expires_at__gte=datetime.utcnow()) | Q(expires_at__isnull=True)
            )
        return queryset


@gql.interface
class Owner:
    id: GlobalID
    slug: str
    name: str
    created_at: datetime
    updated_at: datetime
    projects: gql.relay.Connection[Annotated["Project", lazy(".project")]]
    access_tokens: gql.relay.Connection[Annotated["AccessToken", lazy(".token")]]
