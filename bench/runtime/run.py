from __future__ import annotations

import asyncio
import enum
import traceback
import typing
from typing import Optional

import structlog

from bench.language.type import InterpSymbol, LiteralValue
from bench.runtime.instance import AsyncCodeInstance, SyncCodeInstance
from bench.runtime.tracing import Tracer, tracer_boundary
from bench.runtime.type import CodeInstance, PyFrameData
from bench.utils.utils import get_from_env, to_pyidentifier

logger = structlog.stdlib.get_logger(__name__)
ALLOW_UNTRUSTED_CODE = get_from_env("ALLOW_UNTRUSTED_CODE", False, type_cast=bool)


class RunErrorType(enum.Enum):
    INTERNAL = 0, "Internal error"
    PARSE = 1, "Parse error"
    VALIDATION = 2, "Validation error"
    RUNTIME = 3, "Runtime code error"
    UNTRUSTED = 4, "Untrusted code error"

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

    def get_traceback(self, from_code: CodeInstance) -> Optional[list[PyFrameData]]:
        if self.cause is None:
            return None
        stack_summary = traceback.StackSummary.extract(traceback.walk_tb(self.cause.__traceback__))
        stack = PyFrameData.from_stack(stack_summary)
        return PyFrameData.clean(stack, from_code)


class Proxy:
    """A worker-side proxy for wrapping symbol access."""

    def __init__(
        self,
        tracer: Tracer,
        cache_inferences: bool,
        inference_timeout: int = 20,
        inference_retries: int = 3,
    ):
        self.tracer = tracer
        self.cache_inferences = cache_inferences
        self.inference_timeout = inference_timeout
        self.inference_retries = inference_retries

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
                    timeout=self.inference_timeout,
                    retries=self.inference_retries,
                )
                setattr(inference_proxy, modality, endpoint_proxy)
        model.inference = inference_proxy
        return model


def run_sync(
    code: SyncCodeInstance, arguments: dict[str, LiteralValue] | None = None
) -> LiteralValue:
    """Runs the code instance synchronously. Not to be used in production."""
    arguments = arguments or {}
    try:
        if code.is_async:
            raise RuntimeError(f"cannot run async code synchronously: {code}")
        with tracer_boundary():
            return code(**arguments)
    except Exception as e:
        raise RunError(RunErrorType.RUNTIME, code, cause=e) from e


async def run(
    code: AsyncCodeInstance | SyncCodeInstance,
    arguments: dict[str, LiteralValue] | None = None,
    is_trusted: bool = False,
) -> LiteralValue:
    if not is_trusted and not ALLOW_UNTRUSTED_CODE:
        raise RunError(RunErrorType.UNTRUSTED, code)
    # transform keys to valid python identifiers
    arguments = {to_pyidentifier(k): v for k, v in (arguments or {}).items()}
    try:
        with tracer_boundary():
            if code.is_async:
                return await code(**arguments)
            else:
                return await asyncio.to_thread(code, **arguments)
    except Exception as e:
        raise RunError(RunErrorType.RUNTIME, code, cause=e) from e
