import dataclasses
import enum
import typing
from typing import Any, Optional

import anthropic
import openai

from bench.bench.code_ import Code
from bench.bench.const import TypeFlag, TypeTag
from bench.bench.core import LookupBy, Module, Statement, parse_absolute_statement_reference
from bench.bench.model import Model
from bench.bench.reflect import (
    _model_compilers,
    _model_impls,
    _symbolx_reflect,
    x_enum,
    x_model,
    x_struct,
    x_task,
)
from bench.bench.remote import RemoteObject
from bench.bench.task import (
    IncapableError,
    Task,
    TaskCompiler,
    TaskOutputFunction,
    TaskOutputResult,
)
from bench.bench.type import Key, TypeBase, Vector, strip_py_value
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


@x_enum("OpenAIChatRole", file=_openai_chat)
class OpenAIChatRole(enum.StrEnum):
    system = "system"
    developer = "developer"
    assistant = "assistant"
    user = "user"
    function = "function"


@x_struct("OpenAIChatMessage", file=_openai_chat)
class OpenAIChatMessage:
    role: OpenAIChatRole
    content: str
    name: Optional[str] = None
    function_call: Optional[str] = None


@x_struct("OpenAIChatCompletionSettings", file=_openai_chat)
class OpenAIChatCompletionSettings:
    temperature: Optional[float]
    max_tokens: Optional[int] = None
    top_p: Optional[float] = None
    stop: Optional[str] = None
    logit_bias: Optional[dict[str, float]] = None
    frequence_penalty: Optional[float] = None
    presence_penalty: Optional[float] = None
    function_call: Optional[str] = None
    user: Optional[str] = None


@x_enum("OpenAIFunctionParameterType", file=_openai_chat)
class OpenAIFunctionParameterType(enum.StrEnum):
    string = "string"
    number = "number"
    boolean = "boolean"
    object = "object"
    array = "array"
    null = "null"


@x_struct("OpenAIFunctionParameter", file=_openai_chat)
class OpenAIFunctionParameter:
    type: OpenAIFunctionParameterType
    description: Optional[str] = None
    properties: Optional[dict[str, "OpenAIFunctionParameter"]] = None
    enum: Optional[list[str]] = None
    required: Optional[list[str]] = None


@x_struct("OpenAIFunction", file=_openai_chat)
class OpenAIFunction:
    name: Key
    description: str
    parameters: "OpenAIFunctionParameter"


@x_struct("OpenAIFunctionCall", file=_openai_chat)
class OpenAIFunctionCall:
    name: Key
    parameters: dict[str, Any]


@x_struct("OpenAITokenUsage", file=_openai_text)
class OpenAITokenUsage:
    prompt_tokens: int
    completion_tokens: Optional[int]
    total_tokens: int


@x_struct("OpenAIChatCompletion", file=_openai_chat)
class OpenAIChatCompletion:
    text: Optional[str]
    function_call: Optional[OpenAIFunctionCall]
    usage: OpenAITokenUsage


@x_model("gpt3", external_name="gpt3-5", file=_openai_chat)
@x_model("gpt4", external_name="gpt4", file=_openai_chat)
class OpenAIChatCompletionModel(Model):
    async def _endpoint(
        self,
        messages: list[OpenAIChatMessage],
        functions: dict[str, OpenAIFunction],
        settings: OpenAIChatCompletionSettings,
    ) -> OpenAIChatCompletion:
        response = await openai.ChatCompletion.acreate(
            model=self.external_name,
            messages=[dataclasses.asdict(m) for m in messages],
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

    def _compiler(self, task: "Task", inputs: dict, is_batched: bool) -> "TaskCompiler":
        return OpenAIChatCompiler(task, inputs, is_batched)


class OpenAIChatCompiler(TaskCompiler):
    SYSTEM_MESSAGE = OpenAIChatMessage(
        role=OpenAIChatRole.system,
        content="You are a precise and helpful bot that interprets instructions generously."
        " You may think through (and spell out) intermediate steps as required or deemed necessary,"
        " but you must call a provided functions with the exact arguments specified.",
    )
    PANIC_FUNCTION = OpenAIFunction(
        name="panic",
        description="Error if the task is impossible or unreasonable given the instructions",
        parameters=OpenAIFunctionParameter(
            type=OpenAIFunctionParameterType.object,
            properties={
                "reason": OpenAIFunctionParameter(
                    type=OpenAIFunctionParameterType.string,
                    description="The reason of incapability",
                    properties=None,
                    enum=None,
                )
            },
            required=["reason"],
        ),
    )
    PARAM_TYPE_BY_TAG = {
        TypeTag.STRING: OpenAIFunctionParameterType.string,
        TypeTag.NUMBER: OpenAIFunctionParameterType.number,
        TypeTag.BOOLEAN: OpenAIFunctionParameterType.boolean,
    }

    def _compile_function(self, function: Code | Task | Model) -> OpenAIFunction:
        return OpenAIFunction(
            name=function.name,
            description=function.description,
            parameters={field.name: self._compile_type(field) for field in function.inputs},
        )

    def _compile_terminate_function(self) -> OpenAIFunction:
        return OpenAIFunction(
            name="terminate",
            description="Complete the task with the answer (if any).",
            parameters={field.name: self._compile_type(field) for field in self.task.outputs},
        )

    def _compile_type(self, type: TypeBase, ignore_array: bool = False) -> OpenAIFunctionParameter:
        fields = type.resolved_fields or type.fields
        if type.flags & TypeFlag.IsArray and not ignore_array:
            return OpenAIFunctionParameter(
                type=OpenAIFunctionParameterType.array,
                description=type.description,
                properties=self._compile_type(type, ignore_array=True),
            )
        elif type.flags & TypeFlag.IsArrayable:
            raise NotImplementedError(f"unsupported type {type}: arrayable not yet supported")
        elif type.effective_tag == TypeTag.STRUCT:
            return OpenAIFunctionParameter(
                type=OpenAIFunctionParameterType.object,
                description=type.description,
                properties={field.name: self._compile_type(field) for field in fields},
                required=[
                    field.name for field in fields if not (field.flags & TypeFlag.IsNullable)
                ],
            )
        elif type.effective_tag == TypeTag.ENUM:
            return OpenAIFunctionParameter(
                type=OpenAIFunctionParameterType.string,
                description=type.description,
                enum=[value.name for value in fields],
            )
        elif type.effective_tag in (TypeTag.STRING, TypeTag.NUMBER, TypeTag.BOOLEAN):
            return OpenAIFunctionParameter(
                type=self.PARAM_TYPE_BY_TAG[type.effective_tag],
                description=type.description,
            )
        else:
            raise IncapableError(f"unsupported type {type}")

    async def run(self, model: OpenAIChatCompletionModel) -> TaskOutputFunction | TaskOutputResult:
        messages: list[OpenAIChatMessage] = [
            self.SYSTEM_MESSAGE,
            OpenAIChatMessage(
                role=OpenAIChatRole.developer,
                content=f"Your task is {self.task.name}: {self.task.description}."
                f"You will be given user inputs and you must call the most appropriate function.",
            ),
            OpenAIChatMessage(
                role=OpenAIChatRole.user,
                content=f"User inputs to the task {self.task.name}: \n\n: {strip_py_value(self.inputs, self.task)}",
            ),
            OpenAIChatMessage(
                role=OpenAIChatRole.developer,
                content="Now, begin and perform the task by calling a function as instructed.",
            ),
        ]
        functions: dict[str, OpenAIFunction] = {
            name: self._compile_function(function) for name, function in self.functions.items()
        }
        # include function to terminate with a result for overall task
        functions["terminate"] = self._compile_terminate_function()
        functions["panic"] = self.PANIC_FUNCTION
        settings = OpenAIChatCompletionSettings(
            temperature=0.8,
            max_tokens=None,
            top_p=None,
            stop=None,
            logit_bias=None,
            frequence_penalty=None,
            presence_penalty=None,
            function_call="auto",
            user=None,
        )

        completion: OpenAIChatCompletion = await model(
            messsages=messages,
            functions=functions,
            settings=settings,
        )
        if completion.function_call is not None:
            if completion.function_call == "panic":
                raise IncapableError(completion.text)
            elif completion.function_call == "terminate":
                return TaskOutputResult(result=completion.function_call.parameters)
            else:  # actual function call
                return TaskOutputFunction(
                    function=self.functions[completion.function_call.name],
                    inputs=completion.function_call.parameters,
                )
        else:
            raise IncapableError("no function call returned")


@x_struct("OpenAITextEmbeddingResponse", file=_openai_text)
class OpenAITextEmbeddingResponse:
    vector: typing.Union[Vector, list[Vector]]
    usage: OpenAITokenUsage


@x_model("ada", external_name="text-embedding-ada-002", file=_openai_text)
class OpenAITextEmbeddingModel(Model):
    async def _endpoint(self, text: typing.Union[str, list[str]]) -> OpenAITextEmbeddingResponse:
        is_batched = isinstance(text, list)
        if not is_batched:
            text = [text]
        rep = await openai.Embedding.acreate(
            input=text, model=self.external_name, api_key=self._key
        )
        if not is_batched:
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


@x_struct("OpenAIAudioTranscriptionResponse", file=_openai_text)
class OpenAIAudioTranscriptionResponse:
    text: str


@x_model("whisper", external_name="whisper", file=_openai_audio)
class OpenAIAudioTranscriptionModel(Model):
    async def _endpoint(self, audio: RemoteObject) -> OpenAIAudioTranscriptionResponse:
        raise NotImplementedError


anthropic_lib = Module(name="anthropic.lib")
_anthropic_text = anthropic_lib.create_file("text")


@x_struct("AnthropicTextCompletionSettings", file=_anthropic_text)
class AnthropicTextCompletionSettings:
    temperature: float
    top_p: Optional[float]
    top_k: Optional[int]
    max_tokens_to_sample: int
    stop_sequences: Optional[list[str]]


@x_struct("AnthropicTextCompletion", file=_anthropic_text)
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
DEFAULT_MODULES_IDS = {module.id for module in DEFAULT_MODULES.values()}

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


def lookup_model_compiler(path: str) -> Optional[typing.Callable]:
    return _model_compilers.get(path)
