from __future__ import annotations

from datetime import datetime
from typing import TYPE_CHECKING, Optional
from uuid import UUID, uuid4

import pytz
import structlog
from django.db import models
from django.db.models import Q
from django.db.models.expressions import RawSQL

from bench.bench import ExpectationModifier, StatementType, TypeHint, TypeTag
from bench.bench.const import FIELD_KEY_LENGTH, TypeFlag
from bench.bench.type import new_field_key
from bench.models.utils import NAME_VALIDATOR, CrudModel, UUIDModel, get_choices, Revisioned
from bench.utils.uuidt import MAX_NAME_LENGTH

if TYPE_CHECKING:
    from bench.models import File, ProjectVersion, RefMapping

logger = structlog.get_logger(__name__)


class FieldManager(models.Manager["Field"]):
    def get_queryset(self):
        # soft-deleted statements are not returned by default
        return super().get_queryset().select_related("statement")


class Field(UUIDModel, CrudModel, Revisioned):
    """
    A (usually) named type of something.
    Do not write to this model directly as any change affects the opensearch indices.
    """

    statement = models.ForeignKey("Statement", on_delete=models.CASCADE, related_name="fields")
    name = models.CharField(
        max_length=MAX_NAME_LENGTH, null=True, blank=True, validators=[NAME_VALIDATOR]
    )
    key = models.CharField(max_length=FIELD_KEY_LENGTH, default=new_field_key)
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
        output_str = "output" if self.flags & TypeFlag.IsOutput else ""
        array_str = "array" if self.flags & TypeFlag.IsArray else ""
        nullable_str = "nullable" if self.flags & TypeFlag.IsNullable else ""
        unioned_str = "unioned" if self.flags & TypeFlag.IsUnionWith else ""
        flags_str = ", ".join([f for f in [output_str, array_str, nullable_str, unioned_str] if f])
        flags_str = f" ({flags_str})" if flags_str else ""
        name_str = f"{self.name} " if self.name else ""
        return f"{self.statement} {name_str}{self.tag}{flags_str}"

    def __repr__(self):
        return f"<Field {str(self)}>"

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


class StatementManager(models.Manager["Statement"]):
    def get_queryset(self) -> models.QuerySet[Statement]:
        # soft-deleted statements are not returned by default
        return super().get_queryset().filter(deleted_at__isnull=True)

    def create_statement(
        self,
        project_version: ProjectVersion,
        file: File,
        parent: Optional[Statement],
        order_key: Optional[str],
        type: StatementType,
        name: Optional[str],
        **kwargs,
    ) -> Statement:
        if order_key is None:
            # set order key to the end of siblings (parent/file children)
            raise NotImplementedError("auto order key not implemented yet")
        return self.create(
            project_version=project_version,
            file=file,
            parent=parent,
            order_key=order_key,
            type=type,
            name=name,
            **kwargs,
        )

    def copy_statements(
        self,
        statements: models.QuerySet[Statement],
        target_files: dict[UUID, File],
        source_version: ProjectVersion,
        target_version: ProjectVersion,
        target_statement_ids: dict[UUID, UUID] | None = None,
        target_parent_ids: dict[UUID, UUID] | None = None,
        target_order_keys: dict[UUID, str] | None = None,
        copy_revisions: bool = True,
    ) -> list["RefMapping"]:
        """Copies the given source statements into the target version in given new files"""

        from bench.models import RefMapping, RefType  # avoid circular import

        ref_mappings: list[RefMapping] = []
        ref_mappings_ids: dict[UUID, UUID] = {}

        def _refmap(type: RefType, old_id: UUID, old_revision: int, new: models.Model):
            ref_mapping = RefMapping(
                type=type,
                source_version=source_version,
                target_version=target_version,
                source_id=old_id,
                target_id=new.id,
                source_revision=old_revision,
                target_revision=new.revision,
            )
            ref_mappings_ids[old_id] = ref_mapping.id
            ref_mappings.append(ref_mapping)

        # (pre-determine new statement ids to re-create source mappings in one go)
        target_statement_ids = target_statement_ids or {
            statement.id: uuid4() for statement in statements
        }
        target_parent_ids = target_parent_ids or {}
        target_order_keys = target_order_keys or {}

        # nocheckin: replace copy files/statements with packer-based copy
        # nocheckin: also copy datasets if versioned
        raise NotImplementedError  # nocheckin

        return ref_mappings

    def get_descendants(
        self, statement_ids: list[UUID], deleted_at: Optional[datetime] = None
    ) -> models.QuerySet[Statement]:
        """Gets descendants of statements with given ids (including the statements themselves)."""
        query = """
           WITH RECURSIVE descendants(id, parent_id) AS (
               SELECT id, parent_id
               FROM bench_statement
               WHERE id = ANY(%s)
               UNION ALL
               SELECT bench_statement.id, bench_statement.parent_id
               FROM bench_statement
               INNER JOIN descendants ON descendants.id = bench_statement.parent_id
           )
           SELECT DISTINCT id
           FROM descendants
        """
        return Statement._base_manager.filter(
            id__in=RawSQL(query, (statement_ids,)), deleted_at=deleted_at
        )


class Statement(UUIDModel, CrudModel, Revisioned):
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
    commented = models.BooleanField(default=False)

    parent = models.ForeignKey(
        "Statement", on_delete=models.CASCADE, null=True, blank=True, related_name="children"
    )
    children: models.QuerySet[Statement]  # noqa via Statement.parent
    order_key = models.CharField(max_length=64)  # in file/parent

    # symbol
    modifier = models.CharField(
        max_length=32, choices=get_choices(ExpectationModifier), null=True, blank=True
    )
    reference = models.ForeignKey(
        "Statement", on_delete=models.SET_NULL, null=True, blank=True, related_name="references+"
    )
    description = models.TextField(null=True, blank=True)
    root_type_tag = models.CharField(
        max_length=32, choices=get_choices(TypeTag), null=True, blank=True
    )
    root_type_flags = models.IntegerField(null=True, blank=True)
    lang = models.CharField(max_length=32, null=True, blank=True)
    text = models.TextField(null=True, blank=True)
    code = models.TextField(null=True, blank=True)
    value = models.JSONField(null=True, blank=True)
    reference_project_version = models.ForeignKey(  # for requirement
        "ProjectVersion", on_delete=models.SET_NULL, null=True, blank=True
    )
    external_name = models.CharField(max_length=128, null=True, blank=True)  # for model
    dataset = models.OneToOneField("Dataset", on_delete=models.SET_NULL, null=True, blank=True)
    fields: models.QuerySet[Field]  # noqa via Field.statement
    # interp state
    issues: models.QuerySet["Issue"]  # noqa via Issue.statement
    resolved_fields = models.ManyToManyField("Field", related_name="+", through="ResolvedField")

    def __str__(self):
        modifier_str = f" {self.modifier}" if self.modifier else ""
        return f"{self.path}{modifier_str} {self.type} {self.name}"

    @property
    def descendants(self) -> models.QuerySet[Statement]:
        return Statement.objects.get_descendants([self.id])

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

    def set_commented(self, commented: bool):
        """Sets the commented flag on this statement and all descendants."""
        self.commented = commented
        self.descendants.update(commented=commented)

    objects: StatementManager = StatementManager()

    class Meta:
        ordering = ["order_key"]
        default_manager_name = "objects"
        constraints = [
            # check that order key is unique within parent/file (if not "deleted")
            models.UniqueConstraint(
                fields=["file", "order_key"],
                name="bench_statement_file_order_key_ak",
                condition=models.Q(parent__isnull=True, deleted_at__isnull=True),
            ),
            models.UniqueConstraint(
                fields=["parent", "order_key"],
                name="bench_statement_parent_order_key_ak",
                condition=models.Q(parent__isnull=False, deleted_at__isnull=True),
            ),
        ]
