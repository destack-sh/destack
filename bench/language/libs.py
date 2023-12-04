"""
Built-in library implementations.
"""
import enum
import json
import re
import typing
from dataclasses import dataclass
from functools import cached_property
from json import JSONDecodeError
from typing import Any, Optional, Union
from uuid import UUID

import aiohttp
import anthropic
import deepgram
import numpy as np
import openai

from bench.language import Blob, HasRun, HasText, Module, Run, RunError, render
from bench.language.builtin import (
    anthropic_lib,
    deepgram_lib,
    huggingface_lib,
    openai_lib,
    symbolx_lib,
)
from bench.language.const import (
    INTERP_NODE_TYPES,
    NodeType,
    RunStatus,
    TypeFlag,
    TypeHint,
    TypeTag,
    new_dynamic_node_key,
)
from bench.language.database import HasDatabase
from bench.language.field import Field, HasFields, Key, Vector
from bench.language.model import HasModel, ModelError, ModelErrorType
from bench.language.module import Node, NodeList, ScopeNode, get_node_id
from bench.language.packer import map_value, pack_value_flat, render_value
from bench.language.reference import Projection
from bench.language.reflect import (
    _derive_constant_key,
    _model_compilers,
    _model_impls,
    x_code,
    x_enum,
    x_model,
    x_struct,
    x_tag,
    x_task,
)
from bench.language.statement import Statement
from bench.language.task import (
    CompiledInput,
    IncapableError,
    TaskCompiler,
    TaskError,
    TaskErrorType,
)
from bench.language.text import Text, patch_text_html, render_text_simple
from bench.utils.utils import DEBUG, LOCAL, UnreachableError, format_python, omit_empty

#
# symbolx.lib
#

_symbolx_builtins = symbolx_lib.files.create("builtins")
_symbolx_utils = symbolx_lib.files.create("utils")


@x_tag("cache", "Cache runs", file=_symbolx_builtins)
class Cache:
    pass


@x_tag("randomize", "Seed every run randomly", file=_symbolx_builtins)
class Randomize:
    pass


@x_tag("test", "A test case", file=_symbolx_builtins)
class Test:
    pass


@x_tag("template", "A template", file=_symbolx_builtins)
class Template:
    pass


@x_tag("export", "Make code outputs available for import", file=_symbolx_builtins)
class Export:
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


@x_task("transcribe", "Transcribe any audio into text", file=_symbolx_builtins)
def transcribe(url: Optional[str] = None, file: Optional[Blob] = None) -> TranscriptionOutput:
    raise UnreachableError()  # stub


@x_struct("GetWebsiteHtmlOutput", "Extracted website content", file=_symbolx_builtins)
class GetWebsiteHtmlOutput:
    html: str


@x_code("get website html", "Reads website HTML from a URL", file=_symbolx_builtins)
def get_website_html(url: str) -> GetWebsiteHtmlOutput:
    raise UnreachableError()


@x_struct("SendEmailOutput", "Email sent", file=_symbolx_builtins)
class SendEmailOutput:
    pass


@x_code("send email", "Sends an email to a registered Bench user", file=_symbolx_builtins)
def send_email(
    to: str,
    subject: str,
    body: str,
) -> SendEmailOutput:
    raise UnreachableError()


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

    def visit(self) -> None:
        yield self
        if self.properties:
            for p in self.properties:
                yield from p.visit()
        if self.items:
            yield from self.items.visit()

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
            type=self.type.name,
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
    type: Union[Field, Statement], ignore_array: bool = False, is_output: bool = None
) -> JsonSchemaElement:
    """Convert a Bench type to a JSON schema element."""
    fields = [
        f
        for f in type.resolved_fields
        if is_output is None or bool(f.flags & TypeFlag.IS_OUTPUT) == is_output
    ]
    if type.flags & TypeFlag.IS_ARRAY and not ignore_array:
        element_type = _type_to_json_schema(type, ignore_array=True)
        element_type.name = None  # not needed for array element
        return JsonSchemaElement(
            name=type.py_ident,
            type=JsonSchemaElementType.array,
            text=type.text_plain,
            items=element_type,
        )
    elif type.flags & TypeFlag.IS_ARRAYABLE:
        raise NotImplementedError(f"unsupported type {type}: arrayable not yet supported")
    elif type._effective_tag == TypeTag.FUNCTION:
        return JsonSchemaElement(
            name=None,
            type=JsonSchemaElementType.object,
            text=type.text_plain,
            properties=[_type_to_json_schema(field) for field in fields],
            required=[
                field.py_ident for field in fields if not (field.flags & TypeFlag.IS_OPTIONAL)
            ],
        )
    elif type._effective_tag in TypeTag.STRUCT:
        return JsonSchemaElement(
            name=type.py_ident,
            type=JsonSchemaElementType.object,
            text=type.text_plain,
            properties=[_type_to_json_schema(field) for field in fields],
            required=[
                field.py_ident for field in fields if not (field.flags & TypeFlag.IS_OPTIONAL)
            ],
        )
    elif type._effective_tag == TypeTag.ENUM:
        return JsonSchemaElement(
            name=type.py_ident,
            type=JsonSchemaElementType.string,
            text=type.text_plain,
            enum=[value.name for value in fields],
        )
    elif type._effective_tag in (TypeTag.STRING, TypeTag.NUMBER, TypeTag.BOOLEAN):
        return JsonSchemaElement(
            name=type.py_ident,
            type=_PARAM_TYPE_BY_TAG[type._effective_tag],
            text=type.text_plain,
        )
    else:
        raise IncapableError(f"unsupported type {type}")


class BaseTextTaskCompiler(TaskCompiler):
    SYSTEM_MESSAGE = (
        "You are a precise and highly capable bot that can do almost anything a user asks."
        " Interpret inputs generously and attentively, be concise, be considerate."
        " You are accessed through an API, so don't respond to the user directly."
    )

    def _render_value_flat(self, value: Any, type: Union[Field, Statement], *args, **kwargs) -> Any:
        """Model-friendly rendering of instantiated value."""
        if type._effective_tag == TypeTag.ENUM:
            return type.resolved_fields.get(value).name
        elif type.hint == TypeHint.RICH_TEXT and isinstance(value, Text):
            return render_text_simple(value.spans)
        elif type.hint in (TypeHint.STATEMENT, TypeHint.FIELD):
            return value.py_ident
        else:
            return pack_value_flat(value, type, *args, **kwargs)

    async def _render_context(
        self, task: Statement, projection: Projection, *, exclude_output: bool
    ) -> str:
        """Model-friendly string describing the entire task context."""
        rendered = render(*projection.nodes, recursive=False)
        return rendered

    def _render_error(self, error: RunError | TaskError) -> str:
        if isinstance(error, TaskError):
            if error.type in (TaskErrorType.InvalidType, TaskErrorType.InvalidFormat):
                return f"{error.message} (follow the schema!)"
            else:
                return error.message
        else:
            return f"{error.type}: {error.message}"

    def _compile_run_text(self, run: Run) -> str:
        if run.status == RunStatus.Failed:
            return self._compile_error_text(run)
        else:
            run_inputs_str = json.dumps(
                map_value(run.inputs, run.statement, map_v=self._render_value_flat), indent=2
            )
            run_outputs_str = json.dumps(
                map_value(run.outputs, run.statement, map_v=self._render_value_flat), indent=2
            )
            return f"Previous result for '{run.statement.py_ident}' given '{run_inputs_str}':\n {run_outputs_str}"

    def _compile_error_text(self, error: RunError | TaskError) -> str:
        return f"Avoid previous error: {self._render_error(error)}"

    async def _prepare_text_prompt(
        self,
        task: Statement,
        projection: Projection,
        inputs: dict,
        previous_results: list[Union[TaskError, "Run"]],
        nonce: Optional[str],
    ) -> str:
        # system wrapper
        messages: list[str] = [
            self.SYSTEM_MESSAGE,
            f"Your main task is '{task.name}'.",
        ]

        # module context
        context_str = await self._render_context(task, projection, exclude_output=True)
        if context_str:
            messages.append(
                f"The definition of task '{task.name}':\n {context_str}"
                f"\nFollow the above context carefully w.r.t. to the following inputs."
            )

        # inputs
        inputs_strs = []
        for field_ in task.resolved_fields:
            if field_.flags & TypeFlag.IS_OUTPUT or field_.flags & TypeFlag.IS_CONFIG:
                continue
            field_value = inputs.get(field_.py_ident)
            if field_value is None:
                continue
            field_str = render_value(field_value, field_)
            inputs_strs.append(f"{field_.py_ident}: {field_str}")
        inputs_str = ", ".join(inputs_strs)
        inputs_str = format_python(f"{{{inputs_str}}}")
        nonce_str = f"(nonce:{nonce})" if nonce else ""
        messages.append(
            f"{nonce_str}\n\nThe user's inputs for '{task.name}': \n{inputs_str}",
        )

        # output schema
        output_fields = [f for f in task.resolved_fields if f.flags & TypeFlag.IS_OUTPUT]
        output_schema = _type_to_json_schema(task, is_output=True).to_dict()
        output_schema = omit_empty(output_schema)
        output_schema_str = json.dumps(output_schema, indent=2)
        messages.append(
            f"JSON schema for output to '{task.name}': \n{output_schema_str}",
        )

        # final CTA
        messages.append(
            f"Now, complete the task '{task.name}' given the inputs. The result should reflect the user inputs.\n"
            f" COMPLETE with a result, PANIC with a 'reason' field if completion is impossible."
            f" (Strongly prefer COMPLETE with error information)."
            f" Respond with COMPLETE|PANIC\\n\\n"
            f' "<top-level-field name>":\\n```\n<json value>\n``` (repeat for top-level schema properties).\n'
            f" \nFor example:\n"
            f"COMPLETE\n"
            f'"{output_fields[0].py_ident}":\n```\n<the value>\n```\n'
        )

        # context from previous runs
        for result in previous_results:
            if isinstance(result, TaskError):
                messages.append(self._compile_error_text(result))
            elif isinstance(result, Run):
                messages.append(self._compile_run_text(result))
            else:
                raise ValueError(f"unexpected result {result}")

        # compile final prompt
        prompt = "\n\n".join(messages)
        return prompt

    @staticmethod
    def _parse_text_completion(model: Statement, task: Statement, completion: str) -> dict:
        try:
            # strip everything up to COMPLETE or PANIC
            completion = re.sub(r"^.*?(COMPLETE|PANIC)", r"\1", completion, flags=re.DOTALL)
            header, body = completion.split("\n", maxsplit=1)
            action = header.strip()
        except (TypeError, ValueError) as e:
            raise TaskError(TaskErrorType.InvalidFormat, model, f"invalid response: {str(e)}")

        # parse out all top level fields
        outputs = {}
        try:
            completed_pairs = re.finditer(
                r"^\"?(?P<key>[\w ]+?)\"?:\s*(?P<body>([^\n(```)]+$)|```(\w+)?\n?(?P<inner>.*?)\n?```)",
                body.strip(),
                flags=re.DOTALL | re.MULTILINE,
            )
            for match in completed_pairs:
                field = task.resolved_fields.get(match.group("key"))
                if not field:
                    continue  # ignore
                value = match.group("inner") or match.group("body")
                value = value.strip()
                if value.startswith('"'):
                    value = value[1:]
                if value.endswith('"'):
                    value = value[:-1]  # sometimes the model forgets to close the quote
                value = value.strip()  # yes twice
                if field._effective_tag in (TypeTag.STRUCT, TypeTag.BOOLEAN, TypeTag.NUMBER):
                    # replace any """...""" with valid JSON string (with newlines escaped)
                    value = re.sub(
                        r'"""\\?\n?(.*?)"""',
                        lambda m: '"' + m.group(1).replace("\n", "\\n") + '"',
                        value,
                        flags=re.DOTALL,
                    )
                    value = json.loads(value)
                outputs[field.py_ident] = value
        except (TypeError, ValueError, JSONDecodeError) as e:
            raise TaskError(TaskErrorType.InvalidFormat, model, f"invalid JSON arguments: {str(e)}")

        if "COMPLETE" in action:
            return outputs
        elif "PANIC" in action:
            raise TaskError(TaskErrorType.Incapable, outputs.get("reason", "unknown"))
        elif action == "CALL_FUNCTION":
            raise NotImplementedError("anthropic task functions :TaskFunctions")
        else:
            raise TaskError(TaskErrorType.InvalidFormat, model, f"unknown action {action}")


#
# openai.lib
#

# Note that apart from the symbolx standard lib, all other libs should later
# be defined and update in Bench itself. That may also happen via code or some other
# automatic mechanism, it just shouldn't be here.
# The model implementations should be just like Code implementations,
# so we don't need to hot-swap in 'impl' when calling. :LibImplementation

openai_lib.add_dependency(symbolx_lib)
_openai_chat = openai_lib.files.create("chat")
_openai_text = openai_lib.files.create("text")
_openai_utils = openai_lib.files.create("utils")


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
        d = dict(role=self.role.name, content=self.content)
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
    "gpt3-turbo",
    "OpenAI's instruct-tuned 16k context GPT3.5 based chat model",
    external_name="gpt-3.5-turbo-1106",
    file=_openai_chat,
)
@x_model(
    "gpt4-turbo",
    "OpenAI's latest and largest 128k context GPT4 based chat model",
    external_name="gpt-4-1106-preview",
    file=_openai_chat,
)
class OpenAIChatCompletionModel(HasModel):
    async def _endpoint(
        self,
        messages: list[OpenAIChatMessage],
        functions: Optional[list[OpenAIFunction]],
        settings: OpenAIChatSettings,
    ) -> OpenAIChatCompletion:
        settings_raw = omit_empty(settings.to_dict())
        messages_raw = [(OpenAIChatMessage.to_dict(m)) for m in messages]
        functions_raw = (
            [omit_empty(OpenAIFunction.to_dict(f)) for f in functions] if functions else None
        )
        try:
            response = await openai.ChatCompletion.acreate(
                model=self.external_name,
                messages=messages_raw,
                **({"functions": functions_raw} if functions_raw else {}),
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
        # TODO @Task: select text/chat compiler more intelligently
        return OpenAITextCompiler()


def _map_openai_error(model: Statement, e: Exception) -> ModelError:
    if isinstance(e, openai.InvalidRequestError):
        return ModelError(ModelErrorType.InvalidRequest, model, str(e))
    return ModelError(ModelErrorType.Unavailable, model, str(e))


@dataclass
class OpenAIChatInput(CompiledInput):
    settings: OpenAIChatSettings
    messages: list[OpenAIChatMessage]
    functions: Optional[list[OpenAIFunction]]
    statements_by_name: dict[str, HasRun]

    def __str__(self):
        return f"{self.tokens} tokens"

    def __repr__(self):
        return f"<OpenAIChatInput {self}>"

    @cached_property
    def tokens(self) -> int:
        """Estimated token usage (very rough)."""
        messages_str = "\n".join([f"{m.role}: {m.content}" for m in self.messages])
        return int(len(messages_str) * 0.3)


class OpenAIChatCompiler(BaseTextTaskCompiler):
    CHAT_SYSTEM_MESSAGE = OpenAIChatMessage(
        role=OpenAIChatRole.system,
        content="You are a precise and highly capable bot that can do almost anything a user asks."
        " Interpret inputs generously and attentively, be concise, be considerate."
        " You are accessed through an API, so don't respond to the user directly.",
    )
    CHAT_PANIC_FUNCTION = OpenAIFunction(
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

    def _compile_function_chat(self, tool: HasRun, prefix: str) -> OpenAIFunction:
        return OpenAIFunction(
            name=prefix + tool.py_ident,
            text=f"{tool.type.name.lower()} {self._render_text(tool)}",
            parameters=_type_to_json_schema(tool, is_output=False),
        )

    def _compile_run_chat(self, run: Run) -> OpenAIChatMessage:
        if run.status == RunStatus.Failed:
            return self._compile_error_text(run.error)
        else:
            return OpenAIChatMessage(
                role=OpenAIChatRole.function,
                name=run.statement.py_ident,
                content=json.dumps(
                    map_value(run.outputs, run.statement, map_v=self._render_value_flat), indent=2
                ),
            )

    def _compile_error_chat(self, error: RunError | TaskError) -> OpenAIChatMessage:
        return OpenAIChatMessage(
            role=OpenAIChatRole.system,
            content=f"Avoid previous error: {self._render_error(error)}",
        )

    async def compile(
        self,
        task: Statement,
        projection: Projection,
        inputs: dict,
        previous_results: list[TaskError | Run],
        nonce: Optional[str],
    ) -> OpenAIChatInput:
        # system wrapper
        messages: list[OpenAIChatMessage] = [
            self.CHAT_SYSTEM_MESSAGE,
            OpenAIChatMessage(
                role=OpenAIChatRole.system,
                content=f"Your main task is '{task.name}'.",
            ),
        ]

        # module context
        context_str = await self._render_context(task, projection, exclude_output=True)
        messages.append(
            OpenAIChatMessage(
                role=OpenAIChatRole.system,
                content=f"The definition of task '{task.name}':\n {context_str}"
                f"\nFollow the above carefully.",
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
        nonce_str = f"(nonce:{nonce})" if nonce else ""
        messages.append(
            OpenAIChatMessage(
                role=OpenAIChatRole.user,
                content=f"{nonce_str}The user's inputs for '{task.name}': \n\n{inputs}",
            )
        )

        # context from previous runs
        for result in previous_results:
            if isinstance(result, TaskError):
                messages.append(self._compile_error_chat(result))
            elif isinstance(result, Run):
                messages.append(self._compile_run_chat(result))
            else:
                raise ValueError(f"unexpected result {result}")

        # final CTA
        messages.append(
            OpenAIChatMessage(
                role=OpenAIChatRole.system,
                content=f"Now, complete the task '{task.name}' given the inputs.",
            ),
        )

        # task functions (not supported yet, will be parsed from input/output types :TaskFunctions)
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
            self.CHAT_PANIC_FUNCTION,
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
            statements_by_name=user_functions_by_name,
        )

    def can_run(self, model: "Statement", input: OpenAIChatInput) -> bool:
        context_window: int = {
            "gpt3-turbo": 16 * 1024,
            "gpt4-turbo": 128 * 1024,
        }[model.name]
        return input.tokens <= context_window

    async def run(
        self,
        model: OpenAIChatCompletionModel,
        input: OpenAIChatInput,
    ) -> dict:
        rep = await model(
            messages=input.messages, functions=input.functions, settings=input.settings
        )
        msg: OpenAIChatMessage = rep.message
        if msg.function_call is None:  # missing function call
            # try to figure out what it tried to do by parsing out json
            try:
                arguments_str = re.match(r"\{.*}", msg.content, flags=re.DOTALL)
                json.loads(arguments_str.group(0))
                function_call = "complete"
            except Exception:
                raise TaskError(TaskErrorType.InvalidFormat, model, "no function call")
        else:
            function_call = msg.function_call.name

        # try to parse arguments (only json format check, no type check)
        try:
            arguments = json.loads(msg.function_call.arguments)
        except JSONDecodeError as e:
            raise TaskError(TaskErrorType.InvalidFormat, model, str(e))

        # handle standard panic/terminate
        if function_call == "panic":
            raise TaskError(TaskErrorType.Incapable, arguments["reason"])
        elif function_call == "complete":
            return arguments
        raise UnreachableError(f"unexpected function call {function_call}")


class OpenAITextCompiler(BaseTextTaskCompiler):
    async def compile(
        self,
        task: Statement,
        projection: Projection,
        inputs: dict,
        previous_results: list[Union[TaskError, "Run"]],
        nonce: Optional[str],
    ) -> OpenAIChatInput:
        prompt = await self._prepare_text_prompt(task, projection, inputs, previous_results, nonce)
        messages = [OpenAIChatMessage(role=OpenAIChatRole.user, content=prompt)]

        # settings
        settings = OpenAIChatSettings(
            temperature=0.8,
            max_tokens=None,
            top_p=None,
            stop=None,
            logit_bias=None,
            frequence_penalty=None,
            presence_penalty=None,
            function_call=None,
            user=None,
        )

        return OpenAIChatInput(
            task=task,
            settings=settings,
            messages=messages,
            functions=None,
            statements_by_name={},
        )

    def can_run(self, model: "Statement", input: OpenAIChatInput) -> bool:
        context_window: int = {
            "gpt3-turbo": 16 * 1024,
            "gpt4-turbo": 128 * 1024,
        }[model.name]
        return input.tokens <= context_window

    async def run(
        self,
        model: OpenAIChatCompletionModel,
        input: OpenAIChatInput,
    ) -> dict:
        rep = await model(
            messages=input.messages, functions=input.functions, settings=input.settings
        )
        msg: OpenAIChatMessage = rep.message
        completion = msg.content.strip()
        return self._parse_text_completion(model, input.task, completion)


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
class OpenAITextEmbeddingModel(HasModel):
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

_anthropic_text = anthropic_lib.files.create("text")


@x_struct(
    "AnthropicTextSettings", "Inference settings for Anthropic text models", file=_anthropic_text
)
class AnthropicTextCompletionSettings:
    temperature: float
    top_p: float
    top_k: int
    max_tokens_to_sample: int


@x_struct("AnthropicTextCompletion", "Completion from Anthropic text models", file=_anthropic_text)
class AnthropicTextCompletion:
    completion: str
    stop_reason: str


def _map_anthropic_error(model: Statement, e: Exception) -> ModelError:
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
class AnthropicTextCompletionModel(HasModel):
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
                stop_sequences=[anthropic.HUMAN_PROMPT],
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
        return int(len(self.prompt) * 0.3)


class AnthropicTextCompiler(BaseTextTaskCompiler):
    async def compile(
        self,
        task: Statement,
        projection: Projection,
        inputs: dict,
        previous_results: list[Union[TaskError, "Run"]],
        nonce: Optional[str],
    ) -> AnthropicTextInput:
        prompt = await self._prepare_text_prompt(task, projection, inputs, previous_results, nonce)
        prompt = f"{anthropic.HUMAN_PROMPT}: {prompt}{anthropic.AI_PROMPT}"

        # settings
        max_tokens_to_sample = 99 * 1000 - int(len(prompt) * 0.3)
        assert max_tokens_to_sample > 0, f"prompt for {task!r} too long: {len(prompt)}"
        settings = AnthropicTextCompletionSettings(
            temperature=0.8, top_p=0.7, top_k=5, max_tokens_to_sample=max_tokens_to_sample
        )

        return AnthropicTextInput(task=task, settings=settings, prompt=prompt)

    def can_run(self, model: "Statement", input: AnthropicTextInput) -> bool:
        context_window: int = {
            "claude-2": 100 * 1024,
            "claude-instant-1": 100 * 1024,
        }[model.name]
        return input.tokens <= context_window

    async def run(self, model: "Statement", input: AnthropicTextInput) -> dict:
        rep: AnthropicTextCompletion = await model(prompt=input.prompt, settings=input.settings)
        completion = rep.completion.strip()
        return self._parse_text_completion(model, input.task, completion)


#
# deepgram.lib
#

deepgram_lib.add_dependency(symbolx_lib)
_deepgram_audio = deepgram_lib.files.create("audio")


@x_struct(
    "DeepgramAudioTranscription", "Transcription from Deepgram audio models", file=_deepgram_audio
)
class DeepgramAudioTranscription:
    text: str


@x_model(
    "nova-2",
    "Deepgram's latest audio transcription model",
    external_name="nova-2.0",
    file=_deepgram_audio,
)
class DeepgramAudioTranscriptionModel(HasModel):
    async def _endpoint(
        self, url: str, language: Optional[str] = None
    ) -> DeepgramAudioTranscription:
        dg_client = deepgram.Deepgram(self._api_key)
        options = {"model": "nova-2", "smart_format": True}
        if language:
            options["language"] = language
        response = await dg_client.transcription.prerecorded({"url": url}, options)
        results = response["results"]
        alternatives = results["channels"][0]["alternatives"]
        transcript = alternatives[0]["transcript"]

        return DeepgramAudioTranscription(text=transcript)


#
# huggingface.lib
# TODO @Architecture @Cleanup: don't hard-code specific HF model?
#

huggingface_lib.add_dependency(symbolx_lib)
_huggingface_text = huggingface_lib.files.create("text")


@x_struct(
    "HuggingfaceEmbeddingResponse",
    "Response from Huggingface text embedding models",
    file=_huggingface_text,
)
class HuggingfaceEmbeddingResponse:
    vector: typing.Union[Vector, list[Vector]]


@x_model(
    "llm-embedder",
    "BAAI/llm-embedder",
    external_name="baai/llm-embedder",
    file=_huggingface_text,
)
class HuggingfaceEmbeddingModel(HasModel):
    async def _endpoint(self, text: typing.Union[str, list[str]]) -> HuggingfaceEmbeddingResponse:
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
                vector = await response.json()
        if not isinstance(vector, list):
            raise TaskError(TaskErrorType.Incapable, self, f"invalid response: {vector}")

        # quantize embeddings to [-128, 127] bytearray
        vector = np.array(vector, dtype=np.float32)
        vector = (vector * 128).clip(-128, 127).astype(np.int8)
        vector = [v.tolist() for v in vector]
        if not is_batched:
            vector = vector[0]

        return HuggingfaceEmbeddingResponse(vector=vector)


DEFAULT_MODULES: dict[str, Module] = {
    "symbolx.lib": symbolx_lib,
    "openai.lib": openai_lib,
    "anthropic.lib": anthropic_lib,
    "deepgram.lib": deepgram_lib,
    "huggingface.lib": huggingface_lib,
}
DEFAULT_DEPENDENCIES = ("symbolx.templates",)

# assign reproducible ids, and interp/index them
for name, module in DEFAULT_MODULES.items():
    module.committed = True
    assert module.name == name

    # assign stable cks / versioned ids
    nodes = [n for n in module._walk_rec() if n.node_type not in INTERP_NODE_TYPES]
    node_by_path: dict[str, Node] = {}
    target_cks: dict[UUID, UUID] = {}
    for node in nodes:
        if isinstance(node, Module):
            continue  # already assigned in builtin
        if not node.name:
            path = node.parent.path + ":" + node.order_key
        else:
            path = node.path
        if path in node_by_path:
            raise ValueError(f"node path conflict for '{path}': {node!r} vs {node_by_path[path]!r}")
        new_ck = _derive_constant_key(path)
        target_cks[node.ck] = new_ck
        node.ck = new_ck
        node_by_path[path] = node
        node.id = get_node_id(module.id, node.ck)
        if HasDatabase in node._components:  # takes precedence over HasFields
            node.key = HasDatabase._derive_key(node)
        elif isinstance(node, Field) or HasFields in node._components:
            node.key = new_dynamic_node_key(node.ck)
    # hard re-index everything (ids changed)
    # reset all inline node lists
    for node in nodes:  # clear resets references to their ids, so run after assigning all ids
        for name, prop in node.__list_properties__.items():
            if prop.list_type == NodeList:
                setattr(node, name, prop.list_type(node, prop))
        node._clear_self(node.scope)
    module._clear_self(module)
    module._local_tree.set(nodes)
    for node in nodes:
        # we re-init above to reset the key, so manually update lists
        if isinstance(node, ScopeNode):
            node._update_lists(node)
    # patch references
    for node in nodes:
        # TODO @Broken: use same reference patching as in wire (and share with hot reload, etc.)
        if node.node_type == NodeType.STATEMENT and HasText in node._components:
            # only patching text here is fine since we clear after all ids/cks are updated
            # and only in-text references are not automatically updated
            node.text = patch_text_html(node.text, target_cks)

    # and check everything is ok
    module._index_rec()
    module._interp_rec()
    if module.issues:
        raise RuntimeError(f"default module {module.name} has issues: {module.issues}")
    module._validate_rec()

    # extra sanity checks for debugging
    if DEBUG or LOCAL:
        # also check for issues after reload to prevent any sneaky reference bugs
        from bench.language import wire

        module_data = wire.pack_module_inline(module, exclude=INTERP_NODE_TYPES)
        module_reloaded = wire.unpack_module(module_data.nodes, session=None)
        if module_reloaded.name != "symbolx.lib":
            module_reloaded.add_dependency(symbolx_lib)
        module_reloaded._interp_rec()

        if module_reloaded.issues:  # maybe something got lost in pack/unpack
            raise RuntimeError(
                f"module {module_reloaded} has flaky issues: {module_reloaded.issues}"
            )

    # manually 'deactivate session' for module since we're outside a session
    for node in module._nodes:
        if node.node_type == NodeType.STATEMENT and HasDatabase in node._components:
            for record in node.records:
                record._set_untracked("value", record._raw_value(_force=True))


def lookup_model_impl(path: str) -> Optional[typing.Callable]:
    return _model_impls.get(path)


def lookup_model_compiler(path: str) -> Optional[typing.Callable]:
    return _model_compilers.get(path)
