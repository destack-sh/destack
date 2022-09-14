from django.db import models

from bench.models.utils import MAX_DESCRIPTION_LENGTH, MAX_NAME_LENGTH, UUIDModel


class Project(UUIDModel):
    name: models.CharField = models.CharField(max_length=MAX_NAME_LENGTH)
    description: models.CharField = models.CharField(max_length=MAX_DESCRIPTION_LENGTH)
    created_at: models.DateTimeField = models.DateTimeField(auto_now_add=True)
    updated_at: models.DateTimeField = models.DateTimeField(auto_now=True)

    organization: models.ForeignKey = models.ForeignKey(
        "Organization", on_delete=models.CASCADE, related_name="projects"
    )
    artifacts: models.ManyToManyField = models.ManyToManyField(
        "Artifact", through="ProjectArtifactLink", related_name="projects"
    )


class ProjectArtifactLink(UUIDModel):
    project: models.ForeignKey = models.ForeignKey("Project", on_delete=models.CASCADE)
    artifact: models.ForeignKey = models.ForeignKey("Artifact", on_delete=models.RESTRICT)
