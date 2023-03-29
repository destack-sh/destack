from datetime import datetime
from typing import TYPE_CHECKING, Annotated, Optional

from django.db.models import Q
from strawberry import auto, lazy
from strawberry.types import Info
from strawberry_django_plus import gql
from strawberry_django_plus.types import OperationInfo

from bench import models
from bench.api.auth import check_can_write_user
from bench.api.util import safe_mutation

if TYPE_CHECKING:
    from bench.api.organization import OrganizationInvite
    from bench.api.user import User

NotificationType = gql.enum(models.NotificationType)
NotificationStatus = gql.enum(models.NotificationStatus)


@gql.django.filter(models.Notification)
class NotificationFilter:
    status: Optional[NotificationStatus] = None
    not_archived: Optional[bool] = None
    created_at__gte: Optional[datetime] = None

    def filter(self, queryset):
        if self.not_archived:
            queryset = queryset.filter(
                Q(archived_at__isnull=True)
                & (Q(expires_at__isnull=True) | Q(expires_at__gte=datetime.utcnow()))
            )

        if self.status == NotificationStatus.ACTIVE:
            queryset = queryset.filter(
                Q(read_at__isnull=True)
                & Q(archived_at__isnull=True)
                & (Q(expires_at__isnull=True) | Q(expires_at__gte=datetime.utcnow()))
            )
        elif self.status == NotificationStatus.READ:
            queryset = queryset.filter(read_at__isnull=False)
        elif self.status is not None:
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


@gql.input
class NotificationMarkInput(gql.NodeInput):
    status: NotificationStatus


@gql.type
class NotificationMutation:
    @safe_mutation
    def mark_notification(
        self, info: Info, input: NotificationMarkInput
    ) -> Notification | OperationInfo:
        notification = models.Notification.objects.get(pk=input.id.node_id)
        check_can_write_user(info, notification)
        notification.mark_as(input.status)
        return notification


@gql.type
class NotificationSubscription:
    pass
