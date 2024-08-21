import dataclasses
import functools
from contextlib import contextmanager
from dataclasses import dataclass
from typing import Any, Mapping, cast, override

import structlog
from opentelemetry import trace

from bench.language import render
from bench.language.code import Code, CodeType
from bench.language.file import upload
from bench.language.path import get_node, get_node_or_error
from bench.language.render import RenderOptions
from bench.language.run import RunKind
from bench.language.value import ValueObject, coerce_value_object
from bench.runtime.capture import (
    MAX_LOG_LINE_LENGTH,
    MAX_LOGS_PER_CAPTURE,
    LogSink,
    capture_logs,
)
from bench.runtime.compiler import CompiledCode, compile_code
from bench.runtime.core import SyntaxError
from bench.runtime.runner import (
    RunnableNode,
    Runner,
    RunnerCache,
    runner_,
)

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)

# NOTE :Performance :Robustness: run (some?) sync code in a separate thread?


@dataclass(slots=True)
class CodeRunnerCache[T: RunnableNode](RunnerCache[T]):
    compiled: "CompiledCode | None" = None
    exports: Mapping[str, Any] | None = None  # for scripts
    last_expr_value: Any | None = None  # for snippets


@dataclass(slots=True)
class CodeRunnerBase[T: RunnableNode](Runner[CodeRunnerCache[T], T]):
    """Common base for compiling and running code."""

    cache_cls = CodeRunnerCache
    log_sink: LogSink = dataclasses.field(init=False)

    def __post_init__(self):
        self.log_sink = LogSink(
            self.runtime.oracle, max_logs=MAX_LOGS_PER_CAPTURE, max_log_length=MAX_LOG_LINE_LENGTH
        )

    @tracer.start_as_current_span("code.compile")
    async def _compile_code(self, kind: CodeType) -> CompiledCode:
        """Prepares valid compiled code (raises SyntaxError if invalid)."""
        assert self.cache.code, f"no code for {self!r}"
        compiled = self.cache.compiled
        if compiled is None:
            self.cache.compiled = compiled = compile_code(
                self.cache.code.to_string(),
                kind,
                self.runtime.combined_glbls,
            )
        if compiled.syntax_error:
            syntax_e = compiled.syntax_error
            # wrap in our own SyntaxError
            wrapped = SyntaxError(self.cache.code.to_string())
            if syntax_e.lineno is not None and syntax_e.offset is not None:
                wrapped.lineno, wrapped.offset = compiled.transformation.reverse(
                    syntax_e.lineno, syntax_e.offset
                )
            if syntax_e.end_lineno is not None and syntax_e.end_offset is not None:
                wrapped.end_lineno, wrapped.end_offset = compiled.transformation.reverse(
                    syntax_e.end_lineno, syntax_e.end_offset
                )
            wrapped.msg = syntax_e.msg
            raise wrapped from compiled.syntax_error  # re-raise
        return compiled

    @contextmanager
    def _capture_logs(self):
        """Capture logs into this run."""
        with capture_logs(self.log_sink):
            try:
                yield self.log_sink
            finally:
                self.logs.extend(self.log_sink.logs)

    @tracer.start_as_current_span("code.prepare_context")
    def _prepare_glbls(self) -> dict[str, Any]:
        """Prepares the context for running the code."""
        assert self.cache.compiled, f"no compiled code for {self!r}"

        # assemble globals
        assert self.node is not None, f"no node scope for {self!r}"
        _get_node = functools.partial(get_node, self.node)
        _render = functools.partial(render, options=RenderOptions(scope=self.node))
        _upload = functools.partial(upload, parent=self.node)
        glbls = {  # :CodeGlobals
            # static
            **self.runtime.static_glbls,
            # dynamic
            "self": self.node,
            "get_node": _get_node,
            "render": _render,
            "upload": _upload,
            "log": self.log_sink,
            "trace": self.log_sink.trace,
            "debug": self.log_sink.debug,
            "info": self.log_sink.info,
            "warn": self.log_sink.warn,
            "error": self.log_sink.error,
            "critical": self.log_sink.critical,
            "print": self.log_sink.print,
        }

        # references
        # NOTE :Incomplete: handle references to exported definitions (not just node references)
        resolved_references = {}
        for reference_name in self.cache.compiled.references:
            reference = get_node_or_error(self.node, f"^{reference_name}")
            resolved_references[reference_name] = reference
        glbls.update(resolved_references)  # may shadow existing glbls

        return glbls

    @tracer.start_as_current_span("code.coerce_outputs")
    def _coerce_outputs(self, outputs_raw: Any) -> ValueObject:
        """Coerves raw outputs into the output type for this run."""
        assert self.output_type, f"no output type for {self!r}"
        outputs = coerce_value_object(self.output_type, outputs_raw)
        return outputs


@runner_(RunKind.CODE, CodeType.SNIPPET)
class CodeSnippetRunner(CodeRunnerBase):
    """Run a code snippet and update the value of the state's last expression."""

    @override
    async def run_once(self) -> None:
        raise NotImplementedError


@runner_(RunKind.CODE, CodeType.SCRIPT)
class CodeScriptRunner(CodeRunnerBase):
    """Run a code script and update the state's exported definitions."""

    @override
    async def run_once(self) -> None:
        # compile
        compiled = await self._compile_code(CodeType.SCRIPT)
        if not compiled.body_co:
            return  # empty

        # context
        glbls = self._prepare_glbls()

        # run
        logger.trace("code.run", runner=self, code=cast(Code, self.code).to_string())
        with self._capture_logs():
            if compiled.is_coroutine:
                coro = eval(compiled.body_co, glbls)
                await coro
            else:
                exec(compiled.body_co, glbls)
        exports = {defn: glbls[defn] for defn in compiled.definitions}
        self.cache.exports = exports


@runner_(RunKind.CODE, CodeType.FUNCTION)
class CodeFunctionRunner(CodeRunnerBase):
    """Run a code function and update the run's outputs."""

    @override
    async def run_once(self) -> None:
        # compile
        compiled = await self._compile_code(CodeType.FUNCTION)
        if not compiled.body_co:
            return  # empty
        assert compiled.function_name, f"no function name for {self!r}"

        # context
        glbls = self._prepare_glbls()
        if self.inputs is not None:
            for field in self.inputs.fields:
                value = self.inputs._do_get(field)
                glbls[field.name] = value
                if field.py_name:
                    glbls[field.py_name] = value

        # run
        logger.trace("code.run", runner=self, code=cast(Code, self.code).to_string())
        exec(compiled.body_co, glbls)  # shouldn't error
        func = glbls[compiled.function_name]
        with self._capture_logs():
            if compiled.is_coroutine:
                outputs_raw = await func()
            else:
                outputs_raw = func()
        self.outputs = self._coerce_outputs(outputs_raw)
