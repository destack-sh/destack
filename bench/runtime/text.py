import abc
from dataclasses import dataclass
from typing import Any, ClassVar, Literal, Mapping, override

import anthropic
import openai

from bench.language.code import Code
from bench.language.project import Projection, ProjectOptions, project
from bench.language.render import RenderOptions, render
from bench.language.run import ModelProvider, ModelType, RunKind
from bench.runtime.core import RUN_ONCE, NotRunnableError
from bench.runtime.runner import Runner, runner
from bench.utils.utils import get_from_env


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
class ChatMessage:
    role: Literal["user", "assistant", "system"]
    content: str


class ChatModelRunnerBase(Runner, abc.ABC):
    """
    The base for chat-like text function runners.
    Basically, the model generates code that produces the answer, we run it and return that.
    """

    SYSTEM_MESSAGE = """
You are a computational assistant on a new development platform called Bench.

Users define their programs in a language of Blocks, Fields, Steps, Views, etc.,
 some of which are 'rendered' into Python code for you to consider as context & instructions.
The instructions therein may be unclear, incomplete or conflicting 
 - use your best judgement as to probable intent.

Your one and only job is to generate valid Python code that returns outputs to a SPECIFIC input for a SPECIFIC function.
For instance, for a task like the following:

TellJoke = Block.new(
    BlockType.TEXT,
    "GiveSentiment",
    fields=[Field.input("Topic", str), Field.output("Joke", str)],
)

You may be tasked to generate the output for the given inputs:

TellJoke("I'm very happy!")

And you must return something like (note no method wrapper):

return {"Joke": "Why did the scarecrow win an award? Because he was outstanding in his field!"}
"""

    @property
    def model(self) -> ModelType | None:
        if self.handle.options.model_options is not None:
            return self.handle.options.model_options.model
        else:
            return None

    @override
    async def run(self) -> None:
        projection = project(self.state.node, self.handle.inputs, options=ProjectOptions())
        code_str = await self._generate_code(projection)
        code = Code.from_string(code_str)
        code_handle = await self.runtime.make_run_handle(
            RunKind.CODE, node=self.node, code=code, options=RUN_ONCE, track=True
        )
        await self.runtime.run_handle(code_handle)
        self.handle.outputs = code_handle.outputs

    async def _make_messages(
        self, projection: Projection, *, include_system_message: bool
    ) -> list[ChatMessage]:
        # NOTE :Incomplete: add previous attempts errors to messages
        # NOTE :Incomplete: add images/file references to messages
        # rendered_context = render(...)  # nocheckin: context
        if self.inputs is None or len(self.inputs) == 0:
            raise NotRunnableError(f"no inputs for {self.handle!r}")
        render_options = RenderOptions(scope=self.node)
        rendered_task = render(self.node, options=render_options)
        rendered_inputs = render(self.inputs, options=render_options)
        messages: list[ChatMessage] = [
            ChatMessage(
                "user",
                f"""\
# 
# Inputs for your task
#
                        
{rendered_inputs}

#
# Your task is `{self.node.name}`
#

{rendered_task}

#
# Generate the answer to the task '{self.node.name}' with the given inputs and return it. 
# Do NOT attempt to generalize over inputs, just return the answer for the given inputs only.
# You may import and use the python standard library for maths and such if required, but nothing else.
# 
""",
            ),
            ChatMessage(
                "assistant",
                # "# here is how I'll return the answer explained in comments, followed by the code: \n",
                """\
# Here is the code method body that returns the right answer for the given inputs only
#  without any method wrapper or consideration for other possible inputs:""",
            ),
        ]
        if include_system_message:
            messages = [ChatMessage("system", self.SYSTEM_MESSAGE), *messages]
        return messages

    @abc.abstractmethod
    async def _generate_code(self, projection: Projection) -> str: ...


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

    def _convert_message(self, message: ChatMessage) -> Any:  # nocheckin: type this
        return {"role": message.role, "content": message.content}

    @override
    async def _generate_code(self, projection: Projection) -> str:
        model = self.model or self.DEFAULT_MODEL
        model_key = self.MODEL_BY_TYPE.get(model)
        if model_key is None:
            raise NotRunnableError(f"unsupported model type {self.state.key}")
        messages = await self._make_messages(projection, include_system_message=True)
        completion = await openai_client.chat.completions.create(
            messages=[self._convert_message(message) for message in messages],
            model=model_key,
            temperature=0.1,
            user=str(self.runtime.package.id),
        )
        completion_text = completion.choices[0].message.content
        assert completion_text, "empty completion"
        return completion_text


@runner(RunKind.TEXT, ModelProvider.ANTHROPIC)
class AnthropicModelRunner(ChatModelRunnerBase):
    DEFAULT_MODEL = ModelType.CLAUDE_3_5_SONNET
    MODEL_BY_TYPE: ClassVar[Mapping[ModelType, str]] = {
        ModelType.CLAUDE_3_5_SONNET: "claude-3-5-sonnet-20240620"
    }

    def _convert_message(self, message: ChatMessage) -> anthropic.types.MessageParam:
        assert message.role != "system", "system messages are not supported"
        return {"role": message.role, "content": message.content}

    @override
    async def _generate_code(self, projection: Projection) -> str:
        model = self.model or self.DEFAULT_MODEL
        model_key = self.MODEL_BY_TYPE.get(model)
        if model_key is None:
            raise NotRunnableError(f"unsupported model type {self.state.key}")
        messages = await self._make_messages(projection, include_system_message=False)
        completion = await anthropic_client.messages.create(
            system=self.SYSTEM_MESSAGE,
            messages=[self._convert_message(message) for message in messages],
            model=model_key,
            temperature=0.1,
            max_tokens=1024 * 4,
        )
        assert completion.content, "empty completion"
        assert (
            completion.content[0].type == "text"
        ), f"unexpected completion type: {completion.content}"
        completion_text = completion.content[0].text
        return completion_text
