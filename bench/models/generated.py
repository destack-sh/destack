from __future__ import annotations

from django.db import models
from django_choices_field import TextChoicesField

from bench.models.utils import UUIDModel


class GeneratedContentMixin:
    """Build content of mappings."""

    generated_mappings: models.QuerySet["SourceMapping"]  # noqa via SourceMapping.build


class SourceMappingType(models.TextChoices):
    STATEMENT = "statement"
    RECORD = "record"
    TYPE_NODE = "type_node"


class SourceMapping(UUIDModel):
    """
    A source map for builds to track dependencies of generated content.
    """

    statement = models.ForeignKey(
        "Statement", on_delete=models.CASCADE, related_name="generated_mappings"
    )
    type = TextChoicesField(choices_enum=SourceMappingType, default=SourceMappingType.STATEMENT)
    source_statement = models.ForeignKey(
        "Statement", on_delete=models.CASCADE, related_name="target_mappings", null=True, blank=True
    )
    source_record = models.ForeignKey(
        "DatasetRecord",
        on_delete=models.CASCADE,
        related_name="target_mappings",
        null=True,
        blank=True,
    )
    source_type_node = models.ForeignKey(
        "SimpleTypeNode",
        on_delete=models.CASCADE,
        related_name="target_mappings",
        null=True,
        blank=True,
    )
    source_revision = models.IntegerField()
    target_statement = models.ForeignKey(
        "Statement", on_delete=models.CASCADE, related_name="source_mappings", null=True, blank=True
    )
    target_record = models.ForeignKey(
        "DatasetRecord",
        on_delete=models.CASCADE,
        related_name="source_mappings",
        null=True,
        blank=True,
    )
    target_type_node = models.ForeignKey(
        "SimpleTypeNode",
        on_delete=models.CASCADE,
        related_name="source_mappings",
        null=True,
        blank=True,
    )
    target_revision = models.IntegerField(null=True, blank=True)
