"""
Built-in library implementations.
"""
import enum
import json
import typing
from dataclasses import dataclass
from functools import cached_property
from json import JSONDecodeError
from typing import Any, Optional, Union
from uuid import UUID

import anthropic
import openai

from bench.language import Dataset, HasRun, HasText, Module, Run, RunError, Variable
from bench.language.builtin import anthropic_lib, openai_lib, symbolx_lib
from bench.language.const import RunStatus, TypeFlag, TypeTag
from bench.language.field import Field, HasFields, Key, Vector, new_dynamic_node_key
from bench.language.mapping import map_value, pack_value_flat
from bench.language.model import ModelError, ModelErrorType
from bench.language.module import get_node_id
from bench.language.reference import ModuleView
from bench.language.reflect import (
    _derive_constant_key,
    _model_compilers,
    _model_impls,
    x_enum,
    x_model,
    x_struct,
    x_tag,
    x_task,
)
from bench.language.remote import RemoteObject
from bench.language.statement import Model, Statement, Task, Type
from bench.language.task import (
    CompiledInput,
    IncapableError,
    TaskCompiler,
    TaskError,
    TaskErrorType,
    TaskOutput,
)
from bench.language.text import TextMention
from bench.utils.utils import DEBUG, LOCAL, UnreachableError, omit_empty

#
# symbolx.lib
#

_symbolx_builtins = symbolx_lib.create_file("builtins")
_symbolx_utils = symbolx_lib.create_file("utils")


@x_tag("tool", "A tool for a bot", file=_symbolx_builtins)
class Tool:
    pass


@x_tag("cache", "Cache runs", file=_symbolx_builtins)
class Cache:
    pass


@x_tag("randomize", "Seed every run randomly", file=_symbolx_builtins)
class Randomize:
    pass


@x_tag("test", "A test case", file=_symbolx_builtins)
class Test:
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


@x_tag("export", "Make code outputs available for import", file=_symbolx_builtins)
class Export:
    pass


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
    text: Optional[str] = None
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
            text=self.text,
            properties=properties,
            items=items,
            enum=self.enum,
            required=self.required,
        )


_PARAM_TYPE_BY_TAG = {
    TypeTag.STRING: JsonSchemaElementType.string,
    TypeTag.NUMBER: JsonSchemaElementType.number,
    TypeTag.BOOLEAN: JsonSchemaElementType.boolean,
}


def _type_to_json_schema(
    type: Field | Type, ignore_array: bool = False, is_output: bool = None
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
            text=type.text_plain,
            items=element_type,
        )
    elif type.flags & TypeFlag.IsArrayable:
        raise NotImplementedError(f"unsupported type {type}: arrayable not yet supported")
    elif type.effective_tag == TypeTag.FUNCTION:
        return JsonSchemaElement(
            name=None,
            type=JsonSchemaElementType.object,
            text=type.text_plain,
            properties=[_type_to_json_schema(field) for field in fields],
            required=[
                field.py_ident for field in fields if not (field.flags & TypeFlag.IsOptional)
            ],
        )
    elif type.effective_tag in TypeTag.STRUCT:
        return JsonSchemaElement(
            name=type.py_ident,
            type=JsonSchemaElementType.object,
            text=type.text_plain,
            properties=[_type_to_json_schema(field) for field in fields],
            required=[
                field.py_ident for field in fields if not (field.flags & TypeFlag.IsOptional)
            ],
        )
    elif type.effective_tag == TypeTag.ENUM:
        return JsonSchemaElement(
            name=type.py_ident,
            type=JsonSchemaElementType.string,
            text=type.text_plain,
            enum=[value.name for value in fields],
        )
    elif type.effective_tag in (TypeTag.STRING, TypeTag.NUMBER, TypeTag.BOOLEAN):
        return JsonSchemaElement(
            name=type.py_ident,
            type=_PARAM_TYPE_BY_TAG[type.effective_tag],
            text=type.text_plain,
        )
    else:
        raise IncapableError(f"unsupported type {type}")


class BaseTextTaskCompiler(TaskCompiler):
    def _render_value_flat(self, value: Any, type: Field | Type, *args, **kwargs) -> Any:
        """Model-friendly rendering of instantiated value."""
        if type.effective_tag == TypeTag.ENUM:
            return type.get_field(value).name
        else:
            return pack_value_flat(value, type, *args, **kwargs)

    def _render_text(self, text: HasText) -> str:
        if text.text is None:
            return "<no text>"
        return "".join(str(s) if isinstance(s, TextMention) else str(s) for s in text.text_spans)

    def _render_statement_header(self, statement: Statement, *, name: str = None) -> Optional[str]:
        """Model-friendly string describing statement header."""
        name = name or statement.name  # allow overriding name
        if name:
            if statement.text:
                return f"'{statement.type.name.lower()}' {name}: {self._render_text(statement)}"
            else:
                return f"'{statement.type.name.lower()}' {name}"
        elif statement.text:
            return f"'{statement.type.name.lower()}' {self._render_text(statement)}"
        else:
            return None

    # TODO @Performance @Task: cache statement rendering (for datasets)
    async def _render_statement_body(self, statement: Statement) -> tuple[str | None, list[UUID]]:
        """Model-friendly string describing statement content (excl. header)."""
        if isinstance(statement, Variable):
            value_str = json.dumps(statement._raw_named_value(), indent=2)
            return value_str, []
        elif isinstance(statement, Dataset):
            records = await statement.limit(10).atolist()
            records_str = "\n".join([json.dumps(r._raw_named_value(), indent=2) for r in records])
            return records_str, [r.id for r in records]
        elif isinstance(statement, Type):
            if statement.tag == TypeTag.ENUM:
                options_str = "\n".join("  - " + self._render_field(f) for f in statement.fields)
                return f"has options (one of):\n{options_str}", [f.id for f in statement.fields]
            elif statement.tag == TypeTag.STRUCT:
                fields_str = "\n".join("  - " + self._render_field(f) for f in statement.fields)
                return f"has fields (all of):\n{fields_str}", [f.id for f in statement.fields]
        else:
            return None, []

    def _render_field(self, field: Field) -> str:
        if field.tag == TypeTag.LITERAL:
            if field.text:
                return f"{field.name}: {self._render_text(field)}"
            else:
                return f"{field.name}"
        else:
            if field.text:
                return f"{field.name}: {self._render_text(field)} ({field._type_str})"
            else:
                return f"{field.name} ({field._type_str})"

    async def _render_context(self, task: Task, view: ModuleView, *, exclude_output: bool) -> str:
        """Model-friendly string describing the entire task context."""
        # ignore output types, they're covered by function schemas
        if exclude_output:
            seen_node_ids = {n.id for o in task.outputs for n in o.walk_type()}
        else:
            seen_node_ids = set()
        context_strs = []
        for level in view.nodes_by_distance:
            for node in level:
                if node.id in seen_node_ids:
                    continue
                if not isinstance(node, Statement) or node.id == task.id:
                    continue
                node: Statement
                header = self._render_statement_header(node)
                body, covered_children = await self._render_statement_body(node)
                for c in covered_children:
                    seen_node_ids.add(c)
                if header and body:
                    context_strs.append(header + body)
                elif header:
                    context_strs.append(header)
                elif body:
                    context_strs.append(body)
        context_str = "\n".join(context_strs)
        return context_str

    def _render_error(self, error: RunError | TaskError) -> str:
        if isinstance(error, TaskError):
            if error.type in (TaskErrorType.InvalidType, TaskErrorType.InvalidFormat):
                return f"{error.message} (follow the schema!)"
            else:
                return error.message
        else:
            return f"{error.type}: {error.message}"


#
# openai.lib
#

# Note that apart from the symbolx standard lib, all other libs should later
# be defined and update in Bench itself. That may also happen via code or some other
# automatic mechanism, it just shouldn't be here.
# The model implementations should be just like Code implementations,
# so we don't need to hot-swap in 'impl' when calling. :LibImplementation

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
    text: Optional[str]
    parameters: "JsonSchemaElement"

    def to_dict(self) -> dict[str, Any]:  # :ToDict
        return dict(
            name=self.name,
            text=self.text,
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
    "OpenAI's instruct-tuned 16k context GPT3.5 based chat model",
    external_name="gpt-3.5-turbo-16k-0613",
    file=_openai_chat,
)
@x_model(
    "gpt4",
    "OpenAI's latest and largest 8k context GPT4 based chat model",
    external_name="gpt-4-0613",
    file=_openai_chat,
)
@x_model(
    "gpt4-32k",
    "OpenAI's latest and largest 32k context GPT4 based chat model",
    external_name="gpt-4-32k-0613",
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
        try:
            response = await openai.ChatCompletion.acreate(
                model=self.external_name,
                messages=messages_raw,
                functions=functions_raw,
                **settings_raw,
                api_key=self._api_key,
            )
        except Exception as e:
            raise _map_openai_error(self, e) from e
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

    def _compiler(self) -> "TaskCompiler":
        return OpenAIChatCompiler()


def _map_openai_error(model: Model, e: Exception) -> ModelError:
    if isinstance(e, openai.InvalidRequestError):
        return ModelError(ModelErrorType.InvalidRequest, model, str(e))
    return ModelError(ModelErrorType.Unavailable, model, str(e))


@dataclass
class OpenAIChatInput(CompiledInput):
    settings: OpenAIChatSettings
    messages: list[OpenAIChatMessage]
    functions: list[OpenAIFunction]
    runnables_by_name: dict[str, HasRun]

    def __str__(self):
        return f"{self.tokens} tokens"

    def __repr__(self):
        return f"<OpenAIChatInput {self}>"

    @cached_property
    def tokens(self) -> int:
        """Estimated token usage (very rough)."""
        messages_str = json.dumps([m.to_dict() for m in self.messages])
        functions_str = json.dumps([f.to_dict() for f in self.functions])
        return (len(messages_str) + len(functions_str)) * 4


class OpenAIChatCompiler(BaseTextTaskCompiler):
    SYSTEM_MESSAGE = OpenAIChatMessage(
        role=OpenAIChatRole.system,
        content="You are a precise and highly capable bot that can do almost anything a user asks."
        " Interpret inputs generously and attentively, be concise, be considerate."
        " You are accessed through an API, so don't respond to the user directly.",
    )
    PANIC_FUNCTION = OpenAIFunction(
        name="panic",
        text="Error if no reasonable termination is possible at all."
        " Strongly prefer 'complete' with the relevant error info instead.",
        parameters=JsonSchemaElement(
            name=None,  # not needed for root object
            type=JsonSchemaElementType.object,
            properties=[
                JsonSchemaElement(
                    name="reason",
                    type=JsonSchemaElementType.string,
                    text="The reason of incapability",
                    properties=None,
                    enum=None,
                )
            ],
            required=["reason"],
        ),
    )

    def _compile_function(self, tool: HasRun, prefix: str) -> OpenAIFunction:
        return OpenAIFunction(
            name=prefix + tool.py_ident,
            text=f"{tool.type.name.lower()} {self._render_text(tool)}",
            parameters=_type_to_json_schema(tool, is_output=False),
        )

    def _compile_run(self, run: Run) -> OpenAIChatMessage:
        if run.status == RunStatus.Failed:
            return self._compile_error(run)
        else:
            return OpenAIChatMessage(
                role=OpenAIChatRole.function,
                name=run.runnable.py_ident,
                content=json.dumps(
                    map_value(run.outputs, run.runnable, map_v=self._render_value_flat), indent=2
                ),
            )

    def _compile_error(self, error: RunError | TaskError) -> OpenAIChatMessage:
        return OpenAIChatMessage(
            role=OpenAIChatRole.system,
            content=f"Avoid previous error: {self._render_error(error)}",
        )

    async def compile(
        self,
        task: Task,
        view: ModuleView,
        inputs: dict,
        previous_results: list[TaskError | Run],
        nonce: Optional[str],
    ) -> OpenAIChatInput:
        # system wrapper
        messages: list[OpenAIChatMessage] = [
            self.SYSTEM_MESSAGE,
            OpenAIChatMessage(
                role=OpenAIChatRole.system,
                content=f"Your main task is {self._render_statement_header(task)}.",
            ),
        ]

        # module context
        context_str = await self._render_context(task, view, exclude_output=True)
        messages.append(
            OpenAIChatMessage(
                role=OpenAIChatRole.system,
                content=f"Additional user instructions for task '{task.name}':\n {context_str}"
                f"\nFollow the above very carefully.",
            )
        )

        # inputs
        inputs: dict = map_value(
            inputs,
            task,
            map_k=lambda f: (f.py_ident, f.py_ident),
            map_v=self._render_value_flat,
            is_output=False,
        )
        nonce_str = f"(nonce:{nonce})\n" if nonce else ""
        messages.append(
            OpenAIChatMessage(
                role=OpenAIChatRole.user,
                content=f"{nonce_str}The user's inputs for '{task.name}': \n\n{inputs}",
            )
        )

        # context from previous runs
        for result in previous_results:
            if isinstance(result, TaskError):
                messages.append(self._compile_error(result))
            elif isinstance(result, Run):
                messages.append(self._compile_run(result))
            else:
                raise ValueError(f"unexpected result {result}")

        # final CTA
        messages.append(
            OpenAIChatMessage(
                role=OpenAIChatRole.system,
                content=f"Now, complete the task '{task.name}' given the inputs according to the schema."
                f" Consider the instructions and context for every part carefully.",
            ),
        )

        # task functions (not supported yet :TaskFunctions)
        available_functions = []
        user_function_prefix = "_"
        user_functions_by_name: dict[str, HasRun] = {
            user_function_prefix + f.py_ident: f for f in available_functions
        }
        functions: list[OpenAIFunction] = [
            OpenAIFunction(
                name="complete",
                text=f"Complete the task '{task.name}' with an answer (if any)."
                f" Call this on successful completion.",
                parameters=_type_to_json_schema(task, is_output=True),
            ),
            self.PANIC_FUNCTION,
        ]

        # settings
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

        return OpenAIChatInput(
            task=task,
            settings=settings,
            messages=messages,
            functions=functions,
            runnables_by_name=user_functions_by_name,
        )

    def can_run(self, model: "Model", input: OpenAIChatInput) -> bool:
        context_window: int = {
            "gpt3": 16 * 1024,
            "gpt4": 8 * 1024,
            "gpt4-32k": 32 * 1024,
        }[model.name]
        return input.tokens <= context_window

    async def run(
        self,
        model: OpenAIChatCompletionModel,
        input: OpenAIChatInput,
    ) -> TaskOutput:
        rep = await model(
            messages=input.messages, functions=input.functions, settings=input.settings
        )
        msg: OpenAIChatMessage = rep.message
        if msg.function_call is None:  # missing function call
            raise TaskError(TaskErrorType.InvalidFormat, model, "no function call")

        # try to parse arguments (only json format check, no type check)
        try:
            arguments = json.loads(msg.function_call.arguments)
        except JSONDecodeError as e:
            raise TaskError(TaskErrorType.InvalidFormat, model, str(e))

        # handle standard panic/terminate
        if msg.function_call.name == "panic":
            raise TaskError(TaskErrorType.Incapable, arguments["reason"])
        elif msg.function_call.name == "complete":
            return TaskOutput(result_raw=arguments)

        # handle other function calls
        if msg.function_call.name not in input.runnables_by_name:
            raise TaskError(
                TaskErrorType.InvalidFormat,
                model,
                f"unknown function {msg.function_call.name}",
            )
        function = input.runnables_by_name[msg.function_call.name]
        return TaskOutput(result_raw=arguments, function=function)


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
        try:
            rep = await openai.Embedding.acreate(
                input=text, model=self.external_name, api_key=self._api_key
            )
        except Exception as e:
            raise _map_openai_error(self, e)
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


#
# anthropic.lib
#

_anthropic_text = anthropic_lib.create_file("text")


@x_struct(
    "AnthropicTextSettings", "Inference settings for Anthropic text models", file=_anthropic_text
)
class AnthropicTextCompletionSettings:
    temperature: float
    top_p: float
    top_k: int
    max_tokens_to_sample: int
    stop_sequences: Optional[list[str]]


@x_struct("AnthropicTextCompletion", "Completion from Anthropic text models", file=_anthropic_text)
class AnthropicTextCompletion:
    completion: str
    stop_reason: str


def _map_anthropic_error(model: Model, e: Exception) -> ModelError:
    if isinstance(
        e, (anthropic.BadRequestError, anthropic.NotFoundError, anthropic.UnprocessableEntityError)
    ):
        return ModelError(ModelErrorType.InvalidRequest, model, str(e))
    elif isinstance(e, (anthropic.RateLimitError,)):
        return ModelError(ModelErrorType.ExceededLimit, model, str(e))
    else:
        return ModelError(ModelErrorType.Unknown, model, str(e))


@x_model(
    "claude-2",
    "Anthropic's latest 100k context Claude based text model",
    external_name="claude-2.0",
    file=_anthropic_text,
)
@x_model(
    "claude-instant-1",
    "Anthropic's faster 100k context Claude based text model",
    external_name="claude-instant-1.2",
    file=_anthropic_text,
)
class AnthropicTextCompletionModel(Model):
    async def _endpoint(
        self, prompt: str, settings: AnthropicTextCompletionSettings
    ) -> AnthropicTextCompletion:
        # monkey patch Anthropic validation (which is broken)
        client = anthropic.AsyncAnthropic(api_key=self._api_key)
        # see https://console.anthropic.com/docs/api
        try:
            rep = await client.completions.create(
                prompt=prompt,
                model=self.external_name,
                stop_sequences=[anthropic.HUMAN_PROMPT, *(settings.stop_sequences or [])],
                temperature=settings.temperature,
                max_tokens_to_sample=int(settings.max_tokens_to_sample),
                top_p=settings.top_p,
            )
        except Exception as e:
            raise _map_anthropic_error(self, e)
        return AnthropicTextCompletion(completion=rep.completion, stop_reason=rep.stop_reason)

    def _compiler(self) -> "TaskCompiler":
        return AnthropicTextCompiler()


@dataclass
class AnthropicTextInput(CompiledInput):
    settings: AnthropicTextCompletionSettings
    prompt: str

    def __str__(self):
        return f"{self.tokens} tokens"

    def __repr__(self):
        return f"<AnthropicTextInput {self}>"

    @cached_property
    def tokens(self) -> int:
        """Estimated token usage (very rough)."""
        return len(self.prompt) * 4


class AnthropicTextCompiler(BaseTextTaskCompiler):
    SYSTEM_MESSAGE = (
        "You are a precise and highly capable bot that can do almost anything a user asks."
        " Interpret inputs generously and attentively, be concise, be considerate."
        " You are accessed through an API, so don't respond to the user directly."
    )

    def _compile_run(self, run: Run) -> str:
        if run.status == RunStatus.Failed:
            return self._compile_error(run)
        else:
            run_inputs_str = json.dumps(
                map_value(run.inputs, run.runnable, map_v=self._render_value_flat), indent=2
            )
            run_outputs_str = json.dumps(
                map_value(run.outputs, run.runnable, map_v=self._render_value_flat), indent=2
            )
            return f"Previous result for '{run.runnable.py_ident}' given '{run_inputs_str}':\n {run_outputs_str}"

    def _compile_error(self, error: RunError | TaskError) -> str:
        return f"Avoid previous error: {self._render_error(error)}"

    async def compile(
        self,
        task: Task,
        view: ModuleView,
        inputs: dict,
        previous_results: list[Union[TaskError, "Run"]],
        nonce: Optional[str],
    ) -> AnthropicTextInput:
        # system wrapper
        messages: list[str] = [
            self.SYSTEM_MESSAGE,
            f"Your main task is {self._render_statement_header(task)}.",
        ]

        # module context
        context_str = await self._render_context(task, view, exclude_output=True)
        if context_str:
            messages.append(
                f"Additional user instructions for task '{task.name}':\n {context_str}"
                f"\nFollow the above very carefully."
            )

        # inputs
        inputs = map_value(
            inputs,
            task,
            map_k=lambda f: (f.py_ident, f.py_ident),
            map_v=self._render_value_flat,
            is_output=False,
        )
        nonce_str = f"(nonce:{nonce}\n)" if nonce else ""
        messages.append(
            f"{nonce_str}The user's inputs for '{task.name}': \n{inputs}",
        )

        # output schema
        output_schema = _type_to_json_schema(task, is_output=True).to_dict()
        output_schema = omit_empty(output_schema)
        output_schema_str = json.dumps(output_schema, indent=2)
        messages.append(
            f"JSON schema for output to '{task.name}': \n{output_schema_str}",
        )

        # final CTA
        # TODO @Task: anthropic task functions :TaskFunctions
        available_actions = ["COMPLETE", "PANIC"]
        messages.append(
            f"Now, complete the task '{task.name}' given the inputs according to the schema."
            f" Consider the instructions and context for every part carefully."
            f" Respond with one of {available_actions}, then a newline, then JSON arguments."
            f" COMPLETE with a result for the task, PANIC with a 'reason' property if reasonable termination is impossible."
            f" (Strongly prefer COMPLETE with error information as feasible)."
            # f" CALL_FUNCTION <func_name> to run one of the given functions (if any).",
        )

        # context from previous runs
        for result in previous_results:
            if isinstance(result, TaskError):
                messages.append(self._compile_error(result))
            elif isinstance(result, Run):
                messages.append(self._compile_run(result))
            else:
                raise ValueError(f"unexpected result {result}")

        # compile final prompt
        prompt = "\n\n".join(messages)
        prompt = f"{anthropic.HUMAN_PROMPT}: {prompt}{anthropic.AI_PROMPT}"

        # settings
        settings = AnthropicTextCompletionSettings(
            temperature=0.8,
            top_p=0.7,
            top_k=5,
            max_tokens_to_sample=99 * 1024 - len(prompt) * 4,
            stop_sequences=None,
        )

        return AnthropicTextInput(task=task, settings=settings, prompt=prompt)

    def can_run(self, model: "Model", input: AnthropicTextInput) -> bool:
        context_window: int = {
            "claude-2": 100 * 1024,
            "claude-instant-1": 100 * 1024,
        }[model.name]
        return input.tokens <= context_window

    async def run(self, model: "Model", input: AnthropicTextInput) -> TaskOutput:
        rep: AnthropicTextCompletion = await model(prompt=input.prompt, settings=input.settings)

        try:
            completion = rep.completion.strip()
            # action is supposed to come first, but sometimes it's last
            if completion.startswith("{"):
                # model goofed, header is in last line
                header = completion.split("\n")[-1]
                body = completion[: -len(header)].strip()
            else:
                header = completion.split("\n", maxsplit=1)[0]
                body = completion[len(header) :].strip()
            header_parts = header.split(" ", maxsplit=1)
            action = header_parts[0]
            # function_name = header_parts[1] if len(header_parts) > 1 else None
        except (ValueError, TypeError) as e:
            raise TaskError(TaskErrorType.InvalidFormat, model, str(e))

        try:
            arguments = json.loads(body or "{}")
        except JSONDecodeError as e:
            raise TaskError(TaskErrorType.InvalidFormat, model, f"invalid JSON arguments: {str(e)}")

        if action == "COMPLETE":
            return TaskOutput(result_raw=arguments)
        elif action == "PANIC":
            raise TaskError(TaskErrorType.Incapable, arguments["reason"])
        elif action == "CALL_FUNCTION":
            raise NotImplementedError("anthropic task functions :TaskFunctions")
        else:
            raise TaskError(TaskErrorType.InvalidFormat, model, f"unknown action {action}")


DEFAULT_MODULES: dict[str, Module] = {
    "symbolx.lib": symbolx_lib,
    "openai.lib": openai_lib,
    "anthropic.lib": anthropic_lib,
}

# assign reproducible ids, and interp/index them
for name, module in DEFAULT_MODULES.items():
    assert module.name == name

    # assign stable cks / versioned ids
    module.clear()
    module.index()  # need to index for walk
    for node in module._walk():
        if isinstance(node, Module):
            continue  # already assigned in builtin
        node.ck = _derive_constant_key(node.path)
        node.id = get_node_id(module.id, node.ck)
        if isinstance(node, (Field, HasFields)):
            node.key = new_dynamic_node_key(node.ck)
    module.clear()  # ids changed

    # index
    module.index()
    module.interp()
    if module.issues:
        raise RuntimeError(f"default module {module.name} has issues: {module.issues}")

    # some extra checks for debugging
    if DEBUG or LOCAL:
        # also check for issues after reload to prevent any sneaky reference bugs
        from bench.language import wire

        module_data = wire.pack_module(module)
        module_reloaded = wire.unpack_module(module_data, session=None)
        if module_reloaded.name != "symbolx.lib":
            module_reloaded.add_dependency(symbolx_lib)
        module_reloaded.index()
        module_reloaded.interp()

        if module_reloaded.issues:
            raise RuntimeError(f"module {module_reloaded} has bad issues: {module_reloaded.issues}")


def lookup_model_impl(path: str) -> Optional[typing.Callable]:
    return _model_impls.get(path)


def lookup_model_compiler(path: str) -> Optional[typing.Callable]:
    return _model_compilers.get(path)
