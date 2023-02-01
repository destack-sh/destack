from __future__ import annotations

import ast
import enum
from collections import OrderedDict
from contextlib import contextmanager
from dataclasses import dataclass, field
from typing import Any, Optional

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
    Task,
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

    def emit_explain_type(self, node: TypeNode):
        """Emits BPL to explain the given type and all its references."""
        # TODO @Cleanup: emit_explain_type is unwieldy (esp. _render_type_node)

        # accumulate pending types that we need to explain at the end
        explained_types = set()
        types_to_explain: dict[str, TypeNode] = OrderedDict()

        def _needs_explanation(node: TypeNode) -> bool:
            return (
                node.tag not in PRIMITIVE_TYPES
                and node.tag != TypeTag.ANY
                and node.tag != TypeTag.LITERAL
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
            name_str = f"{node.name}: " if node.name else ""
            description_str = f' # "{node.description}"' if node.description else ""
            if node.source_reference and not is_root:
                # refer to types by their source reference unless we're at root
                #  (at 'root' we want to explain this type inline)
                return f"{name_str}{node.source_reference}{description_str}"
            elif node.tag == TypeTag.UNION:
                union_str = " | ".join(_render_type_node(child) for child in node.children)
                return f"{name_str}{union_str}{description_str}"
            elif node.tag == TypeTag.ARRAY:
                element_type = _render_type_node(node.children[0])
                return f"{name_str}{element_type}[]{description_str}"
            elif node.tag == TypeTag.FUNCTION:
                func_strs = [
                    f"func {node.name}{description_str} {{",
                    *("  " + _render_type_node(child) for child in node.input.children),
                    "  " + _render_type_node(node.output),
                ]
                # since function inlines the input types, we need to consider children
                for child in node.children + node.input.children:
                    explained_types.add(_source_name(child))
                    _consider_children(child)
                return "\n".join(func_strs) + "\n}"
            elif node.tag == TypeTag.STRUCT:
                struct_strs = [f"struct {node.source_reference}{description_str} {{"]
                for child in node.children:
                    struct_strs.append("  " + _render_type_node(child))
                    explained_types.add(_source_name(child))
                    _consider_children(node)
                return "\n".join(struct_strs) + "\n}"
            elif node.tag == TypeTag.ENUM:
                # assumes literal enum (only value members)
                enum_strs = [f"enum {node.source_reference}{description_str} {{"]
                for child in node.members:
                    enum_strs.append(f'  "{child.value}" # "{child.description or child.name}"')
                return "\n".join(enum_strs) + "\n}"
            elif node.tag in PRIMITIVE_TYPES or node.tag == TypeTag.ANY:
                return f"{name_str}{node.tag.value}{description_str}"
            else:
                raise RuntimeError(f"unhandled type {node}")

        types_to_explain[_source_name(node)] = node
        while len(types_to_explain) > 0:
            node = types_to_explain.popitem()[1]
            for line in _render_type_node(node, is_root=True).splitlines():
                self.emit(line + "\n")
            self.blank()
            explained_types.add(_source_name(node))
            _consider_children(node)  # see if we need to explain any children

    def emit_show_examples(self, examples: Dataset | DataBuilder):
        """Emits BPL to show the given examples."""
        with self.block(f"for example in context['{examples.name}']:\n"):
            self.emit("{ \n")
            for i, node in enumerate(examples.type.children):
                # weird _aliasing because BPL
                # 1) doesn't get variable scoping and
                # 2) can't understand [..] inside f-strings
                self.append(f"_{node.name} = example['{node.name}']")
                self.emit(f'  "{node.name}" = {{_{node.name}}}')
                self.emit(",\n")
            self.emit("}\n")
        self.blank()

    def emit_show_record(self, value: Any, type: TypeNode, indent: str = ""):
        """Emits BPL to show the given example."""

        def _emit(s: str):
            self.emit(indent + s)

        if type.tag == TypeTag.STRUCT:
            _emit("\\{ \n")
            for node in type.children:
                _emit(f'  "{node.name}": ')
                self.emit_show_record(value[node.name], node, indent + "  ")
                _emit(",\n")
            _emit("\\}\n")
        elif type.tag == TypeTag.ARRAY:
            _emit("\\[ \n")
            for val in value:
                self.emit_show_record(val, type.head_type, indent + "  ")
                _emit(",\n")
            _emit("\\]\n")
        elif type.is_flat:  # ignore indent
            pfix_str = '"' if type.tag == TypeTag.STRING else ""
            self.emit(f"{pfix_str}{value}{pfix_str}")
        else:
            raise RuntimeError(f"unhandled type {type}")

    def emit_get_record(
        self,
        inputs: list[TypeNode],
        output: TypeNode,
        result_var: str = "output",
        indent: str = "",
        open_brace: bool = True,
        close_brace: bool = True,
    ):
        """Emits BPL to get the output record given the inputs"""
        # TODO @Cleanup: emit_show_examples, emit_show_record and emit_get_record are 3 sides of the same coin
        #  (using runtime records, using static records, mixing runtime and model-generated records)
        #  The way they're currently implemented is unwieldy and requires 3 sites to be updated for format changes.
        #  Either we force JSON for everything or use a "format" class to handle all specific formatting.
        # like for example, we create a json-like object

        def _emit(s: str):
            self.emit(indent + s)

        if open_brace:
            _emit("{\n")
        # inputs
        if any(not input.is_flat for input in inputs):
            raise RuntimeError(f"non-flat inputs not yet supported: {inputs}")
        for i, node in enumerate(inputs):
            pfix_str = '"' if node.tag == TypeTag.STRING else ""
            _emit(f'  "{node.name}": {pfix_str}{{{node.name}}}{pfix_str},\n')

        # outputs
        if output.is_flat:  # simple case
            field_type_str = render_type_node(output, ignore_name=True, ignore_description=True)
            pfix_str = '"' if output.tag == TypeTag.STRING else ""
            _emit(f'  "{output.name}": {pfix_str}[{result_var}: {field_type_str}]{pfix_str},\n')
        elif output.tag == TypeTag.ARRAY:
            self.append(f"{result_var} = []")
            _emit(f'  "{output.name}": [\n')
            with self.block("while True:"):
                _emit("  ")
                # single character continuation signal
                _emit('[cont: `" "` | `"\\{"` | `"\\]"`]')
                self.append("if cont == ']':")
                self.append("    break")
                self.append("elif cont == ' ':")
                self.append('    " "')  # re-add extra space to align
                self.append("elif cont == '{':")
                self.append('    "\\n"')  # consume newline of new object
                self.emit_get_record(
                    inputs=[],
                    output=output.head_type,
                    result_var="element",
                    open_brace=False,  # already emitted above
                    indent=indent + "  ",
                )
                self.append(f"{result_var}.append(element)")
        elif output.tag == TypeTag.STRUCT:
            for node in output.children:
                if not node.is_flat:
                    raise RuntimeError(f"non-flat struct output fields not yet supported: {node}")
                field_type_str = render_type_node(node, ignore_name=True, ignore_description=True)
                _emit(f'  "{node.name}": [{node.name}: {field_type_str}],\n')
            init_args = ", ".join(f"{node.name}={node.name}" for node in output.children)
            self.append(f"{result_var} = {output.source_reference}({init_args})")
        else:
            raise RuntimeError(f"output type supported: {output}")
        if close_brace:
            _emit("}")


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
        target_code.emit_show_examples(examples_data)

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
    target_code.emit_get_record(task_t.input.children, task_t.output, result_var="final_output")
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
