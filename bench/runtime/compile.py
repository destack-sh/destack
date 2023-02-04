from __future__ import annotations

import ast
import enum
import json
import uuid
from contextlib import contextmanager
from dataclasses import dataclass, field
from typing import Any, Optional, Union

import structlog

from bench.language.reconstruct import render_type_node, render_type_node_struct
from bench.language.type import (
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
    StatementModifier,
    StatementType,
    Task,
    Type,
    TypeNode,
    TypeTag,
)
from bench.language.typer import check_type, fabricate
from bench.utils.fractional import generate_n_keys_between

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
    candidates: list[CompilationCandidate] = field(default_factory=list)
    best_candidate: Optional[CompilationCandidate] = None


@dataclass
class CompilationCandidate:
    state: CompilationState
    compilation: Compilation
    task: Task
    model: Model
    candidate_id: uuid = field(default_factory=uuid.uuid4)
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

    def to_result(self) -> CompilationResult:
        return CompilationResult(
            compilation=self.compilation,
            target_symbols=self.target_symbols,
            source_mappings=self.source_mappings,
        )


@dataclass
class CompilationResult:
    compilation: Compilation
    target_symbols: list[InterpSymbol]
    source_mappings: list[SourceMapping]

    def to_file(self, module: Module | None = None) -> File:
        if module:
            module = Module(name="<compilation>")
        file = File(path=self.compilation.id.hex + ".gen", module=module)
        return down(self.target_symbols, file)


class DataBuilder:
    """Build a dataset."""

    def __init__(self, name: str, type_node: TypeNode):
        self.name = name
        self.type_node = type_node
        self.records: list[LiteralValue] = []

    def append(self, record: LiteralValue):
        check_type(record, self.type_node)
        self.records.append(record)

    def extend(self, records: list[LiteralValue]):
        for record in records:
            self.append(record)


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

    def emit_split(self, line: str):
        self.emit_many([(line + "\n") for line in line.splitlines()])

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
        """Emits BPL to explain the given type (recursively)."""

        for node in node.walk():
            # ignore improper type
            if node.source_reference is None:
                continue
            if node.tag == TypeTag.FUNCTION:  # recast to struct for simplicity
                node = TypeNode(
                    name=node.name, tag=TypeTag.STRUCT, children=[*node.input.children, node.output]
                )
            type_str = render_type_node(node, ignore_name=True, ignore_reference=True)
            if node.tag == TypeTag.STRUCT:
                self.emit(f"type {node.source_reference}:")
                self.emit_split(type_str)
            elif node.tag == TypeTag.ENUM:
                self.emit(f"enum {node.source_reference}:")
                self.emit_split(type_str)
            else:
                self.emit(f"type {node.source_reference} = {type_str}")
            self.emit("\n")
            self.blank()

    def emit_show_dataset(self, dataset: Dataset | DataBuilder):
        """Emits BPL to show the given dataset."""
        element_type_str = render_type_node(dataset.type_node, ignore_description=True)
        self.emit(f"data '{dataset.name}' :: ({element_type_str}):\n")
        self.emit("```jsonl\n")
        with self.block(f"for record in context['{dataset.name}']:"):
            self.append("_record_as_json = json.dumps(record, indent=2)")
            self.emit("|{_record_as_json}|\n")
        self.emit("```\n")
        self.blank()

    def emit_show_value(self, value: Any, type: TypeNode, name: str = "example"):
        """Emits BPL to show the given example."""
        check_type(value, type)
        record_type_str = render_type_node_struct(type, seperator=", ")
        self.emit(f"value {name} :: ({record_type_str}):\n")
        self.emit("```json\n")
        self.emit_split(json.dumps(value, indent=2))
        self.emit("```\n")
        self.blank()

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
        self.emit("```json\n")
        self.emit("{\n")
        # render input fields as BPL variables like |{var_name}|
        for input in inputs:
            self.emit(f'  "{input.name}": |{{{input.name}}}|,\n')
        # render output as giant hole of its type :JsonHole
        output_type_str = render_type_node(output, ignore_name=True, ignore_description=True)
        self.emit(f'  "{output.name}": |[{output_var}: {output_type_str}]|\n')
        self.emit("}\n")
        self.emit("```")
        self.blank()


async def compile(compilation: Compilation) -> CompilationResult:
    logger.debug("compile.start", compilation=compilation)
    if len(compilation.tasks) != 1 or len(compilation.models) != 1:
        raise CompileError(CompileErrorType.INTERNAL, compilation)

    state = CompilationState(compilation=compilation)
    candidate = CompilationCandidate(
        state=state, compilation=compilation, task=compilation.tasks[0], model=compilation.models[0]
    )
    await _compile_task(candidate, candidate.task)
    state.best_candidate = candidate
    logger.debug("compile.end", compilation=compilation, state=candidate)
    return state.best_candidate.to_result()


Expect = Union[Task, Code, Dataset, Expectation]


def _gather_expectations(symbol: Type | Expectation | Task) -> list[Expect]:
    expects = []
    if isinstance(symbol, Expectation):
        expects.append(symbol)
    if isinstance(symbol, (Type, Task, Expectation)):
        for child in symbol.expectations:
            if not isinstance(child, Expectation):
                expects.append(child)
            expects.extend(_gather_expectations(child))
    return expects


async def _compile_task(state: CompilationCandidate, task: Task) -> None:
    target_code = PromptBuilder(name=task.name, type_node=task.type)
    target_code.comment("Task metadata")
    target_code.emit(f'task "{task.name}"\n')
    target_code.emit(task.description + "\n")
    target_code.blank()

    # task type explanation
    target_code.comment("Task type instruction")
    target_code.emit_explain_type(task.type)

    # task type expectations (that apply to the task's types)
    for type in task.type.walk():
        if isinstance(type.reference, Type):
            type = type.reference
        if not isinstance(type, Type):
            continue
        type_examples = DataBuilder(name=type.name + " examples", type_node=type)
        for expect in _gather_expectations(type):
            if isinstance(expect, Dataset) and expect.modifier == StatementModifier.LIKE:
                type_examples.extend(expect.records)
                # ignore non-like datasets for now
        if type_examples.records:
            state.create_data(type_examples)
            target_code.emit(f"{type.name} should be used like this:")
            target_code.emit_show_dataset(type_examples)

    # task expectations (that apply to the task directly)
    task_expects = _gather_expectations(task)

    # task examples
    examples_type = TypeNode(
        name=task.name + "_unravelled",
        tag=TypeTag.STRUCT,
        children=[*task.type.input.children, task.type.output],
    )
    examples_data = DataBuilder(name=task.name + " examples", type_node=examples_type)
    for expect in task_expects:
        if isinstance(expect, Dataset) and expect.modifier == StatementModifier.LIKE:
            examples_data.extend(expect.records)
            # ignore non-like datasets for now

    # task example instruction
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
    for expect in task_expects:
        if isinstance(expect, Expectation):
            target_code.emit(" - " + expect.description + "\n")

    # task type example stub
    # (basically an example that is properly formatted but has obviously fake values)
    target_code.comment("Task type example stub")
    target_code.emit("The data should look like this (with real values obviously):\n")
    fake_data = fabricate(examples_type)
    target_code.emit_show_value(fake_data, examples_type)
    target_code.blank()

    # task input fields
    target_code.emit_get_record(task.type.input.children, task.type.output, "final_output")
    target_code.append("return final_output")

    # target pragma
    # TODO @Feature: generate task target pragma properly
    #  1. Get temperature from task description + type info
    #  2. Get max_tokens from emitted size..? Set max_new_tokens instead?
    target_code.pragma(
        temperature=0.5,
        max_tokens=1024,
        max_generated_tokens=1024,
        model=f'context["{state.model.name}"]',
    )

    state.create_data(examples_data)
    state.create_code(target_code)


def down(symbols: list[InterpSymbol], file: File | None = None) -> File:
    """Map high-level interpreted symbols back to lower level statements."""

    if file is None:
        file = File(module=Module(name="<generated>"), path="<generated>")

    # render symbols themselves
    order_keys = generate_n_keys_between(None, None, len(symbols))
    for order_key, symbol in zip(order_keys, symbols):
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
            order_key=order_key,
        )
        file.statements.append(statement)

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
