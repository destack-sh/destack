from datetime import datetime
from typing import TYPE_CHECKING, Annotated, Optional

import strawberry
import strawberry_django
import structlog
from django.db.models import Q
from strawberry import auto, lazy, relay
from strawberry.types import Info
from strawberry_django.fields.types import OperationInfo

from bench import models
from bench.api.auth import check_can_write_user
from bench.api.utils import safe_mutation
from bench.utils.dt import utcnow_with_tz

if TYPE_CHECKING:
    from bench.api.organization import OrganizationInvite
    from bench.api.project import ProjectInvite
    from bench.api.session import Run
    from bench.api.user import User

logger = structlog.get_logger(__name__)

NotificationType = strawberry.enum(models.NotificationType)
NotificationStatus = strawberry.enum(models.NotificationStatus)


@strawberry_django.filter(models.Notification)
class NotificationFilter:
    status: Optional[NotificationStatus] = None
    not_archived: Optional[bool] = None
    created_at__gte: Optional[datetime] = None

    def filter(self, queryset):
        if self.not_archived:
            queryset = queryset.filter(
                Q(archived_at__isnull=True)
                & (Q(expires_at__isnull=True) | Q(expires_at__gte=utcnow_with_tz()))
            )

        if self.status == NotificationStatus.ACTIVE:
            queryset = queryset.filter(
                Q(read_at__isnull=True)
                & Q(archived_at__isnull=True)
                & (Q(expires_at__isnull=True) | Q(expires_at__gte=utcnow_with_tz()))
            )
        elif self.status == NotificationStatus.READ:
            queryset = queryset.filter(read_at__isnull=False)
        elif self.status is not None:
            raise NotImplementedError(f"filtering by status {self.status} is not implemented")

        if self.created_at__gte:
            queryset = queryset.filter(created_at__gte=self.created_at__gte)
        return queryset


@strawberry_django.type(models.Notification)
class Notification(relay.Node):
    type: NotificationType
    status: NotificationStatus
    user: Annotated["User", lazy(".user")]

    created_at: auto
    read_at: auto
    expires_at: auto
    archived_at: auto

    organization_invite: Annotated["OrganizationInvite", lazy(".organization")]
    project_invite: Annotated["ProjectInvite", lazy(".project")]
    run: Annotated["Run", lazy(".session")]


@strawberry.input
class NotificationMarkInput(strawberry_django.NodeInput):
    status: NotificationStatus


@strawberry.type
class NotificationMutation:
    @safe_mutation
    def mark_notification(
        self, info: Info, input: NotificationMarkInput
    ) -> Notification | OperationInfo:
        notification = models.Notification.objects.select_related("user").get(pk=input.id.node_id)
        check_can_write_user(info, notification.user)
        notification.mark_as(input.status)
        return notification


@strawberry.type
class NotificationSubscription:
    pass
