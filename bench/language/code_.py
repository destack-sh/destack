from __future__ import annotations

import ast
import hashlib
import itertools
import textwrap
import types
import typing
from dataclasses import field
from functools import cached_property
from json import JSONDecodeError
from random import Random
from typing import Any, Optional

import structlog
from more_itertools import first, last

from bench.language import IssueType
from bench.language.const import NodePath
from bench.language.field import HasFields, TypedDict
from bench.language.mapping import check_type, pack_value, unpack_value
from bench.language.module import LookupBy, ModuleNode, Scope, node
from bench.language.query import Q, Query, QueryOp, Sort, SortMode, SortOrder
from bench.language.remote import RemoteObject, RemoteObjectStatus
from bench.utils.dt import utcnow_with_tz
from bench.utils.utils import get_from_env

if typing.TYPE_CHECKING:
    from bench.language import ModuleVisitor, Statement

logger = structlog.get_logger(__name__)


@node
class CodeTransformation:
    original_code: str
    transformed_code: str
    method_name: str
    start_offset: int
    end_offset: int


@node
class CodeParse:
    references: dict[str, NodePath] = field(default_factory=dict)
    is_async: bool = False
    x_imports: dict[int, dict[str, NodePath]] = field(default_factory=dict)


@node(tracked=["code"])
class HasCode(HasFields, ModuleNode):
    code: str | None = None
    _is_async: Optional[bool] = None
    _parse: Optional[CodeParse] = None
    _transform: Optional[CodeTransformation] = None
    _statement_references: dict[str, "Statement"] | None = None
    _callable_wrapped: AsyncCodeCallable | SyncCodeCallable | None = None
    _cached_exports: dict[str, Any] | None = None

    def _clear(self) -> None:
        self._parse = None
        self._transform = None
        self._statement_references = None
        self._callable_wrapped = None
        self._cached_exports = None

    def _index(self) -> None:
        pass

    def _interp(self, scope: Scope) -> None:
        self._parse = _parse_code(self.code)
        self._is_async = self._parse.is_async
        self._statement_references = {}
        self._code_export_references = {}
        for key, reference in self._parse.references.items():
            resolved = scope.lookup(reference, by=LookupBy.PyIdent)
            if resolved is not None:
                self._statement_references[key] = resolved

        # check if code is exportable if marked as such
        if self.exported:
            if len(self.fields) > 0:
                self._on_issue(type=IssueType.CODE_NOT_EXPORTABLE, subject=self)

    def _visit(self, visitor: "ModuleVisitor") -> None:
        pass  # should visit statement references?

    @cached_property
    def cached(self) -> bool:
        # TODO @Cleanup: manage stdlib references centrally :CentralStdlibAccess
        from bench.language.libs import symbolx_lib

        return self.has_tag(symbolx_lib.lookup_or_error(".builtins.cache"))

    @cached_property
    def exported(self) -> bool:
        from bench.language.libs import symbolx_lib  # :CentralStdlibAccess

        return self.has_tag(symbolx_lib.lookup_or_error(".builtins.export"))

    @cached_property
    def is_test(self) -> bool:
        from bench.language.libs import symbolx_lib  # :CentralStdlibAccess

        return self.has_tag(symbolx_lib.lookup_or_error(".builtins.test"))

    @cached_property
    def _code_hash(self) -> str:
        return hashlib.sha256(self.code.encode("utf-8")).hexdigest()

    def _do_import_sync(self, path: str, name: str) -> tuple[Any, ...]:
        """Import a statement or exported Python object at runtime."""
        reference = NodePath(path, name)
        resolved = self.lookup(reference, by=LookupBy.PyIdent)
        if resolved is None:
            # fall back to code object import
            resolved = self.lookup(reference.path, by=LookupBy.PyIdent)
            if not isinstance(resolved, HasCode) or not resolved.exported:
                raise ImportError(f"cannot import '{path}.{name}'->{resolved} (is it exported?)")
            ret = resolved.to_sync()()
            if name not in ret:
                raise ImportError(f"cannot import '{path}.{name}'->{resolved} (no such export)")
            return ret[name]
        return resolved

    async def _do_import_async(self, path: str, name: str) -> tuple[Any, ...]:
        """Import a statement or exported Python object at runtime."""
        reference = NodePath(path, name)
        resolved = self.lookup(reference, by=LookupBy.PyIdent)
        if resolved is None:
            # fall back to code object import
            resolved = self.lookup(reference.path, by=LookupBy.PyIdent)
            if not isinstance(resolved, HasCode) or not resolved.exported:
                raise ImportError(f"cannot import '{path}.{name}'->{resolved} (is it exported?)")
            ret = await resolved.to_async()()
            if name not in ret:
                raise ImportError(f"cannot import '{path}.{name}'->{resolved} (no such export)")
            return ret[name]
        return resolved

    def _import_sync(self, path: str, *names: str) -> tuple[Any, ...]:
        if len(names) == 1:
            return self._do_import_sync(path, names[0])
        return tuple(self._do_import_sync(path, name) for name in names)

    async def _import_async(self, path: str, *names: str) -> tuple[Any, ...]:
        if len(names) == 1:
            return await self._do_import_async(path, names[0])
        return tuple(await self._do_import_async(path, name) for name in names)

    def _prep_locals(self) -> dict[str, Any]:
        """Gets the locals required for the code to run."""
        # assemble context
        from .run import HasRun

        context = {**self._statement_references}
        if not self._parse.is_async:
            # replace any async functions with sync versions
            for key, symbol in context.items():
                if isinstance(symbol, HasRun) and symbol._is_async:
                    context[key] = symbol.to_sync()
        dynamic_context = {
            "session": self.session,
            "context": {symbol.name: symbol for symbol in context.values()},  # by name
            "cache": self.session.cache_async if self._parse.is_async else self.session.cache_sync,
            "storage": self.session.storage,
            **context,  # inlined
            "random": Random(self.id.hex.encode()),
            "self": self,
            "ximport": self._import_sync if not self._parse.is_async else self._import_async,
        }

        import bench.language

        imports = {
            # everything from bench.language
            **{k: v for k, v in bench.language.__dict__.items() if not k.startswith("_")},
        }

        if self.is_test:  # very crude initial test support
            import pytest

            imports["pytest"] = pytest

        locals = {**imports, **STATIC_BUILTINS, **dynamic_context}
        return locals

    def _prep_func_body(self) -> tuple[str, int, int]:
        """Prepares the function body of this code with all modifications."""
        func_body_lines = ((self.code.strip() if self.code else None) or "pass").splitlines()
        # replace real python x imports with _ximport
        # e.g. replace `from .utils import a, b` with `a, b = _ximport(".utils", "a", "b")`
        await_str = "await " if self._parse.is_async else ""
        for i, x_refs in self._parse.x_imports.items():
            x_paths = list(x_refs.values())
            path = x_paths[0].path
            keys_str = ", ".join(repr(k) for k in x_refs)
            line = f"{', '.join(x_refs)} = {await_str}ximport({path!r}, {keys_str})"
            func_body_lines[i] = line

        if self.exported:
            # capture all locals at the end of the function
            # remember locals at the start of the function to exclude them
            func_body_lines.insert(0, "__locals_start = locals().copy()")
            func_body_lines.append("__locals_end = locals().copy()")
            func_body_lines.append(
                "return {k: v for k, v in __locals_end.items() if k not in __locals_start}"
            )
            return "\n".join(func_body_lines), 1, 2
        else:
            return "\n".join(func_body_lines), 0, 0

    def _prepare_callable(self) -> None:
        """Creates a callable wrapping this code for execution with the required context."""
        if self._callable_wrapped is not None:
            return

        locals = self._prep_locals()
        func_body, start_offset, end_offset = self._prep_func_body()
        func_name = self.py_ident or "_anon" + self.id.hex[:6]
        func_params = ", ".join(i.py_ident + "=None" for i in self.inputs)
        try:
            method_str = f"def {func_name}({func_params}):\n{textwrap.indent(func_body, ' ' * 4)}"
            if self._parse.is_async:
                method_str = f"async {method_str}"
            self._callable_wrapped = self._make_callable(method_str, locals, func_name)
        except SyntaxError as e:
            # raise error in code when called for proper reporting
            err_str = e.args[1][3] or str(e)
            raise_str = f"raise {e.__class__.__name__}('invalid syntax: ' + {err_str})"
            indented_raise = textwrap.indent(raise_str, " " * 4)
            method_str = f"def {func_name}({func_params}):\n{indented_raise}"
            self._callable_inner = _do_exec_get_globals(method_str, locals)[func_name]
            self._callable_wrapped = self._callable_inner
        self._transform = CodeTransformation(
            original_code=self.code,
            transformed_code=method_str,
            start_offset=start_offset + 1,  # for method signature
            end_offset=end_offset,
            method_name=func_name,
        )

    def _make_callable(
        self, code_str: str, locals: dict[str, Any], func_name: str
    ) -> typing.Callable:
        if self.is_test:
            from _pytest.assertion.rewrite import rewrite_asserts

            # rewrite asserts for better debugging
            tree = ast.parse(code_str)
            rewrite_asserts(tree, code_str.encode())
            co = compile(tree, "<string>", "exec", dont_inherit=True)
            callable = _do_exec_get_globals(co, locals)[func_name]
        else:
            callable = _do_exec_get_globals(code_str, locals)[func_name]
        if self.exported:
            callable = self._wrap_exported(callable)
        elif self.cached:
            callable = self._wrap_cached(callable)
        if self.is_test:
            callable = self._wrap_test(callable)
        return callable

    def _wrap_cached(self, callable: AsyncCodeCallable | SyncCodeCallable) -> typing.Callable:
        from bench.language.run import CachedRun, get_run_cache_subkey

        def _get_cached_output(inputs: dict, cached_run: bytes) -> Optional[dict]:
            try:
                from .run import CachedRun

                run = CachedRun.from_json_bytes(cached_run)
                outputs = unpack_value(run.outputs, self, ignore_outer_map=True, is_output=True)
                check_type(outputs, self, is_output=True)
                self.session.tracer.run_cached(
                    self, inputs, outputs, run.generated_at, run.duration
                )
                return TypedDict(self, outputs)
            except (ValueError, TypeError, JSONDecodeError) as e:
                logger.exception("code.cache.error", e=e, excinfo=e)
                # ignore, will be overwritten on success
                return None

        def _cached_sync(*args, **kwargs):
            inputs = self._inputs_from_args(args, kwargs)
            inputs_raw = pack_value(inputs, self, is_output=False)

            cache_subkey = get_run_cache_subkey(inputs_raw=inputs_raw, content_id=self._code_hash)
            cached_run = self.cache.get(cache_subkey)
            cached_output = _get_cached_output(inputs, cached_run) if cached_run else None
            if cached_output is not None:
                return cached_output
            started_at = utcnow_with_tz()

            result = callable(*args, **kwargs)
            outputs_raw = pack_value(result, self, is_output=True)
            run_bytes = CachedRun.bytes_from_run(inputs_raw, outputs_raw, started_at)
            self.cache.set(cache_subkey, run_bytes)
            return result

        async def _cached_async(*args, **kwargs):
            # yes this is annoyingly duplicated...
            inputs = self._inputs_from_args(args, kwargs)
            inputs_raw = pack_value(inputs, self, is_output=False)

            cache_subkey = get_run_cache_subkey(inputs_raw=inputs_raw, content_id=self._code_hash)
            cached_run = await self.cache.get(cache_subkey)
            cached_output = _get_cached_output(inputs, cached_run) if cached_run else None
            if cached_output is not None:
                return cached_output
            started_at = utcnow_with_tz()

            result = await callable(*args, **kwargs)
            outputs_raw = pack_value(result, self, is_output=True)
            run_bytes = CachedRun.bytes_from_run(inputs_raw, outputs_raw, started_at)
            await self.cache.set(cache_subkey, run_bytes)
            return result

        return _cached_async if self._parse.is_async else _cached_sync

    def _wrap_exported(self, callable: AsyncCodeCallable | SyncCodeCallable) -> typing.Callable:
        def _exported_sync(*args, **kwargs):
            if self._cached_exports is not None:
                return self._cached_exports
            result = callable(*args, **kwargs)
            self._cached_exports = result
            return result

        async def _exported_async(*args, **kwargs):
            if self._cached_exports is not None:
                return self._cached_exports
            result = await callable(*args, **kwargs)
            self._cached_exports = result
            return result

        return _exported_async if self._parse.is_async else _exported_sync

    def _wrap_test(self, callable: AsyncCodeCallable | SyncCodeCallable) -> typing.Callable:
        if not self._parse.is_async:

            def _test_sync(*args, **kwargs):
                self.current_run.value.test = True
                ret = callable(*args, **kwargs)
                self.session.flush()  # force any write errors to appear immediately
                return ret

            return _test_sync
        else:

            async def _test_async(*args, **kwargs):
                self.current_run.value.test = True
                ret = await callable(*args, **kwargs)  # force any errors to appear immediately
                await self.session.aflush()
                return ret

            return _test_async

    async def __call_async__(self, *args, **kwargs):
        inputs = self._inputs_from_args(args, kwargs)
        try:
            self.session.tracer.run_enter(self, inputs)
            self._prepare_callable()
            result = await self._callable_wrapped(*args, **kwargs)
            self.session.tracer.run_exit(self, result if not self.exported else None)
            return _to_outputs_dict(self, result)
        except BaseException as exception:
            self.session.tracer.run_exception(self, exception)
            raise

    def __call_sync__(self, *args, **kwargs):
        inputs = self._inputs_from_args(args, kwargs)
        try:
            self.session.tracer.run_enter(self, inputs)
            self._prepare_callable()
            result = self._callable_wrapped(*args, **kwargs)
            self.session.tracer.run_exit(self, result if not self.exported else None)
            return _to_outputs_dict(self, result)
        except BaseException as exception:
            self.session.tracer.run_exception(self, exception)
            raise


AsyncCodeCallable = typing.Callable[..., typing.Coroutine]
SyncCodeCallable = typing.Callable[..., Any]

STATIC_BUILTINS: dict[str, Any] = {
    # primitive types
    "string": str,
    "text": str,
    "number": float,
    "boolean": bool,
    # querying
    "Q": Q,
    "Query": Query,
    "QueryOp": QueryOp,
    "Sort": Sort,
    "SortOrder": SortOrder,
    "SortMode": SortMode,
    # remote
    "RemoteObject": RemoteObject,
    "RemoteObjectSatus": RemoteObjectStatus,
    # functional builtins
    "first": first,
    "last": last,
    "chain": itertools.chain,
}
DYNAMIC_BUILTINS: set[str] = {"builtins", "session", "storage", "cache", "random", "self"}
ALLOW_UNTRUSTED_CODE = get_from_env("ALLOW_UNTRUSTED_CODE", False, type_cast=bool)


def _do_exec_get_globals(code: str | types.CodeType, globals: dict[str, Any]) -> dict:
    # remember the globals we started with, do not modify originals
    if not ALLOW_UNTRUSTED_CODE:
        raise RuntimeError("untrusted code execution is disabled")
    globals_local = {**globals}
    exec(code, globals_local)
    return globals_local


def _parse_code(code: str | None) -> "CodeParse":
    """
    Extracts references and other info for Bench from the Python code.
    TODO @Architecture @Cleanup: remove manual code parsing, integrate into LSP/Jedi stuff

    Handles plain references like
    ```py
    import asyncio
    x = 1
    for y in z:
        pass
    ```
    -> 'z' is an external reference to (".", "z").

    Also handles imported references like
    ```py
    from x.symbolx.lib.y import z
    from x.flotothemoon.test.a import b as c
    from .x.local import apple
    from .local import banana
    ```
    -> 'z' is an external reference to ("x.symbolx.lib.y", "z")
    -> 'c' is an external reference to ("x.flotothemoon.test.a", "b")
    -> 'apple' is a local reference to ("<module>.local", "apple").
    -> 'banana' is a local reference to ("<module>.local", "banana").
    """
    if code is None:
        return CodeParse()

    class ReferenceExtractor(ast.NodeVisitor):
        def __init__(self):
            self.references: dict[str, NodePath] = {}
            self.local_variables = set()
            self.imports = set()
            self.is_async = False
            self.codelines = code.splitlines()
            self.x_imports: dict[int, dict[str, NodePath]] = {}

        def visit_Import(self, node):
            for alias in node.names:
                self.imports.add(alias.name.split(".")[0])
            self.generic_visit(node)

        def visit_ImportFrom(self, node):
            # parse 'x' imports
            if node.module is not None:
                # recover module name from source to keep any leading dots
                sourceline = self.codelines[node.lineno - 1]
                module = sourceline[node.col_offset : node.end_col_offset].split(" ")[1]
                if module.startswith("x."):
                    reference = module.split(".", maxsplit=1)[1]
                elif module.startswith(".x."):
                    reference = "." + module.split(".", maxsplit=2)[2]
                elif module.startswith("."):
                    reference = module[1:]
                else:
                    reference = None
                if reference is not None:
                    local_references = {}
                    for alias in node.names:
                        local_references[alias.asname or alias.name] = NodePath(
                            reference, alias.name
                        )
                    self.x_imports[node.lineno - 1] = local_references
                    self.references.update(local_references)
            for alias in node.names:
                self.imports.add(alias.name)
            self.generic_visit(node)

        def visit_FunctionDef(self, node):
            self.local_variables.add(node.name)
            self.generic_visit(node)

        def visit_AsyncFunctionDef(self, node):
            self.local_variables.add(node.name)
            self.is_async = True
            self.generic_visit(node)

        def visit_arg(self, node):
            self.local_variables.add(node.arg)
            self.generic_visit(node)

        def visit_arguments(self, node):
            for arg in node.args:
                self.local_variables.add(arg.arg)
            self.generic_visit(node)

        def visit_Await(self, node):
            self.is_async = True
            self.generic_visit(node)

        def visit_Assign(self, node):
            if isinstance(node.targets[0], ast.Name):
                self.local_variables.add(node.targets[0].id)
            self.generic_visit(node)

        def visit_AnnAssign(self, node):
            if isinstance(node.target, ast.Name):
                self.local_variables.add(node.target.id)
            self.generic_visit(node)

        def visit_Name(self, node):
            if (
                node.id not in self.local_variables
                and node.id not in self.imports
                and node.id not in _PYTHON_BUILTINS
                and node.id not in STATIC_BUILTINS
                and node.id not in DYNAMIC_BUILTINS
            ):
                self.references[node.id] = NodePath(".", node.id)
            self.generic_visit(node)

        def visit_For(self, node):
            if isinstance(node.target, ast.Name):
                self.local_variables.add(node.target.id)
            elif isinstance(node.target, ast.Tuple):
                for target in node.target.elts:
                    if isinstance(target, ast.Name):
                        self.local_variables.add(target.id)
            self.generic_visit(node)

        def visit_AsyncFor(self, node):
            if isinstance(node.target, ast.Name):
                self.local_variables.add(node.target.id)
            elif isinstance(node.target, ast.Tuple):
                for target in node.target.elts:
                    if isinstance(target, ast.Name):
                        self.local_variables.add(target.id)
            self.is_async = True
            self.generic_visit(node)

        def visit_With(self, node):
            for item in node.items:
                if isinstance(item.optional_vars, ast.Name):
                    self.local_variables.add(item.optional_vars.id)
            self.generic_visit(node)

        def visit_AsyncWith(self, node):
            for item in node.items:
                if isinstance(item.optional_vars, ast.Name):
                    self.local_variables.add(item.optional_vars.id)
            self.is_async = True
            self.generic_visit(node)

        def visit_ExceptHandler(self, node):
            if node.name is not None:
                self.local_variables.add(node.name)
            self.generic_visit(node)

        def visit_Lambda(self, node):
            for arg in node.args.args:
                if isinstance(arg, ast.Name):
                    self.local_variables.add(arg.id)
            self.generic_visit(node)

        def visit_ListComp(self, node) -> Any:
            for generator in node.generators:
                if isinstance(generator.target, ast.Name):
                    self.local_variables.add(generator.target.id)
            self.generic_visit(node)

        def visit_SetComp(self, node) -> Any:
            for generator in node.generators:
                if isinstance(generator.target, ast.Name):
                    self.local_variables.add(generator.target.id)
            self.generic_visit(node)

        def visit_DictComp(self, node) -> Any:
            for generator in node.generators:
                if isinstance(generator.target, ast.Name):
                    self.local_variables.add(generator.target.id)
            self.generic_visit(node)

        def visit_GeneratorExp(self, node) -> Any:
            for generator in node.generators:
                if isinstance(generator.target, ast.Name):
                    self.local_variables.add(generator.target.id)
            self.generic_visit(node)

    try:
        tree = ast.parse(code)
        extractor = ReferenceExtractor()
        extractor.visit(tree)
    except (SystemError, SyntaxError) as e:
        logger.debug("code.parse.error", e=e, excinfo=e)
        return CodeParse()

    # remove references to builtins

    return CodeParse(
        references=extractor.references, is_async=extractor.is_async, x_imports=extractor.x_imports
    )


def _to_outputs_dict(code: "HasCode", result: Any) -> TypedDict:
    if isinstance(result, TypedDict):
        return result
    elif result is None:
        return TypedDict(code, {}, is_output=True)
    else:
        return TypedDict(code, result, is_output=True)


_PYTHON_BUILTINS = {
    "abs",
    "aiter",
    "all",
    "any",
    "anext",
    "ascii",
    "bin",
    "bool",
    "breakpoint",
    "bytearray",
    "bytes",
    "callable",
    "chr",
    "classmethod",
    "compile",
    "complex",
    "delattr",
    "dict",
    "dir",
    "divmod",
    "enumerate",
    "eval",
    "exec",
    "filter",
    "float",
    "format",
    "frozenset",
    "getattr",
    "globals",
    "hasattr",
    "hash",
    "help",
    "hex",
    "id",
    "input",
    "int",
    "isinstance",
    "issubclass",
    "iter",
    "len",
    "list",
    "locals",
    "map",
    "max",
    "memoryview",
    "min",
    "next",
    "object",
    "oct",
    "open",
    "ord",
    "pow",
    "print",
    "property",
    "range",
    "repr",
    "reversed",
    "round",
    "set",
    "setattr",
    "slice",
    "sorted",
    "staticmethod",
    "str",
    "sum",
    "super",
    "tuple",
    "type",
    "vars",
    "zip",
    "ValueError",
    "SyntaxError",
    "RuntimeError",
    "NameError",
    "KeyError",
    "IndexError",
    "ImportError",
    "AttributeError",
    "ZeroDivisionError",
    "NotImplementedError",
    "TypeError",
    "StopIteration",
    "GeneratorExit",
    "Exception",
    "Ellipsis",
    "__import__",
}
