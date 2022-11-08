from __future__ import annotations

from typing import TYPE_CHECKING, Optional

from django.db import models
from django.db.models import QuerySet

from bench.models.utils import MAX_DESCRIPTION_LENGTH, MAX_NAME_LENGTH, MODEL_TYPE, UUIDModel

if TYPE_CHECKING:
    from bench.models import Organization, TaggableMixin
else:
    Organization = object


class ModelManager(models.Manager):
    def get_queryset(self) -> QuerySet[Model]:
        return super().get_queryset().filter(type__exact=MODEL_TYPE)

    def get_or_create(self, *args, **kwargs):
        kwargs["type"] = MODEL_TYPE
        return super().get_or_create(*args, **kwargs)

    def create_model(
        self,
        name: str,
        organization: Organization,
        description: Optional[str],
    ) -> Model:
        """Creates the given model with an initial version"""
        model = Model(
            type=MODEL_TYPE, name=name, organization=organization, description=description
        )
        model.save()
        return model


class Model(TaggableMixin, UUIDModel):
    """
    A model is a language model provided and stored elsewhere.

    Baseline models are typically provided externally and may then be fine-tuned within a project.

    TODO @Cleanup: null project_id + global_name to access models directly from orgs feels hacky
        But is required to have external models and custom models (fine-tuned in a project).
    """

    name = models.CharField(max_length=MAX_NAME_LENGTH)
    # optional organization-wide name linking to this model
    global_name = models.CharField(max_length=MAX_NAME_LENGTH, null=True, blank=True)
    description = models.CharField(max_length=MAX_DESCRIPTION_LENGTH, null=True, blank=True)
    created_at = models.DateTimeField(auto_now_add=True)
    handler_id = models.CharField()

    organization: models.ForeignKey = models.ForeignKey(
        "bench.Organization", on_delete=models.CASCADE, related_name="models"
    )
    project = models.ForeignKey(
        "bench.Project", on_delete=models.CASCADE, null=True, blank=True, related_name="models"
    )

    objects = ModelManager()

    def __str__(self):
        return f"{self.organization.slug}/{self.name}"

    class Meta:
        constraints = [
            # check that the name is unique within the project
            models.UniqueConstraint(
                fields=["project_id", "name"], name="bench_model_project_name_ak"
            ),
            # check that the organization + global_name is unique
            models.UniqueConstraint(
                fields=["organization_id", "global_name"],
                name="bench_model_organization_global_name_ak",
            ),
        ]
