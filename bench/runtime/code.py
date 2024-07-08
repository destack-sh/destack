from typing import override

from bench.language.run import CodeKind, RunnableKind
from bench.runtime.compiler import compile_code
from bench.runtime.runner import Runner, runner

# NOTE :Performance :Robustness: run (some?) sync code in a separate thread?


@runner((RunnableKind.CODE, CodeKind.SNIPPET))
class CodeSnippetRunner(Runner):
    @override
    async def run(self) -> None:
        raise NotImplementedError


@runner((RunnableKind.CODE, CodeKind.SCRIPT))
class CodeScriptRunner(Runner):
    @override
    async def run(self) -> None:
        assert self.runnable.code, f"no code for {self!r}"
        compiled = self.runnable.compiled
        if compiled is None:
            self.runnable.compiled = compiled = compile_code(
                str(self.runnable.code.id),
                self.runnable.code.to_string(),
                CodeKind.SCRIPT,
                self.runner.glbls,
            )
        if compiled.syntax_error:
            raise compiled.syntax_error  # re-raise
        elif not compiled.body_co:
            return  # empty code

        # NOTE :Incomplete!: script exports
        assert compiled.body_co, f"no compiled code for {self!r}"
        # nocheckin: proper context
        glbls = {**self.runner.glbls, "self": self.scope}
        if compiled.is_coroutine:
            coro = eval(compiled.body_co, glbls)
            await coro
        else:
            exec(compiled.body_co, glbls)


@runner((RunnableKind.CODE, CodeKind.FUNCTION))
class CodeFunctionRunner(Runner):
    @override
    async def run(self) -> None:
        raise NotImplementedError
