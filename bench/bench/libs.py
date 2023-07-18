import enum
import json
import typing
from json import JSONDecodeError
from typing import Any, Optional

import anthropic
import openai

from bench.bench.basic import Expectation
from bench.bench.code_ import Code
from bench.bench.const import TypeFlag, TypeTag
from bench.bench.core import LookupBy, Module, Statement, parse_absolute_statement_reference
from bench.bench.model import Model
from bench.bench.reflect import (
    _model_compilers,
    _model_impls,
    _symbolx_reflect,
    _versioned_id,
    x_enum,
    x_model,
    x_struct,
    x_tag,
    x_task,
)
from bench.bench.remote import RemoteObject
from bench.bench.task import (
    IncapableError,
    Task,
    TaskCompiler,
    TaskError,
    TaskErrorType,
    TaskRunner,
)
from bench.bench.type import Key, TypeBase, Vector, check_type, map_value, strip_py_value_flat
from bench.utils.utils import UnreachableError, omit_empty

symbolx_lib = Module(name="symbolx.lib")
_symbolx_builtins = symbolx_lib.create_file("builtins")
_symbolx_utils = symbolx_lib.create_file("utils")
symbolx_lib.add_file(_symbolx_reflect)


@x_tag("tool", "Mark as a tool", file=_symbolx_builtins)
class Tool:
    pass


@x_tag("step", "Mark as a required step", file=_symbolx_builtins)
class Step:
    pass


@x_tag("consider", "Mark as something to consider", file=_symbolx_builtins)
class Consider:
    pass


@x_tag("check", "Mark as a mandatory check", file=_symbolx_builtins)
class Check:
    pass


@x_tag("retry", "Auto-retry runs", file=_symbolx_builtins)
class Retry:  # like tenacity but maybe with autoheal?
    pass


@x_tag("cache", "Auto-cache runs", file=_symbolx_builtins)
class Cache:  # like cachetools
    pass


@x_tag("confirm", "Prompt for confirmation before running", file=_symbolx_builtins)
class Confirm:
    pass


@x_tag("autoheal", "Auto-heal on error", file=_symbolx_builtins)
class Autoheal:
    pass


@x_struct("EmbeddingOutput", "Embedding output", file=_symbolx_builtins)
class EmbeddingOutput:
    vector: typing.Union[Vector, list[Vector]]


@x_task("embed", "Embed any text into a vector", file=_symbolx_builtins)
def embed(text: typing.Union[str, list[str]]) -> EmbeddingOutput:
    raise UnreachableError()  # stub


@x_struct("TranscriptionOutput", "Transcription output", file=_symbolx_builtins)
class TranscriptionOutput:
    text: str


@x_task("transcribe", "Transcribe any audio to text", file=_symbolx_builtins)
def transcribe(audio: RemoteObject) -> TranscriptionOutput:
    raise UnreachableError()  # stub


@x_enum("JsonSchemaElementType", "The type of a JSON Schema element", file=_symbolx_utils)
class JsonSchemaElementType(enum.StrEnum):
    string = "string"
    number = "number"
    boolean = "boolean"
    object = "object"
    array = "array"
    null = "null"


@x_struct("JsonSchemaElement", "A JSON schema element (may be nested)", file=_symbolx_utils)
class JsonSchemaElement:
    name: Optional[str]
    type: JsonSchemaElementType
    description: Optional[str] = None
    properties: Optional[list["JsonSchemaElement"]] = None
    items: Optional["JsonSchemaElement"] = None
    enum: Optional[list[str]] = None
    required: Optional[list[str]] = None

    def to_dict(self) -> dict[str, Any]:  # :ToDict
        # map properties to dict by name (Bench doesn't have a native map type yet)
        properties = (
            {p.name: JsonSchemaElement.to_dict(p) for p in self.properties}
            if self.properties
            else None
        )
        # 'self' may be a DotDict here because we don't really handle custom type instances yet
        if isinstance(self, dict):
            items = self.get("items")
        else:
            items = self.items
        items = JsonSchemaElement.to_dict(items) if items else None
        return dict(
            name=self.name,
            type=self.type,
            description=self.description,
            properties=properties,
            items=items,
            enum=self.enum,
            required=self.required,
        )


PARAM_TYPE_BY_TAG = {
    TypeTag.STRING: JsonSchemaElementType.string,
    TypeTag.NUMBER: JsonSchemaElementType.number,
    TypeTag.BOOLEAN: JsonSchemaElementType.boolean,
}


def _type_to_json_schema(
    type: TypeBase, ignore_array: bool = False, is_output: bool = None
) -> JsonSchemaElement:
    """Convert a Bench type to a JSON schema element."""
    if is_output is None:
        fields = type.resolved_fields or type.fields
    else:
        fields = type.outputs if is_output else type.inputs
    if type.flags & TypeFlag.IsArray and not ignore_array:
        element_type = _type_to_json_schema(type, ignore_array=True)
        element_type.name = None  # not needed for array element
        return JsonSchemaElement(
            name=type.py_ident,
            type=JsonSchemaElementType.array,
            description=type.description,
            items=element_type,
        )
    elif type.flags & TypeFlag.IsArrayable:
        raise NotImplementedError(f"unsupported type {type}: arrayable not yet supported")
    elif type.effective_tag == TypeTag.FUNCTION:
        return JsonSchemaElement(
            name=None,
            type=JsonSchemaElementType.object,
            description=type.description,
            properties=[_type_to_json_schema(field) for field in fields],
            required=[
                field.py_ident for field in fields if not (field.flags & TypeFlag.IsOptional)
            ],
        )
    elif type.effective_tag in TypeTag.STRUCT:
        return JsonSchemaElement(
            name=type.py_ident,
            type=JsonSchemaElementType.object,
            description=type.description,
            properties=[_type_to_json_schema(field) for field in fields],
            required=[
                field.py_ident for field in fields if not (field.flags & TypeFlag.IsOptional)
            ],
        )
    elif type.effective_tag == TypeTag.ENUM:
        return JsonSchemaElement(
            name=type.py_ident,
            type=JsonSchemaElementType.string,
            description=type.description,
            enum=[value.name for value in fields],
        )
    elif type.effective_tag in (TypeTag.STRING, TypeTag.NUMBER, TypeTag.BOOLEAN):
        return JsonSchemaElement(
            name=type.py_ident,
            type=PARAM_TYPE_BY_TAG[type.effective_tag],
            description=type.description,
        )
    else:
        raise IncapableError(f"unsupported type {type}")


# Note that apart from the symbolx standard lib, all other libs should later
# be defined and update in Bench itself. That may also happen via code or some other
# automatic mechanism, it just shouldn't be here.
# The model implementations should be just like Code implementations,
# so we don't need to hot-swap in 'impl' when calling. :LibImplementation

openai_lib = Module(name="openai.lib")
openai_lib.add_dependency(symbolx_lib)
_openai_chat = openai_lib.create_file("chat")
_openai_text = openai_lib.create_file("text")
_openai_audio = openai_lib.create_file("audio")
_openai_utils = openai_lib.create_file("utils")


@x_enum("OpenAIChatRole", "Message role in OpenAI chat models", file=_openai_utils)
class OpenAIChatRole(enum.StrEnum):
    system = "system"
    assistant = "assistant"
    user = "user"
    function = "function"


@x_struct("OpenAIFunction", "Function in OpenAI chat models", file=_openai_chat)
class OpenAIFunction:
    name: Key
    description: Optional[str]
    parameters: "JsonSchemaElement"

    def to_dict(self) -> dict[str, Any]:  # :ToDict
        return dict(
            name=self.name,
            description=self.description,
            parameters=JsonSchemaElement.to_dict(self.parameters),
        )


@x_struct("OpenAIFunctionCall", "Requested function call in OpenAI chat models", file=_openai_chat)
class OpenAIFunctionCall:
    name: Key
    arguments: Optional[dict[str, Any]] = None

    def to_dict(self) -> dict[str, Any]:  # :ToDict
        return omit_empty(
            dict(
                name=self.name,
                arguments=self.arguments,
            )
        )


@x_struct("OpenAIChatMessage", "An individual message in OpenAI chat models", file=_openai_chat)
class OpenAIChatMessage:
    role: OpenAIChatRole
    content: Optional[str] = None
    name: Optional[str] = None
    function_call: Optional[OpenAIFunctionCall] = None

    def to_dict(self) -> dict[str, Any]:
        function_call = (
            OpenAIFunctionCall.to_dict(self.function_call) if self.function_call else None
        )
        # can't use omit_empty like usual here because content is always required, but name isn't?
        d = dict(role=self.role, content=self.content)
        if self.name is not None:
            d["name"] = self.name
        if function_call is not None:
            d["function_call"] = function_call
        return d


@x_struct("OpenAIChatSettings", "Inference settings for OpenAI chat models", file=_openai_chat)
class OpenAIChatSettings:
    temperature: Optional[float]
    max_tokens: Optional[int] = None
    top_p: Optional[float] = None
    stop: Optional[str] = None
    logit_bias: Optional[dict[str, float]] = None
    frequence_penalty: Optional[float] = None
    presence_penalty: Optional[float] = None
    # technically function call is a union of string and function call but we don't support that yet
    function_call: Optional[str] = None
    user: Optional[str] = None


@x_struct("OpenAITokenUsage", "Reported token usage for OpenAI text models", file=_openai_text)
class OpenAITokenUsage:
    prompt_tokens: int
    completion_tokens: Optional[int]
    total_tokens: int


@x_struct("OpenAIChatCompletion", "Completion from OpenAI chat models", file=_openai_chat)
class OpenAIChatCompletion:
    message: OpenAIChatMessage
    usage: OpenAITokenUsage


@x_model(
    "gpt3",
    "OpenAI's instruct-tuned 4k context GPT3.5 based chat model",
    external_name="gpt-3.5-turbo",
    file=_openai_chat,
)
@x_model(
    "gpt3-16k",
    "OpenAI's instruct-tuned 16k context GPT3.5 based chat model",
    external_name="gpt-3.5-turbo-16k",
    file=_openai_chat,
)
@x_model(
    "gpt4",
    "OpenAI's latest and largest 8k context GPT4 based chat model",
    external_name="gpt-4",
    file=_openai_chat,
)
@x_model(
    "gpt4-32k",
    "OpenAI's latest and largest 8k context GPT4 based chat model",
    external_name="gpt-4-32k",
    file=_openai_chat,
)
class OpenAIChatCompletionModel(Model):
    async def _endpoint(
        self,
        messages: list[OpenAIChatMessage],
        functions: list[OpenAIFunction],
        settings: OpenAIChatSettings,
    ) -> OpenAIChatCompletion:
        settings_raw = omit_empty(settings.to_dict())
        messages_raw = [(OpenAIChatMessage.to_dict(m)) for m in messages]
        functions_raw = [omit_empty(OpenAIFunction.to_dict(f)) for f in functions]
        response = await openai.ChatCompletion.acreate(
            model=self.external_name,
            messages=messages_raw,
            functions=functions_raw,
            **settings_raw,
            api_key=self._api_key,
        )
        message = response["choices"][0]["message"]
        if "function_call" in message:
            function_call = OpenAIFunctionCall(
                name=message["function_call"]["name"],
                arguments=message["function_call"]["arguments"],
            )
        else:
            function_call = None
        return OpenAIChatCompletion(
            message=OpenAIChatMessage(
                role=message["role"],
                content=message["content"],
                name=message.get("name"),
                function_call=function_call,
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
        description="Error if no reasonable termination is possible given the instructions."
        " Strongly prefer calling 'terminate' with the relevant error info instead.",
        parameters=JsonSchemaElement(
            name=None,  # not needed for root object
            type=JsonSchemaElementType.object,
            properties=[
                JsonSchemaElement(
                    name="reason",
                    type=JsonSchemaElementType.string,
                    description="The reason of incapability",
                    properties=None,
                    enum=None,
                )
            ],
            required=["reason"],
        ),
    )

    def _compile_function(self, function: Code | Task | Model) -> OpenAIFunction:
        return OpenAIFunction(
            name=function.py_ident,
            description=function.description,
            parameters=_type_to_json_schema(function, is_output=False),
        )

    def _compile_terminate_function(self) -> OpenAIFunction:
        return OpenAIFunction(
            name="terminate",
            description="Complete the task with an answer (if any).",
            parameters=_type_to_json_schema(self.task, is_output=True),
        )

    def _compile_expectation(self, expectation: Expectation) -> OpenAIChatMessage:
        return OpenAIChatMessage(
            role=OpenAIChatRole.system,
            content=f"Expectation '{expectation.name}': {expectation.description}",
        )

    def _compile_error(self, error: TaskError) -> OpenAIChatMessage:
        return OpenAIChatMessage(
            role=OpenAIChatRole.system,
            content=f"Avoid previous error: {error}",
        )

    def _compile_function_result(
        self, function: Code | Task | Model, result: dict | TaskError
    ) -> OpenAIChatMessage:
        if isinstance(result, TaskError):
            return OpenAIChatMessage(
                role=OpenAIChatRole.system,
                content=f"Avoid Previous error: {result}",
            )
        else:
            return OpenAIChatMessage(
                role=OpenAIChatRole.function,
                name=function.py_ident,
                content=json.dumps(map_value(result, function, map_v=strip_py_value_flat)),
            )

    async def run(self, model: OpenAIChatCompletionModel, runner: TaskRunner) -> dict:
        messages: list[OpenAIChatMessage] = [
            self.SYSTEM_MESSAGE,
            OpenAIChatMessage(
                role=OpenAIChatRole.system,
                content=f"Your task is '{self.task.name}': {self.task.description}."
                f"You will be given user inputs and you must call the most appropriate function.",
            ),
            *(self._compile_expectation(expectation) for expectation in self.expectations),
            OpenAIChatMessage(
                role=OpenAIChatRole.user,
                content=f"Inputs for '{self.task.name}': \n\n: {map_value(self.inputs, self.task, map_k=lambda f: (f.py_ident, f.py_ident), map_v=strip_py_value_flat)}",
            ),
            OpenAIChatMessage(
                role=OpenAIChatRole.system,
                content=f"Now, perform the task '{self.task.name}' using the inputs as needed and call a relevant function as instructed.",
            ),
        ]
        functions: list[OpenAIFunction] = [
            *(self._compile_function(function) for function in self.functions),
            # include function to terminate with a result for overall task
            self._compile_terminate_function(),
            self.PANIC_FUNCTION,
        ]
        settings = OpenAIChatSettings(
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
        errors: list[TaskError] = []

        async def _error(error: TaskError):
            await runner.error(error)
            errors.append(error)
            messages.append(self._compile_error(error))
            return error

        while True:
            await runner.step()
            completion: Optional[OpenAIChatCompletion] = None
            while completion is None:
                try:
                    await runner.model_step(model)
                    completion = await model(
                        messages=messages, functions=functions, settings=settings
                    )
                    break
                except RuntimeError as e:
                    await runner.model_error(model, e)
                    continue
            m = completion.message
            messages.append(m)
            if m.function_call is None:  # missing function call
                await _error(TaskError(TaskErrorType.INVALID_FORMAT, "no function call"))
                continue
            try:  # try to parse arguments (only json format check, no type check)
                arguments = json.loads(m.function_call.arguments)
            except JSONDecodeError as e:
                await _error(TaskError(TaskErrorType.INVALID_FORMAT, str(e)))
                continue
            # handle valid function call
            if m.function_call.name == "panic":
                raise await _error(TaskError(TaskErrorType.INCAPABLE, arguments["reason"]))
            elif m.function_call.name == "terminate":
                try:
                    check_type(arguments, self.task, is_output=True)
                    return arguments
                except (ValueError, TypeError, TaskError) as e:
                    await _error(e)
            elif m.function_call.name not in self.functions_by_py_ident:
                await _error(
                    TaskError(
                        TaskErrorType.INVALID_FORMAT,
                        f"unknown function {m.function_call.name}",
                    )
                )
            else:
                function = self.functions_by_py_ident[m.function_call.name]
                ret = await runner.call_function(function, arguments)
                messages.append(self._compile_function_result(function, ret))


@x_struct(
    "OpenAITextEmbeddingResponse", "Response from OpenAI text embedding models", file=_openai_text
)
class OpenAITextEmbeddingResponse:
    vector: typing.Union[Vector, list[Vector]]
    usage: OpenAITokenUsage


@x_model(
    "ada",
    "OpenAI's latest 1536 dimensional text embedding model",
    external_name="text-embedding-ada-002",
    file=_openai_text,
)
class OpenAITextEmbeddingModel(Model):
    async def _endpoint(self, text: typing.Union[str, list[str]]) -> OpenAITextEmbeddingResponse:
        is_batched = isinstance(text, list)
        if not is_batched:
            text = [text]
        rep = await openai.Embedding.acreate(
            input=text, model=self.external_name, api_key=self._api_key
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


@x_struct("OpenAIAudioTranscriptionResponse", "Response ", file=_openai_text)
class OpenAIAudioTranscriptionResponse:
    text: str


@x_model(
    "whisper",
    "OpenAI's latest audio transcription model",
    external_name="whisper",
    file=_openai_audio,
)
class OpenAIAudioTranscriptionModel(Model):
    async def _endpoint(self, audio: RemoteObject) -> OpenAIAudioTranscriptionResponse:
        raise NotImplementedError


anthropic_lib = Module(name="anthropic.lib")
_anthropic_text = anthropic_lib.create_file("text")


@x_struct(
    "AnthropicTextSettings", "Inference settings for Anthropic text models", file=_anthropic_text
)
class AnthropicTextCompletionSettings:
    temperature: float
    top_p: Optional[float]
    top_k: Optional[int]
    max_tokens_to_sample: int
    stop_sequences: Optional[list[str]]


@x_struct("AnthropicTextCompletion", "Completion from Anthropic text models", file=_anthropic_text)
class AnthropicTextCompletion:
    completion: str
    stop_reason: str


@x_model(
    "claude-1",
    "Anthropic's latest 9k context Claude based text model",
    external_name="claude-1",
    file=_anthropic_text,
)
@x_model(
    "claude-1-100k",
    "Anthropic's latest 100k context Claude based text model",
    external_name="claude-1-100k",
    file=_anthropic_text,
)
@x_model(
    "claude-instant-1",
    "Anthropic's faster 9k context Claude based text model",
    external_name="claude-instant-1",
    file=_anthropic_text,
)
@x_model(
    "claude-instant-1-100k",
    "Anthropic's faster 100k context Claude based text model",
    external_name="claude-instant-1-100k",
    file=_anthropic_text,
)
class AnthropicTextCompletionModel(Model):
    _client: anthropic.Client | None = None

    def _clear(self) -> None:
        self._client = None

    async def _endpoint(
        self, prompt: str, settings: AnthropicTextCompletionSettings
    ) -> AnthropicTextCompletion:
        if self._client is None:
            self.client = anthropic.Client(self._api_key)

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
    # assign stable versioned ids
    module.index()  # need to index for walk
    for node in module.walk():
        node.id = _versioned_id(node.path)
    module.clear()  # ids changed

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
