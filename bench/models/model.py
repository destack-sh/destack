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
    A model is a machine learning model stored and provided elsewhere.
    """

    name = models.CharField(max_length=MAX_NAME_LENGTH)
    description = models.CharField(max_length=MAX_DESCRIPTION_LENGTH, null=True, blank=True)
    created_at = models.DateTimeField(auto_now_add=True)
    handler_id = models.CharField()

    organization: models.ForeignKey = models.ForeignKey(
        "bench.Organization", on_delete=models.CASCADE, related_name="models"
    )

    objects = ModelManager()

    class Meta:
        proxy = True
