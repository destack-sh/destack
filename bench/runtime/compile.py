from __future__ import annotations

import ast
import enum
import json
from contextlib import contextmanager
from dataclasses import dataclass, field
from typing import Any, Optional

import structlog

from bench.language.reconstruct import render_type_node, render_type_node_struct
from bench.language.type import (
    PRIMITIVE_TYPES,
    Code,
    CodeContent,
    Compilation,
    Dataset,
    DatasetContent,
    Expectation,
    File,
    InterpSymbol,
    LiteralValue,
    Model,
    Module,
    SourceMapping,
    Statement,
    StatementType,
    Task,
    Type,
    TypeNode,
    TypeTag,
)
from bench.language.typer import check_type, fabricate

logger = structlog.get_logger(__name__)


#
# Compilation
#
# On a high level, compilation is a meta-program that takes a compilation and produces optimal
# executable code (and any other symbols) for a task given the compilation constraints.
#
# More formally: compile is a function of task to executable code given a compilation.
# Implementing compile equals some/many forms of optimization problem, we'll see.
#


class CompileErrorType(enum.Enum):
    INTERNAL = 0, "Internal error"
    RUN = 1, "Error running user code"

    def __new__(cls, value, description):
        obj = object.__new__(cls)
        obj._value_ = value
        obj.description = description
        return obj


class CompileError(ValueError):
    def __init__(
        self,
        _t: CompileErrorType,
        symbol: Optional[InterpSymbol],
        cause: Optional[Exception] = None,
    ):
        self.type = _t
        self.symbol = symbol
        self.cause = cause
        super().__init__(self.type.description)


@dataclass
class CompilationState:
    compilation: Compilation
    task: Task
    model: Model
    target_symbols: list[InterpSymbol] = field(default_factory=list)
    # TODO @Incomplete: track source mappings during compilation
    source_mappings: list[SourceMapping] = field(default_factory=list)

    def create_data(self, builder: DataBuilder) -> Dataset:
        dataset = Dataset(
            name=builder.name,
            type_node=builder.type_node,
            type=builder.type_node.to_type(),
            records=builder.records,
            description=None,
            language="jsonl",
        )
        # type check records
        for record in dataset.records:
            check_type(record, dataset.type)
        self.target_symbols.append(dataset)
        return dataset

    def create_code(self, builder: PromptBuilder) -> Code:
        code = Code(
            name=builder.name,
            type_node=builder.type_node,
            type=builder.type_node.to_type(),
            language="bpl",
            code=builder.to_code_content(),
            builtin_id=None,
            description=None,
        )
        # try to parse code
        ast.parse(code.code)
        self.target_symbols.append(code)
        return code


class DataBuilder:
    """Build a dataset."""

    def __init__(self, name: str, type_node: TypeNode):
        self.name = name
        self.type_node = type_node
        self.records: list[LiteralValue] = []

    def append(self, record: LiteralValue):
        self.records.append(record)

    def extend(self, records: list[LiteralValue]):
        self.records.extend(records)


class PromptBuilder:
    """Build a BPL dynamic prompt."""

    def __init__(self, name: str | None = None, type_node: TypeNode | None = None):
        self.name = name
        self.type_node = type_node
        self.bpl_lines = []
        self.current_indent = 0

    def prepend(self, line: str):
        indent = " " * 4 * self.current_indent
        self.bpl_lines.insert(0, indent + line)

    def append(self, line: str):
        indent = " " * 4 * self.current_indent
        self.bpl_lines.append(indent + line)

    def extend(self, lines: list[str]):
        for line in lines:
            self.append(line)

    def emit(self, line: str):
        # escape quotes and newlines
        line = line.replace("\n", "\\n")
        line = line.replace('"', '\\"')
        self.append(f'"{line}"')

    def emit_many(self, lines: list[str]):
        for line in lines:
            self.emit(line)

    @contextmanager
    def block(self, line: str | None = None, indent: int = 1):
        if line:
            self.append(line)
        self.current_indent += indent
        yield
        self.current_indent -= indent

    def blank(self):
        self.append("")

    def comment(self, line: str):
        self.bpl_lines.append(f"# {line}")

    def pragma(self, **kwargs):
        self.prepend(f"pragma({', '.join(f'{k}={v}' for k, v in kwargs.items())})")

    def to_code_content(self) -> str:
        return "\n".join(self.bpl_lines)

    #
    # Structured emits (maybe should live elsewhere?)
    #

    def emit_explain_type(self, node: Type):
        """Emits BPL to explain the given type and all its references."""

        explained_types: list[TypeNode] = []
        types_to_explain: list[TypeNode] = [node]
        while len(types_to_explain) > 0:
            node = types_to_explain.pop()
            if node.tag == TypeTag.FUNCTION:  # recast to struct for simplicity
                node = TypeNode(
                    name=node.name, tag=TypeTag.STRUCT, children=[*node.input.children, node.output]
                )
            type_str = render_type_node(node, ignore_name=True, ignore_reference=True)
            if node.tag == TypeTag.STRUCT:
                self.emit(f"type {node.source_reference}:")
                self.emit(type_str)
            elif node.tag == TypeTag.ENUM:
                self.emit(f"enum {node.source_reference}:")
                self.emit(type_str)
            else:
                self.emit(f"type {node.source_reference} = {type_str}")
            self.emit("\n")
            self.blank()
            explained_types.append(node)

            for child in node.children or []:
                while child.tag == TypeTag.ARRAY:
                    child = child.children[0]  # skip array (simple wrapper)
                needs_explanation = (
                    child.tag not in PRIMITIVE_TYPES
                    and child.tag != TypeTag.ANY
                    and child.tag != TypeTag.LITERAL
                    and not child.is_flat
                )
                if (
                    needs_explanation
                    and child not in explained_types
                    and child.source_reference is not None
                ):
                    types_to_explain.append(child)

    def emit_show_dataset(self, dataset: Dataset | DataBuilder):
        """Emits BPL to show the given dataset."""
        element_type_str = render_type_node(dataset.type_node)
        with self.block(f"for record in context['{dataset.name}']:\n"):
            self.append("_record_as_json = json.dumps(record, indent=2)")
            self.emit(f"value example :: ({element_type_str}):\n")
            self.emit("```\n")
            self.emit("{_record_as_json}\n")
            self.emit("```\n")

    def emit_show_record(self, value: Any, type: TypeNode):
        """Emits BPL to show the given example."""
        check_type(value, type)
        record_type_str = render_type_node_struct(type, seperator=", ")
        self.emit(f"value example :: ({record_type_str}):\n")
        self.emit("```\n")
        self.emit(json.dumps(value, indent=2))
        self.emit("\n```\n")

    def emit_get_record(
        self,
        inputs: list[TypeNode],
        output: TypeNode,
        output_var: str,
    ):
        """Emits BPL to get the output record given the inputs"""
        # combine inputs and outputs into single example struct
        combined_type = TypeNode(name=None, tag=TypeTag.STRUCT, children=inputs + [output])
        combined_type_str = render_type_node_struct(combined_type, seperator=", ")
        self.emit(f"value :: ({combined_type_str}):\n")
        self.emit("```\n")
        self.emit("{\n")
        # render input fields as BPL variables like |{var_name}|
        for input in inputs:
            self.emit(f'  "{input.name}": |{{{input.name}}}|,\n')
        # render output as giant hole of its type :JsonHole
        output_type_str = render_type_node(output, ignore_name=True, ignore_description=True)
        self.emit(f'  "{output.name}": |[{output_var}: {output_type_str}]|\n')
        self.emit("}\n")
        self.emit("```")


async def compile(compilation: Compilation) -> tuple[list[InterpSymbol], list[SourceMapping]]:
    logger.debug("compile.start", compilation=compilation)
    if len(compilation.tasks) != 1 or len(compilation.models) != 1:
        raise CompileError(CompileErrorType.INTERNAL, compilation)

    state = CompilationState(
        compilation=compilation, task=(compilation.tasks[0]), model=(compilation.models[0])
    )
    await _compile_task(state, state.task)
    logger.debug("compile.end", compilation=compilation, state=state)
    return state.target_symbols, state.source_mappings


def _gather_expectations(symbol: Expectation | Task) -> list[Expectation]:
    expectations = []
    if isinstance(symbol, Expectation):
        expectations.append(symbol)
    if isinstance(symbol, (Task, Expectation)):
        for child in symbol.expectations:
            expectations.extend(_gather_expectations(child))
    return expectations


async def _compile_task(state: CompilationState, task: Task) -> None:
    task_t = task.type
    task_expectations = _gather_expectations(task)

    target_code = PromptBuilder(name=task.name, type_node=task_t)
    target_code.comment("Task metadata")
    target_code.emit(f'task "{task.name}"\n')
    target_code.emit(task.description + "\n")
    target_code.blank()

    # task type explanation
    target_code.comment("Task type instruction")
    target_code.emit_explain_type(task_t)

    # task examples
    examples_type = TypeNode(
        name=task.name + "_unravelled",
        tag=TypeTag.STRUCT,
        children=[*task_t.input.children, task_t.output],
    )
    examples_data = DataBuilder(name=task.name + " examples", type_node=examples_type)

    # task example instruction
    # TODO @Incomplete: generate examples for task
    if examples_data.records:
        target_code.comment("Task example instruction")
        target_code.emit(f'examples for task "{task.name}":\n')
        target_code.emit(task.description + "\n")
        target_code.emit_show_dataset(examples_data)

    # task inference
    target_code.comment("Task inference")
    target_code.emit(
        f"Perform the task {task.name} as described above to complete the output with the correct types."
        f" Consider the instructions carefully:\n"
    )
    target_code.emit(task.description + "\n")
    # inline expectation restatement
    for expectation in task_expectations:
        if isinstance(expectation, Expectation):
            target_code.emit(" - " + expectation.description + "\n")

    # task type example stub
    # (basically an example that is properly formatted but has obviously fake values)
    target_code.comment("Task type example stub")
    target_code.emit("The data should look like this (with real values obviously):\n")
    fake_data = fabricate(examples_type)
    target_code.emit_show_record(fake_data, examples_type)
    target_code.blank()

    # task input fields
    target_code.emit_get_record(task_t.input.children, task_t.output, "final_output")
    target_code.append("return final_output")

    # target pragma
    # TODO @Incomplete: generate task target pragma properly
    #  1. Get temperature from task description + type info
    #  2. Get max_tokens from emitted size..? Set max_new_tokens instead?
    #  3. Set temperature in pragma zones? (field by field)
    target_code.pragma(temperature=0.5, max_tokens=1024, model=f'context["{state.model.name}"]')

    state.create_data(examples_data)
    state.create_code(target_code)


def down(symbols: list[InterpSymbol], file: File | None = None) -> File:
    """Map high-level interpreted symbols back to lower level statements."""

    if file is None:
        file = File(module=Module(name="<generated>"), path="<generated>")

    # render symbols themselves
    for symbol in symbols:
        if isinstance(symbol, Dataset):
            content = down_dataset_content(symbol)
        elif isinstance(symbol, Code):
            content = down_code_content(symbol)
        else:
            raise RuntimeError(f"unexpected symbol {symbol}")
        statement = Statement(
            id=symbol.id,
            type=StatementType.DEFINITION,
            symbol_type=symbol.symbol_type,
            modifier=symbol.modifier,
            name=symbol.name,
            content=content,
            file=file,
            parent=None,
            index=len(file.statements),
        )
        file.statements.append(statement)

    # render relations
    pass  # (not needed yet)

    return file


def down_dataset_content(dataset: Dataset) -> DatasetContent:
    return DatasetContent(
        description=dataset.description,
        language=dataset.language,
        type_node=dataset.type_node,
        records=dataset.records,
    )


def down_code_content(code: Code) -> CodeContent:
    return CodeContent(
        description=code.description,
        language=code.language,
        type_node=code.type_node,
        code=code.code,
        builtin_id=code.builtin_id,
    )
