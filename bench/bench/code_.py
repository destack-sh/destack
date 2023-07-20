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
from typing import Any, Callable, Optional

import msgpack
import pytz
import structlog
from more_itertools import first, last

from bench.bench.const import StatementType, TypeTag
from bench.bench.core import IssueType, LookupBy, Scope, Session, Statement, StatementPath, node
from bench.bench.query import Q, Query, QueryOp, Sort, SortMode, SortOrder
from bench.bench.remote import RemoteObject, RemoteObjectStatus
from bench.bench.tag import HasTags, Tag
from bench.bench.type import HasType, check_type, instantiate_py_value, strip_py_value
from bench.bench.utils import Runnable, get_run_cache_key
from bench.utils.cache import redis, redis_sync
from bench.utils.utils import DotDict, IdentifierType, get_from_env, to_pyidentifier

logger = structlog.get_logger(__name__)


@node
class CodeTransformation:
    original_code: str
    transformed_code: str
    method_name: str
    start_offset: int


@node
class CodeParse:
    references: dict[str, "StatementPath"] = field(default_factory=dict)
    is_async: bool = False
    fake_line_numbers: list[int] = field(default_factory=list)


@node(tracked=["language", "code"])
class Code(HasType, HasTags, Runnable, Statement):
    language: str = "python"  # will probably merge into environment when we have it
    tag: TypeTag = TypeTag.FUNCTION
    type: StatementType = StatementType.CODE
    description: Optional[str] = None
    code: Optional[str] = None
    _is_async: Optional[bool] = None
    _parse: Optional[CodeParse] = None
    _references: dict[str, Statement] | None = None
    _transform: Optional[CodeTransformation] = None
    _callable: AsyncCodeCallable | SyncCodeCallable | None = None

    def _clear(self) -> None:
        Statement._clear(self)
        HasType._clear(self)
        HasTags._clear(self)
        self._parse = None
        self._references = None
        self._transform = None
        self._callable = None

    def _interp(self, scope: Scope) -> None:
        HasType._interp(self, scope)
        HasTags._interp(self, scope)

        # parse and resolve code references
        input_idents = {input.py_ident for input in self.inputs}
        self._parse = _parse_code(self.code)
        self._is_async = self._parse.is_async
        self._references = {}
        for key, reference in self._parse.references.items():
            if key in input_idents:
                continue  # input arguments are not context
            resolved = scope.lookup(reference, by=LookupBy.PyIdent)
            if resolved is not None:
                self._references[key] = resolved
            else:
                self._on_issue(type=IssueType.MISSING_REFERENCE, subject=self, path=key)

    def _prep_callable(self) -> None:
        if self._callable is None:
            self._transform, self._callable = instantiate_callable(self, self.session)

    def __call__(self, *args, **kwargs):
        if self._is_async:
            return self.__call_async__(*args, **kwargs)
        else:
            return self.__call_sync__(*args, **kwargs)

    @cached_property
    def cached(self) -> bool:
        from bench.bench.libs import symbolx_lib

        return self.has_tag(symbolx_lib.lookup_or_error(".builtins.cache", statement_t=Tag))

    @cached_property
    def _code_hash(self) -> str:
        return hashlib.sha256(self.code.encode("utf-8")).hexdigest()

    def _get_cached_output(self, inputs: dict, cached_run: bytes) -> Optional[dict]:
        try:
            run = CachedExecution.from_json_bytes(cached_run)
            outputs = instantiate_py_value(run.outputs, self, ignore_outer_map=True, is_output=True)
            check_type(outputs, self, is_output=True)
            self.session.tracer.run_cached(self, inputs, outputs, run.generated_at, run.duration)
            return DotDict(outputs)
        except (ValueError, TypeError, JSONDecodeError) as e:
            logger.exception("code.cache.error", e=e, excinfo=e)
            # ignore, will be overwritten on success
            return None

    def _to_result_dict(self, result: Any) -> dict:
        if isinstance(result, DotDict):
            return result
        if result is None:
            return DotDict()
        else:
            return DotDict(result)

    async def __call_async__(self, *args, **kwargs):
        self._prep_callable()
        inputs = self._inputs_from_args(args, kwargs)
        if self.cached:
            inputs_raw = strip_py_value(inputs, self, is_output=False)
            cache_key = get_run_cache_key(self.id, inputs_raw, content_id=self._code_hash)
            cached_run = await redis.get(cache_key)
            cached_output = self._get_cached_output(inputs, cached_run) if cached_run else None
            if cached_output is not None:
                return cached_output
            started_at = datetime.utcnow().replace(tzinfo=pytz.utc)
        try:
            self.session.tracer.run_enter(self, inputs)
            result = await self._callable(*args, **kwargs)
            self.session.tracer.run_exit(self, result)
            if self.cached:
                outputs_raw = strip_py_value(result, self, is_output=True)
                run_bytes = CachedExecution.bytes_from_run(inputs_raw, outputs_raw, started_at)
                await redis.set(cache_key, run_bytes)
            return self._to_result_dict(result)
        except Exception as exception:
            self.session.tracer.run_exception(self, exception)
            raise

    def __call_sync__(self, *args, **kwargs):
        self._prep_callable()
        # the duplication here is obvious and a bit unfortunate,
        # but I can't think of a way to avoid it without complex and unnecessary intermediates
        inputs = self._inputs_from_args(args, kwargs)
        if self.cached:
            inputs_raw = strip_py_value(inputs, self, is_output=False)
            cache_key = get_run_cache_key(self.id, inputs_raw, content_id=self._code_hash)
            cached_run = redis_sync.get(cache_key)
            cached_output = self._get_cached_output(inputs, cached_run) if cached_run else None
            if cached_output is not None:
                return cached_output
            started_at = datetime.utcnow().replace(tzinfo=pytz.utc)
        try:
            self.session.tracer.run_enter(self, inputs)
            result = self._callable(*args, **kwargs)
            self.session.tracer.run_exit(self, result)
            if self.cached:
                outputs_raw = strip_py_value(result, self, is_output=True)
                run_bytes = CachedExecution.bytes_from_run(inputs_raw, outputs_raw, started_at)
                redis_sync.set(cache_key, run_bytes)
            return self._to_result_dict(result)
        except Exception as exception:
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


@dataclass(slots=True)
class CachedExecution:
    """A cached run of a code statement."""

    generated_at: datetime
    duration: float
    inputs: dict[str, Any]
    outputs: dict[str, Any]

    @staticmethod
    def bytes_from_run(inputs: dict, outputs: dict, started_at: datetime):
        now = datetime.utcnow().replace(tzinfo=pytz.utc)
        run = CachedExecution(
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
    def from_json_bytes(json_bytes: bytes) -> "CachedExecution":
        run_json = msgpack.unpackb(json_bytes, raw=False)
        return CachedExecution(
            generated_at=datetime.fromisoformat(run_json["generated_at"]),
            duration=run_json["duration"],
            inputs=run_json["inputs"],
            outputs=run_json["outputs"],
        )


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


AsyncCodeCallable = typing.Callable[..., typing.Coroutine]
SyncCodeCallable = typing.Callable[..., Any]


def instantiate_callable(
    code: Code, session: Session
) -> tuple[CodeTransformation, Callable[..., Any]]:
    """Instantiates code into a Python callable in the context of the session."""

    context = {**code._references}
    if not code._parse.is_async:
        # replace any async functions with sync versions
        for key, symbol in context.items():
            from bench.bench import Task

            if isinstance(symbol, (Code, Task)) and symbol._is_async:
                context[key] = symbol.to_sync()

    dynamic_context = {
        "session": session,
        "context": {symbol.name: symbol for symbol in context.values()},  # by name
        **context,  # inlined
        "random": Random(code.id.hex.encode()),
        "self": code,
    }

    if code.language == "python":
        python_code = code.code or "pass"
        locals = {**STATIC_BUILTINS, **dynamic_context}
    else:
        raise ValueError(f"unexpected code language: {code}")

    # stub fake lines
    python_code_lines = python_code.splitlines()
    for i in code._parse.fake_line_numbers:
        python_code_lines[i] = "pass # " + python_code_lines[i]
    python_code = "\n".join(python_code_lines)

    # create python function from python code
    input_keys = [i.name for i in code.inputs]
    func_name = code.py_ident or "_anon" + code.id.hex[:6]
    async_str = "async " if code._parse.is_async else ""
    func_params = ", ".join(
        to_pyidentifier(key, IdentifierType.VARIABLE) + "=None" for key in input_keys
    )
    indented_code = textwrap.indent(python_code, " " * 4)
    try:
        method_str = f"{async_str}def {func_name}({func_params}):\n{indented_code}"
        callable = do_execute_arbitrary_code(method_str, locals)[func_name]
    except SyntaxError as e:
        # raise error in code when called for proper reporting
        err_str = e.args[1][3] or str(e)
        raise_str = f"raise {e.__class__.__name__}('invalid syntax: ' + {err_str})"
        indented_raise = textwrap.indent(raise_str, " " * 4)
        method_str = f"{async_str}def {func_name}({func_params}):\n{indented_raise}"
        callable = do_execute_arbitrary_code(method_str, locals)[func_name]

    transform = CodeTransformation(
        original_code=code.code,
        transformed_code=method_str,
        start_offset=1,  # for method signature
        method_name=func_name,
    )
    return transform, callable


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
    "Object": RemoteObject,
    "ObjectSatus": RemoteObjectStatus,
    # functional builtins
    "first": first,
    "last": last,
    "chain": itertools.chain,
}
DYNAMIC_BUILTINS: set[str] = {"builtins", "session", "random", "self"}
ALLOW_UNTRUSTED_CODE = get_from_env("ALLOW_UNTRUSTED_CODE", False, type_cast=bool)


def do_execute_arbitrary_code(code: str, globals: dict[str, Any]) -> dict:
    # remember the globals we started with, do not modify originals
    if not ALLOW_UNTRUSTED_CODE:
        raise RuntimeError("untrusted code execution is disabled")
    globals_local = {**globals}
    globals_local_keys_initial = {*globals_local.keys()}
    exec(code, globals_local)
    new_globals = {
        k: v
        for k, v in globals_local.items()
        if k not in globals_local_keys_initial and k not in ("__builtins__", "__annotations__")
    }
    return new_globals


async def run(
    code: Code,
    arguments: dict[str, Any] | None,
    session: "Session",
    is_trusted: bool = False,
) -> Any:
    from bench.bench.session import RunError, RunErrorKind

    if not is_trusted and not ALLOW_UNTRUSTED_CODE:
        raise RunError(kind=RunErrorKind.UNTRUSTED, type="untrusted", runnable=code)
    # transform keys to valid python identifiers
    arguments = {
        to_pyidentifier(k, IdentifierType.VARIABLE): v for k, v in (arguments or {}).items()
    }
    try:
        # set current session
        await session.aopen()
        if not code._is_async:
            code = code.to_async()
        ret = await code(**arguments)
        await session.aclose()
        return ret
    except Exception as e:
        raise RunError(
            kind=RunErrorKind.RUNTIME, type=type(e).__name__, message=str(e), runnable=code
        ) from e


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
    ```
    -> 'z' is an external reference to ("x.symbolx.lib.y", "z")
    -> 'c' is an external reference to ("x.flotothemoon.test.a", "b")
    -> 'apple' is an external reference to ("<module>.local", "apple").
    """
    if code is None:
        return CodeParse()

    class ReferenceExtractor(ast.NodeVisitor):
        def __init__(self):
            self.references: dict[str, StatementPath] = {}
            self.local_variables = set()
            self.imports = set()
            self.is_async = False
            self.codelines = code.splitlines()
            self.x_import_lines: list[int] = []

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
                    self.x_import_lines.append(node.lineno - 1)
                elif module.startswith(".x."):
                    reference = "." + module.split(".", maxsplit=2)[2]
                    self.x_import_lines.append(node.lineno - 1)
                else:
                    reference = None
                if reference is not None:
                    for alias in node.names:
                        self.references[alias.asname or alias.name] = StatementPath(
                            reference, alias.name
                        )
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
                self.references[node.id] = StatementPath(".", node.id)
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
    except SyntaxError:
        return CodeParse()

    # remove references to builtins

    return CodeParse(
        references=extractor.references,
        is_async=extractor.is_async,
        fake_line_numbers=extractor.x_import_lines,
    )


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
