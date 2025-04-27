from typing import TYPE_CHECKING

from bench.language.core import (
    BuiltinEnum,
    ColorType,
    EnumType,
    enum_,
)

if TYPE_CHECKING:
    pass

# NOTE: Architecture: Model should maybe just be a Node instead of builtin enums?


@enum_(EnumType.MODEL_DEVELOPER)
class ModelDeveloper(BuiltinEnum):
    # internal
    # ...
    # external
    # META = 1000, "Meta", "Meta", "fab fa-meta", ColorType.BLUE
    OPENAI = 1010, "OpenAI", "OpenAI", "https://chatgpt.com/favicon.ico", ColorType.GRAY
    ANTHROPIC = 1020, "Anthropic", "Anthropic", "https://claude.ai/favicon.ico", ColorType.PURPLE
    GOOGLE = 1030, "Google", "Google", "https://www.google.com/favicon.ico", ColorType.RED
    # MICROSOFT = 1040, "Microsoft", "Microsoft", "fab fa-microsoft", ColorType.BLUE
    # DEEPSEEK = 1050, "DeepSeek", "DeepSeek", "https://www.deepseek.com/favicon.ico", ColorType.GRAY
    XAI = 1060, "xAI", "xAI", "https://x.com/favicon.ico", ColorType.GRAY


@enum_(EnumType.MODEL_PROVIDER)
class ModelProvider(BuiltinEnum):
    # internal
    # ...
    # external
    OPENROUTER = 1000, "OpenRouter", "OpenRouter"
    OPENAI = 1010, "OpenAI", "OpenAI"
    ANTHROPIC = 1020, "Anthropic", "Anthropic"
    GOOGLE = 1030, "Google", "Google"
    XAI = 1040, "xAI", "xAI"
