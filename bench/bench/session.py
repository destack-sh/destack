import abc
import contextvars
import enum
import textwrap
import typing
from concurrent.futures import Executor, ThreadPoolExecutor
from dataclasses import dataclass
from datetime import date, datetime, time
from random import Random
from typing import Any, Callable, Optional
from uuid import UUID, uuid4

import aiohttp
import structlog
from asgiref.sync import async_to_sync

from bench.bench import TypeHint, TypeTag, wire
from bench.bench.const import (
    STATIC_BUILTINS,
    TYPE_TAG_BY_TYPE_HINT,
    ModuleOp,
    RemoteObjectStatus,
    SessionContext,
    SessionMode,
    TypeFlag,
)
from bench.bench.dataset import Query, Sort
from bench.bench.mutate import ModuleMutation, ModuleMutator
from bench.bench.tracing import SessionTracer
from bench.bench.type import (
    Code,
    CodeTransformation,
    Dataset,
    File,
    HasSession,
    Model,
    Module,
    ModuleNode,
    RemoteObject,
    Scope,
    Secret,
    Statement,
    Symbol,
    Task,
    Type,
    TypeBase,
)
from bench.bench.typer import map_rekey_enum, map_unkey_enum
from bench.bench.unsecure import do_execute_arbitrary_code
from bench.msg import NMessageType
from bench.msg.core import NMessage, request
from bench.msg.messages import (
    RepReadObjectPayload,
    RepReadSecretPayload,
    ReqReadObjectPayload,
    ReqReadSecretPayload,
)
from bench.utils.utils import to_pyidentifier

if typing.TYPE_CHECKING:
    from bench.bench.build import XPrompt
    from bench.bench.inference import ModelInference

logger = structlog.get_logger(__name__)

active_session: contextvars.ContextVar[Optional["Session"]] = contextvars.ContextVar(
    "active_session", default=None
)


class SessionBase(abc.ABC):
    module: Module

    @property
    def is_open(self) -> bool:
        raise NotImplementedError

    def add(self, obj: ModuleNode):
        raise NotImplementedError

    def remove(self, obj: ModuleNode):
        raise NotImplementedError


class Session:
    """A managed context for running code in a module (may mutate)."""

    def __init__(
        self,
        module: Module,
        ctx: SessionContext | None = None,
        instances: list["ModuleNode"] = None,
        default_models: list["Model"] = None,
        cache_inferences: bool = True,
        inference_timeout: int = 30,
        inference_retries: int = 5,
        mode: SessionMode = SessionMode.READ_ONLY,
        write: Callable[[list[ModuleMutation]], typing.Awaitable[bool]] = None,
        executor: Executor = None,
    ):
        if mode != SessionMode.READ_ONLY and write is None:
            raise ValueError("write must be provided for non-readonly sessions")
        self.id = uuid4()
        self.ctx = ctx
        self.module = module
        self.instances: dict[UUID, "HasSession"] = {i.id: i for i in instances} if instances else {}
        self.default_models = default_models or [
            module.lookup_symbol("openai.std.text.gpt3"),
            module.lookup_symbol("anthropic.std.text.claude-instant"),
        ]
        self.cache_inferences = cache_inferences
        self.inference_timeout = inference_timeout
        self.inference_retries = inference_retries
        self.mode = mode
        self.write = write

        self.anonymous_scope = Scope(parent=self.module)
        self.executor = executor or ThreadPoolExecutor(max_workers=1)
        self.logger = logger.bind(session=self)
        self.mutator = ModuleMutator(self.module)
        self.tracer = SessionTracer(self, mutator=self.mutator, publish=True, validate=True)
        self.instance = SessionAccess(self)
        self.opened_at: Optional[datetime] = None
        self.closed_at: Optional[datetime] = None

    def __str__(self):
        status = "open" if self.opened_at else ("closed" if self.closed_at else "pending")
        return (
            f"{self.module.name} {self.id} ({self.mode}, {status}, {len(self.mutator.mutations)})"
        )

    def __repr__(self):
        return f"<Session {self}>"

    @property
    def is_open(self) -> bool:
        return self.opened_at is not None and self.closed_at is None

    def add(self, *objs: "HasSession", new: bool = False) -> None:
        if new:
            for obj in objs:
                # permissions are checked in tracer
                if isinstance(obj, File):
                    self.tracer.file_create(obj)
                elif isinstance(obj, Statement):
                    self.tracer.statement_create(obj)
                    if isinstance(obj, Symbol):
                        # it feels like this should be done in some tracer? also (re?)-index?
                        self.module.symbols_by_id[obj.id] = obj
        for obj in objs:
            self.instances[obj.id] = obj

    def remove(self, *objs: "HasSession") -> None:
        for obj in objs:
            if isinstance(obj, (Symbol, Statement, File)) and obj.id in self.instances:
                del self.instances[obj.id]
                # not doing anything yet?

    def check_can(self, op: ModuleOp, thing: File | Statement):
        if not self.can(op, thing):
            raise RuntimeError(f"cannot {op} {thing} in {self}")

    def can(self, op: ModuleOp, thing: File | Statement) -> bool:
        if self.mode == SessionMode.READ_ONLY:
            return op in (ModuleOp.READ, ModuleOp.SEARCH)
        elif self.mode == SessionMode.WRITE:
            return True
        else:
            raise RuntimeError(f"unknown session mode {self.mode}")

    def open(self):
        """Opens the session for execution and modification."""
        if self.opened_at is not None:
            raise RuntimeError(f"session already opened {self}")
        self.opened_at = datetime.now()
        if active_session.get() is not None:
            raise RuntimeError(f"another session is active: {active_session.get()}")
        active_session.set(self)
        logger.debug("session.open", session=self)

    async def aflush(self, keep_open: bool = True):
        """Flushes all module mutations."""
        if not self.mutator.mutations:
            return
        if self.mode == SessionMode.READ_ONLY:
            raise RuntimeError(f"cannot mutate read-only session {self}")
        logger.debug("session.flush", session=self, mutator=self.mutator)
        mutations = self.mutator.bundle().compact()
        success = await self.write(mutations)
        if not success:
            raise RuntimeError(f"failed to write mutations {self.mutator.mutations}")
        logger.debug("session.flush.done", session=self)

    def flush(self):
        async_to_sync(self.aflush)()

    async def aclose(self, flush: bool = True):
        """Closes the session, flushing any mutations and preventing further execution/mutation."""
        if self.closed_at is not None:
            raise RuntimeError(f"session already closed {self}")
        self.closed_at = datetime.now()
        if flush:
            await self.aflush(keep_open=False)
        active_session.set(None)
        logger.debug("session.close", session=self)

    def close(self, flush: bool = True):
        async_to_sync(self.aclose)(flush=flush)

    async def __aenter__(self):
        self.open()
        return self

    async def __aexit__(self, exc_type, exc_value, traceback):
        await self.aclose()

    def sync(self):
        """A sync context manager for this session."""
        session = self

        class SyncSession:
            def __enter__(self):
                session.open()
                return session

            def __exit__(self, exc_type, exc_value, traceback):
                session.close()

        return SyncSession()


class SessionAccess:
    """Not sure what this is about yet. Wrapping some previously instance-level/build stuff."""

    def __init__(self, session: "Session"):
        self.session = session
        self._cached_implementations: dict[tuple[UUID, UUID], "XPrompt"] = {}

    def get_implementations(self, task: "Task", model: Model | str = None) -> list["XPrompt"]:
        """Gets or builds an implementation for a task."""
        from bench.bench.build import build_task_implementation

        if model is not None:
            if isinstance(model, str):
                model = self.session.module.find_symbol(model, symbol_t=Model)
            models = [model]
        else:
            models = self.session.default_models

        implementations = []
        for model in models:
            cache_key = (task.id, model.id)
            if cache_key not in self._cached_implementations:
                self._cached_implementations[cache_key] = build_task_implementation(
                    task, model, self.session
                )
            implementations.append(self._cached_implementations[cache_key])
        return implementations

    def get_inference(self, model: "Model"):
        """Instantiates the inference component of a Model symbol"""
        return _instantiate_model(model, self.session)

    def get_code_callable(self, code: Code):
        """Instantiates the callable of a Code symbol"""
        return _instantiate_code(code, self.session)

    def get_py_type(self, type: Type):
        """Instantiates the python type of a Type symbol"""
        return instantiate_py_type(type, self.session)

    async def dataset_asearch(self, dataset: Dataset, query: Query, sort: list[Sort]):
        raise NotImplementedError

    async def remote_object_aread(self, obj: RemoteObject, timeout: int):
        """Reads a remote object in full."""
        # get GET url to access file
        rep: NMessage[RepReadObjectPayload] = await request(
            NMessageType.REQUEST_READ_OBJECT,
            ReqReadObjectPayload(objects=[wire.pack_data(obj)]),
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

    async def secret_areveal(self, secret: Secret, timeout: int):
        """Reads a remote secret."""
        rep: NMessage[RepReadSecretPayload] = await request(
            NMessageType.REQUEST_READ_SECRET,
            ReqReadSecretPayload(secrets=[wire.pack_data(secret)]),
            reply_t=RepReadSecretPayload,
            timeout=timeout,
        )
        return rep.p.secrets[0].value


TYPENAME_SENTINEL = "__typename"  # :TypeSentinel
REMOTE_OBJECT_TYPENAME = "RemoteObject"
SECRET_TYPENAME = "Secret"
TypeSignature = typing.NamedTuple(
    "TypeSignature", [("tag", TypeTag), ("hint", Optional[TypeHint]), ("flags", TypeFlag)]
)


class TypeMapper:
    """
    Maps specific types (and values) into and from Python.
    Don't bother with lists and optional types here.
    """

    def to_py_type(self, type: TypeBase) -> type:
        raise NotImplementedError

    def to_py_value(self, type: TypeBase, value: Any) -> Any:
        return value

    def from_py_value(self, type: TypeBase, value: Any) -> Any:
        return value


type_mappers: dict[TypeSignature, TypeMapper] = {}


def register_mapper(
    mapping: TypeMapper,
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
        type_mappers[TypeSignature(tag, None, flags)] = mapping
    for hint in hints:
        tag = TYPE_TAG_BY_TYPE_HINT[hint]
        type_mappers[TypeSignature(tag, hint, flags)] = mapping


def get_flat_mapper(type: TypeBase) -> TypeMapper:
    """
    Gets the most appropriate mapping for the given type.
    (flat because we ignore list and optional types).
    """
    # strip to only relevant flags for mapping
    stripped_flags = type.flags & TypeFlag.IsSecret
    exact_signature = TypeSignature(type.tag, type.hint, stripped_flags)
    mapping = type_mappers.get(exact_signature)
    if mapping is not None:
        return mapping
    # no exact match, try generic without hint
    stripped_signature = TypeSignature(type.tag, None, stripped_flags)
    mapping = type_mappers.get(stripped_signature)
    if mapping is not None:
        return mapping
    raise LookupError(f"no mapping found for {type}")


@dataclass(repr=False, slots=True)
class StaticTypeMapper(TypeMapper):
    py_type: type

    def to_py_type(self, type: TypeBase) -> type:
        return self.py_type

    def to_py_value(self, type: TypeBase, value: Any) -> Any:
        return self.py_type(value)


class StringifyTypeMapping(StaticTypeMapper):
    def to_py_value(self, type: TypeBase, value: Any) -> Any:
        return self.py_type(value)

    def from_py_value(self, type: TypeBase, value: Any) -> str:
        return str(value)


class IsoDtTypeMapping(StaticTypeMapper):
    def to_py_value(self, type: TypeBase, value: Any) -> Any:
        return self.py_type.fromisoformat(value)

    def from_py_value(self, type: TypeBase, value: Any) -> str:
        return value.isoformat()


class EnumMapper(TypeMapper):
    def to_py_type(self, type: TypeBase) -> Any:
        members = {to_pyidentifier(child.name): child.name for child in type.fields}
        enum_name = type.name or "_anon_" + uuid4().hex
        return enum.StrEnum(enum_name, members)

    def to_py_value(self, type: TypeBase, value: Any) -> Any:
        return map_unkey_enum(value, type)

    def from_py_value(self, type: TypeBase, value: Any) -> Any:
        return map_rekey_enum(value, type)


class FileMapper(TypeMapper):
    def to_py_type(self, type: TypeBase) -> type:
        return RemoteObject

    def to_py_value(self, type: TypeBase, value: Any) -> Any:
        return RemoteObject(
            id=UUID(value["id"]),
            name=value["name"],
            content_type=value["content_type"],
            content_length=value["content_length"],
            sha512=value["sha512"],
            status=RemoteObjectStatus[value["status"]],
        )

    def from_py_value(self, type: TypeBase, value: Any) -> Any:
        return {
            TYPENAME_SENTINEL: REMOTE_OBJECT_TYPENAME,
            "id": str(value.id),
            "name": value.name,
            "content_type": value.content_type,
            "content_length": value.content_length,
            "sha512": value.sha512,
            "status": value.status.name,
        }


class SecretTypeMapper(TypeMapper):
    def to_py_type(self, type: TypeBase) -> Any:
        return Secret

    def to_py_value(self, type: TypeBase, value: Any) -> Any:
        return Secret(
            id=UUID(value["id"]),
            sha512=value["sha512"],
        )

    def from_py_value(self, type: TypeBase, value: Any) -> Any:
        return {
            TYPENAME_SENTINEL: SECRET_TYPENAME,
            "id": str(value.id),
            "sha512": value.sha512,
        }


class StructTypeMapper(TypeMapper):
    def to_py_type(self, type: TypeBase) -> typing.TypedDict:
        return typing.TypedDict(
            type.name,
            {member.ident: instantiate_py_type(member) for member in type.fields},
        )

    def to_py_value(self, type: TypeBase, value: Any) -> Any:
        if not isinstance(type, Type):
            # may be a simple type node
            if isinstance(type.reference, Type):
                type = type.reference
            else:
                raise ValueError(f"struct type is not an instance: {type}")
        return type(**value)

    def from_py_value(self, type: TypeBase, value: Any) -> Any:
        return {TYPENAME_SENTINEL: type.key, **value}


# type tags
register_mapper(StaticTypeMapper(str), tags=[TypeTag.STRING])
register_mapper(StaticTypeMapper(float), tags=[TypeTag.NUMBER])
register_mapper(StaticTypeMapper(type(None)), tags=[TypeTag.NULL])
register_mapper(StaticTypeMapper(bool), tags=[TypeTag.BOOLEAN])
register_mapper(FileMapper(), tags=[TypeTag.FILE])
register_mapper(EnumMapper(), tags=[TypeTag.ENUM])
register_mapper(StructTypeMapper(), tags=[TypeTag.STRUCT])
# type hints
register_mapper(StringifyTypeMapping(UUID), hints=[TypeHint.UUID])
register_mapper(IsoDtTypeMapping(date), hints=[TypeHint.DATE])
register_mapper(IsoDtTypeMapping(datetime), hints=[TypeHint.DATETIME])
register_mapper(IsoDtTypeMapping(time), hints=[TypeHint.TIME])
register_mapper(StaticTypeMapper(int), hints=[TypeHint.INTEGER])
# other
register_mapper(SecretTypeMapper(), tags=[TypeTag.STRING, TypeTag.NUMBER], flags=TypeFlag.IsSecret)


def instantiate_py_type(node: TypeBase) -> type | Any | None:
    """Create the Python-native type for the given type node."""
    if node.tag == TypeTag.FUNCTION:
        return None  # functions don't have a pytype
    map = get_flat_mapper(node)
    py_type = map.to_py_type(node)
    if node.flags & TypeFlag.IsArray:
        return list[py_type]
    else:
        return py_type


def instantiate_py_value_flat(value: Any, type: TypeBase, ignore_array: bool = False) -> Any:
    """Maps to the Python representation of the given value."""
    if value is None:  # skip null values
        return None  # type checking is done elsewhere
    # auto coerce lists to element and vice versa (like in frontend) :ArrayCoercion
    mapping = get_flat_mapper(type)
    try:
        if type.flags & TypeFlag.IsArray and not ignore_array:
            if not isinstance(value, list):
                value = [value]
            return [mapping.to_py_value(type, v) for v in value]
        else:
            if isinstance(value, list):
                value = value[0]
            return mapping.to_py_value(type, value)
    except (KeyError, ValueError, TypeError):
        logger.warning("instantiate_failed", exc_info=True, value=value, type=type)
        return value  # type checking is done elsewhere


def strip_py_value_flat(value: Any, type: TypeBase, *args, **kwargs) -> Any:
    """Maps back to the raw value from the Python representation."""
    # we don't auto-coerce here since that's only needed for external data
    if value is None:
        return None
    mapping = get_flat_mapper(type)
    return mapping.from_py_value(type, value)


def _instantiate_code(
    code: Code, session: Session
) -> tuple[CodeTransformation, Callable[..., Any]]:
    """Instantiates code into a Python callable in the context of the session."""
    context = {**code._references}
    if not code._parse.is_async:
        # replace any async functions with sync versions
        for key, symbol in context.items():
            if isinstance(symbol, (Code, Task)) and symbol.is_async:
                context[key] = symbol.to_sync()

    dynamic_context = {
        "session": session,
        "context": {symbol.name: symbol for symbol in context.values()},  # by name
        **context,  # inlined
        "random": Random(code.id.hex.encode()),
    }

    if code.language == "python":
        python_code = code.code or "pass"
        locals = {**STATIC_BUILTINS, **dynamic_context}
    else:
        raise ValueError(f"unexpected code language: {code}")

    # stub fake lines
    python_code_lines = python_code.splitlines()
    for i in code._parse.fake_line_numbers:
        python_code_lines[i] = "pass # " + python_code_lines[i]
    python_code = "\n".join(python_code_lines)

    # create python function from python code
    input_keys = [i.name for i in code.inputs]
    func_name = f"{to_pyidentifier(code.name)}_{code.id.hex[:6]}"
    async_str = "async " if code._parse.is_async else ""
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
        start_offset=1,  # for method signature
        method_name=func_name,
    )
    return transform, callable


def _instantiate_model(model: Model, session: Session) -> "ModelInference":
    """
    Instantiates the model inference endpoints for the session.
    If we don't have the key, we proxy to the langserver.
    """
    from bench.bench.inference import (
        CachedInferenceEndpoint,
        ModelInference,
        RemoteInferenceEndpoint,
    )
    from bench.runtime.common.models import get_inference_endpoints_cls

    key = None  # TODO @Broken: get model key from module? same file? some constant?
    inference = ModelInference(external_name=model.external_name, key=key)
    endpoints = list(get_inference_endpoints_cls(model))

    if not endpoints:
        raise RuntimeError(f"no endpoints found for model: {model}")
    for modality, endpoint_cls in endpoints:
        if key is not None:
            endpoint = getattr(endpoint_cls(**inference.__dict__), modality)
        else:
            endpoint = RemoteInferenceEndpoint(
                model=model, modality=modality, timeout=session.inference_timeout
            )
        endpoint_proxy = CachedInferenceEndpoint(
            model=model,
            modality=modality,
            endpoint=endpoint,
            tracer=session.tracer,
            cache_inferences=session.cache_inferences,
            timeout=session.inference_timeout,
        )
        setattr(inference, modality, endpoint_proxy)
    return inference
