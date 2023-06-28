import enum
import functools
import typing
from dataclasses import dataclass
from typing import Any, Optional

import anthropic
import openai

from bench.bench.const import TypeTag
from bench.bench.core import File, Module
from bench.bench.model import Model
from bench.bench.remote import RemoteObject
from bench.bench.task import Task
from bench.bench.type import Key, Vector, type_from_py_type
from bench.utils.utils import UnreachableError


def _type(file: File, name: str, tag: TypeTag):
    def decorator(cls):
        bench_type = type_from_py_type(cls, name=name)
        if bench_type.tag != tag:
            raise TypeError(f"Expected {tag}, got {bench_type.tag}")
        file.append(bench_type)
        return cls

    return decorator


def _struct(file: File, name: str):
    def decorator(cls):
        cls = dataclass(cls)
        bench_type = type_from_py_type(cls, name=name)
        if bench_type.tag != TypeTag.STRUCT:
            raise TypeError(f"Expected {TypeTag.STRUCT}, got {bench_type.tag}")
        file.append(bench_type)
        return cls

    return decorator


_enum = functools.partial(_type, tag=TypeTag.ENUM)


def _task(file: File, name: str):
    def decorator(fn):
        task = Task(name=name)
        task_type = type_from_py_type(fn, name=None)
        task.fields = task_type._copy_fields(to=task)
        file.append(task)
        return task

    return decorator


def _model(file: File, name: str, external_name: str):
    def decorator(cls):
        model = Model(name=name, external_name=external_name)
        model_type = type_from_py_type(cls._endpoint, name=None)
        model.fields = model_type._copy_fields(to=model)
        file.append(model)
        return cls

    return decorator


symbolx_lib = Module(name="symbolx.lib")
_symbolx_builtins = symbolx_lib.create_file("builtins")

EmbeddingOutput = typing.TypedDict(
    "EmbeddingOutput", {"vector": typing.Union[Vector, list[Vector]]}
)


@_task(_symbolx_builtins, "embed")
def embed(text: typing.Union[str, list[str]]) -> EmbeddingOutput:
    raise UnreachableError()  # stub


TranscriptionOutput = typing.TypedDict("TranscriptionOutput", {"text": str})


@_task(_symbolx_builtins, "transcribe")
def transcribe(audio: RemoteObject) -> TranscriptionOutput:
    raise UnreachableError()  # stub


openai_lib = Module(name="openai.lib")
_openai_chat = openai_lib.create_file("chat")
_openai_text = openai_lib.create_file("text")
_openai_audio = openai_lib.create_file("audio")


@_enum(_openai_chat, "ChatRole")
class OpenAIChatRole(enum.StrEnum):
    system = "system"
    developer = "developer"
    assistant = "assistant"
    user = "user"
    function = "function"


@_struct(_openai_chat, "ChatMessage")
class OpenAIChatMessage:
    role: OpenAIChatRole
    content: str


@_struct(_openai_chat, "ChatCompletionSettings")
class OpenAIChatCompletionSettings:
    temperature: float = 1.0
    max_tokens: int = None
    top_p: float = 1.0
    stop: Optional[str] = None
    logit_bias: Optional[dict[str, float]] = None
    frequence_penalty: float = 0.0
    presence_penalty: float = 0.0
    function_call: Optional[str] = None
    user: Optional[str] = None


@_struct(_openai_chat, "FunctionParameter")
class OpenAIFunctionParameter:
    type: Key
    description: str
    properties: Optional[dict[str, "OpenAIFunctionParameter"]] = None
    enum: Optional[list[str]] = None
    required: Optional[list[str]] = None


@_struct(_openai_chat, "Function")
class OpenAIFunction:
    name: Key
    description: str
    parameters: "OpenAIFunctionParameter"


@_struct(_openai_chat, "FunctionCall")
class OpenAIFunctionCall:
    name: Key
    parameters: dict[str, Any]


@_struct(_openai_text, "TokenUsage")
class OpenAITokenUsage:
    prompt_tokens: int
    completion_tokens: Optional[int]
    total_tokens: int


@_struct(_openai_chat, "ChatCompletion")
class OpenAIChatCompletion:
    text: Optional[str]
    function_call: Optional[OpenAIFunctionCall]
    usage: OpenAITokenUsage


@_model(_openai_chat, "gpt3", "gpt3-5")
@_model(_openai_chat, "gpt4", "gpt4")
class OpenAIChatCompletionModel(Model):
    async def _endpoint(
        self,
        messages: list[OpenAIChatMessage],
        functions: dict[str, OpenAIFunction],
        settings: OpenAIChatCompletionSettings,
    ) -> OpenAIChatCompletion:
        response = await openai.ChatCompletion.acreate(
            model=self.model,
            messages=[asdict(m) for m in messages],
            temperature=settings.temperature,
            max_tokens=settings.max_tokens,
            top_p=settings.top_p,
            stop=settings.stop or None,
            logit_bias=settings.logit_bias,
            api_key=self.key,
        )
        response_message = response["choices"][0]["message"]
        text = response_message.get("text")
        function_call = response_message.get("function_call")
        return OpenAIChatCompletion(
            text=text,
            function_call=function_call,
            usage=OpenAITokenUsage(
                prompt_tokens=response["usage"]["prompt_tokens"],
                completion_tokens=response["usage"].get("completion_tokens"),
                total_tokens=response["usage"]["total_tokens"],
            ),
        )


@_struct(_openai_text, "TextEmbeddingResponse")
class OpenAITextEmbeddingResponse:
    vector: typing.Union[Vector, list[Vector]]
    usage: OpenAITokenUsage


@_model(_openai_text, "ada", "text-embedding-ada-002")
class OpenAITextEmbeddingModel(Model):
    async def _endpoint(self, text: typing.Union[str, list[str]]) -> OpenAITextEmbeddingResponse:
        rep = await openai.Embedding.acreate(text, model=self.model, api_key=self.key)
        if text is not None:
            vector = rep["data"][0]["embedding"]
        else:
            vector = [d["embedding"] for d in rep["data"]]
        return OpenAITextEmbeddingResponse(
            vector=vector,
            usage=OpenAITokenUsage(
                prompt_tokens=rep["usage"]["prompt_tokens"],
                completion_tokens=rep["usage"].get("completion_tokens"),
                total_tokens=rep["usage"]["total_tokens"],
            ),
        )


@_struct(_openai_text, "AudioTranscriptionResponse")
class OpenAIAudioTranscriptionResponse:
    text: str


@_model(_openai_audio, "whisper", "whisper")
class OpenAIAudioTranscriptionModel(Model):
    async def _endpoint(self, audio: RemoteObject) -> OpenAIAudioTranscriptionResponse:
        raise NotImplementedError


anthropic_lib = Module(name="anthropic.lib")
_anthropic_text = anthropic_lib.create_file("text")


@_struct(_openai_text, "TextCompletionSettings")
class AnthropicTextCompletionSettings:
    temperature: float = 1.0
    top_p: Optional[float] = None
    top_k: Optional[int] = None
    max_tokens_to_sample: int = 64
    stop_sequences: Optional[list[str]] = None


@_struct(_openai_text, "TextCompletion")
class AnthropicTextCompletion:
    completion: str
    stop_reason: str


@_model(_anthropic_text, "claude-1", "claude-1")
@_model(_anthropic_text, "claude-1-100k", "claude-1-100k")
@_model(_anthropic_text, "claude-instant-1", "claude-instant-1")
@_model(_anthropic_text, "claude-instant-1-100k", "claude-instant-1-100k")
class AnthropicTextCompletionModel(Model):
    _client: anthropic.Client | None = None

    def _clear(self) -> None:
        self._client = None

    async def _endpoint(
        self, prompt: str, settings: AnthropicTextCompletionSettings
    ) -> AnthropicTextCompletion:
        if self._client is None:
            self.client = anthropic.Client(self.key)

            # monkey patch Anthropic validation (which is broken)
            from anthropic import api

            api._validate_prompt_length = lambda *args, **kwargs: None

        # see https://console.anthropic.com/docs/api
        rep = await self.client.acompletion(
            prompt=prompt,
            model=self.name,
            stop_sequences=[anthropic.HUMAN_PROMPT, *(settings.stop or [])],
            temperature=settings.temperature,
            max_tokens_to_sample=settings.max_tokens_to_sample,
            top_p=settings.top_p,
        )
        return rep["completion"]


DEFAULT_MODULES: dict[str, Module] = {
    "symbolx.lib": symbolx_lib,
    "openai.lib": openai_lib,
    "anthropic.lib": anthropic_lib,
}

# interp/index them
for name, module in DEFAULT_MODULES.items():
    assert module.name == name
    module.index()
    module.interp()
    if module.issues:
        raise RuntimeError(f"default module {module.name} has issues: {module.issues}")
