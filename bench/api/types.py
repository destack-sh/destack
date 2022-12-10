from __future__ import annotations

from typing import Optional

from strawberry import auto
from strawberry_django_plus import gql

from bench import models
from bench.utils import schema

ValueType = gql.enum(schema.ValueType)


@gql.type
class SchemaElement:
    name: str
    type: ValueType
    required: bool
    # TODO @Cleanup: schema element choices should be unions
    choices: Optional[list[str]]
    elements: Optional[list[SchemaElement]]


@gql.django.type(models.User)
class User(gql.relay.Node):
    username: auto
    first_name: auto
    last_name: auto
    email: auto
    created_at: auto
    updated_at: auto
    organizations: list[Organization]


@gql.django.type(models.Organization)
class Organization(gql.relay.Node):
    name: auto
    slug: auto
    created_at: auto
    updated_at: auto
    projects: list[Project]
    members: list[User]


@gql.django.type(models.Project)
class Project(gql.Node):
    name: auto
    slug: auto
    organization: Organization
    created_at: auto
    updated_at: auto
    head: ProjectVersion
    versions: list[ProjectVersion]  # TODO @Cleanup: use relay connections


@gql.django.type(models.ProjectVersion)
class ProjectVersion(gql.Node):
    project: Project
    name: auto
    description: auto
    parents: list[ProjectVersion]
    children: list[ProjectVersion]
    created_at: auto
    committed: auto
    committed_at: auto
    libraries: list[ProjectVersion]
    main_program: Optional[Symbol]
    files: list[File]
    compilations: list[Compilation]
    symbols: list[Symbol]


@gql.django.type(models.File)
class File(gql.Node):
    project_version: ProjectVersion
    name: auto
    path: auto
    created_at: auto
    updated_at: auto
    is_folder: auto
    parent: Optional[File]  # containing folder
    files: list[File]  # if folder
    symbols: list[Symbol]  # if file


@gql.django.type(models.Symbol)
class Symbol(gql.Node):
    project_version: ProjectVersion
    name: auto
    type: auto
    type_shortname: auto
    type_name_declaration: auto
    file: File
    parent: Optional[Symbol]
    children: list[Symbol]
    index: auto
    created_at: auto
    updated_at: auto
    generated: auto
    content: SymbolContent
    parameters: list[SymbolParameter]
    arguments: list[SymbolArgument]


@gql.django.interface(models.SymbolContent)
class SymbolContent(gql.Node):
    # TODO @Cleanup: fix symbol in SymbolContent interface
    #  (should work since it's a 1:1 but doesn't)
    # symbol: Symbol
    pass


@gql.django.type(models.SymbolParameter)
class SymbolParameter(gql.Node):
    symbol: Symbol
    name: auto
    created_at: auto
    updated_at: auto
    type: auto
    schema: Optional[SchemaElement]


@gql.django.type(models.SymbolArgument)
class SymbolArgument(gql.Node):
    symbol: Symbol
    name: auto
    created_at: auto
    updated_at: auto
    type: auto
    reference: Optional[Symbol]
    value: auto


@gql.django.type(models.Task)
class Task(SymbolContent):
    symbol: Symbol
    input_schema: SchemaElement
    output_schema: SchemaElement
    expectations: list[Expectation]
    template_implementation: Optional[Symbol]
    compilations: list[Compilation]


@gql.django.type(models.Compilation)
class Compilation(gql.Node):
    created_at: auto
    updated_at: auto
    task: Task
    name: auto
    backends: list[Model]
    target_task: Optional[Task]
    target_code: Optional[Code]
    mappings: list[SourceMapping]


@gql.django.type(models.SourceMapping)
class SourceMapping(gql.Node):
    compilation: Compilation
    source: Symbol
    source_path: auto
    target: Symbol
    target_path: auto


@gql.django.type(models.Expectation)
class Expectation(SymbolContent):
    symbol: Symbol
    description: auto
    statements: list[Symbol]


@gql.django.type(models.Code)
class Code(SymbolContent):
    symbol: Symbol
    input_schema: SchemaElement
    output_schema: SchemaElement
    task: Optional[Task]
    builtin_id: auto
    code: auto


@gql.django.type(models.Execution)
class Execution(gql.Node):
    created_at: auto
    updated_at: auto
    started_at: auto
    terminated_at: auto
    duration_millis: auto
    status: auto
    inputs: auto
    outputs: auto
    error: auto
    parent: Optional[Execution]
    children: list[Execution]
    code: Code
    model: Optional[Model]


@gql.django.type(models.Model)
class Model(SymbolContent):
    symbol: Symbol
    baseline: Optional[Model]
    provider: auto


@gql.django.type(models.Dataset)
class Dataset(SymbolContent):
    symbol: Symbol
    schema: SchemaElement
    length: auto
    records: list[DatasetRecord]


@gql.django.type(models.DatasetRecord)
class DatasetRecord(gql.Node):
    index: auto
    data: auto


@gql.django.type(models.DatasetView)
class DatasetView(SymbolContent):
    symbol: Symbol
    dataset: Dataset
