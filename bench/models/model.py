from __future__ import annotations

from django.db import models

from bench.models.tag import TaggableMixin
from bench.models.utils import MAX_DESCRIPTION_LENGTH, MAX_NAME_LENGTH, UUIDModel


class ModelManager(models.Manager):
    pass


class Model(TaggableMixin, UUIDModel):
    """
    A model is a language model provided and stored elsewhere.

    Baseline models are typically provided externally and may then be fine-tuned within a project.

    TODO @Cleanup: null project_id + global_name to access models directly from orgs feels hacky
        But is required to have external models and custom models (fine-tuned in a project).
        Later, can  move this concept to a separate table.
    """

    name = models.CharField(max_length=MAX_NAME_LENGTH)
    # optional organization-wide name linking to this model
    global_name = models.CharField(max_length=MAX_NAME_LENGTH, null=True, blank=True)
    description = models.CharField(max_length=MAX_DESCRIPTION_LENGTH, null=True, blank=True)
    created_at = models.DateTimeField(auto_now_add=True)
    handler_id = models.CharField(max_length=64)

    organization = models.ForeignKey(
        "Organization", on_delete=models.CASCADE, related_name="models"
    )
    project = models.ForeignKey(
        "Project", on_delete=models.CASCADE, null=True, blank=True, related_name="models"
    )

    objects = ModelManager()

    def __str__(self):
        return f"{self.organization.slug}/{self.project.slug}/models/{self.name}@{self.id}"

    class Meta:
        constraints = [
            # check that the organization + global_name is unique
            models.UniqueConstraint(
                fields=["organization_id", "global_name"],
                name="bench_model_organization_global_name_ak",
            ),
        ]
