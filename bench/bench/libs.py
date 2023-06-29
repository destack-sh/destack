import enum
import typing
from typing import Any, Optional

import anthropic
import openai

from bench.bench.core import LookupBy, Module, Statement, parse_absolute_statement_reference
from bench.bench.model import Model
from bench.bench.reflect import _model_impls, _symbolx_reflect, x_enum, x_model, x_struct, x_task
from bench.bench.remote import RemoteObject
from bench.bench.type import Key, Vector
from bench.utils.utils import UnreachableError

symbolx_lib = Module(name="symbolx.lib")
_symbolx_builtins = symbolx_lib.create_file("builtins")
symbolx_lib.add_file(_symbolx_reflect)


@x_struct("EmbeddingOutput", file=_symbolx_builtins)
class EmbeddingOutput:
    vector: typing.Union[Vector, list[Vector]]


@x_task("embed", file=_symbolx_builtins)
def embed(text: typing.Union[str, list[str]]) -> EmbeddingOutput:
    raise UnreachableError()  # stub


@x_struct("TranscriptionOutput", file=_symbolx_builtins)
class TranscriptionOutput:
    text: str


@x_task("transcribe", file=_symbolx_builtins)
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


@x_enum("ChatRole", file=_openai_chat)
class OpenAIChatRole(enum.StrEnum):
    system = "system"
    developer = "developer"
    assistant = "assistant"
    user = "user"
    function = "function"


@x_struct("ChatMessage", file=_openai_chat)
class OpenAIChatMessage:
    role: OpenAIChatRole
    content: str


@x_struct("ChatCompletionSettings", file=_openai_chat)
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


@x_struct("FunctionParameter", file=_openai_chat)
class OpenAIFunctionParameter:
    type: Key
    description: str
    properties: Optional[dict[str, "OpenAIFunctionParameter"]]
    enum: Optional[list[str]]
    required: Optional[list[str]]


@x_struct("Function", file=_openai_chat)
class OpenAIFunction:
    name: Key
    description: str
    parameters: "OpenAIFunctionParameter"


@x_struct("FunctionCall", file=_openai_chat)
class OpenAIFunctionCall:
    name: Key
    parameters: dict[str, Any]


@x_struct("TokenUsage", file=_openai_text)
class OpenAITokenUsage:
    prompt_tokens: int
    completion_tokens: Optional[int]
    total_tokens: int


@x_struct("ChatCompletion", file=_openai_chat)
class OpenAIChatCompletion:
    text: Optional[str]
    function_call: Optional[OpenAIFunctionCall]
    usage: OpenAITokenUsage


@x_model("gpt3", external_name="gpt3-5", file=_openai_chat)
@x_model("gpt4", external_name="gpt4", file=_openai_chat)
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


@x_struct("TextEmbeddingResponse", file=_openai_text)
class OpenAITextEmbeddingResponse:
    vector: typing.Union[Vector, list[Vector]]
    usage: OpenAITokenUsage


@x_model("ada", external_name="text-embedding-ada-002", file=_openai_text)
class OpenAITextEmbeddingModel(Model):
    async def _impl(self, text: typing.Union[str, list[str]]) -> OpenAITextEmbeddingResponse:
        was_list = isinstance(text, list)
        if not was_list:
            text = [text]
        rep = await openai.Embedding.acreate(
            input=text, model=self.external_name, api_key=self._key
        )
        if not was_list:
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


@x_struct("AudioTranscriptionResponse", file=_openai_text)
class OpenAIAudioTranscriptionResponse:
    text: str


@x_model("whisper", external_name="whisper", file=_openai_audio)
class OpenAIAudioTranscriptionModel(Model):
    async def _impl(self, audio: RemoteObject) -> OpenAIAudioTranscriptionResponse:
        raise NotImplementedError


anthropic_lib = Module(name="anthropic.lib")
_anthropic_text = anthropic_lib.create_file("text")


@x_struct("TextCompletionSettings", file=_anthropic_text)
class AnthropicTextCompletionSettings:
    temperature: float
    top_p: Optional[float]
    top_k: Optional[int]
    max_tokens_to_sample: int
    stop_sequences: Optional[list[str]]


@x_struct("TextCompletion", file=_anthropic_text)
class AnthropicTextCompletion:
    completion: str
    stop_reason: str


@x_model("claude-1", external_name="claude-1", file=_anthropic_text)
@x_model("claude-1-100k", external_name="claude-1-100k", file=_anthropic_text)
@x_model("claude-instant-1", external_name="claude-instant-1", file=_anthropic_text)
@x_model("claude-instant-1-100k", external_name="claude-instant-1-100k", file=_anthropic_text)
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
