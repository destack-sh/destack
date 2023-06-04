from __future__ import annotations

from datetime import datetime
from typing import TYPE_CHECKING, Optional
from uuid import UUID, uuid4

import pytz
import structlog
from django.db import models
from django.db.models import Q
from django.db.models.expressions import RawSQL
from django_choices_field import TextChoicesField
from strawberry_django_plus import gql

from bench.language.type import (
    FIELD_KEY_LENGTH,
    StatementModifier,
    StatementType,
    SymbolType,
    TypeFlag,
    TypeHint,
    TypeTag,
    new_field_key,
)
from bench.models.data import Record
from bench.models.utils import NAME_VALIDATOR, CrudModel, UUIDModel, walk_children_bfs_batched
from bench.utils.uuidt import MAX_NAME_LENGTH

if TYPE_CHECKING:
    from bench.models import File, ProjectVersion, RefMapping

logger = structlog.get_logger(__name__)


class FieldManager(models.Manager["Field"]):
    def get_queryset(self):
        # soft-deleted statements are not returned by default
        return super().get_queryset().select_related("statement")


class Field(UUIDModel, CrudModel):
    """
    A (usually) named type of something.
    """

    statement = models.ForeignKey("Statement", on_delete=models.CASCADE, related_name="fields")
    name = models.CharField(
        max_length=MAX_NAME_LENGTH, null=True, blank=True, validators=[NAME_VALIDATOR]
    )
    key = models.CharField(max_length=FIELD_KEY_LENGTH, default=new_field_key)
    order_key = models.CharField(max_length=MAX_NAME_LENGTH)
    tag = TextChoicesField(choices_enum=TypeTag)
    hint = TextChoicesField(choices_enum=TypeHint, null=True, blank=True)
    flags = models.IntegerField(default=0)
    description = models.TextField(null=True, blank=True)
    value = models.JSONField(null=True, blank=True)
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
        return f"{self.statement} {name_str}{self.tag.value}{flags_str}"

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
        # TODO @Performance: copy statements server-side (in SQL)

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
        new_statements: dict[UUID, Statement] = {}
        new_fields: dict[UUID, Field] = {}
        new_records: dict[UUID, Record] = {}

        # copy statements
        for statement in statements:
            if statement.type == StatementType.DEFINITION:
                # the relations are saved below after statement creation
                # copy type nodes
                if statement.root_type_tag is not None:
                    for field in statement.fields.all():
                        old_id = field.id
                        old_revision = field.revision
                        field.id = uuid4()
                        field._state.adding = True
                        field.statement_id = target_statement_ids[statement.id]
                        if field.reference_id is not None:
                            # replace type node reference if it was copied (default to same for externals)
                            field.reference_id = target_statement_ids.get(
                                field.reference_id, field.reference_id
                            )
                        new_fields[old_id] = field
                        _refmap(RefType.FIELD, old_id, old_revision, field)

                # copy records (obviously very inefficient)
                if statement.symbol_type == SymbolType.DATA:
                    for record in statement.records.all():
                        old_id = record.id
                        old_revision = record.revision
                        record.id = uuid4()
                        record._state.adding = True
                        record.statement_id = target_statement_ids[statement.id]
                        new_records[old_id] = record
                        _refmap(RefType.RECORD, old_id, old_revision, record)

            # copy statement
            # automatically copies all non-relational columns
            old_id = statement.id
            old_revision = statement.revision
            statement.id = target_statement_ids[old_id]
            statement._state.adding = True
            if statement.parent_id is not None:
                if statement.parent_id not in target_statement_ids:
                    # TODO @Robustness: ensure orphaned statements are impossible
                    logger.warning("statement_lost_parent", statement=statement)
                    continue
                statement.parent_id = target_parent_ids.get(
                    statement.id,
                    target_statement_ids[statement.parent_id],
                )
            statement.order_key = target_order_keys.get(statement.id, statement.order_key)
            statement.deleted_at = None  # restore in copy if it was deleted
            statement.revision = old_revision if copy_revisions else 0
            statement.file = target_files[statement.file_id]
            statement.project_version = target_version
            statement.reference = None
            new_statements[old_id] = statement
            _refmap(RefType.STATEMENT, old_id, old_revision, statement)

        # create statements BFS, starting at roots that are _within_ selection (may not be actual roots)
        for new_statements_batch in walk_children_bfs_batched(new_statements.values(), "parent_id"):
            Statement.objects.bulk_create(new_statements_batch)

        # re-assign references (can't be part of bfs walk)
        for old in statements.only("id", "reference_id"):
            if old.id not in new_statements:
                # skip ghost statement whose parent was deleted or lost somehow
                # TODO @Robustness: fix/prevent ghost orphan statements on insert
                continue
            new = new_statements[old.id]
            # replace ref (default to same ref if not in refs since library refs are not copied)
            new.reference_id = target_statement_ids.get(old.reference_id, old.reference_id)
        Statement.objects.bulk_update(new_statements.values(), ["reference_id"])

        # create referencing statement's relations (FKs to statements)
        Field.objects.bulk_create(new_fields.values())
        Record.objects.bulk_create(new_records.values())

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


class Statement(UUIDModel, CrudModel):
    """
    A nested statement in a file for working with Bench symbols and other stuff.
    """

    project_version = models.ForeignKey(
        "ProjectVersion", on_delete=models.CASCADE, related_name="statements"
    )
    file = models.ForeignKey("File", on_delete=models.CASCADE, related_name="statements")
    type = TextChoicesField(choices_enum=StatementType)
    modifier = TextChoicesField(choices_enum=StatementModifier, null=True, blank=True)
    name = models.CharField(
        max_length=MAX_NAME_LENGTH, null=True, blank=True, validators=[NAME_VALIDATOR]
    )
    commented = models.BooleanField(default=False)
    generated = models.BooleanField(default=False)

    parent = models.ForeignKey(
        "Statement", on_delete=models.CASCADE, null=True, blank=True, related_name="children"
    )
    children: models.QuerySet[Statement]  # noqa via Statement.parent
    order_key = models.CharField(max_length=64)  # in file/parent

    symbol_type = TextChoicesField(choices_enum=SymbolType, null=True, blank=True)
    reference = models.ForeignKey(
        "Statement",
        on_delete=models.SET_NULL,
        null=True,
        blank=True,
        related_name="referenced_by",
    )
    reference_id: Optional[UUID]  # noqa via Statement.reference
    referenced_by: models.QuerySet[Statement]  # noqa via Statement.reference
    # symbol contents
    records: models.QuerySet["Record"]  # noqa via Record.statement
    root_type_tag = TextChoicesField(choices_enum=TypeTag, null=True, blank=True)
    root_type_flags = models.IntegerField(null=True, blank=True)
    fields: models.QuerySet[Field]  # noqa via Field.statement
    lang = models.CharField(max_length=32, null=True, blank=True)
    code = models.TextField(null=True, blank=True)
    description = models.TextField(null=True, blank=True)
    reference_project_version = models.ForeignKey(  # for requirement
        "ProjectVersion", on_delete=models.SET_NULL, null=True, blank=True
    )
    external_name = models.CharField(max_length=128, null=True, blank=True)  # for model
    # interp state
    issues: models.QuerySet["Issue"]  # noqa via Issue.statement
    resolved_fields = models.ManyToManyField("Field", related_name="+", through="ResolvedField")

    def __str__(self):
        if self.type == StatementType.DEFINITION:
            content_str = "()"  # should have some nice __str__ here
        elif self.type in (StatementType.IMPORT, StatementType.REFERENCE):
            content_str = f"{self.reference}"
        elif self.type == StatementType.COMMENT:
            content_str = ""
        elif self.type == StatementType.BLANK:
            content_str = ""
        else:
            raise ValueError(f"unknown statement type {self.type}")
        modifier_str = f" {self.modifier}" if self.modifier else ""
        return f"{self.path}{modifier_str} {self.type} {self.symbol_type} {self.name} {content_str}"

    @property
    def descendants(self) -> models.QuerySet[Statement]:
        return Statement.objects.get_descendants([self.id])

    @property
    def path(self) -> str:
        return self.file.path + ":" + str(self.order_key)

    @gql.model_property(only=["type", "reference"], select_related=["reference"])
    def source_definition(self) -> Statement:
        """Traverses references to get the source definition."""
        if self.type == StatementType.DEFINITION:
            return self
        elif self.reference is None:
            raise ValueError(f"{self} has no reference")
        else:
            return self.reference.source_definition

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
        # no constraint on contents since statements may be partially defined
        #  (during creation, editing and after reference deletion)
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
            # if reference is set symbol type must also be set
            models.CheckConstraint(
                check=models.Q(reference__isnull=True) | models.Q(symbol_type__isnull=False),
                name="bench_statement_reference_symbol_type_set",
            ),
        ]
