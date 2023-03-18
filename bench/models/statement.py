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

from bench.language.type import StatementModifier, StatementType, SymbolType, TypeTag
from bench.models.data import DatasetContentMixin, DatasetRecord
from bench.models.generated import GeneratedContentMixin, GeneratedMapping
from bench.models.utils import UUIDModel, walk_children_bfs_batched
from bench.utils.uuidt import MAX_NAME_LENGTH

if TYPE_CHECKING:
    from bench.models import File, ProjectVersion, RefMapping

logger = structlog.get_logger(__name__)


class SimpleTypeNodeManager(models.Manager["SimpleTypeNode"]):
    def get_queryset(self):
        # soft-deleted statements are not returned by default
        return super().get_queryset().select_related("statement")


class SimpleTypeNode(UUIDModel):
    """
    A simplified and interaction-optimized variant of TypeNode
    """

    statement = models.ForeignKey("Statement", on_delete=models.CASCADE, related_name="type_nodes")
    revision = models.IntegerField(default=1)
    created_at = models.DateTimeField(auto_now_add=True)
    updated_at = models.DateTimeField(auto_now=True)
    deleted_at = models.DateTimeField(null=True, blank=True)
    name = models.CharField(max_length=MAX_NAME_LENGTH, null=True, blank=True)
    order_key = models.CharField(max_length=MAX_NAME_LENGTH)
    tag = TextChoicesField(choices_enum=TypeTag)
    is_output = models.BooleanField(default=False)
    is_array = models.BooleanField(default=False)
    is_nullable = models.BooleanField(default=False)
    description = models.TextField(null=True, blank=True)
    value = models.JSONField(null=True, blank=True)
    reference = models.ForeignKey(
        "Statement",
        on_delete=models.SET_NULL,
        null=True,
        blank=True,
        related_name="type_node_references+",
    )

    def __str__(self):
        output_str = "output" if self.is_output else ""
        array_str = "array" if self.is_array else ""
        nullable_str = "nullable" if self.is_nullable else ""
        flags_str = ", ".join([f for f in [output_str, array_str, nullable_str] if f])
        flags_str = f" ({flags_str})" if flags_str else ""
        name_str = f"{self.name} " if self.name else ""
        return f"{self.statement} {name_str}{self.tag.value}{flags_str}"

    def __repr__(self):
        return f"<SimpleTypeNode {str(self)}>"

    def soft_delete(self):
        self.deleted_at = datetime.utcnow().replace(tzinfo=pytz.utc)

    def restore(self):
        self.deleted_at = None

    objects = SimpleTypeNodeManager()

    class Meta:
        ordering = ["order_key"]
        default_manager_name = "objects"
        indexes = [models.Index(fields=["statement"])]
        constraints = [
            models.UniqueConstraint(
                fields=["statement", "order_key"],
                name="bench_statement_type_node_order_key_ak",
                condition=Q(deleted_at__isnull=True),
            ),
        ]


class XKind(models.TextChoices):
    Static = "static"
    Input = "input"
    Output = "output"


class XSource(models.TextChoices):
    System = "system"
    User = "user"
    Developer = "developer"
    Model = "model"


class XBlock(UUIDModel):
    statement = models.ForeignKey("Statement", on_delete=models.CASCADE, related_name="xblocks")
    created_at = models.DateTimeField(auto_now_add=True)
    # not actually revisioned yet (only accessed programmatically)
    revision = models.IntegerField(default=1)
    order_key = models.CharField(max_length=MAX_NAME_LENGTH)
    kind = TextChoicesField(choices_enum=XKind)
    source = TextChoicesField(choices_enum=XSource)
    value = models.JSONField(null=True, blank=True)
    description = models.TextField(null=True, blank=True)
    settings = models.JSONField(null=True, blank=True)


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
        copy_generated_mappings: bool,
        target_statement_ids: dict[UUID, UUID] | None = None,
        target_parent_ids: dict[UUID, UUID] | None = None,
        target_order_keys: dict[UUID, str] | None = None,
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
        new_type_nodes: dict[UUID, SimpleTypeNode] = {}
        new_records: dict[UUID, DatasetRecord] = {}
        new_xblocks: dict[UUID, XBlock] = {}
        new_gen_mappings: list[GeneratedMapping] = []

        def _copy_statement(statement: Statement) -> Statement:
            """Copies a single statement and its contents (without saving)"""
            # copy statement contents/relations
            if statement.type == StatementType.DEFINITION:
                # the relations are saved below after statement creation
                # copy type nodes
                if statement.root_type_tag is not None:
                    for type_node in statement.type_nodes.all():
                        old_id = type_node.id
                        old_revision = type_node.revision
                        type_node.id = uuid4()
                        type_node._state.adding = True
                        type_node.statement_id = target_statement_ids[statement.id]
                        if type_node.reference_id is not None:
                            # replace type node reference if it was copied (default to same for externals)
                            type_node.reference_id = target_statement_ids.get(
                                type_node.reference_id, type_node.reference_id
                            )
                        new_type_nodes[old_id] = type_node
                        _refmap(RefType.TYPE_NODE, old_id, old_revision, type_node)

                # copy records
                if statement.symbol_type == SymbolType.DATA:
                    for record in statement.records.all():
                        old_id = record.id
                        old_revision = record.revision
                        record.id = uuid4()
                        record._state.adding = True
                        record.statement_id = target_statement_ids[statement.id]
                        new_records[old_id] = record
                        _refmap(RefType.RECORD, old_id, old_revision, record)

                # copy xblocks
                elif statement.symbol_type == SymbolType.CODE:
                    for xblock in statement.xblocks.all():
                        old_id = xblock.id
                        old_revision = xblock.revision
                        xblock.id = uuid4()
                        xblock._state.adding = True
                        xblock.statement_id = target_statement_ids[statement.id]
                        new_xblocks[old_id] = xblock
                        _refmap(RefType.XBLOCK, old_id, old_revision, xblock)

                # copy generated mappings (only Builds can have them right now)
                elif statement.symbol_type == SymbolType.BUILD and copy_generated_mappings:
                    for mapping in statement.generated_mappings.all():
                        mapping.pk = None
                        mapping.statement_id = target_statement_ids[mapping.statement_id]
                        mapping.source_id = ref_mappings_ids.get(
                            mapping.source_id, mapping.source_id
                        )
                        mapping.target_id = ref_mappings_ids.get(
                            mapping.target_id, mapping.target_id
                        )
                        mapping.source_revision = 0
                        mapping.target_revision = 0
                        new_gen_mappings.append(mapping)

            # copy statement
            # automatically copies all non-relational columns
            old_id = statement.id
            old_revision = statement.revision
            statement.id = target_statement_ids[old_id]
            statement._state.adding = True
            statement.parent_id = target_parent_ids.get(
                statement.id, target_statement_ids.get(statement.parent_id)
            )
            statement.order_key = target_order_keys.get(statement.id, statement.order_key)
            statement.deleted_at = None  # restore in copy if it was deleted
            statement.revision = 0  # reset revision
            statement.file = target_files[statement.file_id]
            statement.project_version = target_version
            statement.reference = None
            new_statements[old_id] = statement
            _refmap(RefType.STATEMENT, old_id, old_revision, statement)
            return statement

        # walk statements BFS, starting at roots that are _within_ selection (may not be actual roots)
        for old_statements_batch in walk_children_bfs_batched(list(statements), "parent_id"):
            new_statements_batch = []
            for old_statement in old_statements_batch:
                new_statement = _copy_statement(old_statement)
                new_statements_batch.append(new_statement)
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

        # save statement's relations
        SimpleTypeNode.objects.bulk_create(new_type_nodes.values())
        DatasetRecord.objects.bulk_create(new_records.values())
        XBlock.objects.bulk_create(new_xblocks.values())
        GeneratedMapping.objects.bulk_create(new_gen_mappings)

        return ref_mappings

    def get_descendants(self, statement_ids: list[UUID]) -> models.QuerySet[Statement]:
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
           SELECT id
           FROM descendants
        """
        return Statement._base_manager.filter(id__in=RawSQL(query, (statement_ids,)))


class Statement(UUIDModel, DatasetContentMixin, GeneratedContentMixin):
    """
    A statement in a file to import, define, redefine, reference, comment.. symbols.
    Statements are semantic and may be nested (parent-child relationships, comments, etc.).
    Statements and the files that contain them can be soft-deleted.
    """

    project_version = models.ForeignKey(
        "ProjectVersion", on_delete=models.CASCADE, related_name="statements"
    )
    file = models.ForeignKey("File", on_delete=models.CASCADE, related_name="statements")
    revision = models.IntegerField(default=1)
    type = TextChoicesField(choices_enum=StatementType)
    modifier = TextChoicesField(choices_enum=StatementModifier, null=True, blank=True)
    name = models.CharField(max_length=MAX_NAME_LENGTH, null=True, blank=True)
    created_at = models.DateTimeField(auto_now_add=True)
    updated_at = models.DateTimeField(auto_now=True)
    deleted_at = models.DateTimeField(null=True, blank=True)
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
    # symbol contents (sync with SYMBOL_CONTENT_*_FIELDS above)
    xblocks: models.QuerySet[XBlock]  # noqa via XBlock.statement
    root_type_tag = TextChoicesField(choices_enum=TypeTag, null=True, blank=True)
    type_nodes: models.QuerySet[SimpleTypeNode]  # noqa via SimpleTypeNode.statement
    lang = models.CharField(max_length=32, null=True, blank=True)
    code = models.TextField(null=True, blank=True)
    description = models.TextField(null=True, blank=True)  # for any descriptions
    reference_project_version = models.ForeignKey(  # for requirement
        "ProjectVersion", on_delete=models.SET_NULL, null=True, blank=True
    )
    value = models.JSONField(null=True, blank=True)  # for value
    on = models.TextField(null=True, blank=True)  # for expect-likes
    external_name = models.CharField(max_length=128, null=True, blank=True)  # for model
    provider = models.CharField(max_length=64, null=True, blank=True)  # for model

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
