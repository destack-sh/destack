import abc
from dataclasses import dataclass
from typing import Any, ClassVar, Literal, Mapping, Union, assert_never, cast, override

import anthropic
import openai
import regex
import structlog
from openai.types import chat as openai_chat_types
from opentelemetry import trace

from bench.language.code import Code
from bench.language.file import (
    File,
    FileFormat,
    FileInfoBase,
    FileReference,
    FileType,
    download_batch,
)
from bench.language.project import Projection, ProjectOptions, project
from bench.language.render import Aliasing, RenderOptions, render, render_value_expr
from bench.language.run import ModelProvider, ModelType, RunKind
from bench.language.value import sample_value
from bench.runtime.core import RUN_ONCE, ModelFailedError, ModelIncapableError, RunImpossibleError
from bench.runtime.runner import RunnableNode, Runner, RunnerCache
from bench.utils.utils import get_from_env

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


class TextRunner(Runner):
    """The router for text functions without explicitly assigned models/providers."""

    @override
    async def run_once(self) -> None:
        runner = await self.runtime.make_runner(
            RunKind.TEXT,
            node=self.node,
            options=RUN_ONCE.override(model_provider=ModelProvider.OPENAI),
            inputs=self.inputs,
            track=False,
        )
        await self.runtime.run_runner(runner)
        self.outputs = runner.outputs


#
# Chat models
#


@dataclass
class PageContext:
    path: str
    body: str


@dataclass
class ChatMessage:
    role: Literal["user", "assistant", "system"]
    content: list["ChatMessageContent"]


@dataclass
class ChatMessageTextContent:
    text: str


@dataclass
class ChatMessageFileContent:
    file: FileInfoBase


ChatMessageContent = Union[ChatMessageTextContent, ChatMessageFileContent]


class ChatModelRunnerBase(Runner[RunnerCache, RunnableNode], abc.ABC):
    """
    The base for chat-like text function runners.
    Basically, the model generates code that produces the answer, we run it and return that.
    """

    SYSTEM_MESSAGE = """\
# You are an obedient AI emulator on a new development platform called Bench.
# The nodes in a Bench program are represented with a Python ORM (read & write).
# You MUST always answer with valid Python statements that include a 'return'. 
# Use good formatting, watch your commas and escaping.
"""
    USER_POSTFIX_MESSAGE = """\
#
# Return the answer to the specific invocation of '{task_alias}' with the given inputs.
#  - You MUST NOT attempt to generalize over inputs; return the answer for the specific inputs only.
#  - You SHOULD solve math, simple string operations and such in Python when appropriate, but nothing fancy.
#  - You MUST use your native capabilities (NOT Python) to emulate AI stuff (like image processing or summarization).
#  - You SHOULD `raise ModelIncapableError("<reason>")` IF an output for the given inputs is impossible.
#    (e.g., error if the given output Field's type don't agree with their names/descriptions or instructions)
# 

# 
# Some general (simplified) examples 
#

# Example: returning a single value directly (no need to wrap in dict for one output field)
return 7

# Example: using Python to compute the answer directly since it's easy in code
return {
    'Count': String.count(Pattern)
} 

# Example: using Python to stage and help with the answer (with proper escaping)
EntitiesInImage = ['John', 'Mary\'s Dog']
return {
    'Entities In Image': EntitiesInImage, 
    'Num Entities': len(Persons),
    'Scene Description': md("*John* and *Mary* are standing in front of someone's a house"),
}

# Example: giving the answer directly, putting rationale before output (even if field order differs)
Explanation = '''\
To detect whether something is a prime number, we have to check if it is divisible by any number other than 1 and itself. 
If it is divisible by any number other than 1 and itself, then it is not a prime number.
Otherwise, it is a prime number.
'''
return {
    "Improved Code": code('''\
if n <= 1:
    return False
for i in range(2, int(n**0.5) + 1):
    if n % i == 0:
        return False
return True
'''),
    "Explanation": Explanation,
}
"""
    ASSISTANT_PREFIX_MESSAGE = """\
#
"""

    @property
    def model(self) -> ModelType | None:
        if self.options.model_type is not None:
            return self.options.model_type
        else:
            return None

    @property
    def task_alias(self) -> str:
        return self.node.absolute_path

    @override
    async def run_once(self) -> None:
        projection = project(self.node, self.inputs, options=ProjectOptions())
        render_options = RenderOptions(scope=self.node, aliasing=Aliasing())
        log = logger.bind(runner=self, projection=projection)

        with tracer.start_as_current_span("text.prepare_input"):
            try:
                messages = await self._prepare_input(projection, render_options)
                log.trace("text.prepare_input", messages=messages, span="current")
            except BaseException as e:
                log.trace("text.prepare_input.error", exc_info=e, span="current")
                raise

        with tracer.start_as_current_span("text.generate_output"):
            try:
                code = await self._generate_output(messages)
                log.trace("text.generate_code", code=code, span="current")
            except BaseException as e:
                log.trace("text.generate_code.error", exc_info=e, span="current")
                raise

        code = Code.from_string(code)
        with tracer.start_as_current_span("text.run_code"):
            code_runner = await self.runtime.make_runner(
                RunKind.CODE,
                node=self.node,
                code=code,
                options=RUN_ONCE,
                track=self.is_tracked,
                inputs=self.inputs,
            )
            try:
                await self.runtime.run_runner(code_runner)
            except Exception as e:
                raise ModelFailedError(f"failed to run code: {e}") from e
        self.outputs = code_runner.outputs

    async def render_system_message(
        self, projection: Projection, render_options: RenderOptions
    ) -> tuple[ChatMessageContent, ...]:
        """Renders the system message for the projection"""
        return (ChatMessageTextContent(self.SYSTEM_MESSAGE),)

    async def render_page_context(
        self, projection: Projection, render_options: RenderOptions
    ) -> tuple[ChatMessageContent, ...]:
        """Renders the source context pages from the projection"""
        pages = projection.get_containing_pages()
        rendered_pages = []
        for page in pages:
            rendered_page = render(page, options=render_options.replace(scope=page, as_page=True))
            rendered_context = f"""\
# {'=' * 24}
# `{page.absolute_path}`
# {'=' * 24}

{rendered_page}
"""
            rendered_pages.append(rendered_context)
        rendered = ChatMessageTextContent(f"""\
# 
# Context for '{self.task_alias}' (may include irrelevant information!)
#

{'\n\n'.join(rendered_pages) or "# <no context available>"}
""")
        return (rendered,)

    async def render_file_context(
        self,
        projection: Projection,
        render_options: RenderOptions,
        *,
        native_types: tuple[FileType, ...],
        native_formats_by_type: Mapping[FileType, tuple[FileFormat, ...]] = {},
        image_max_pixels: int = 2000,
        image_max_size: int = 2 * 1024 * 1024,
    ) -> tuple[ChatMessageContent, ...]:
        """
        Renders the context files from the projection, converting into native formats.
        """
        processed_types = (FileType.TEXT, FileType.CODE, FileType.IMAGE, FileType.DOCUMENT)

        raw_files: list[File | FileReference] = projection.get_nodes_like(File, FileReference)
        if not raw_files:
            return ()

        # download files
        _ = await download_batch(
            raw_files,
            include_content=[f for f in raw_files if f.coarse_type in processed_types],
        )

        # convert/preprocess files
        # NOTE :Performance: cache preprocessed files in the session/some other cache?
        preprocessed_files: list[FileInfoBase] = []
        for file in raw_files:
            # downscale large images to smaller JPEGs
            if file.coarse_type == FileType.IMAGE and (
                (file.width or 0) > image_max_pixels
                or (file.height or 0) > image_max_pixels
                or file.size > image_max_size
            ):
                file = await file.downscale(max_pixels=image_max_pixels, max_size=image_max_size)

            # convert files to native types
            if file.coarse_type not in native_types:
                # NOTE :Incomplete: automap non-document files (e.g. Audio->Text?)
                assert FileType.TEXT in native_types, f"{self!r} has no text type"
                file = await file.convert(FileFormat.MARKDOWN)

            # convert files to native format (if there are specific formats)
            if (
                native_formats_by_type.get(file.coarse_type)
                and file.format not in native_formats_by_type[file.coarse_type]
            ):
                file = await file.convert(native_formats_by_type[file.coarse_type][0])

            preprocessed_files.append(file)

        # turn into chat message contents
        aliasing = render_options.aliasing
        assert aliasing is not None, f"no aliasing for {self!r}"
        contents: list[ChatMessageContent] = []
        for file in preprocessed_files:
            preamble = ChatMessageTextContent(f"""\
#
# {aliasing.get_or_add(file.original)}: '{file.title}' 
# '{file.mime_type}' 
# 
""")
            content = ChatMessageFileContent(file)
            contents.extend((preamble, content))
        return tuple(contents)

    async def render_task(
        self, projection: Projection, render_options: RenderOptions
    ) -> tuple[ChatMessageContent, ...]:
        """Renders the task from the projection"""
        rendered_task = render(self.node, options=render_options)
        if self.inputs is not None:
            rendered_inputs = render(self.inputs, options=render_options)
        else:
            rendered_inputs = "# <no inputs>"

        rendered = ChatMessageTextContent(f"""\
#
# Inputs for '{self.task_alias}'
# 

{rendered_inputs}

# 
# You are emulating one invocation of '{self.task_alias}' for the above inputs.
#

{rendered_task}
""")
        return (rendered,)

    async def render_examples(
        self, projection: Projection, render_options: RenderOptions
    ) -> tuple[ChatMessageContent, ...]:
        """Renders relevant examples for the task / context"""
        if self.output_type is None or len(self.output_type._fields) == 0:
            raise RunImpossibleError(f"no output fields for {self!r}")

        rendered_example = {}
        for field in self.output_type._fields:
            field_value = sample_value(field)
            rendered_example[field.code_name] = render_value_expr(
                field_value, field, options=render_options
            )
        rendered_example_parts = [f"    '{k}': {v}" for k, v in rendered_example.items()]
        rendered_example_str = f"{{\n{',\n'.join(rendered_example_parts)}\n}}"

        rendered = ChatMessageTextContent(f"""\
# 
# A syntactically valid answer to '{self.task_alias}' looks something like this:
# 
                                          
return {rendered_example_str}
""")
        return (rendered,)

    @abc.abstractmethod
    async def _prepare_input(
        self, projection: Projection, render_options: RenderOptions
    ) -> list[ChatMessage]:
        """Generates the chat messages as input to the model."""
        ...

    @abc.abstractmethod
    async def _generate_output(self, messages: list[ChatMessage]) -> str:
        """Generates the code output from the model."""
        ...


openai_client = openai.AsyncClient(
    api_key=get_from_env("OPENAI_API_KEY", description="OpenAI API key")
)
anthropic_client = anthropic.AsyncClient(
    api_key=get_from_env("ANTHROPIC_API_KEY", description="Anthropic API key")
)


class OpenaiModelRunner(ChatModelRunnerBase):
    DEFAULT_MODEL = ModelType.OPENAI_GPT4_0
    MODEL_BY_TYPE: ClassVar[Mapping[ModelType, str]] = {
        ModelType.OPENAI_GPT4_0: "gpt-4o-2024-08-06",
        ModelType.OPENAI_GPT4_O_MINI: "gpt-4o-mini-2024-07-18",
        ModelType.OPENAI_O1_MINI: "o1-mini-2024-09-12",
        ModelType.OPENAI_O1_PREVIEW: "o1-preview-09-12",
    }

    @override
    async def _prepare_input(
        self, projection: Projection, render_options: RenderOptions
    ) -> list[ChatMessage]:
        system_message = ChatMessage(
            role="system",
            content=[*(await self.render_system_message(projection, render_options))],
        )
        file_context = await self.render_file_context(
            projection,
            render_options,
            native_types=(FileType.TEXT, FileType.CODE, FileType.IMAGE),
        )
        user_message = ChatMessage(
            role="user",
            content=[
                *(await self.render_page_context(projection, render_options)),
                *file_context,
                *(await self.render_task(projection, render_options)),
                *(await self.render_examples(projection, render_options)),
            ],
        )
        assistant_message = ChatMessage(
            role="assistant",
            content=[ChatMessageTextContent(self.ASSISTANT_PREFIX_MESSAGE)],
        )
        return [system_message, user_message, assistant_message]

    async def _convert_message(
        self, message: ChatMessage
    ) -> openai_chat_types.ChatCompletionMessageParam:
        # (for some reason we need to check each message.role separately for typechecking)
        if message.role == "system":
            # must be all text
            assert all(isinstance(c, ChatMessageTextContent) for c in message.content)
            return {
                "role": "system",
                "content": "\n\n".join(
                    cast(ChatMessageTextContent, c).text for c in message.content
                ),
            }
        elif message.role == "assistant":
            # must be all text
            assert all(isinstance(c, ChatMessageTextContent) for c in message.content)
            return {
                "role": "assistant",
                "content": "\n\n".join(
                    cast(ChatMessageTextContent, c).text for c in message.content
                ),
            }
        elif message.role == "user":
            contents: list[openai_chat_types.ChatCompletionContentPartParam] = []
            for content in message.content:
                if isinstance(content, ChatMessageTextContent):
                    if not content.text:
                        continue
                    contents.append({"type": "text", "text": content.text})
                elif isinstance(content, ChatMessageFileContent):
                    if content.file.coarse_type in (FileType.TEXT, FileType.CODE):
                        if not content.file.text:
                            continue
                        contents.append({"type": "text", "text": content.file.text})
                    elif content.file.coarse_type == FileType.IMAGE:
                        contents.append(
                            {
                                "type": "image_url",
                                "image_url": {
                                    "url": f"data:{content.file.mime_type};base64,{content.file.b64encode()}"
                                },
                            }
                        )
                    else:
                        raise ModelIncapableError(f"cannot convert output {content.file!r}")
                else:
                    assert_never(content)

            return {"role": message.role, "content": contents}
        else:
            assert_never(message.role)

    @override
    async def _generate_output(self, messages: list[ChatMessage]) -> str:
        model = self.model or self.DEFAULT_MODEL
        model_key = self.MODEL_BY_TYPE.get(model)
        if model_key is None:
            raise RunImpossibleError(f"unsupported model type {model}")
        converted_messages = [await self._convert_message(message) for message in messages]
        completion = await openai_client.chat.completions.create(
            messages=converted_messages,
            model=model_key,
            temperature=0.1,
            user=str(self.runtime.package.id),
        )
        completion_text = completion.choices[0].message.content
        if not completion_text:
            raise ModelFailedError(f"bad completion to {self!r}: {completion}")

        # clean completion
        completion_text = completion_text.strip()
        # strip ```[python] ... ``` wrapper
        completion_text = regex.sub(r"^```[a-zA-Z]*\n", "", completion_text)
        completion_text = regex.sub(r"\n```$", "", completion_text)
        # replace suspicious unicode characters
        completion_text = completion_text.replace("’", "'")  # noqa: RUF001
        completion_text = completion_text.replace("‘", "'")  # noqa: RUF001
        completion_text = completion_text.replace("“", '"')
        completion_text = completion_text.replace("”", '"')

        return completion_text


class AnthropicModelRunner(ChatModelRunnerBase):
    DEFAULT_MODEL = ModelType.ANTHROPIC_CLAUDE_3_5_SONNET
    MODEL_BY_TYPE: ClassVar[Mapping[ModelType, str]] = {
        ModelType.ANTHROPIC_CLAUDE_3_5_SONNET: "claude-3-5-sonnet-20240620"
    }

    @override
    async def _prepare_input(
        self, projection: Projection, render_options: RenderOptions
    ) -> list[ChatMessage]:
        file_context = await self.render_file_context(
            projection,
            render_options,
            native_types=(FileType.TEXT, FileType.CODE, FileType.IMAGE),
            native_formats_by_type={
                FileType.IMAGE: (FileFormat.JPEG, FileFormat.PNG, FileFormat.GIF, FileFormat.WEBP)
            },
        )
        user_message = ChatMessage(
            role="user",
            content=[
                *(await self.render_system_message(projection, render_options)),
                *(await self.render_page_context(projection, render_options)),
                *file_context,
                *(await self.render_task(projection, render_options)),
                *(await self.render_examples(projection, render_options)),
            ],
        )
        assistant_message = ChatMessage(
            role="assistant",
            content=[ChatMessageTextContent(self.ASSISTANT_PREFIX_MESSAGE)],
        )
        return [user_message, assistant_message]

    async def _convert_message(self, message: ChatMessage) -> anthropic.types.MessageParam:
        assert message.role != "system", "system messages are not supported"
        contents: list[anthropic.types.TextBlockParam | anthropic.types.ImageBlockParam] = []

        for content in message.content:
            if isinstance(content, ChatMessageTextContent):
                if not content.text:
                    continue
                contents.append({"type": "text", "text": content.text})
            elif isinstance(content, ChatMessageFileContent):
                if content.file.coarse_type in (FileType.TEXT, FileType.CODE):
                    if not content.file.text:
                        continue
                    contents.append({"type": "text", "text": content.file.text})
                elif content.file.coarse_type == FileType.IMAGE:
                    contents.append(
                        {
                            "type": "image",
                            "source": {
                                "type": "base64",
                                # image must be of right type (see above)
                                "media_type": cast(Any, content.file.mime_type),
                                "data": content.file.b64encode(),
                            },
                        }
                    )
                else:
                    raise ModelIncapableError(f"cannot convert input {content.file!r}")
            else:
                assert_never(content)
        return {"role": message.role, "content": contents}

    @override
    async def _generate_output(self, messages: list[ChatMessage]) -> str:
        model = self.model or self.DEFAULT_MODEL
        model_key = self.MODEL_BY_TYPE.get(model)
        if model_key is None:
            raise RunImpossibleError(f"unsupported model type {model}")
        converted_messages = [await self._convert_message(message) for message in messages]
        completion = await anthropic_client.messages.create(
            system=self.SYSTEM_MESSAGE,
            messages=converted_messages,
            model=model_key,
            temperature=0.1,
            max_tokens=1024 * 4,
        )
        if not completion.content or completion.content[0].type != "text":
            raise ModelFailedError(f"bad completion to {self!r}: {completion}")
        completion_text = completion.content[0].text
        return completion_text


class GoogleModelRunner(ChatModelRunnerBase):
    pass
