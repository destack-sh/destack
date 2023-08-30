from typing import TYPE_CHECKING, Optional

from django.db import models, transaction

from bench.models.owner import OwnerSlug
from bench.models.utils import UUIDModel
from bench.utils.uuidt import MAX_DESCRIPTION_LENGTH

if TYPE_CHECKING:
    from bench.models import User


class OrganizationManager(models.Manager["Organization"]):
    def get_by_slug(self, organization: str):
        return self.get(owner_slug_id=organization)

    @transaction.atomic
    def create_organization(self, name: str, slug: str) -> "Organization":
        owner_slug = OwnerSlug.objects.create_slug(slug)
        organization = self.create(name=name, owner_slug=owner_slug)
        return organization


class Organization(UUIDModel):
    """
    An organization is a group of users and projects.
    """

    name = models.CharField(max_length=256)
    owner_slug = models.OneToOneField(
        "OwnerSlug",
        unique=True,
        on_delete=models.CASCADE,
        null=True,
        related_name="organization",
    )
    owner_slug_id: Optional[str]  # noqa
    description = models.CharField(max_length=MAX_DESCRIPTION_LENGTH, blank=True, null=True)
    created_at = models.DateTimeField(auto_now_add=True)
    updated_at = models.DateTimeField(auto_now=True)

    projects: models.QuerySet["Project"]  # noqa via Project.user
    members = models.ManyToManyField(
        "User", through="OrganizationMembership", related_name="organizations"
    )
    memberships: models.QuerySet[
        "OrganizationMembership"
    ]  # noqa via OrganizationMembership.organization
    invites: models.QuerySet["OrganizationInvite"]  # noqa via OrganizationInvite.organization

    objects = OrganizationManager()

    @transaction.atomic(savepoint=False)
    def create_invite(
        self,
        email: str,
        level: "OrganizationRole",
        message: str = None,
        created_by: "User" = None,
    ) -> "OrganizationInvite":
        from bench.models.notification import Notification, NotificationType
        from bench.models.user import User

        user = User.objects.filter(email=email).first()
        if user is not None and self.members.filter(id=user.id).exists():
            raise ValueError("user already a member of organization")

        invite = OrganizationInvite.objects.create(
            organization=self,
            email=email,
            level=level,
            message=message,
            created_by=created_by,
            user=user,
        )

        # create notification if the user is signed up
        if user is not None:
            Notification.objects.create(
                type=NotificationType.ORGANIZATION_INVITE,
                user=user,
                invite=invite,
            )

        return invite

    @transaction.atomic
    def accept_invite(self, invite: "OrganizationInvite"):
        if invite.user is None:
            raise ValueError("cannot accept invite without registered user")
        invite.organization.add_user(invite.user, invite.level)
        invite.delete()

    @property
    def slug(self) -> str:
        return self.owner_slug_id

    def __str__(self):
        return self.slug

    def __repr__(self):
        return f"<Organization {self.slug} {self.id}>"

    class Meta:
        default_manager_name = "objects"


class OrganizationRole(models.IntegerChoices):
    Guest = 1
    Member = 4
    Manager = 8
    Owner = 12


class OrganizationMembership(UUIDModel):
    organization = models.ForeignKey(
        Organization, on_delete=models.CASCADE, related_name="memberships"
    )
    user = models.ForeignKey(
        "User", on_delete=models.CASCADE, related_name="organization_memberships"
    )
    level = models.SmallIntegerField(choices=OrganizationRole.choices)

    created_at = models.DateTimeField(auto_now_add=True)
    updated_at = models.DateTimeField(auto_now=True)

    def __str__(self):
        return f"{self.organization} -> {self.user} ({self.level})"

    def __repr__(self):
        return f"<OrganizationMembership {self}>"

    class Meta:
        ordering = ["-created_at"]
        constraints = [
            models.UniqueConstraint(
                name="bench_organization_membership_ak", fields=["organization", "user"]
            )
        ]


class OrganizationInvite(UUIDModel):
    """
    An invitation to join an organization (for existing or not yet existing users).
    """

    organization = models.ForeignKey(Organization, on_delete=models.CASCADE, related_name="invites")
    email = models.EmailField()
    user = models.ForeignKey(
        "User", on_delete=models.CASCADE, null=True, blank=True, related_name="organization_invites"
    )
    level = models.SmallIntegerField(choices=OrganizationRole.choices)
    message = models.TextField(blank=True, null=True)
    email_sent_at = models.DateTimeField(blank=True, null=True)

    created_by = models.ForeignKey("User", null=True, on_delete=models.SET_NULL)
    created_at = models.DateTimeField(auto_now_add=True)
    updated_at = models.DateTimeField(auto_now=True)

    def __str__(self):
        return f"{self.organization} -> {self.email} ({self.level})"

    def __repr__(self):
        return f"<OrganizationInvite {self}>"

    def accept(self):
        self.organization.accept_invite(self)

    class Meta:
        ordering = ["created_at"]
        constraints = [
            models.UniqueConstraint(
                name="bench_organization_invite_ak", fields=["organization_id", "email"]
            )
        ]
