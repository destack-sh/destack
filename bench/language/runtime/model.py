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


@enum_(EnumType.MODEL_PROVIDER)
class ModelProvider(BuiltinEnum):
    # internal
    # ...
    # external
    OPENAI = 1000, "OpenAI", "OpenAI"
    ANTHROPIC = 1010, "Anthropic", "Anthropic"
    GOOGLE = 1020, "Google", "Google"
    OPENROUTER = 1030, "OpenRouter", "OpenRouter"
