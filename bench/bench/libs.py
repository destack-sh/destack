import enum
import typing
from typing import Any, Optional

import anthropic
from more_itertools import first
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
from bench.bench.type import Key, TypeBase, Vector, map_value, strip_py_value_flat
from bench.utils.utils import UnreachableError, omit_empty

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
    assistant = "assistant"
    user = "user"
    function = "function"


@x_struct("OpenAIChatMessage", file=_openai_chat)
class OpenAIChatMessage:
    role: OpenAIChatRole
    content: str
    name: Optional[str] = None
    function_call: Optional[str] = None

    def to_dict(self) -> dict[str, Any]:
        return dict(
            role=self.role,
            content=self.content,
            name=self.name,
            function_call=self.function_call,
        )


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
    name: Optional[str]
    type: OpenAIFunctionParameterType
    description: Optional[str] = None
    properties: Optional[list["OpenAIFunctionParameter"]] = None
    enum: Optional[list[str]] = None
    required: Optional[list[str]] = None

    def to_dict(self) -> dict[str, Any]:  # :ToDict
        # map properties to dict by name (Bench doesn't have a native map type yet)
        properties = (
            {p.name: OpenAIFunctionParameter.to_dict(p) for p in self.properties}
            if self.properties
            else None
        )
        return dict(
            name=self.name,
            type=self.type,
            description=self.description,
            properties=properties,
            enum=self.enum,
            required=self.required,
        )


@x_struct("OpenAIFunction", file=_openai_chat)
class OpenAIFunction:
    name: Key
    description: Optional[str]
    parameters: "OpenAIFunctionParameter"

    def to_dict(self) -> dict[str, Any]:  # :ToDict
        return dict(
            name=self.name,
            description=self.description,
            parameters=OpenAIFunctionParameter.to_dict(self.parameters),
        )


@x_struct("OpenAIFunctionCall", file=_openai_chat)
class OpenAIFunctionCall:
    name: Key
    arguments: dict[str, Any]


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


@x_model("gpt3", external_name="gpt-3.5-turbo", file=_openai_chat)
@x_model("gpt4", external_name="gpt-4", file=_openai_chat)
class OpenAIChatCompletionModel(Model):
    async def _endpoint(
        self,
        messages: list[OpenAIChatMessage],
        functions: list[OpenAIFunction],
        settings: OpenAIChatCompletionSettings,
    ) -> OpenAIChatCompletion:
        settings_raw = omit_empty(settings.to_dict())
        messages_raw = [omit_empty(OpenAIChatMessage.to_dict(m)) for m in messages]
        functions_raw = [omit_empty(OpenAIFunction.to_dict(f)) for f in functions]
        response = await openai.ChatCompletion.acreate(
            model=self.external_name,
            messages=messages_raw,
            functions=functions_raw,
            **settings_raw,
            api_key=self.key,
        )
        response_message = response["choices"][0]["message"]
        text = response_message.get("text")
        function_call = response_message.get("function_call")
        return OpenAIChatCompletion(
            text=text,
            function_call=OpenAIFunctionCall(
                name=function_call["name"],
                arguments=function_call["arguments"],
            ),
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
        content="You are a precise and helpful bot that interprets instructions intelligently."
        " You may think through (and spell out) intermediate steps as required or deemed necessary,"
        " but you must call a provided functions with the exact arguments specified.",
    )
    PANIC_FUNCTION = OpenAIFunction(
        name="panic",
        description="Error if the task is impossible or unreasonable given the instructions",
        parameters=OpenAIFunctionParameter(
            name=None,  # not needed for root object
            type=OpenAIFunctionParameterType.object,
            properties=[
                OpenAIFunctionParameter(
                    name="reason",
                    type=OpenAIFunctionParameterType.string,
                    description="The reason of incapability",
                    properties=None,
                    enum=None,
                )
            ],
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
            name=function.py_ident,
            description=function.description,
            parameters=self._compile_type(function, is_output=False),
        )

    def _compile_terminate_function(self) -> OpenAIFunction:
        return OpenAIFunction(
            name="terminate",
            description="Complete the task with the answer (if any).",
            parameters=self._compile_type(self.task, is_output=True),
        )

    def _compile_type(
        self, type: TypeBase, ignore_array: bool = False, is_output: bool = None
    ) -> OpenAIFunctionParameter:
        if is_output is None:
            fields = type.resolved_fields or type.fields
        else:
            fields = type.outputs if is_output else type.inputs
        if type.flags & TypeFlag.IsArray and not ignore_array:
            element_type = self._compile_type(type, ignore_array=True)
            element_type.name = None  # not needed for array element
            return OpenAIFunctionParameter(
                name=type.py_ident,
                type=OpenAIFunctionParameterType.array,
                description=type.description,
                properties=[element_type],
            )
        elif type.flags & TypeFlag.IsArrayable:
            raise NotImplementedError(f"unsupported type {type}: arrayable not yet supported")
        elif type.effective_tag == TypeTag.FUNCTION:
            return OpenAIFunctionParameter(
                name=None,
                type=OpenAIFunctionParameterType.object,
                description=type.description,
                properties=[self._compile_type(field) for field in fields],
                required=[
                    field.py_ident for field in fields if not (field.flags & TypeFlag.IsOptional)
                ],
            )
        elif type.effective_tag in TypeTag.STRUCT:
            return OpenAIFunctionParameter(
                name=type.py_ident,
                type=OpenAIFunctionParameterType.object,
                description=type.description,
                properties=[self._compile_type(field) for field in fields],
                required=[
                    field.py_ident for field in fields if not (field.flags & TypeFlag.IsOptional)
                ],
            )
        elif type.effective_tag == TypeTag.ENUM:
            return OpenAIFunctionParameter(
                name=type.py_ident,
                type=OpenAIFunctionParameterType.string,
                description=type.description,
                enum=[value.name for value in fields],
            )
        elif type.effective_tag in (TypeTag.STRING, TypeTag.NUMBER, TypeTag.BOOLEAN):
            return OpenAIFunctionParameter(
                name=type.py_ident,
                type=self.PARAM_TYPE_BY_TAG[type.effective_tag],
                description=type.description,
            )
        else:
            raise IncapableError(f"unsupported type {type}")

    async def run(self, model: OpenAIChatCompletionModel) -> TaskOutputFunction | TaskOutputResult:
        messages: list[OpenAIChatMessage] = [
            self.SYSTEM_MESSAGE,
            OpenAIChatMessage(
                role=OpenAIChatRole.system,
                content=f"Your task is '{self.task.name}': {self.task.description}."
                f"You will be given user inputs and you must call the most appropriate function.",
            ),
            OpenAIChatMessage(
                role=OpenAIChatRole.user,
                content=f"Inputs for '{self.task.name}': \n\n: {map_value(self.inputs, self.task, map_v=strip_py_value_flat)}",
            ),
            OpenAIChatMessage(
                role=OpenAIChatRole.system,
                content="Now, perform the task by calling a relevant function as instructed.",
            ),
        ]
        functions: list[OpenAIFunction] = [
            *(self._compile_function(function) for function in self.functions.values()),
            # include function to terminate with a result for overall task
            self._compile_terminate_function(),
            self.PANIC_FUNCTION,
        ]
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
            messages=messages, functions=functions, settings=settings
        )
        if completion.function_call is None:
            raise IncapableError("no function call returned")
        if completion.function_call == "panic":
            raise IncapableError(completion.text)
        elif completion.function_call == "terminate":
            return TaskOutputResult(result=completion.function_call.arguments)
        else:  # actual function call
            function = first(
                (f for f in self.functions.values() if f.py_ident == completion.function_call.name),
                None,
            )
            if function is None:
                raise IncapableError(f"unknown function {completion.function_call.name}")
            return TaskOutputFunction(
                function=function,
                inputs=completion.function_call.arguments,
            )


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
