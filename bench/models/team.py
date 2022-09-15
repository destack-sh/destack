from django.db import models

from bench.models.utils import UUIDModel

DEFAULT_TEAM_NAME = "A Team"


class TeamManager(models.Manager):
    pass


class Team(UUIDModel):
    name: models.CharField = models.CharField(max_length=64, default=DEFAULT_TEAM_NAME)
    created_at: models.DateTimeField = models.DateTimeField(auto_now_add=True)
    updated_at: models.DateTimeField = models.DateTimeField(auto_now=True)

    organization: models.ForeignKey = models.ForeignKey(
        "Organization", on_delete=models.CASCADE, related_name="teams", related_query_name="team"
    )
    members: models.ManyToManyField = models.ManyToManyField("User", through="TeamMembership")

    objects = TeamManager()


class TeamMembership(UUIDModel):
    # kept in sync with OrganizationMembership.Level
    class Level(models.IntegerChoices):
        Member = 1
        Administrator = 8

    team: models.ForeignKey = models.ForeignKey(
        Team, on_delete=models.CASCADE, related_name="memberships"
    )
    user: models.ForeignKey = models.ForeignKey(
        "bench.User", on_delete=models.CASCADE, related_name="team_memberships+"
    )
    level: models.SmallIntegerField = models.SmallIntegerField(choices=Level.choices)

    created_at: models.DateTimeField = models.DateTimeField(auto_now_add=True)
    updated_at: models.DateTimeField = models.DateTimeField(auto_now=True)

    class Meta:
        constraints = [
            models.UniqueConstraint(name="bench_team_membership_ak", fields=["team_id", "user_id"])
        ]
