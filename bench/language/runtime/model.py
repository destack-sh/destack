from typing import TYPE_CHECKING, Optional

from bench.language.core import (
    EnumType,
    Struct,
    StructType,
    TypeConstraintIn,
    enum_,
    p_regular,
    struct_,
)
from bench.utils.func import IdEnum

if TYPE_CHECKING:
    pass


@enum_(EnumType.MODEL_DEVELOPER)
class ModelDeveloper(IdEnum):
    # internal
    # ...
    # external
    META = 100
    OPENAI = 200
    ANTHROPIC = 300
    GOOGLE = 400
    AMAZON = 500
    MICROSOFT = 600


@enum_(EnumType.MODEL_PROVIDER)
class ModelProvider(IdEnum):
    # internal
    # ...
    # external developers (100-19999)
    META = 100
    OPENAI = 200
    ANTHROPIC = 300
    GOOGLE = 400
    AMAZON = 500
    MICROSOFT = 600
    # external providers (20000-)
    # ...


@enum_(EnumType.MODEL_FAMILY)
class ModelFamily(IdEnum):
    # internal
    # ...
    # external
    # meta
    META_LLAMA = 100
    # openai
    OPENAI_GPT = 200
    OPENAI_O = 210
    # anthropic
    ANTHROPIC_CLAUDE = 300
    # google
    GOOGLE_GEMINI = 400
    # amazon
    # ...
    # microsoft
    # ...


@enum_(EnumType.MODEL_TYPE)
class ModelType(IdEnum):  # :ModelType
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
    OPENAI_O1 = 210
    OPENAI_O1_MINI = 211
    # anthropic-claude
    ANTHROPIC_CLAUDE_3_5_SONNET = 301
    # google-gemini
    GOOGLE_GEMINI_1_5_PRO = 401
    GOOGLE_GEMINI_2_0_FLASH = 402


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
