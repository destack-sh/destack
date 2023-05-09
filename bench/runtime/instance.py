#
# Instances
#
import asyncio
import enum
import inspect
import itertools
import textwrap
import typing
from collections import OrderedDict
from dataclasses import dataclass, field
from random import Random
from typing import Any, Callable, Coroutine, Optional
from uuid import UUID

import numpy
import structlog
from more_itertools import first, last

from bench.language import (
    Build,
    Code,
    Dataset,
    Model,
    Module,
    ModuleIndex,
    Record,
    SymbolType,
    Task,
)
from bench.language.mutate import ModuleMutation
from bench.language.type import InterpSymbol, Type, TypeNode, TypeTag
from bench.language.typer import map_value
from bench.runtime.inference import InferenceContext, ModelInference
from bench.runtime.model import get_endpoints
from bench.runtime.run import Proxy
from bench.runtime.tracing import (
    ExecutionTracer,
    MultiTracer,
    PubExecutionTracker,
    Tracer,
    ValidationTracer,
)
from bench.runtime.utils import do_execute_arbitrary_code
from bench.runtime.x import X_BUILTINS
from bench.utils.func import describe_type, dict_minus
from bench.utils.utils import required_field, to_pyidentifier

logger = structlog.get_logger(__name__)

AsyncCodeCallable = Callable[..., Coroutine]
SyncCodeCallable = Callable[..., Any]


@dataclass(repr=False)
class ModuleInstance:
    module: Module
    index: ModuleIndex


@dataclass(repr=False)
class SymbolInstance:
    build: Optional[Build] = None

    @property
    def symbol_type(self):
        return SYMBOL_TYPE_BY_INSTANCE_CLASS[self.__class__]


@dataclass(repr=False)
class TaskInstance(SymbolInstance, Task):
    implementation: "CodeInstance" = required_field()


@dataclass(repr=False)
class TypeInstance(SymbolInstance, Type):
    py_type: Any = required_field()


@dataclass(repr=False)
class RecordInstance(Record):
    dataset: "DatasetInstance" = required_field()


@dataclass(repr=False)
class DatasetInstance(SymbolInstance, Dataset):
    pending_mutations: list[ModuleMutation] = field(default_factory=list)

    def __getitem__(self, item: int):
        return self.records[item]


@dataclass(repr=False)
class ModelInstance(SymbolInstance, Model):
    inference: "ModelInference" = required_field()


@dataclass(repr=False)
class CodeTransformation:
    original_code: str
    transformed_code: str
    method_name: str
    start_offset: int


@dataclass(repr=False)
class CodeInstance(SymbolInstance, Code):
    task: Optional[TaskInstance] = None
    transform: Optional[CodeTransformation] = None
    code_callable: SyncCodeCallable | AsyncCodeCallable = required_field()
    is_async: bool = required_field()
    tracer: Tracer = required_field()


class AsyncCodeInstance(CodeInstance):
    is_async = True

    async def __call__(self, *args, **kwargs):
        log = logger.bind(code=self.code, args=len(args), kwargs=describe_type(kwargs))
        try:
            self.tracer.code_enter(self, args, kwargs)
            log.debug("code.enter")
            result = await self.code_callable(*args, **kwargs)
            self.tracer.code_exit(self, args, kwargs, result)
            log.debug("code.exit", result=describe_type(result))
            return result
        except Exception as exception:
            self.tracer.code_exception(self, args, kwargs, exception)
            log.debug("code.exception", excinfo=True)
            raise


class SyncCodeInstance(CodeInstance):
    is_async = False

    def __call__(self, *args, **kwargs):
        log = logger.bind(code=self.code, args=len(args), kwargs=describe_type(kwargs))
        try:
            self.tracer.code_enter(self, args, kwargs)
            log.debug("code.enter")
            result = self.code_callable(*args, **kwargs)
            self.tracer.code_exit(self, args, kwargs, result)
            log.debug("code.exit", result=describe_type(result))
            return result
        except Exception as exception:
            self.tracer.code_exception(self, args, kwargs, exception)
            log.debug("code.exception", excinfo=True)
            raise


SYMBOL_TYPE_BY_INSTANCE_CLASS = {
    TaskInstance: SymbolType.TASK,
    TypeInstance: SymbolType.TYPE,
    DatasetInstance: SymbolType.DATA,
    ModelInstance: SymbolType.MODEL,
    CodeInstance: SymbolType.CODE,
}


# :RemoteObjectType
class ObjectStatus(enum.StrEnum):
    PREPARED = "prepared"
    UPLOADING = "uploading"
    AVAILABLE = "available"


@dataclass(repr=False)
class ObjectProxy:
    sha512: str
    content_length: int
    content_type: str
    name: str
    status: ObjectStatus

    @staticmethod
    def from_dict(value: dict):
        return ObjectProxy(
            sha512=value["sha512"],
            content_length=value["contentLength"],
            content_type=value["contentType"],
            name=value["name"],
            status=ObjectStatus[value["status"]],
        )

    def to_dict(self) -> dict:
        return {
            "sha512": self.sha512,
            "contentLength": self.content_length,
            "contentType": self.content_type,
            "name": self.name,
            "status": self.status.name,
        }


def _instantiate_py_value(value: Any, type: TypeNode) -> Any:
    if type.tag == TypeTag.FILE:
        try:
            return ObjectProxy.from_dict(value)
        except (ValueError, TypeError):
            return value
    return value


def instantiate_py_value(value: Any, type: TypeNode) -> Any:
    return map_value(value, type, map_v=_instantiate_py_value)


def instantiate_dataset(dataset: Dataset, build: Build) -> DatasetInstance:
    # map record data into proper python types
    records = []
    for record in dataset.records:
        py_record_data = {}
        # unkey into real names
        raw_data = dataset.type.unkey(record.data)
        for key, value in raw_data.items():
            py_field = to_pyidentifier(key)
            py_value = instantiate_py_value(value, dataset.type[key])
            py_record_data[py_field] = py_value
        records.append(Record(id=record.id, order_key=record.order_key, data=py_record_data))

    return DatasetInstance(**dict_minus(dataset.__dict__, "records"), records=records, build=build)


STATIC_BUILTINS = {
    # primitive type builtins
    "string": str,
    "text": str,
    "number": float,
    "file": ObjectProxy,
    "boolean": bool,
    "image": ObjectProxy,
    "audio": ObjectProxy,
    # library builtins
    "numpy": numpy,
    "asyncio": asyncio,
    # functional builtins
    "itertools": itertools,
    "more_itertools": itertools,
    "first": first,
    "last": last,
    "chain": itertools.chain,
}


def instantiate_code_callable(
    code: Code,
    context: OrderedDict[str, SymbolInstance],
) -> tuple[CodeTransformation, SyncCodeCallable | AsyncCodeCallable]:
    """
    Instantiates code into a Python callable in the context.
    If the code is a dynamic prompt (BPL), the callable will be wrapped and use the proxy for contexts.
    """
    # inline all possible context variables
    inlined_context = {to_pyidentifier(name): value for name, value in context.items()}
    source_context = (
        {
            "__statement__": code.source,
            "__file__": code.source.file,
            "__module__": code.source.file.module,
        }
        if code.source
        else {}
    )
    dynamic_context = {
        "source_context": context,
        "context": inlined_context,
        "_xblocks": code.xblocks,
        **inlined_context,
        "random": Random(code.id.hex.encode()),
        **source_context,
    }

    start_offset = 1  # for method signature
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

    # if we have xblocks, add line to copy them to top of method
    if code.xblocks:
        python_code = f"xblocks = [x.copy() for x in _xblocks]\n{python_code}"
        start_offset += 1

    # create python function from python code
    input_keys = [i.name for i in code.inputs]
    func_name = f"{to_pyidentifier(code.name)}_{code.id.hex[:6]}"
    async_str = "async " if is_async else ""
    func_params = ", ".join(to_pyidentifier(key) for key in input_keys)
    indented_code = textwrap.indent(python_code, " " * 4)
    code_str = f"{async_str}def {func_name}({func_params}):\n{indented_code}"
    try:
        callable = do_execute_arbitrary_code(code_str, locals)[func_name]
    except Exception as e:
        # shouldn't error unless it's a python parse issue since we're just defining a function
        raise ValueError("") from e

    transform = CodeTransformation(
        original_code=code,
        transformed_code=code_str,
        start_offset=start_offset,
        method_name=func_name,
    )
    return transform, callable


@dataclass
class ModelInferenceImpl(ModelInference):
    ctx: InferenceContext


def instantiate_model_inference(model: Model) -> ModelInference:
    ctx = InferenceContext(model=model, user_opaque_id=model.id.hex, streaming_callback=None)
    impl = ModelInferenceImpl(ctx)
    endpoints = list(get_endpoints(model))
    if not endpoints:
        raise RuntimeError(f"no endpoints found for model: {model}")
    for modality, endpoint_cls in endpoints:
        endpoint = getattr(endpoint_cls(ctx), modality)
        setattr(impl, modality, endpoint)
    return impl


DEFAULT_TRACER = MultiTracer([ExecutionTracer(PubExecutionTracker()), ValidationTracer()])
DEFAULT_PROXY = Proxy(
    tracer=DEFAULT_TRACER,
    cache_inferences=True,
    inference_timeout=15,
    inference_retries=2,
)


def instantiate(
    symbol: InterpSymbol,
    build: Optional[Build] = None,
    buildmap: Optional["BuildMap"] = None,
    refmap: dict[UUID, SymbolInstance] = None,
    proxy: Proxy | None = None,
) -> InterpSymbol:
    """Instantiate a symbol in a build with all relevant context recursively."""
    if symbol.abstract:
        raise ValueError(f"cannot instantiate abstract symbol: {symbol}")
    buildmap = buildmap or (lambda s: None)
    refmap = refmap or {}
    proxy = proxy or DEFAULT_PROXY
    # instantiate context (preserving order)
    instantiated_context = OrderedDict()
    for name, value in symbol.context.items():
        if symbol is value:
            # self-reference is not supported for now
            # mainly because it would require either
            #  1) allowing invalid/mock initial instance state (and populate that later)
            #  2) tracking and somehow swapping the reference after it is actually created
            continue
        instantiated_context[name] = instantiate(
            value, build=build, refmap=refmap, buildmap=buildmap, proxy=proxy
        )

    if isinstance(symbol, Task):
        if buildmap is None:
            raise ValueError(f"cannot instantiate task without build: {symbol}")
        implementation = buildmap(symbol)
        if implementation is None:
            raise ValueError(f"cannot instantiate task in {build} without target: {symbol}")
        implementation_instance = instantiate(
            implementation, build=build, buildmap=buildmap, proxy=proxy
        )
        task = TaskInstance(
            **symbol.__dict__,
            build=build,
            implementation=typing.cast(CodeInstance, implementation_instance),
        )
        implementation_instance.task = task
        return task
    elif isinstance(symbol, Code):
        transform, code_callable = instantiate_code_callable(symbol, instantiated_context)
        code_instance = CodeInstance(
            **symbol.__dict__,
            build=build,
            transform=transform,
            code_callable=code_callable,
            is_async=inspect.iscoroutinefunction(code_callable),
        )
        # TODO @Broken: set task on code instance if instantiated directly
        #  Likely will require breaking circles with a refmap.
        return proxy.proxy_code(code_instance)
    elif isinstance(symbol, Model):
        inference = instantiate_model_inference(symbol)
        model_instance = ModelInstance(**symbol.__dict__, inference=inference, build=build)
        return proxy.proxy_model(model_instance)
    elif isinstance(symbol, Dataset):
        return instantiate_dataset(symbol, build)
    elif isinstance(symbol, Type):
        py_type = instantiate_py_type(symbol)
        return TypeInstance(**symbol.__dict__, build=build, py_type=py_type)
    else:
        raise ValueError(f"cannot instantiate {symbol} in {build}")


BuildMap = Callable[[InterpSymbol], Optional[InterpSymbol]]
