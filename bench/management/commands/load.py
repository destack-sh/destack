from __future__ import annotations

import datetime
import typing
from dataclasses import dataclass
from functools import cached_property
from pathlib import Path
from typing import Optional

import structlog
from asgiref.sync import async_to_sync
from django.core.management import BaseCommand
from django.core.management.base import CommandParser
from django.db import transaction

from bench.models import (
    Code,
    Dataset,
    Expectation,
    File,
    Organization,
    Project,
    ProjectVersion,
    Schema,
    StatementType,
    SymbolContent,
    SymbolType,
    Task,
)
from bench.models.project import FileType
from bench.models.symbol import Statement, StatementModifier
from bench.utils.schema import (
    SchemaElement,
    SchemaObjectSerializer,
    ValueType,
    derive_schema_from_function,
)

logger = structlog.get_logger(__name__)


@dataclass
class LibraryDependency:
    library: str


@dataclass
class StatementSegment:
    virtual_path: str
    type: StatementType
    header: str  # in the format <symbol_type> [key=value]*: <name>
    lines: list[str]
    source_index: int

    @cached_property
    def symbol_name(self) -> str:
        return self.header.split(":")[1].strip()

    @cached_property
    def symbol_type(self) -> Optional[SymbolType]:
        symbol_type = self.header.split(":")[0].split(" ")[0]
        try:
            return SymbolType(symbol_type)
        except ValueError:
            return None

    @cached_property
    def symbol_args(self) -> dict[str, str]:
        args = self.header.split(":")[0].split(" ")[1:]
        return {arg.split("=")[0]: arg.split("=")[1] for arg in args}

    @property
    def full_code(self):
        # code already contains newlines
        return "".join(self.lines).strip()


class Command(BaseCommand):
    help = "Loads a instructions from a file into a project"

    def add_arguments(self, parser: CommandParser):
        # project as organization/project
        parser.add_argument("organization_project", type=str)
        # symbol file path (must exist and end in .py)
        parser.add_argument("path", type=str)

    @transaction.atomic
    def handle(self, organization_project: str, path: str, *args, **options):
        organization_slug, project_slug = organization_project.split("/")
        organization = Organization.objects.get(slug=organization_slug)
        project = Project.objects.filter(slug=project_slug, organization=organization).first()
        if project is None:
            project = Project.objects.create_project(
                organization=organization, name=project_slug, slug=project_slug
            )

        last_modified = datetime.datetime.fromtimestamp(Path(path).stat().st_mtime)
        version_id = str(int(last_modified.timestamp()))

        project_v = project.create_version(name=version_id)
        project_v.reset()
        project_v.bootstrap()
        load(project_v, path)

        # advance head to new version
        project.head = project_v
        project.save()

        logger.info(f"Updated head to {project_v} in {project}")


def load(project_v: ProjectVersion, path: str):
    """Loads symbols from a file into a project version"""

    # read task file lines
    with open(path, "r") as f:
        lines = f.readlines()
    dependencies, segments = parse_file_segment(lines)
    for library_dependency in dependencies:
        library_org, library_slug = library_dependency.library.split("/")
        library = Project.objects.get_by_slug(library_org, library_slug)
        if library is None:
            raise ValueError(f"library {library_dependency.library} not found")
        library_v = library.head  # just use head
        project_v.add_requirement(library_v, file=project_v.project_file)
        logger.info(f"Import library {library_v}")
    # convert segments to a single task symbol tree
    for segment in segments:
        if segment.header.startswith("ignore"):
            continue

        symbols = async_to_sync(executor.run_text)(segment.full_code, {})

        def _get_symbol(name: str | None = None, required: bool = True):
            # replace non-white space characters with underscores
            name = name.replace(" ", "_") if name else None
            if name is None:
                return symbols
            elif name not in symbols:
                if not required:
                    return None
                else:
                    raise LookupError(
                        f"symbol '{name}' not found in segment '{segment.header}':\n{segment.full_code}"
                    )
            return symbols[name]

        definition_parsers: dict[str, typing.Callable] = {
            SymbolType.SCHEMA: parse_schema,
            SymbolType.TASK: parse_task,
            SymbolType.EXPECTATION: parse_expect,
            SymbolType.CODE: parse_code,
            SymbolType.DATASET: parse_data,
        }
        if segment.type == StatementType.DEFINITION:
            segment_parser = definition_parsers.get(segment.symbol_type)
            if segment_parser is None:
                raise ValueError(f"unexpected segment type: {segment.symbol_type}")

            result = segment_parser(project_v=project_v, segment=segment, lookup_def=_get_symbol)
            if isinstance(result, tuple):
                symbol_content, on_defined = result
            else:
                symbol_content = result
                on_defined = None

            file = project_v.create_file_from_path(
                segment.virtual_path, FileType.INSTRUCT, exists_ok=True
            )
            statement = project_v.define_symbol(
                segment.symbol_name, content=symbol_content, file=file
            )

            if on_defined:
                on_defined(statement)

            parse_extra(project_v, segment, statement, lookup_def=_get_symbol)
        elif segment.type == StatementType.IMPORT:
            file = project_v.create_file_from_path(
                segment.virtual_path, FileType.INSTRUCT, exists_ok=True
            )
            statement = parse_import(project_v, file, segment)
        else:
            raise ValueError(f"unexpected segment type: {segment.type}")

        logger.info(f"{statement}")


def parse_file_segment(lines: list[str]) -> tuple[list[LibraryDependency], list[StatementSegment]]:
    segments: list[StatementSegment] = []
    dependencies: list[LibraryDependency] = []

    # segments are controlled via @ switches in comments
    switches = {"@path", "@define", "@import", "@ignore", "@library"}

    segment: Optional[StatementSegment] = None
    virtual_path = "main"  # default to main
    for i, line in enumerate(lines):
        # close previous segment if we're starting a new one
        if segment is not None and any(switch in line for switch in switches):
            segments.append(segment)
            print(f"segment {segment.header} with {len(segment.lines)} lines")
            segment = None
        if "@library" in line:
            # library import
            library = line.split("@library")[1].strip()
            dependencies.append(LibraryDependency(library=library))
            print(f"library {library}")
        elif "@path" in line:
            # change path
            # parse path like # @path <path>
            virtual_path = line[line.find("@path") + 5 :].strip()
            print(f"path = {virtual_path}")
        elif "@define" in line or "@import" in line:
            # start new segment
            statement_type = StatementType.IMPORT if "@import" in line else StatementType.DEFINITION
            segment = StatementSegment(
                virtual_path=virtual_path,
                type=statement_type,
                header=line[10:].strip(),
                lines=[],
                source_index=i,
            )
        elif segment is not None:
            segment.lines.append(line)
    if segment is None:
        raise ValueError("task file must contain at least one @bench segment")
    # close last segment
    segments.append(segment)
    print(f"segment {segment.header} with {len(segment.lines)} lines")

    return dependencies, segments


def parse_import(project_v: ProjectVersion, file: File, segment: StatementSegment) -> Statement:
    # imports look like
    # text-davinci-003 as model from openai.stdlib.text
    # or more generally
    # <symbol> [as <alias>] from [<library>].<path>
    # (the "import" prefix is stripped by the parser)

    parts = segment.header.split("from")
    if len(parts) != 2:
        raise ValueError(f"invalid import statement: {segment.header}")

    # get source symbol
    declaration = parts[0].strip()
    if " as " in declaration:
        source_name, alias = declaration.split(" as ")
    else:
        source_name = declaration
        alias = declaration  # default to same name

    # get source statement
    source = parts[1].strip()
    if source.startswith("."):
        # relative import
        source_file = project_v.get_file(source)
        source_statement = project_v.statement(source_file, source_name)
    elif "." in source:
        # absolute import
        organization = source.split(".")[0]
        library = source.split(".")[1]
        path = ".".join(source.split(".")[2:])
        dependency = project_v.dependency(organization, library)
        source_file = dependency.get_file(path, FileType.INSTRUCT)
        source_statement = dependency.statement(source_file, source_name)
    else:
        raise ValueError(f"invalid import statement: {segment.header}")

    # create import statement
    return project_v.import_statement(source_statement, alias=alias, file=file)


def parse_extra(
    project_v: ProjectVersion,
    segment: StatementSegment,
    statement: Statement,
    lookup_def: typing.Callable,
):
    if "parent_ref" in segment.symbol_args:
        parent_name = segment.symbol_args["parent_ref"]
        parent = project_v.statement(file=None, name=parent_name, type=StatementType.DEFINITION)
        _mount_child_statement(project_v, parent, None, StatementType.REFERENCE, statement)
    if "parent_def" in segment.symbol_args:
        parent_name = segment.symbol_args["parent_def"]
        parent = project_v.statement(file=None, name=parent_name, type=StatementType.DEFINITION)
        _mount_child_statement(project_v, parent, None, StatementType.DEFINITION, statement)

    add_statements = lookup_def("add_statements", required=False)
    if add_statements is None:
        return
    for parent_declr, modifier, statement_type, child_declr in add_statements:
        parent_type, parent_name = parent_declr.split(" ", maxsplit=1)
        parent = project_v.statement(None, parent_name, symbol_type=SymbolType(parent_type))
        _parse_child_statement(project_v, parent, modifier, statement_type, child_declr)


def _parse_child_statement(
    project_v: ProjectVersion,
    parent: Statement,
    modifier: Optional[str],
    statement_type: str,
    child_symbol_declr: str,
):
    modifier = StatementModifier(modifier) if modifier else None
    statement_type = StatementType(statement_type)
    symbol_type, symbol_name = child_symbol_declr.split(" ", maxsplit=1)
    child = project_v.statement(file=None, name=symbol_name, symbol_type=symbol_type)
    _mount_child_statement(project_v, parent, modifier, statement_type, child)


def _mount_child_statement(
    project_v: ProjectVersion,
    parent: Statement,
    modifier: Optional[StatementModifier],
    statement_type: StatementType,
    child: Statement,
):
    if statement_type == StatementType.DEFINITION:
        # move definition to parent
        child.modifier = modifier
        child.move_to(parent.file, parent)
    elif statement_type == StatementType.REFERENCE:
        # reference child in parent (via import if necessary)
        if child.file != parent.file:
            imported = project_v.get_import_of(parent.file, child)
            if imported is None:
                imported = project_v.import_statement(child, alias=None, file=parent.file)
        else:
            imported = child
        # create reference statement to child
        reference = project_v.reference_statement(
            imported, alias=None, file=parent.file, parent=parent
        )
        reference.modifier = modifier
        reference.save()
    else:
        raise ValueError(f"invalid type to mount: {statement_type}")


def parse_schema(
    project_v: ProjectVersion,
    segment: StatementSegment,
    lookup_def: typing.Callable,
) -> tuple[SymbolContent, typing.Callable[[Statement], None]]:
    schema_element = SchemaObjectSerializer.from_json(
        segment.symbol_name, lookup_def(segment.symbol_name)
    )
    description = lookup_def("description", required=False) or ""
    schema = Schema.objects.create(description=description, element=schema_element)
    return schema


def parse_task(
    project_v: ProjectVersion,
    segment: StatementSegment,
    lookup_def: typing.Callable,
) -> tuple[SymbolContent, typing.Callable[[Statement], None]]:
    description = lookup_def("task")
    task = Task.objects.create(description=description)

    def on_defined(statement: Statement):
        if not lookup_def("input_schema", required=False):
            # schema may be set directly with add_statements
            return
        input_schema = SchemaObjectSerializer.from_json("input", lookup_def("input_schema"))
        output_schema = SchemaObjectSerializer.from_json("output", lookup_def("output_schema"))
        schema_element = SchemaElement(
            "schema", type=ValueType.OBJECT, elements=[input_schema, output_schema]
        )
        task.set_schema_element(schema_element)

    return task, on_defined


def parse_code(
    project_v: ProjectVersion,
    segment: StatementSegment,
    lookup_def: typing.Callable,
):
    function = lookup_def(segment.symbol_name)
    if not callable(function):
        raise ValueError(f"code symbol '{segment.symbol_name}' is not typing.Callable")
    code = Code.objects.create(
        code=segment.full_code,
        code_function_name=function.__name__,
    )

    def on_defined(statement: Statement):
        # add schema (requires symbol & statement to be defined)
        input_schema, output_schema = derive_schema_from_function(function)
        schema_element = SchemaElement(
            name="", type=ValueType.OBJECT, elements=[input_schema, output_schema]
        )
        code.set_schema_element(schema_element)

    return code, on_defined


def parse_data(
    project_v: ProjectVersion,
    segment: StatementSegment,
    lookup_def: typing.Callable,
):
    records = lookup_def(segment.symbol_name)
    if not isinstance(records, list):
        raise ValueError(f"data symbol '{segment.symbol_name}' is not a list")
    dataset = Dataset.objects.from_list(records)

    def on_defined(statement: Statement):
        dataset.derive_schema()

    return dataset, on_defined


def parse_expect(
    project_v: ProjectVersion,
    segment: StatementSegment,
    lookup_def: typing.Callable,
):
    description = lookup_def("expectation")
    if not isinstance(description, str):
        raise ValueError(f"expectation description '{segment.symbol_name}' is not a str")
    statements_names = lookup_def("statements")
    if not isinstance(statements_names, list):
        raise ValueError(f"expectation statements '{segment.symbol_name}' is not a list")

    expectation = Expectation.objects.create(description=description)

    def on_defined(statement: Statement):
        for statement_def in statements_names:
            # expectation statement e.g. ("like", "REFERENCE", "code respect_command_hints")
            modifier, statement_type, symbol_declr = statement_def
            _parse_child_statement(project_v, statement, modifier, statement_type, symbol_declr)

    return expectation, on_defined
