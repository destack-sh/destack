from __future__ import annotations

import uuid
from datetime import datetime
from typing import TYPE_CHECKING, Optional, Union
from uuid import UUID

import structlog
from django.db import models
from django.db.models import Q
from django.db.models.expressions import RawSQL

from bench.language import StatementType, TypeHint, TypeTag, wire
from bench.language.const import ScheduleType, TriggerType, TypeFlag
from bench.models.utils import NAME_VALIDATOR, CrudNode, create_models_bfs, get_choices
from bench.utils.dt import utcnow_with_tz
from bench.utils.uuidt import MAX_NAME_LENGTH

if TYPE_CHECKING:
    from bench.models import File, ProjectVersion
    from bench.models.packer import _PackedCopy

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
    metadata = models.JSONField(null=True, blank=True)
    description = models.TextField(null=True, blank=True)
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

    def soft_delete(self):
        self.deleted_at = utcnow_with_tz()

    def restore(self):
        self.deleted_at = None

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
    A trigger to a runnable.
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
    runnable_ck = models.UUIDField(null=True, blank=True)
    scope_ck = models.UUIDField(null=True, blank=True)
    # internal
    processed_up_to = models.DateTimeField(null=True, blank=True)

    @property
    def parent_id(self) -> Optional[uuid.UUID]:
        return self.statement_id

    @property
    def parent(self) -> Statement:
        return self.statement

    def soft_delete(self):
        self.deleted_at = utcnow_with_tz()

    def restore(self):
        self.deleted_at = None


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
    metadata = models.JSONField(null=True, blank=True)

    @property
    def parent_id(self):
        return self.statement_id

    @property
    def parent(self):
        return self.statement

    def soft_delete(self):
        self.deleted_at = utcnow_with_tz()

    def restore(self):
        self.deleted_at = None


class TileManager(models.Manager["Tile"]):
    def get_queryset(self):
        # soft-deleted statements are not returned by default
        return super().get_queryset().filter(deleted_at__isnull=True)


class Tile(CrudNode):
    """
    An element on a screen statement (not used yet)
    """

    project_version = models.ForeignKey(
        "ProjectVersion", on_delete=models.CASCADE, related_name="tiles"
    )
    statement = models.ForeignKey("Statement", on_delete=models.CASCADE, related_name="tiles")
    name = models.CharField(max_length=MAX_NAME_LENGTH, blank=True)
    parent_tile = models.ForeignKey(
        "Tile", on_delete=models.CASCADE, related_name="children", null=True, blank=True
    )
    order_key = models.CharField(max_length=64)  # in parent
    x = models.IntegerField(null=True, blank=True)
    y = models.IntegerField(null=True, blank=True)

    children: models.QuerySet[Tile]  # noqa via Tile.parent


def duplicate_versioned_datasets(
    *,
    source: ProjectVersion,
    target: ProjectVersion,
    copy: _PackedCopy,
    keep_cks: bool,
) -> None:
    """
    Duplicates the *versioned* datasets amongst the old statements.
    If we're keeping cks, we only replace the ids. Otherwise, both ids and cks are replaced.
    """
    from bench.opensearch.index import batch_duplicate_records

    duplicate_target_ids = {}
    duplicate_target_cks = {}
    for statement in copy.nodes.values():
        if not isinstance(statement, wire.DatasetData) or not statement.versioned:
            continue
        source_id = copy.target_ids_reversed[statement.id]
        duplicate_target_ids[source_id] = statement.id
        source_ck = copy.target_cks_reversed[statement.ck]
        duplicate_target_cks[source_ck] = statement.ck

    if duplicate_target_ids or duplicate_target_cks:
        batch_duplicate_records(
            source_project_v=source,
            target_project_v=target,
            new_statement_ids=duplicate_target_ids,
            new_statement_cks=duplicate_target_cks,
            keep_cks=keep_cks,
        )


class StatementManager(models.Manager["Statement"]):
    def get_queryset(self) -> models.QuerySet[Statement]:
        # soft-deleted statements are not returned by default
        return super().get_queryset().filter(deleted_at__isnull=True)

    def copy(
        self,
        statements: models.QuerySet[Statement],
        source: ProjectVersion,
        target: ProjectVersion,
        keep_cks: bool,
        target_ids: dict[UUID, UUID] | None = None,
        target_cks: dict[UUID, UUID] | None = None,
        target_parent_ids: dict[UUID, UUID] | None = None,
        target_order_keys: dict[UUID, str] | None = None,
        copy_revisions: bool = True,
    ) -> None:
        """Copies the given source statements into the target version in given new files"""

        from bench.models import File, ProjectVersion, packer

        # pack relevant nodes
        copy = ProjectVersion.objects.pack_copy(
            source=source,
            target=target,
            nodes=list(statements),
            keep_cks=keep_cks,
            target_ids=target_ids,
            target_cks=target_cks,
            copy_revisions=copy_revisions,
        )
        for node in copy.nodes.values():  # patch parent and order keys
            if node.id in target_parent_ids:
                node.parent_id = target_parent_ids[node.id]
            elif node.parent_id in target_ids:
                node.parent_id = copy.target_ids[node.parent_id]
            if isinstance(node, wire.HasOrder):
                node.order_key = target_order_keys.get(node.id, node.order_key)

        # collect parents at target (not part of the packed tree since they're the destination)
        # assumes parents can only be File or Statement (will error below if parent is missing)
        target_parents = [
            *self.filter(id__in=target_parent_ids.values()),
            *File.objects.filter(id__in=target_parent_ids.values()),
        ]
        # unpack and save
        unpacked = packer.unpack_nodes_tree(
            copy.nodes_list(),
            pre_unpacked={target.id: target, **{p.id: p for p in target_parents}},
        )
        create_models_bfs(unpacked.walk_bfs_batched())
        duplicate_versioned_datasets(source=source, target=target, copy=copy, keep_cks=keep_cks)

    def get_descendants(
        self, statement_ids: list[UUID], deleted_at: Optional[datetime] = None
    ) -> models.QuerySet[Statement]:
        """Gets descendants of statements with given ids (including the statements themselves)."""
        query = """
           WITH RECURSIVE descendants(id, parent_statement_id) AS (
               SELECT id, parent_statement_id
               FROM bench_statement
               WHERE id = ANY(%s)
               UNION ALL
               SELECT bench_statement.id, bench_statement.parent_statement_id
               FROM bench_statement
               INNER JOIN descendants ON descendants.id = bench_statement.parent_statement_id
           )
           SELECT DISTINCT id
           FROM descendants
        """
        return Statement._base_manager.filter(
            id__in=RawSQL(query, (statement_ids,)), deleted_at=deleted_at
        )


class Statement(CrudNode):
    """
    A nested statement in a file for working with Bench symbols and other stuff.
    """

    project_version = models.ForeignKey(
        "ProjectVersion", on_delete=models.CASCADE, related_name="statements"
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
    description = models.TextField(null=True, blank=True)
    key = models.CharField(max_length=48, null=True, blank=True)
    root_type_tag = models.CharField(
        max_length=32, choices=get_choices(TypeTag), null=True, blank=True
    )
    root_type_flags = models.IntegerField(null=True, blank=True)
    lang = models.CharField(max_length=32, null=True, blank=True)
    text = models.TextField(null=True, blank=True)
    code = models.TextField(null=True, blank=True)
    value = models.JSONField(null=True, blank=True)
    external_name = models.CharField(max_length=128, null=True, blank=True)
    reference_ck = models.UUIDField(null=True, blank=True)
    fields: models.QuerySet[Field]  # noqa via Field.statement
    taggings: models.QuerySet[Tagging]  # noqa via Tagging.statement
    triggers: models.QuerySet[Trigger]  # noqa via Trigger.statement
    # interp state
    issues: models.QuerySet["Issue"]  # noqa via Issue.statement
    resolved_fields: models.QuerySet["ResolvedField"]  # noqa via ResolvedField.statement

    def __str__(self):
        return f"{self.path} {self.type} {self.name}"

    @property
    def descendants(self) -> models.QuerySet[Statement]:
        return Statement.objects.get_descendants([self.id])

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

    def soft_delete(self):
        self.deleted_at = utcnow_with_tz()
        # soft delete descendants (that aren't yet deleted)
        self.descendants.filter(deleted_at=None).update(deleted_at=self.deleted_at)

    def restore(self):
        # restore descendants (that were deleted at the same time)
        self.descendants.filter(deleted_at=self.deleted_at).update(deleted_at=None)
        self.deleted_at = None

    objects: StatementManager = StatementManager()

    class Meta:
        ordering = ["order_key"]
        default_manager_name = "objects"
        constraints = [
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
