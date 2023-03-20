from __future__ import annotations

import enum
import inspect
import re
import textwrap
import time
import typing
from asyncio import iscoroutinefunction
from collections import OrderedDict
from random import Random
from typing import Any, Optional
from uuid import UUID, uuid4

import PIL.Image
import pydub
import structlog
from django.db import models

from bench.language import ModuleIndex
from bench.language.type import (
    Build,
    Code,
    Dataset,
    InterpSymbol,
    LiteralValue,
    Model,
    Task,
    Type,
    TypeNode,
    TypeTag,
    Value,
    XBlock,
)
from bench.runtime.model import InferenceContext, InferenceEndpoint
from bench.runtime.tracing import Tracer
from bench.runtime.type import (
    AsyncCodeCallable,
    CodeInstance,
    DatasetInstance,
    ModelInstance,
    SymbolInstance,
    SyncCodeCallable,
    TaskInstance,
    TypeInstance,
    ValueInstance,
    summarize_args,
)
from bench.utils.record import RecordList

logger = structlog.stdlib.get_logger(__name__)


class ProviderKey(models.TextChoices):
    OPENAI = "openai"
    GOOSEAI = "gooseai"
    AI21 = "ai21"
    FOREFRONT = "forefront"
    COHERE = "cohere"
    ANTHROPIC = "anthropic"
    STABILITYAI = "stabilityai"
    HUGGINGFACE = "huggingface"


# TODO @Cleanup: static builtins should be in the run environment context?
STATIC_BUILTINS = {
    # primitive type builtins
    "string": str,
    "number": float,
    "null": None,
    "boolean": bool,
    "image": PIL.Image.Image,
    "audio": pydub.AudioSegment,
}


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
        symbol: typing.Optional[InterpSymbol],
        cause: typing.Optional[Exception] = None,
    ):
        self.type = _t
        self.symbol = symbol
        self.cause = cause
        super().__init__(self.type.description)


ModelInference = typing.NamedTuple("ModelInference", [("id", UUID), ("output", dict)])


class SyncCodeProxy:
    """A worker-side proxy for code tracing."""

    def __init__(self, code: CodeInstance, raw_callable: SyncCodeCallable, tracer: Tracer):
        self.code = code
        self.raw_callable = raw_callable
        self.tracer = tracer

    def __call__(self, *args, **kwargs):
        log = logger.bind(code=self.code, args=len(args), kwargs=summarize_args(kwargs))
        try:
            self.tracer.code_enter(self.code, args, kwargs)
            log.debug("code.call.enter")
            result = self.raw_callable(*args, **kwargs)
            self.tracer.code_exit(self.code, args, kwargs, result)
            log.debug("code.call.exit", result=summarize_args(result))
            return result
        except Exception as exception:
            self.tracer.code_exception(self.code, args, kwargs, exception)
            log.debug("code.call.exception", excinfo=True)
            raise


class AsyncCodeProxy:
    """A worker-side proxy for code tracing."""

    def __init__(self, code: CodeInstance, raw_callable: AsyncCodeCallable, tracer: Tracer):
        self.code = code
        self.raw_callable = raw_callable
        self.tracer = tracer

    async def __call__(self, *args, **kwargs):
        log = logger.bind(code=self.code, args=len(args), kwargs=summarize_args(kwargs))
        try:
            self.tracer.code_enter(self.code, args, kwargs)
            log.debug("code.call.enter")
            result = await self.raw_callable(*args, **kwargs)
            self.tracer.code_exit(self.code, args, kwargs, result)
            log.debug("code.call.exit", result=summarize_args(result))
            return result
        except Exception as exception:
            self.tracer.code_exception(self.code, args, kwargs, exception)
            log.debug("code.call.exception", excinfo=True)
            raise


class InferenceProxy:
    """A worker-side proxy for inference tracing."""

    def __init__(self, endpoint: InferenceEndpoint, tracer: Tracer):
        self.endpoint = endpoint
        self.tracer = tracer

    async def __call__(self, ctx: InferenceContext, *blocks: XBlock) -> Any:
        start_time = time.time()
        self.tracer.inference_enter(ctx, blocks)
        ret = await self.endpoint(ctx, *blocks)
        self.tracer.inference_exit(ctx, blocks, ret)
        logger.debug(
            "inference.generate",
            duration=time.time() - start_time,
        )
        return ret


class Proxy:
    """A worker-side proxy for wrapping symbol access."""

    def __init__(self, tracer: Tracer):
        self.tracer = tracer

    def proxy_code(self, code: CodeInstance) -> CodeInstance:
        code_proxy_cls = (
            AsyncCodeProxy if iscoroutinefunction(code.code_callable) else SyncCodeProxy
        )
        code_proxy = code_proxy_cls(code, code.code_callable, self.tracer)
        code.code_callable = code_proxy
        return code

    def proxy_inference(self, endpoint: InferenceEndpoint) -> InferenceEndpoint:
        return InferenceProxy(endpoint, self.tracer)


def unwrap(value: SymbolInstance):
    return value.py_handle


def unwrap_args(self, arguments: dict[str, Any]) -> dict[str, Any]:
    return {name: self.unwrap(value) for name, value in arguments.items()}


def instantiate_py_type(node: TypeNode) -> type | LiteralValue:
    # :PrimitiveTypeMap
    if node.tag == TypeTag.STRING:
        return str
    elif node.tag == TypeTag.NUMBER:
        return float
    elif node.tag == TypeTag.NULL:
        return type(None)
    elif node.tag == TypeTag.BOOLEAN:
        return bool
    elif node.tag == TypeTag.IMAGE:
        return PIL.Image.Image
    elif node.tag == TypeTag.AUDIO:
        return pydub.AudioSegment
    elif node.tag == TypeTag.ARRAY:
        return list
    elif node.tag == TypeTag.UNION:
        return typing.Union[tuple(instantiate_py_type(child) for child in node.children)]
    elif node.tag == TypeTag.STRUCT:
        return typing.TypedDict(
            node.name,
            {node.name: instantiate_py_type(node) for node in node.children},
        )
    elif node.tag == TypeTag.ENUM:
        # create 'fake' enum with the given constants pointing to themselves
        # assumes enums are value enums (not type union enums)
        if node.head_type.tag == TypeTag.STRING:
            enum_cls = enum.StrEnum
        elif node.head_type.tag == TypeTag.NUMBER:
            enum_cls = enum.IntEnum
        else:
            raise ValueError(f"unexpected enum head type: {node.head_type}")
        members = {child.name: child.value for child in node.members}
        enum_name = node.name or "_anon_" + uuid4().hex
        return enum_cls(enum_name, members)
    elif node.tag == TypeTag.LITERAL:
        return node.value
    elif node.tag == TypeTag.ANY:
        return Any
    else:
        raise ValueError(f"unexpected type node: {node}")


def _to_pyidentifier(name: str) -> str:
    return re.sub(r"\W|^(?=\d)", "_", name)


def _instantiate_code_callable(
    code: Code,
    context: OrderedDict[str, SymbolInstance],
    proxy: Proxy | None,
) -> tuple[str | None, SyncCodeCallable | AsyncCodeCallable]:
    """
    Instantiates code into a Python callable in the context.
    If the code is a dynamic prompt (BPL), the callable will be wrapped and use the proxy for contexts.
    """
    unwrapped_context = {name: unwrap(value) for name, value in context.items()}

    # inline all possible context variables
    # collect transformed invalid identifiers
    inlined_context = {
        _to_pyidentifier(name): value
        for name, value in unwrapped_context.items()
        if not name.isidentifier()
    }
    # overwrite with valid identifiers
    # TODO @Linting: check for indirect identifier collisions like this (e.g. 'a b' and 'a_b')
    inlined_context.update(
        {name: value for name, value in unwrapped_context.items() if name.isidentifier()}
    )

    dynamic_context = {
        "source_context": context,
        "context": unwrapped_context,
        **inlined_context,
        "__statement__": code.source,
        "__file__": code.source.file,
        "__module__": code.source.file.module,
        "random": Random(code.source.id.hex.encode()),
    }

    if code.language == "python":
        python_code = code.code or ""
        locals = {**STATIC_BUILTINS, **dynamic_context}
        is_async = "await " in python_code  # TODO @Robustness: detect async python code properly
    elif code.language == "x":
        python_code = code.code or ""
        locals = {**STATIC_BUILTINS, **dynamic_context}
        is_async = True
    else:
        raise ValueError(f"unknown code language: {code}")

    # create python function from python code
    input_keys = code.type_node.input.keys
    func_name = f"_{code.name}_{code.source.id.hex[:3]}"
    async_str = "async " if is_async else ""
    func_params = ", ".join(input_keys)
    indented_code = textwrap.indent(python_code, " " * 4)
    code_str = f"{async_str}def {func_name}({func_params}):\n{indented_code}"
    try:
        callable = _execute_code(code_str, locals)[func_name]
    except Exception as e:
        # shouldn't error unless it's a python parse issue since we're just defining a function
        raise RunError(RunErrorType.PARSE, code.source, cause=e) from e

    return python_code, callable


def instantiate(
    symbol: InterpSymbol,
    idx: ModuleIndex,
    build: Optional[Build] = None,
    proxy: Proxy | None = None,
) -> SymbolInstance:
    """Instantiate a symbol in a build with all relevant context recursively."""
    if symbol.abstract:
        raise ValueError(f"cannot instantiate abstract symbol: {symbol}")
    proxy = proxy or Proxy(tracer=Tracer())
    # instantiate context (preserving order)
    instantiated_context = OrderedDict()
    for name, value in symbol.context.items():
        if symbol is value:
            # self-reference is not supported for now
            # mainly because it would require either
            #  1) allowing invalid/mock initial instance state (and populate that later)
            #  2) tracking and somehow swapping the reference after it is actually created
            continue
        instantiated_context[name] = instantiate(value, idx=idx, build=build, proxy=proxy)

    if isinstance(symbol, Task):
        if build is None:
            raise ValueError(f"cannot instantiate task without build: {symbol}")
        target_id = build.get_target(symbol.id)
        if target_id is None:
            raise ValueError(f"cannot instantiate task in {build} without target: {symbol}")
        implementation = idx.symbol_by_id(target_id, Code)
        implementation_instance = instantiate(implementation, idx=idx, build=build, proxy=proxy)
        task = TaskInstance(
            **symbol.__dict__,
            build=build,
            implementation_instance=typing.cast(CodeInstance, implementation_instance),
        )
        implementation_instance.task = task
        return task
    elif isinstance(symbol, Code):
        code_str, code_callable = _instantiate_code_callable(symbol, instantiated_context, proxy)
        code_instance = CodeInstance(
            **symbol.__dict__,
            build=build,
            transformed_code=code_str,
            code_callable=code_callable,
            is_async=inspect.iscoroutinefunction(code_callable),
        )
        # TODO @Broken: set task on code instance if instantiated directly
        #  Likely will require breaking circles with a refmap.
        return proxy.proxy_code(code_instance)
    elif isinstance(symbol, Value):
        return ValueInstance(**symbol.__dict__, build=build)
    elif isinstance(symbol, Model):
        return ModelInstance(**symbol.__dict__, build=build)
    elif isinstance(symbol, Dataset):
        records_data = [r.data for r in symbol.records]
        return DatasetInstance(
            **symbol.__dict__, build=build, records_batch=RecordList(records_data)
        )
    elif isinstance(symbol, Type):
        py_type = instantiate_py_type(symbol)
        return TypeInstance(**symbol.__dict__, build=build, py_type=py_type)
    else:
        raise ValueError(f"cannot instantiate {symbol} in {build} (idx={idx})")


def run_sync(code: CodeInstance, arguments: dict[str, LiteralValue] | None = None) -> LiteralValue:
    arguments = arguments or {}
    try:
        if code.is_async:
            raise RuntimeError(f"cannot run async code synchronously: {code}")
        return code.py_handle(**arguments)
    except Exception as e:
        raise RunError(RunErrorType.RUNTIME, code, cause=e) from e


async def run(code: CodeInstance, arguments: dict[str, LiteralValue] | None = None) -> LiteralValue:
    arguments = arguments or {}
    try:
        if code.is_async:
            return await code.py_handle(**arguments)
        else:
            return code.py_handle(**arguments)
    except Exception as e:
        raise RunError(RunErrorType.RUNTIME, code, cause=e) from e


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
    exec(code, globals)


def map_runnable(
    idx: ModuleIndex,
    runconfig_id: Optional[UUID],
    runnable_id: Optional[UUID],
    build_id: Optional[UUID],
):
    pass
