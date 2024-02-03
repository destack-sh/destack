from __future__ import annotations

import ast
import hashlib
import itertools
import textwrap
import types
import typing
from dataclasses import dataclass
from json import JSONDecodeError
from random import Random
from typing import Any, Optional

import more_itertools
import structlog
from more_itertools import first, last

from bench.language.builtin import symbolx_package
from bench.language.field import TypedDict
from bench.language.node import Node, ScopeNode, node_component, p_runtime
from bench.utils.dt import utcnow_with_tz
from bench.utils.utils import get_from_env

if typing.TYPE_CHECKING:
    from bench.language import Block
    from bench.language.notice import NoticeHandler

logger = structlog.get_logger(__name__)


@dataclass
class CodeTransformation:
    original_code: str
    transformed_code: str
    method_name: str
    start_offset: int
    end_offset: int


@dataclass
class CodeParse:
    is_async: bool = False


def _install_package(name: str, timeout: int = 300, try_import: str = None) -> None:
    """Helper to install a package in the current worker. Not in lib because it feels wrong."""
    if try_import:
        try:
            __import__(try_import)
            return
        except ImportError:
            pass

    import subprocess
    import sys

    subprocess.run(
        [sys.executable, "-m", "pip", "install", name],
        check=True,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        timeout=timeout,
    )


@node_component
class HasCode(Node):
    _is_async: Optional[bool] = p_runtime(default=None)
    _transform: Optional[CodeTransformation] = p_runtime(default=None)
    _block_references: dict[str, "Block"] | None = p_runtime(default=None)
    _callable_wrapped: AsyncCodeCallable | SyncCodeCallable | None = p_runtime(default=None)
    _cached_exports: dict[str, Any] | None = p_runtime(default=None)

    def _clear_inner(self, scope: Optional[ScopeNode] = None) -> None:
        self._transform = None
        self._block_references = None
        self._callable_wrapped = None
        self._cached_exports = None

    def _interp_inner(self, scope: ScopeNode, on_notice: "NoticeHandler") -> None:
        self._is_async = "await " in self.code
        self._block_references = {}
        self._code_export_references = {}
        for key, reference in self._parse.references.items():
            resolved = scope.lookup(reference)
            if resolved is not None:
                self._block_references[key] = resolved

    def _untrack_inner(self) -> None:
        self._callable_wrapped = None  # locals are bound to session

    @property
    def _cache(self) -> bool:
        return symbolx_package.resolve(".builtins.cache") in self.tags

    @property
    def _export(self) -> bool:
        return symbolx_package.resolve(".builtins.export") in self.tags

    @property
    def _test(self) -> bool:
        return symbolx_package.resolve(".builtins.test") in self.tags

    @property
    def _code_hash(self) -> str:
        return hashlib.sha256(self.code.encode("utf-8")).hexdigest()

    def _prep_locals(self) -> dict[str, Any]:
        """Gets the context ('locals') required for the code to run."""

        dynamic_context = {
            "self": self,
            "package": self.package,
            "session": self.session,
            "cache": self.session._cache,
            "random": Random(self.id.hex.encode()),
            "install": _install_package,
            **self._block_references,
            **{s.py_ident: s for s in symbolx_package.files.builtins.blocks},
        }

        import bench.language

        imports = {
            # everything from bench.language
            **{k: v for k, v in bench.language.__dict__.items() if not k.startswith("_")},
        }

        if self._test:  # very crude initial test support
            import pytest

            imports["pytest"] = pytest

        locals = {**imports, **STATIC_BUILTINS, **dynamic_context}
        return locals

    def _prep_func_body(self) -> tuple[str, int, int]:
        """Prepares the function body of this code with all modifications."""
        func_body_lines = (self.code.strip() if self.code else "").splitlines() + ["pass"]
        return "\n".join(func_body_lines), 0, 0

    def _prepare_callable(self) -> None:
        """Creates a callable wrapping this code for execution with the required context."""
        if self._callable_wrapped is not None:
            return

        locals = self._prep_locals()
        func_body, start_offset, end_offset = self._prep_func_body()
        func_name = f"{self.py_ident or '_anon'}_{self.id.hex[:6]}"
        func_params = ", ".join(f.py_ident + "=None" for f in self.fields if not f.is_output)
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
        if self._test:
            from _pytest.assertion.rewrite import rewrite_asserts

            # rewrite asserts for better debugging
            tree = ast.parse(code_str)
            rewrite_asserts(tree, code_str.encode())
            co = compile(tree, "<string>", "exec", dont_inherit=True)
            callable = _do_exec_get_globals(co, locals)[func_name]
        else:
            callable = _do_exec_get_globals(code_str, locals)[func_name]

        # wrap callable as needed (wrapping and proxying will be generalized later - tags?)
        if self._proxied:
            callable = self._wrap_proxied(callable)
        elif self._export:
            callable = self._wrap_exported(callable)
        elif self._cache:
            callable = self._wrap_cached(callable)
        if self._test:
            callable = self._wrap_test(callable)
        return callable

    def _wrap_proxied(self, callable: AsyncCodeCallable | SyncCodeCallable) -> typing.Callable:
        """
        Wraps a callable to be executed remotely in the runtime server.
        Used if the code needs some built-in special credentials. Obviously not great.
        """

        async def _proxied_async(*args, **kwargs):
            from bench.language.value import pack_value, unpack_value

            inputs = self._inputs_from_args(args, kwargs)
            inputs_raw = pack_value(inputs, self, is_output=False, ignore_outer=True)
            try:
                logger.debug("code.proxy", code=self, inputs=inputs_raw)
                outputs = await self.session.host.run_proxy_block(self, inputs_raw)
                outputs = unpack_value(outputs, self, is_output=True)
                return TypedDict(outputs, self, is_output=True)
            except BaseException as e:
                logger.exception("code.proxy.error", code=self, e=e, excinfo=e)
                raise

        return _proxied_async  # is auto wrapped for sync because of _is_async

    def _wrap_cached(self, callable: AsyncCodeCallable | SyncCodeCallable) -> typing.Callable:
        """Wraps a callable with caching for #cache tag."""
        from bench.language.value import check_type, pack_value, unpack_value
        from bench.language.run import CachedRun, get_run_cache_subkey

        def _get_cached_output(inputs: dict, cached_run: bytes) -> Optional[dict]:
            try:
                from .run import CachedRun

                run = CachedRun.from_json_bytes(cached_run)
                outputs = unpack_value(run.outputs_packed, self, ignore_outer=True, is_output=True)
                check_type(outputs, self, is_output=True)
                self.session._run_cached(
                    block=self,
                    inputs=inputs,
                    outputs=outputs,
                    generated_at=run.generated_at,
                    generated_in=run.generated_in,
                    duration=run.duration,
                )
                return TypedDict(outputs, self)
            except (ValueError, KeyError, TypeError, JSONDecodeError) as e:
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
            run_id = self.session.current_run.id
            run_bytes = CachedRun.bytes_from_run(run_id, inputs_raw, outputs_raw, started_at)
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
            run_id = self.session.current_run.id
            run_bytes = CachedRun.bytes_from_run(run_id, inputs_raw, outputs_raw, started_at)
            await self.cache.set(cache_subkey, run_bytes)
            return result

        return _cached_async if self._parse.is_async else _cached_sync

    def _wrap_exported(self, callable: AsyncCodeCallable | SyncCodeCallable) -> typing.Callable:
        """'Exports' definitions of a callable for #export tag."""

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
        """Marks a callable as a test for #test tag."""
        if not self._parse.is_async:

            def _test_sync(*args, **kwargs):
                self.current_run.value.test = True
                ret = callable(*args, **kwargs)
                if self.session.has_regular_edits and not self.session._failed_commit:
                    self.session.commit()  # force commit errors to appear immediately
                return ret

            return _test_sync
        else:

            async def _test_async(*args, **kwargs):
                self.current_run.value.test = True
                ret = await callable(*args, **kwargs)  # force commit errors to appear immediately
                if self.session.has_regular_edits and not self.session._failed_commit:
                    await self.session.commit()
                return ret

            return _test_async

    async def _call_inner_async(self, *args, **kwargs):
        inputs = self._inputs_from_args(args, kwargs)
        session = self.session
        session._run_enter(self, is_async=True, inputs=inputs)
        try:
            self._prepare_callable()
            result = await self._callable_wrapped(*args, **kwargs)
        except BaseException as exception:
            session._run_exception(self, exception)
            raise
        session._run_exit(self, result if not self._export else None)
        return _to_outputs_dict(self, result)

    def _call_inner_sync(self, *args, **kwargs):
        inputs = self._inputs_from_args(args, kwargs)
        session = self.session
        session._run_enter(self, is_async=False, inputs=inputs)
        try:
            self._prepare_callable()
            result = self._callable_wrapped(*args, **kwargs)
        except BaseException as exception:
            session._run_exception(self, exception)
            raise
        session._run_exit(self, result if not self._export else None)
        return _to_outputs_dict(self, result)


AsyncCodeCallable = typing.Callable[..., typing.Coroutine]
SyncCodeCallable = typing.Callable[..., Any]

STATIC_BUILTINS: dict[str, Any] = {
    # functional builtins
    "first": first,
    "last": last,
    "batched": more_itertools.batched,
    "chain": itertools.chain,
}
DYNAMIC_BUILTINS: set[str] = {"builtins", "session", "cache", "random", "self"}
ALLOW_UNTRUSTED_CODE = get_from_env("ALLOW_UNTRUSTED_CODE", False, type_cast=bool)


def _do_exec_get_globals(code: str | types.CodeType, globals: dict[str, Any]) -> dict:
    # remember the globals we started with, do not modify originals
    if not ALLOW_UNTRUSTED_CODE:
        raise RuntimeError("untrusted code execution is disabled")
    globals_local = {**globals}
    exec(code, globals_local)
    return globals_local


def _to_outputs_dict(code: "HasCode", result: Any) -> TypedDict:
    if isinstance(result, TypedDict):
        return result
    elif result is None:
        return TypedDict({}, code, is_output=True)
    else:
        return TypedDict(result, code, is_output=True)


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
