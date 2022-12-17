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

from bench.backend.builtins import code_builtins
from bench.backend.executor import Executor
from bench.backend.resolver import Resolver
from bench.models import (
    Code,
    Dataset,
    Expectation,
    Organization,
    ParameterType,
    Project,
    ProjectVersion,
    Schema,
    StatementType,
    Symbol,
    SymbolContent,
    SymbolType,
    Task,
)
from bench.models.symbol import Statement, StatementModifier
from bench.utils.schema import (
    SchemaElement,
    SchemaObjectSerializer,
    ValueType,
    derive_schema_from_function,
    get_value_type,
)

logger = structlog.get_logger(__name__)


@dataclass
class LibraryDependency:
    library: str


@dataclass
class FileSegment:
    virtual_path: str
    header: str  # in the format <symbol_type> [key=value]*: <name>
    lines: list[str]
    source_index: int

    @cached_property
    def symbol_name(self) -> str:
        return self.header.split(":")[1].strip()

    @cached_property
    def symbol_type(self) -> SymbolType:
        symbol_type = self.header.split(":")[0].split(" ")[0]
        try:
            return SymbolType(symbol_type)
        except ValueError:
            raise ValueError(f"unknown symbol type {symbol_type}")

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
        load_symbols(project_v, path)

        # advance head to new version
        project.head = project_v
        project.save()

        logger.info(f"Updated head to {project_v} in {project}")


def load_symbols(project_v: ProjectVersion, path: str):
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
        project_v.dependencies.add(library_v)
        logger.info(f"Import library {library_v}")
    executor = Executor(Resolver())
    # convert segments to a single task symbol tree
    for segment in segments:
        if segment.header.startswith("ignore"):
            continue

        symbols = async_to_sync(executor.run_text)(segment.full_code, {})

        def _get_symbol(name: str | None = None, required: bool = True):
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

        segment_parsers: dict[str, typing.Callable] = {
            SymbolType.SCHEMA: parse_schema,
            SymbolType.TASK: parse_task,
            SymbolType.EXPECTATION: parse_expect,
            SymbolType.CODE: parse_code,
            SymbolType.DATASET: parse_data,
        }
        segment_parser = segment_parsers.get(segment.symbol_type)
        if segment_parser is None:
            raise ValueError(f"unexpected segment type: {segment.symbol_type}")

        result = segment_parser(project_v=project_v, segment=segment, lookup_def=_get_symbol)
        if isinstance(result, tuple):
            symbol_content, on_defined = result
        else:
            symbol_content = result
            on_defined = None

        file = project_v.create_file_from_path(segment.virtual_path, exists_ok=True)
        symbol = project_v.define_symbol(segment.symbol_name, content=symbol_content, file=file)

        if on_defined:
            on_defined(symbol)

        parse_extra(project_v, segment, symbol, lookup_def=_get_symbol)

        logger.info(f"{symbol.definition}")


def parse_file_segment(lines: list[str]) -> tuple[list[LibraryDependency], list[FileSegment]]:
    segments: list[FileSegment] = []
    dependencies: list[LibraryDependency] = []

    # segments are controlled via @ switches in comments
    switches = {"@path", "@symbol", "@ignore", "@library"}

    segment: Optional[FileSegment] = None
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
        elif "@symbol" in line:
            # start new segment
            segment = FileSegment(
                virtual_path=virtual_path, header=line[10:].strip(), lines=[], source_index=i
            )
        elif segment is not None:
            segment.lines.append(line)
    if segment is None:
        raise ValueError("task file must contain at least one @bench segment")
    # close last segment
    segments.append(segment)
    print(f"segment {segment.header} with {len(segment.lines)} lines")

    return dependencies, segments


def parse_extra(
    project_v: ProjectVersion,
    segment: FileSegment,
    symbol: Symbol,
    lookup_def: typing.Callable,
):
    if "parent_ref" in segment.symbol_args:
        parent_name = segment.symbol_args["parent_ref"]
        parent = project_v.statement(file=None, name=parent_name)
        _mount_child_statement(parent, None, StatementType.REFERENCE, symbol)
    if "parent_def" in segment.symbol_args:
        parent_name = segment.symbol_args["parent_def"]
        parent = project_v.statement(file=None, name=parent_name)
        _mount_child_statement(parent, None, StatementType.DEFINITION, symbol)

    add_statements = lookup_def("add_statements", required=False)
    if add_statements is None:
        return
    for parent_declr, modifier, statement_type, child_declr in add_statements:
        parent_type, parent_name = parent_declr.split(" ")
        parent = project_v.statement(file=None, name=parent_name, type=SymbolType(parent_type))
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
    symbol_type, symbol_name = child_symbol_declr.split(" ")
    child_symbol = project_v.statement(file=None, name=symbol_name, type=symbol_type).symbol_
    _mount_child_statement(parent, modifier, statement_type, child_symbol)


def _mount_child_statement(
    parent: Statement,
    modifier: Optional[StatementModifier],
    statement_type: StatementType,
    child_symbol: Symbol,
):
    if statement_type == StatementType.DEFINITION:
        child_symbol.definition.modifier = modifier
        child_symbol.definition.move_to(parent.file, parent)
    else:
        parent.add_child(type=statement_type, content=child_symbol, modifier=modifier)


def parse_schema(
    project_v: ProjectVersion,
    segment: FileSegment,
    lookup_def: typing.Callable,
) -> tuple[SymbolContent, typing.Callable[[Symbol], None]]:
    schema_element = SchemaObjectSerializer.from_json(
        segment.symbol_name, lookup_def(segment.symbol_name)
    )
    description = lookup_def("description", required=False) or ""
    schema = Schema.objects.create(description=description, element=schema_element)
    return schema


def parse_task(
    project_v: ProjectVersion,
    segment: FileSegment,
    lookup_def: typing.Callable,
) -> tuple[SymbolContent, typing.Callable[[Symbol], None]]:
    description = lookup_def("task")
    task = Task.objects.create(description=description)

    def on_defined(symbol: Symbol):
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
    segment: FileSegment,
    lookup_def: typing.Callable,
):
    function = lookup_def(segment.symbol_name)
    if not callable(function):
        raise ValueError(f"code symbol '{segment.symbol_name}' is not typing.Callable")
    code = Code.objects.create(
        code=segment.full_code,
        code_function_name=function.__name__,
    )

    def on_defined(symbol: Symbol):
        # add schema (requires symbol & statement to be defined)
        input_schema, output_schema = derive_schema_from_function(function)
        schema_element = SchemaElement(
            name="", type=ValueType.OBJECT, elements=[input_schema, output_schema]
        )
        code.set_schema_element(schema_element)

        # parameters can also be defined in the schema extracted from the function signature
        for param in input_schema.elements:
            symbol.definition.add_parameter(name=param.name, type=ParameterType.VALUE, schema=param)
        consumed_lines = bind_parameters(segment, project_v, symbol.definition)
        if consumed_lines:
            # remove consumed lines from segment
            segment.lines = segment.lines[consumed_lines:]
            code.code = segment.full_code
            code.save()

    return code, on_defined


def parse_data(
    project_v: ProjectVersion,
    segment: FileSegment,
    lookup_def: typing.Callable,
):
    records = lookup_def(segment.symbol_name)
    if not isinstance(records, list):
        raise ValueError(f"data symbol '{segment.symbol_name}' is not a list")
    dataset = Dataset.objects.from_list(records)

    def on_defined(symbol: Symbol):
        dataset.derive_schema()

    return dataset, on_defined


def parse_expect(
    project_v: ProjectVersion,
    segment: FileSegment,
    lookup_def: typing.Callable,
):
    description = lookup_def("expectation")
    if not isinstance(description, str):
        raise ValueError(f"expectation description '{segment.symbol_name}' is not a str")
    statements_names = lookup_def("statements")
    if not isinstance(statements_names, list):
        raise ValueError(f"expectation statements '{segment.symbol_name}' is not a list")

    expectation = Expectation.objects.create(description=description)

    def on_defined(symbol: Symbol):
        for statement_def in statements_names:
            # expectation statement e.g. ("like", "REFERENCE", "code respect_command_hints")
            modifier, statement_type, symbol_declr = statement_def
            _parse_child_statement(
                project_v, symbol.definition, modifier, statement_type, symbol_declr
            )
        if "task" in segment.symbol_args:
            task_name = segment.symbol_args["task"]
            task = project_v.statement(file=None, name=task_name).symbol_.task_
            task.definition.add_child(StatementType.REFERENCE, symbol)

    return expectation, on_defined


def bind_parameters(
    segment: FileSegment,
    project_v: ProjectVersion,
    statement: Statement,
) -> int:
    # parameters are defined as type only symbol lines like:
    # name: Task|Code|Model|Dataset|..
    # name: <type>
    # Parameters are bound to their name or an @alias.
    consumed_lines: int = 0
    for line in segment.lines:
        # assume all parameters are declared up front
        if ":" not in line or "=" in line or "(" in line:
            break
        consumed_lines += 1
        if "#" in line:
            comment = line[line.find("#") :]
            line = line[: line.find("#")]
        else:
            comment = ""
        param_name, param_type = line.split(":", 1)
        param_name = param_name.strip()
        param_type = param_type.strip()
        if param_name in code_builtins:
            continue  # ignore builtins

        param_schema = None
        if param_type == "Dataset":
            param_type = ParameterType.DATA
        elif param_type == "Model":
            param_type = ParameterType.MODEL
        elif param_type == "Code":
            param_type = ParameterType.CODE
        else:
            param_schema = SchemaElement(name=param_name, type=get_value_type(param_type))
            param_type = ParameterType.VALUE
        statement.add_parameter(name=param_name, type=param_type, schema=param_schema)

        # use alias if set
        if "@alias" in comment:
            symbol_ref_name = comment[comment.find("@alias") + 6 :].strip()
        else:
            symbol_ref_name = param_name

        if param_type == ParameterType.VALUE:
            raise NotImplementedError(f"json argument resolution not supported: {line}")
        symbol_ref = project_v.statement(file=None, name=symbol_ref_name)
        statement.bind_argument(param_name, symbol_ref)
    return consumed_lines
