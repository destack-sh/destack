from typing import TYPE_CHECKING, Optional

from bench.language.core import (
    BuiltinEnum,
    EnumType,
    Struct,
    StructType,
    TypeConstraintIn,
    enum_,
    p_regular,
    struct_,
)

if TYPE_CHECKING:
    pass


@enum_(EnumType.MODEL_DEVELOPER)
class ModelDeveloper(BuiltinEnum):
    # internal
    # ...
    # external
    META = 100
    OPENAI = 200
    ANTHROPIC = 300
    GOOGLE = 400
    AMAZON = 500
    MICROSOFT = 600
    DEEPSEEK = 700


@enum_(EnumType.MODEL_PROVIDER)
class ModelProvider(BuiltinEnum):
    # internal
    # ...
    # external developers (100-19999)
    META = 100, "Meta", "Meta"
    OPENAI = 200, "OpenAI", "OpenAI"
    ANTHROPIC = 300, "Anthropic", "Anthropic"
    GOOGLE = 400, "Google", "Google"
    AMAZON = 500, "Amazon", "Amazon"
    MICROSOFT = 600, "Microsoft", "Microsoft"
    DEEPSEEK = 700, "DeepSeek", "DeepSeek"
    # external providers (20000-)
    # ...


@enum_(EnumType.MODEL_FAMILY)
class ModelFamily(BuiltinEnum):
    # internal
    # ...
    # external
    # meta
    META_LLAMA = 100, "Meta Llama", "Llama"
    # openai
    OPENAI_GPT = 200, "OpenAI GPT", "GPT"
    OPENAI_O = 220, "OpenAI o", "o"
    # anthropic
    ANTHROPIC_CLAUDE = 300, "Anthropic Claude", "Claude"
    # google
    GOOGLE_GEMINI = 400, "Google Gemini", "Gemini"
    # amazon
    # ...
    # microsoft
    # ...
    # deepseek
    DEEPSEEK_R = 700, "DeepSeek R", "DeepSeek R"


@enum_(EnumType.MODEL_TYPE)
class ModelType(BuiltinEnum):  # :ModelType
    # internal
    ...  # ?
    # external
    # meta-llama
    META_LLAMA_3_1_80B = 101, "Llama 3.1 80B", "Llama 3.1 80B"
    META_LLAMA_3_1_400B = 102, "Llama 3.1 400B", "Llama 3.1 400B"
    # openai-gpt
    OPENAI_GPT4_0 = 201, "GPT-4", "GPT-4"
    OPENAI_GPT4_O_MINI = 202, "GPT-4o-mini", "GPT-4o-mini"
    OPENAI_GPT4_5 = 203, "GPT-4o-mini", "GPT-4o-mini"
    # openai-o
    OPENAI_O1 = 220, "o1", "o1"
    OPENAI_O1_MINI = 221, "o1-mini", "o1-mini"
    OPENAI_O3_MINI = 222, "o3-mini", "o3-mini"
    # anthropic-claude
    ANTHROPIC_CLAUDE_3_5_SONNET = 301, "Claude 3.5 Sonnet", "Claude 3.5 Sonnet"
    ANTHROPIC_CLAUDE_3_7_SONNET = 302, "Claude 3.7 Sonnet", "Claude 3.7 Sonnet"
    # google-gemini
    GOOGLE_GEMINI_2_0_FLASH = 400, "Gemini 2.0 Flash", "Gemini 2.0 Flash"
    GOOGLE_GEMINI_2_0_FLASH_THINKING = 401, "Gemini 2.0 Flash Thinking", "Gemini 2.0 Flash Thinking"
    GOOGLE_GEMINI_2_5_PRO = 410, "Gemini 2.5 Pro", "Gemini 2.5 Pro"
    # deepseek-r
    DEEPSEEK_R_1_0 = 701, "DeepSeek R 1.0", "DeepSeek R 1.0"


@struct_(StructType.TEXT_OPTIONS)
class TextOptions(Struct):
    """Options for text models (in/out)."""

    temperature: Optional[float] = p_regular(
        30, constraint=TypeConstraintIn(min_value=0.0, max_value=5.0)
    )


@struct_(StructType.AUDIO_OPTIONS)
class AudioOptions(Struct):
    """Options for audio models (in/out)."""

    pass


@struct_(StructType.IMAGE_OPTIONS)
class ImageOptions(Struct):
    """Options for image models (in/out)."""

    pass


@struct_(StructType.VIDEO_OPTIONS)
class VideoOptions(Struct):
    """Options for video models (in/out)."""

    pass
