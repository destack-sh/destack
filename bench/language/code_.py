from __future__ import annotations

import ast
import hashlib
import itertools
import textwrap
import typing
from dataclasses import dataclass, field
from datetime import datetime
from functools import cached_property
from json import JSONDecodeError
from random import Random
from typing import Any, Optional

import msgpack
import structlog
from more_itertools import first, last

from bench.language.basic import HasText
from bench.language.const import TypeTag
from bench.language.core import IssueType, LookupBy, ModuleVisitor, NodePath, Scope, Statement, node
from bench.language.flow import IsFlowNode
from bench.language.query import Q, Query, QueryOp, Sort, SortMode, SortOrder
from bench.language.remote import RemoteObject, RemoteObjectStatus
from bench.language.tag import HasTags, Tag
from bench.language.type import HasType, check_type, pack_value, unpack_value
from bench.language.utils import Runnable, get_run_cache_subkey
from bench.utils.dt import utcnow_with_tz
from bench.utils.utils import DotDict, IdentifierType, get_from_env, to_pyidentifier

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


@node(tracked=["language", "code"])
class Code(HasType, IsFlowNode, HasTags, HasText, Runnable, Statement):
    language: str = "python"  # will probably merge into environment when we have it
    tag: TypeTag = TypeTag.FUNCTION
    text: Optional[str] = None
    code: Optional[str] = None
    _is_async: Optional[bool] = None
    _parse: Optional[CodeParse] = None
    _transform: Optional[CodeTransformation] = None
    _statement_references: dict[str, Statement] | None = None
    _callable_inner: AsyncCodeCallable | SyncCodeCallable | None = None
    _callable_wrapped: AsyncCodeCallable | SyncCodeCallable | None = None
    _cached_exports: dict[str, Any] | None = None

    def _clear(self) -> None:
        Statement._clear(self)
        HasText._clear(self)
        HasType._clear(self)
        HasTags._clear(self)
        IsFlowNode._clear(self)
        self._parse = None
        self._transform = None
        self._statement_references = None
        self._callable_inner = None
        self._callable_wrapped = None
        self._cached_exports = None

    def _interp(self, scope: Scope) -> None:
        HasText._interp(self, scope)
        HasType._interp(self, scope)
        HasTags._interp(self, scope)
        IsFlowNode._interp(self, scope)

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
        for n in itertools.chain(self.children, self.fields, self.tags, self.triggers):
            visitor.visit_child(n)
        HasText._visit(self, visitor)

    def __call__(self, *args, **kwargs):
        if self._is_async:
            return self.__call_async__(*args, **kwargs)
        else:
            return self.__call_sync__(*args, **kwargs)

    @cached_property
    def cached(self) -> bool:
        # TODO @Cleanup: manage stdlib references centrally :CentralStdlibAccess
        from bench.language.libs import symbolx_lib

        return self.has_tag(symbolx_lib.lookup_or_error(".builtins.cache", node_t=Tag))

    @cached_property
    def exported(self) -> bool:
        from bench.language.libs import symbolx_lib  # :CentralStdlibAccess

        return self.has_tag(symbolx_lib.lookup_or_error(".builtins.export", node_t=Tag))

    @cached_property
    def is_test(self) -> bool:
        from bench.language.libs import symbolx_lib  # :CentralStdlibAccess

        return self.has_tag(symbolx_lib.lookup_or_error(".builtins.test", node_t=Tag))

    @cached_property
    def _code_hash(self) -> str:
        return hashlib.sha256(self.code.encode("utf-8")).hexdigest()

    def _get_cached_output(self, inputs: dict, cached_run: bytes) -> Optional[dict]:
        try:
            run = CachedRun.from_json_bytes(cached_run)
            outputs = unpack_value(run.outputs, self, ignore_outer_map=True, is_output=True)
            check_type(outputs, self, is_output=True)
            self.session.tracer.run_cached(self, inputs, outputs, run.generated_at, run.duration)
            return DotDict(outputs)
        except (ValueError, TypeError, JSONDecodeError) as e:
            logger.exception("code.cache.error", e=e, excinfo=e)
            # ignore, will be overwritten on success
            return None

    def _do_import_sync(self, path: str, name: str) -> tuple[Any, ...]:
        """Import a statement or exported Python object at runtime."""
        reference = NodePath(path, name)
        resolved = self.lookup(reference, by=LookupBy.PyIdent)
        if resolved is None:
            # fall back to code object import
            resolved = self.lookup(reference.path, by=LookupBy.PyIdent)
            if not isinstance(resolved, Code) or not resolved.exported:
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
            if not isinstance(resolved, Code) or not resolved.exported:
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
        context = {**self._statement_references}
        if not self._parse.is_async:
            # replace any async functions with sync versions
            for key, symbol in context.items():
                if isinstance(symbol, Runnable) and symbol._is_async:
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

    def _prep_callable(self) -> None:
        """Creates a callable wrapping this code for execution with the required context."""
        if self._callable_inner is not None:
            return

        locals = self._prep_locals()
        func_body, start_offset, end_offset = self._prep_func_body()
        func_name = self.py_ident or "_anon" + self.id.hex[:6]
        func_params = ", ".join(
            to_pyidentifier(i.name, IdentifierType.VARIABLE) + "=None" for i in self.inputs
        )
        try:
            method_str = f"def {func_name}({func_params}):\n{textwrap.indent(func_body, ' ' * 4)}"
            if self._parse.is_async:
                method_str = f"async {method_str}"
            self._callable_inner = do_execute_arbitrary_code(method_str, locals)[func_name]
            self._callable_wrapped = self._wrap_callable(self._callable_inner)
        except SyntaxError as e:
            # raise error in code when called for proper reporting
            err_str = e.args[1][3] or str(e)
            raise_str = f"raise {e.__class__.__name__}('invalid syntax: ' + {err_str})"
            indented_raise = textwrap.indent(raise_str, " " * 4)
            method_str = f"def {func_name}({func_params}):\n{indented_raise}"
            self._callable_inner = do_execute_arbitrary_code(method_str, locals)[func_name]
            self._callable_wrapped = self._callable_inner
        self._transform = CodeTransformation(
            original_code=self.code,
            transformed_code=method_str,
            start_offset=start_offset + 1,  # for method signature
            end_offset=end_offset,
            method_name=func_name,
        )

    def _wrap_callable(self, callable: AsyncCodeCallable | SyncCodeCallable) -> typing.Callable:
        if self.exported:
            callable = self._wrap_exported(callable)
        elif self.cached:
            callable = self._wrap_cached(callable)
        return callable

    def _wrap_cached(self, callable: AsyncCodeCallable | SyncCodeCallable) -> typing.Callable:
        def _wrapped_sync(*args, **kwargs):
            inputs = self._inputs_from_args(args, kwargs)
            inputs_raw = pack_value(inputs, self, is_output=False)
            cache_subkey = get_run_cache_subkey(inputs_raw=inputs_raw, content_id=self._code_hash)
            cached_run = self.cache.get(cache_subkey)
            cached_output = self._get_cached_output(inputs, cached_run) if cached_run else None
            if cached_output is not None:
                return cached_output
            started_at = utcnow_with_tz()
            result = callable(*args, **kwargs)
            outputs_raw = pack_value(result, self, is_output=True)
            run_bytes = CachedRun.bytes_from_run(inputs_raw, outputs_raw, started_at)
            self.cache.set(cache_subkey, run_bytes)

        async def _wrapped_async(*args, **kwargs):
            # yes this is annoyingly duplicated...
            inputs = self._inputs_from_args(args, kwargs)
            inputs_raw = pack_value(inputs, self, is_output=False)
            cache_subkey = get_run_cache_subkey(inputs_raw=inputs_raw, content_id=self._code_hash)
            cached_run = await self.cache.get(cache_subkey)
            cached_output = self._get_cached_output(inputs, cached_run) if cached_run else None
            if cached_output is not None:
                return cached_output
            started_at = utcnow_with_tz()
            result = await callable(*args, **kwargs)
            outputs_raw = pack_value(result, self, is_output=True)
            run_bytes = CachedRun.bytes_from_run(inputs_raw, outputs_raw, started_at)
            await self.cache.set(cache_subkey, run_bytes)

        return _wrapped_async if self._parse.is_async else _wrapped_sync

    def _wrap_exported(self, callable: AsyncCodeCallable | SyncCodeCallable) -> typing.Callable:
        def _wrapped_sync(*args, **kwargs):
            if self._cached_exports is not None:
                return self._cached_exports
            result = callable(*args, **kwargs)
            self._cached_exports = result
            return result

        async def _wrapped_async(*args, **kwargs):
            if self._cached_exports is not None:
                return self._cached_exports
            result = await callable(*args, **kwargs)
            self._cached_exports = result
            return result

        return _wrapped_async if self._parse.is_async else _wrapped_sync

    async def __call_async__(self, *args, **kwargs):
        inputs = self._inputs_from_args(args, kwargs)
        try:
            self.session.tracer.run_enter(self, inputs)
            self._prep_callable()
            result = await self._callable_wrapped(*args, **kwargs)
            self.session.tracer.run_exit(self, result if not self.exported else None)
            return _to_result_dict(result)
        except BaseException as exception:
            self.session.tracer.run_exception(self, exception)
            raise

    def __call_sync__(self, *args, **kwargs):
        inputs = self._inputs_from_args(args, kwargs)
        try:
            self.session.tracer.run_enter(self, inputs)
            self._prep_callable()
            result = self._callable_wrapped(*args, **kwargs)
            self.session.tracer.run_exit(self, result if not self.exported else None)
            return _to_result_dict(result)
        except BaseException as exception:
            self.session.tracer.run_exception(self, exception)
            raise

    def to_sync(self) -> "Code":
        if not self._is_async:
            return self
        return CodeProxy.to_sync(self)

    def to_async(self) -> "Code":
        if self._is_async:
            return self
        return CodeProxy.to_async(self)


class CodeProxy:  # :SyncProxy
    """
    A simple proxy for Code to enable to_sync/to_async while keeping the original Code object.
    """

    def __init__(self, code: Code, is_async: bool):
        self._code = code
        self._is_async = is_async

    def __call__(self, *args, **kwargs):
        if self._is_async:
            return self.__call_async__(*args, **kwargs)
        else:
            return self.__call_sync__(*args, **kwargs)

    def __getattr__(self, item):
        return getattr(self._code, item)

    @classmethod
    def to_sync(cls, code: Code) -> Code:
        proxy = cls(code, is_async=False)
        proxy.__call_sync__ = code.session.async_to_sync(code.__call_async__)
        return typing.cast(Code, proxy)

    @classmethod
    def to_async(cls, code: Code) -> Code:
        proxy = cls(code, is_async=True)
        proxy.__call_async__ = code.session.sync_to_async(code.__call_sync__)
        return typing.cast(Code, proxy)


@dataclass(slots=True)
class CachedRun:
    """A cached run of a code statement."""

    generated_at: datetime
    duration: float
    inputs: dict[str, Any]
    outputs: dict[str, Any]

    @staticmethod
    def bytes_from_run(inputs: dict, outputs: dict, started_at: datetime):
        now = utcnow_with_tz()
        run = CachedRun(
            generated_at=now,
            duration=(now - started_at).total_seconds(),
            inputs=inputs,
            outputs=outputs,
        )
        return run.to_json_bytes()

    def to_json_bytes(self) -> bytes:
        run_json = {
            "generated_at": self.generated_at.isoformat(),
            "duration": self.duration,
            "inputs": self.inputs,
            "outputs": self.outputs,
        }
        return msgpack.packb(run_json, use_bin_type=True)

    @staticmethod
    def from_json_bytes(json_bytes: bytes) -> "CachedRun":
        run_json = msgpack.unpackb(json_bytes, raw=False)
        return CachedRun(
            generated_at=datetime.fromisoformat(run_json["generated_at"]),
            duration=run_json["duration"],
            inputs=run_json["inputs"],
            outputs=run_json["outputs"],
        )


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


def do_execute_arbitrary_code(code: str, globals: dict[str, Any]) -> dict:
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


def _to_result_dict(result: Any) -> dict:
    if isinstance(result, DotDict):
        return result
    if result is None:
        return DotDict()
    else:
        return DotDict(result)


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
