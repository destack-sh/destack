#
# Instances
#
import asyncio
import contextvars
import enum
import itertools
import textwrap
import typing
from collections import OrderedDict
from dataclasses import dataclass
from datetime import datetime
from random import Random
from typing import Any, Callable, Coroutine, Optional
from uuid import UUID, uuid4

import numpy
import structlog
from more_itertools import first, last

from bench.language import (
    Build,
    Code,
    Dataset,
    Model,
    ModuleIndex,
    Record,
    SymbolType,
    Task,
    wire,
)
from bench.language.mutate import ModuleMutation, ModuleMutator
from bench.language.type import (
    InterpSymbol,
    LiteralValue,
    RemoteObject,
    RemoteObjectStatus,
    Type,
    TypeNode,
    TypeTag,
)
from bench.language.typer import check_type, map_value, rekey_value
from bench.runtime.inference import InferenceProxy, Modality, ModelInference
from bench.runtime.model import get_endpoints
from bench.runtime.proxy import proxy_value, unproxy_value
from bench.runtime.tracing import (
    ExecutionTracer,
    MultiTracer,
    PubExecutionTracker,
    Tracer,
    ValidationTracer,
)
from bench.runtime.unsecure import do_execute_arbitrary_code
from bench.runtime.x import X_BUILTINS
from bench.utils.fractional import INTEGER_ZERO, generate_key_between, generate_n_keys_between
from bench.utils.func import describe_type, dict_minus
from bench.utils.utils import required_field, to_pyidentifier

logger = structlog.get_logger(__name__)

DEFAULT_TRACER = MultiTracer([ExecutionTracer(PubExecutionTracker()), ValidationTracer()])


class SessionMode(enum.StrEnum):
    READ_ONLY = "ro"
    WRITE_LOCAL = "wl"
    WRITE_GLOBAL = "w"
    WRITE_ONLY = "wo"


active_session: contextvars.ContextVar[Optional["Session"]] = contextvars.ContextVar(
    "active_session", default=None
)


class Session:
    """A managed context for running code in a module (may mutate)."""

    def __init__(
        self,
        idx: ModuleIndex,
        instances: list["SymbolInstance"] = None,
        cache_inferences: bool = True,
        tracer: Tracer = DEFAULT_TRACER,
        inference_timeout: int = 20,
        inference_retries: int = 3,
        mode: SessionMode = SessionMode.READ_ONLY,
        write: Callable[[list[ModuleMutation]], typing.Awaitable[bool]] = None,
    ):
        if mode != SessionMode.READ_ONLY and write is None:
            raise ValueError("write must be provided for non-readonly sessions")
        self.id = uuid4()
        self.idx = idx
        self.module = idx.module
        self.opened_at: Optional[datetime] = None
        self.closed_at: Optional[datetime] = None
        self.tracer = tracer
        self.cache_inferences = cache_inferences
        self.inference_timeout = inference_timeout
        self.inference_retries = inference_retries
        self.mode = mode
        self.write = write
        self.mutator = ModuleMutator(idx)
        self.instances: dict[UUID, SymbolInstance] = {
            symbol.id: symbol for symbol in (instances or [])
        }

    def __str__(self):
        status = "open" if self.opened_at else ("closed" if self.closed_at else "pending")
        return (
            f"{self.module.name} {self.id} ({self.mode}, {status}, {len(self.mutator.mutations)})"
        )

    def __repr__(self):
        return f"<Session {self}>"

    @property
    def mut(self) -> ModuleMutator:
        return self.mutator

    @property
    def is_open(self) -> bool:
        return self.opened_at is not None and self.closed_at is None

    def add(self, symbol: "SymbolInstance", new: bool) -> None:
        self.instances[symbol.id] = symbol
        if new:
            raise NotImplementedError("can't handle in-session create yet")

    def remove(self, symbol: "SymbolInstance") -> None:
        if symbol.id in self.instances:
            del self.instances[symbol.id]
            # TODO @Feature @Robustness: track delete / remove relevant mutations (if open)

    def check_can_write(self, symbol: InterpSymbol):
        if not self.can_write(symbol):
            raise RuntimeError(f"cannot write {symbol} in {self}")

    def can_write(self, symbol: InterpSymbol):
        if self.mode == SessionMode.READ_ONLY:
            return False
        elif self.mode == SessionMode.WRITE_LOCAL:
            return symbol.is_local
        elif self.mode == SessionMode.WRITE_GLOBAL:
            return True
        else:
            raise RuntimeError(f"unknown session mode {self.mode}")

    def open(self):
        if self.opened_at is not None:
            raise RuntimeError(f"session already opened {self}")
        self.opened_at = datetime.now()
        active_session.set(self)

    async def aflush(self):
        if not self.mutator.mutations:
            return
        if self.mode == SessionMode.READ_ONLY:
            raise RuntimeError(f"cannot mutate read-only session {self}")
        success = await self.write(self.mutator.bundle().collapse())
        if not success:
            raise RuntimeError(f"failed to write mutations {self.mutator.mutations}")
        self.mutator.reset()

    async def aclose(self):
        if self.closed_at is not None:
            raise RuntimeError(f"session already closed {self}")
        self.closed_at = datetime.now()
        await self.aflush()


AsyncCodeCallable = Callable[..., Coroutine]
SyncCodeCallable = Callable[..., Any]


@dataclass(repr=False)
class SymbolInstance:
    id: UUID = required_field()
    session: Session = None
    build: Optional[Build] = None
    mode: Optional[SessionMode] = None

    def __post_init__(self):
        if self.session is None:
            self.session = active_session.get()
            # we pass in session on instantiate, so this must be new
            self.session.add(self, new=True)
        else:
            self.session.add(self, new=False)

    def __del__(self):
        if self.session is not None:
            self.session.remove(self)

    def in_mode(self, mode: SessionMode | str) -> "SymbolInstance":
        """Applies a read/write mode to _this_ symbol instance."""
        if self.mode is not None and self.mode != mode:
            raise RuntimeError(f"symbol {self} already in mode {self.mode}")
        if isinstance(mode, str):
            mode = SessionMode(mode)
        self.mode = mode
        return self

    @property
    def symbol_type(self):
        return SYMBOL_TYPE_BY_INSTANCE_CLASS[self.__class__]


@dataclass(repr=False)
class TaskInstance(SymbolInstance, Task):
    implementation: "CodeInstance" = required_field()


@dataclass(repr=False)
class TypeInstance(SymbolInstance, Type):
    py_type: Any = required_field()

    # mimic python type behavior
    def __instancecheck__(self, instance):
        return isinstance(instance, self.py_type)

    def __subclasscheck__(self, subclass):
        return issubclass(subclass, self.py_type)

    def __call__(self, *args, **kwargs):
        return self.py_type(*args, **kwargs)


@dataclass(repr=False)
class RecordInstance(Record):
    dataset: "DatasetInstance" = required_field()

    def __post_init__(self):
        self.data = proxy_value(
            self.data,
            onread=lambda k: None,
            onwrite=lambda k: self._notify_update(k),
        )

    def _to_wire(self, include_data: bool = True) -> wire.RecordData:
        if include_data:
            raw_data = strip_value(self.data, self.type)
            raw_data = rekey_value(raw_data, self.type)
        else:
            raw_data = None
        return wire.RecordData(
            id=self.id,
            revision=1,
            order_key=self.order_key,
            statement_id=self.dataset.id,
            data=raw_data,
        )

    def _notify_update(self, key: Optional[str]):
        if key is None or key == "":
            check_type(self.data, self.type)
        elif key not in self.type:
            raise ValueError(f"'{key}' not present in {self.type}")
        else:
            check_type(self.data.get(key), self.type[key])
        self.dataset.session.check_can_write(self.dataset)
        self.dataset.session.mut.update(self._to_wire())

    @property
    def type(self):
        return self.dataset.type


@dataclass(repr=False)
class DatasetInstance(SymbolInstance, Dataset):
    def clear(self):
        self.session.check_can_write(self)
        # should really be truncate operation
        for record in self.records:
            self.session.mut.delete(record._to_wire(include_data=False))
        self.records = []

    def append(self, record: Record = None, **data):
        if record is not None:
            if data:
                raise ValueError("cannot pass both record and data")
            data = record.data
        data = unproxy_value(data)  # remove source proxy if any
        check_type(data, self.type)
        self.session.check_can_write(self)
        # insert
        last_ok = self.records[-1].order_key if self.records else INTEGER_ZERO
        record = RecordInstance(
            id=uuid4(),
            dataset=self,
            order_key=generate_key_between(last_ok, None),
            data=data,
        )
        self.records.append(record)
        self.session.mut.create(record._to_wire())

    def extend(self, records: typing.Iterable[Record | dict]):
        datas = [  # remove source proxy if any
            unproxy_value(record.data) if isinstance(record, Record) else unproxy_value(record)
            for record in records
        ]
        for data in datas:
            check_type(data, self.type)
        self.session.check_can_write(self)
        # insert
        last_ok = self.records[-1].order_key if self.records else INTEGER_ZERO
        oks = generate_n_keys_between(last_ok, None, len(datas))
        records = [
            RecordInstance(
                id=uuid4(),
                dataset=self,
                order_key=ok,
                data=data,
            )
            for ok, data in zip(oks, datas)
        ]
        self.records.extend(records)
        self.session.mut.create_many(*(record._to_wire() for record in records))

    def __iter__(self):
        return iter(self.records)


@dataclass(repr=False)
class ModelInstance(SymbolInstance, Model, ModelInference):
    inference: "ModelInference" = required_field()

    # forward inference methods
    def __getattr__(self, item):
        if item in Modality:
            return getattr(self.inference, item)
        else:
            raise AttributeError(item)


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
    SyncCodeInstance: SymbolType.CODE,
    AsyncCodeInstance: SymbolType.CODE,
}


@dataclass(repr=False, slots=True)
class RemoteObjectInstance(RemoteObject):
    def __getitem__(self, item):
        return self.to_dict()[item]

    @staticmethod
    def from_dict(value: dict):
        return RemoteObjectInstance(
            id=UUID(value["id"]),
            sha512=value["sha512"],
            content_length=value["contentLength"],
            content_type=value["contentType"],
            name=value["name"],
            status=RemoteObjectStatus[value["status"]],
        )

    def to_dict(self) -> dict:
        return {
            "id": str(self.id),
            "sha512": self.sha512,
            "contentLength": self.content_length,
            "contentType": self.content_type,
            "name": self.name,
            "status": self.status.name,
        }


def instantiate_py_type(node: TypeNode) -> type | LiteralValue:
    """Create the Python-native type for the given type node."""
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
        return RemoteObject
    elif node.tag == TypeTag.AUDIO:
        return RemoteObject
    elif node.tag == TypeTag.FILE:
        return RemoteObject
    elif node.tag == TypeTag.EMBEDDING:
        return numpy.ndarray
    elif node.tag == TypeTag.UNION:
        return typing.Union[tuple(instantiate_py_type(child) for child in node.type_nodes)]
    elif node.tag == TypeTag.STRUCT:
        return typing.TypedDict(
            node.name,
            {node.name: instantiate_py_type(node) for node in node.type_nodes},
        )
    elif node.tag == TypeTag.ENUM:
        # create 'fake' enum with the given constants pointing to themselves
        # assumes enums are value enums (not type union enums)
        members = {to_pyidentifier(child.name): child.name for child in node.type_nodes}
        enum_name = node.name or "_anon_" + uuid4().hex
        return enum.StrEnum(enum_name, members)
    elif node.tag == TypeTag.LITERAL:
        return node.value
    elif node.tag == TypeTag.ANY:
        return Any
    else:
        raise ValueError(f"unexpected type node: {node}")


def instantiate_type(type: Type, build: Build, session: Session) -> TypeInstance:
    """Instrument and instantiate a type for use."""
    py_type = instantiate_py_type(type)
    return TypeInstance(**type.__dict__, build=build, py_type=py_type, session=session)


def _instantiate_py_value_inner(value: Any, type: TypeInstance) -> Any:
    if type.tag == TypeTag.FILE:
        try:
            return RemoteObjectInstance.from_dict(value)
        except (KeyError, ValueError, TypeError):
            return value
    elif type.tag == TypeTag.STRUCT:
        return type(**value)
    return value


def _strip_py_value_inner(value: Any, type: TypeInstance) -> Any:
    if type.tag == TypeTag.FILE:
        try:
            return value.to_dict()
        except (KeyError, ValueError, TypeError):
            return None  # raise? (but should be type error earlier}
    return value


def instantiate_py_value(value, type: TypeInstance) -> Any:
    return map_value(value, type, map_v=_instantiate_py_value_inner)


def strip_value(value, type: TypeNode) -> Any:
    return map_value(value, type, map_v=_strip_py_value_inner)


# TODO @Feature: what's the counter-part to instantiate record data?


def instantiate_dataset(dataset: Dataset, build: Build, session: Session) -> DatasetInstance:
    """Instrument and instantiate a dataset for use."""
    instance = DatasetInstance(
        **dict_minus(dataset.__dict__, "records"), records=[], build=build, session=session
    )
    for raw_record in dataset.records:
        py_record_data = {}
        # unkey into real names
        raw_data = dataset.type.unkey(raw_record.data)
        for key, value in raw_data.items():
            py_field = to_pyidentifier(key)
            py_value = instantiate_py_value(value, dataset.type[key])
            py_record_data[py_field] = py_value
        record = RecordInstance(
            id=raw_record.id, order_key=raw_record.order_key, data=py_record_data, dataset=instance
        )
        instance.records.append(record)
    return instance


STATIC_BUILTINS = {
    # primitive type builtins
    "string": str,
    "text": str,
    "number": float,
    "file": RemoteObject,
    "boolean": bool,
    "image": RemoteObject,
    "audio": RemoteObject,
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


def instantiate_code(
    code: Code,
    context: OrderedDict[str, SymbolInstance],
    build: Optional[Build],
    session: Session,
) -> CodeInstance:
    """
    Instantiates code into a Python callable in the context.
    If the code is a dynamic prompt (BPL), the callable will be wrapped and use the session for contexts.
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
        "session": session,
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
    code_cls = AsyncCodeInstance if is_async else SyncCodeInstance
    return code_cls(
        **code.__dict__,
        build=build,
        transform=transform,
        code_callable=callable,
        is_async=is_async,
        tracer=session.tracer,
        session=session,
    )


def instantiate_model(model: Model, session: Session) -> ModelInstance:
    inference = ModelInference()
    endpoints = list(get_endpoints(model))
    if not endpoints:
        raise RuntimeError(f"no endpoints found for model: {model}")
    for modality, endpoint_cls in endpoints:
        endpoint = getattr(endpoint_cls(model), modality)
        endpoint_proxy = InferenceProxy(
            model=model,
            modality=modality,
            endpoint=endpoint,
            tracer=session.tracer,
            cache_inferences=session.cache_inferences,
            timeout=session.inference_timeout,
            retries=session.inference_retries,
        )
        setattr(inference, modality, endpoint_proxy)
    return ModelInstance(**model.__dict__, inference=inference, build=None, session=session)


def instantiate(
    symbol: InterpSymbol,
    session: Session,
    build: Optional[Build] = None,
    buildmap: Optional["BuildMap"] = None,
) -> SymbolInstance:
    """Instantiate a symbol in a build recursively."""
    if symbol.abstract:
        raise ValueError(f"cannot instantiate abstract symbol: {symbol}")
    buildmap = buildmap or (lambda s: None)
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
            value, build=build, buildmap=buildmap, session=session
        )

    if isinstance(symbol, Task):
        if buildmap is None:
            raise ValueError(f"cannot instantiate task without build: {symbol}")
        implementation = buildmap(symbol)
        if implementation is None:
            raise ValueError(f"cannot instantiate task in {build} without target: {symbol}")
        implementation_instance = instantiate(
            implementation, build=build, buildmap=buildmap, session=session
        )
        task = TaskInstance(
            **symbol.__dict__,
            build=build,
            implementation=typing.cast(CodeInstance, implementation_instance),
        )
        implementation_instance.task = task
        return task
    elif isinstance(symbol, Code):
        return instantiate_code(symbol, instantiated_context, build, session)
    elif isinstance(symbol, Model):
        return instantiate_model(symbol, session)
    elif isinstance(symbol, Dataset):
        return instantiate_dataset(symbol, build, session)
    elif isinstance(symbol, Type):
        return instantiate_type(symbol, build, session)
    else:
        raise ValueError(f"cannot instantiate {symbol} in {build}")


BuildMap = Callable[[InterpSymbol], Optional[InterpSymbol]]
