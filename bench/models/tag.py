from django.db import models

from bench.models.utils import MAX_DESCRIPTION_LENGTH, MAX_NAME_LENGTH, UUIDModel


class Tag(UUIDModel):
    """
    A generic label for associating groups and/or parts of primitives like
    artifacts, functions and executions with some metadata.
    """

    type = models.CharField(max_length=64)
    name = models.CharField(max_length=MAX_NAME_LENGTH)
    description = models.CharField(
        max_length=MAX_DESCRIPTION_LENGTH, null=True, blank=True
    )
    created_at = models.DateTimeField(auto_now_add=True)
    updated_at = models.DateTimeField(auto_now=True)
    metadata = models.JSONField(default=dict)


class TaggedItem(UUIDModel):
    """
    Tagged items track Tag<->Model relationships in a single place.
    """

    tag = models.ForeignKey(Tag, on_delete=models.CASCADE, related_name="tagged_items")

    artifact = models.ForeignKey(
        "Artifact", on_delete=models.CASCADE, related_name="tagged_items"
    )
    artifact_version = models.ForeignKey(
        "ArtifactVersion", on_delete=models.CASCADE, related_name="tagged_items"
    )
    flow = models.ForeignKey(
        "Flow", on_delete=models.CASCADE, related_name="tagged_items"
    )
    flow_version = models.ForeignKey(
        "FlowVersion", on_delete=models.CASCADE, related_name="tagged_items"
    )


class Alias(Tag):
    class Meta:
        proxy = True


class Capability(Tag):
    class Meta:
        proxy = True


class Stage(Tag):
    class Meta:
        proxy = True
