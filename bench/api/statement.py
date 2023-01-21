from typing import TYPE_CHECKING, Annotated, Optional
from uuid import UUID

from django import db
from django.db.models import F, Q, Value
from django.db.models.functions import Concat
from strawberry import lazy
from strawberry.scalars import JSON
from strawberry_django_plus import gql
from strawberry_django_plus.gql import auto
from strawberry_django_plus.relay import GlobalID

from bench import models
from bench.api.util import async_safe_mutation

if TYPE_CHECKING:
    from bench.api.project import File, ProjectVersion

StatementType = gql.enum(models.StatementType)
SymbolType = gql.enum(models.SymbolType)


@gql.type
class Type:
    description: Optional[str]
    btl: str


@gql.type
class SourceMapping:
    source_id: UUID
    source_path: JSON
    source_revision: int
    target_id: UUID
    target_path: JSON
    target_revision: int


@gql.django.type(models.DatasetRecord)
class DatasetRecord:
    index: int
    data: JSON


@gql.django.type(models.Statement)
class Statement(gql.relay.Node):
    project_version: Annotated["ProjectVersion", lazy(".project")]
    file: Annotated["File", lazy(".project")]
    revision: auto
    type: StatementType
    modifier: auto
    name: auto
    created_at: auto
    updated_at: auto
    deleted_at: auto
    commented: auto
    compiled: auto
    parent: Optional["Statement"]
    children: list["Statement"]
    descendants: list["Statement"]
    index: auto
    reference: Optional["Statement"]
    symbol_type: Optional[SymbolType]
    text: auto
    # symbol contents
    code: auto
    code_builtin_id: auto
    description: auto
    reference_project_version: Optional[Annotated["ProjectVersion", lazy(".project")]]
    value: auto
    btl: auto
    records: list[DatasetRecord]
    mappings: list[SourceMapping]

    @gql.field
    def import_path(self) -> Optional[str]:
        if self.type != StatementType.IMPORT:
            return None
        # TODO @Performance: statement import path should be batch loaded (DataLoader?)
        #  We try to only load the reference information required for the frontend,
        #  so we do some ugly SQL-side joins and concats.
        #  But maybe this is just the wrong approach overall, and we should just bake
        #  the import path into each project_version or even file already.
        is_local = Q(project_version_id=F("reference__project_version_id"))
        local_path = Concat(Value("."), F("reference__file__name"))
        absolute_path = Concat(
            F("reference__project_version__project__organization__slug"),
            Value("."),
            F("reference__project_version__project__slug"),
            local_path,
            output_field=db.models.CharField(),
        )
        import_path = db.models.Case(
            db.models.When(is_local, then=local_path),
            default=absolute_path,
            output_field=db.models.CharField(),
        )
        result = (
            models.Statement.objects.filter(id=self.id)
            .annotate(import_path=import_path)
            .values_list("import_path", flat=True)
            .first()
        )
        return result


@gql.input
class StatementCreateInput:
    file_id: GlobalID
    type: StatementType
    name: Optional[str] = None
    parent_id: Optional[GlobalID] = None
    index: Optional[int] = None


@gql.type
class StatementPayload:
    statement: Statement


@gql.input
class StatementMorphInput:
    statement_id: GlobalID
    type: StatementType
    symbol_type: Optional[SymbolType] = None


@gql.django.partial(models.Statement)
class StatementSetModifierInput(gql.NodeInput):
    modifier: auto


@gql.input
class StatementSetReferenceInput(gql.NodeInput):
    reference_id: Optional[GlobalID] = None


@gql.django.partial(models.Statement)
class StatementRenameInput(gql.NodeInput):
    name: auto


@gql.input
class StatementMoveInput:
    id: GlobalID
    file_id: GlobalID
    parent_id: Optional[GlobalID] = None
    index: Optional[int] = None


@gql.input
class StatementSoftDeleteInput(gql.NodeInput):
    pass


@gql.input
class StatementRestoreInput(gql.NodeInput):
    pass


@gql.input
class StatementCommentedInput(gql.NodeInput):
    commented: bool


@gql.type
class StatementMutation:
    rename_statement: Statement = gql.django.update_mutation(StatementRenameInput)
    update_statement_modifier: Statement = gql.django.update_mutation(StatementSetModifierInput)

    @async_safe_mutation
    def create_statement(self, input: StatementCreateInput) -> StatementPayload:
        file = models.File.objects.get(id=input.file_id.node_id)
        project_version = file.project_version
        parent = (
            models.Statement.objects.get(id=input.parent_id.node_id) if input.parent_id else None
        )
        statement = models.Statement.objects.create_statement(
            project_version=project_version,
            file=file,
            type=input.type,
            name=input.name,
            parent=parent,
            index=input.index,
        )
        return StatementPayload(statement=statement)

    @async_safe_mutation
    def soft_delete_statement(self, input: StatementSoftDeleteInput) -> StatementPayload:
        statement = models.Statement.objects.get(id=input.id.node_id)
        statement.soft_delete()
        return StatementPayload(statement=statement)

    @async_safe_mutation
    def restore_statement(self, input: StatementRestoreInput) -> StatementPayload:
        # use base manager since default manager excludes soft deleted statements
        statement = models.Statement._base_manager.get(id=input.id.node_id)
        statement.restore()
        return StatementPayload(statement=statement)

    @async_safe_mutation
    def update_statement_reference(self, input: StatementSetReferenceInput) -> StatementPayload:
        statement = models.Statement.objects.get(id=input.id.node_id)
        reference = (
            models.Statement.objects.get(id=input.reference_id.node_id)
            if input.reference_id
            else None
        )
        statement.reference = reference
        statement.save()
        return StatementPayload(statement=statement)

    @async_safe_mutation
    def morph_statement(self, input: StatementMorphInput) -> StatementPayload:
        statement = models.Statement.objects.get(id=input.statement_id.node_id)
        statement.morph_to(input.type, input.symbol_type)
        return StatementPayload(statement=statement)

    @async_safe_mutation
    def comment_statement(self, input: StatementCommentedInput) -> StatementPayload:
        statement = models.Statement.objects.get(id=input.id.node_id)
        statement.set_commented(input.commented)
        return StatementPayload(statement=statement)

    @async_safe_mutation
    def move_statement(self, input: StatementMoveInput) -> StatementPayload:
        statement = models.Statement.objects.get(id=input.id.node_id)
        file = models.File.objects.get(id=input.file_id.node_id)
        parent = (
            models.Statement.objects.get(id=input.parent_id.node_id) if input.parent_id else None
        )
        old_file = statement.file
        statement.move_to(file, parent, input.index)
        return StatementPayload(statement=statement, old_file=old_file, new_file=file)


#
# Statement content / symbol mutations
#


@gql.django.partial(models.Statement)
class StatementTextInput(gql.NodeInput):
    text: auto


@gql.django.partial(models.Statement)
class StatementUpdateDescriptionInput(gql.NodeInput):
    description: str


@gql.django.partial(models.Statement)
class StatementUpdateCodeInput(gql.NodeInput):
    code_builtin_id: Optional[str] = None
    code: Optional[str] = None


@gql.django.partial(models.Statement)
class StatementUpdateTypeInput(gql.NodeInput):
    btl: str


@gql.input
class StatementUpdateRecordsInput(gql.NodeInput):
    records: list[JSON]


@gql.type
class SymbolMutation:
    update_statement_text: Statement = gql.django.update_mutation(StatementTextInput)
    update_statement_description: Statement = gql.django.update_mutation(
        StatementUpdateDescriptionInput
    )
    update_statement_code: Statement = gql.django.update_mutation(StatementUpdateCodeInput)
    update_statement_type_node: Statement = gql.django.update_mutation(StatementUpdateTypeInput)
