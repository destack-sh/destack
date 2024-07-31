import abc
import base64
from dataclasses import dataclass
from typing import ClassVar, Literal, Mapping, Union, assert_never, cast, override

import anthropic
import openai
import structlog
from openai.types import chat as openai_chat_types
from opentelemetry import trace

from bench.language.code import Code
from bench.language.file import FileReference, FileType
from bench.language.project import Projection, ProjectOptions, project
from bench.language.render import RenderOptions, render, render_value_expr
from bench.language.run import ModelProvider, ModelType, RunKind
from bench.language.value import sample_value
from bench.runtime.core import RUN_ONCE, ModelFailedError, ModelIncapableError, RunImpossibleError
from bench.runtime.runner import Runner, runner
from bench.utils.utils import get_from_env

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


@runner(RunKind.TEXT, None)
class ModelRouter(Runner):
    """The router for text functions without explicitly assigned models/providers."""

    @override
    async def run(self) -> None:
        model_handle = await self.runtime.make_run_handle(
            RunKind.TEXT,
            key=ModelProvider.ANTHROPIC,
            node=self.node,
            options=RUN_ONCE,
            inputs=self.inputs,
            track=False,
        )
        await self.runtime.run_handle(model_handle)
        self.handle.outputs = model_handle.outputs


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
    file: FileReference


ChatMessageContent = Union[ChatMessageTextContent, ChatMessageFileContent]


class ChatModelRunnerBase(Runner, abc.ABC):
    """
    The base for chat-like text function runners.
    Basically, the model generates code that produces the answer, we run it and return that.
    """

    SYSTEM_MESSAGE = """\
# You are a computational assistant on a new development platform called Bench.
# Users define their programs in a language of Blocks, Fields, Steps, Views, etc.,
#  some will be 'rendered' into Python code for you to consider as context, inputs & instructions.

# Your one and only job is to generate valid Python answers as outputs to a SPECIFIC invocation of a SPECIFIC task.
# You MUST use your best judgement to fill in incomplete or conflicting information.
# Consider an unrelated example task like the following:

TellJoke = Block.new(
    BlockType.TEXT,
    "GiveSentiment",
    fields=[Field.input("Topic", str), Field.output("Joke", str)],
)

# For the given inputs:

TellJoke("I'm very happy!")

# You would return something like:

return {"Joke": "Why did the scarecrow win an award? Because he was outstanding in his field!"}

# There are many more complex tasks and types; more examples are provided as needed.
"""
    USER_POSTFIX_MESSAGE = """\
#
# Return the answer to the specific invocation of task '{task_alias}' with the given inputs.
#  - You MUST NOT attempt to generalize over inputs; return the answer for the given inputs only.
#  - You MAY generate reasoning *before* the respective answer (especially if it's in the output).
#  - You MAY import and use the Python standard library for maths and such, but nothing else.
#  - You MAY raise ModelIncapableError("<reason>") if a fitting output is impossible.
# 
"""
    ASSISTANT_PREFIX_MESSAGE = """\
# Here is the Python method body that returns the specific answer for these specific inputs:
"""

    @property
    def model(self) -> ModelType | None:
        if self.handle.options.model_options is not None:
            return self.handle.options.model_options.model
        else:
            return None

    @property
    def task_alias(self) -> str:
        return self.node.absolute_path

    @override
    async def run(self) -> None:
        projection = project(self.state.node, self.handle.inputs, options=ProjectOptions())
        log = logger.bind(runner=self, projection=projection)
        messages = await self._generate_messages(projection)
        with tracer.start_as_current_span("text.generate_code"):
            try:
                code = await self._generate_code(messages)
                log.trace("text.generate_code", code=code, span="current")
            except BaseException as e:
                log.trace("text.generate_code.error", exc_info=e, span="current")
                raise

        code = Code.from_string(code)
        with tracer.start_as_current_span("text.run_code"):
            code_handle = await self.runtime.make_run_handle(
                RunKind.CODE, node=self.node, code=code, options=RUN_ONCE, track=True
            )
            await self.runtime.run_handle(code_handle)
        self.handle.outputs = code_handle.outputs

    async def render_system_message(self, projection: Projection):
        """Renders the system message for the projection"""
        return (ChatMessageTextContent(self.SYSTEM_MESSAGE),)

    async def render_page_context(self, projection: Projection):
        """Renders the source context pages from the projection"""
        pages = projection.get_containing_pages()
        rendered_pages = []
        for page in pages:
            rendered_page = render(page, options=RenderOptions(scope=page, as_page=True))
            rendered_context = f"""\
# {'=' * 24}
# `{page.absolute_path}`
# {'=' * 24}

{rendered_page}
"""
            rendered_pages.append(rendered_context)
        rendered = ChatMessageTextContent(f"""\
# 
# Context around your task '{self.task_alias}'
# Includes relevant and irrelevant instructions and information to consider.
#

{'\n\n'.join(rendered_pages) or "# <no context available>"}
""")
        return (rendered,)

    async def render_file_context(self, projection: Projection):
        """
        Renders the context files from the projection.
        """
        raise NotImplementedError("nocheckin: render_file_context")

    async def render_task(self, projection: Projection):
        """Renders the task from the projection"""
        if self.inputs is None or len(self.inputs) == 0:
            raise RunImpossibleError(f"no inputs for {self.handle!r}")
        render_options = RenderOptions(scope=self.node)
        rendered_task = render(self.node, options=render_options)
        rendered_inputs = render(self.inputs, options=render_options)

        rendered = ChatMessageTextContent(f"""\
#
# Inputs for your specific task '{self.task_alias}'
# 

{rendered_inputs}

# 
# Your specific task is `{self.task_alias}`
# You MUST focus on this task with the inputs above in relation to the provided context.
#

{rendered_task}
""")
        return (rendered,)

    async def render_examples(self, projection: Projection):
        """Renders relevant examples for the task / context"""
        if self.output_type is None or len(self.output_type._fields) == 0:
            raise RunImpossibleError(f"no outputs for {self.handle!r}")

        render_options = RenderOptions(scope=self.node)
        rendered_examples = []
        for field in self.output_type._fields:
            example_value = sample_value(field)
            rendered_example = render_value_expr(example_value, field, options=render_options)
            rendered_examples.append(f"{field.name} = {rendered_example}")

        rendered = ChatMessageTextContent(f"""\
# 
# Some random syntax examples for values of the right types
#  (the values are *not* specific to your actual task and semantically irrelevant)
#

{'\n'.join(e for e in rendered_examples)}
""")
        return (rendered,)

    # @tracer.start_as_current_span("text.make_messages")
    # async def _make_messages(self, projection: Projection) -> list[ChatMessage]:
    #     # NOTE :Incomplete: add previous attempts errors to messages

    #     contents: list[ChatMessageContent] = []
    #     task_alias = self.node.absolute_path

    # specific task (repeated) + inputs

    # example values

    # build messages
    #         messages: list[ChatMessage] = [
    #             ChatMessage(
    #                 "user",
    #                 f"""\
    # #
    # # Context around your task '{task_alias}'
    # # Includes relevant and irrelevant instructions and information to consider.
    # #

    # {'\n\n'.join(rendered_pages) or "# <no context available>"}

    # #
    # # Inputs for your specific task '{task_alias}'
    # #

    # {rendered_inputs}

    # #
    # # Your specific task is `{task_alias}`
    # # You MUST focus on this task with these inputs in relation to the provided context.
    # #

    # {rendered_task}

    # #
    # # Some random syntax examples for values of the right types
    # #  (the values are *not* specific to your actual task and semantically irrelevant)
    # #

    # {'\n'.join(e for e in rendered_examples)}

    #
    # Return the answer to the specific invocation of task '{task_alias}' with the given inputs.
    #  - You MUST NOT attempt to generalize over inputs; return the answer for the given inputs only.
    #  - You MAY generate reasoning *before* the respective answer (especially if it's in the output).
    #  - You MAY import and use the Python standard library for maths and such, but nothing else.
    #  - You MAY raise ModelIncapableError("<reason>") if a fitting output is impossible.
    #
    # """,
    #             ),
    #             ChatMessage(
    #                 "assistant",
    #                 """\
    # # Here is the Python method body that returns the specific answer for these specific inputs:""",
    #             ),
    #         ]
    # if self.INCLUDE_SYSTEM_MESSAGE:
    #     messages = [
    #         ChatMessage("system", [ChatMessageTextContent(self.SYSTEM_MESSAGE)]),
    #         *messages,
    #     ]
    # return messages

    @abc.abstractmethod
    async def _generate_messages(self, projection: Projection) -> list[ChatMessage]: ...

    @abc.abstractmethod
    async def _generate_code(self, messages: list[ChatMessage]) -> str: ...


openai_client = openai.AsyncClient(
    api_key=get_from_env("OPENAI_API_KEY", description="OpenAI API key")
)
anthropic_client = anthropic.AsyncClient(
    api_key=get_from_env("ANTHROPIC_API_KEY", description="Anthropic API key")
)


@runner(RunKind.TEXT, ModelProvider.OPENAI)
class OpenaiModelRunner(ChatModelRunnerBase):
    DEFAULT_MODEL = ModelType.GPT40
    MODEL_BY_TYPE: ClassVar[Mapping[ModelType, str]] = {ModelType.GPT40: "gpt-4o"}

    @override
    async def _generate_messages(self, projection: Projection) -> list[ChatMessage]:
        system_message = ChatMessage(
            role="system",
            content=[*(await self.render_system_message(projection))],
        )
        user_message = ChatMessage(
            role="user",
            content=[
                *(await self.render_page_context(projection)),
                *(await self.render_file_context(projection)),
                *(await self.render_task(projection)),
                *(await self.render_examples(projection)),
            ],
        )
        assistant_message = ChatMessage(
            role="assistant",
            content=[ChatMessageTextContent(self.ASSISTANT_PREFIX_MESSAGE)],
        )
        return [system_message, user_message, assistant_message]

    def _convert_message_content(
        self, content: ChatMessageContent
    ) -> openai_chat_types.ChatCompletionContentPartParam:
        raise NotImplementedError("nocheckin: convert_message_content")

    def _convert_message(
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
            return {
                "role": message.role,
                "content": [self._convert_message_content(c) for c in message.content],
            }
        else:
            assert_never(message.role)

    @override
    async def _generate_code(self, messages: list[ChatMessage]) -> str:
        model = self.model or self.DEFAULT_MODEL
        model_key = self.MODEL_BY_TYPE.get(model)
        if model_key is None:
            raise RunImpossibleError(f"unsupported model type {self.state.key}")
        completion = await openai_client.chat.completions.create(
            messages=[self._convert_message(message) for message in messages],
            model=model_key,
            temperature=0.1,
            user=str(self.runtime.package.id),
        )
        completion_text = completion.choices[0].message.content
        if not completion_text:
            raise ModelFailedError(f"bad completion to {self!r}: {completion}")
        return completion_text


@runner(RunKind.TEXT, ModelProvider.ANTHROPIC)
class AnthropicModelRunner(ChatModelRunnerBase):
    DEFAULT_MODEL = ModelType.CLAUDE_3_5_SONNET
    MODEL_BY_TYPE: ClassVar[Mapping[ModelType, str]] = {
        ModelType.CLAUDE_3_5_SONNET: "claude-3-5-sonnet-20240620"
    }
    INCLUDE_SYSTEM_MESSAGE: ClassVar[bool] = False

    @override
    async def _generate_messages(self, projection: Projection) -> list[ChatMessage]:
        user_message = ChatMessage(
            role="user",
            content=[
                *(await self.render_system_message(projection)),
                *(await self.render_page_context(projection)),
                *(await self.render_file_context(projection)),
                *(await self.render_task(projection)),
                *(await self.render_examples(projection)),
            ],
        )
        assistant_message = ChatMessage(
            role="assistant",
            content=[ChatMessageTextContent(self.ASSISTANT_PREFIX_MESSAGE)],
        )
        return [user_message, assistant_message]

    def _convert_message_content(
        self, content: ChatMessageContent
    ) -> Union[anthropic.types.TextBlockParam, anthropic.types.ImageBlockParam]:
        if isinstance(content, ChatMessageTextContent):
            return {"type": "text", "text": content.text}
        elif isinstance(content, ChatMessageFileContent):
            if content.file.kind == FileType.IMAGE:
                image_b64 = base64.b64encode(content.file.content).decode()
                if content.file.mime_type not in (
                    "image/jpeg",
                    "image/png",
                    "image/gif",
                    "image/webp",
                ):
                    raise ModelIncapableError(f"unsupported image: {content.file!r}")
                return {
                    "type": "image",
                    "source": {
                        "type": "base64",
                        "media_type": content.file.mime_type,
                        "data": image_b64,
                    },
                }
            else:
                raise NotImplementedError(f"cannot convert file {content.file!r}")
        else:
            assert_never(content)

    def _convert_message(self, message: ChatMessage) -> anthropic.types.MessageParam:
        assert message.role != "system", "system messages are not supported"
        return {
            "role": message.role,
            "content": [self._convert_message_content(c) for c in message.content],
        }

    @override
    async def _generate_code(self, messages: list[ChatMessage]) -> str:
        model = self.model or self.DEFAULT_MODEL
        model_key = self.MODEL_BY_TYPE.get(model)
        if model_key is None:
            raise RunImpossibleError(f"unsupported model type {self.state.key}")
        completion = await anthropic_client.messages.create(
            system=self.SYSTEM_MESSAGE,
            messages=[self._convert_message(message) for message in messages],
            model=model_key,
            temperature=0.1,
            max_tokens=1024 * 4,
        )
        if not completion.content or completion.content[0].type != "text":
            raise ModelFailedError(f"bad completion to {self!r}: {completion}")
        completion_text = completion.content[0].text
        return completion_text


@runner(RunKind.TEXT, ModelProvider.GOOGLE)
class GoogleModelRunner(ChatModelRunnerBase):
    pass
