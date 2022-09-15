from django.db import models

from bench.models.tag import TaggableMixin
from bench.models.utils import MAX_DESCRIPTION_LENGTH, MAX_NAME_LENGTH, UUIDModel


class Batch(TaggableMixin, UUIDModel):
    """
    A group of tasks with specific instructions.

    Tasks can be sourced from existing datasets?.
    """

    name: models.CharField = models.CharField(max_length=MAX_NAME_LENGTH, null=True)
    description: models.CharField = models.CharField(max_length=MAX_DESCRIPTION_LENGTH, null=True)
    created_at: models.DateTimeField = models.DateTimeField(auto_now_add=True)
    updated_at: models.DateTimeField = models.DateTimeField(auto_now=True)

    project: models.ForeignKey = models.ForeignKey(
        "Project", on_delete=models.CASCADE, related_name="batches"
    )
    # TODO @Cleanup @Architecture: do we even need to copy dataset records to here?
    #  Probably yes in case the source changes under us or we expand the dataset.
    dataset: models.ForeignKey = models.ForeignKey(
        "Dataset", on_delete=models.RESTRICT, related_name="+"
    )

    source_dataset: models.ForeignKey = models.ForeignKey(
        "Dataset", null=True, on_delete=models.SET_NULL, related_name="derived_batches"
    )
    source_view: models.ForeignKey = models.ForeignKey(
        "ArtifactView", null=True, on_delete=models.SET_NULL, related_name="derived_batches"
    )
    source_view_inline: models.JSONField = models.JSONField()
