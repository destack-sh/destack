from datetime import datetime
from typing import TYPE_CHECKING, Annotated, Optional

import strawberry
import strawberry_django
from django.db.models import Q
from strawberry import lazy
from strawberry.relay import GlobalID

from bench import models
from bench.utils.dt import utcnow_with_tz

if TYPE_CHECKING:
    from bench.api.project import Project
    from bench.api.token import AccessToken


@strawberry_django.filter(models.AccessToken)
class AccessTokenFilter:
    include_inactive: Optional[bool] = False

    def filter(self, queryset):
        if not self.include_inactive:
            queryset = queryset.filter(Q(revoked_at__isnull=True)).filter(
                Q(expires_at__gte=utcnow_with_tz()) | Q(expires_at__isnull=True)
            )
        return queryset


@strawberry.interface
class Owner:
    id: GlobalID
    slug: str
    name: str
    created_at: datetime
    updated_at: datetime
    projects: strawberry_django.relay.ListConnectionWithTotalCount[
        Annotated["Project", lazy(".project")]
    ]
    access_tokens: strawberry_django.relay.ListConnectionWithTotalCount[
        Annotated["AccessToken", lazy(".token")]
    ]
    can_view_full: bool
    can_write: bool
