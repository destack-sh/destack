from datetime import datetime

from django.db import models
from django_choices_field import TextChoicesField

from bench.models.user import User
from bench.models.utils import UUIDModel


class NotificationType(models.TextChoices):
    ORGANIZATION_INVITE = "organization_invite"


class NotificationStatus(models.TextChoices):
    ACTIVE = "active"
    READ = "read"
    EXPIRED = "expired"
    ARCHIVED = "archived"


class Notification(UUIDModel):
    type = TextChoicesField(choices_enum=NotificationType)
    user = models.ForeignKey("User", on_delete=models.CASCADE, related_name="notifications")

    created_at = models.DateTimeField(auto_now_add=True)
    expires_at = models.DateTimeField(blank=True, null=True)
    read_at = models.DateTimeField(blank=True, null=True)
    archived_at = models.DateTimeField(blank=True, null=True)

    invite = models.ForeignKey(
        "OrganizationInvite", on_delete=models.CASCADE, null=True, related_name="+"
    )

    def __str__(self):
        return f"{self.type} -> {self.user}"

    def __repr__(self):
        return f"<Notification {self}>"

    def mark_as(self, status: NotificationStatus):
        if status == NotificationStatus.ACTIVE:
            self.read_at = None
            self.archived_at = None
        elif status == NotificationStatus.READ:
            self.read_at = datetime.utcnow()
        elif status == NotificationStatus.ARCHIVED:
            self.archived_at = datetime.utcnow()
        else:
            raise NotImplementedError(f"marking as {status} is not implemented")
        self.save()

    @property
    def status(self) -> NotificationStatus:
        if self.read_at:
            return NotificationStatus.READ
        elif self.archived_at:
            return NotificationStatus.ARCHIVED
        elif self.expires_at and self.expires_at < datetime.utcnow():
            return NotificationStatus.EXPIRED
        else:
            return NotificationStatus.ACTIVE

    class Meta:
        ordering = ["-created_at"]


def create_notifications_on_signup(user: User):
    """Create onboarding notifications, recover invite notifications sent before signup"""

    # recover invite notifications
    notifications: list[Notification] = []
    for invite in user.invites.all():
        notification = Notification(
            type=NotificationType.ORGANIZATION_INVITE,
            user=user,
            invite=invite,
        )
        notifications.append(notification)

    # bulk create notifications
    Notification.objects.bulk_create(notifications)
