from django.db import models

from bench.models.utils import UUIDModel


class Project(UUIDModel):
    organization: models.ForeignKey = models.ForeignKey(
        "Organization", on_delete=models.CASCADE, related_name="teams"
    )
    name: models.CharField = models.CharField(max_length=64)
    description: models.CharField = models.CharField(max_length=64)
    created_at: models.DateTimeField = models.DateTimeField(auto_now_add=True)
    updated_at: models.DateTimeField = models.DateTimeField(auto_now=True)

    artifacts: models.ManyToManyField = models.ManyToManyField(
        "Artifact", through="ProjectArtifactLink"
    )


class ProjectArtifactLink(UUIDModel):
    project: models.ForeignKey = models.ForeignKey("Project", on_delete=models.CASCADE)
    artifact: models.ForeignKey = models.ForeignKey("Artifact", on_delete=models.RESTRICT)
