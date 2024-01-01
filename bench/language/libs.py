"""
Built-in library implementations.
"""
import enum
import json
import re
import typing
from dataclasses import dataclass
from json import JSONDecodeError
from typing import Any, Optional, Union

import aiohttp
import deepgram
import numpy as np
import openai

from bench.language import Module, Run, RunError, render
from bench.language.builtin import symbolx_lib
from bench.language.const import RunStatus, TypeFlag, TypeHint, TypeTag
from bench.language.field import Field
from bench.language.model import HasModel, ModelError, ModelErrorType
from bench.language.packer import map_value, pack_value_flat, render_value
from bench.language.projection import Projection
from bench.language.statement import Statement
from bench.language.task import IncapableError, TaskCompiler, TaskError, TaskErrorType
from bench.language.text import Text, render_text_simple
from bench.utils.utils import format_python, omit_empty


class JsonSchemaElementType(enum.StrEnum):
    string = "string"
    number = "number"
    boolean = "boolean"
    object = "object"
    array = "array"
    null = "null"


@dataclass
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
        " When given examples to consider, don't copy them directly unless explicitly asked."
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
        if run.status == RunStatus.FAILED:
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
            function_call = dict(
                name=message["function_call"]["name"],
                arguments=message["function_call"]["arguments"],
            )
        else:
            function_call = None
        return dict(
            message=dict(
                role=message["role"],
                content=message["content"],
                name=message.get("name"),
                function_call=function_call,
            ),
            usage=dict(
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


def tokens(self) -> int:
    """Estimated token usage (very rough)."""
    messages_str = "\n".join([f"{m.role}: {m.content}" for m in self.messages])
    return int(len(messages_str) * 0.3)


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
        messages = [dict(role="user", content=prompt)]

        # settings
        settings = dict(temperature=0.8)

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


class DeepgramAudioTranscriptionModel(HasModel):
    async def _endpoint(self, url: str, language: Optional[str] = None) -> dict:
        dg_client = deepgram.Deepgram(self._api_key)
        options = {"model": "nova-2", "smart_format": True, "diarize": True}
        if language:
            options["language"] = language
        response = await dg_client.transcription.prerecorded({"url": url}, options)

        # transcript = newline separated utterances
        # timestamps as [<HH:mm:ss>]
        # speaker prefix as [Speaker:<speaker_id>]
        utterances = []
        alternatives = response["results"]["channels"][0]["alternatives"]
        for paragraph in alternatives[0]["paragraphs"]["paragraphs"]:
            start_time = paragraph["start"]
            hours = int(start_time) // 3600
            minutes = int(start_time) // 60 % 60
            seconds = int(start_time) % 60
            start_time = f"{hours:02}:{minutes:02}:{seconds:02}"
            paragraph_text = " ".join(s["text"] for s in paragraph["sentences"])
            utterances.append(f"[{start_time}][Speaker:{paragraph['speaker']}] {paragraph_text}")

        transcript = "\n".join(utterances)
        return dict(text=transcript)


class HuggingfaceEmbeddingModel(HasModel):
    # TODO @Performance: move embedding/HF model into our own cluster
    async def _endpoint(self, text: typing.Union[str, list[str]]) -> dict:
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
                rep = await response.json()
        if isinstance(rep, dict) and "error" in rep:
            if "gateway" in rep["error"].lower():
                error = TaskErrorType.TemporarilyUnavailable
            else:
                error = TaskErrorType.Unknown
            raise TaskError(error, self, rep["error"])
        elif not isinstance(rep, list):
            raise TaskError(TaskErrorType.Incapable, self, f"invalid response: {rep}")

        # quantize embeddings to [-128, 127] bytearray
        vector = np.array(rep, dtype=np.float32)
        vector = (vector * 128).clip(-128, 127).astype(np.int8)
        vector = [v.tolist() for v in vector]
        if not is_batched:
            vector = vector[0]

        return dict(vector=vector)


DEFAULT_MODULES: dict[str, Module] = {"symbolx.lib": symbolx_lib}
DEFAULT_DEPENDENCIES = ("symbolx.bench",)
