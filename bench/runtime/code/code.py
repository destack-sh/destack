import functools
from abc import ABC
from contextlib import contextmanager
from typing import Any, ClassVar, cast, override

import structlog
from opentelemetry import trace

from bench.language import (
    IS_IN_USER_CODE,
    Aliasing,
    Code,
    CodeType,
    CustomObject,
    Field,
    IsRuntime,
    Node,
    RenderOptions,
    RunnableNode,
    RunOptions,
    RunType,
    TypeBase,
    coerce_custom_object_scalar,
    get_node,
    get_node_or_error,
    get_path,
    render,
    upload_file,
)
from bench.runtime.core import CodeInvalidError, RunIn, Runner, Runtime

from .capture import MAX_LOG_LINE_LENGTH, MAX_LOGS_PER_RUN, LogSink, capture_logs
from .compiler import CompiledCode, compile_code

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)

# NOTE :Performance :Robustness: run (some?) sync code in a separate thread?


class CodeRunner(Runner, ABC):
    """Common base for compiling and running code."""

    __slots__ = ("aliasing", "code", "combined_glbls", "compiled", "log_sink")

    runner_type: ClassVar[RunType] = RunType.ACTION

    def __init__(
        self,
        *,
        runtime: Runtime,
        node: RunnableNode,
        code: Code,
        aliasing: Aliasing,
        options: RunOptions,
        context: IsRuntime,
        run: RunIn,
        parent: Runner | None = None,
        inputs: CustomObject | None = None,
        resources: CustomObject | None = None,
        outputs: TypeBase | CustomObject | None = None,
    ) -> None:
        super().__init__(
            runtime=runtime,
            node=node,
            options=options,
            context=context,
            parent=parent,
            inputs=inputs,
            resources=resources,
            outputs=outputs,
            run=run,
        )
        self.code = code
        self.aliasing = aliasing
        self.combined_glbls: dict[str, Any] = {
            **self.runtime.combined_glbls,
            **(self.aliasing._node_by_alias if self.aliasing else {}),
        }
        self.compiled: CompiledCode | None = None
        self.log_sink = LogSink(
            runtime=self.runtime,
            max_logs=MAX_LOGS_PER_RUN,
            max_log_length=MAX_LOG_LINE_LENGTH,
        )

    @tracer.start_as_current_span("code.compile")
    async def _compile_code(self, kind: CodeType) -> CompiledCode:
        """Prepares valid compiled code (raises SyntaxError if invalid)."""
        compiled = self.compiled
        if compiled is None:
            self.compiled = compiled = compile_code(
                self.code.to_string(), kind, self.combined_glbls
            )
        if compiled.syntax_error:
            syntax_e = compiled.syntax_error
            # wrap in our own SyntaxError
            wrapped = CodeInvalidError(self.code.to_string())
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
            yield self.log_sink

    @tracer.start_as_current_span("code.prepare_context")
    def _prepare_glbls(self) -> dict[str, Any]:
        """Prepares the context for running the code."""
        assert self.compiled, f"no compiled code for {self!r}"

        # assemble globals
        assert self.node is not None, f"no node scope for {self!r}"
        _get_node = functools.partial(get_node, self.node, self.context)
        _get_node_or_error = functools.partial(get_node_or_error, self.node, self.context)
        _get_path = functools.partial(get_path, self.node)
        _render = functools.partial(
            render, options=RenderOptions(scope=self.node, aliasing=self.aliasing)
        )
        _upload = functools.partial(upload_file)
        glbls = {  # :CodeGlobals
            # static
            **self.combined_glbls,
            # dynamic
            "self": self.node,
            "session": self.runtime.session,
            "runtime": self.runtime,
            "bench": self.runtime.bench,
            "run": self.closest_tracked_run,
            "runner": self,
            "aliasing": self.aliasing,
            "resources": self.resources or {},
            "inputs": self.inputs or {},
            "outputs": self.outputs or {},
            "get_node": _get_node,
            "get_node_or_error": _get_node_or_error,
            "get_path": _get_path,
            "render": _render,
            "upload": _upload,
            "log": self.log_sink,
            "trace": self.log_sink.trace,
            "debug": self.log_sink.debug,
            "info": self.log_sink.info,
            "warn": self.log_sink.warn,
            "error": self.log_sink.error,
            "panic": self.log_sink.panic,
            "print": self.log_sink.print,
        }

        # references
        # NOTE :Incomplete: handle references to exported definitions (not just node references)
        resolved_references: dict[str, Node] = {}
        for reference_name in self.compiled.references:
            reference = get_node_or_error(self.node, self.context, f"^{reference_name}")
            if isinstance(reference, Field):
                # replace Field reference with the underlying type if it's the same name
                # (this is useful for Choice/)
                base_type = reference.base_type
                if base_type is not None and base_type.code_name == reference_name:
                    reference = base_type
            resolved_references[reference_name] = reference
        glbls.update(resolved_references)  # may shadow existing glbls

        return glbls

    @tracer.start_as_current_span("code.coerce_outputs")
    def _coerce_outputs(self, outputs_raw: Any) -> CustomObject:
        """Coerves raw outputs into the output type for this run."""
        assert self.output_type, f"no output type for {self!r}"
        outputs = coerce_custom_object_scalar(outputs_raw, self.output_type)
        return outputs


class CodeScriptRunner(CodeRunner):
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
        with self._capture_logs(), tracer.start_as_current_span("code.run.script") as span:
            span.set_attribute("code", compiled.code)
            if compiled.is_coroutine:
                coro = eval(compiled.body_co, glbls)
                await coro
            else:
                exec(compiled.body_co, glbls)
            logger.trace(
                "code.run", runner=self, code=cast(Code, self.code).to_string(), span="current"
            )


class CodeFunctionRunner(CodeRunner):
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
                if field.code_name:
                    glbls[field.code_name] = value

        # run
        exec(compiled.body_co, glbls)  # shouldn't error
        func = glbls[compiled.function_name]
        with self._capture_logs(), tracer.start_as_current_span("code.run.function") as span:
            span.set_attribute("code", compiled.code)
            token = IS_IN_USER_CODE.set(True)
            try:
                if compiled.is_coroutine:
                    outputs_raw = await func()
                else:
                    outputs_raw = func()
                logger.debug(
                    "code.run", runner=self, code=cast(Code, self.code).to_string(), span="current"
                )
            except Exception as e:
                logger.debug(
                    "code.run.error",
                    runner=self,
                    code=cast(Code, self.code).to_string(),
                    span="current",
                    exc_info=e,
                )
                raise
            finally:
                IS_IN_USER_CODE.reset(token)
        self.outputs = self._coerce_outputs(outputs_raw)
