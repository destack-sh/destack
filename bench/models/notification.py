from django.db import models

from bench.models.user import User
from bench.models.utils import UUIDModel
from bench.utils.dt import utcnow_with_tz


class NotificationType(models.TextChoices):
    ORGANIZATION_INVITE = "organization_invite"
    PROJECT_INVITE = "bench_invite"
    RUN_FAILED = "run_failed"
    RUN_SUSPENDED = "run_suspended"


class NotificationStatus(models.TextChoices):
    ACTIVE = "active"
    READ = "read"
    EXPIRED = "expired"
    ARCHIVED = "archived"


class Notification(UUIDModel):
    type = models.CharField(max_length=20, choices=NotificationType.choices)
    user = models.ForeignKey("User", on_delete=models.CASCADE, related_name="notifications")

    created_at = models.DateTimeField(auto_now_add=True)
    expires_at = models.DateTimeField(blank=True, null=True)
    read_at = models.DateTimeField(blank=True, null=True)
    archived_at = models.DateTimeField(blank=True, null=True)

    # related objects
    organization_invite = models.ForeignKey(
        "OrganizationInvite", on_delete=models.CASCADE, null=True, related_name="+"
    )
    bench_invite = models.ForeignKey(
        "BenchInvite", on_delete=models.CASCADE, null=True, related_name="+"
    )
    run = models.ForeignKey("Run", on_delete=models.CASCADE, null=True, related_name="+")

    def __str__(self):
        return f"{self.type} -> {self.user}"

    def __repr__(self):
        return f"<Notification {self}>"

    def mark_as(self, status: NotificationStatus):
        if status == NotificationStatus.ACTIVE:
            self.read_at = None
            self.archived_at = None
        elif status == NotificationStatus.READ:
            self.read_at = utcnow_with_tz()
        elif status == NotificationStatus.ARCHIVED:
            self.archived_at = utcnow_with_tz()
        else:
            raise NotImplementedError(f"marking as {status} is not implemented")
        self.save()

    @property
    def status(self) -> NotificationStatus:
        if self.read_at:
            return NotificationStatus.READ
        elif self.archived_at:
            return NotificationStatus.ARCHIVED
        elif self.expires_at and self.expires_at < utcnow_with_tz():
            return NotificationStatus.EXPIRED
        else:
            return NotificationStatus.ACTIVE

    class Meta:
        ordering = ["-created_at"]


def create_notifications_on_signup(user: User):
    """Create onboarding notifications, recover invite notifications sent before signup"""

    # recover invite notifications
    organization_invites = [
        Notification(
            type=NotificationType.ORGANIZATION_INVITE, user=user, organization_invite=invite
        )
        for invite in user.organization_invites.all()
    ]
    bench_invites = [
        Notification(type=NotificationType.PROJECT_INVITE, user=user, bench_invite=invite)
        for invite in user.bench_invites.all()
    ]

    # bulk create notifications
    Notification.objects.bulk_create(organization_invites + bench_invites)
