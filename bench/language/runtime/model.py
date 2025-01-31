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
    META = 100
    OPENAI = 200
    ANTHROPIC = 300
    GOOGLE = 400
    AMAZON = 500
    MICROSOFT = 600
    DEEPSEEK = 700
    # external providers (20000-)
    # ...


@enum_(EnumType.MODEL_FAMILY)
class ModelFamily(BuiltinEnum):
    # internal
    # ...
    # external
    # meta
    META_LLAMA = 100
    # openai
    OPENAI_GPT = 200
    OPENAI_O = 220
    # anthropic
    ANTHROPIC_CLAUDE = 300
    # google
    GOOGLE_GEMINI = 400
    # amazon
    # ...
    # microsoft
    # ...
    # deepseek
    DEEPSEEK_R = 700


@enum_(EnumType.MODEL_TYPE)
class ModelType(BuiltinEnum):  # :ModelType
    # internal
    ...  # ?
    # external
    # meta-llama
    META_LLAMA_3_1_80B = 101
    META_LLAMA_3_1_400B = 102
    # openai-gpt
    OPENAI_GPT4_0 = 201
    OPENAI_GPT4_O_MINI = 202
    # openai-o
    OPENAI_O1 = 220
    OPENAI_O1_MINI = 221
    OPENAI_O3_MINI = 222
    # anthropic-claude
    ANTHROPIC_CLAUDE_3_5_SONNET = 301
    # google-gemini
    GOOGLE_GEMINI_2_0_FLASH = 400
    GOOGLE_GEMINI_2_0_FLASH_THINKING = 401
    # deepseek-r
    DEEPSEEK_R_1_0 = 701


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
