from typing import TYPE_CHECKING

from destack.core import Enum, EnumType, declare_enum

if TYPE_CHECKING:
    pass

# NOTE: Architecture: Model should maybe just be a Node instead of builtin enums?


@declare_enum(EnumType.MODEL_DEVELOPER)
class ModelDeveloper(Enum):
    # internal
    # ...
    # external
    # META = 1000, "Meta", "Meta", "fab fa-meta"
    OPENAI = 1010, "OpenAI", "OpenAI", "https://chatgpt.com/favicon.ico"
    ANTHROPIC = 1020, "Anthropic", "Anthropic", "https://claude.ai/favicon.ico"
    GOOGLE = 1030, "Google", "Google", "https://www.google.com/favicon.ico"
    # MICROSOFT = 1040, "Microsoft", "Microsoft", "fab fa-microsoft"
    # DEEPSEEK = 1050, "DeepSeek", "DeepSeek", "https://www.deepseek.com/favicon.ico"
    XAI = 1060, "xAI", "xAI", "https://x.com/favicon.ico"


@declare_enum(EnumType.MODEL_PROVIDER)
class ModelProvider(Enum):
    # internal
    # ...
    # external
    OPENROUTER = 1000, "OpenRouter", "OpenRouter"
    OPENAI = 1010, "OpenAI", "OpenAI"
    ANTHROPIC = 1020, "Anthropic", "Anthropic"
    GOOGLE = 1030, "Google", "Google"
    XAI = 1040, "xAI", "xAI"
