from typing import Any, override

from bench.language.run import CodeKind, RunnableKind
from bench.language.value import ValueObject, coerce_value_object
from bench.runtime.compiler import CompiledCode, compile_code
from bench.runtime.runner import Runner, runner

# NOTE :Performance :Robustness: run (some?) sync code in a separate thread?


class CodeRunnerBase(Runner):
    """Common base for compiling and running code."""

    async def _compile_code(self, kind: CodeKind) -> CompiledCode:
        """Prepares valid compiled code (raises SyntaxError if invalid)."""
        assert self.runnable.code, f"no code for {self!r}"
        compiled = self.runnable.compiled
        if compiled is None:
            self.runnable.compiled = compiled = compile_code(
                str(self.runnable.code.id),
                self.runnable.code.to_string(),
                kind,
                self.runner.glbls,
            )
        if compiled.syntax_error:
            raise compiled.syntax_error  # re-raise
        return compiled

    def _coerce_outputs(self, outputs_raw: Any) -> ValueObject:
        assert self.handle.run and self.handle.run.output_type, f"no output type for {self!r}"
        outputs = coerce_value_object(self.handle.run.output_type, outputs_raw)
        return outputs


@runner((RunnableKind.CODE, CodeKind.SNIPPET))
class CodeSnippetRunner(CodeRunnerBase):
    """Run a code snippet and updates the value of its last expression."""

    @override
    async def run(self) -> None:
        raise NotImplementedError


@runner((RunnableKind.CODE, CodeKind.SCRIPT))
class CodeScriptRunner(CodeRunnerBase):
    """Run a code script and updates its exported definitions."""

    @override
    async def run(self) -> None:
        # compile
        compiled = await self._compile_code(CodeKind.SCRIPT)
        if not compiled.body_co:
            return  # empty

        # context
        # nocheckin: proper context

        # run it
        glbls = {**self.runner.glbls, "self": self.node}
        if compiled.is_coroutine:
            coro = eval(compiled.body_co, glbls)
            await coro
        else:
            exec(compiled.body_co, glbls)


@runner((RunnableKind.CODE, CodeKind.FUNCTION))
class CodeFunctionRunner(CodeRunnerBase):
    """Run a code function and update its outputs."""

    @override
    async def run(self) -> None:
        # compile
        compiled = await self._compile_code(CodeKind.FUNCTION)
        if not compiled.body_co:
            return  # empty
        assert compiled.function_name, f"no function name for {self!r}"

        # context
        glbls = {**self.runner.glbls, "self": self.node}
        assert self.inputs is not None, f"no inputs for {self!r}"
        for field in self.inputs.fields:
            value = self.inputs._do_get(field)
            glbls[field.name] = value
            if field.py_ident:
                glbls[field.py_ident] = value

        # run it
        exec(compiled.body_co, glbls)
        func = glbls[compiled.function_name]
        if compiled.is_coroutine:
            outputs_raw = await func()
        else:
            outputs_raw = func()

        self.handle.outputs = self._coerce_outputs(outputs_raw)
