from __future__ import annotations

import contextlib
from datetime import datetime
from typing import TYPE_CHECKING, Optional
from uuid import UUID

import pytz
import structlog
from django.db import models, transaction
from django.db.models import Q
from django_choices_field import TextChoicesField
from strawberry_django_plus import gql

from bench.language import wire
from bench.language.type import (
    EMPTY_FUNC_TYPE,
    EMPTY_STRUCT_TYPE,
    StatementModifier,
    StatementType,
    SymbolType,
)
from bench.language.wire import rmap_type_node
from bench.models.compile import CompilationContentMixin
from bench.models.data import DatasetContentMixin
from bench.models.utils import MAX_NAME_LENGTH, UUIDModel
from bench.zmq.serialize import from_dict, to_dict

if TYPE_CHECKING:
    from bench.models import File, ProjectVersion

logger = structlog.get_logger(__name__)


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


# Django field for our wire.TypeNodeData dataclass
class TypeNodesDataField(models.JSONField):
    def get_default(self):
        return []

    def from_db_value(self, value, expression, connection):
        if value is None:
            return None
        value = super().from_db_value(value, expression, connection)
        return [from_dict(wire.TypeNodeData, val) for val in value]

    def to_python(self, value):
        if value is None:
            return None
        return [from_dict(wire.TypeNodeData, val) for val in value]

    def get_prep_value(self, value):
        if value is None:
            return None
        return super().get_prep_value(to_dict(value, omit_empty=True))


# sync with actual symbol content fields of Statement
SYMBOL_CONTENT_VALUE_FIELDS = (
    "language",
    "code",
    "code_builtin_id",
    "description",
    "value",
    "type_nodes",
)
SYMBOL_CONTENT_RELATION_1TOM_FIELDS = ("reference_project_version",)
SYMBOL_CONTENT_RELATION_MTOM_FIELDS = (
    "mappings",
    "compilations",
)


class Statement(UUIDModel, DatasetContentMixin, CompilationContentMixin):
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
    order_key = models.CharField(max_length=64, null=True, blank=True)  # in file/parent

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
    text = models.TextField(null=True, blank=True)  # for comment
    # symbol contents (sync with SYMBOL_CONTENT_*_FIELDS above)
    lang = models.CharField(max_length=32, null=True, blank=True)
    code = models.TextField(null=True, blank=True)
    code_builtin_id = models.CharField(max_length=64, null=True, blank=True)
    description = models.TextField(null=True, blank=True)  # for any descriptions
    reference_project_version = models.ForeignKey(  # for requirement
        "ProjectVersion", on_delete=models.SET_NULL, null=True, blank=True
    )
    value = models.JSONField(null=True, blank=True)  # for value
    type_nodes = TypeNodesDataField(null=True, blank=True)
    on = models.TextField(null=True, blank=True)  # for expect-likes
    external_name = models.CharField(max_length=128, null=True, blank=True)  # for model
    provider = models.CharField(max_length=64, null=True, blank=True)  # for model

    def __str__(self):
        path = self.file.path + ":" + str(self.order_key)
        if self.type == StatementType.DEFINITION:
            content_str = "()"  # should have some nice __str__ here
        elif self.type in (StatementType.IMPORT, StatementType.REFERENCE):
            content_str = f"{self.reference}"
        elif self.type == StatementType.COMMENT:
            content_str = f"{len(self.text)}"
        elif self.type == StatementType.BLANK:
            content_str = ""
        else:
            raise ValueError(f"unknown statement type {self.type}")
        modifier_str = f" {self.modifier}" if self.modifier else ""
        return f"{path}{modifier_str} {self.type} {self.symbol_type} {self.name} {content_str}"

    @contextlib.contextmanager
    def edit(self):
        """Edits a statement and saves it with a new revision when done"""
        with transaction.atomic():
            yield self
            self.revision += 1
            self.save()

    @property
    def siblings(self) -> models.QuerySet[Statement]:
        return self.parent.children if self.parent else self.file.root_statements

    @property
    def descendants(self) -> models.QuerySet[Statement]:
        # TODO @Broken: get all descendants, not just children of children (recursive)
        return Statement._base_manager.filter(
            Q(parent=self) | Q(parent__parent=self) | Q(parent__parent__parent=self)
        )

    @gql.model_property(only=["type", "reference"], select_related=["reference"])
    def source_definition(self) -> Statement:
        """Traverses references to get the source definition."""
        if self.type == StatementType.DEFINITION:
            return self
        elif self.reference is None:
            raise ValueError(f"{self} has no reference")
        else:
            return self.reference.source_definition

    def morph_to(
        self,
        type: StatementType,
        symbol_type: SymbolType | None,
    ):
        """
        Changes the type of the statement (new default content fields may overwrite old ones).
        Since this overwrites overlapping fields, we only want to morph from blank statements to
        preserve undo-ability. As this only applies in the UI, we don't check it here.
        """

        self.type = type
        # set default content if not already set
        self.symbol_type = symbol_type
        # create default content for the given type if not already set
        if symbol_type is not None and self.type == StatementType.DEFINITION:
            self.description = ""
            if symbol_type == SymbolType.DATASET:
                self.language = "jsonl"
                self.type_nodes = rmap_type_node(EMPTY_STRUCT_TYPE)
            elif symbol_type == SymbolType.CODE or symbol_type == SymbolType.TASK:
                self.language = "python"
                self.type_nodes = rmap_type_node(EMPTY_FUNC_TYPE)
                self.code = ""
            elif symbol_type == SymbolType.TYPE:
                self.type_nodes = rmap_type_node(EMPTY_STRUCT_TYPE)
        self.save()

    def soft_delete(self):
        self.deleted_at = datetime.utcnow().replace(tzinfo=pytz.utc)
        # soft delete descendants (that aren't yet deleted)
        self.descendants.filter(deleted_at=None).update(deleted_at=self.deleted_at)
        self.save()

    def restore(self):
        self.refresh_from_db(fields=["deleted_at", "file"])
        if self.file.deleted_at:
            raise ValueError(f"cannot restore {self} because containing {self.file} is deleted")
        # restore descendants (that were deleted at the same time)
        self.descendants.filter(deleted_at=self.deleted_at).update(deleted_at=None)
        self.deleted_at = None
        self.save()

    def set_commented(self, commented: bool):
        """Sets the commented flag on this statement and all descendants."""
        self.commented = commented
        self.descendants.update(commented=commented)
        self.save()

    objects: StatementManager = StatementManager()

    class Meta:
        ordering = ["order_key"]
        default_manager_name = "objects"
        # no constraint on contents since statements may be partially defined
        #  (during creation, editing and after reference deletion)
        constraints = [
            # if reference is set symbol type must also be set
            models.CheckConstraint(
                check=models.Q(reference__isnull=True) | models.Q(symbol_type__isnull=False),
                name="bench_statement_reference_symbol_type_set",
            ),
            # if reference is set name must also be set
            models.CheckConstraint(
                check=models.Q(reference__isnull=True) | models.Q(name__isnull=False),
                name="bench_statement_reference_name_set",
            ),
        ]
