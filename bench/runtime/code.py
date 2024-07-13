import functools
from contextlib import contextmanager
from typing import Any, override

import structlog
from opentelemetry import trace

from bench.language import render
from bench.language.code import CodeType
from bench.language.path import get_node, get_node_or_error
from bench.language.render import RenderOptions
from bench.language.run import RunKind
from bench.language.session import Session
from bench.language.value import ValueObject, coerce_value_object
from bench.runtime.capture import (
    MAX_LOG_LINE_LENGTH,
    MAX_LOGS_PER_CAPTURE,
    LogSink,
    capture_logs,
)
from bench.runtime.compiler import CompiledCode, compile_code
from bench.runtime.core import CodeSyntaxError
from bench.runtime.runner import RunHandle, Runner, RuntimeRunner, runner

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)

# NOTE :Performance :Robustness: run (some?) sync code in a separate thread?


class CodeRunnerBase(Runner):
    """Common base for compiling and running code."""

    def __init__(self, runner: RuntimeRunner, session: Session, handle: RunHandle):
        super().__init__(runner, session, handle)
        self.log_sink = LogSink(
            self.runtime.oracle, max_logs=MAX_LOGS_PER_CAPTURE, max_log_length=MAX_LOG_LINE_LENGTH
        )

    @tracer.start_as_current_span("code.compile")
    async def _compile_code(self, kind: CodeType) -> CompiledCode:
        """Prepares valid compiled code (raises SyntaxError if invalid)."""
        assert self.state.code, f"no code for {self!r}"
        compiled = self.state.compiled
        if compiled is None:
            self.state.compiled = compiled = compile_code(
                str(self.state.code.id),
                self.state.code.to_string(),
                kind,
                self.runtime.combined_glbls,
            )
        if compiled.syntax_error:
            raise CodeSyntaxError(repr(self)) from compiled.syntax_error  # re-raise
        return compiled

    @contextmanager
    def _capture_logs(self):
        """Capture logs into this run."""
        with capture_logs(self.log_sink):
            try:
                yield self.log_sink
            finally:
                self.handle.logs.extend(self.log_sink.logs)

    @tracer.start_as_current_span("code.prepare_context")
    def _prepare_glbls(self) -> dict[str, Any]:
        """Prepares the context for running the code."""
        assert self.state.compiled, f"no compiled code for {self!r}"

        # references
        # NOTE :Incomplete: handle references to exported definitions (not just node references)
        resolved_references = {}
        for reference_name in self.state.compiled.references:
            reference = get_node_or_error(self.node, f"^{reference_name}")
            resolved_references[reference_name] = reference

        # assemble globals
        _get_node = functools.partial(get_node, self.node)
        _render = functools.partial(render, options=RenderOptions(scope=self.node))
        glbls = {  # :CodeGlobals
            # static
            **self.runtime.static_glbls,
            # dynamic
            "self": self.node,
            "get_node": _get_node,
            "render": _render,
            "log": self.log_sink,
            "trace": self.log_sink.trace,
            "debug": self.log_sink.debug,
            "info": self.log_sink.info,
            "warn": self.log_sink.warn,
            "error": self.log_sink.error,
            "critical": self.log_sink.critical,
            "print": self.log_sink.print,
        }
        # references (nodes/exports)
        glbls.update(resolved_references)
        return glbls

    def _coerce_outputs(self, outputs_raw: Any) -> ValueObject:
        """Coerves raw outputs into the output type for this run."""
        assert self.handle.output_type, f"no output type for {self!r}"
        outputs = coerce_value_object(self.handle.output_type, outputs_raw)
        return outputs


@runner(RunKind.CODE, CodeType.SNIPPET)
class CodeSnippetRunner(CodeRunnerBase):
    """Run a code snippet and update the value of the state's last expression."""

    @override
    async def run(self) -> None:
        raise NotImplementedError


@runner(RunKind.CODE, CodeType.SCRIPT)
class CodeScriptRunner(CodeRunnerBase):
    """Run a code script and update the state's exported definitions."""

    @override
    async def run(self) -> None:
        # compile
        compiled = await self._compile_code(CodeType.SCRIPT)
        if not compiled.body_co:
            return  # empty

        # context
        glbls = self._prepare_glbls()

        # run
        with self._capture_logs():
            if compiled.is_coroutine:
                coro = eval(compiled.body_co, glbls)
                await coro
            else:
                exec(compiled.body_co, glbls)
        exports = {defn: glbls[defn] for defn in compiled.definitions}
        self.state.exports = exports


@runner(RunKind.CODE, CodeType.FUNCTION)
class CodeFunctionRunner(CodeRunnerBase):
    """Run a code function and update the run's outputs."""

    @override
    async def run(self) -> None:
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
                if field.py_ident:
                    glbls[field.py_ident] = value

        # run
        exec(compiled.body_co, glbls)  # shouldn't error
        func = glbls[compiled.function_name]
        with self._capture_logs():
            if compiled.is_coroutine:
                outputs_raw = await func()
            else:
                outputs_raw = func()
        self.handle.outputs = self._coerce_outputs(outputs_raw)
