from django.db import models

from bench.models.tag import TaggableMixin
from bench.models.utils import MAX_DESCRIPTION_LENGTH, MAX_NAME_LENGTH, UUIDModel


class Project(TaggableMixin, UUIDModel):
    name: models.CharField = models.CharField(max_length=MAX_NAME_LENGTH)
    description: models.CharField = models.CharField(max_length=MAX_DESCRIPTION_LENGTH, null=True)
    created_at: models.DateTimeField = models.DateTimeField(auto_now_add=True)
    updated_at: models.DateTimeField = models.DateTimeField(auto_now=True)

    organization: models.ForeignKey = models.ForeignKey(
        "Organization", on_delete=models.CASCADE, related_name="projects"
    )
    # we'll likely have multiple programs per project soon
    program: models.ForeignKey = models.ForeignKey(
        "Flow", on_delete=models.CASCADE, related_name="projects"
    )
    flows = models.ManyToManyField("Flow", related_name="projects")
    datasets = models.ManyToManyField("Dataset", related_name="projects")
