import enum
import functools
import typing
from dataclasses import dataclass
from typing import Any, Optional

import anthropic
import openai

from bench.bench.const import TypeTag
from bench.bench.core import File, LookupBy, Module, Statement, parse_absolute_statement_reference
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
        # turn it into dataclass that behaves like a dict
        # it needs to be a dataclass for getattr and getitem access
        # and it needs to be a real distinct class so we can type map references to it properly
        cls = dataclass(cls)
        cls.__getitem__ = lambda self, key: getattr(self, key, None)
        cls.__setitem__ = lambda self, key, value: setattr(self, key, value)
        cls.__contains__ = lambda self, key: hasattr(self, key) and getattr(self, key) is not None
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


_model_impls: dict[str, typing.Callable] = {}


def _model(file: File, name: str, external_name: str):
    def decorator(cls):
        model = Model(name=name, external_name=external_name)
        model_type = type_from_py_type(cls._impl, name=None)
        model.fields = model_type._copy_fields(to=model)
        file.append(model)
        _model_impls[model.path] = cls._impl
        return cls

    return decorator


symbolx_lib = Module(name="symbolx.lib")
_symbolx_builtins = symbolx_lib.create_file("builtins")


@_struct(_symbolx_builtins, "EmbeddingOutput")
class EmbeddingOutput:
    vector: typing.Union[Vector, list[Vector]]


@_task(_symbolx_builtins, "embed")
def embed(text: typing.Union[str, list[str]]) -> EmbeddingOutput:
    raise UnreachableError()  # stub


@_struct(_symbolx_builtins, "TranscriptionOutput")
class TranscriptionOutput:
    text: str


@_task(_symbolx_builtins, "transcribe")
def transcribe(audio: RemoteObject) -> TranscriptionOutput:
    raise UnreachableError()  # stub


# Note that beyond the symbolx standard lib, all other libs should later
# be defined and update in Bench itself. That may also happen via code or some other
# automatic mechanism, it just shouldn't be here.
# The model implementations should be just like Code implementations,
# so we don't need to hot-swap in 'impl' when calling. :LibImplementation

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
    temperature: Optional[float]
    max_tokens: Optional[int]
    top_p: Optional[float]
    stop: Optional[str]
    logit_bias: Optional[dict[str, float]]
    frequence_penalty: Optional[float]
    presence_penalty: Optional[float]
    function_call: Optional[str]
    user: Optional[str]


@_struct(_openai_chat, "FunctionParameter")
class OpenAIFunctionParameter:
    type: Key
    description: str
    properties: Optional[dict[str, "OpenAIFunctionParameter"]]
    enum: Optional[list[str]]
    required: Optional[list[str]]


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
    async def _impl(
        self,
        messages: list[OpenAIChatMessage],
        functions: dict[str, OpenAIFunction],
        settings: OpenAIChatCompletionSettings,
    ) -> OpenAIChatCompletion:
        response = await openai.ChatCompletion.acreate(
            model=self.external_name,
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
    async def _impl(self, text: typing.Union[str, list[str]]) -> OpenAITextEmbeddingResponse:
        rep = await openai.Embedding.acreate(
            input=text, model=self.external_name, api_key=self._key
        )
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
    async def _impl(self, audio: RemoteObject) -> OpenAIAudioTranscriptionResponse:
        raise NotImplementedError


anthropic_lib = Module(name="anthropic.lib")
_anthropic_text = anthropic_lib.create_file("text")


@_struct(_anthropic_text, "TextCompletionSettings")
class AnthropicTextCompletionSettings:
    temperature: float
    top_p: Optional[float]
    top_k: Optional[int]
    max_tokens_to_sample: int
    stop_sequences: Optional[list[str]]


@_struct(_anthropic_text, "TextCompletion")
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

    async def _impl(
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
            model=self.external_name,
            stop_sequences=[anthropic.HUMAN_PROMPT, *(settings.stop or [])],
            temperature=settings.temperature,
            max_tokens_to_sample=settings.max_tokens_to_sample,
            top_p=settings.top_p,
        )
        return AnthropicTextCompletion(completion=rep["completion"], stop_reason=rep["stop_reason"])


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


def lookup(path: str, by: LookupBy = LookupBy.Name) -> Optional[Statement]:
    module_name, local_path = parse_absolute_statement_reference(path)
    module = DEFAULT_MODULES.get(module_name)
    if module is None:
        return None
    return module.lookup(local_path, by=by)


def lookup_model_impl(path: str) -> Optional[typing.Callable]:
    return _model_impls.get(path)
