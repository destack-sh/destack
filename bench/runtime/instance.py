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
from dataclasses import dataclass, fields
from datetime import date, datetime, time
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
    Data,
    Model,
    ModuleIndex,
    Record,
    SymbolType,
    Task,
    wire,
)
from bench.language.mutate import ModuleMutation, ModuleMutator
from bench.language.type import (
    TYPE_TAG_BY_TYPE_HINT,
    InterpSymbol,
    LiteralValue,
    RemoteObject,
    RemoteObjectStatus,
    Secret,
    Type,
    TypeFlag,
    TypeHint,
    TypeNode,
    TypeTag,
)
from bench.language.typer import check_type, map_rekey_enum, map_unkey_enum, map_value
from bench.msg import NMessageType
from bench.msg.core import NMessage, request
from bench.msg.messages import (
    RepReadObjectPayload,
    RepReadSecretPayload,
    ReqReadObjectPayload,
    ReqReadSecretPayload,
)
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

    def __getattr__(self, item):
        if item in self:
            return self[item]
        raise AttributeError(f"{self} has no attribute {item}")


@dataclass(repr=False)
class RecordInstance(Record):
    # TODO @Performance: mark & collect dirty on session flush for records/datasets
    #  Currently we just write the whole record on any change, which is ughh.
    # :RecordInstanceFieldKeys
    type: TypeInstance = required_field()
    session: Session = required_field()
    owner: InterpSymbol = required_field()

    def __post_init__(self):
        self.data = proxy_value(
            self.data,
            onread=lambda k: None,
            onwrite=lambda k: self._notify_update(k),
        )

    def _to_wire(self, include_data: bool = True) -> wire.RecordData:
        if include_data:
            raw_data = map_value(
                self.data, self.type, map_v=strip_py_value_flat, map_k=lambda t: (t.ident, t.key)
            )
        else:
            raw_data = None
        return wire.RecordData(
            id=self.id,
            revision=1,
            order_key=self.order_key,
            statement_id=self.owner.id,
            data=raw_data,
        )

    def _notify_update(self, key: Optional[str]):
        if key is None or key == "":
            check_type(self.data, self.type)
        elif key not in self.type:
            raise ValueError(f"'{key}' not present in {self.type}")
        else:
            check_type(self.data.get(key), self.type[key])
        self.session.check_can_write(self.owner)
        self.session.mut.update(self._to_wire())


@dataclass(repr=False)
class DataTableInstance(SymbolInstance, Data):
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
            session=self.session,
            type=self.type,
            owner=self,
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

    def __getitem__(self, item: int | slice) -> RecordInstance | list[RecordInstance]:
        return self.records[item]

    def __iter__(self):
        return iter(self.records)


@dataclass(repr=False)
class DataValueInstance(SymbolInstance, Data):
    # imitate/proxy record instance

    @property
    def keys(self):
        return self.records[0].data.keys()

    def __getitem__(self, item):
        return self.records[0].data[item]

    def __setitem__(self, key, value):
        self.records[0].data[key] = value

    # proxy to record data if not in this class

    def __getattr__(self, item):
        if item in self.__dict__:
            return self.__dict__[item]
        elif item in self.records[0].data:
            return self.records[0][item]
        else:
            raise AttributeError(item)

    def __setattr__(self, key, value):
        if key in DATA_VALUE_INSTANCE_FIELDS_KEYS:
            super().__setattr__(key, value)
        else:
            setattr(self.records[0], key, value)


DATA_VALUE_INSTANCE_FIELDS_KEYS = {field.name for field in fields(DataValueInstance)}


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
    DataTableInstance: SymbolType.DATA,
    DataValueInstance: SymbolType.DATA,
    ModelInstance: SymbolType.MODEL,
    CodeInstance: SymbolType.CODE,
    SyncCodeInstance: SymbolType.CODE,
    AsyncCodeInstance: SymbolType.CODE,
}


@dataclass(repr=False, slots=True)
class RemoteObjectInstance(RemoteObject):
    """A proxy to a remotely stored object behaving like a Python file on demand."""

    def __getitem__(self, item):
        return self.__dict__[item]

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


@dataclass(repr=False, slots=True)
class SecretInstance(Secret):
    """A proxy to a remotely stored secret."""

    async def areveal(self) -> Any:
        if self.value is not None:
            return self.value
        rep: NMessage[RepReadSecretPayload] = await request(
            NMessageType.REQUEST_READ_SECRET,
            ReqReadSecretPayload(secrets=[wire.rmap_secret(self)]),
            reply_t=RepReadSecretPayload,
        )
        self.value = rep.p.secrets[0].value
        return self.value

    def reveal(self) -> Any:
        return async_to_sync(self.areveal)()


TYPENAME_SENTINEL = "__typename"  # :TypeSentinel
REMOTE_OBJECT_TYPENAME = "RemoteObject"
SECRET_TYPENAME = "Secret"
TypeSignature = typing.NamedTuple(
    "TypeSignature", [("tag", TypeTag), ("hint", Optional[TypeHint]), ("flags", TypeFlag)]
)


class TypeMapping:
    """
    Maps specific types (and values) into and from Python.
    Don't bother with lists and optional types here.
    """

    def to_py_type(self, type: TypeNode) -> type:
        raise NotImplementedError

    def to_py_value(self, type: TypeNode, value: Any) -> Any:
        return value

    def from_py_value(self, type: TypeNode, value: Any) -> Any:
        return value


type_mappings: dict[TypeSignature, TypeMapping] = {}


def register_mapping(
    mapping: TypeMapping,
    *,
    tags: list[TypeTag] = None,
    hints: list[TypeHint] = None,
    flags: TypeFlag = None,
):
    if not tags and not hints:
        raise ValueError("at least one tag or hint must be specified")
    tags = tags or []
    hints = hints or []
    flags = flags or TypeFlag.Zero
    for tag in tags:
        type_mappings[TypeSignature(tag, None, flags)] = mapping
    for hint in hints:
        tag = TYPE_TAG_BY_TYPE_HINT[hint]
        type_mappings[TypeSignature(tag, hint, flags)] = mapping


def get_flat_mapping(type: TypeNode) -> TypeMapping:
    """
    Gets the most appropriate mapping for the given type.
    (flat because we ignore list and optional types).
    """
    # strip list and optional types
    stripped_flags = type.flags & ~TypeFlag.IsArray & ~TypeFlag.IsNullable
    exact_signature = TypeSignature(type.tag, type.hint, stripped_flags)
    mapping = type_mappings.get(exact_signature)
    if mapping is not None:
        return mapping
    # no exact match, try generic without hint
    stripped_signature = TypeSignature(type.tag, None, stripped_flags)
    mapping = type_mappings.get(stripped_signature)
    if mapping is not None:
        return mapping
    raise LookupError(f"no mapping for {type}")


@dataclass(repr=False, slots=True)
class StaticTypeMapping(TypeMapping):
    py_type: type

    def to_py_type(self, type: TypeNode) -> type:
        return self.py_type

    def to_py_value(self, type: TypeNode, value: Any) -> Any:
        return self.py_type(value)


class StringifyTypeMapping(StaticTypeMapping):
    def to_py_value(self, type: TypeNode, value: Any) -> Any:
        return self.py_type(value)

    def from_py_value(self, type: TypeNode, value: Any) -> str:
        return str(value)


class IsoDtTypeMapping(StaticTypeMapping):
    def to_py_value(self, type: TypeNode, value: Any) -> Any:
        return self.py_type.fromisoformat(value)

    def from_py_value(self, type: TypeNode, value: Any) -> str:
        return value.isoformat()


class EnumMapping(TypeMapping):
    def to_py_type(self, type: TypeNode) -> Any:
        members = {to_pyidentifier(child.name): child.name for child in type.type_nodes}
        enum_name = type.name or "_anon_" + uuid4().hex
        return enum.StrEnum(enum_name, members)

    def to_py_value(self, type: TypeNode, value: Any) -> Any:
        return map_unkey_enum(value, type)

    def from_py_value(self, type: TypeNode, value: Any) -> Any:
        return map_rekey_enum(value, type)


class FileMapping(TypeMapping):
    def to_py_type(self, type: TypeNode) -> type:
        return RemoteObjectInstance

    def to_py_value(self, type: TypeNode, value: Any) -> Any:
        return RemoteObjectInstance(
            id=UUID(value["id"]),
            name=value["name"],
            content_type=value["content_type"],
            content_length=value["content_length"],
            sha512=value["sha512"],
            status=RemoteObjectStatus[value["status"]],
        )

    def from_py_value(self, type: TypeNode, value: Any) -> Any:
        return {
            TYPENAME_SENTINEL: REMOTE_OBJECT_TYPENAME,
            "id": str(value.id),
            "name": value.name,
            "content_type": value.content_type,
            "content_length": value.content_length,
            "sha512": value.sha512,
            "status": value.status.name,
        }


class SecretTypeMapping(TypeMapping):
    def to_py_type(self, type: TypeNode) -> Any:
        return SecretInstance

    def to_py_value(self, type: TypeNode, value: Any) -> Any:
        return SecretInstance(
            id=UUID(value["id"]),
            sha512=value["sha512"],
        )

    def from_py_value(self, type: TypeNode, value: Any) -> Any:
        return {
            TYPENAME_SENTINEL: SECRET_TYPENAME,
            "id": str(value.id),
            "sha512": value.sha512,
        }


class StructTypeMapping(TypeMapping):
    def to_py_type(self, type: TypeNode) -> typing.TypedDict:
        return typing.TypedDict(
            type.name,
            {member.ident: instantiate_py_type(member) for member in type.type_nodes},
        )

    def to_py_value(self, type: TypeNode, value: Any) -> Any:
        if not isinstance(type, TypeInstance):
            raise ValueError(f"struct type {type} is not an instance")
        return type(**value)

    def from_py_value(self, type: TypeNode, value: Any) -> Any:
        return {TYPENAME_SENTINEL: type.key, **value}


# type tags
register_mapping(StaticTypeMapping(str), tags=[TypeTag.STRING])
register_mapping(StaticTypeMapping(float), tags=[TypeTag.NUMBER])
register_mapping(StaticTypeMapping(type(None)), tags=[TypeTag.NULL])
register_mapping(StaticTypeMapping(bool), tags=[TypeTag.BOOLEAN])
register_mapping(FileMapping(), tags=[TypeTag.FILE, TypeTag.IMAGE, TypeTag.AUDIO])
register_mapping(EnumMapping(), tags=[TypeTag.ENUM])
register_mapping(StructTypeMapping(), tags=[TypeTag.STRUCT])
# type hints
register_mapping(StringifyTypeMapping(UUID), hints=[TypeHint.UUID])
register_mapping(IsoDtTypeMapping(date), hints=[TypeHint.DATE])
register_mapping(IsoDtTypeMapping(datetime), hints=[TypeHint.DATETIME])
register_mapping(IsoDtTypeMapping(time), hints=[TypeHint.TIME])
register_mapping(StaticTypeMapping(int), hints=[TypeHint.INTEGER])
# other
register_mapping(
    SecretTypeMapping(), tags=[TypeTag.STRING, TypeTag.NUMBER], flags=TypeFlag.IsSecret
)


def instantiate_py_type(node: TypeNode) -> type | LiteralValue:
    """Create the Python-native type for the given type node."""
    mapping = get_flat_mapping(node)
    py_type = mapping.to_py_type(node)
    if node.flags & TypeFlag.IsArray:
        return list[py_type]
    else:
        return py_type


def instantiate_type(type: Type, session: Session) -> TypeInstance:
    """Instrument and instantiate a type for use."""
    py_type = instantiate_py_type(type)
    return TypeInstance(**type.__dict__, py_type=py_type, session=session)


def instantiate_py_value_flat(value: Any, type: TypeNode) -> Any:
    """Maps to the Python representation of the given value."""
    if value is None:  # skip null values
        return None  # type checking is done elsewhere
    # auto coerce lists to element and vice versa (like in frontend) :ArrayCoercion
    mapping = get_flat_mapping(type)
    try:
        if type.flags & TypeFlag.IsArray:
            if not isinstance(value, list):
                value = [value]
            return [mapping.to_py_value(type, v) for v in value]
        else:
            if isinstance(value, list):
                value = value[0]
            return mapping.to_py_value(type, value)
    except (KeyError, ValueError, TypeError):
        logger.warning("instantiate_failed", exc_info=True, value=value, type=type)
        return None


def strip_py_value_flat(value: Any, type: TypeNode) -> Any:
    """Maps back to the raw value from the Python representation."""
    # we don't auto-coerce here since that's only needed for external data
    if value is None:
        return None
    mapping = get_flat_mapping(type)
    if isinstance(value, list):
        return [mapping.from_py_value(type, v) for v in value]
    else:
        return mapping.from_py_value(type, value)


def instantiate_data(dataset: Data, session: Session) -> DataTableInstance | DataValueInstance:
    """Instrument and instantiate a data symbol."""
    records = []
    for raw_record in dataset.records:
        py_record_data = map_value(
            raw_record.data,
            dataset.type,
            map_v=instantiate_py_value_flat,
            map_k=lambda t: (t.key, t.ident),
            ignore_outer_map=True,  # we're mapping that to RecordInstance
        )
        record = RecordInstance(
            id=raw_record.id,
            order_key=raw_record.order_key,
            data=py_record_data,
            type=dataset.type,
            session=session,
            owner=dataset,
        )
        records.append(record)
    if dataset.flags & TypeFlag.IsArray:
        return DataTableInstance(
            **dict_minus(dataset.__dict__, "records"), records=records, session=session
        )
    else:
        return DataValueInstance(
            **dict_minus(dataset.__dict__, "records"), records=records, session=session
        )


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
    elif isinstance(symbol, Data):
        return instantiate_data(symbol, session)
    elif isinstance(symbol, Type):
        return instantiate_type(symbol, session)
    else:
        raise ValueError(f"cannot instantiate {symbol} in {session}")


BuildMap = Callable[[InterpSymbol], Optional[InterpSymbol]]
