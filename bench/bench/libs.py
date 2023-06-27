import functools
from typing import Optional, Any
import enum

from bench.bench.const import TypeTag
from bench.bench.core import Module, File
from bench.bench.model import Model
from bench.bench.remote import RemoteObject
from bench.bench.type import type_from_py_type


def _type(file: File, name: str, tag: TypeTag):
    def decorator(cls):
        bench_type = type_from_py_type(cls, name=name, root=tag)
        file.append(bench_type)
        return cls

    return decorator


def _task(file: File, name: str):
    def decorator(fn):
        task = Task(fn, name=name)
        file.append(task)
        return task

    return decorator


def _model(file: File, names: list[str]):
    def decorator(cls):
        statement = Model(cls, names=names)
        for name in names:
            file.append(statement)
        return cls

    return decorator


_struct = functools.partial(_type, tag=TypeTag.STRUCT)
_enum = functools.partial(_type, tag=TypeTag.ENUM)

symbolx = Module("symbolx.std")
symbolx_builtins = symbolx.create_file("builtin")


@_task(symbolx_builtins, "embed")
def embed(text: str | list[str]) -> list[float] | list[list[float]]:
    raise NotImplementedError


@_task(symbolx_builtins, "transcribe")
def transcribe(audio: RemoteObject) -> str:
    raise NotImplementedError


SYMBOLX_STD_BUILTINS: set[str] = {symbol.name for symbol in symbolx_builtins.symbols_by_id.values()}

openai = Module("openai.std")
openai_chat = openai.create_file("chat")
openai_text = openai.create_file("text")
openai_audio = openai.create_file("audio")


@_enum(openai_chat, "ChatRole")
class OpenAIChatRole(enum.StrEnum):
    system = "system"
    developer = "developer"
    assistant = "assistant"
    user = "user"
    function = "function"


@_struct(openai_chat, "ChatMessage")
class OpenAIChatMessage:
    role: OpenAIChatRole
    content: str


@_struct(openai_chat, "ChatCompletionSettings")
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


@_struct(openai_chat, "Function")
class OpenAIFunction:
    name: str
    description: str
    parameters: "OpenAIFunctionParameter"


@_struct(openai_chat, "FunctionParameter")
class OpenAIFunctionParameter:
    type: str
    description: str
    properties: dict[str, "OpenAIFunctionParameter"] | None = None
    enum: list[str] | None = None
    required: list[str] | None = None


@_struct(openai_chat, "FunctionCall")
class OpenAIFunctionCall:
    name: str
    parameters: dict[str, Any]


@_struct(openai_text, "TokenUsage")
class OpenAITokenUsage:
    prompt_tokens: int
    completion_tokens: Optional[int]
    total_tokens: int


@_struct(openai_chat, "ChatCompletion")
class OpenAIChatCompletion:
    text: Optional[str]
    function_call: Optional[OpenAIFunctionCall]
    usage: OpenAITokenUsage


@_model(openai_chat, ["gpt3", "gpt4"])
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


@_struct("TextEmbeddingResponse")
class OpenAITextEmbeddingResponse:
    embedding: list[float] | list[list[float]]
    usage: OpenAITokenUsage


@_model(openai_text, ["ada"])
class OpenAITextEmbeddingModel(Model):
    async def _endpoint(self, text: str | list[str]) -> OpenAITextEmbeddingResponse:
        rep = await openai.Embedding.acreate(text, model=self.model, api_key=self.key)
        if text is not None:
            embedding = rep["data"][0]["embedding"]
        else:
            embedding = [d["embedding"] for d in rep["data"]]
        return OpenAITextEmbeddingResponse(
            embedding=embedding,
            usage=OpenAITokenUsage(
                prompt_tokens=rep["usage"]["prompt_tokens"],
                completion_tokens=rep["usage"].get("completion_tokens"),
                total_tokens=rep["usage"]["total_tokens"],
            ),
        )


@_model(["whisper"])
class OpenAIAudioTranscriptionModel(Model):
    async def _endpoint(self, audio: RemoteObject) -> str:
        raise NotImplementedError


anthropic = Module("anthropic.std")
anthropic_text = anthropic.create_file("text")


@_struct("TextCompletionSettings")
class AnthropicTextCompletionSettings:
    temperature: float = 1.0
    top_p: Optional[float] = None
    top_k: Optional[int] = None
    max_tokens_to_sample: int = 64
    stop_sequences: list[str] | None = None


@_struct("TextCompletion")
class AnthropicTextCompletion:
    completion: str
    stop_reason: str


@_model(anthropic_text, ["claude-1", "claude-1-100k", "clause-instant-1", "clause-instant-1-100k"])
class AnthropicTextCompletionModel(Model):
    _client: anthropic.Client | None = None

    def _clear(self) -> None:
        self._client = None

    async def _endpoint(
        self, prompt: str, settings: AnthropicTextCompletionSettings
    ) -> AnthropicTextCompletion:
        if self._client is None:
            self.client = anthropic.Client(self.key)

            # monkey patch Anthropic's validation (which is broken)
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
    "symbolx.std": symbolx,
    "openai.std": openai,
    "anthropic.std": anthropic,
}
