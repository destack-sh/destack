# ruff: noqa: N802


import ast
import inspect
import linecache
import textwrap
import types
from dataclasses import dataclass, field
from enum import StrEnum
from typing import Any, Mapping, override

import structlog
from opentelemetry import trace

from bench.language.const import new_struct_id
from bench.language.run import CodeKind

# NOTE: some of the analysis logic was adapted from marimo (Apache 2 licensed)
#  see https://github.com/marimo-team/marimo/blob/fec7d780488ab1478984468598d00d283e8c1c9d/marimo/_ast/visitor.py

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


@dataclass
class CodeImport:
    namespace: str | None = field(init=False)
    module: str  # full module name (e.g., a.b.c.)
    original_name: str | None = None  # `import a.b.c import d as e` -> d
    as_name: str | None = None  # `import a.b.c import d as e` -> e
    fully_qualified_name: str | None = None  # fully qualified name
    relative_level: int | None = None

    def __post_init__(self) -> None:
        self.namespace = self.module.split(".")[0]


class CodeDefinitionKind(StrEnum):
    FUNCTION = "function"
    CLASS = "class"
    IMPORT = "import"
    VARIABLE = "variable"


@dataclass
class CodeDefinition:
    """A definition of a name in a block."""

    kind: CodeDefinitionKind
    # If kind == function or class, it may be dependent on externally defined
    # variables.
    #
    # NOTE: This is populated by `ScopedVisitor.ref_stack`. Ref stack holds the
    #  references required for the current context, it's more general than a
    #  "block", since it covers all variable level interactions.
    # e.g.
    # >> x = foo + bar
    # x has the required refs foo and bar, and references holds that context
    #  while traversing the tree.
    references: set[str] = field(default_factory=set)
    imprt: CodeImport | None = None  # for imports


@dataclass
class CodeBlock:
    """A scope in which names are declared."""

    # Names defined with the global keyword
    global_names: set[str] = field(default_factory=set)
    # Map from defined names to metadata about their variables
    definitions: dict[str, CodeDefinition] = field(default_factory=dict)
    # Comprehensions have special scoping rules
    is_comprehension: bool = False

    def is_defined(self, name: str) -> bool:
        return any(name == defn for defn in self.definitions)


@dataclass
class CodeObscuredDefinition:
    """The scope in which a name is hidden."""

    # Variable id if this block hides a name
    name: str | None = None


@dataclass
class CodeReference:
    """Metadata about variables referenced but not defined."""

    # Whether the ref was deleted
    deleted: bool
    # Ancestors of the block in which this ref was used
    parent_blocks: list[CodeBlock]


def is_local_name(name: str):
    return name.startswith("_")


class CodeAnalysisVisitor(ast.NodeVisitor):
    """An AST visitor to do our code analysis."""

    def __init__(self, *, kind: CodeKind, code_id: str | None = None) -> None:
        self._kind = kind
        self._code_id = code_id or new_struct_id()
        self._block_stack: list[CodeBlock] = [CodeBlock()]
        self._ref_stack: list[set[str]] = [set()]  # names for CodeDefinition.references
        self._obscured_defn_stack: list[CodeObscuredDefinition] = []
        self._references: dict[str, CodeReference] = {}

    def __str__(self) -> str:
        return f"kind={self._kind}, code_id={self._code_id}, definitions={self.definitions}, references={self.references}"

    def __repr__(self) -> str:
        return f"<{self.__class__.__name__} {self}>"

    @property
    def references(self) -> Mapping[str, CodeReference]:
        """Get all the references."""
        return self._references

    @property
    def definitions(self) -> Mapping[str, CodeDefinition]:
        """Get top level definitions."""
        return self._block_stack[0].definitions

    def _mangle_if_needed(self, name: str, ignore_scope: bool = False) -> str:
        """
        Mangle local variable name declared at top-level scope if not in a function.
        """
        if (
            self._kind != CodeKind.FUNCTION
            and is_local_name(name)
            and (len(self._block_stack) == 1 or ignore_scope)
        ):
            return f"_{self._code_id}_{name}"
        else:
            return name

    def _get_alias_name(self, node: ast.alias) -> str:
        """
        Get the string name of an imported alias.

        NOTE: We disallow `import *` because Python only allows
         star imports at module-level, but we may run code as functions.
        """
        if node.asname is None:
            # Imported name without an "as" clause. Examples:
            #   import [a.b.c] - we define a
            #   from foo import [a] - we define a
            #   from foo import [*] - we don't define anything
            #
            # NOTE: don't mangle - user has no control over package name
            basename = node.name.split(".")[0]
            if basename == "*":
                line = f"line {node.lineno}" if hasattr(node, "lineno") else "line ..."
                raise SyntaxError(f"{line} SyntaxError: `import *` is not allowed.")
            return basename
        else:
            return self._mangle_if_needed(node.asname)

    def _is_defined(self, identifier: str) -> bool:
        """Check if `identifier` is defined in any block."""
        return any(block.is_defined(identifier) for block in self._block_stack)

    def _add_ref(self, name: str, deleted: bool) -> None:
        """Register a referenced name."""
        self._references[name] = CodeReference(
            deleted=deleted, parent_blocks=self._block_stack[:-1]
        )
        self._ref_stack[-1].add(name)

    def _remove_ref(self, name: str) -> None:
        """Remove a referenced name."""
        del self._references[name]

    def _define_in_block(self, name: str, variable_data: CodeDefinition, block_idx: int) -> None:
        """Define a name in a given block."""
        self._block_stack[block_idx].definitions[name] = variable_data
        # If `name` is added to the top-level block, it is also evicted from
        # any captured refs (if present) --- this handles cases where a name is
        # encountered and captured before it is declared, such as in
        #
        # ```
        # def f():
        #   print(x)
        # x = 0
        # ```
        if (
            name in self._references
            and self._block_stack[block_idx] in self._references[name].parent_blocks
        ):
            # `name` was used as a capture, not a reference
            self._remove_ref(name)

    def _define(self, name: str, variable: CodeDefinition) -> None:
        """
        Define a name in the current block.

        Names created with the global keyword are added to the top-level
        (global scope) block.
        """
        block_idx = 0 if name in self._block_stack[-1].global_names else -1
        self._define_in_block(name, variable, block_idx=block_idx)

    def _push_block(self, is_comprehension: bool) -> None:
        """Push a block onto the block stack."""
        self._block_stack.append(CodeBlock(is_comprehension=is_comprehension))

    def _pop_block(self) -> None:
        """Pop a block from the block stack."""
        self._block_stack.pop()

    def _push_obscured_definition(self, name: str | None) -> None:
        """Push scope onto the stack."""
        self._obscured_defn_stack.append(CodeObscuredDefinition(name=name))

    def _pop_obscured_definition(self) -> None:
        """Pop scope from the stack."""
        self._obscured_defn_stack.pop()

    def _visit_and_get_refs(self, node: ast.AST) -> set[str]:
        """
        Create a ref scope for the variable to be declared (e.g. function,
        class), visit the children the node, propagate the refs to the higher
        scope and then return the refs.
        """
        self._ref_stack.append(set())
        self.generic_visit(node)
        refs = self._ref_stack.pop()
        # The scope a level up from the one just investigated also is dependent
        # on these refs. Consider the case:
        # >> def foo():
        # >>   def bar(): <- current scope
        # >>     print(x)
        #
        # the variable `foo` needs to be aware that it may require the ref `x`
        # during execution.
        self._ref_stack[-1].update(refs)
        return refs

    @override
    def generic_visit(self, node: ast.AST) -> None:
        """
        Visits the children of node and manages the block stack.
        NOTE: visit calls visit_ClassName, or generic_visit() if not defined.
        That means that _this method should never call visit on `node`_, as this could recurse badly.
            (Calling visit on `node`'s children is fine.)
        In summary: call super().generic_visit on `node` and `visit()` on node's children.
        """
        if isinstance(node, (ast.ClassDef, ast.Lambda)):
            # These AST nodes introduce a new scope, but otherwise do not
            # require special treatment.
            self._push_block(is_comprehension=False)
            super().generic_visit(node)
            self._pop_block()
        elif isinstance(node, (ast.AsyncFunctionDef, ast.FunctionDef)):
            self._push_block(is_comprehension=False)
            # We need to visit generic type parameters before arguments
            # to make sure type parameters don't get added as refs. eg, in
            #
            #   def foo[U](u: U) -> U: ...
            #
            # `U` should not be a ref
            for child in node.type_params:
                self.visit(child)
            # This will revisit the type_params, but that's okay because
            # visiting is idempotent
            super().generic_visit(node)
            self._pop_block()
        elif isinstance(node, (ast.ListComp, ast.SetComp, ast.GeneratorExp)):
            # In comprehensions, generators must be visited before elements
            # because generators define local targets that elements may use.
            self._push_block(is_comprehension=True)
            for generator in node.generators:
                self.visit(generator)
            self.visit(node.elt)
            self._pop_block()
        elif isinstance(node, ast.DictComp):
            # Special-cased for the same reason that other comprehensions are
            # special-cased.
            self._push_block(is_comprehension=True)
            for generator in node.generators:
                self.visit(generator)
            self.visit(node.value)
            self.visit(node.key)
            self._pop_block()
        elif isinstance(node, (ast.Try, ast.TryStar)):
            # "Try" nodes have "handlers" that introduce exception context
            # variables that are tied to the try block, and don't exist beyond
            # it.
            for stmt in node.body:
                self.visit(stmt)
            for handler in node.handlers:
                self._push_obscured_definition(name=handler.name)
                self.visit(handler)
                self._pop_obscured_definition()
            for stmt in node.orelse:
                self.visit(stmt)
            for stmt in node.finalbody:
                self.visit(stmt)
        elif isinstance(node, ast.TypeAlias):
            self.visit(node.name)
            self._push_block(is_comprehension=False)
            for t in node.type_params:
                self.visit(t)
            self.visit(node.value)
            self._pop_block()
        else:
            # Other nodes that don't introduce a new scope
            super().generic_visit(node)

    # ClassDef and FunctionDef nodes don't have ast.Name nodes as children
    @override
    def visit_ClassDef(self, node: ast.ClassDef) -> None:
        node.name = self._mangle_if_needed(node.name)
        refs = self._visit_and_get_refs(node)
        self._define(
            node.name,
            CodeDefinition(kind=CodeDefinitionKind.CLASS, references=refs),
        )

    @override
    def visit_AsyncFunctionDef(self, node: ast.AsyncFunctionDef) -> None:
        node.name = self._mangle_if_needed(node.name)
        refs = self._visit_and_get_refs(node)
        self._define(
            node.name,
            CodeDefinition(kind=CodeDefinitionKind.FUNCTION, references=refs),
        )

    @override
    def visit_FunctionDef(self, node: ast.FunctionDef) -> None:
        node.name = self._mangle_if_needed(node.name)
        refs = self._visit_and_get_refs(node)
        self._define(
            node.name,
            CodeDefinition(kind=CodeDefinitionKind.FUNCTION, references=refs),
        )

    @override
    def visit_Call(self, node: ast.Call) -> None:
        ...  # NOTE :Incomplete: parse special Bench functions (resolve path)

        # Visit arguments, keyword args, etc.
        self.generic_visit(node)

    @override
    def visit_Lambda(self, node: ast.Lambda) -> None:
        # Inject the dummy name `_lambda` into ref scope to denote there's a
        # callable that might require additional refs.
        self._ref_stack[-1].add("_lambda")
        self.generic_visit(node)

    @override
    def visit_arg(self, node: ast.arg) -> None:
        node.arg = self._mangle_if_needed(node.arg)
        self._define(node.arg, CodeDefinition(kind=CodeDefinitionKind.VARIABLE))
        if node.annotation is not None:
            self.visit(node.annotation)

    @override
    def visit_arguments(self, node: ast.arguments) -> None:
        # process potential refs before defs, to handle patterns like
        #
        # def f(x=x):
        #   ...
        for v in node.kw_defaults:
            if v is not None:
                self.visit(v)
        for v in node.defaults:
            if v is not None:
                self.visit(v)

        for arg in node.posonlyargs:
            self.visit(arg)
        for arg in node.args:
            self.visit(arg)
        for arg in node.kwonlyargs:
            self.visit(arg)
        if node.vararg is not None:
            self.visit(node.vararg)
        if node.kwarg is not None:
            self.visit(node.kwarg)

    @override
    def visit_Assign(self, node: ast.Assign) -> None:
        # Visit the value first, to handle cases like
        #
        # class A:
        #   x = x
        #
        # Handling value first is required to register `x` as a ref.
        self._ref_stack.append(set())
        self.visit(node.value)
        for target in node.targets:
            self.visit(target)
        refs = self._ref_stack.pop()
        self._ref_stack[-1].update(refs)

    @override
    def visit_AugAssign(self, node: ast.AugAssign) -> None:
        # Augmented assign (has op)
        # e.g., x += 1
        self._ref_stack.append(set())
        self.visit(node.value)
        self.visit(node.target)
        refs = self._ref_stack.pop()
        self._ref_stack[-1].update(refs)

    @override
    def visit_AnnAssign(self, node: ast.AnnAssign) -> None:
        # Annotated assign
        # e.g., x: int = 0
        self._ref_stack.append(set())
        if node.value is not None:
            self.visit(node.value)
        self.visit(node.annotation)
        self.visit(node.target)
        refs = self._ref_stack.pop()
        self._ref_stack[-1].update(refs)

    @override
    def visit_NamedExpr(self, node: ast.NamedExpr) -> None:
        self.visit(node.value)
        if self._block_stack[-1].is_comprehension and isinstance(node.target, ast.Name):
            for block_idx, block in reversed(list(enumerate(self._block_stack))):
                # go up the block stack until we find the first
                # non-comprehension block
                if not block.is_comprehension:
                    node.target.id = self._mangle_if_needed(
                        node.target.id,
                        ignore_scope=(block == self._block_stack[0]),
                    )
                    self._define_in_block(
                        node.target.id,
                        CodeDefinition(kind=CodeDefinitionKind.VARIABLE),
                        block_idx=block_idx,
                    )
                    break
        else:
            self.generic_visit(node)

    @override
    def visit_Name(self, node: ast.Name) -> None:
        # NOTE: AugAssign has a Store ctx; this means that mutating a var
        # will create a def, which we can catch as an error later if
        # that var was also defined elsewhere in the same scope.
        #
        # NOTE: Only mangle loaded or deleted names if they are local
        # and found to be referring to a top-level variable. This prevents
        # us from mangling references to variables names conforming to local
        # spec but declared in a nested scope.
        #
        # NOTE: we don't implement visit_Attribute because refs and defs
        # are not tracked at the attribute level. The default behavior
        # with our implemented visitors does the right thing (foo.bar[.*]
        # generates a ref to foo if foo has not been def'd).
        #
        # NOTE: Nodes like "Try" nodes introduce variable names that do not exist
        # beyond their inner scope. We traverse blocks to see if the name is
        # "obscured" in this way.

        for scope in self._obscured_defn_stack:
            if node.id == scope.name:
                self.generic_visit(node)
                return

        if isinstance(node.ctx, ast.Store):
            node.id = self._mangle_if_needed(node.id)
            self._define(
                node.id,
                CodeDefinition(kind=CodeDefinitionKind.VARIABLE, references=self._ref_stack[-1]),
            )
        elif (
            isinstance(node.ctx, ast.Load)
            and not self._is_defined(node.id)
            and not is_local_name(node.id)
        ):
            self._add_ref(node.id, deleted=False)
        elif (
            isinstance(node.ctx, ast.Del)
            and not self._is_defined(node.id)
            and not is_local_name(node.id)
        ):
            self._add_ref(node.id, deleted=True)
        elif is_local_name(node.id):
            mangled_name = self._mangle_if_needed(node.id, ignore_scope=True)
            for block in reversed(self._block_stack):
                if block == self._block_stack[0] and block.is_defined(mangled_name):
                    node.id = mangled_name
                elif block.is_defined(node.id):
                    break

        # Handle refs on the block scope level, or capture top level references
        if (
            isinstance(node.ctx, ast.Load)
            and self._is_defined(node.id)
            and node.id not in self._ref_stack[-1]
            and (node.id not in self._block_stack[-1].definitions or len(self._block_stack) == 1)
        ):
            self._ref_stack[-1].add(node.id)

        self.generic_visit(node)

    @override
    def visit_Global(self, node: ast.Global) -> None:
        node.names = [self._mangle_if_needed(name, ignore_scope=True) for name in node.names]
        for name in node.names:
            self._block_stack[-1].global_names.add(name)
            self._add_ref(name, deleted=False)

    @override
    def visit_Import(self, node: ast.Import) -> None:
        for alias_node in node.names:
            variable_name = self._get_alias_name(alias_node)
            imprt = CodeImport(module=alias_node.name, fully_qualified_name=None)
            self._define(variable_name, CodeDefinition(kind=CodeDefinitionKind.IMPORT, imprt=imprt))

    @override
    def visit_ImportFrom(self, node: ast.ImportFrom) -> None:
        module = node.module if node.module is not None else ""
        # we don't recurse into the alias nodes, since we define the
        # aliases here
        for alias_node in node.names:
            alias_name = self._get_alias_name(alias_node)
            original_name = alias_node.name
            imprt = CodeImport(
                module=module,
                fully_qualified_name=module + "." + original_name,
                original_name=original_name,
                as_name=alias_name,
                relative_level=node.level,
            )
            self._define(alias_name, CodeDefinition(kind=CodeDefinitionKind.IMPORT, imprt=imprt))

    @override
    def visit_MatchAs(self, node: ast.MatchAs) -> None:
        if node.name is not None:
            node.name = self._mangle_if_needed(node.name)
            self._define(node.name, CodeDefinition(kind=CodeDefinitionKind.VARIABLE))
        if node.pattern is not None:
            # pattern may contain additional MatchAs statements in it
            self.visit(node.pattern)

    @override
    def visit_MatchMapping(self, node: ast.MatchMapping) -> None:
        if node.rest is not None:
            node.rest = self._mangle_if_needed(node.rest)
            self._define(node.rest, CodeDefinition(kind=CodeDefinitionKind.VARIABLE))
        for key in node.keys:
            self.visit(key)
        for pattern in node.patterns:
            self.visit(pattern)

    @override
    def visit_MatchStar(self, node: ast.MatchStar) -> None:
        if node.name is not None:
            node.name = self._mangle_if_needed(node.name)
            self._define(
                node.name,
                CodeDefinition(kind=CodeDefinitionKind.VARIABLE),
            )

    @override
    def visit_TypeVar(self, node: ast.TypeVar) -> None:
        # node.name is a str, not an ast.Name node
        self._define(
            node.name,
            CodeDefinition(kind=CodeDefinitionKind.VARIABLE, references=self._ref_stack[-1]),
        )
        if isinstance(node.bound, tuple):
            for name in node.bound:
                self.visit(name)
        elif node.bound is not None:
            self.visit(node.bound)

    @override
    def visit_ParamSpec(self, node: ast.ParamSpec) -> None:
        # node.name is a str, not an ast.Name node
        self._define(
            node.name,
            CodeDefinition(kind=CodeDefinitionKind.VARIABLE, references=self._ref_stack[-1]),
        )

    @override
    def visit_TypeVarTuple(self, node: ast.TypeVarTuple) -> None:
        # node.name is a str, not an ast.Name node
        self._define(
            node.name,
            CodeDefinition(kind=CodeDefinitionKind.VARIABLE, references=self._ref_stack[-1]),
        )


def _is_coroutine(co: types.CodeType) -> bool:
    """Check if a code object is a coroutine."""
    return co is not None and inspect.CO_COROUTINE & co.co_flags == inspect.CO_COROUTINE


@dataclass
class CompiledCode:
    """Compiled and analysed Code."""

    kind: CodeKind
    code: str
    transformed_code: str
    transformation: "CodeTransformation | None"
    definitions: Mapping[str, "CodeDefinition"]
    references: Mapping[str, "CodeReference"]
    imports: Mapping[str, "CodeImport"]
    syntax_error: SyntaxError | None  # in case code is not valid
    module: ast.Module | None  # the entire parsed AST
    body_co: types.CodeType | None  # the compiled code object (excl. last_expr if kind=snippet)
    last_expr: ast.Expression | None  # for snippets
    last_expr_co: types.CodeType | None  # for snippets
    function_name: str | None  # for functions
    is_coroutine: bool

    @property
    def cache_key(self):
        return hash((self.kind, self.code))


@dataclass
class CodeTransformation:
    """
    Simple source mapping for transformed code.
    NOTE :Incomplete: CodeTransformation should contain all transformations (like async, paths, etc.)
     (more like a source map so we can attribute every line/column perfectly to the original code)
    """

    line_offset: int
    column_offset: int

    def forward(self, line: int, column: int) -> tuple[int, int]:
        return line + self.line_offset, column + self.column_offset

    def reverse(self, line: int, column: int) -> tuple[int, int]:
        return line - self.line_offset, column - self.column_offset


def _get_filename(code_id: str, suffix: str = "") -> str:
    return f"<_code_{code_id}{suffix}>"


def _cache_in_linecache(filename: str, code: str) -> None:
    linecache.cache[filename] = (
        len(code),
        None,
        code.splitlines(True),
        filename,
    )


@tracer.start_as_current_span("compiler.compile_code")
def compile_code(
    code_id: str,
    code: str,
    kind: CodeKind,
    glbls: Mapping[str, Any],
) -> CompiledCode:
    """
    Parse, analyze and compile code.
    """
    assert code_id.isalnum(), f"code_id must be alphanumeric: {code_id}"

    # wrap code in function if it's a function
    function_name = f"_code_{code_id}"
    if kind == CodeKind.FUNCTION:
        # compile to figure out if it's a coroutine (simple string matching wouldn't work)
        # NOTE :Performance: we compile twice to figure out if functions are async before wrapping
        module = compile(
            # can't return at top level
            code.replace("return ", "_____ =").replace("return", "pass"),
            "<code>",
            mode="exec",
            flags=ast.PyCF_ALLOW_TOP_LEVEL_AWAIT,
        )
        is_coroutine = _is_coroutine(module)
        if is_coroutine:
            transformed_code = f"async def {function_name}():\n{textwrap.indent(code, 4 * " ")}"
        else:
            transformed_code = f"def {function_name}():\n{textwrap.indent(code, 4 * " ")}"
        transformation = CodeTransformation(line_offset=1, column_offset=4)
    else:
        is_coroutine = None
        transformed_code = code
        transformation = None

    # store the code in Python's linecache so debuggers can find it
    body_filename = _get_filename(code_id)
    _cache_in_linecache(body_filename, transformed_code)

    # compile into AST
    try:
        module = compile(
            transformed_code,
            body_filename,
            mode="exec",
            flags=ast.PyCF_ONLY_AST | ast.PyCF_ALLOW_TOP_LEVEL_AWAIT,
        )
        syntax_error = None
    except SyntaxError as e:
        module = None
        syntax_error = e
    if not module or not module.body:
        return CompiledCode(
            kind=kind,
            code=code,
            transformed_code=transformed_code,
            transformation=None,
            imports={},
            definitions={},
            references={},
            syntax_error=syntax_error,
            module=module,
            body_co=None,
            last_expr=None,
            last_expr_co=None,
            function_name=None,
            is_coroutine=False,
        )

    # analyze
    analysis = CodeAnalysisVisitor(kind=kind, code_id=code_id)
    analysis.visit(module)
    if kind == CodeKind.FUNCTION:
        # remove the wrapped function definition from the analysis
        analysis._block_stack[0].definitions.pop(function_name)

    # compile into code object
    body_co = compile(module, body_filename, mode="exec", flags=ast.PyCF_ALLOW_TOP_LEVEL_AWAIT)

    # parse out last expression for snippets (and compile that)
    last_expr: ast.Expression | None
    if kind == CodeKind.SNIPPET:
        if isinstance(module.body[-1], ast.Expr):
            last_expr = ast.Expression(module.body.pop().value)
        else:
            last_expr = ast.Expression(ast.Constant(None))
        last_expr_filename = _get_filename(code_id, suffix="_last_expr")
        _cache_in_linecache(
            last_expr_filename,
            ast.unparse(last_expr) if not isinstance(last_expr, str) else "None",
        )
        last_expr_co = compile(
            last_expr, last_expr_filename, mode="eval", flags=ast.PyCF_ALLOW_TOP_LEVEL_AWAIT
        )
        is_coroutine = _is_coroutine(body_co) or _is_coroutine(last_expr_co)
    else:
        last_expr = None
        last_expr_co = None
        if is_coroutine is None:
            is_coroutine = _is_coroutine(body_co)

    # remove globals from references (they are references)
    definitions = analysis.definitions
    references = {k: v for k, v in analysis.references.items() if k not in glbls}
    imports = {k: v.imprt for k, v in definitions.items() if v.imprt is not None}

    return CompiledCode(
        kind=kind,
        code=code,
        transformed_code=transformed_code,
        transformation=transformation,
        definitions=definitions,
        references=references,
        imports=imports,
        syntax_error=syntax_error,
        module=module,
        body_co=body_co,
        last_expr=last_expr,
        last_expr_co=last_expr_co,
        function_name=function_name,
        is_coroutine=is_coroutine,
    )
