import asyncio
import enum
from logging import Logger
from typing import TYPE_CHECKING, Any, Optional, Union
from uuid import UUID

import aiohttp
import deepgram
import numpy as np
import openai
import structlog

from bench.language.cache import Cache
from bench.language.field import Field, HasFields, TypedDict
from bench.language.node import Node, ScopeNode, node_component, struct_runtime
from bench.utils.dt import utcnow_with_tz
from bench.utils.func import describe_type
from bench.utils.utils import get_from_env, omit_empty

if TYPE_CHECKING:
    from bench.language.issue import IssueHandler
    from bench.language.statement import Statement

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
    _remote: bool = struct_runtime(default=False)
    _api_key: Optional[str] = struct_runtime(default=None)
    _has_vector_io: bool = struct_runtime(default=None)

    # we only cache models without vector inputs/outputs

    def _clear_inner(self, scope: Optional[ScopeNode] = None) -> None:
        self._api_key = None
        self._remote = True
        self._has_vector_io = False

    def _interp_inner(self, scope: ScopeNode, on_issue: "IssueHandler") -> None:
        # model is remote if we don't have the key in scope or environment
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
        from bench.language.value import check_type, pack_value, unpack_value
        from bench.language.run import get_run_cache_subkey

        if cache is None:
            cache = self.should_cache
        inputs_raw = pack_value(inputs, self, is_output=False, ignore_outer=True)
        cache_subkey = get_run_cache_subkey(inputs_raw=inputs_raw)
        log = logger.bind(model=self, inputs=describe_type(inputs), cache_subkey=cache_subkey)
        log.debug("inference.enter.pre")

        # try to read from cache if enabled
        if cache:
            cached_inference = await self.cache.get(cache_subkey)
            if cached_inference is not None:
                try:
                    inference = MiniRun.from_json_bytes(cached_inference)
                    outputs = unpack_value(
                        inference.outputs, self, ignore_outer=True, is_output=True
                    )
                    log.debug("inference.cache.hit", output=describe_type(outputs))
                    check_type(outputs, self, is_output=True)
                    self.session._run_cached(
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

            self.session._run_enter(self, is_async=True, inputs=inputs)
            timeout = timeout if timeout is not None else self.session.inference_timeout
            try:
                outputs = await self.session.host.run_proxy_inference(self, inputs_raw, timeout)
                outputs = unpack_value(outputs, self, is_output=True)
                log.debug("inference.remote.exit", output=describe_type(outputs))
                outputs = TypedDict(outputs, self, is_output=True)
            except BaseException as e:
                self.session._run_exception(self, e)
                log.warning("inference.remote.error", e=e, exc_info=e)
                if isinstance(e, (ModelError, asyncio.CancelledError)):
                    raise
                else:
                    raise ModelError(ModelErrorType.Unavailable, self, f"remote {self} failed")
            self.session._run_exit(self, outputs)
            return outputs
        else:
            # otherwise run inference through endpoint :LibImplementation
            try:
                self.session._run_enter(self, is_async=True, inputs=inputs)
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
                self.session._run_exit(self, outputs)
                log.debug("inference.exit", output=describe_type(outputs))
                return TypedDict(outputs, self, is_output=True)
            except BaseException as e:
                self.session._run_exception(self, e)
                log.warning("inference.error", e=e, exc_info=e)
                if isinstance(e, (ModelError, asyncio.CancelledError)):
                    raise
                elif isinstance(e, asyncio.TimeoutError):
                    raise ModelError(ModelErrorType.Timeout, self, f"timeout {self} failed") from e
                else:
                    raise ModelError(ModelErrorType.Unavailable, self, f"{self} failed") from e

    async def _inference(
        self,
        inputs: Any,
        cache_subkey: str,
        cache: Cache | None,
        run_id: UUID,
        log: Logger = logger,
    ) -> Any:
        """
        Runs inference on the given endpoint without timeout.
        This should be asyncio.shield-ed to ensure we write the result to cache (if enabled).
        """
        from bench.language.value import pack_value

        started_at = utcnow_with_tz()
        log.debug("inference.enter")
        output = await self._endpoint_resolved(**inputs)
        now = utcnow_with_tz()
        duration = (now - started_at).total_seconds()
        if cache and self.should_cache:
            # result is assumed to be JSON serializable, will obviously error here if not
            inference = MiniRun(
                generated_at=now,
                generated_in=run_id,
                duration=duration,
                inputs=pack_value(inputs, self, is_output=False, ignore_outer=True),
                outputs=pack_value(output, self, is_output=True, ignore_outer=True),
            )
            await cache.set(cache_subkey, inference.to_json_bytes(), expire=INFERENCE_CACHE_EXPIRY)
        log.debug("inference.exit", ret=describe_type(output), duration=duration)
        return output


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


class OpenAIChatCompletionModel(HasModel):
    async def _endpoint(
        self,
        messages: list[dict],
        settings: dict,
    ) -> dict:
        try:
            response = await openai.ChatCompletion.acreate(
                model=self.external_name,
                messages=[omit_empty(m) for m in messages],
                **omit_empty(settings),
                api_key=self._api_key,
            )
        except Exception as e:
            raise _map_openai_error(self, e) from e
        message = response["choices"][0]["message"]
        if "function_call" in message:
            function_call = dict(
                name=message["function_call"]["name"],
                arguments=message["function_call"]["arguments"],
            )
        else:
            function_call = None
        return dict(
            message=dict(
                role=message["role"],
                content=message["content"],
                name=message.get("name"),
                function_call=function_call,
            ),
            usage=dict(
                prompt_tokens=response["usage"]["prompt_tokens"],
                completion_tokens=response["usage"].get("completion_tokens"),
                total_tokens=response["usage"]["total_tokens"],
            ),
        )

    def _compiler(self) -> "TaskCompiler":
        # TODO @Task: select text/chat compiler more intelligently
        return OpenAITextCompiler()


def _map_openai_error(model: "Statement", e: Exception) -> ModelError:
    if isinstance(e, openai.InvalidRequestError):
        return ModelError(ModelErrorType.InvalidRequest, model, str(e))
    return ModelError(ModelErrorType.Unavailable, model, str(e))


def tokens(self) -> int:
    """Estimated token usage (very rough)."""
    messages_str = "\n".join([f"{m.role}: {m.content}" for m in self.messages])
    return int(len(messages_str) * 0.3)


class DeepgramAudioTranscriptionModel(HasModel):
    async def _endpoint(self, url: str, language: Optional[str] = None) -> dict:
        dg_client = deepgram.Deepgram(self._api_key)
        options = {"model": "nova-2", "smart_format": True, "diarize": True}
        if language:
            options["language"] = language
        response = await dg_client.transcription.prerecorded({"url": url}, options)

        # transcript = newline separated utterances
        # timestamps as [<HH:mm:ss>]
        # speaker prefix as [Speaker:<speaker_id>]
        utterances = []
        alternatives = response["results"]["channels"][0]["alternatives"]
        for paragraph in alternatives[0]["paragraphs"]["paragraphs"]:
            start_time = paragraph["start"]
            hours = int(start_time) // 3600
            minutes = int(start_time) // 60 % 60
            seconds = int(start_time) % 60
            start_time = f"{hours:02}:{minutes:02}:{seconds:02}"
            paragraph_text = " ".join(s["text"] for s in paragraph["sentences"])
            utterances.append(f"[{start_time}][Speaker:{paragraph['speaker']}] {paragraph_text}")

        transcript = "\n".join(utterances)
        return dict(text=transcript)


class HuggingfaceEmbeddingModel(HasModel):
    # TODO @Performance: move embedding/HF model into our own cluster
    async def _endpoint(self, text: Union[str, list[str]]) -> dict:
        # check and package text
        is_batched = isinstance(text, list)
        if not is_batched:
            text = [text]
        for i, t in enumerate(text):
            if not t:
                raise TaskError(TaskErrorType.InvalidFormat, self, f"empty text at {i}")

        # get embeddings
        async with aiohttp.ClientSession() as session:
            async with session.post(
                "https://a1cuxmqvoagqss78.eu-west-1.aws.endpoints.huggingface.cloud",
                headers={
                    "Authorization": f"Bearer {self._api_key}",
                    "Content-Type": "application/json",
                },
                json={"inputs": text},
            ) as response:
                rep = await response.json()
        if isinstance(rep, dict) and "error" in rep:
            if "gateway" in rep["error"].lower():
                error = TaskErrorType.TemporarilyUnavailable
            else:
                error = TaskErrorType.Unknown
            raise TaskError(error, self, rep["error"])
        elif not isinstance(rep, list):
            raise TaskError(TaskErrorType.Incapable, self, f"invalid response: {rep}")

        # quantize embeddings to [-128, 127] bytearray
        vector = np.array(rep, dtype=np.float32)
        vector = (vector * 128).clip(-128, 127).astype(np.int8)
        vector = [v.tolist() for v in vector]
        if not is_batched:
            vector = vector[0]

        return dict(vector=vector)
