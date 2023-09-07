"""
Built-in library implementations.
"""

import enum
import json
import typing
from dataclasses import dataclass
from json import JSONDecodeError
from typing import Any, Optional
from uuid import UUID

import anthropic
import openai

from bench.language import Dataset, HasText, Run, Runnable, Tag, Variable
from bench.language.builtin import anthropic_lib, openai_lib, symbolx_lib
from bench.language.code_ import Code
from bench.language.const import TypeFlag, TypeTag
from bench.language.core import (
    LookupBy,
    Module,
    Statement,
    get_node_id,
    parse_absolute_statement_reference,
)
from bench.language.model import Model
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
from bench.language.session import RunError, RunStatus
from bench.language.task import (
    CompiledInput,
    IncapableError,
    ModuleView,
    Task,
    TaskCompiler,
    TaskError,
    TaskErrorType,
    TaskOutput,
)
from bench.language.type import (
    Field,
    Key,
    Type,
    TypeBase,
    Vector,
    check_type,
    map_value,
    new_field_key,
    strip_value_flat,
)
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
            text=type.text,
            items=element_type,
        )
    elif type.flags & TypeFlag.IsArrayable:
        raise NotImplementedError(f"unsupported type {type}: arrayable not yet supported")
    elif type.effective_tag == TypeTag.FUNCTION:
        return JsonSchemaElement(
            name=None,
            type=JsonSchemaElementType.object,
            text=type.text,
            properties=[_type_to_json_schema(field) for field in fields],
            required=[
                field.py_ident for field in fields if not (field.flags & TypeFlag.IsOptional)
            ],
        )
    elif type.effective_tag in TypeTag.STRUCT:
        return JsonSchemaElement(
            name=type.py_ident,
            type=JsonSchemaElementType.object,
            text=type.text,
            properties=[_type_to_json_schema(field) for field in fields],
            required=[
                field.py_ident for field in fields if not (field.flags & TypeFlag.IsOptional)
            ],
        )
    elif type.effective_tag == TypeTag.ENUM:
        return JsonSchemaElement(
            name=type.py_ident,
            type=JsonSchemaElementType.string,
            text=type.text,
            enum=[value.name for value in fields],
        )
    elif type.effective_tag in (TypeTag.STRING, TypeTag.NUMBER, TypeTag.BOOLEAN):
        return JsonSchemaElement(
            name=type.py_ident,
            type=_PARAM_TYPE_BY_TAG[type.effective_tag],
            text=type.text,
        )
    else:
        raise IncapableError(f"unsupported type {type}")


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
    "OpenAI's latest and largest 32k context GPT4 based chat model",
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

    def _compiler(self) -> "TaskCompiler":
        return OpenAIChatCompiler()


@dataclass
class OpenAIChatInput(CompiledInput):
    settings: OpenAIChatSettings
    messages: list[OpenAIChatMessage]
    functions: list[OpenAIFunction]
    runnables_by_name: dict[str, Runnable]


class OpenAIChatCompiler(TaskCompiler):
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

    def _render_value_flat(self, value: Any, type: TypeBase, *args, **kwargs) -> Any:
        """Model-friendly rendering of instantiated value."""
        if type.effective_tag == TypeTag.ENUM:
            return type.get_field(value).name
        else:
            return strip_value_flat(value, type, *args, **kwargs)

    def _render_statement_header(self, statement: Statement) -> str:
        """Model-friendly rendering of instantiated statement."""
        if isinstance(statement, HasText):
            return f"'{statement.type.name.lower()}' {statement.name or '<no name>'}: {statement.text_plain or '<no text>'}"
        else:
            return f"'{statement.type.name.lower()}' {statement.name or '<no name>'}"

    # TODO @Performance @Task: cache statement rendering (for datasets)
    async def _render_statement_content(
        self, statement: Statement
    ) -> tuple[str, list[UUID]] | None:
        if isinstance(statement, Variable):
            value_str = json.dumps(statement._raw_named_value())
            return value_str, []
        elif isinstance(statement, Dataset):
            records = await statement.limit(10).atolist()
            records_str = "\n".join([json.dumps(r._raw_named_value()) for r in records])
            return records_str, [r.id for r in records]
        elif isinstance(statement, Type):
            if statement.tag == TypeTag.ENUM:
                options_str = ", ".join(value.name for value in statement.fields)
                return f"options: {options_str}", [f.id for f in statement.fields]
            elif statement.tag == TypeTag.STRUCT:
                fields_str = ", ".join(
                    f"{f.name}: {f.type.name} {f.type.description}" for f in statement.fields
                )
                return f"fields:\n{fields_str}", [f.id for f in statement.fields]
        else:
            return None

    def _compile_function(self, tool: Runnable, prefix: str) -> OpenAIFunction:
        return OpenAIFunction(
            name=prefix + tool.py_ident,
            text=tool.text,
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
                    map_value(run.outputs, run.runnable, map_v=self._render_value_flat)
                ),
            )

    def _compile_error(self, error: RunError | TaskError) -> OpenAIChatMessage:
        return OpenAIChatMessage(
            role=OpenAIChatRole.system,
            content=f"Avoid previous error: {error}",
        )

    async def compile(
        self,
        task: Task,
        view: ModuleView,
        inputs: dict,
        previous_results: list[TaskError | Run],
        nonce: Optional[str],
    ) -> OpenAIChatInput:
        # TODO @Task: improve context message / module view generation
        # custom context messages
        # ignore output types, they're covered by function schemas
        covered_nodes = {n.id for o in task.outputs for n in o.walk_type()}
        context_strs = []
        for level in view.nodes_by_distance:
            for node in level:
                if node.id in covered_nodes:
                    continue
                if not isinstance(node, Statement) or node.id == task.id:
                    continue
                node: Statement
                head = self._render_statement_header(node)
                content = await self._render_statement_content(node)
                if content:
                    content, covered_children = content
                    for c in covered_children:
                        covered_nodes.add(c)
                    context_strs.append(f"{head}\n{content}")
                else:
                    context_strs.append(head)
        context_str = "\n".join(context_strs)
        context_messages = [
            OpenAIChatMessage(
                role=OpenAIChatRole.system,
                content=f"Additional user instructions for task '{task.name}':\n {context_str}\mFollow the above well.",
            )
        ]

        # context from previous runs (function executions, failures)
        previous_messages = []
        for result in previous_results:
            if isinstance(result, TaskError):
                previous_messages.append(self._compile_error(result))
            elif isinstance(result, Run):
                previous_messages.append(self._compile_run(result))
            else:
                raise ValueError(f"unexpected result {result}")

        # system and wrapper messages
        inputs = map_value(
            inputs,
            task,
            map_k=lambda f: (f.py_ident, f.py_ident),
            map_v=self._render_value_flat,
            is_output=False,
        )
        nonce_str = f"nonce:{nonce}\n" if nonce else ""
        messages: list[OpenAIChatMessage] = [
            self.SYSTEM_MESSAGE,
            OpenAIChatMessage(
                role=OpenAIChatRole.system,
                content=f"Your main task is {self._render_statement_header(task)}.",
            ),
            *context_messages,
            OpenAIChatMessage(
                role=OpenAIChatRole.user,
                content=f"{nonce_str}The user's inputs for '{task.name}': \n\n{inputs}",
            ),
            *previous_messages,
            OpenAIChatMessage(
                role=OpenAIChatRole.system,
                content=f"Now complete the task '{task.name}' given the inputs according to the schema."
                f" Consider the instructions and context for every part carefully.",
            ),
        ]

        # relevant functions
        available_functions = [f for f in view.nodes if isinstance(f, Code)]
        user_function_prefix = "_"
        user_functions_by_name: dict[str, Runnable] = {
            user_function_prefix + f.py_ident: f for f in available_functions
        }
        functions: list[OpenAIFunction] = [
            # TODO @Task: select functions from view more intelligently
            #  (and coerce? e.g. datasets)
            *(self._compile_function(f, prefix=user_function_prefix) for f in available_functions),
            # include function to terminate with a result for overall task
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

    async def run(
        self,
        model: OpenAIChatCompletionModel,
        input: OpenAIChatInput,
    ) -> TaskOutput:
        completion = await model(
            messages=input.messages, functions=input.functions, settings=input.settings
        )
        rep: OpenAIChatMessage = completion.message
        if rep.function_call is None:  # missing function call
            raise TaskError(TaskErrorType.InvalidFormat, model, "no function call")

        # try to parse arguments (only json format check, no type check)
        try:
            arguments = json.loads(rep.function_call.arguments)
        except JSONDecodeError as e:
            raise TaskError(TaskErrorType.InvalidFormat, model, str(e))

        # handle standard panic/terminate
        if rep.function_call.name == "panic":
            raise TaskError(TaskErrorType.Incapable, arguments["reason"])
        elif rep.function_call.name == "complete":
            try:
                check_type(arguments, input.task, is_output=True)
                return TaskOutput(result_raw=arguments)
            except (ValueError, TypeError) as e:
                raise TaskError.from_exception(e, model)

        # handle other function calls
        if rep.function_call.name not in input.runnables_by_name:
            raise TaskError(
                TaskErrorType.InvalidFormat,
                model,
                f"unknown function {rep.function_call.name}",
            )
        function = input.runnables_by_name[rep.function_call.name]
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


#
# anthropic.lib
#

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

# assign reproducible ids, and interp/index them
for name, module in DEFAULT_MODULES.items():
    assert module.name == name

    # assign stable cks / versioned ids
    module.index()  # need to index for walk
    for node in module._walk():
        if isinstance(node, Module):
            continue  # already assigned in builtin
        node.ck = _derive_constant_key(node.path)
        node.id = get_node_id(module.id, node.ck)
        if isinstance(node, (Field, Tag)):
            node.key = new_field_key(node.ck)
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

        # check that all HasType things have fields
        for statement in module_reloaded._statements_by_id.values():
            if isinstance(statement, (Type, Code, Task, Model)) and not statement.fields:
                raise RuntimeError(f"statement {statement} has no fields")

        if module_reloaded.issues:
            raise RuntimeError(f"module {module_reloaded} has bad issues: {module_reloaded.issues}")


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
