from __future__ import annotations

import enum
import json
import re
import typing
import uuid
from copy import deepcopy
from dataclasses import asdict, dataclass, is_dataclass
from json import JSONDecodeError
from typing import Optional, Self

from bench.bench import (
    Dataset,
    Expectation,
    HasExpectations,
    HasType,
    Model,
    Scope,
    Symbol,
    Type,
    TypeTag,
)
from bench.bench.core import Session, node
from bench.bench.model import (
    SETTINGS_CLS_BY_MODALITY,
    Modality,
    TextGenerationSettings,
    ValueT,
    XBlock,
    XBlockContent,
    XKind,
    XSource,
)
from bench.bench.type import (
    TypeBase,
    TypeFlag,
    TypeHint,
    check_type,
    instantiate_py_value_flat,
    map_value,
)
from bench.utils.utils import DotDict


@node
class Task(Symbol, HasType, HasExpectations):
    description: Optional[str] = None
    tag: TypeTag = TypeTag.FUNCTION
    _is_async: bool = True
    _implementations: dict[str, "XPrompt"] | None = None
    # should probably store last good implementation ... in redis?
    last_good_impl_idx: int = 0

    def _clear(self) -> None:
        Symbol._clear(self)
        HasType._clear(self)
        HasExpectations._clear(self)
        self._implementations = None

    def _interp(self, scope: Scope) -> None:
        HasType._interp(self, scope)
        HasExpectations._interp(self, scope)

    async def __call__(
        self,
        *args,
        build: str = None,
        model: Model | str = None,
        retries: int = None,
        cache: bool = None,
        timeout: float = None,
        **kwargs,
    ):
        # get candidate task implementations
        if model is not None:
            if isinstance(model, str):
                model = self.module.find_symbol(model, symbol_t=Model)
            models = [model]
        else:
            models = self.session.default_models
        candidates = []
        for model in models:
            cache_key = (self.id, model.id)
            if cache_key not in self._cached_implementations:
                implementation = build_task_implementation(self, model, self.session)
                self._cached_implementations[cache_key] = implementation
            candidates.append(self._cached_implementations[cache_key])
        # TODO @Broken: sort/filter implementations with some smartness
        impl_idx = self.last_good_impl_idx
        retries = retries if retries is not None else self.session.inference_retries
        remaining_retries = retries

        # actually run the task
        self.session.tracer.code_enter(self, args, kwargs)
        semantic_errors = []
        while remaining_retries >= 0:
            remaining_retries -= 1
            impl = candidates[impl_idx]
            log = self.session.logger.bind(
                task=self, retries=remaining_retries, implementation=impl
            )
            try:
                ret = await impl(*args, **kwargs, cache=cache, timeout=timeout)
                self.last_good_impl_idx = impl_idx
                self.session.tracer.code_exit(self, args, kwargs, ret)
                return ret
            except XGenerationError as e:
                semantic_errors.append(e)
                log.warning("task.failed", exc_info=e)
                if len(semantic_errors) <= self.session.inference_retries / len(candidates):
                    # retry with error info a few times
                    candidates[impl_idx] = impl.copy().emit(XConsiderError(e))
                else:
                    # fail over
                    impl_idx = (impl_idx + 1) % len(candidates)
                    semantic_errors = []
            except TimeoutError as e:
                # fail over
                self.session.logger.warning("task.failed", exc_info=e)
                impl_idx = (impl_idx + 1) % len(candidates)

        # give up
        errors_repr = "\n".join(str(e) for e in semantic_errors) if semantic_errors else "<timeout>"
        e = RuntimeError(f"{self} failed after {retries} retries: {errors_repr}")
        self.session.tracer.code_exception(self, args, kwargs, e)
        if semantic_errors:
            raise e from semantic_errors[-1]
        else:
            raise e

    def to_sync(self) -> "Self":
        if self.is_async:
            raise NotImplementedError  # nocheckin
        return self


class XGenerationErrorType(enum.StrEnum):
    TIMEOUT = "timeout"
    INVALID_JSON = "invalid_json"
    INVALID_TYPE = "invalid_type"
    UNKNOWN = "unknown"


class XGenerationError(ValueError):
    def __init__(self, type: XGenerationErrorType, message: str, path: str = None):
        super().__init__(message)
        self.type = type
        self.path = path


@dataclass(repr=False)
class XEmit:
    """Generate X blocks for models with dynamic code to manage dynamic values."""

    def __call__(self) -> XBlock | DynamicXBlock | list[XBlock | DynamicXBlock]:
        raise NotImplementedError


def xemit(func):
    # just forward to dataclass(repr=False, slots=True)
    return dataclass(repr=False, slots=True)(func)


ValueT = typing.TypeVar("ValueT", bound=typing.Any)


def xstatic(
    value: ValueT, source: XSource = XSource.Developer, path: str = None
) -> XBlockContent[ValueT]:
    return XBlockContent(kind=XKind.Static, source=source, value=value, path=path)


XInputHandler = typing.Callable[[XBlock, typing.Any], None]
XOutputHandler = typing.Callable[[typing.Any], typing.Any]


@dataclass
class DynamicXBlock:
    xblock: XBlockContent
    handler: XInputHandler | XOutputHandler


def build_task_implementation(task: Task, model: Model, session: Session) -> XPrompt:
    """Build the implementation for a task using some model."""
    # TODO @Broken: consider context length in X prompt planning/building
    if not task.outputs:
        raise RuntimeError(f"cannot build task {task} without output")
    expectations: list[Expectation] = [
        e for e in task.walk_expectations() if isinstance(e, Expectation)
    ]
    data_samples: list[Dataset] = [d for d in task.walk_expectations() if isinstance(d, Dataset)]
    x = XPrompt(task=task, model=model, modality=Modality.GenerateText, session=session)
    x.emit(
        XSystem(),
        XTypeSchema(type=task.type, type_label="Output", recursive=True),
    )
    if expectations:
        x.emit(XExpectations(task_label=task.name, expectations=expectations))
    for dataset in data_samples:
        if len(dataset) > 0:
            from bench.bench import ExpectationModifier

            positive = dataset.modifier == ExpectationModifier.LIKE
            x.emit(XSamples(dataset=dataset, task_label=task.name, positive=positive))
    x.emit(
        XTask(task=task),
        XTypeFabricatedSample(type=task.type, type_label="Output", is_output=True),
    )
    if task.type.inputs:
        x.emit(XInput(type=task.type))
    x.emit(
        # TODO @Broken: adjust & tune generation settings
        XEmitSettings(TextGenerationSettings(temperature=0.5, max_tokens=512, top_p=1.0)),
        XOutputText(type=task.type, type_label=f"output for task {task.name}"),
    )

    return x


class XPrompt:
    """Build a structured X prompt."""

    def __init__(self, task: Task, model: Model, modality: Modality, session: Session):
        self.task = task
        self.model = model
        self.modality = modality
        self.session = session
        # the actual prompt
        self.blocks: list[XBlock] = []
        self.input_handlers: dict[int, XInputHandler] = {}
        self.output_handler: XOutputHandler | None = None
        self.settings: typing.Any | None = None

    def __str__(self):
        return f"{self.model.fqn} {self.modality} ({len(self.blocks)})"

    def __repr__(self):
        return f"<XPrompt {self}>"

    def copy(self) -> XPrompt:
        x = XPrompt(self.task, self.model, self.modality, self.session)
        x.blocks = [b.copy() for b in self.blocks]
        x.input_handlers = {**self.input_handlers}
        x.output_handler = self.output_handler
        x.settings = deepcopy(self.settings)
        return x

    def emit(self, *emits: XEmit):
        blocks = []
        for emit in emits:
            x = emit()
            if isinstance(x, list):
                blocks.extend(x)
            else:
                blocks.append(x)
        for x in blocks:
            if isinstance(x, DynamicXBlock):
                if x.xblock.kind == XKind.Input:
                    self.input_handlers[len(self.blocks)] = x.handler
                    self.blocks.append(x.xblock)
                elif x.xblock.kind == XKind.Output:
                    if self.output_handler:
                        raise RuntimeError("cannot have multiple output handlers")
                    self.output_handler = x.handler
                    # not added to blocks since it's not a real xblock
                else:
                    raise RuntimeError(f"cannot have dynamic x block of kind {x.xblock.kind}")
            elif x.kind == XKind.Settings:
                if self.settings:
                    raise RuntimeError("cannot have multiple settings")
                settings_cls = SETTINGS_CLS_BY_MODALITY[self.modality]
                self.settings = settings_cls(**x.value)
            else:
                self.blocks.append(x)
        return self

    async def __call__(
        self, *args, cache: bool = None, timeout: float = None, **kwargs
    ) -> typing.Any:
        inputs = {**kwargs}  # combine inputs from args/kwargs
        for input_t, input in zip(self.task.type.inputs, args):
            inputs[input_t.name] = input
        # copy x blocks to impute dynamic inputs
        blocks_copy = [xblock.copy() for xblock in self.blocks]
        # apply dynamic inputs
        for i, impute in self.input_handlers.items():
            impute(blocks_copy[i], inputs)
        try:
            outputs = await self.model.inference(
                self.modality, blocks_copy, self.settings, cache=cache, timeout=timeout
            )
        except TimeoutError as e:
            raise XGenerationError(XGenerationErrorType.TIMEOUT, "model backend timed out") from e
        except Exception as e:
            raise XGenerationError(XGenerationErrorType.UNKNOWN, "model backend failed") from e
        return self.output_handler(outputs)


@dataclass(repr=False)
class XSystem(XEmit):
    """Emits the system message about general expectations for JSON."""

    message: str = (
        "You are a precise and concise assistant."
        " Perform the given tasks following the instructions to the letter."
        " If the task is underspecified or ambiguous, guess without asking."
        " Output valid JSON as dictated by the type schema."
    )

    def __call__(self) -> XBlock:
        return xstatic(self.message, XSource.System)


@xemit
class XTask(XEmit):
    """Emits the task exactly as written"""

    task: Task
    task_label: str = None
    include_description: bool = True

    def __call__(self) -> XBlock:
        text = f"Task {self.task_label or self.task.name}:"
        if self.include_description:
            text += f" {self.task.description}"
        return xstatic(text, XSource.Developer)


@xemit
class XExpectations(XEmit):
    """Emits the expectation exactly as written"""

    task_label: str
    expectations: list[Expectation]

    def __call__(self) -> XBlock:
        expectation_strs = [
            f" - {expectation.name}: {expectation.description}" for expectation in self.expectations
        ]
        return xstatic(
            f"For task {self.task_label}, you must consider:\n" + "\n".join(expectation_strs),
            XSource.Developer,
        )


@xemit
class XSamples(XEmit):
    """Emits fewshot examples in a specific format"""

    dataset: Dataset
    task_label: str
    positive: bool

    def __call__(self) -> XBlock:
        if len(self.dataset) == 0:
            raise RuntimeError(f"expected at least one sample for {self.task.name}")
        if self.positive:
            preamble = f"Good examples of {self.task_label}"
        else:
            preamble = f"Bad examples of {self.task_label} (don't do this!)"
        data_str = "\n".join(json.dumps(record.data, sort_keys=True) for record in self.dataset)
        return xstatic(f"{preamble}:\n{data_str}", XSource.Developer)


@xemit
class XTypeSchema(XEmit):
    """Emits the type exactly as written"""

    type: Type
    type_label: Optional[str]
    recursive: bool

    def __call__(self) -> XBlock:
        bench_lines = []
        seen_types: set[uuid.UUID] = set()  # TODO @Cleanup: seen types dedup shouldn't be needed
        for node in self.type.walk_type(include_references=True):
            if node.id in seen_types:
                continue
            seen_types.add(node.id)
            if node.reference is not None:
                continue  # skip the link
            if node.tag in (TypeTag.STRUCT, TypeTag.FUNCTION, TypeTag.ENUM, TypeTag.UNION):
                # nocheckin: render type schema properly depending on model backend
                line = render_statement(node.source, include_content=node.tag != TypeTag.FUNCTION)
                bench_lines.append(line)
        bench_str = "\n\n".join(bench_lines)
        schema_str = f"Type schemas you must adhere to. Do not invent new fields or options. ? = optional:\n{bench_str}".strip()
        return xstatic(schema_str, XSource.Developer)


@xemit
class XTypeFabricatedSample(XEmit):
    """Emits a single sample output of the given type (default to fabricate)"""

    type: Type
    type_label: Optional[str]
    is_output: bool

    def __call__(self) -> list[XBlock]:
        fabricated_sample = fabricate_value(self.type, is_output=self.is_output)
        sample_declaration = xstatic(
            f"Example {self.type_label or self.type.name} with fabricated values:",
            XSource.System,
        )
        sample = xstatic(json.dumps(fabricated_sample, sort_keys=True), XSource.Developer)
        return [sample_declaration, sample]


@xemit
class XInput(XEmit):
    """Emits the code to input the given type"""

    type: Type
    type_label: str = "Input"
    path: str = ""

    def impute_input(self, input: XBlock, value: Any) -> None:
        input.value = json.dumps(value, sort_keys=True)

    def __call__(self) -> list[XBlock | DynamicXBlock]:
        input_declaration = xstatic(f"{self.type_label}:", XSource.System)
        input = XBlockContent(kind=XKind.Input, source=XSource.User, value=None, path=self.path)
        return [input_declaration, DynamicXBlock(input, self.impute_input)]


@xemit
class XOutputText(XEmit):
    """Emits the code to request and read generated output of the given type"""

    type: Type
    type_label: str = "Output"
    path: str = ""

    def parse_output(self, output: str):
        # escape/try to parse the output if needed (handles trivial model confusions)
        value = output.strip()
        if not value.startswith("{"):
            # sometimes the model prefixes the output with some explanation, find the { ... }
            value = re.compile(r"\{.*}", re.DOTALL).search(value)
            if value:
                value = value.group(0)
            else:
                raise XGenerationError(
                    XGenerationErrorType.INVALID_JSON,
                    f"output does not contain JSON object: {output}",
                )

        # escape strings with multiline content
        # these aren't technically valid JSON, but they're very useful for model output
        def sub_multiline_str(match):
            # replace line breaks with \n escape sequence
            modified_string = match.group(1).replace("\n", "\\n").replace("\r", "")
            return f'"{modified_string}"'

        value = re.compile(r'"(.*?)(?<!\\)"', re.DOTALL).sub(sub_multiline_str, value)

        try:
            ret = json.loads(value)
            ret = map_value(
                ret,
                self.type,
                map_v=instantiate_py_value_flat,
                is_output=True,
                ignore_outer_map=True,
            )
            check_type(ret, self.type, is_output=True)
            ret = DotDict(**ret)  # behave like a typed dict
            return ret
        except Exception as e:
            if isinstance(e, JSONDecodeError):
                error_type = XGenerationErrorType.INVALID_JSON
            elif isinstance(e, TypeError):
                error_type = XGenerationErrorType.INVALID_TYPE
            else:
                error_type = XGenerationErrorType.UNKNOWN
            raise XGenerationError(
                type=error_type, message=f"output is invalid for {self.type}: {e}", path=None
            ) from e

    def __call__(self) -> list[XBlock | DynamicXBlock]:
        output_keys = ", ".join(t.name for t in self.type.outputs)
        output_request = xstatic(
            f"Generate {self.type_label} given the inputs and instructions - a JSON object with keys [{output_keys}], starting with {{",
            XSource.System,
        )
        output = XBlockContent(kind=XKind.Output, source=XSource.Model, value=None, path=self.path)
        return [output_request, DynamicXBlock(output, self.parse_output)]


@xemit
class XConsiderError(XEmit):
    """Emits a note about an error that occured previously"""

    error: XGenerationError

    def __call__(self) -> XBlock:
        error_str = str(self.error)
        # remove (source=...) from error message
        error_str = re.sub(r"\(source=.+\)", "", error_str)
        return xstatic(f"Note: please avoid mistakes like this: {error_str}", XSource.System)


@xemit
class XEmitSettings(XEmit):
    settings: typing.Any

    def __call__(self) -> XBlock:
        if is_dataclass(self.settings):
            value = asdict(self.settings)
        return XBlockContent(
            kind=XKind.Settings, source=XSource.System, value=self.settings, path=None
        )

    @property
    def sources(self) -> list[Symbol]:
        return []


SAMPLE_BY_TYPE_HINT = {
    TypeHint.UUID: str(uuid.uuid4()),
    TypeHint.NAME: "Max Mustermann",
    TypeHint.EMAIL: "florian@symbolx.com",
    TypeHint.PHONE: "+49 123 456 789",
    TypeHint.URL: "https://symbolx.com",
    TypeHint.KEY: "sk_test_1234567890",
    TypeHint.DATE: "2023-01-01",
    TypeHint.DATETIME: "2023-01-01T10:30:45",
    TypeHint.TIME: "02:08:00",
    TypeHint.RATING: 3,
}


def fabricate_value(type: TypeBase, skip_array: bool = False, is_output: bool = None) -> typing.Any:
    """Synthesizes a value of the given type with fake fields."""
    if type.flags & TypeFlag.IsArray and not skip_array:
        return [fabricate_value(type, skip_array=True)]
    if SAMPLE_BY_TYPE_HINT.get(type.hint) is not None:
        return SAMPLE_BY_TYPE_HINT[type.hint]
    elif type.tag == TypeTag.STRING:
        return "lorem ipsum"
    elif type.tag == TypeTag.NUMBER:
        return 42
    elif type.tag == TypeTag.BOOLEAN:
        return False
    elif type.tag == TypeTag.ENUM:
        if len(type.fields) == 0:
            return None
        return type.fields[0].name
    elif type.tag == TypeTag.STRUCT or type.tag == TypeTag.FUNCTION:
        return {
            subtype.name: fabricate_value(subtype)
            for subtype in type.fields
            if is_output is None or bool(subtype.flags & TypeFlag.IsOutput) == is_output
        }
    elif type.tag == TypeTag.UNION:
        return fabricate_value(type.fields[0])
    elif type.tag == TypeTag.NULL:
        return None
    elif type.tag == TypeTag.LITERAL:
        return type.name  # assumes enum string literals
    elif type.tag == TypeTag.ANY:
        return 42  # not sure what to do here
    else:
        raise RuntimeError(f"unexpected type {type.tag}")
