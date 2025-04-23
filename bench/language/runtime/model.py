from typing import TYPE_CHECKING

from bench.language.core import (
    BuiltinEnum,
    ColorType,
    EnumType,
    enum_,
)

if TYPE_CHECKING:
    pass


@enum_(EnumType.MODEL_DEVELOPER)
class ModelDeveloper(BuiltinEnum):
    # internal
    # ...
    # external
    META = 1000, "Meta", "Meta", "fab fa-meta", ColorType.BLUE
    OPENAI = 1010, "OpenAI", "OpenAI", "fas fa-o", ColorType.GRAY
    ANTHROPIC = 1020, "Anthropic", "Anthropic", "fas fa-a", ColorType.PURPLE
    GOOGLE = 1030, "Google", "Google", "fab fa-google", ColorType.RED
    MICROSOFT = 1040, "Microsoft", "Microsoft", "fab fa-microsoft", ColorType.BLUE
    DEEPSEEK = 1050, "DeepSeek", "DeepSeek", "fas fa-whale", ColorType.GRAY
    XAI = 1060, "XAI", "XAI", "fas fa-x", ColorType.GRAY


@enum_(EnumType.MODEL_PROVIDER)
class ModelProvider(BuiltinEnum):
    # internal
    # ...
    # external
    OPENROUTER = 1000, "OpenRouter", "OpenRouter"
    OPENAI = 1010, "OpenAI", "OpenAI"
    ANTHROPIC = 1020, "Anthropic", "Anthropic"
    GOOGLE = 1030, "Google", "Google"
    XAI = 1040, "XAI", "XAI"
