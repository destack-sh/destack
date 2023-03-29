from django.db import models
from django_choices_field import TextChoicesField

from bench.models.utils import UUIDModel
from bench.utils.uuidt import MAX_NAME_LENGTH


class BuildCandidateStatus(models.TextChoices):
    Planned = "planned"
    Building = "building"
    Evaluating = "evaluating"
    CompletedWon = "completed_won"
    CompletedAbandoned = "completed_abandoned"
    Cancelled = "cancelled"


class BuildCandidate(UUIDModel):
    """
    A retained candidate from a Bench build.
    """

    created_at = models.DateTimeField(auto_now_add=True)
    updated_at = models.DateTimeField(auto_now=True)
    project = models.ForeignKey("Project", on_delete=models.CASCADE, related_name="+")
    project_version = models.ForeignKey(
        "ProjectVersion", on_delete=models.CASCADE, related_name="+"
    )
    job = models.ForeignKey("Job", on_delete=models.CASCADE, related_name="+", null=True)
    file = models.ForeignKey("File", on_delete=models.SET_NULL, related_name="+", null=True)
    status = TextChoicesField(BuildCandidateStatus, default=BuildCandidateStatus.Planned)
    name = models.CharField(max_length=MAX_NAME_LENGTH)
    build = models.ForeignKey(
        "Statement", on_delete=models.CASCADE, related_name="build_candidates+"
    )
    evaluation = models.ForeignKey(
        "EvaluationResult", on_delete=models.CASCADE, related_name="+", null=True
    )
    order_key = models.CharField(max_length=64)

    def __str__(self):
        return f"{self.build} {self.name} {self.status} ({self.id})"

    def __repr__(self):
        return f"<BuildCandidate {self}>"

    class Meta:
        ordering = ["created_at"]
        constraints = [
            # order key must be unique per build job
            models.UniqueConstraint(
                fields=["build", "job", "order_key"],
                name="bench_buildcandidate_order_key_ak",
            ),
            # can only be one completed_won per build job
            models.UniqueConstraint(
                fields=["build", "job", "status"],
                condition=models.Q(status=BuildCandidateStatus.CompletedWon),
                name="bench_buildcandidate_completed_won_ak",
            ),
        ]
