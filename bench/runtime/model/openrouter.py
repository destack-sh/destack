from typing import override

import openai
import structlog
from opentelemetry import trace

from bench.language import Code
from bench.utils.func import hash_stable_hex
from bench.utils.utils import get_from_env

from .chat import ChatModelRunner
from .code import StreamingCodeRunner
from .openai import build_openai_chat_messages
from .prompt import LOG_PROMPTS, log_completion, log_prompt
from .token import TiktokenTokenizer

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)

openrouter_client = openai.AsyncClient(
    base_url="https://openrouter.ai/api/v1",
    api_key=get_from_env("OPENROUTER_API_KEY", description="OpenRouter API key"),
)
OPENROUTER_DEFAULT_MODEL = "openai/gpt-4o"


class OpenRouterChatModelRunner(ChatModelRunner):
    """Run any chat model supported by OpenRouter."""

    BREAK = "\n"
    SEPARATOR = "#" * 32  # = exactly 1 token

    @override
    async def run(self) -> None:
        from bench.runtime import MACROS, AgentRunner

        agent = self.agent
        assert agent is not None, f"{self!r} has no Agent"
        model_id = self.model_id or OPENROUTER_DEFAULT_MODEL

        # build
        max_tokens = 16_384
        messages, pieces = await build_openai_chat_messages(
            prompt=self.prompt,
            tokenizer=TiktokenTokenizer(),
            max_tokens=max_tokens,
            session=self.session,
        )
        if LOG_PROMPTS:
            log_prompt(self.prompt, pieces)

        # generate & execute simultaneously
        agent_runner = self.closest_runner_like(AgentRunner)
        code_runner = StreamingCodeRunner(
            runner=agent_runner, macros=MACROS, aliasing=self.prompt.aliasing
        )
        completion = await openrouter_client.chat.completions.create(
            messages=messages,
            model=model_id,
            max_tokens=max_tokens,
            user=hash_stable_hex(self.runtime.bench.id.int),
            stream=True,
        )
        async for chunk in completion:
            if chunk.choices and chunk.choices[0].delta.content:
                chunk_content = chunk.choices[0].delta.content
                code_runner.add_and_execute(chunk_content)
        if LOG_PROMPTS:
            log_completion(code_runner.code)
        code_runner.complete_and_execute()
        self.code = Code.from_string(code_runner.code or "pass", language="python")
