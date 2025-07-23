import ast
import asyncio
import functools
import os
import subprocess
import textwrap
import traceback
from typing import TYPE_CHECKING, Any

import structlog
import typer
from opentelemetry import trace

if TYPE_CHECKING:
    from destack.language import GraphKey, Region

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


def async_to_sync(func=None):
    """Automatically convert async functions to sync if not called in async context."""

    def decorate(func):
        # check that the func is async
        if not asyncio.iscoroutinefunction(func):
            raise TypeError(f"{func} is not a coroutine function")

        @functools.wraps(func)
        def wrapped(*args, **kwargs):
            # are we in an async context?
            try:
                asyncio.get_running_loop()
                is_in_loop = True
            except RuntimeError:
                is_in_loop = False
            if is_in_loop:
                return func(*args, **kwargs)
            else:
                try:
                    import uvloop

                    return uvloop.run(func(*args, **kwargs))
                except ImportError:
                    return asyncio.run(func(*args, **kwargs))

        return wrapped

    if func is None:
        return decorate
    else:
        return decorate(func)


def run_shell_sync(cmd: str, check=True, **kwargs):
    """Executes a shell command in a subprocess."""
    cwd = os.getcwd()
    logger.debug("shell", cmd=cmd, cwd=cwd, check=check, **kwargs)
    subprocess.run(cmd, shell=True, check=check, **kwargs)


def parse_region(region: "str | Region") -> "Region":
    """Parse a Region from a string."""
    from destack.language.core import REGION_BY_SLUG, Region

    if isinstance(region, Region):
        return region

    region = region.lower()
    try:
        if region in REGION_BY_SLUG:
            # try by slug
            return REGION_BY_SLUG[region]
        elif region in Region.__members__:
            # try by name
            return Region[region]
        else:
            # try by value
            return Region(int(region))
    except (TypeError, ValueError) as e:
        raise typer.BadParameter(
            f"invalid region: '{region}' (expected: {'|'.join(r.slug for r in Region)})"
        ) from e


def parse_store_key(store_key: "str | GraphKey") -> "GraphKey":
    """Parse a NodeArea from a string."""
    from destack.language.core import GraphKey

    if isinstance(store_key, GraphKey):
        return store_key

    store_key = store_key.upper()
    try:
        if store_key in GraphKey.__members__:
            # try by name
            return GraphKey[store_key]
        else:
            # try by value
            return GraphKey(int(store_key))
    except (TypeError, ValueError) as e:
        raise typer.BadParameter(
            f"invalid store key: '{store_key}' (expected: {'|'.join(a.name.lower() for a in GraphKey)})"
        ) from e


#
# Repl
#


def _contains_await(src: str) -> bool:
    """Return True if the given source code contains an 'await' anywhere."""
    try:
        tree = ast.parse(src)
    except SyntaxError:
        return False
    return any(isinstance(node, ast.Await) for node in ast.walk(tree))


async def eval_async(source: str, env: dict[str, Any]) -> Any:
    """Evaluate/execute code asynchronously, supporting top-level await."""
    # If the code has an await anywhere, wrap it in an async def
    if _contains_await(source):
        wrapped = f"async def _temp_func():\n{textwrap.indent(source, '    ')}"
        try:
            exec(wrapped, env, env)  # Defines _temp_func in env
            return await env["_temp_func"]()
        finally:
            env.pop("_temp_func", None)
    else:
        # If it's an expression, eval it; otherwise, exec it
        try:
            code_obj = compile(source, "<repl>", "eval")
            return eval(code_obj, env, env)
        except SyntaxError:
            code_obj = compile(source, "<repl>", "exec")
            exec(code_obj, env, env)
            return None


async def repl(banner: str, vars: dict[str, Any]):
    """Interactive async Python shell with syntax highlighting, auto-suggestions, etc."""
    print(banner)  # noqa: T201

    from prompt_toolkit import PromptSession
    from prompt_toolkit.auto_suggest import AutoSuggestFromHistory
    from prompt_toolkit.history import InMemoryHistory
    from prompt_toolkit.key_binding import KeyBindings
    from prompt_toolkit.lexers import PygmentsLexer
    from prompt_toolkit.styles import Style
    from pygments.lexers import PythonLexer

    kb = KeyBindings()

    fancy_style = Style.from_dict(
        {
            "pygments.keyword": "bold ansigreen",
            "pygments.comment": "italic ansiwhite",
            "pygments.string": "ansimagenta",
            "pygments.number": "ansiblue",
            "prompt": "bold ansiyellow",
        }
    )

    @kb.add("c-c")  # type: ignore
    @kb.add("c-d")  # type: ignore
    def _(event):
        event.app.exit()

    session = PromptSession(
        lexer=PygmentsLexer(PythonLexer),
        history=InMemoryHistory(),
        auto_suggest=AutoSuggestFromHistory(),
        key_bindings=kb,
        style=fancy_style,
    )

    while True:
        try:
            text = await session.prompt_async(">>> ")
            if not text.strip():
                continue
            try:
                result = await eval_async(text, vars)
                if result is not None:
                    print(repr(result))  # noqa: T201
            except Exception:
                traceback.print_exc()
        except (EOFError, KeyboardInterrupt):
            break
