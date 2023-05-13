#
# Instances
#
import asyncio
import contextvars
import enum
import itertools
import textwrap
import typing
from concurrent.futures import ThreadPoolExecutor
from dataclasses import dataclass
from datetime import datetime
from random import Random
from typing import Any, Callable, Coroutine, Optional
from uuid import UUID, uuid4

import aiohttp
import numpy
import structlog
from asgiref.sync import async_to_sync, sync_to_async
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
from bench.msg import NMessageType
from bench.msg.core import NMessage, request
from bench.msg.messages import RepReadObjectPayload, ReqReadObjectPayload
from bench.runtime.build import build_task_implementation
from bench.runtime.inference import InferenceProxy, ModelInference
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
        default_build: Build = None,
        cache_inferences: bool = True,
        tracer: Tracer = DEFAULT_TRACER,
        inference_timeout: int = 20,
        inference_retries: int = 3,
        mode: SessionMode = SessionMode.READ_ONLY,
        write: Callable[[list[ModuleMutation]], typing.Awaitable[bool]] = None,
        executor: ThreadPoolExecutor = None,
    ):
        if mode != SessionMode.READ_ONLY and write is None:
            raise ValueError("write must be provided for non-readonly sessions")
        self.id = uuid4()
        self.idx = idx
        self.module = idx.module
        self.instances: dict[UUID, SymbolInstance] = {
            symbol.id: symbol for symbol in (instances or [])
        }
        self.default_build = default_build
        self.tracer = tracer
        self.cache_inferences = cache_inferences
        self.inference_timeout = inference_timeout
        self.inference_retries = inference_retries
        self.mode = mode
        self.write = write
        self.executor = executor or ThreadPoolExecutor(max_workers=1)

        self.mutator = ModuleMutator(idx)
        self.opened_at: Optional[datetime] = None
        self.closed_at: Optional[datetime] = None
        self._cached_implementations: dict[Any, AsyncCodeInstance] = {}

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

    async def prepare(self):
        """Prepares instances in the session for execution."""
        logger.debug("session.prepare", session=self)
        # prepare default implementations for tasks
        if self.default_build is not None:
            for instance in list(self.instances.values()):  # copy to avoid concurrent modification
                if isinstance(instance, TaskInstance):
                    await self.get_implementation(instance, build=self.default_build)

    async def get_implementation(
        self, task: "TaskInstance", build: Build | str = None, model: Model | str = None
    ) -> "AsyncCodeInstance":
        """Gets or builds an implementation for a task."""
        if build is not None:
            if isinstance(build, str):
                build = self.idx.symbol(build, symbol_t=Build)
        elif model is not None:
            if isinstance(model, str):
                model = self.idx.symbol(model, symbol_t=Model)
            # TODO @Feature: find or make build for model
            raise NotImplementedError("model key for task implementation not yet supported")
        else:
            if self.default_build is None:
                raise RuntimeError(f"no build specified for {task} (no default in {self})")
            build = self.default_build
        cache_key = (task.id, build.id)
        if cache_key not in self._cached_implementations:
            implementation = await build_task_implementation(task, build.models[0])
            self._cached_implementations[cache_key] = instantiate(implementation, session=self)
        return self._cached_implementations[cache_key]

    def open(self):
        """Opens the session to access and modification."""
        if self.opened_at is not None:
            raise RuntimeError(f"session already opened {self}")
        self.opened_at = datetime.now()
        active_session.set(self)
        logger.debug("session.open", session=self)

    async def aflush(self):
        """Flushes all module mutations to the underlying store."""
        if not self.mutator.mutations:
            return
        if self.mode == SessionMode.READ_ONLY:
            raise RuntimeError(f"cannot mutate read-only session {self}")
        logger.debug("session.flush", session=self, mutator=self.mutator)
        success = await self.write(self.mutator.bundle().collapse())
        if not success:
            raise RuntimeError(f"failed to write mutations {self.mutator.mutations}")
        self.mutator.reset()
        logger.debug("session.flush.done", session=self)

    async def aclose(self):
        """Closes the session, flushing any mutations and preventing further access."""
        if self.closed_at is not None:
            raise RuntimeError(f"session already closed {self}")
        self.closed_at = datetime.now()
        await self.aflush()
        active_session.set(None)
        logger.debug("session.close", session=self)


AsyncCodeCallable = Callable[..., Coroutine]
SyncCodeCallable = Callable[..., Any]


@dataclass(repr=False)
class SymbolInstance:
    id: UUID = required_field()
    session: Session = None
    mode: Optional[SessionMode] = None

    def __post_init__(self):
        if self.session is None:
            self.session = active_session.get()
            if self.session is None:
                raise RuntimeError(f"no active session for {self}")
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
    # TODO @Performance: mark & collect dirty on session flush for records/datasets
    #  Currently we just write the whole record on any change, which is ughh.
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
        self.session.mut.truncate_records(self.id)
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
class ModelInstance(SymbolInstance, Model):
    inference: "ModelInference" = required_field()

    # forward inference methods
    def __getattr__(self, item: str):
        if item in self.inference.__dict__:
            return getattr(self.inference, item)
        else:
            raise AttributeError(item)


@dataclass(repr=False)
class TaskInstance(SymbolInstance, Task):
    is_async = True

    async def __call__(self, *args, build: Build | str = None, model: Model | str = None, **kwargs):
        implementation = await self.session.get_implementation(self, build=build, model=model)
        return await implementation(*args, **kwargs)

    def to_sync(self) -> "SyncTaskInstance":
        return SyncTaskInstance(**dict_minus(self.__dict__, ["is_async"]))


@dataclass(repr=False)
class SyncTaskInstance(TaskInstance):
    is_async = False

    def __call__(self, *args, build: Build | str = None, model: Model | str = None, **kwargs):
        return async_to_sync(super().__call__)(*args, build=build, model=model, **kwargs)


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
    tracer: Tracer = required_field()


@dataclass(repr=False)
class AsyncCodeInstance(CodeInstance):
    is_async = True

    async def __call__(self, *args, **kwargs):
        log = logger.bind(code=self, args=len(args), kwargs=describe_type(kwargs))
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

    def to_sync(self) -> "SyncCodeInstance":
        return SyncCodeInstance(
            **dict_minus(self.__dict__, "code_callable"),
            code_callable=async_to_sync(self.code_callable),
        )


@dataclass(repr=False)
class SyncCodeInstance(CodeInstance):
    is_async = False

    def __call__(self, *args, **kwargs):
        log = logger.bind(code=self, args=len(args), kwargs=describe_type(kwargs))
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

    def to_async(self) -> "AsyncCodeInstance":
        return AsyncCodeInstance(
            **dict_minus(self.__dict__, "code_callable"),
            code_callable=sync_to_async(
                self.code_callable, thread_sensitive=False, executor=self.session.executor
            ),
        )


SYMBOL_TYPE_BY_INSTANCE_CLASS = {
    TaskInstance: SymbolType.TASK,
    SyncTaskInstance: SymbolType.TASK,
    TypeInstance: SymbolType.TYPE,
    DatasetInstance: SymbolType.DATA,
    ModelInstance: SymbolType.MODEL,
    CodeInstance: SymbolType.CODE,
    SyncCodeInstance: SymbolType.CODE,
    AsyncCodeInstance: SymbolType.CODE,
}


@dataclass(repr=False, slots=True)
class RemoteObjectInstance(RemoteObject):
    """A proxy to a remotely stored object behaving like a Python file on demand."""

    def __getitem__(self, item):
        return self.to_dict()[item]

    async def aread(self, timeout: float = 1) -> bytes:
        """Read the object from the remote storage."""
        if self.status != RemoteObjectStatus.AVAILABLE:
            raise ValueError(f"unable to read {self}")
        # get GET url to access file
        rep: NMessage[RepReadObjectPayload] = await request(
            NMessageType.REQUEST_READ_OBJECT,
            ReqReadObjectPayload(objects=[wire.rmap_remote_object(self)]),
            reply_t=RepReadObjectPayload,
            timeout=timeout,
        )
        get_url = rep.p.get_urls[0]
        if get_url is None:
            raise ValueError(f"unable to get {self}")
        # download file from url
        async with aiohttp.ClientSession() as session:
            async with session.get(get_url) as response:
                if response.status != 200:
                    raise ValueError(f"unable to download {self}")
                return await response.read()

    async def areadtext(self) -> str:
        return (await self.aread()).decode()

    async def areadlines(self) -> list[str]:
        return (await self.aread()).decode().splitlines()

    def read(self, timeout: float = 1) -> bytes:
        """Read the object from the remote storage."""
        return async_to_sync(self.aread)(timeout=timeout)

    def readtext(self) -> str:
        return self.read().decode()

    def readlines(self) -> list[str]:
        return self.read().decode().splitlines()

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


def instantiate_type(type: Type, session: Session) -> TypeInstance:
    """Instrument and instantiate a type for use."""
    py_type = instantiate_py_type(type)
    return TypeInstance(**type.__dict__, py_type=py_type, session=session)


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


def instantiate_dataset(dataset: Dataset, session: Session) -> DatasetInstance:
    """Instrument and instantiate a dataset for use."""
    instance = DatasetInstance(
        **dict_minus(dataset.__dict__, "records"), records=[], session=session
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

DYNAMIC_BUILTINS = {
    "session",
    "context",
    "_xblocks",
    "random",
}


def instantiate_code(code: Code, session: Session) -> SyncCodeInstance | AsyncCodeInstance:
    """Instantiates code into a Python callable in the context of the session."""

    # instantiate context (preserving scoping)
    symbol_context: dict[str, InterpSymbol] = {
        name: instantiate(child, session=session) for name, child in code.context.items()
    }

    if not code.parse.is_async:
        # replace any async functions with sync versions
        for key, symbol in symbol_context.items():
            if isinstance(symbol, (CodeInstance, TaskInstance)) and symbol.is_async:
                symbol_context[key] = symbol.to_sync()

    dynamic_context = {
        "session": session,
        "context": {symbol.name: symbol for symbol in symbol_context.values()},  # by name
        "_xblocks": code.xblocks,
        **symbol_context,  # inlined
        "random": Random(code.id.hex.encode()),
    }

    start_offset = 1  # for method signature
    if code.language == "python":
        python_code = code.code or "pass"
        locals = {**STATIC_BUILTINS, **dynamic_context}
    elif code.language == "x":
        python_code = code.code or "pass"
        locals = {**STATIC_BUILTINS, **X_BUILTINS, **dynamic_context}
    else:
        raise ValueError(f"unknown code language: {code}")

    # if we have xblocks, add line to copy them to top of method
    if code.xblocks:
        python_code = f"xblocks = [x.copy() for x in _xblocks]\n{python_code}"
        start_offset += 1

    # stub fake lines
    python_code_lines = python_code.splitlines()
    for i in code.parse.fake_line_numbers:
        python_code_lines[i] = "pass # " + python_code_lines[i]
    python_code = "\n".join(python_code_lines)

    # create python function from python code
    input_keys = [i.name for i in code.inputs]
    func_name = f"{to_pyidentifier(code.name)}_{code.id.hex[:6]}"
    async_str = "async " if code.parse.is_async else ""
    func_params = ", ".join(to_pyidentifier(key) for key in input_keys)
    indented_code = textwrap.indent(python_code, " " * 4)
    try:
        method_str = f"{async_str}def {func_name}({func_params}):\n{indented_code}"
        callable = do_execute_arbitrary_code(method_str, locals)[func_name]
    except SyntaxError as e:
        # raise error in code when called for proper reporting
        raise_str = f"raise {e.__class__.__name__}('invalid syntax: ' + {e.args[1][3]!r})"
        indented_raise = textwrap.indent(raise_str, " " * 4)
        method_str = f"{async_str}def {func_name}({func_params}):\n{indented_raise}"
        callable = do_execute_arbitrary_code(method_str, locals)[func_name]

    transform = CodeTransformation(
        original_code=code.code,
        transformed_code=method_str,
        start_offset=start_offset,
        method_name=func_name,
    )
    code_cls = AsyncCodeInstance if code.parse.is_async else SyncCodeInstance
    return code_cls(
        **code.__dict__,
        transform=transform,
        code_callable=callable,
        tracer=session.tracer,
        session=session,
    )


def instantiate_model(model: Model, session: Session) -> ModelInstance:
    """Instantiates the model inference endpoints for the session."""
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
    return ModelInstance(**model.__dict__, inference=inference, session=session)


def instantiate(symbol: InterpSymbol, session: Session) -> SymbolInstance:
    """Instantiate a symbol in a session (incl. any references recursively)."""
    if symbol.id in session.instances:
        return session.instances[symbol.id]
    if symbol.abstract:
        raise ValueError(f"cannot instantiate abstract symbol: {symbol}")
    if isinstance(symbol, Task):
        return TaskInstance(**symbol.__dict__, session=session)
    elif isinstance(symbol, Code):
        return instantiate_code(symbol, session)
    elif isinstance(symbol, Model):
        return instantiate_model(symbol, session)
    elif isinstance(symbol, Dataset):
        return instantiate_dataset(symbol, session)
    elif isinstance(symbol, Type):
        return instantiate_type(symbol, session)
    else:
        raise ValueError(f"cannot instantiate {symbol} in {session}")


BuildMap = Callable[[InterpSymbol], Optional[InterpSymbol]]
