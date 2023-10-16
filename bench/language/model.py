import asyncio
import enum
import functools
import os
import typing
from dataclasses import dataclass
from datetime import datetime
from logging import Logger
from typing import Any
from uuid import UUID

import msgpack
import structlog

from bench.language.cache import CacheAsync
from bench.language.field import Field, HasFields, TypedDict, TypeTag
from bench.language.module import Node, ScopeNode, node_component, nruntime
from bench.language.packer import check_type, pack_value, unpack_value
from bench.utils.dt import utcnow_with_tz
from bench.utils.func import describe_type
from bench.utils.utils import get_from_env

if typing.TYPE_CHECKING:
    from bench.language import Statement
    from bench.language.task import TaskCompiler

logger = structlog.get_logger(__name__)

INFERENCE_CACHE_EXPIRY = get_from_env("INFERENCE_CACHE_EXPIRY", 60 * 60 * 24 * 30, type_cast=int)
ALLOW_KEY_FROM_ENV = get_from_env("MODEL_API_KEY_FROM_ENV", True, type_cast=bool)


class ModelErrorType(enum.StrEnum):
    Timeout = "Timeout"
    InvalidRequest = "InvalidRequest"
    ExceededLimit = "ExceededLimit"
    Unavailable = "Unavailable"
    Unknown = "Unknown"


@node_component
class HasModel(HasFields, Node):
    _remote: bool = nruntime(default=False)
    _endpoint_impl: typing.Optional[typing.Callable] = nruntime(default=None)
    _compiler_impl: typing.Optional[typing.Callable] = nruntime(default=None)
    _api_key: typing.Optional[str] = nruntime(default=None)
    _has_vector_io: bool = nruntime(default=None)

    # we only cache models without vector inputs/outputs

    def _clear_inner(self) -> None:
        self._api_key = None
        self._remote = True
        self._endpoint_impl = None
        self._compiler_impl = None
        self._has_vector_io = False

    def _interp_inner(self, scope: ScopeNode) -> None:
        # model is remote if we don't have the key in scope or environment
        provider = self.path.split(".")[0]
        if ALLOW_KEY_FROM_ENV:
            self._api_key = os.environ.get(f"{provider.upper()}_API_KEY")
        self._remote = self._api_key is None

        self._has_vector_io = False
        for n in self._walk_rec():
            if isinstance(n, Field) and n.tag == TypeTag.VECTOR:
                self._has_vector_io = True
                break

    @property
    def should_cache(self) -> bool:
        return not self._has_vector_io

    @property
    def _is_async(self) -> bool:
        return True

    async def _call_inner_async(self, timeout: int = None, cache: bool = None, **inputs):
        from bench.language.run import get_run_cache_subkey

        if cache is None:
            cache = self.should_cache
        inputs_raw = pack_value(inputs, self, is_output=False, ignore_outer_map=True)
        cache_subkey = get_run_cache_subkey(inputs_raw=inputs_raw)
        log = logger.bind(model=self, inputs=describe_type(inputs), cache_subkey=cache_subkey)
        log.debug("inference.enter.pre")

        # try to read from cache if enabled
        if cache and self.session.cache_inferences:
            # TODO @Performance: use leases to cooperatively inference endpoints
            cached_inference = await self.cache.get(cache_subkey)
            if cached_inference is not None:
                try:
                    inference = Inference.from_json_bytes(cached_inference)
                    outputs = unpack_value(
                        inference.outputs, self, ignore_outer_map=True, is_output=True
                    )
                    log.debug("inference.cache.hit", output=describe_type(outputs))
                    check_type(outputs, self, is_output=True)
                    self.session.tracer.run_cached(
                        statement=self,
                        inputs=inputs,
                        outputs=outputs,
                        generated_at=inference.generated_at,
                        generated_in=inference.generated_in,
                        duration=inference.duration,
                    )
                    return TypedDict(outputs, self, is_output=True)
                except Exception as e:
                    log.warning("inference.cache.error", e=e, exc_info=e)
                    # ignore and continue, will be overwritten

        if self._remote:
            # request remotely proxied inference if needed (key not available locally)
            from bench.msg.core import NMessage, request
            from bench.msg.messages import (
                NMessageType,
                RepRunInferencePayload,
                ReqRunInferencePayload,
            )

            self.session.tracer.run_enter(self, is_async=True, inputs=inputs)
            timeout = timeout if timeout is not None else self.session.inference_timeout
            try:
                req = ReqRunInferencePayload(
                    project_id=self.module.project_id,
                    model_path=self.path,
                    inputs=inputs_raw,
                    timeout=timeout,
                    run_id=self.session.current_run.id,
                )
                rep: NMessage[RepRunInferencePayload] = await request(
                    NMessageType.RUN_PROXY_INFERENCE,
                    req,
                    RepRunInferencePayload,
                    timeout=timeout + 3,
                )
                if rep.p.error is not None:
                    raise ModelError(rep.p.error, self, f"remote {self} failed")
                outputs = unpack_value(rep.p.outputs, self, is_output=True)
                self.session.tracer.run_exit(self, outputs)
                log.debug("inference.remote.exit", output=describe_type(outputs))
                return TypedDict(outputs, self, is_output=True)
            except Exception as e:
                self.session.tracer.run_exception(self, e)
                log.warning("inference.remote.error", e=e, exc_info=e)
                if isinstance(e, ModelError):
                    raise
                else:
                    raise ModelError(ModelErrorType.Unavailable, self, f"remote {self} failed")
        else:
            # otherwise run inference through endpoint :LibImplementation
            try:
                self.session.tracer.run_enter(self, is_async=True, inputs=inputs)
                timeout = timeout if timeout is not None else self.session.inference_timeout
                outputs = await asyncio.wait_for(
                    asyncio.shield(
                        self._inference(
                            inputs=inputs,
                            cache_subkey=cache_subkey,
                            cache=self.cache if cache else None,
                            run_id=self.session.current_run.id,
                            log=log,
                        )
                    ),
                    timeout,
                )
                self.session.tracer.run_exit(self, outputs)
                log.debug("inference.exit", output=describe_type(outputs))
                return TypedDict(outputs, self, is_output=True)
            except Exception as e:
                self.session.tracer.run_exception(self, e)
                log.warning("inference.error", e=e, exc_info=e)
                if isinstance(e, ModelError):
                    raise
                elif isinstance(e, asyncio.TimeoutError):
                    raise ModelError(ModelErrorType.Timeout, self, f"timeout {self} failed") from e
                else:
                    raise ModelError(ModelErrorType.Unavailable, self, f"{self} failed") from e

    async def _inference(
        self,
        inputs: Any,
        cache_subkey: str,
        cache: CacheAsync | None,
        run_id: UUID,
        log: Logger = logger,
    ) -> Any:
        """
        Runs inference on the given endpoint without timeout.
        This should be asyncio.shield-ed to ensure we write the result to cache (if enabled).
        """
        started_at = utcnow_with_tz()
        log.debug("inference.enter")
        output = await self._endpoint_resolved(**inputs)
        now = utcnow_with_tz()
        duration = (now - started_at).total_seconds()
        if cache and self.should_cache:
            # result is assumed to be JSON serializable, will obviously error here if not
            inference = Inference(
                generated_at=now,
                generated_in=run_id,
                duration=duration,
                inputs=pack_value(inputs, self, is_output=False, ignore_outer_map=True),
                outputs=pack_value(output, self, is_output=True, ignore_outer_map=True),
            )
            await cache.set(cache_subkey, inference.to_json_bytes(), expire=INFERENCE_CACHE_EXPIRY)
        log.debug("inference.exit", ret=describe_type(output), duration=duration)
        return output

    @property
    def _endpoint_resolved(self):
        if self._endpoint_impl is None:
            from bench.language import libs

            # get actual model implementation from libs  :LibImplementation
            # (this is a stop gap until we fully support model statements, then it's just like Code)
            actual_model_impl = libs.lookup_model_impl(self.path)
            if actual_model_impl is None:
                raise ValueError(f"cannot find {self} in libs")
            self._endpoint_impl = functools.partial(actual_model_impl, self=self)
        return self._endpoint_impl

    async def _endpoint(self, **kwargs) -> Any:
        raise NotImplementedError  # fake stub for :LibImplementation of model

    @property
    def _compiler_resolved(self):
        if self._compiler_impl is None:
            from bench.language import libs

            # get actual model implementation from libs  :LibCompiler (see above)
            actual_model_compiler = libs.lookup_model_compiler(self.path)
            if actual_model_compiler is None:
                raise ValueError(f"cannot find {self} in libs")
            self._compiler_impl = actual_model_compiler
        return self._compiler_impl

    def _compiler(self) -> "TaskCompiler":
        raise NotImplementedError  # stub for :LibCompiler of model compiler

    @property
    def compiler(self) -> "TaskCompiler":
        return self._compiler_resolved(self)


# avoid circular import
from .run import RunError, RunErrorKind  # noqa: E402


class ModelError(RunError):
    def __init__(
        self,
        type: ModelErrorType,
        statement: "Statement",
        message: str = None,
        path: str = None,
    ):
        super().__init__(
            kind=RunErrorKind.Runtime,
            type=type.name,
            statement=statement,
            message=f"{type.value}: {message}",
        )
        self.type = type
        self.path = path


@dataclass(slots=True)
class Inference:
    """A model inference - inputs/outputs are raw."""

    generated_at: datetime
    generated_in: UUID
    duration: float
    inputs: Any
    outputs: Any

    def to_json_bytes(self) -> bytes:
        inference_json = {
            "generated_at": self.generated_at.isoformat(),
            "generated_in": str(self.generated_in),  # UUID is not JSON serializable
            "duration": self.duration,
            "inputs": self.inputs,
            "output": self.outputs,
        }
        return msgpack.packb(inference_json, use_bin_type=True)

    @classmethod
    def from_json_bytes(cls, json_str: str):
        data = msgpack.unpackb(json_str, raw=False)
        return cls(
            generated_at=datetime.fromisoformat(data["generated_at"]),
            generated_in=UUID(data["generated_in"]),
            duration=data["duration"],
            inputs=data["inputs"],
            outputs=data["output"],
        )
