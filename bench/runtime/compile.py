from __future__ import annotations

import enum
from collections import OrderedDict
from contextlib import contextmanager
from dataclasses import dataclass, field
from typing import Optional
from uuid import uuid4

import structlog

from bench.language.reconstruct import render_type_node
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
    SymbolType,
    Task,
    TypeNode,
    TypeTag,
)

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
            id=uuid4(),
            name=builder.name,
            type_node=builder.type_node,
            records=builder.records,
            abstract=False,
            source=None,
            symbol_type=SymbolType.DATASET,
            modifier=None,
            context=OrderedDict(),
            description=None,
            language="jsonl",
        )
        self.target_symbols.append(dataset)
        return dataset

    def create_code(self, builder: PromptBuilder) -> Code:
        code = Code(
            id=uuid4(),
            name=builder.name,
            type_node=builder.type_node,
            language="bpl",
            code=builder.to_code_content(),
            abstract=False,
            builtin_id=None,
            symbol_type=SymbolType.CODE,
            modifier=None,
            context=OrderedDict(),
            description=None,
            source=None,
        )
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

    def emit_explain_type(self, node: TypeNode):
        """Explains the given type and all referenced types."""

        # accumulate pending types that we need to explain at the end
        explained_types = set()
        types_to_explain: dict[str, TypeNode] = OrderedDict()

        def _needs_explanation(node: TypeNode) -> bool:
            return (
                node.type not in PRIMITIVE_TYPES
                and node.type != TypeTag.ANY
                and node.type != TypeTag.LITERAL
            )

        def _consider_children(node: TypeNode):
            if not node.children:
                return
            for child in node.children:
                ref_name = _source_name(child)
                if _needs_explanation(child) and ref_name not in explained_types:
                    types_to_explain[child.source_reference] = child

        def _source_name(node: TypeNode) -> str:
            if node.source_reference:
                return node.source_reference
            else:  # make one up (that's stable)
                return id(node).__str__()

        def _render_type_node(node: TypeNode, is_root: bool = False) -> str:
            name_str = f"{node.name} = " if node.name else ""
            description_str = f' # "{node.description}"' if node.description else ""
            if node.source_reference and not is_root:
                # refer to types by their source reference unless we're at root
                #  (at 'root' we want to explain this type inline)
                return f"{name_str}{node.source_reference}{description_str}"
            elif node.type == TypeTag.UNION:
                union_str = " | ".join(_render_type_node(child) for child in node.children)
                return f"{name_str}{union_str}{description_str}"
            elif node.type == TypeTag.ARRAY:
                element_type = _render_type_node(node.children[0])
                return f"{name_str}{element_type}[]{description_str}"
            elif node.type == TypeTag.FUNCTION:
                func_strs = [
                    f"function {node.name}{description_str}:",
                    " # inputs",
                    *(_render_type_node(child) for child in node.input.children),
                    " # -> output",
                    _render_type_node(node.output),
                ]
                # since function inlines the input types, we need to consider children
                for child in node.children + node.input.children:
                    explained_types.add(_source_name(child))
                    _consider_children(child)
                return "\n".join(func_strs)
            elif node.type == TypeTag.STRUCT:
                struct_strs = [f"struct {name_str}{description_str}:"]
                for child in node.children:
                    struct_strs.append(_render_type_node(child))
                    explained_types.add(_source_name(child))
                    _consider_children(node)
                return "\n".join(struct_strs)
            elif node.type == TypeTag.ENUM:
                # assumes literal enum (only value members)
                enum_strs = [f"enum {name_str}{description_str}:"]
                for child in node.members:
                    enum_strs.append(f'"{child.name}": {child.value} # "{child.description}"')
                return "\n".join(enum_strs)
            elif node.type in PRIMITIVE_TYPES or node.type == TypeTag.ANY:
                return f"{name_str}{node.type.value}{description_str}"
            else:
                raise RuntimeError(f"unhandled type {node}")

        types_to_explain[_source_name(node)] = node
        while len(types_to_explain) > 0:
            node = types_to_explain.popitem()[1]
            self.emit_many(_render_type_node(node, is_root=True).splitlines())
            self.blank()
            explained_types.add(_source_name(node))
            _consider_children(node)  # see if we need to explain any children

    def emit_show_examples(self, examples: Dataset | DataBuilder):
        with self.block(f"for example in context['{examples.name}']:\n"):
            self.emit("{ ")
            for i, node in enumerate(examples.type_node.children):
                # weird _aliasing because BPL
                # 1) doesn't get variable scoping and
                # 2) can't understand [..] inside f-strings
                self.append(f"_{node.name} = example['{node.name}']")
                self.emit(f'"{node.name}" = {{_{node.name}}}')
                if i < len(examples.type_node.children) - 1:
                    self.emit(", ")
            self.emit(" }\n")
        self.blank()

    def emit_show_record(self, record: LiteralValue):
        raise NotImplementedError


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
    for child in symbol.expectations:
        expectations.extend(_gather_expectations(child))
    return expectations


async def _compile_task(state: CompilationState, task: Task) -> None:
    task_t = task.type_node
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
        type=TypeTag.STRUCT,
        children=[*task_t.input.children, task_t.output],
    )
    examples_data = DataBuilder(name=task.name + " examples", type_node=examples_type)

    # task example instruction
    # TODO @Incomplete: generate examples for task
    target_code.comment("Task example instruction")
    target_code.emit(f'examples for task "{task.name}":\n')
    target_code.emit(task.description + "\n")
    target_code.emit_show_examples(examples_data)

    # task inference
    target_code.comment("Task inference")
    # inline expectation restatement
    for expectation in task_expectations:
        if isinstance(expectation, Expectation):
            target_code.emit(expectation.description + "\n")

    # task input fields
    target_code.emit("{ ")
    for i, node in enumerate(task_t.input.children):
        target_code.emit(f'"{node.name}": "{{{node.name}}}"')
        target_code.emit(", ")
    # task final output fields (to be generated by model)
    # (currently only works if the output is a single valued field (no array or struct))
    field_type_str = render_type_node(task_t.output, ignore_name=True, ignore_description=True)
    target_code.emit(f'"{task_t.output.name}": [{task_t.output.name}: {field_type_str}]')
    target_code.emit(" }")
    target_code.append(f"return {task_t.output.name}")

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
