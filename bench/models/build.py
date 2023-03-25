from django.db import models

from bench.models.utils import UUIDModel
from bench.utils.uuidt import MAX_NAME_LENGTH


class BuildCandidate(UUIDModel):
    created_at = models.DateTimeField(auto_now_add=True)
    updated_at = models.DateTimeField(auto_now=True)
    project = models.ForeignKey("Project", on_delete=models.CASCADE, related_name="+")
    project_version = models.ForeignKey(
        "ProjectVersion", on_delete=models.CASCADE, related_name="+"
    )
    job = models.ForeignKey("Job", on_delete=models.CASCADE, related_name="+", null=True)
    name = models.CharField(max_length=MAX_NAME_LENGTH)
    build = models.ForeignKey(
        "Statement", on_delete=models.CASCADE, related_name="build_candidates+"
    )
    evaluation = models.ForeignKey("EvaluationResult", on_delete=models.CASCADE, related_name="+")
