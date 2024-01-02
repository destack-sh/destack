from __future__ import annotations

import uuid
from typing import TYPE_CHECKING, Optional, Union

import structlog
from django.db import models
from django.db.models import Q

from bench.language import StatementType, TypeHint, TypeTag
from bench.language.const import ScheduleType, TriggerType, TypeFlag
from bench.language.validation import MAX_NAME_LENGTH
from bench.models.utils import NAME_VALIDATOR, CrudNode, get_choices

if TYPE_CHECKING:
    from bench.models import File

logger = structlog.get_logger(__name__)


class FieldManager(models.Manager["Field"]):
    def get_queryset(self):
        # soft-deleted statements are not returned by default
        return super().get_queryset().select_related("statement")


class Field(CrudNode):
    """
    A (usually) named type of something.
    Do not write to this model directly as any change affects the opensearch indices.
    """

    statement = models.ForeignKey("Statement", on_delete=models.CASCADE, related_name="fields")
    name = models.CharField(
        max_length=MAX_NAME_LENGTH, null=True, blank=True, validators=[NAME_VALIDATOR]
    )
    key = models.CharField(max_length=48)
    order_key = models.CharField(max_length=MAX_NAME_LENGTH)
    tag = models.CharField(max_length=20, choices=get_choices(TypeTag))
    hint = models.CharField(max_length=20, choices=get_choices(TypeHint), null=True, blank=True)
    flags = models.IntegerField(default=0)
    value = models.JSONField(null=True, blank=True)
    text = models.TextField(null=True, blank=True)
    reference_ck = models.UUIDField(null=True, blank=True)

    def __str__(self):
        flag_str = ", ".join(flag.short_name.lower() for flag in TypeFlag if self.flags & flag)
        flags_str = f" ({flag_str})" if flag_str else ""
        name_str = f"{self.name} " if self.name else ""
        return f"{self.statement} {name_str}{self.tag}{flags_str}"

    def __repr__(self):
        return f"<Field {str(self)}>"

    @property
    def parent_id(self) -> Optional[uuid.UUID]:
        return self.statement_id

    @property
    def parent(self) -> Statement:
        return self.statement

    objects = FieldManager()

    class Meta:
        ordering = ["order_key"]
        default_manager_name = "objects"
        indexes = [models.Index(fields=["statement"])]
        constraints = [
            models.UniqueConstraint(
                fields=["statement", "order_key"],
                name="bench_statement_field_order_key_ak",
                condition=Q(deleted_at__isnull=True),
            ),
        ]


class TriggerManager(models.Manager["Trigger"]):
    def get_queryset(self):
        # soft-deleted statements are not returned by default
        return super().get_queryset().select_related("statement")


class Trigger(CrudNode):
    """
    A trigger to a statement.
    """

    statement = models.ForeignKey("Statement", on_delete=models.CASCADE, related_name="triggers")
    type = models.CharField(max_length=32, choices=get_choices(TriggerType))
    active = models.BooleanField(default=True)
    mapping = models.JSONField(null=True, blank=True)
    schedule_type = models.CharField(
        max_length=32, null=True, blank=True, choices=get_choices(ScheduleType)
    )
    timezone = models.CharField(max_length=64, null=True, blank=True)
    interval = models.IntegerField(null=True, blank=True)
    cron = models.CharField(max_length=64, null=True, blank=True)
    statement_ck = models.UUIDField(null=True, blank=True)
    scope_ck = models.UUIDField(null=True, blank=True)
    # internal
    processed_up_to = models.DateTimeField(null=True, blank=True)

    @property
    def parent_id(self) -> Optional[uuid.UUID]:
        return self.statement_id

    @property
    def parent(self) -> Statement:
        return self.statement

    class Meta:
        # interval must be >60 if set
        constraints = [
            models.CheckConstraint(
                check=models.Q(interval__isnull=True) | models.Q(interval__gte=60),
                name="bench_trigger_interval_gt_60_ck",
            ),
        ]


class TaggingManager(models.Manager["Tagging"]):
    def get_queryset(self) -> models.QuerySet[Tagging]:
        # soft-deleted statements are not returned by default
        return super().get_queryset().filter(deleted_at__isnull=True)


class Tagging(CrudNode):
    """
    An association between a tag and a statement.
    """

    statement = models.ForeignKey("Statement", on_delete=models.CASCADE, related_name="tags")
    key = models.CharField(max_length=48)
    reference_ck = models.UUIDField(null=True, blank=True)
    value = models.JSONField(null=True, blank=True)

    @property
    def parent_id(self):
        return self.statement_id

    @property
    def parent(self):
        return self.statement


class StatementManager(models.Manager["Statement"]):
    def get_queryset(self) -> models.QuerySet[Statement]:
        # soft-deleted statements are not returned by default
        return super().get_queryset().filter(deleted_at__isnull=True)


class Statement(CrudNode):
    """
    A nested statement in a file for working with Bench symbols and other stuff.
    """

    bench_version = models.ForeignKey(
        "BenchVersion", on_delete=models.CASCADE, related_name="statements"
    )
    file = models.ForeignKey("File", on_delete=models.CASCADE, related_name="statements")
    type = models.CharField(max_length=32, choices=get_choices(StatementType))
    name = models.CharField(
        max_length=MAX_NAME_LENGTH, null=True, blank=True, validators=[NAME_VALIDATOR]
    )

    parent_statement = models.ForeignKey(
        "Statement", on_delete=models.CASCADE, null=True, blank=True, related_name="children"
    )
    children: models.QuerySet[Statement]  # noqa via Statement.parent
    order_key = models.CharField(max_length=64)  # in file/parent

    # statement data
    key = models.CharField(max_length=32, null=True, blank=True)
    heading_level = models.IntegerField(null=True, blank=True)
    text = models.TextField(null=True, blank=True)
    code = models.TextField(null=True, blank=True)
    value = models.JSONField(null=True, blank=True)
    versioned = models.BooleanField(default=True)  # nocheckin: negate & rename to ???
    fields: models.QuerySet[Field]  # noqa via Field.statement
    taggings: models.QuerySet[Tagging]  # noqa via Tagging.statement
    triggers: models.QuerySet[Trigger]  # noqa via Trigger.statement
    # interp state
    issues: models.QuerySet["Issue"]  # noqa via Issue.statement
    resolved_fields: models.QuerySet["ResolvedField"]  # noqa via ResolvedField.statement

    def __str__(self):
        return f"{self.path} {self.type} {self.name}"

    @property
    def parent_id(self) -> Optional[uuid.UUID]:
        return self.parent_statement_id or self.file_id

    @property
    def parent(self) -> Union["Statement", "File"]:
        if self.parent_statement_id is not None:
            return self.parent_statement
        else:
            return self.file

    @property
    def path(self) -> str:
        return self.file.path + ":" + str(self.order_key)

    objects: StatementManager = StatementManager()

    class Meta:
        ordering = ["order_key"]
        default_manager_name = "objects"
        constraints = [
            # ck is unique per bench version
            models.UniqueConstraint(
                fields=["bench_version", "ck"],
                name="bench_statement_bench_version_ck_ak",
                condition=models.Q(deleted_at__isnull=True),
            ),
            # check that order key is unique within parent/file (if not "deleted")
            models.UniqueConstraint(
                fields=["file", "order_key"],
                name="bench_statement_file_order_key_ak",
                condition=models.Q(parent_statement__isnull=True, deleted_at__isnull=True),
            ),
            models.UniqueConstraint(
                fields=["parent_statement", "order_key"],
                name="bench_statement_parent_order_key_ak",
                condition=models.Q(parent_statement__isnull=False, deleted_at__isnull=True),
            ),
        ]
