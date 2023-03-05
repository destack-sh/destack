from datetime import datetime
from typing import TYPE_CHECKING, Annotated, Optional

from django.db.models import Q
from strawberry import auto, lazy
from strawberry_django_plus import gql

from bench import models

if TYPE_CHECKING:
    from bench.api.organization import OrganizationInvite
    from bench.api.user import User

NotificationType = gql.enum(models.NotificationType)
NotificationStatus = gql.enum(models.NotificationStatus)


@gql.django.filter(models.Notification)
class NotificationFilter:
    status: Optional[NotificationStatus] = None
    created_at__gte: Optional[datetime] = None

    def filter(self, queryset):
        if self.status == NotificationStatus.ACTIVE:
            queryset = queryset.filter(
                Q(read_at__isnull=True)
                & Q(archived_at__isnull=True)
                & (Q(expires_at__isnull=True) | Q(expires_at__gte=datetime.utcnow()))
            )
        elif self.status == NotificationStatus.READ:
            queryset = queryset.filter(read_at__isnull=False)
        else:
            raise NotImplementedError(f"filtering by status {self.status} is not implemented")
        if self.created_at__gte:
            queryset = queryset.filter(created_at__gte=self.created_at__gte)
        return queryset


@gql.django.type(models.Notification)
class Notification(gql.Node):
    type: NotificationType
    status: NotificationStatus
    user: Annotated["User", lazy(".user")]

    created_at: auto
    read_at: auto
    expires_at: auto
    archived_at: auto

    invite: Annotated["OrganizationInvite", lazy(".organization")]
