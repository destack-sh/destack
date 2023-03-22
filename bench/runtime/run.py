from __future__ import annotations

import asyncio
import enum
import hashlib
import inspect
import json
import re
import textwrap
import typing
from asyncio import iscoroutinefunction
from collections import OrderedDict
from dataclasses import dataclass
from json import JSONDecodeError
from random import Random
from typing import Any, Optional
from uuid import UUID, uuid4

import PIL.Image
import pydub
import structlog
from django.db import models
from more_itertools import first, last

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
from bench.runtime.model import InferenceContext, InferenceEndpoint, get_endpoints
from bench.runtime.tracing import Tracer
from bench.runtime.type import (
    AsyncCodeCallable,
    CodeInstance,
    DatasetInstance,
    Modality,
    ModelInference,
    ModelInstance,
    SymbolInstance,
    SyncCodeCallable,
    TaskInstance,
    TypeInstance,
    ValueInstance,
    XBlocks,
    summarize_args,
)
from bench.runtime.x import X_BUILTINS
from bench.utils.cache import redis
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
    ASSEMBLYAI = "assemblyai"


# TODO @Cleanup: static builtins should be in the run environment context?
STATIC_BUILTINS = {
    # primitive type builtins
    "string": str,
    "number": float,
    "null": None,
    "boolean": bool,
    "image": PIL.Image.Image,
    "audio": pydub.AudioSegment,
    # functional builtins
    "first": first,
    "last": last,
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
    """A worker-side proxy for tracing (and caching) a specific inference endpoint."""

    def __init__(
        self,
        ctx: InferenceContext,
        modality: Modality,
        endpoint: InferenceEndpoint,
        tracer: Tracer,
        cache_inferences: bool,
    ):
        self.ctx = ctx
        self.modality = modality
        self.endpoint = endpoint
        self.tracer = tracer
        self.cache_inferences = cache_inferences

    # insecure hash is fine here, it's just for caching
    # noinspection InsecureHash
    async def __call__(self, blocks: list[XBlock], settings: Any) -> Any:
        # make hash key
        block_strings = [f"{b.kind}{b.source}{b.value}{b.path}" for b in blocks]
        blocks_hash = hashlib.sha256("".join(block_strings).encode("utf-8")).hexdigest()
        settings_hash = hashlib.sha256(
            json.dumps(settings, sort_keys=True).encode("utf-8")
        ).hexdigest()
        cache_key = f"inference.{self.ctx.model.fqn}.{self.modality}:{blocks_hash}:{settings_hash}"

        log = logger.bind(
            modality=self.modality,
            ctx=self.ctx,
            blocks=len(blocks),
            cache_key=cache_key,
            cache_inferences=self.cache_inferences,
        )

        if self.cache_inferences:
            cached_ret = await redis.get(cache_key)
            if cached_ret is not None:
                try:
                    ret = json.loads(cached_ret)
                    log.debug("inference.cache.hit", ret=summarize_args(ret))
                    return ret
                except JSONDecodeError:
                    log.error("inference.cache.error", excinfo=True)
                    # ignore and continue, will be overwritten

        try:
            self.tracer.inference_enter(self.ctx, blocks, settings)
            log.debug("inference.call.enter")
            ret = await self.endpoint(blocks, settings)
            self.tracer.inference_exit(self.ctx, blocks, settings, ret)
            log.debug("inference.call.exit", ret=summarize_args(ret))

            if self.cache_inferences:
                # ret is assumed to be JSON-serializable
                # (may not be true when we get to images, but this will error obviously enough)
                await redis.set(cache_key, json.dumps(ret))

            return ret
        except Exception as exception:
            self.tracer.inference_exception(self.ctx, blocks, settings, exception)
            log.debug("inference.call.exception", excinfo=True)
            raise


class Proxy:
    """A worker-side proxy for wrapping symbol access."""

    def __init__(self, tracer: Tracer, cache_inferences: bool):
        self.tracer = tracer
        self.cache_inferences = cache_inferences

    def proxy_code(self, code: CodeInstance) -> CodeInstance:
        code_proxy_cls = (
            AsyncCodeProxy if iscoroutinefunction(code.code_callable) else SyncCodeProxy
        )
        code_proxy = code_proxy_cls(code, code.code_callable, self.tracer)
        code.code_callable = code_proxy
        return code

    def proxy_model(self, model: ModelInstance) -> ModelInstance:
        # proxy every available modality endpoint (i.e. method) on the model
        inference = typing.cast(ModelInferenceImpl, model.inference)
        inference_proxy = ModelInferenceImpl(ctx=inference.ctx)

        for modality in Modality:
            if hasattr(inference, modality):
                endpoint = getattr(inference, modality)
                endpoint_proxy = InferenceProxy(
                    ctx=inference.ctx,
                    modality=modality,
                    endpoint=endpoint,
                    tracer=self.tracer,
                    cache_inferences=self.cache_inferences,
                )
                setattr(inference_proxy, modality, endpoint_proxy)
        model.inference = inference_proxy
        return model


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
        "xblocks": code.xblocks,
        "x": XBlocks(code.xblocks),
        **inlined_context,
        "__statement__": code.source,
        "__file__": code.source.file,
        "__module__": code.source.file.module,
        "random": Random(code.source.id.hex.encode()),
    }

    if code.language == "python":
        python_code = code.code or "pass"
        locals = {**STATIC_BUILTINS, **dynamic_context}
        is_async = "await " in python_code  # TODO @Robustness: detect async python code properly
    elif code.language == "x":
        python_code = code.code or "pass"
        locals = {**STATIC_BUILTINS, **X_BUILTINS, **dynamic_context}
        is_async = True
    else:
        raise ValueError(f"unknown code language: {code}")

    # create python function from python code
    input_keys = code.type_node.input.keys
    func_name = f"_{_to_pyidentifier(code.name)}_{code.source.id.hex[:6]}"
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


@dataclass
class ModelInferenceImpl(ModelInference):
    ctx: InferenceContext


def _instantiate_model_inference(model: Model) -> ModelInference:
    # Model inference assumes its context is unique per instance :ReusableInstances
    #  (could also just use context vars for this)
    ctx = InferenceContext(model=model, n=1, user_opaque_id=model.id.hex, streaming_callback=None)

    impl = ModelInferenceImpl(ctx)

    endpoints = get_endpoints(model)
    if not endpoints:
        raise RuntimeError(f"no endpoints found for model: {model}")
    for modality, endpoint_cls in endpoints:
        endpoint = getattr(endpoint_cls(ctx), modality)
        setattr(impl, modality, endpoint)

    return impl


def instantiate(
    symbol: InterpSymbol,
    idx: ModuleIndex,
    build: Optional[Build] = None,
    proxy: Proxy | None = None,
) -> SymbolInstance:
    """Instantiate a symbol in a build with all relevant context recursively."""
    if symbol.abstract:
        raise ValueError(f"cannot instantiate abstract symbol: {symbol}")
    proxy = proxy or Proxy(tracer=Tracer(), cache_inferences=False)
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
            implementation=typing.cast(CodeInstance, implementation_instance),
        )
        implementation_instance.task = task
        return task
    elif isinstance(symbol, Code):
        code_str, code_callable = _instantiate_code_callable(symbol, instantiated_context)
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
        inference = _instantiate_model_inference(symbol)
        model_instance = ModelInstance(**symbol.__dict__, inference=inference, build=build)
        return proxy.proxy_model(model_instance)
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
    """Runs the code instance synchronously. Not to be used in production."""
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
            return await asyncio.to_thread(code.py_handle, **arguments)
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
