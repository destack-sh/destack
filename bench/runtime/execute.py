from __future__ import annotations

import enum
import functools
import textwrap
import time
import typing
from asyncio import iscoroutinefunction
from collections import OrderedDict
from dataclasses import replace
from random import Random
from typing import Any
from uuid import UUID, uuid4

import structlog
from django.db import models

from bench.language.parse import ModuleIndex
from bench.language.type import (
    CodeContent,
    DatasetContent,
    LiteralValue,
    ModelContent,
    Statement,
    StatementType,
    TypeContent,
    TypeNode,
    TypeTag,
    ValueContent,
)
from bench.runtime.bpl import (
    BPL_BUILTINS,
    DynamicPrompt,
    InferenceContext,
    parse_bpl,
    run_bpl_controlled,
    run_bpl_speculative,
)
from bench.runtime.inference import LocalHfTransformersInference, OpenAIInference
from bench.runtime.tracing import Tracer
from bench.runtime.type import (
    AsyncCodeCallable,
    CodeInstance,
    DatasetInstance,
    DecoderSettings,
    ModelInstance,
    StatementInstance,
    SymbolInstance,
    SyncCodeCallable,
    TypeInstance,
    ValueInstance,
)
from bench.settings import DEBUG, TEST
from bench.utils.record import RecordList


class ProviderKey(models.TextChoices):
    OPENAI = "openai"
    GOOSEAI = "gooseai"
    AI21 = "ai21"
    TRANSFORMERS = "transformers"


CAN_EXEC = DEBUG or TEST
# TODO @Cleanup: static builtins should be in the run environment context?
STATIC_BUILTINS = {
    # primitive type builtins
    "string": str,
    "number": float,
    "null": None,
    "boolean": bool,
}


logger = structlog.stdlib.get_logger()


class RunErrorType(enum.Enum):
    INTERNAL = 0, "Internal error"
    PARSE = 1, "Parse error"
    VALIDATION = 2, "Validation error"
    RUNTIME = 3, "Runtime code error"

    def __new__(cls, value, description):
        obj = object.__new__(cls)
        obj._value_ = value
        obj.description = description
        return obj


class RunError(Exception):
    def __init__(
        self,
        _t: RunErrorType,
        statement: typing.Optional[Statement],
        cause: typing.Optional[Exception] = None,
    ):
        self.type = _t
        self.statement = statement
        self.cause = cause
        super().__init__(self.type.description)


ModelInference = typing.NamedTuple("ModelInference", [("id", UUID), ("output", dict)])


class SyncCodeProxy:
    """A worker-side proxy for code tracing."""

    def __init__(self, code: CodeInstance, tracer: Tracer):
        self.code = code
        self.tracer = tracer

    def __call__(self, *args, **kwargs):
        log = logger.bind(code=self.code, args=len(args), kwargs=_summarize_args(kwargs))
        self.tracer.code_enter(self.code, args, kwargs)
        log.debug("code.call.enter")
        try:
            result = self.code.code_callable(*args, **kwargs)
            self.tracer.code_exit(self.code, args, kwargs, result)
            log.debug("code.call.exit", result=_summarize_args(result))
            return result
        except Exception as exception:
            self.tracer.code_exception(self.code, args, kwargs, exception)
            log.debug("code.call.exception", exception=exception)
            raise


class AsyncCodeProxy:
    """A worker-side proxy for code tracing."""

    def __init__(self, code: CodeInstance, tracer: Tracer):
        self.code = code
        self.tracer = tracer

    async def __call__(self, *args, **kwargs):
        log = logger.bind(code=self.code, args=len(args), kwargs=_summarize_args(kwargs))
        self.tracer.code_enter(self.code, args, kwargs)
        log.debug("code.call.enter")
        try:
            result = await self.code.code_callable(*args, **kwargs)
            self.tracer.code_exit(self.code, args, kwargs, result)
            log.debug("code.call.exit", result=_summarize_args(result))
            return result
        except Exception as exception:
            self.tracer.code_exception(self.code, args, kwargs, exception)
            log.debug("code.call.exception", exception=exception)
            raise


class InferenceContextProxy:
    """A worker-side proxy for inference tracing."""

    def __init__(self, context: InferenceContext, tracer: Tracer):
        self.context = context
        self.tracer = tracer
        # inference immediately enters on init (maybe not great?)
        # see InferenceContext for the actual implementation
        self.tracer.inference_enter(self.context)
        logger.debug("inference.enter", context=self.context)

    async def generate(self, step: DecoderSettings) -> str:
        start_time = time.time()
        ret = await self.context.generate(step)
        duration = time.time() - start_time
        self.tracer.inference_generate(self.context, step, duration)
        logger.debug("inference.generate", step=step, ret=len(ret), duration=duration)
        return ret

    def close(self):
        self.tracer.inference_exit(self.context)
        logger.debug("inference.exit", context=self.context)

    # forward all other methods to the underlying context
    def __getattr__(self, name):
        return getattr(self.context, name)


class Proxy:
    """A worker-side proxy for wrapping symbol access."""

    def __init__(self, tracer: Tracer):
        self.tracer = tracer

    def proxy_code(self, code: CodeInstance) -> CodeInstance:
        code_proxy_cls = (
            AsyncCodeProxy if iscoroutinefunction(code.code_callable) else SyncCodeProxy
        )
        code_proxy = code_proxy_cls(code, self.tracer)
        return replace(code, code_callable=code_proxy)

    def proxy_inference(self, context: InferenceContext) -> InferenceContext:
        proxy = InferenceContextProxy(context, self.tracer)
        return typing.cast(InferenceContext, proxy)  # not same type, but duck-typed

    def proxy_symbol(self, symbol: SymbolInstance) -> SymbolInstance:
        if isinstance(symbol, TypeInstance):
            return symbol  # not proxied
        elif isinstance(symbol, DatasetInstance):
            return symbol  # not proxied
        elif isinstance(symbol, ValueInstance):
            return symbol  # not proxied
        elif isinstance(symbol, ModelInstance):
            return symbol  # not proxied
        elif isinstance(symbol, CodeInstance):
            return self.proxy_code(symbol)
        else:
            raise ValueError(f"unknown symbol type: {symbol}")


def unwrap(value: StatementInstance):
    if not isinstance(value, SymbolInstance):
        raise ValueError(f"cannot unwrap {value}")
    return value.py_handle


def unwrap_args(self, arguments: dict[str, Any]) -> dict[str, Any]:
    return {name: self.unwrap(value) for name, value in arguments.items()}


def instantiate_py_type(node: TypeNode) -> type | LiteralValue:
    if node.type == TypeTag.STRING:
        return str
    elif node.type == TypeTag.NUMBER:
        return float
    elif node.type == TypeTag.NULL:
        return type(None)
    elif node.type == TypeTag.BOOLEAN:
        return bool
    elif node.type == TypeTag.ARRAY:
        return list
    elif node.type == TypeTag.UNION:
        return typing.Union[tuple(instantiate_py_type(child) for child in node.children)]
    elif node.type == TypeTag.STRUCT:
        return typing.TypedDict(
            node.name,
            {node.name: instantiate_py_type(node) for node in node.children},
        )
    elif node.type == TypeTag.ENUM:
        # create 'fake' enum with the given constants pointing to themselves
        # assumes enums are value enums (not type union enums)
        if node.head_type.type == TypeTag.STRING:
            enum_cls = enum.StrEnum
        elif node.head_type.type == TypeTag.NUMBER:
            enum_cls = enum.IntEnum
        else:
            raise ValueError(f"unexpected enum head type: {node.head_type}")
        members = {child.name: child.value for child in node.members}
        enum_name = node.name or "_anon_" + uuid4().hex
        return enum_cls(enum_name, members)
    elif node.type == TypeTag.LITERAL:
        return node.value
    else:
        raise ValueError(f"unexpected type node: {node}")


def _instantiate_code_callable(
    code: CodeContent,
    context: OrderedDict[str, StatementInstance],
    proxy: Proxy | None,
) -> tuple[str | None, SyncCodeCallable | AsyncCodeCallable, DynamicPrompt | None]:
    """
    Instantiates code into a Python callable in the context.
    If the code is a dynamic prompt (BPL), the callable will be wrapped and use the proxy for contexts.
    """
    if code.builtin_id:
        # builtins are already defined and are just curried using the arguments
        builtin = STATIC_BUILTINS.get(code.builtin_id)
        if builtin is None:
            raise ValueError(f"unknown builtin in {code}: {code.builtin_id}")
        return None, builtin, None

    unwrapped_context = {name: unwrap(value) for name, value in context.items()}
    dynamic_context = {
        "source_context": context,
        "context": unwrapped_context,
        # 'inline' all context variables that are valid Python identifiers
        **{name: value for name, value in unwrapped_context.items() if name.isidentifier()},
        "__statement__": code.definition,
        "__file__": code.definition.file,
        "__module__": code.definition.file.module,
        "random": Random(code.definition.id.hex.encode()),
    }

    # transform to python code if necessary
    prompt = None
    if code.language == "python":
        python_code = code.code
        locals = {**STATIC_BUILTINS, **dynamic_context}
        is_async = "await " in python_code  # TODO @Cleanup: detect async python code properly
    elif code.language == "bpl":
        prompt = parse_bpl(code.code, dynamic_context)
        python_code = prompt.python_code
        locals = {**STATIC_BUILTINS, **BPL_BUILTINS, **dynamic_context}
        is_async = True
    else:
        raise ValueError(f"unknown code language: {code}")

    # create python function from python code
    input_keys = code.type_node.input.keys
    func_name = f"_anon_{code.definition.id.hex}"
    async_str = "async " if is_async else ""
    func_params = ", ".join(input_keys)
    indented_code = textwrap.indent(python_code, " " * 4)
    code_str = f"{async_str}def {func_name}({func_params}):\n{indented_code}"
    try:
        callable = _execute_code(code_str, locals)[func_name]
    except Exception as e:
        # shouldn't error unless it's a python parse issue since we're just defining a function
        raise RunError(RunErrorType.PARSE, code.definition, cause=e) from e

    # wrap function to manage inference contexts
    if code.language == "bpl":
        callable = wrap_prompt_callable(callable, prompt, proxy)

    return python_code, callable, prompt


def wrap_prompt_callable(
    callable: AsyncCodeCallable, prompt: DynamicPrompt, proxy: Proxy | None
) -> AsyncCodeCallable:
    """Wraps a BPL callable to manage inference contexts."""

    @functools.wraps(callable)
    async def wrapped_callable(*args, **kwargs):
        if prompt.settings.model.provider == ProviderKey.TRANSFORMERS:
            inference = LocalHfTransformersInference(prompt.settings.model)
            run = run_bpl_controlled
        elif prompt.settings.model.provider == ProviderKey.OPENAI:
            inference = OpenAIInference(prompt.settings.model)
            run = run_bpl_speculative
        else:
            raise ValueError(f"unknown inference provider: {prompt.settings.model.provider}")

        ctx = InferenceContext(prompt.settings, inference)
        if proxy is not None:
            ctx = proxy.proxy_inference(ctx)
        try:
            generator = callable(*args, **kwargs)
            return await run(generator, ctx)
        finally:
            ctx.close()

    return wrapped_callable


def instantiate(
    statement: Statement, idx: ModuleIndex, proxy: Proxy | None = None
) -> StatementInstance:
    """Instantiate a statement, its context and children (recursively)."""
    context = get_context(statement, idx, used_only=True)

    proxy = proxy or Proxy(tracer=Tracer())
    # instantiate context (preserving order)
    instantiated_context = OrderedDict()
    for name, value in context.items():
        instantiated_context[name] = instantiate(statement=value, idx=idx, proxy=proxy)

    # resolve to underlying content if it's an import
    if statement.type == StatementType.IMPORT:
        content = statement.reference.content
    else:
        content = statement.content

    # instantiate statement itself
    if isinstance(content, CodeContent):
        code_str, code_callable, prompt = _instantiate_code_callable(
            content, instantiated_context, proxy
        )
        instance = CodeInstance(
            **content.__dict__,
            transformed_code=code_str,
            code_callable=code_callable,
            prompt=prompt,
        )
    elif isinstance(content, ValueContent):
        instance = ValueInstance(**content.__dict__)
    elif isinstance(content, ModelContent):
        instance = ModelInstance(**content.__dict__)
    elif isinstance(content, DatasetContent):
        instance = DatasetInstance(**content.__dict__, records_batch=RecordList(content.records))
    elif isinstance(content, TypeContent):
        py_type = instantiate_py_type(content.type_node)
        instance = TypeInstance(**content.__dict__, py_type=py_type)
    else:
        raise ValueError(f"cannot instantiate {statement}")
    return proxy.proxy_symbol(instance)


def get_context(
    statement: Statement, idx: ModuleIndex, used_only: bool
) -> OrderedDict[str, Statement]:
    # gather all available statements: everything above and next to the statement
    available_statements = []
    current_parent = statement.parent
    while current_parent is not None:
        available_statements.extend(idx.statements_by_parent[current_parent.id])
        current_parent = current_parent.parent
    available_statements.extend(statement.file.root_statements)

    available_context = OrderedDict()
    for available_statement in available_statements:
        var_name = available_statement.name
        if var_name is not None and var_name not in available_context:
            # there may be local shadowing, so use the first reference
            # (also ignore duplicate definitions, that's for semantic parse)
            available_context[var_name] = available_statement

    if not used_only:
        return available_context

    # filter to used context only
    # TODO @Cleanup: improve context visibility filters (beyond just string matching)
    if isinstance(statement.content, CodeContent):
        used_keys = {key for key in available_context if key in statement.content.code}
    else:
        used_keys = set()

    used_context = OrderedDict()
    for key in available_context:  # preserve order
        if key in used_keys:
            used_context[key] = available_context[key]
    return used_context


def execute_sync(
    code: CodeInstance, arguments: dict[str, LiteralValue] | None = None
) -> LiteralValue:
    arguments = arguments or {}
    try:
        return code.py_handle(**arguments)
    except Exception as e:
        raise RunError(RunErrorType.RUNTIME, code.definition, cause=e) from e


async def execute(
    code: CodeInstance, arguments: dict[str, LiteralValue] | None = None
) -> LiteralValue:
    arguments = arguments or {}
    try:
        return await code.py_handle(**arguments)
    except Exception as e:
        raise RunError(RunErrorType.RUNTIME, code.definition, cause=e) from e


def _summarize_args(arguments: Any) -> str:
    """
    Summarize the names (if available) and types of arguments.
    """
    if isinstance(arguments, dict):
        return ", ".join(f"{name}={type(value).__name__}" for name, value in arguments.items())
    elif isinstance(arguments, (list, tuple, set)):
        return ", ".join(type(value).__name__ for value in arguments)
    else:
        return type(arguments).__name__


def _execute_code(code: str, globals: dict[str, Any]) -> dict:
    # remember the globals we started with, do not modify originals
    globals_local = {**globals}
    globals_local_keys_initial = {*globals_local.keys()}
    _do_execute(code, globals_local)
    new_globals = {
        k: v
        for k, v in globals_local.items()
        if k not in globals_local_keys_initial and k not in ("__builtins__", "__annotations__")
    }
    return new_globals


def _do_execute(code: str, globals: dict):
    if not CAN_EXEC:
        raise RuntimeError("exec outside sandbox is not allowed")

    exec(code, globals)
