from __future__ import annotations

import uuid
from datetime import datetime
from typing import TYPE_CHECKING, Optional, Union
from uuid import UUID

import pytz
import structlog
from django.db import models
from django.db.models import Q
from django.db.models.expressions import RawSQL

from bench.bench import StatementType, TypeHint, TypeTag, wire
from bench.bench.const import DatasetBackend, TypeFlag
from bench.bench.dataset import new_dataset_backend_id
from bench.bench.tag import TAG_KEY_LENGTH, new_tag_key
from bench.bench.type import new_field_key
from bench.models.utils import (
    NAME_VALIDATOR,
    CrudModel,
    ModuleNode,
    Revisioned,
    UUIDModel,
    create_models_bfs,
    get_choices,
)
from bench.utils.uuidt import MAX_NAME_LENGTH

if TYPE_CHECKING:
    from bench.models import Dataset, File, ProjectVersion, RefMappingKind

logger = structlog.get_logger(__name__)


class FieldManager(models.Manager["Field"]):
    def get_queryset(self):
        # soft-deleted statements are not returned by default
        return super().get_queryset().select_related("statement")


class Field(UUIDModel, CrudModel, ModuleNode, Revisioned):
    """
    A (usually) named type of something.
    Do not write to this model directly as any change affects the opensearch indices.
    """

    statement = models.ForeignKey("Statement", on_delete=models.CASCADE, related_name="fields")
    name = models.CharField(
        max_length=MAX_NAME_LENGTH, null=True, blank=True, validators=[NAME_VALIDATOR]
    )
    key = models.CharField(max_length=48, default=new_field_key)
    order_key = models.CharField(max_length=MAX_NAME_LENGTH)
    tag = models.CharField(max_length=20, choices=get_choices(TypeTag))
    hint = models.CharField(max_length=20, choices=get_choices(TypeHint), null=True, blank=True)
    flags = models.IntegerField(default=0)
    metadata = models.JSONField(null=True, blank=True)
    description = models.TextField(null=True, blank=True)
    reference = models.ForeignKey(
        "Statement", on_delete=models.SET_NULL, null=True, blank=True, related_name="+"
    )

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
        self.deleted_at = datetime.utcnow().replace(tzinfo=pytz.utc)

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


class TaggingManager(models.Manager["Tagging"]):
    def get_queryset(self) -> models.QuerySet[Tagging]:
        # soft-deleted statements are not returned by default
        return super().get_queryset().filter(deleted_at__isnull=True)


class Tagging(UUIDModel, CrudModel, ModuleNode, Revisioned):
    """
    An association between a tag and a statement.
    """

    tag = models.ForeignKey("Statement", on_delete=models.CASCADE, related_name="+")
    statement = models.ForeignKey("Statement", on_delete=models.CASCADE, related_name="tags")
    key = models.CharField(max_length=TAG_KEY_LENGTH, default=new_tag_key)
    metadata = models.JSONField(null=True, blank=True)

    def soft_delete(self):
        self.deleted_at = datetime.utcnow().replace(tzinfo=pytz.utc)

    def restore(self):
        self.deleted_at = None


class StatementManager(models.Manager["Statement"]):
    def get_queryset(self) -> models.QuerySet[Statement]:
        # soft-deleted statements are not returned by default
        return super().get_queryset().filter(deleted_at__isnull=True)

    def duplicate_datasets_inplace(
        self, source: ProjectVersion, target: ProjectVersion, datasets: list["Dataset"]
    ) -> None:
        """Duplicates the given datasets in-place to the target version"""
        from bench.models import Dataset
        from bench.opensearch.index import batch_duplicate_records

        new_dataset_ids = {d.backend_id: new_dataset_backend_id() for d in datasets}
        batch_duplicate_records(source, target, new_dataset_ids)
        for dataset in datasets:
            dataset.backend_id = new_dataset_ids[dataset.backend_id]
        Dataset.objects.bulk_update(datasets, ["backend_id"])

    def copy(
        self,
        statements: models.QuerySet[Statement],
        source: ProjectVersion,
        target: ProjectVersion,
        kind: RefMappingKind,
        target_ids: dict[UUID, UUID] | None = None,
        target_parent_ids: dict[UUID, UUID] | None = None,
        target_order_keys: dict[UUID, str] | None = None,
        copy_revisions: bool = True,
    ) -> None:
        """Copies the given source statements into the target version in given new files"""

        from bench.models import Dataset, File, ProjectVersion, RefMapping, packer

        # pack relevant nodes
        target_ids = {**(target_ids or {}), source.id: target.id}
        packed, mappings, target_ids = ProjectVersion.objects.pack_copy(
            source=source,
            target=target,
            nodes=list(statements),
            target_ids=target_ids,
            copy_revisions=copy_revisions,
            kind=kind,
        )
        for node in packed.nodes.values():  # patch parent and order keys
            if node.id in target_parent_ids:
                node.parent_id = target_parent_ids[node.id]
            elif node.parent_id in target_ids:
                node.parent_id = target_ids[node.parent_id]
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
            packed.nodes_list(),
            pre_unpacked={target.id: target, **{p.id: p for p in target_parents}},
        )
        create_models_bfs(unpacked.walk_bfs_batched())
        # duplicate versioned datasets
        versioned_datasets = [
            n for n in unpacked.nodes.values() if isinstance(n, Dataset) and n.versioned
        ]
        self.duplicate_datasets_inplace(source, target, versioned_datasets)
        # save mappings
        RefMapping.objects.bulk_create(mappings)

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


class Statement(UUIDModel, CrudModel, ModuleNode, Revisioned):
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

    # symbol data
    # TODO @Cleanup @Architecture: normalize statement data where reasonable
    reference = models.ForeignKey(
        "Statement", on_delete=models.SET_NULL, null=True, blank=True, related_name="references+"
    )
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
    dataset = models.OneToOneField("Dataset", on_delete=models.SET_NULL, null=True, blank=True)
    fields: models.QuerySet[Field]  # noqa via Field.statement
    taggings: models.QuerySet[Tagging]  # noqa via Tagging.statement
    # interp state
    issues: models.QuerySet["Issue"]  # noqa via Issue.statement
    resolved_fields = models.ManyToManyField("Field", related_name="+", through="ResolvedField")

    def __str__(self):
        return f"{self.path} {self.type} {self.name}"

    def create_symbol_if_needed(self):
        # TODO @Cleanup @Architecture: create symbol if needed shouldn't be needed
        # (currently only used in API, ideally relations should be passed in explicitly?)
        if self.type == StatementType.DATASET and self.dataset is None:
            from bench.models import Dataset

            self.dataset = Dataset.objects.create(
                id=Dataset.get_id(self),
                statement=self,
                backend=DatasetBackend.OPENSEARCH,
                backend_id=new_dataset_backend_id(),
            )

    @property
    def descendants(self) -> models.QuerySet[Statement]:
        return Statement.objects.get_descendants([self.id])

    @property
    def parent_id(self) -> Optional[uuid.UUID]:
        return self.parent_statement_id or self.file_id

    def parent(self) -> Union["Statement", "File"]:
        if self.parent_statement_id is not None:
            return self.parent_statement
        else:
            return self.file

    @property
    def path(self) -> str:
        return self.file.path + ":" + str(self.order_key)

    def soft_delete(self):
        self.deleted_at = datetime.utcnow().replace(tzinfo=pytz.utc)
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
