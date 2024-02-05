import abc
import asyncio
import enum
import json
import random
import re
from dataclasses import dataclass
from json import JSONDecodeError
from typing import TYPE_CHECKING, Any, Literal, Optional, Union

import structlog

from bench.language.const import NodeType, RunErrorKind, RunStatus
from bench.language.field import Field, TypedDict
from bench.language.model import HasModel
from bench.language.node import Node, ScopeNode, node_component, p_runtime
from bench.language.projection import Projection
from bench.language.render import render
from bench.language.session import Run
from bench.utils.func import describe_type
from bench.utils.utils import format_python, omit_empty

if TYPE_CHECKING:
    from bench.language.block import Block
    from bench.language.notice import NoticeHandler

logger = structlog.get_logger(__name__)


@node_component
class HasTask(Node):
    _root_models: list["HasModel"] | None = p_runtime(default=None)
    _randomize: bool = p_runtime(default=False)

    @property
    def _is_async(self):
        return True

    def _clear_inner(self, scope: Optional[ScopeNode] = None) -> None:
        self._root_models = None
        self._randomize = False

    def _interp_inner(self, scope: ScopeNode, on_notice: "NoticeHandler") -> None:
        from . import symbolx_lib

        randomize_tag = symbolx_lib.resolve(".builtins.randomize")
        self._randomize = randomize_tag in self.tags

        if not any(f.flags & TypeFlag.IS_OUTPUT for f in self.fields):
            on_notice(subject=self, type=IssueType.TASK_MISSING_IO)
        # TODO @UX @Task: interp task feasibility
        #  - check if task is possible given the fields, models & available blocks

    async def _call_inner_async(
        self,
        *args,
        # :TaskConfig
        cache: bool = None,
        nonce: str = None,
        mode: Literal["fast", "deliberate", "auto"] = "auto",
        **kwargs,
    ):
        # prepare inputs
        nonce = nonce or (str(random.randint(0, 2**16)) if self._randomize else None)
        kwargs["cache"] = cache
        kwargs["nonce"] = nonce
        kwargs["mode"] = mode
        inputs = self._inputs_from_args(args, kwargs)

        # shortcut for built-in tasks with fixed implementations
        if self.path == "symbolx.lib.builtins.embed":
            builtin_model: "Block" = self.session.package.resolve(
                "huggingface.lib.text.llm-embedder"
            )
        elif self.path == "symbolx.lib.builtins.transcribe":
            builtin_model: "Block" = self.session.package.resolve("deepgram.lib.audio.nova-2")
        else:
            builtin_model = None
            if mode == "auto":
                mode = "fast"
            if mode == "fast":
                models = ["gpt3-turbo"]
            else:
                models = ["gpt4-turbo"]
            models = [self.session.package.resolve(m) for m in models]

        # do task
        projection = Projection(self.package)
        seen_from_node = projection.view_node(self, ancestors_up_to=NodeType.FILE, max_distance=5)
        await projection.view_records(seen_from_node.values(), limit=10)
        seen_from_value = projection.view_value(inputs, self, is_output=False)
        projection.view_node(
            seen_from_value.values(), ancestors_up_to=NodeType.FILE, max_distance=2
        )
        await projection.view_records(seen_from_value.values(), limit=10)

        self.session._run_enter(self, is_async=True, inputs=inputs)
        try:
            if builtin_model:
                # passthrough model
                inputs = {k: v for k, v in inputs.items() if k not in ("cache", "nonce", "mode")}
                outputs = await run_builtin_task(self, builtin_model, inputs)
            else:
                outputs = await run_task(self, projection, inputs, nonce, models)
        except BaseException as e:
            self.session._run_exception(self, e)
            raise
        self.session._run_exit(self, outputs)
        return outputs


class TaskErrorType(enum.StrEnum):
    Incapable = "Incapable"
    Timeout = "Timeout"
    InvalidFormat = "InvalidFormat"
    InvalidType = "InvalidType"
    ExceededLimit = "ExceededLimit"
    Unavailable = "Unavailable"
    TemporarilyUnavailable = "TemporarilyUnavailable"
    Unknown = "Unknown"


UNRECOVERABLE_ERRORS = {TaskErrorType.Incapable, TaskErrorType.Unknown}
TASK_MODEL_ATTEMPTS = 3
TASK_TOTAL_ATTEMPTS = 10
BUILTIN_TASK_MODEL_ATTEMPTS = 5


async def run_builtin_task(task: HasTask, model: "HasModel", inputs: dict):
    retries = 0
    while retries < BUILTIN_TASK_MODEL_ATTEMPTS:
        try:
            outputs = await model(**inputs)
            # trim output to own outputs
            if isinstance(outputs, dict):
                outputs = {k: v for k, v in outputs.items() if k in task.fields}
            if not isinstance(outputs, TypedDict):
                outputs = TypedDict(outputs, task, is_output=True)
            return outputs
        except TaskError as e:
            if e.type in UNRECOVERABLE_ERRORS:
                raise
            else:
                retries += 1
                await asyncio.sleep((retries + 1) ** 2)
                continue
    raise TaskError(
        TaskErrorType.ExceededLimit,
        model,
        f"could not solve task in {BUILTIN_TASK_MODEL_ATTEMPTS} attempts",
    )


async def run_task(
    task: HasTask,
    projection: Projection,
    inputs: dict,
    nonce: Optional[str],
    models: list["Block"] = None,
) -> dict:
    from bench.language.value import check_type, unpack_value

    total_attempts = 0
    model_attempts = 0
    # in priority order
    model_idx = 0
    log = logger.bind(task=task, inputs=describe_type(inputs), nonce=nonce, models=models)

    previous_results = []
    last_error = None
    while (
        model_attempts < TASK_MODEL_ATTEMPTS
        and total_attempts < TASK_TOTAL_ATTEMPTS
        and model_idx < len(models)
    ):
        task.current_run.value.retries = model_attempts
        # run task step
        model = models[model_idx]
        compiler = model.compiler  # models may share a compiler
        compiled = await compiler.compile(task, projection, inputs, previous_results, nonce)
        if not compiler.can_run(model, compiled):
            model_idx += 1
            model_attempts = 0
            continue  # try next model, not an error, just not capable

        # try model
        model_attempts += 1
        total_attempts += 1
        run_capture = task.session.capture_runs()
        try:
            log.debug("task.run", model=model, compiled=compiled, attempt=model_attempts)
            run_name = f"{task.name} #{total_attempts}"
            with task.session.bind_run_value(retry=model_attempts, nonce=nonce, name=run_name):
                outputs = await compiler.run(model, compiled)

            # done, terminate
            # unpack -> check is not ideal since it doesn't let us collect unpack errors nicely
            outputs = unpack_value(
                outputs, task, is_output=True, map_k=lambda f: (f.py_ident, f.py_ident)
            )
            check_type(outputs, task, is_output=True)
            return TypedDict(outputs, task, is_output=True)
        except Exception as e:
            e = TaskError.from_exception(task, e)
            last_error = e
            logger.debug("task.error", error=e)
            if e.type in UNRECOVERABLE_ERRORS:
                model_idx += 1
                model_attempts = 0
            elif e.type == TaskErrorType.ExceededLimit:
                await asyncio.sleep(0.1)
            else:
                previous_results.append(e)

            run = run_capture.stop_one_or_none()
            if run is not None:
                run.value.verdict = "reject"
                run.value.verdict_reason = str(e)

            continue

    if total_attempts == 0:
        # no models
        raise TaskError(TaskErrorType.Incapable, task, "cannot solve task with given input types")
    else:
        raise TaskError(
            TaskErrorType.ExceededLimit,
            task,
            f"could not solve task in {TASK_MODEL_ATTEMPTS} attempts across {len(models)} models:\n{last_error or '<no details>'}",
        )


# avoid circular import
from .run import RunError  # noqa: E402


class TaskError(RunError):
    def __init__(
        self,
        type: TaskErrorType,
        block: "Block",
        message: str = None,
        path: str = None,
    ):
        super().__init__(
            kind=RunErrorKind.RUNTIME,
            type=type.name,
            block=block,
            message=f"{type.value}: {message}",
        )
        self.type = type
        self.path = path

    @staticmethod
    def from_exception(task: "Block", e: Exception, path: str = None) -> "TaskError":
        if isinstance(e, TaskError):
            return e
        elif isinstance(e, ValueError):
            return TaskError(TaskErrorType.InvalidFormat, task, str(e), path)
        elif isinstance(e, TypeError):
            return TaskError(TaskErrorType.InvalidType, task, str(e), path)
        else:
            return TaskError(TaskErrorType.Unknown, task, str(e), path)


class IncapableError(TaskError):
    def __init__(self, message: str = None, path: str = None):
        super().__init__(TaskErrorType.Incapable, message, path)


class LimitExceededError(TaskError):
    def __init__(self, message: str = None, path: str = None):
        super().__init__(TaskErrorType.ExceededLimit, message, path)


@dataclass
class CompiledInput(abc.ABC):
    task: HasTask


class TaskCompiler(abc.ABC):
    async def compile(
        self,
        task: HasTask,
        projection: Projection,
        inputs: dict,
        previous_results: list[Union[TaskError, "Run"]],
        nonce: Optional[str],
    ) -> CompiledInput:
        raise NotImplementedError

    def can_run(self, model: "Block", input: CompiledInput) -> bool:
        raise NotImplementedError

    async def run(self, model: "Block", input: CompiledInput) -> dict:
        raise NotImplementedError


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


def _type_to_json_schema(
    type: Union[Field, "Block"], ignore_array: bool = False, is_output: bool = None
) -> JsonSchemaElement:
    """Convert a Bench type to a JSON schema element."""
    fields = [
        f
        for f in type.fields
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
    _SYSTEM_MESSAGE = (
        "You are a precise and highly capable bot that can do almost anything a user asks."
        " Interpret inputs generously and attentively, be concise, be considerate."
        " You are accessed through an API, so don't respond to the user directly."
        " When given examples to consider, don't copy them directly unless explicitly asked."
    )

    def _render_value_flat(self, value: Any, type: Union[Field, "Block"], *args, **kwargs) -> Any:
        """Model-friendly rendering of instantiated value."""
        raise NotImplementedError

    async def _render_context(
        self, task: "Block", projection: Projection, *, exclude_output: bool
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
        from bench.language.value import map_value

        if run.status == RunStatus.FAILED:
            return self._compile_error_text(run)
        else:
            run_inputs_str = json.dumps(
                map_value(run.inputs_packed, run.node, map_v=self._render_value_flat), indent=2
            )
            run_outputs_str = json.dumps(
                map_value(run.outputs_packed, run.node, map_v=self._render_value_flat), indent=2
            )
            return f"Previous result for '{run.node.py_ident}' given '{run_inputs_str}':\n {run_outputs_str}"

    def _compile_error_text(self, error: RunError | TaskError) -> str:
        return f"Avoid previous error: {self._render_error(error)}"

    async def _prepare_text_prompt(
        self,
        task: "Block",
        projection: Projection,
        inputs: dict,
        previous_results: list[Union[TaskError, "Run"]],
        nonce: Optional[str],
    ) -> str:
        from bench.language.value import render_value

        # system wrapper
        messages: list[str] = [
            self._SYSTEM_MESSAGE,
            f"Your main task is '{task.name}'.",
        ]

        # package context
        context_str = await self._render_context(task, projection, exclude_output=True)
        if context_str:
            messages.append(
                f"The definition of task '{task.name}':\n {context_str}"
                f"\nFollow the above context carefully w.r.t. to the following inputs."
            )

        # inputs
        inputs_strs = []
        for field_ in task.fields:
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
        output_fields = [f for f in task.fields if f.flags & TypeFlag.IS_OUTPUT]
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
    def _parse_text_completion(model: "Block", task: "Block", completion: str) -> dict:
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
                field = task.fields.get(match.group("key"))
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


# class OpenAITextCompiler(BaseTextTaskCompiler):
#     async def compile(
#         self,
#         task: Block,
#         projection: Projection,
#         inputs: dict,
#         previous_results: list[Union[TaskError, "Run"]],
#         nonce: Optional[str],
#     ) -> OpenAIChatInput:
#         prompt = await self._prepare_text_prompt(task, projection, inputs, previous_results, nonce)
#         messages = [dict(role="user", content=prompt)]
#
#         # settings
#         settings = dict(temperature=0.8)
#
#         return OpenAIChatInput(
#             task=task,
#             settings=settings,
#             messages=messages,
#             functions=None,
#             blocks_by_name={},
#         )
#
#     def can_run(self, model: "Block", input: OpenAIChatInput) -> bool:
#         context_window: int = {
#             "gpt3-turbo": 16 * 1024,
#             "gpt4-turbo": 128 * 1024,
#         }[model.name]
#         return input.tokens <= context_window
#
#     async def run(
#         self,
#         model: OpenAIChatCompletionModel,
#         input: OpenAIChatInput,
#     ) -> dict:
#         rep = await model(
#             messages=input.messages, functions=input.functions, settings=input.settings
#         )
#         msg: OpenAIChatMessage = rep.message
#         completion = msg.content.strip()
#         return self._parse_text_completion(model, input.task, completion)
