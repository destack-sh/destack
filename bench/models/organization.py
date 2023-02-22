from typing import Optional

from django.db import models, transaction

from bench.models.owner import OwnerSlug
from bench.models.utils import UUIDModel


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

    name: models.CharField = models.CharField(max_length=256)
    owner_slug: models.ForeignKey = models.OneToOneField(
        "OwnerSlug",
        unique=True,
        on_delete=models.CASCADE,
        null=True,
        related_name="organization",
    )
    owner_slug_id: Optional[str]  # noqa via Statement.reference
    created_at: models.DateTimeField = models.DateTimeField(auto_now_add=True)
    updated_at: models.DateTimeField = models.DateTimeField(auto_now=True)

    members: models.ManyToManyField = models.ManyToManyField(
        "bench.User",
        through="OrganizationMembership",
        related_name="organizations",
        related_query_name="organization",
    )

    objects = OrganizationManager()

    @property
    def slug(self) -> str:
        return self.owner_slug_id

    def __str__(self):
        return self.slug

    def __repr__(self):
        return f"<Organization {self.slug} {self.id}>"

    class Meta:
        default_manager_name = "objects"


class OrganizationMembership(UUIDModel):
    # kept in sync with TeamMembership.Level
    class Level(models.IntegerChoices):
        Member = 1
        Author = 6
        Administrator = 12
        Owner = 16

    organization: models.ForeignKey = models.ForeignKey(
        Organization, on_delete=models.CASCADE, related_name="memberships"
    )
    user: models.ForeignKey = models.ForeignKey(
        "bench.User", on_delete=models.CASCADE, related_name="organization_memberships+"
    )
    level: models.SmallIntegerField = models.SmallIntegerField(choices=Level.choices)

    created_at: models.DateTimeField = models.DateTimeField(auto_now_add=True)
    updated_at: models.DateTimeField = models.DateTimeField(auto_now=True)

    def __str__(self):
        return self.Level(self.level)

    class Meta:
        constraints = [
            models.UniqueConstraint(
                name="bench_organization_membership_ak", fields=["organization_id", "user_id"]
            )
        ]
